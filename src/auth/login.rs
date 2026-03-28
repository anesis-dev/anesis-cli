use std::{fs, path::Path};

use anyhow::Result;
use inquire::Confirm;

use crate::{BACKEND_URL, auth::{server::run_local_auth_server, token::get_auth_user}};

pub async fn login(auth_path: &Path) -> Result<()> {
  if let Ok(existing) = get_auth_user(auth_path) {
    let proceed = Confirm::new(&format!(
      "Already logged in as @{}. Log in with a different account?",
      existing.name
    ))
    .with_default(false)
    .prompt()?;

    if !proceed {
      return Ok(());
    }
  }

  open::that(format!("{}/auth/cli-login", BACKEND_URL))?;
  println!("Go to your browser for further authorization");
  let user = run_local_auth_server().await?;

  let auth_json = serde_json::to_string(&user)?;

  fs::write(auth_path, auth_json)?;

  println!("✅ Authorization successful as @{}", user.name);

  Ok(())
}
