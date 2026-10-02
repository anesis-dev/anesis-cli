use anesis::addons::catalog::{addon_pick_items, fetch_addon_catalog};
use anesis::{
  addons,
  auth::{account::print_user_info, login::login, logout::logout},
  cli::{
    self,
    commands::{AddonCommands, Commands, StackCommands, TemplateCommands},
    dispatch::{parse_inputs, skip_version_notice},
  },
  completions, config,
  context::AppContext,
  info::print_info,
  stacks::registry::fetch_stack_catalog,
  templates::{
    cache::{get_installed_templates, remove_template_from_cache},
    catalog::{fetch_catalog, template_pick_items},
    generator::{extract_template, overwritten_paths},
    install::{InstallResult, install_template, record_template_use},
    link::link_template,
    loader::get_files,
    publish::publish,
    republish::republish,
  },
  upgrade::{check_cli_version_cached, render_upgrade_notice, upgrade_cli},
  utils::{
    errors::{AnesisError, exit_code_for, print_error},
    picker::{self, ItemKind, PickItem, pick_one},
    ui::{self, spinner},
    validate::{
      ensure_destination_available, is_valid_github_repo_url, validate_project_name,
      validate_template_name,
    },
  },
};
use anyhow::Result;
use inquire::Confirm;

fn install_panic_hook() {
  let default_hook = std::panic::take_hook();

  std::panic::set_hook(Box::new(move |info| {
    picker::restore_terminal();

    if std::env::var("ANESIS_DEBUG").is_ok() {
      default_hook(info);
      return;
    }

    let location = info
      .location()
      .map(|l| format!("{}:{}", l.file(), l.line()))
      .unwrap_or_else(|| "unknown location".to_string());

    let message = info
      .payload()
      .downcast_ref::<&str>()
      .map(|s| (*s).to_string())
      .or_else(|| info.payload().downcast_ref::<String>().cloned())
      .unwrap_or_else(|| "unknown cause".to_string());

    eprintln!();
    ui::error_header("anesis crashed. This is a bug.");
    eprintln!();
    eprintln!("  {message}");
    eprintln!("  at {location}");
    eprintln!(
      "  anesis {} on {}",
      env!("CARGO_PKG_VERSION"),
      std::env::consts::OS
    );
    eprintln!();
    ui::labeled_line(
      "report it",
      "https://github.com/anesis-dev/anesis-cli/issues/new",
    );
    eprintln!("  Re-run with ANESIS_DEBUG=1 for a full backtrace to attach.");
  }));
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
  install_panic_hook();
  config::init_env();
  completions::complete_env();
  if let Err(err) = run().await {
    print_error(&err);
    std::process::exit(exit_code_for(&err));
  }
}

