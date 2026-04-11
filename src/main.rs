use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use oxide_cli::{
  AppContext, CleanupState, addons, completions,
  auth::{account::print_user_info, login::login, logout::logout},
  cache::{get_installed_templates, remove_template_from_cache},
  cli::{
    Cli,
    commands::{AddonCommands, Commands, TemplateCommands},
  },
  paths::OxidePaths,
  templates::{
    generator::extract_template, install::install_template, loader::get_files, publish::publish,
    update::update,
  },
  utils::{
    cleanup::setup_ctrlc_handler,
    validate::{is_valid_github_repo_url, validate_project_name, validate_template_name},
  },
};
use reqwest::Client;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();
  let oxide_paths = OxidePaths::new()?;
  oxide_paths.ensure_directories()?;
  let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
  let cleanup_state: CleanupState = Arc::new(Mutex::new(None));

  setup_ctrlc_handler(cleanup_state.clone(), oxide_paths.templates.clone())?;

  let ctx = AppContext::new(oxide_paths, client, cleanup_state);

  match cli.command {
    Commands::New {
      name,
      template_name,
    } => {
      validate_project_name(&name)?;
      create_new_project(&ctx, &name, &template_name).await?;
    }
    Commands::Template { command } => match command {
      TemplateCommands::Install { template_name } => {
        validate_template_name(&template_name)?;
        install_template(&ctx, &template_name).await?;
      }
      TemplateCommands::List => {
        get_installed_templates(&ctx.paths.templates)?;
      }
      TemplateCommands::Remove { template_name } => {
        remove_template_from_cache(&ctx.paths.templates, &template_name)?;
      }
      TemplateCommands::Publish { template_url } => {
        is_valid_github_repo_url(&template_url)?;
        publish(&ctx, &template_url).await?;
      }
      TemplateCommands::Update { template_url } => {
        update(&ctx, &template_url).await?;
      }
    },
    Commands::Login => {
      login(&ctx.paths.auth, &ctx.backend_url, &ctx.frontend_url).await?;
    }
    Commands::Logout => {
      logout(&ctx.paths.auth)?;
    }
    Commands::Account => {
      print_user_info(&ctx).await?;
    }
    Commands::Addon { command } => match command {
      AddonCommands::Install { addon_id } => {
        addons::install::install_addon(&ctx, &addon_id).await?;
      }
      AddonCommands::List => {
        addons::cache::get_installed_addons(&ctx.paths.addons)?;
      }
      AddonCommands::Remove { addon_id } => {
        addons::cache::remove_addon_from_cache(&ctx.paths.addons, &addon_id)?;
      }
      AddonCommands::Publish { addon_url } => {
        is_valid_github_repo_url(&addon_url)?;
        addons::publish::publish_addon(&ctx, &addon_url).await?;
      }
      AddonCommands::Update { addon_url } => {
        is_valid_github_repo_url(&addon_url)?;
        addons::update::update_addon(&ctx, &addon_url).await?;
      }
    },
    Commands::Completions { shell } => {
      completions::install_completions(&shell)?;
    }
    Commands::Complete { addon_id } => {
      completions::print_dynamic_completions(&ctx.paths.addons, addon_id.as_deref());
    }
    Commands::External(args) => {
      let addon_id = &args[0];
      let command_name = args.get(1).context("Usage: oxide <addon-id> <command>")?;
      let project_root = std::env::current_dir()?;
      addons::runner::run_addon_command(&ctx, addon_id, command_name, &project_root).await?;
    }
  }

  Ok(())
}

async fn create_new_project(
  ctx: &AppContext,
  project_name: &str,
  template_name: &str,
) -> Result<()> {
  let files = get_files(ctx, template_name).await?;
  extract_template(&files, project_name)?;
  println!("✅ Project created successfully!");
  println!("\nNext steps:");
  println!("  cd {}", project_name);
  Ok(())
}
