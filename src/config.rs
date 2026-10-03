use crate::{
  context::{AppContext, CleanupState},
  paths::AnesisPaths,
  utils::cleanup::setup_ctrlc_handler,
};
use anyhow::Result;
use reqwest::Client;
use std::{
  sync::{Arc, Mutex},
  time::Duration,
};

pub fn init_env() {
  #[cfg(debug_assertions)]
  {
    dotenvy::dotenv().ok();
  }
}

pub fn init_logging(verbose: u8) {
  let mut builder = env_logger::Builder::from_env(env_logger::Env::default());

  if verbose > 0 && std::env::var("RUST_LOG").is_err() {
    let level = if verbose == 1 {
      log::LevelFilter::Debug
    } else {
      log::LevelFilter::Trace
    };
    builder.filter_module("anesis", level);
  }

  builder.target(env_logger::Target::Stderr);
  builder.init();
}

pub fn version_check_client() -> Client {
  Client::builder()
    .connect_timeout(Duration::from_secs(2))
    .timeout(Duration::from_secs(2))
    .build()
    .unwrap_or_default()
}

pub fn build_app_context() -> Result<AppContext> {
  let anesis_paths = AnesisPaths::new()?;
  anesis_paths.ensure_directories()?;

  let client = Client::builder()
    .connect_timeout(Duration::from_secs(10))
    .timeout(Duration::from_secs(90))
    .build()?;

  let cleanup_state: CleanupState = Arc::new(Mutex::new(None));

  setup_ctrlc_handler(cleanup_state.clone())?;

  Ok(AppContext::new(anesis_paths, client, cleanup_state))
}
