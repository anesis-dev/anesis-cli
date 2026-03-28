use std::{
  path::PathBuf,
  sync::{Arc, Mutex},
  time::Duration,
};

use crate::{
  auth::{account::print_user_info, login::login, logout::logout},
  cache::{get_installed_templates, remove_template_from_cache},
  cli::{Cli, commands::Commands},
  paths::OxidePaths,
  templates::{
    generator::extract_template, install::install_template, loader::get_files, publish::publish,
  },
  utils::{
    cleanup::setup_ctrlc_handler,
    validate::{is_valid_github_repo_url, validate_project_name},
  },
};
use anyhow::Result;
use clap::Parser;
use reqwest::Client;

mod auth;
mod cache;
mod cli;
mod config;
mod paths;
mod templates;
mod utils;

const BACKEND_URL: &str = "https://oxide-server.onrender.com";
const FRONTEND_URL: &str = "https://oxide-cli.vercel.app";

pub struct AppContext {
  pub paths: OxidePaths,
  pub client: Client,
  pub cleanup_state: CleanupState,
}

type CleanupState = Arc<Mutex<Option<PathBuf>>>;

#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();
  let oxide_paths = OxidePaths::new()?;
  oxide_paths.ensure_directories()?;
  let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
  let cleanup_state: CleanupState = Arc::new(Mutex::new(None));

  setup_ctrlc_handler(cleanup_state.clone(), oxide_paths.templates.clone())?;

  let ctx = AppContext {
    paths: oxide_paths,
    client,
    cleanup_state,
  };

  match cli.command {
    Commands::New {
      name,
      template_name,
    } => {
      validate_project_name(&name)?;

      create_new_project(&ctx, &name, &template_name).await?
    }
    Commands::InstallTemplate { template_name } => install_template(&ctx, &template_name).await?,
    Commands::Delete { template_name } => {
      remove_template_from_cache(&ctx.paths.templates, &template_name)?;
    }
    Commands::Installed => get_installed_templates(&ctx.paths.templates)?,
    Commands::Login => {
      login(&ctx.paths.auth).await?;
    }
    Commands::Logout => {
      logout(&ctx.paths.auth)?;
    }
    Commands::Account => {
      print_user_info(&ctx).await?;
    }
    Commands::PublishTemplate { template_url } => {
      is_valid_github_repo_url(&template_url)?;
      publish(&ctx, &template_url).await?;
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