async fn run() -> Result<()> {
  let cli = cli::parse();

  config::init_logging(cli.verbose);
  if cli.no_color {
    colored::control::set_override(false);
  }
  anesis::utils::ui::set_quiet(cli.quiet);
  anesis::utils::ui::init(cli.ascii);

  let ctx = config::build_app_context()?.with_cli_flags(cli.no_telemetry, cli.allow_run);

  let skip_version_notice = skip_version_notice(&cli.command, cli.quiet);
  let version_check_handle = if skip_version_notice {
    None
  } else {
    let client = ctx.client.clone();
    let version_check_path = ctx.paths.version_check.clone();
    Some(tokio::spawn(async move {
      check_cli_version_cached(&client, &version_check_path).await
    }))
  };

  match cli.command {
    Commands::New {
      name,
      template_name,
      stack,
      installed,
      yes,
      overwrite,
      input,
      dry_run,
    } => {
      validate_project_name(&name)?;
      ensure_destination_available(&name, overwrite)?;
      let inputs = parse_inputs(&input)?;
      if let Some(stack_ref) = stack {
        let stack = anesis::stacks::cache::resolve_stack(&ctx, &stack_ref).await?;
        apply_stack(&ctx, &name, &stack, yes, overwrite, dry_run, &inputs).await?;
      } else {
        let template_name = match template_name {
          Some(template_name) => template_name,
          None => choose_template(&ctx, installed, "Select a template").await?,
        };
        validate_template_name(&template_name)?;
        create_new_project(
          &ctx,
          &name,
          &template_name,
          yes,
          overwrite,
          dry_run,
          &inputs,
        )
        .await?;
      }
    }
    Commands::Template { command } => match command {
      TemplateCommands::Install { template_name } => {
        let template_name = match template_name {
          Some(name) => name,
          None => choose_template(&ctx, false, "Select a template to install").await?,
        };
        validate_template_name(&template_name)?;
        let install_result = install_template(&ctx, &template_name).await?;
        match install_result {
          InstallResult::UpToDate => {
            println!("{}", InstallResult::up_to_date_message(&template_name));
          }
          _ => {
            if let Some(message) = install_result.message(&template_name) {
              println!("{message}");
            }
          }
        }
      }
      TemplateCommands::Link { path, force } => {
        let source = std::path::PathBuf::from(path.as_deref().unwrap_or("."));
        match link_template(&ctx, &source, force)? {
          Some(name) => {
            ui::success(format!("Template '{name}' cached locally."));
            ui::hint("Try it", format!("anesis new <dir> {name}"));
          }
          None => println!("Aborted. The cached template was left unchanged."),
        }
      }
      TemplateCommands::List { json } => {
        if json {
          let templates = anesis::templates::cache::read_installed_templates(&ctx.paths.templates)?;
          println!("{}", serde_json::to_string_pretty(&templates)?);
        } else {
          get_installed_templates(&ctx.paths.templates)?;
        }
      }
      TemplateCommands::Info {
        template_name,
        json,
      } => {
        anesis::templates::info::template_info(&ctx, &template_name, json).await?;
      }
      TemplateCommands::Remove { template_name } => {
        remove_template_from_cache(&ctx.paths.templates, &template_name)?;
      }
      TemplateCommands::Publish {
        template_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&template_url)?;
        publish(&ctx, &template_url, visibility, credential_id, org_id).await?;
      }
      TemplateCommands::Republish {
        template_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&template_url)?;
        republish(&ctx, &template_url, visibility, credential_id, org_id).await?;
      }
    },
    Commands::Login => {
      login(&ctx.paths.auth, &ctx.backend_url, &ctx.frontend_url).await?;
    }
    Commands::Logout => {
      logout(&ctx.paths.auth)?;
    }
    Commands::Account { json } => {
      if json {
        let user = anesis::auth::account::get_user_info(&ctx).await?;
        println!("{}", serde_json::to_string_pretty(&user)?);
      } else {
        print_user_info(&ctx).await?;
      }
    }
    Commands::Addon { command } => match command {
      AddonCommands::Install { addon_id } => {
        let addon_id = match addon_id {
          Some(id) => id,
          None => choose_addon(&ctx, false, "Select an addon to install").await?,
        };
        let install_result = addons::install::install_addon(&ctx, &addon_id).await?;
        match &install_result {
          addons::install::AddonInstallResult::UpToDate(_) => {
            println!(
              "{}",
              addons::install::AddonInstallResult::up_to_date_message(&addon_id)
            );
          }
          _ => {
            if let Some(message) = install_result.message(&addon_id) {
              println!("{message}");
            }
          }
        }
      }
      AddonCommands::Link { path, force } => {
        let source = std::path::PathBuf::from(path.as_deref().unwrap_or("."));
        match addons::link::link_addon(&ctx, &source, force)? {
          Some(id) => {
            ui::success(format!("Addon '{id}' cached locally."));
            ui::hint("Try it", format!("anesis use {id} <command>"));
          }
          None => println!("Aborted. The cached addon was left unchanged."),
        }
      }
      AddonCommands::List { json } => {
        if json {
          let cache = addons::cache::read_cache(&ctx.paths.addons)?;
          println!("{}", serde_json::to_string_pretty(&cache.addons)?);
        } else {
          addons::cache::get_installed_addons(&ctx.paths.addons)?;
        }
      }
      AddonCommands::Info { addon_id, json } => {
        addons::info::addon_info(&ctx, &addon_id, json).await?;
      }
      AddonCommands::Test {
        addon_id,
        command,
        project,
      } => {
        addons::test::test_addon(&ctx, &addon_id, &command, project).await?;
      }
      AddonCommands::Remove { addon_id } => {
        addons::cache::remove_addon_from_cache(&ctx.paths.addons, &addon_id)?;
      }
      AddonCommands::Lint { path } => {
        let dir = std::path::PathBuf::from(path.as_deref().unwrap_or("."));
        let errors = addons::lint::lint_addon(&dir)?;
        if errors.is_empty() {
          ui::success("No issues found.");
        } else {
          for error in &errors {
            ui::failure(error);
          }
          anyhow::bail!("{} issue(s) found in {}", errors.len(), dir.display());
        }
      }
      AddonCommands::Publish {
        addon_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&addon_url)?;
        addons::publish::publish_addon(&ctx, &addon_url, visibility, credential_id, org_id).await?;
      }
      AddonCommands::Republish {
        addon_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&addon_url)?;
        addons::republish::republish_addon(&ctx, &addon_url, visibility, credential_id, org_id)
          .await?;
      }
    },
    Commands::Stack { command } => match command {
      StackCommands::Install { stack_id } => {
        anesis::stacks::cache::install_stack(&ctx, &stack_id).await?;
        ui::success(format!("Stack '{stack_id}' installed."));
        ui::hint(
          "Scaffold it",
          format!("anesis new <dir> --stack {stack_id}"),
        );
      }
      StackCommands::Link { path, force } => {
        let source = std::path::PathBuf::from(path.as_deref().unwrap_or("."));
        match anesis::stacks::link::link_stack(&ctx, &source, force)? {
          Some(id) => {
            ui::success(format!("Stack '{id}' cached locally."));
            ui::hint("Try it", format!("anesis new <dir> --stack {id}"));
          }
          None => println!("Aborted. The cached stack was left unchanged."),
        }
      }
      StackCommands::List { json } => {
        anesis::stacks::info::print_installed_stacks(&ctx, json)?;
      }
      StackCommands::Info { stack_id, json } => {
        anesis::stacks::info::stack_info(&ctx, &stack_id, json).await?;
      }
      StackCommands::Remove { stack_id } => {
        anesis::stacks::cache::remove_cached_stack(&ctx, &stack_id)?;
      }
      StackCommands::Publish {
        stack_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&stack_url)?;
        anesis::stacks::publish::publish_stack(
          &ctx,
          &stack_url,
          false,
          visibility,
          credential_id,
          org_id,
        )
        .await?;
      }
      StackCommands::Republish {
        stack_url,
        visibility,
        credential_id,
        org_id,
      } => {
        is_valid_github_repo_url(&stack_url)?;
        anesis::stacks::publish::publish_stack(
          &ctx,
          &stack_url,
          true,
          visibility,
          credential_id,
          org_id,
        )
        .await?;
      }
    },
    Commands::Use {
      addon_id,
      command,
      installed,
      yes,
      input,
      dry_run,
      diff,
    } => {
      let project_root = std::env::current_dir()?;
      let presets = parse_inputs(&input)?;
      let addon_id = match addon_id {
        Some(id) => id,
        None => choose_addon(&ctx, installed, "Select an addon").await?,
      };
      match command {
        Some(command_name) if diff => {
          use anesis::utils::fs::copy_dir_respecting_gitignore;

          let scratch = tempfile::Builder::new()
            .prefix("anesis-use-diff-")
            .tempdir()?;
          copy_dir_respecting_gitignore(&project_root, scratch.path())?;
          addons::runner::run_addon_command(
            &ctx,
            &addon_id,
            &command_name,
            scratch.path(),
            &presets,
            true,
            false,
          )
          .await?;
          println!();
          addons::diff::show_diff(&project_root, scratch.path());
        }
        Some(command_name) => {
          addons::runner::run_addon_command(
            &ctx,
            &addon_id,
            &command_name,
            &project_root,
            &presets,
            yes,
            dry_run,
          )
          .await?;
        }
        None if diff => {
          anyhow::bail!(
            "--diff requires a command; pass one explicitly, e.g. `anesis use {addon_id} <command> --diff`."
          );
        }
        None => {
          addons::runner::list_addon_commands(
            &ctx,
            &addon_id,
            &project_root,
            &presets,
            yes,
            dry_run,
          )
          .await?;
        }
      }
    }
    Commands::Undo { addon_id, yes } => {
      let project_root = std::env::current_dir()?;
      addons::runner::undo_addon(&addon_id, &project_root, yes)?;
    }
    Commands::Outdated { json } => {
      let project_root = std::env::current_dir()?;
      addons::runner::outdated(&ctx, &project_root, json).await?;
    }
    Commands::Update { addon_id, yes, all } => {
      let project_root = std::env::current_dir()?;
      match (addon_id, all) {
        (Some(_), true) => {
          anyhow::bail!("Pass either an addon id or --all, not both.");
        }
        (Some(addon_id), false) => {
          addons::runner::update_addon(&ctx, &addon_id, &project_root, yes).await?;
        }
        (None, true) => {
          let outdated = addons::runner::collect_outdated(&ctx, &project_root).await?;
          let ids: Vec<String> = outdated
            .iter()
            .filter(|e| e.outdated)
            .map(|e| e.id.clone())
            .collect();
          if ids.is_empty() {
            println!("All addons are up to date.");
          } else {
            for id in &ids {
              addons::runner::update_addon(&ctx, id, &project_root, yes).await?;
            }
            ui::success(format!("Updated {} addon(s).", ids.len()));
          }
        }
        (None, false) => {
          anyhow::bail!("Pass an addon id, or --all to update every outdated addon.");
        }
      }
    }
    Commands::Upgrade => {
      upgrade_cli(&ctx).await?;
    }
    Commands::Mcp => {
      anesis::mcp::run_mcp()?;
    }
    Commands::Man { dir } => {
      let dir = std::path::PathBuf::from(dir);
      let pages = anesis::man::generate(&dir)?;
      println!("Wrote {} man pages to {}", pages.len(), dir.display());
    }
    Commands::Completions { shell, print } => {
      if print {
        completions::print_completions(shell)?;
      } else {
        completions::install_completions(shell)?;
      }
    }
    Commands::Search { query, json } => {
      let sp = spinner("Loading registry...");
      let (templates, addons, stacks) = tokio::join!(
        fetch_catalog(&ctx),
        fetch_addon_catalog(&ctx),
        fetch_stack_catalog(&ctx)
      );
      sp.finish_and_clear();

      let mut items: Vec<PickItem> = templates?.iter().map(|t| t.to_pick_item()).collect();
      items.extend(addons?.iter().map(|a| a.to_pick_item()));
      items.extend(stacks.unwrap_or_default().iter().map(|s| s.to_pick_item()));

      if json {
        let results = picker::search_results_json(&items, query.as_deref());
        println!("{}", serde_json::to_string_pretty(&results)?);
        return Ok(());
      }

      if items.is_empty() {
        println!("The registry is empty.");
      } else {
        let seed = query.unwrap_or_default();
        match pick_one(items, "Search the registry", true, seed).await? {
          Some((ItemKind::Template, id, name)) => {
            println!("{} {}", ui::kind_tag("template"), name);
            ui::hint("scaffold it", format!("anesis new <dir> {id}"));
          }
          Some((ItemKind::Addon, id, name)) => {
            println!("{} {}", ui::kind_tag("addon"), name);
            ui::hint("install", format!("anesis addon install {id}"));
            ui::hint("run", format!("anesis use {id} <command>"));
          }
          Some((ItemKind::Stack, id, name)) => {
            println!("{} {}", ui::kind_tag("stack"), name);
            ui::hint("scaffold it", format!("anesis new <dir> --stack {id}"));
          }
          None => {}
        }
      }
    }
    Commands::Info { json } => {
      if json {
        println!(
          "{}",
          serde_json::to_string_pretty(&anesis::info::info_json(&ctx))?
        );
      } else {
        print_info(&ctx);
      }
    }
    Commands::Status { json } => {
      let project_root = std::env::current_dir()?;
      if json {
        println!(
          "{}",
          serde_json::to_string_pretty(&anesis::status::status_json(&project_root)?)?
        );
      } else {
        anesis::status::print_status(&project_root)?;
      }
    }
    Commands::Doctor { json } => {
      let project_root = std::env::current_dir()?;
      let checks = anesis::doctor::run_checks(&ctx, &project_root).await;
      if json {
        println!(
          "{}",
          serde_json::to_string_pretty(&anesis::doctor::doctor_json(&checks))?
        );
      } else {
        anesis::doctor::print_checks(&checks);
      }
      if anesis::doctor::has_failure(&checks) {
        std::process::exit(anesis::utils::errors::exit_code::FAILURE);
      }
    }
    Commands::Why { path, json } => {
      let project_root = std::env::current_dir()?;
      anesis::why::why(&project_root, path.as_deref(), json)?;
    }
  }

  if let Some(version_check_handle) = version_check_handle
    && let Ok(Ok(Some(latest_version))) = version_check_handle.await
  {
    println!("{}", render_upgrade_notice(&latest_version));
  }

  Ok(())
}

