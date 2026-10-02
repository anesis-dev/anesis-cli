use std::path::Path;

use anyhow::Result;
use inquire::Confirm;

use crate::{
  auth::{
    server::{bind_local_auth_server, serve_local_auth_server},
    token::get_auth_user,
  },
  utils::ui::{self, spinner},
};

pub async fn login(auth_path: &Path, backend_url: &str, frontend_url: &str) -> Result<()> {
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

  let state = generate_state_token();

  let listener = bind_local_auth_server(state.clone(), frontend_url).await?;
  let login_url = format!(
    "{}/auth/cli-login?state={}&port={}",
    backend_url,
    state,
    listener.port()
  );
  if open::that(&login_url).is_err() {
    println!(
      "Could not open your browser automatically. Open this URL to log in:\n  {}",
      login_url
    );
  } else {
    println!("Opening browser for authorization...");
    println!("  {}", login_url);
  }
  let sp = spinner("Waiting for browser authorization...");
  let user = serve_local_auth_server(listener)
    .await
    .inspect_err(|_| sp.finish_and_clear())?;
  sp.finish_and_clear();

  let auth_json = serde_json::to_string(&user)?;
  write_auth_file(auth_path, &auth_json)?;

  ui::success(format!("Authorization successful as @{}", user.name));

  if std::env::var("ANESIS_TOKEN")
    .ok()
    .is_some_and(|t| !t.trim().is_empty())
  {
    ui::warn(
      "ANESIS_TOKEN is set in your environment and takes priority over this saved session — \
       the CLI will keep using it until you unset it.",
    );
  }

  Ok(())
}

fn generate_state_token() -> String {
  uuid::Uuid::new_v4().simple().to_string()
}

#[doc(hidden)]
pub fn generate_state_token_for_tests() -> String {
  generate_state_token()
}

fn write_auth_file(path: &Path, content: &str) -> Result<()> {
  #[cfg(unix)]
  {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    let mut file = std::fs::OpenOptions::new()
      .write(true)
      .create(true)
      .truncate(true)
      .open(path)?;
    file.write_all(content.as_bytes())?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
  }
  #[cfg(not(unix))]
  {
    std::fs::write(path, content)?;
  }
  Ok(())
}

#[doc(hidden)]
pub fn write_auth_file_for_tests(path: &Path, content: &str) -> Result<()> {
  write_auth_file(path, content)
}
