pub mod addons;
pub mod auth;
pub mod cache;
pub mod cli;
pub mod config;
pub mod paths;
pub mod templates;
pub mod utils;

use std::{
  path::PathBuf,
  sync::{Arc, Mutex},
};

use reqwest::Client;

use crate::paths::OxidePaths;

pub type CleanupState = Arc<Mutex<Option<PathBuf>>>;

pub struct AppContext {
  pub paths: OxidePaths,
  pub client: Client,
  pub cleanup_state: CleanupState,
  pub backend_url: String,
  pub frontend_url: String,
}

impl AppContext {
  pub fn new(paths: OxidePaths, client: Client, cleanup_state: CleanupState) -> Self {
    let backend_url = std::env::var("OXIDE_BACKEND_URL")
      .unwrap_or_else(|_| "https://oxide-server.onrender.com".to_string());
    let frontend_url = std::env::var("OXIDE_FRONTEND_URL")
      .unwrap_or_else(|_| "https://oxide-cli.vercel.app".to_string());
    Self {
      paths,
      client,
      cleanup_state,
      backend_url,
      frontend_url,
    }
  }
}