async fn choose_template(ctx: &AppContext, installed: bool, title: &'static str) -> Result<String> {
  let items = template_pick_items(ctx, installed).await?;
  if items.is_empty() {
    anyhow::bail!(if installed {
      "No templates are installed yet"
    } else {
      "No templates are available yet"
    });
  }
  match pick_one(items, title, false, String::new()).await? {
    Some((_, id, _)) => Ok(id),
    None => anyhow::bail!("Selection cancelled"),
  }
}

async fn choose_addon(ctx: &AppContext, installed: bool, title: &'static str) -> Result<String> {
  let items = addon_pick_items(ctx, installed).await?;
  if items.is_empty() {
    anyhow::bail!(if installed {
      "No addons are installed yet"
    } else {
      "No addons are available yet"
    });
  }
  match pick_one(items, title, false, String::new()).await? {
    Some((_, id, _)) => Ok(id),
    None => anyhow::bail!("Selection cancelled"),
  }
}

async fn apply_stack(
  ctx: &AppContext,
  project_name: &str,
  stack: &anesis::stacks::manifest::StackManifest,
  yes: bool,
  overwrite: bool,
  dry_run: bool,
  inputs: &std::collections::HashMap<String, String>,
) -> Result<()> {
  println!("Creating '{project_name}' from stack '{}'...", stack.name);
  create_new_project(
    ctx,
    project_name,
    &stack.template,
    yes,
    overwrite,
    dry_run,
    inputs,
  )
  .await?;
  if dry_run {
    return Ok(());
  }

  let project_root = if project_name == "." {
    std::env::current_dir()?
  } else {
    std::env::current_dir()?.join(project_name)
  };

  let total = stack.addons.len();
  for (idx, addon) in stack.addons.iter().enumerate() {
    println!();
    ui::step(idx, total, format!("addon {}", ui::accent(&addon.id)));
    addons::runner::run_addon_command(
      ctx,
      &addon.id,
      &addon.command,
      &project_root,
      &addon.inputs,
      yes,
      false,
    )
    .await
    .map_err(|err| {
      anyhow::anyhow!(
        "stack '{}' failed while applying addon '{}': {err:#}",
        stack.id,
        addon.id
      )
    })?;
  }

  println!();
  ui::success(format!("Stack '{}' applied.", stack.name));
  Ok(())
}

