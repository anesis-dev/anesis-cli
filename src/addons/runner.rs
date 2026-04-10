use std::{collections::HashMap, path::Path};

use anyhow::{anyhow, Result};
use inquire::{Confirm, Select, Text};

use crate::AppContext;

use super::{
  cache::{get_cached_addon, is_addon_installed},
  detect::detect_variant,
  install::{install_addon, read_cached_manifest},
  lock::{LockEntry, LockFile},
  manifest::{AddonManifest, InputType},
  steps::{
    Rollback,
    append::execute_append,
    copy::execute_copy,
    create::execute_create,
    delete::execute_delete,
    inject::execute_inject,
    move_step::execute_move,
    rename::execute_rename,
    replace::execute_replace,
  },
};
use crate::addons::manifest::Step;

pub async fn run_addon_command(
  ctx: &AppContext,
  addon_id: &str,
  command_name: &str,
  project_root: &Path,
) -> Result<()> {
  // 1. Load manifest
  let manifest: AddonManifest = if get_cached_addon(&ctx.paths.addons, addon_id)?.is_some() {
    read_cached_manifest(&ctx.paths.addons, addon_id)?
  } else {
    install_addon(ctx, addon_id).await?
  };

  // 2. Load lock file
  let mut lock = LockFile::load(project_root)?;

  // 3. Check `once`
  // (deferred until we find the command below, after variant detection)

  // 4. Check addon deps (`requires`)
  for dep_id in &manifest.requires {
    if !is_addon_installed(&ctx.paths.addons, dep_id)? {
      return Err(anyhow!(
        "Addon '{}' requires '{}' to be installed first. Run: oxide addon install {}",
        addon_id,
        dep_id,
        dep_id
      ));
    }
  }

  // 5. Collect inputs
  let mut input_values: HashMap<String, String> = HashMap::new();
  for input in &manifest.inputs {
    let value = match input.input_type {
      InputType::Text => {
        let mut prompt = Text::new(&input.description);
        if let Some(ref default) = input.default {
          prompt = prompt.with_default(default);
        }
        prompt.prompt()?
      }
      InputType::Boolean => {
        let default = input
          .default
          .as_deref()
          .map(|d| d == "true")
          .unwrap_or(false);
        let answer = Confirm::new(&input.description).with_default(default).prompt()?;
        answer.to_string()
      }
      InputType::Select => {
        Select::new(&input.description, input.options.clone()).prompt()?.to_string()
      }
    };
    input_values.insert(input.name.clone(), value);
  }

  // 6. Build Tera context
  let mut tera_ctx = tera::Context::new();
  for (k, v) in &input_values {
    tera_ctx.insert(k, v);
  }

  // 7. Detect variant
  let detected_id = detect_variant(&manifest.detect, project_root);

  // 8. Select variant
  let variant = manifest
    .variants
    .iter()
    .find(|v| v.when.as_deref() == detected_id.as_deref())
    .or_else(|| manifest.variants.iter().find(|v| v.when.is_none()))
    .ok_or_else(|| anyhow!("No matching variant found for addon '{}'", addon_id))?;

  // 9. Find command
  let command = variant
    .commands
    .iter()
    .find(|c| c.name == command_name)
    .ok_or_else(|| anyhow!("Command '{}' not found in addon '{}'", command_name, addon_id))?;

  // 3. Check `once` (now that we have the command)
  if command.once && lock.is_command_executed(addon_id, command_name) {
    println!("Command '{}' has already been executed, skipping.", command_name);
    return Ok(());
  }

  // 4b. Check `requires_commands`
  for req_cmd in &command.requires_commands {
    if !lock.is_command_executed(addon_id, req_cmd) {
      return Err(anyhow!(
        "Command '{}' requires '{}' to be run first. Run: oxide addon run {} {} {}",
        command_name,
        req_cmd,
        addon_id,
        req_cmd,
        project_root.display()
      ));
    }
  }

  // 10. Execute steps
  let addon_dir = ctx.paths.addons.join(addon_id);
  let mut completed_rollbacks: Vec<Rollback> = Vec::new();

  for (idx, step) in command.steps.iter().enumerate() {
    let result = match step {
      Step::Copy(s) => execute_copy(s, &addon_dir, project_root),
      Step::Create(s) => execute_create(s, project_root, &tera_ctx),
      Step::Inject(s) => execute_inject(s, project_root, &tera_ctx),
      Step::Replace(s) => execute_replace(s, project_root, &tera_ctx),
      Step::Append(s) => execute_append(s, project_root, &tera_ctx),
      Step::Delete(s) => execute_delete(s, project_root),
      Step::Rename(s) => execute_rename(s, project_root),
      Step::Move(s) => execute_move(s, project_root),
    };

    match result {
      Ok(rollbacks) => completed_rollbacks.extend(rollbacks),
      Err(err) => {
        eprintln!("Step {} failed: {}", idx + 1, err);
        let choice = Select::new(
          "How would you like to proceed?",
          vec!["Keep changes made so far", "Rollback all changes"],
        )
        .prompt()?;

        if choice == "Rollback all changes" {
          for rollback in completed_rollbacks.into_iter().rev() {
            let _ = apply_rollback(rollback);
          }
        }

        return Err(anyhow!("Addon command failed at step {}: {}", idx + 1, err));
      }
    }
  }

  // 11. Update lock
  lock.mark_command_executed(addon_id, command_name);
  let variant_id = detected_id.unwrap_or_else(|| "universal".to_string());
  lock.upsert_entry(LockEntry {
    id: addon_id.to_string(),
    version: manifest.version.clone(),
    variant: variant_id,
    commands_executed: lock
      .addons
      .iter()
      .find(|e| e.id == addon_id)
      .map(|e| e.commands_executed.clone())
      .unwrap_or_default(),
  });
  lock.save(project_root)?;

  println!("✓ Command '{}' completed successfully.", command_name);
  Ok(())
}

fn apply_rollback(rollback: Rollback) -> Result<()> {
  match rollback {
    Rollback::DeleteCreatedFile { path } => {
      let _ = std::fs::remove_file(path);
    }
    Rollback::RestoreFile { path, original } => {
      std::fs::write(path, original)?;
    }
    Rollback::RenameFile { from, to } => {
      std::fs::rename(from, to)?;
    }
  }
  Ok(())
}
