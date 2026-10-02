mod common;

use anesis::{auth::token::get_auth_user, utils::errors::AnesisError};
use assert_fs::prelude::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use common::{get_auth_user_with_token_override_for_tests, is_token_expired_for_tests};

fn token_with_exp(exp: i64) -> String {
  let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256"}"#);
  let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"exp":{exp}}}"#));
  format!("{header}.{payload}.sig")
}

#[test]
fn reads_valid_auth_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_file = dir.child("auth.json");
  auth_file
    .write_str(r#"{"token":"tok123","name":"alice"}"#)
    .unwrap();

  let user = get_auth_user(auth_file.path()).unwrap();
  assert_eq!(user.token, "tok123");
  assert_eq!(user.name, "alice");
}

#[test]
fn returns_not_logged_in_when_file_missing() {
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_path = dir.path().join("nonexistent.json");

  let err = get_auth_user(&auth_path).unwrap_err();
  assert!(
    err
      .downcast_ref::<AnesisError>()
      .is_some_and(|e| matches!(e, AnesisError::NotLoggedIn)),
    "expected NotLoggedIn, got: {err}"
  );
}

#[test]
fn returns_error_for_invalid_json() {
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_file = dir.child("auth.json");
  auth_file.write_str("not valid json at all").unwrap();

  let err = get_auth_user(auth_file.path()).unwrap_err();
  assert!(
    err.downcast_ref::<AnesisError>().is_none(),
    "invalid JSON should not produce AnesisError"
  );
}

#[test]
fn returns_error_for_missing_required_fields() {
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_file = dir.child("auth.json");
  auth_file.write_str(r#"{"foo":"bar"}"#).unwrap();

  let err = get_auth_user(auth_file.path()).unwrap_err();
  assert!(err.downcast_ref::<AnesisError>().is_none());
}

#[test]
fn tolerates_extra_fields_in_auth_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_file = dir.child("auth.json");
  auth_file
    .write_str(r#"{"token":"t","name":"bob","extra":"ignored"}"#)
    .unwrap();

  let user = get_auth_user(auth_file.path()).unwrap();
  assert_eq!(user.name, "bob");
}

#[test]
fn detects_expired_and_valid_tokens() {
  let now = chrono::Utc::now().timestamp();
  assert!(is_token_expired_for_tests(&token_with_exp(now - 60)));
  assert!(!is_token_expired_for_tests(&token_with_exp(now + 3600)));
  assert!(!is_token_expired_for_tests("not-a-jwt"));
}

#[test]
fn anesis_token_env_var_short_circuits_the_auth_file_entirely() {
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_path = dir.path().join("never-read.json"); // deliberately missing

  let user = get_auth_user_with_token_override_for_tests(&auth_path, Some("env-token"))
    .expect("ANESIS_TOKEN must be honored even when the auth file doesn't exist");
  assert_eq!(user.token, "env-token");
  assert_eq!(user.name, "token");
}

#[test]
fn anesis_token_is_trimmed_of_surrounding_whitespace() {
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_path = dir.path().join("never-read.json");

  let user =
    get_auth_user_with_token_override_for_tests(&auth_path, Some("  env-token  \n")).unwrap();
  assert_eq!(user.token, "env-token");
}

#[test]
fn an_empty_anesis_token_falls_back_to_the_auth_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_file = dir.child("auth.json");
  auth_file
    .write_str(r#"{"token":"file-token","name":"alice"}"#)
    .unwrap();

  let user = get_auth_user_with_token_override_for_tests(auth_file.path(), Some("   ")).unwrap();
  assert_eq!(
    user.token, "file-token",
    "whitespace-only ANESIS_TOKEN must be treated as unset"
  );
}
