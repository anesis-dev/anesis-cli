use std::{fs, path::Path};

use anyhow::Result;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;

use crate::{auth::server::User, utils::errors::AnesisError};

#[derive(Deserialize)]
struct ExpClaim {
  exp: i64,
}

pub fn get_auth_user(auth_path: &Path) -> Result<User> {
  resolve_auth_user(auth_path, std::env::var("ANESIS_TOKEN").ok().as_deref())
}

fn resolve_auth_user(auth_path: &Path, token_override: Option<&str>) -> Result<User> {
  if let Some(token) = token_override {
    let token = token.trim();
    if !token.is_empty() {
      return Ok(User {
        token: token.to_string(),
        name: "token".to_string(),
      });
    }
  }

  let auth_json_str = fs::read_to_string(auth_path).map_err(|_| AnesisError::NotLoggedIn)?;
  let user: User = serde_json::from_str(&auth_json_str)?;

  if is_token_expired(&user.token) {
    return Err(AnesisError::SessionExpired.into());
  }

  Ok(user)
}

#[doc(hidden)]
pub fn get_auth_user_with_token_override_for_tests(
  auth_path: &Path,
  token_override: Option<&str>,
) -> Result<User> {
  resolve_auth_user(auth_path, token_override)
}

fn is_token_expired(token: &str) -> bool {
  let Some(payload_b64) = token.split('.').nth(1) else {
    return false;
  };
  let Ok(bytes) = URL_SAFE_NO_PAD.decode(payload_b64) else {
    return false;
  };
  match serde_json::from_slice::<ExpClaim>(&bytes) {
    Ok(claim) => claim.exp <= chrono::Utc::now().timestamp(),
    Err(_) => false,
  }
}

pub fn is_token_expired_for_tests(token: &str) -> bool {
  is_token_expired(token)
}
