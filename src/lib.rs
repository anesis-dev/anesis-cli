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

pub const BACKEND_URL: &str = "https://oxide-server.onrender.com";
pub const FRONTEND_URL: &str = "https://oxide-cli.vercel.app";

pub type CleanupState = Arc<Mutex<Option<PathBuf>>>;

pub struct AppContext {
  pub paths: OxidePaths,
  pub client: Client,
  pub cleanup_state: CleanupState,
}
