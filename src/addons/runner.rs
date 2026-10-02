use crate::utils::template_engine::TemplateContext;
use std::{
  collections::HashMap,
  fs,
  path::Path,
  sync::{Arc, Mutex},
};

use anyhow::{Context, Result, anyhow};
use inquire::{Confirm, Select, Text};

use crate::{
  context::{AppContext, CleanupState, CleanupTask},
  manifest::AnesisManifest,
  templates::generator::{to_camel_case, to_kebab_case, to_pascal_case, to_snake_case},
  utils::{
    picker::{ItemKind, PickItem, pick_one},
    ui::{self, spinner},
  },
};

use super::{
  detect::detect_variant,
  install::{fetch_latest_version, install_addon, read_cached_manifest, record_addon_use},
  lock::{LockEntry, LockFile},
  manifest::{AddonCommand, InputDef, InputType},
  steps::{
    Rollback, append::execute_append, copy::execute_copy, create::execute_create,
    delete::execute_delete, inject::execute_inject, json_patch::execute_json_patch,
    move_step::execute_move, packages::execute_packages, rename::execute_rename,
    replace::execute_replace, run::execute_run,
  },
};
use crate::addons::manifest::{Step, StepEntry};

fn eval_step_when(expr: &str, inputs: &HashMap<String, String>) -> Result<bool> {
  let expr = expr.trim();
  let (negate, name) = match expr.strip_prefix('!') {
    Some(rest) => (true, rest.trim()),
    None => (false, expr),
  };
  let value = inputs
    .get(name)
    .ok_or_else(|| anyhow!("step 'when' references unknown input '{name}'"))?;
  Ok((value == "true") ^ negate)
}

fn effective_steps(steps: &[StepEntry], inputs: &HashMap<String, String>) -> Result<Vec<Step>> {
  let mut out = Vec::with_capacity(steps.len());
  for entry in steps {
    let include = match &entry.when {
      Some(expr) => eval_step_when(expr, inputs)?,
      None => true,
    };
    if include {
      out.push(entry.kind.clone());
    }
  }
  Ok(out)
}

fn take_journal(journal: &Mutex<Vec<Rollback>>) -> Vec<Rollback> {
  std::mem::take(&mut *journal.lock().unwrap_or_else(|e| e.into_inner()))
}

struct ClearCleanupOnDrop<'a>(&'a CleanupState);

impl Drop for ClearCleanupOnDrop<'_> {
  fn drop(&mut self) {
    let mut guard = self.0.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
  }
}