async fn create_new_project(
  ctx: &AppContext,
  project_name: &str,
  template_name: &str,
  yes: bool,
  overwrite: bool,
  dry_run: bool,
  presets: &std::collections::HashMap<String, String>,
) -> Result<()> {
  let files = get_files(ctx, template_name).await?;

  let mut inputs = std::collections::HashMap::new();
  let mut excluded = std::collections::HashSet::new();
  if let Some(manifest) = anesis::templates::generator::parse_template_manifest(&files) {
    anesis::compat::check_anesis_version(template_name, &manifest.anesis_version)?;
    addons::runner::collect_inputs(&manifest.inputs, presets, yes, &mut inputs)?;
    excluded = anesis::templates::generator::excluded_paths(&manifest.exclude, &inputs);
  }

  let cwd = std::env::current_dir()?;
  let output_path = if project_name == "." {
    cwd
  } else {
    cwd.join(project_name)
  };

  let overwrites = overwritten_paths(&files, &output_path, &excluded)?;

  if dry_run {
    print_new_dry_run_plan(
      &files,
      &output_path,
      &excluded,
      &overwrites,
      project_name,
      template_name,
    );
    return Ok(());
  }

  if !overwrites.is_empty() && !overwrite {
    ui::warn(format!(
      "Generating here will overwrite {} existing file(s):",
      overwrites.len()
    ));
    for path in overwrites.iter().take(20) {
      println!("  {}", path.display());
    }
    if overwrites.len() > 20 {
      println!("  ...and {} more", overwrites.len() - 20);
    }
    if yes {
      anyhow::bail!(
        "Refusing to overwrite {} existing file(s) non-interactively. Pass --overwrite to allow it.",
        overwrites.len()
      );
    }
    if !Confirm::new("Continue and overwrite these files?")
      .with_default(false)
      .prompt()?
    {
      return Err(AnesisError::Aborted.into());
    }
  }

  if project_name != "." {
    println!("Generating project '{project_name}' from template '{template_name}'...");
  } else {
    println!("Generating project from template '{template_name}'...");
  }

  extract_template(&files, &output_path, project_name, ctx, &inputs, &excluded)?;

  let sp = spinner("Finishing up...");
  record_template_use(ctx, template_name).await;
  sp.finish_and_clear();

  if project_name != "." {
    ui::success(format!("Project '{project_name}' created successfully!"));
    println!("\nNext steps:");
    println!("  cd {}", project_name);
  } else {
    ui::success("Project created successfully!");
  }
  Ok(())
}

