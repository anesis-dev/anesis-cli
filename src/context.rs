use std::{
  path::PathBuf,
  sync::{Arc, Mutex},
};

use reqwest::Client;

use crate::{addons::steps::Rollback, paths::AnesisPaths};

#[derive(Clone)]
pub enum CleanupTask {
  PartialDownload {
    path: PathBuf,
    prune_root: PathBuf,
    label: &'static str,
  },
  PartialProject {
    path: PathBuf,
  },
  PartialProjectFiles {
    paths: Vec<PathBuf>,
  },
  PartialAddon {
    addon_id: String,
    project_root: PathBuf,
    journal: Arc<Mutex<Vec<Rollback>>>,
  },
}

pub type CleanupState = Arc<Mutex<Option<CleanupTask>>>;

pub struct AppContext {
  pub paths: AnesisPaths,
  pub client: Client,
  pub cleanup_state: CleanupState,
  pub backend_url: String,
  pub frontend_url: String,
  pub telemetry: bool,
  pub allow_run: bool,
}

pub fn env_flag(name: &str) -> bool {
  match std::env::var(name) {
    Ok(value) => !matches!(
      value.trim().to_ascii_lowercase().as_str(),
      "" | "0" | "false" | "no" | "off"
    ),
    Err(_) => false,
  }
}

impl AppContext {
  pub fn new(paths: AnesisPaths, client: Client, cleanup_state: CleanupState) -> Self {
    let backend_url =
      std::env::var("ANESIS_BACKEND_URL").unwrap_or_else(|_| "https://api.anesis.tech".to_string());
    let frontend_url =
      std::env::var("ANESIS_FRONTEND_URL").unwrap_or_else(|_| "https://anesis.tech".to_string());
    Self {
      paths,
      client,
      cleanup_state,
      backend_url,
      frontend_url,
      telemetry: !env_flag("ANESIS_NO_TELEMETRY"),
      allow_run: env_flag("ANESIS_ALLOW_RUN"),
    }
  }

  pub fn with_cli_flags(mut self, no_telemetry: bool, allow_run: bool) -> Self {
    if no_telemetry {
      self.telemetry = false;
    }
    if allow_run {
      self.allow_run = true;
    }
    self
  }
}

#[doc(hidden)]
pub fn env_flag_for_tests(name: &str) -> bool {
  env_flag(name)
}