pub async fn run_addon_command(
  ctx: &AppContext,
  addon_id: &str,
  command_name: &str,
  project_root: &Path,
  presets: &HashMap<String, String>,
  non_interactive: bool,
  dry_run: bool,
) -> Result<()> {
  let non_interactive = non_interactive || dry_run;
  let addon_dir = ctx.paths.addon_dir(addon_id)?;
  let cached = super::cache::get_cached_addon(&ctx.paths.addons, addon_id)?;

  let (manifest, update_check) = if let Some(cached) = cached.filter(|_| addon_dir.exists()) {
    let manifest = read_cached_manifest(&ctx.paths.addons, addon_id)?;
    let handle = tokio::spawn(super::install::check_addon_update(
      ctx.client.clone(),
      ctx.backend_url.clone(),
      ctx.paths.auth.clone(),
      addon_id.to_string(),
      cached.commit_sha,
    ));
    (manifest, Some(handle))
  } else {
    let sp = spinner(format!("Fetching addon '{addon_id}'..."));
    let install_result = install_addon(ctx, addon_id)
      .await
      .inspect_err(|_| sp.finish_and_clear())?;
    sp.finish_and_clear();
    if let Some(message) = install_result.message(addon_id) {
      println!("{message}");
    }
    (install_result.into_manifest(), None)
  };

  let mut lock = LockFile::load(project_root)?;

  for dep_id in &manifest.requires {
    if !lock.addons.iter().any(|e| &e.id == dep_id) {
      return Err(anyhow!(
        "Addon '{}' requires '{}' to be applied in this project first. Run: anesis use {} <command>",
        addon_id,
        dep_id,
        dep_id
      ));
    }
  }

  let detected_id = detect_variant(&manifest.detect, project_root);

  let variant = manifest
    .variants
    .iter()
    .find(|v| v.when.as_deref() == detected_id.as_deref())
    .or_else(|| manifest.variants.iter().find(|v| v.when.is_none()))
    .ok_or_else(|| anyhow!("No matching variant found for addon '{}'", addon_id))?;

  let command = variant
    .commands
    .iter()
    .find(|c| c.name == command_name)
    .ok_or_else(|| {
      anyhow!(
        "Command '{}' not found in addon '{}'",
        command_name,
        addon_id
      )
    })?;

  if !dry_run && command.once && lock.is_command_executed(addon_id, command_name) {
    if let Some(prompt_message) = rerun_prompt_message(
      command_name,
      lock.addon_version(addon_id),
      &manifest.version,
    ) {
      let rerun = if non_interactive {
        false
      } else {
        Confirm::new(&prompt_message).with_default(false).prompt()?
      };
      if !rerun {
        println!("Skipping command '{}'.", command_name);
        return Ok(());
      }
    } else {
      println!(
        "Command '{}' has already been executed, skipping.",
        command_name
      );
      return Ok(());
    }
  }

  for req_cmd in &command.requires_commands {
    if !lock.is_command_executed(addon_id, req_cmd) {
      return Err(anyhow!(
        "Command '{}' requires '{}' to be run first. Run: anesis use {} {}",
        command_name,
        req_cmd,
        addon_id,
        req_cmd
      ));
    }
  }

  let mut template_ctx = TemplateContext::new();

  let mut input_values: HashMap<String, String> = HashMap::new();
  collect_inputs(
    &manifest.inputs,
    presets,
    non_interactive,
    &mut input_values,
  )?;
  insert_with_derived(&mut template_ctx, &input_values);

  let mut cmd_input_values: HashMap<String, String> = HashMap::new();
  collect_inputs(
    &command.inputs,
    presets,
    non_interactive,
    &mut cmd_input_values,
  )?;
  insert_with_derived(&mut template_ctx, &cmd_input_values);

  let combined_inputs: HashMap<String, String> = input_values
    .iter()
    .chain(cmd_input_values.iter())
    .map(|(k, v)| (k.clone(), v.clone()))
    .collect();
  let steps = effective_steps(&command.steps, &combined_inputs)?;

  if dry_run {
    print_dry_run_plan(
      addon_id,
      command_name,
      detected_id.as_deref(),
      &input_values,
      &cmd_input_values,
      &steps,
    );
    return Ok(());
  }

  if !confirm_addon_execution(addon_id, command_name, &steps, non_interactive)? {
    return Err(crate::utils::errors::AnesisError::Aborted.into());
  }

  let addon_dir = ctx.paths.addon_dir(addon_id)?;
  let total = steps.len();

  let journal: Arc<Mutex<Vec<Rollback>>> = Arc::new(Mutex::new(Vec::new()));
  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(CleanupTask::PartialAddon {
      addon_id: addon_id.to_string(),
      project_root: project_root.to_path_buf(),
      journal: Arc::clone(&journal),
    });
  }
  let _cleanup_guard = ClearCleanupOnDrop(&ctx.cleanup_state);

  let step_progress = ui::StepProgress::new();
  for (idx, step) in steps.iter().enumerate() {
    let label = step_label(step);
    let handle = step_progress.start_step(idx, total, &label);

    let result = match step {
      Step::Copy(s) => execute_copy(s, &addon_dir, project_root, &template_ctx, non_interactive),
      Step::Create(s) => execute_create(s, project_root, &template_ctx, non_interactive),
      Step::Inject(s) => execute_inject(s, project_root, &template_ctx, non_interactive),
      Step::Replace(s) => execute_replace(s, project_root, &template_ctx, non_interactive),
      Step::Append(s) => execute_append(s, project_root, &template_ctx),
      Step::Delete(s) => execute_delete(s, project_root, &template_ctx),
      Step::Rename(s) => execute_rename(s, project_root, &template_ctx),
      Step::Move(s) => execute_move(s, project_root, &template_ctx),
      Step::Packages(s) => execute_packages(s, project_root, non_interactive, ctx.allow_run),
      Step::Run(s) => execute_run(
        s,
        project_root,
        &template_ctx,
        non_interactive,
        ctx.allow_run,
      ),
      Step::JsonPatch(s) => execute_json_patch(s, project_root, &template_ctx),
    };

    match result {
      Ok(rollbacks) => {
        handle.success();
        journal
          .lock()
          .unwrap_or_else(|e| e.into_inner())
          .extend(rollbacks);
      }
      Err(failure) => {
        handle.failure();
        journal
          .lock()
          .unwrap_or_else(|e| e.into_inner())
          .extend(failure.rollbacks);

        let err = failure
          .error
          .context(format!("step {} ({}) failed", idx + 1, label));
        ui::failure(format!("{err:#}"));
        let choice = if non_interactive {
          "Rollback all changes"
        } else {
          Select::new(
            "How would you like to proceed?",
            vec!["Keep changes made so far", "Rollback all changes"],
          )
          .prompt()?
        };

        if choice == "Rollback all changes" {
          for rollback in take_journal(&journal).into_iter().rev() {
            let _ = apply_rollback(rollback, project_root);
          }
          println!("Rolled back all changes made by this command.");
        }

        return Err(err);
      }
    }
  }

  let completed_rollbacks = journal.lock().unwrap_or_else(|e| e.into_inner()).clone();

  let variant_id = detected_id.unwrap_or_else(|| "universal".to_string());
  if let Some(existing) = lock.addons.iter_mut().find(|e| e.id == addon_id) {
    existing.version = manifest.version.clone();
    existing.variant = variant_id;
    existing.inputs = input_values.clone();
    existing.upsert_command(
      command_name,
      cmd_input_values.clone(),
      completed_rollbacks.clone(),
    );
  } else {
    let mut entry = LockEntry::new(addon_id, manifest.version.clone(), variant_id);
    entry.inputs = input_values.clone();
    entry.upsert_command(
      command_name,
      cmd_input_values.clone(),
      completed_rollbacks.clone(),
    );
    lock.addons.push(entry);
  }

  if let Err(err) = lock.save(project_root) {
    ui::failure(format!(
      "Failed to save the rollback journal ({err:#}); rolling back this command's changes."
    ));
    for rollback in take_journal(&journal).into_iter().rev() {
      let _ = apply_rollback(rollback, project_root);
    }
    println!("Rolled back all changes made by this command.");
    return Err(err.context("Failed to save anesis.lock"));
  }

  take_journal(&journal);
  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
  }

  record_addon_use(ctx, addon_id).await;

  if let Err(err) = AnesisManifest::add_addon(addon_id, project_root) {
    eprintln!("Note: could not update anesis.json ({err}).");
  }
  ui::success(format!("Command '{command_name}' completed successfully."));
  let summary = super::summary::ChangeSummary::from_rollbacks(&completed_rollbacks);
  if !summary.is_empty() {
    println!("{}", summary.render_block(&completed_rollbacks));
  }

  if let Some(handle) = update_check
    && matches!(handle.await, Ok(Some(_)))
  {
    let sp = spinner(format!(
      "A newer version of '{addon_id}' is available, updating..."
    ));
    let updated = install_addon(ctx, addon_id).await;
    sp.finish_and_clear();
    match updated {
      Ok(result) => {
        if let Some(message) = result.update_message(addon_id) {
          println!("{message} (will be used next time)");
        }
      }
      Err(err) => eprintln!("Note: could not update addon '{addon_id}' ({err})."),
    }
  }

  Ok(())
}

