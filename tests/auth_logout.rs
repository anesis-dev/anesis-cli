use anesis::auth::logout::logout;
use assert_cmd::Command;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

fn cli(home: &TempDir) -> Command {
  let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("anesis");
  cmd
    .env("HOME", home.path())
    .env("USERPROFILE", home.path())
    .env("ANESIS_HOME", home.path())
    .env("ANESIS_NO_TELEMETRY", "1");
  cmd
}

#[test]
fn removes_auth_file_when_logged_in() {
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_file = dir.child("auth.json");
  auth_file
    .write_str(r#"{"token":"tok","name":"alice"}"#)
    .unwrap();

  logout(auth_file.path()).unwrap();

  assert!(!auth_file.path().exists(), "auth file should be deleted");
}

#[test]
fn returns_error_when_not_logged_in() {
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_path = dir.path().join("nonexistent.json");

  let err = logout(&auth_path).unwrap_err();
  assert!(
    err.to_string().contains("not logged in"),
    "expected 'not logged in' message, got: {err}"
  );
}

#[test]
fn logout_is_idempotent_failure() {
  let dir = assert_fs::TempDir::new().unwrap();
  let auth_file = dir.child("auth.json");
  auth_file.write_str("{}").unwrap();

  logout(auth_file.path()).unwrap();
  let err = logout(auth_file.path()).unwrap_err();
  assert!(err.to_string().contains("not logged in"));
}

#[test]
fn logout_warns_when_anesis_token_still_takes_priority() {
  let home = TempDir::new().unwrap();
  home
    .child(".anesis/auth.json")
    .write_str(r#"{"token":"tok","name":"alice"}"#)
    .unwrap();

  cli(&home)
    .env("ANESIS_TOKEN", "env-token")
    .arg("logout")
    .assert()
    .success()
    .stdout(contains("Logout successful"))
    .stdout(contains("ANESIS_TOKEN"));

  assert!(
    !home.child(".anesis/auth.json").path().exists(),
    "the saved session file must still be removed"
  );
}

#[test]
fn logout_succeeds_with_a_warning_when_only_anesis_token_is_set() {
  let home = TempDir::new().unwrap();

  cli(&home)
    .env("ANESIS_TOKEN", "env-token")
    .arg("logout")
    .assert()
    .success()
    .stdout(contains("No saved session"))
    .stdout(contains("ANESIS_TOKEN"));
}

#[test]
fn logout_without_anesis_token_does_not_mention_it() {
  let home = TempDir::new().unwrap();
  home
    .child(".anesis/auth.json")
    .write_str(r#"{"token":"tok","name":"alice"}"#)
    .unwrap();

  cli(&home)
    .env_remove("ANESIS_TOKEN")
    .arg("logout")
    .assert()
    .success()
    .stdout(contains("Logout successful"))
    .stdout(contains("ANESIS_TOKEN").not());
}