fn print_new_dry_run_plan(
  files: &[anesis::templates::TemplateFile],
  output_path: &std::path::Path,
  excluded: &std::collections::HashSet<std::path::PathBuf>,
  overwrites: &[std::path::PathBuf],
  project_name: &str,
  template_name: &str,
) {
  use anesis::templates::generator::output_relative_path;

  println!(
    "{} new '{project_name}' from '{}'",
    ui::bold("Dry run:"),
    ui::accent(template_name)
  );

  let overwrite_set: std::collections::HashSet<&std::path::Path> =
    overwrites.iter().map(|p| p.as_path()).collect();

  let mut entries: Vec<(std::path::PathBuf, bool)> = Vec::new();
  for file in files {
    let Some(rel) = output_relative_path(file) else {
      continue;
    };
    if excluded.contains(&rel) {
      continue;
    }
    let overwrite = overwrite_set.contains(output_path.join(&rel).as_path());
    entries.push((rel, overwrite));
  }
  entries.sort();

  let root_label = if project_name == "." {
    ".".to_string()
  } else {
    project_name.to_string()
  };
  let tree = build_file_tree(&root_label, &entries);
  println!("{}", anesis::utils::ui::tree::render(&tree));

  if overwrites.is_empty() {
    println!("\nNo files were written.");
  } else {
    println!(
      "\n{} file(s) marked (overwrite) already exist and were not written.",
      overwrites.len()
    );
  }
}