pub async fn list_addon_commands(
  ctx: &AppContext,
  addon_id: &str,
  project_root: &Path,
  presets: &HashMap<String, String>,
  non_interactive: bool,
  dry_run: bool,
) -> Result<()> {
  let addon_dir = ctx.paths.addon_dir(addon_id)?;
  let cached = super::cache::get_cached_addon(&ctx.paths.addons, addon_id)?;
  let manifest = if cached.is_some() && addon_dir.exists() {
    read_cached_manifest(&ctx.paths.addons, addon_id)?
  } else {
    let sp = spinner(format!("Fetching addon '{addon_id}'..."));
    let result = install_addon(ctx, addon_id)
      .await
      .inspect_err(|_| sp.finish_and_clear())?;
    sp.finish_and_clear();
    result.into_manifest()
  };

  let detected_id = detect_variant(&manifest.detect, project_root);
  let matched = manifest
    .variants
    .iter()
    .find(|v| v.when.as_deref() == detected_id.as_deref())
    .or_else(|| manifest.variants.iter().find(|v| v.when.is_none()));
  let commands: Vec<&AddonCommand> = match matched {
    Some(variant) => variant.commands.iter().collect(),
    None => Vec::new(),
  };

  if commands.is_empty() {
    println!("Addon '{addon_id}' has no commands available for this project.");
    return Ok(());
  }

  let items: Vec<PickItem> = commands
    .iter()
    .map(|c| PickItem {
      kind: ItemKind::Addon,
      id: c.name.clone(),
      name: c.name.clone(),
      meta: String::new(),
      description: c.description.clone(),
      haystack: format!("{} {}", c.name, c.description).to_lowercase(),
    })
    .collect();

  match pick_one(
    items,
    format!("Commands for {addon_id}"),
    false,
    String::new(),
  )
  .await?
  {
    Some((_, command_name, _)) => {
      run_addon_command(
        ctx,
        addon_id,
        &command_name,
        project_root,
        presets,
        non_interactive,
        dry_run,
      )
      .await
    }
    None => Ok(()),
  }
}

