use std::{fs, path::Path};

use crate::utils::ui;
use anyhow::{Result, anyhow};

fn token_override() -> Option<String> {
  std::env::var("ANESIS_TOKEN")
    .ok()
    .filter(|t| !t.trim().is_empty())
}

pub fn logout(auth_path: &Path) -> Result<()> {
  let removed = match fs::remove_file(auth_path) {
    Ok(_) => true,
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
    Err(e) => return Err(e.into()),
  };

  let has_token_override = token_override().is_some();

  if !removed && !has_token_override {
    return Err(anyhow!("You are not logged in yet."));
  }

  if removed {
    println!("Logout successful");
  } else {
    println!("No saved session to remove.");
  }

  if has_token_override {
    ui::warn(
      "ANESIS_TOKEN is set in your environment and takes priority over the saved session — \
       the CLI is still authenticated as long as it's set. Unset it to fully log out.",
    );
  }

  Ok(())
}