#[derive(Default)]
struct DirNode {
  children: std::collections::BTreeMap<String, DirNode>,
  is_file: bool,
  overwrite: bool,
}

fn build_file_tree(
  root_label: &str,
  entries: &[(std::path::PathBuf, bool)],
) -> anesis::utils::ui::tree::TreeNode {
  let mut root = DirNode::default();
  for (path, overwrite) in entries {
    let comps: Vec<String> = path
      .components()
      .map(|c| c.as_os_str().to_string_lossy().into_owned())
      .collect();
    let mut node = &mut root;
    for (i, comp) in comps.iter().enumerate() {
      node = node.children.entry(comp.clone()).or_default();
      if i == comps.len() - 1 {
        node.is_file = true;
        node.overwrite = *overwrite;
      }
    }
  }

  fn to_tree(name: &str, dir: &DirNode) -> anesis::utils::ui::tree::TreeNode {
    let label = if dir.is_file && dir.children.is_empty() && dir.overwrite {
      format!("{name} {}", anesis::utils::ui::yellow("(overwrite)"))
    } else {
      name.to_string()
    };
    let mut node = anesis::utils::ui::tree::TreeNode::new(label);
    for (child_name, child_dir) in &dir.children {
      node = node.child(to_tree(child_name, child_dir));
    }
    node
  }

  let mut root_node = anesis::utils::ui::tree::TreeNode::new(root_label);
  for (name, dir) in &root.children {
    root_node = root_node.child(to_tree(name, dir));
  }
  root_node
}