pub(crate) fn step_label(step: &Step) -> String {
  use crate::addons::manifest::Target;
  use crate::utils::sanitize::sanitize_for_display;
  fn target(t: &Target) -> &str {
    match t {
      Target::File { file } => file,
      Target::Glob { glob } => glob,
    }
  }
  let raw = match step {
    Step::Copy(s) => format!("copy '{}' → '{}'", s.src, s.dest),
    Step::Create(s) => format!("create '{}'", s.path),
    Step::Inject(s) => format!("inject into '{}'", target(&s.target)),
    Step::Replace(s) => format!("replace in '{}'", target(&s.target)),
    Step::Append(s) => format!("append to '{}'", target(&s.target)),
    Step::Delete(s) => format!("delete '{}'", target(&s.target)),
    Step::Rename(s) => format!("rename '{}' → '{}'", s.from, s.to),
    Step::Move(s) => format!("move '{}' → '{}'", s.from, s.to),
    Step::Packages(s) => format!(
      "install {} package(s)",
      s.dependencies.len() + s.dev_dependencies.len()
    ),
    Step::Run(s) => format!("run '{}'", s.command),
    Step::JsonPatch(s) => format!("patch JSON in '{}'", s.path),
  };
  sanitize_for_display(&raw)
}

#[doc(hidden)]
pub fn step_label_for_tests(step: &Step) -> String {
  step_label(step)
}

pub struct StepPlan {
  pub label: String,
  pub requires_allow_run: bool,
}

pub struct CommandPlan {
  pub addon_id: String,
  pub command_name: String,
  pub variant: Option<String>,
  pub steps: Vec<StepPlan>,
}

impl CommandPlan {
  pub fn needs_allow_run(&self) -> bool {
    self.steps.iter().any(|s| s.requires_allow_run)
  }
}

pub fn plan_command(
  addon_id: &str,
  command_name: &str,
  variant: Option<&str>,
  steps: &[Step],
) -> CommandPlan {
  CommandPlan {
    addon_id: addon_id.to_string(),
    command_name: command_name.to_string(),
    variant: variant.map(str::to_string),
    steps: steps
      .iter()
      .map(|step| StepPlan {
        label: step_label(step),
        requires_allow_run: matches!(step, Step::Run(_) | Step::Packages(_)),
      })
      .collect(),
  }
}

fn print_dry_run_plan(
  addon_id: &str,
  command_name: &str,
  variant: Option<&str>,
  addon_inputs: &HashMap<String, String>,
  cmd_inputs: &HashMap<String, String>,
  steps: &[Step],
) {
  let plan = plan_command(addon_id, command_name, variant, steps);

  ui::section(format!(
    "Dry run: {} {}",
    ui::accent(&plan.addon_id),
    ui::accent(&plan.command_name)
  ));
  ui::kv("  variant", plan.variant.as_deref().unwrap_or("universal"));

  let mut inputs: Vec<(&String, &String)> = addon_inputs.iter().chain(cmd_inputs.iter()).collect();
  inputs.sort_by(|a, b| a.0.cmp(b.0));
  if inputs.is_empty() {
    ui::kv("  inputs", "(none)");
  } else {
    println!("  inputs:");
    for (k, v) in inputs {
      println!("    {k} = {v}");
    }
  }

  ui::kv("  steps", format!("{} step(s)", plan.steps.len()));
  for (idx, step) in plan.steps.iter().enumerate() {
    print!("    ");
    ui::step(idx, plan.steps.len(), &step.label);
  }
  println!("\nNo files were changed.");
}

fn confirm_addon_execution(
  addon_id: &str,
  command_name: &str,
  steps: &[Step],
  non_interactive: bool,
) -> Result<bool> {
  if non_interactive {
    return Ok(true);
  }
  let (mut writes, mut edits, mut removes) = (0usize, 0usize, 0usize);
  for step in steps {
    match step {
      Step::Create(_) | Step::Copy(_) => writes += 1,
      Step::Inject(_)
      | Step::Replace(_)
      | Step::Append(_)
      | Step::Packages(_)
      | Step::Run(_)
      | Step::JsonPatch(_) => edits += 1,
      Step::Delete(_) | Step::Rename(_) | Step::Move(_) => removes += 1,
    }
  }

  ui::warn(format!(
    "Addon '{addon_id}' command '{command_name}' will modify files in this project \
     ({writes} created/copied, {edits} edited, {removes} deleted/moved)."
  ));
  println!(
    "  Addons run unsandboxed and can overwrite source files or 'package.json'. \
     Only run addons you trust."
  );

  Ok(Confirm::new("Proceed?").with_default(false).prompt()?)
}

fn rerun_prompt_message(
  command_name: &str,
  locked_version: Option<&str>,
  current_version: &str,
) -> Option<String> {
  let locked_version = locked_version.filter(|version| !version.is_empty())?;
  if locked_version == current_version {
    return None;
  }

  Some(format!(
    "Command '{}' was last run with v{} of this add-on. A new version (v{}) is available. Re-run it now?",
    command_name, locked_version, current_version
  ))
}

#[doc(hidden)]
pub fn rerun_prompt_message_for_tests(
  command_name: &str,
  locked_version: Option<&str>,
  current_version: &str,
) -> Option<String> {
  rerun_prompt_message(command_name, locked_version, current_version)
}

pub fn collect_inputs(
  inputs: &[InputDef],
  presets: &HashMap<String, String>,
  non_interactive: bool,
  map: &mut HashMap<String, String>,
) -> Result<()> {
  let mut missing: Vec<&str> = Vec::new();
  for input in inputs {
    if let Some(preset) = presets.get(&input.name) {
      map.insert(input.name.clone(), preset.clone());
      continue;
    }
    if non_interactive {
      match &input.default {
        Some(default) => {
          map.insert(input.name.clone(), default.clone());
        }
        None if input.required => missing.push(&input.name),
        None => {
          let fallback = match input.input_type {
            InputType::Boolean => "false".to_string(),
            _ => String::new(),
          };
          map.insert(input.name.clone(), fallback);
        }
      }
      continue;
    }
    let value = match input.input_type {
      InputType::Text => loop {
        let mut prompt = Text::new(&input.description);
        if let Some(ref default) = input.default {
          prompt = prompt.with_default(default);
        }
        let value = prompt.prompt()?;
        if input.required && value.trim().is_empty() {
          eprintln!("'{}' is required; please enter a value.", input.name);
          continue;
        }
        break value;
      },
      InputType::Boolean => {
        let default = input
          .default
          .as_deref()
          .map(|d| d == "true")
          .unwrap_or(false);
        Confirm::new(&input.description)
          .with_default(default)
          .prompt()?
          .to_string()
      }
      InputType::Select => Select::new(&input.description, input.options.clone())
        .prompt()?
        .to_string(),
    };
    map.insert(input.name.clone(), value);
  }
  if !missing.is_empty() {
    return Err(anyhow!(
      "Missing required input(s) in non-interactive mode: {}. Provide them with --input NAME=VALUE.",
      missing.join(", ")
    ));
  }
  Ok(())
}

fn insert_with_derived(ctx: &mut TemplateContext, map: &HashMap<String, String>) {
  for (k, v) in map {
    ctx.insert(k.as_str(), v);
    ctx.insert(format!("{k}_pascal"), &to_pascal_case(v));
    ctx.insert(format!("{k}_camel"), &to_camel_case(v));
    ctx.insert(format!("{k}_kebab"), &to_kebab_case(v));
    ctx.insert(format!("{k}_snake"), &to_snake_case(v));
  }
}

fn prune_empty_dirs(start: Option<&Path>, project_root: &Path) {
  let canonical_root = project_root
    .canonicalize()
    .unwrap_or_else(|_| project_root.to_path_buf());
  let mut dir = start;
  while let Some(d) = dir {
    let canonical_d = d.canonicalize().unwrap_or_else(|_| d.to_path_buf());
    if canonical_d == canonical_root
      || !canonical_d.starts_with(&canonical_root)
      || fs::remove_dir(d).is_err()
    {
      break;
    }
    dir = d.parent();
  }
}

#[doc(hidden)]
pub fn prune_empty_dirs_for_tests(start: Option<&Path>, project_root: &Path) {
  prune_empty_dirs(start, project_root);
}

pub fn undo_addon(addon_id: &str, project_root: &Path, non_interactive: bool) -> Result<()> {
  let mut lock = LockFile::load(project_root)?;
  let entry = lock
    .addons
    .iter()
    .find(|e| e.id == addon_id)
    .filter(|e| e.has_undoable_changes())
    .ok_or_else(|| {
      anyhow!("Addon '{addon_id}' has no undoable changes recorded in this project.")
    })?;

  let tagged: Vec<(usize, Rollback)> = entry
    .commands
    .iter()
    .enumerate()
    .flat_map(|(ci, cmd)| cmd.journal.iter().map(move |rb| (ci, rb.clone())))
    .collect();

  let conflicts = undo_conflicts(&tagged);

  if !conflicts.is_empty() {
    ui::warn_err(format!(
      "some files changed since '{addon_id}' was applied:"
    ));
    for c in &conflicts {
      eprintln!("  {c}");
    }
  }

  if !non_interactive
    && !Confirm::new(&format!("Undo addon '{addon_id}'?"))
      .with_default(conflicts.is_empty())
      .prompt()?
  {
    return Err(crate::utils::errors::AnesisError::Aborted.into());
  }

  let mut remaining: Vec<(usize, Rollback)> = Vec::new();
  let mut failures = Vec::new();
  let mut applied: Vec<Rollback> = Vec::new();
  for (ci, rollback) in tagged.into_iter().rev() {
    let description = describe_rollback(&rollback);
    if let Err(err) = apply_rollback(rollback.clone(), project_root) {
      failures.push(format!("{description}: {err:#}"));
      remaining.push((ci, rollback));
    } else {
      applied.push(rollback);
    }
  }

  if !failures.is_empty() {
    remaining.reverse();
    let entry = lock.addons.iter_mut().find(|e| e.id == addon_id).unwrap();
    for cmd in &mut entry.commands {
      cmd.journal.clear();
    }
    for (ci, rollback) in remaining {
      entry.commands[ci].journal.push(rollback);
    }
    lock.save(project_root)?;

    ui::warn_err(format!("could not undo every change made by '{addon_id}':"));
    for f in &failures {
      eprintln!("  {f}");
    }
    return Err(anyhow!(
      "Addon '{addon_id}' was partially reverted; {} change(s) remain — fix the issue above and re-run `anesis undo {addon_id}`.",
      failures.len()
    ));
  }

  lock.remove_addon(addon_id);
  lock.save(project_root)?;

  if let Err(err) = AnesisManifest::remove_addon(addon_id, project_root) {
    eprintln!("Note: could not update anesis.json ({err}).");
  }

  ui::success(format!("Reverted addon '{addon_id}'."));
  let summary = super::summary::ChangeSummary::from_rollbacks(&applied);
  if !summary.is_empty() {
    println!("{}", summary.render_block(&applied));
  }
  Ok(())
}

fn is_newer(latest: &str, current: &str) -> bool {
  if latest.trim().is_empty() {
    return false;
  }
  match (
    semver::Version::parse(latest),
    semver::Version::parse(current),
  ) {
    (Ok(l), Ok(c)) => l > c,
    _ => latest != current,
  }
}

#[doc(hidden)]
pub fn is_newer_for_tests(latest: &str, current: &str) -> bool {
  is_newer(latest, current)
}

#[derive(serde::Serialize)]
pub struct OutdatedEntry {
  pub id: String,
  pub current: String,
  pub latest: Option<String>,
  pub outdated: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
}

pub async fn collect_outdated(ctx: &AppContext, project_root: &Path) -> Result<Vec<OutdatedEntry>> {
  let lock = LockFile::load(project_root)?;

  let mut entries = Vec::with_capacity(lock.addons.len());
  for entry in &lock.addons {
    entries.push(match fetch_latest_version(ctx, &entry.id).await {
      Ok(latest) => OutdatedEntry {
        id: entry.id.clone(),
        current: entry.version.clone(),
        outdated: is_newer(&latest, &entry.version),
        latest: Some(latest),
        error: None,
      },
      Err(err) => OutdatedEntry {
        id: entry.id.clone(),
        current: entry.version.clone(),
        latest: None,
        outdated: false,
        error: Some(format!("{err:#}")),
      },
    });
  }
  Ok(entries)
}

pub async fn outdated(ctx: &AppContext, project_root: &Path, json: bool) -> Result<()> {
  let lock = LockFile::load(project_root)?;

  if lock.addons.is_empty() {
    if json {
      println!("[]");
    } else {
      println!("No addons applied in this project.");
    }
    return Ok(());
  }

  let entries = collect_outdated(ctx, project_root).await?;

  if json {
    println!("{}", serde_json::to_string_pretty(&entries)?);
    return Ok(());
  }

  let mut any = false;
  for entry in &entries {
    match (&entry.latest, &entry.error) {
      (Some(latest), _) if entry.outdated => {
        any = true;
        println!(
          "  {} v{} {} v{}",
          ui::accent(&entry.id),
          entry.current,
          ui::symbols::arrow(),
          ui::good(latest)
        );
      }
      (_, Some(err)) => eprintln!("  {} (could not check: {err})", ui::muted(&entry.id)),
      _ => {}
    }
  }

  if any {
    println!("\nRun `anesis update <addon_id>` to upgrade.");
  } else {
    println!("All addons are up to date.");
  }
  Ok(())
}

pub async fn update_addon(
  ctx: &AppContext,
  addon_id: &str,
  project_root: &Path,
  non_interactive: bool,
) -> Result<()> {
  let (current, entry_inputs, command_runs, had_journal) = {
    let lock = LockFile::load(project_root)?;
    let entry = lock
      .addons
      .iter()
      .find(|e| e.id == addon_id)
      .ok_or_else(|| anyhow!("Addon '{addon_id}' is not applied in this project."))?;
    (
      entry.version.clone(),
      entry.inputs.clone(),
      entry
        .commands
        .iter()
        .map(|c| (c.name.clone(), c.inputs.clone()))
        .collect::<Vec<_>>(),
      entry.has_undoable_changes(),
    )
  };

  let latest = fetch_latest_version(ctx, addon_id).await?;
  if !is_newer(&latest, &current) {
    println!("Addon '{addon_id}' is already up to date (v{current}).");
    return Ok(());
  }

  println!("Updating '{addon_id}' v{current} → v{latest}...");

  install_addon(ctx, addon_id).await.with_context(|| {
    format!(
      "Failed to fetch addon '{addon_id}' v{latest}; the currently-applied v{current} is unaffected"
    )
  })?;

  let new_manifest = read_cached_manifest(&ctx.paths.addons, addon_id)?;
  preflight_update(
    &new_manifest,
    project_root,
    &entry_inputs,
    &command_runs,
    ctx.allow_run,
  )
  .with_context(|| {
    format!(
      "addon '{addon_id}' v{latest} cannot be safely re-applied to this project; \
       the currently-applied v{current} is unaffected and nothing was undone"
    )
  })?;

  if had_journal {
    undo_addon(addon_id, project_root, true)?;
  } else {
    let mut lock = LockFile::load(project_root)?;
    lock.remove_addon(addon_id);
    lock.save(project_root)?;
    let _ = AnesisManifest::remove_addon(addon_id, project_root);
  }

  for (cmd, inputs) in &command_runs {
    run_addon_command(
      ctx,
      addon_id,
      cmd,
      project_root,
      inputs,
      non_interactive,
      false,
    )
    .await
    .with_context(|| {
      format!(
        "addon '{addon_id}' was updated to v{latest} and its old files were reverted, but \
         re-applying command '{cmd}' failed; commands before it in this list were re-applied \
         successfully — fix the issue above, then run `anesis use {addon_id} <command>` for \
         any that still need it"
      )
    })?;
  }

  ui::success(format!("Updated '{addon_id}' to v{latest}."));
  Ok(())
}

fn preflight_update(
  manifest: &super::manifest::AddonManifest,
  project_root: &Path,
  entry_inputs: &HashMap<String, String>,
  command_runs: &[(String, HashMap<String, String>)],
  allow_run: bool,
) -> Result<()> {
  let detected_id = detect_variant(&manifest.detect, project_root);

  for (command_name, cmd_inputs) in command_runs {
    let variant = manifest
      .variants
      .iter()
      .find(|v| v.when.as_deref() == detected_id.as_deref())
      .or_else(|| manifest.variants.iter().find(|v| v.when.is_none()))
      .ok_or_else(|| anyhow!("no variant of the new version matches this project anymore"))?;

    let command = variant
      .commands
      .iter()
      .find(|c| &c.name == command_name)
      .ok_or_else(|| anyhow!("command '{command_name}' no longer exists in the new version"))?;

    for req_cmd in &command.requires_commands {
      if !command_runs.iter().any(|(name, _)| name == req_cmd) {
        return Err(anyhow!(
          "command '{command_name}' now requires '{req_cmd}' to run first, which was not \
           previously applied to this project"
        ));
      }
    }

    for input in manifest.inputs.iter().chain(command.inputs.iter()) {
      if input.required
        && input.default.is_none()
        && !entry_inputs.contains_key(&input.name)
        && !cmd_inputs.contains_key(&input.name)
      {
        return Err(anyhow!(
          "command '{command_name}' now requires input '{}', which was not saved for this project",
          input.name
        ));
      }
    }

    let combined_inputs: HashMap<String, String> = entry_inputs
      .iter()
      .chain(cmd_inputs.iter())
      .map(|(k, v)| (k.clone(), v.clone()))
      .collect();
    let steps = effective_steps(&command.steps, &combined_inputs)?;
    let plan = plan_command(&manifest.id, command_name, detected_id.as_deref(), &steps);
    if plan.needs_allow_run() && !allow_run {
      return Err(anyhow!(
        "command '{command_name}' now runs a shell/packages step, which requires --allow-run"
      ));
    }
  }

  Ok(())
}

fn undo_conflicts(tagged: &[(usize, Rollback)]) -> Vec<String> {
  let mut conflicts: Vec<String> = Vec::new();
  for (_, rollback) in tagged {
    match rollback {
      Rollback::DeleteCreatedFile { path } if !path.exists() => {
        conflicts.push(format!("{} (already deleted)", path.display()));
      }
      Rollback::RestoreFile { path, .. } if !path.exists() => {
        conflicts.push(format!("{} (missing)", path.display()));
      }
      Rollback::RenameFile { from, .. } if !from.exists() => {
        conflicts.push(format!("{} (missing)", from.display()));
      }
      _ => {}
    }
  }
  conflicts
}

#[doc(hidden)]
pub fn undo_conflicts_for_tests(tagged: &[(usize, Rollback)]) -> Vec<String> {
  undo_conflicts(tagged)
}

fn describe_rollback(rollback: &Rollback) -> String {
  match rollback {
    Rollback::DeleteCreatedFile { path } => format!("delete {}", path.display()),
    Rollback::RestoreFile { path, .. } => format!("restore {}", path.display()),
    Rollback::RenameFile { from, to } => {
      format!("rename {} back to {}", to.display(), from.display())
    }
    Rollback::IrreversibleRun { command } => format!("(irreversible) {command}"),
  }
}

#[cfg(unix)]
fn restore_symlink(target: &Path, path: &Path) -> Result<()> {
  std::os::unix::fs::symlink(target, path)?;
  Ok(())
}

#[cfg(windows)]
fn restore_symlink(target: &Path, path: &Path) -> Result<()> {
  std::os::windows::fs::symlink_file(target, path)?;
  Ok(())
}

#[cfg(not(any(unix, windows)))]
fn restore_symlink(_target: &Path, _path: &Path) -> Result<()> {
  Err(anyhow!(
    "restoring a symlink is not supported on this platform"
  ))
}

pub fn apply_rollback(rollback: Rollback, project_root: &Path) -> Result<()> {
  match rollback {
    Rollback::DeleteCreatedFile { path } => {
      let _ = std::fs::remove_file(&path);
      prune_empty_dirs(path.parent(), project_root);
    }
    Rollback::RestoreFile {
      path,
      original,
      mode: _mode,
      is_symlink,
    } => {
      if is_symlink {
        let target = String::from_utf8_lossy(&original).into_owned();
        restore_symlink(Path::new(&target), &path)?;
      } else {
        std::fs::write(&path, original)?;
        #[cfg(unix)]
        if let Some(mode) = _mode {
          use std::os::unix::fs::PermissionsExt;
          std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))?;
        }
      }
    }
    Rollback::RenameFile { from, to } => {
      std::fs::rename(&from, to)?;
      prune_empty_dirs(from.parent(), project_root);
    }
    Rollback::IrreversibleRun { command } => {
      ui::warn_err(format!(
        "could not undo shell command '{command}' — its effects remain."
      ));
    }
  }
  Ok(())
}
