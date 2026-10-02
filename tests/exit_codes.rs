use anesis::utils::errors::{AnesisError, exit_code, exit_code_for};
use anyhow::{Context, anyhow};
use assert_cmd::Command;

fn cmd() -> Command {
  assert_cmd::cargo::cargo_bin_cmd!("anesis")
}

#[test]
fn not_logged_in_maps_to_the_auth_code() {
  for err in [
    AnesisError::NotLoggedIn,
    AnesisError::SessionExpired,
    AnesisError::HttpUnauthorized,
  ] {
    assert_eq!(exit_code_for(&err.into()), exit_code::AUTH);
  }
}

#[test]
fn a_missing_resource_maps_to_the_not_found_code() {
  let err = AnesisError::HttpNotFound("template 'nope'".to_string());
  assert_eq!(exit_code_for(&err.into()), exit_code::NOT_FOUND);
}

#[test]
fn network_failures_map_to_the_network_code() {
  for err in [
    AnesisError::NetworkConnect,
    AnesisError::NetworkTimeout,
    AnesisError::HttpServerError("the registry".to_string()),
  ] {
    assert_eq!(exit_code_for(&err.into()), exit_code::NETWORK);
  }
}

#[test]
fn a_missing_terminal_maps_to_its_own_code() {
  let err = AnesisError::NotATerminal("Choosing from a list".to_string());
  assert_eq!(exit_code_for(&err.into()), exit_code::NOT_A_TERMINAL);
}

#[test]
fn declining_a_prompt_maps_to_the_aborted_code() {
  let err = AnesisError::Aborted;
  assert_eq!(exit_code_for(&err.into()), exit_code::ABORTED);
  assert_ne!(exit_code::ABORTED, exit_code::FAILURE);
}

#[test]
fn classification_survives_being_wrapped_in_context() {
  let err = anyhow::Error::from(AnesisError::NotLoggedIn)
    .context("Failed to publish template")
    .context("while running `anesis template publish`");

  assert_eq!(exit_code_for(&err), exit_code::AUTH);
}

#[test]
fn an_unclassified_error_falls_back_to_one() {
  let err = anyhow!("something went sideways");
  assert_eq!(exit_code_for(&err), exit_code::FAILURE);

  let wrapped = Err::<(), _>(anyhow!("inner")).context("outer").unwrap_err();
  assert_eq!(exit_code_for(&wrapped), exit_code::FAILURE);
}

#[test]
fn a_bad_flag_exits_with_the_usage_code() {
  cmd()
    .arg("--definitely-not-a-flag")
    .assert()
    .failure()
    .code(exit_code::USAGE);
}

#[test]
fn an_unknown_subcommand_exits_with_the_usage_code() {
  cmd()
    .arg("frobnicate")
    .assert()
    .failure()
    .code(exit_code::USAGE);
}

#[test]
fn an_unreachable_registry_exits_with_the_network_code() {
  let workdir = assert_fs::TempDir::new().unwrap();

  cmd()
    .current_dir(workdir.path())
    .env("ANESIS_BACKEND_URL", "http://127.0.0.1:1")
    .args(["new", "some-project"])
    .assert()
    .failure()
    .code(exit_code::NETWORK);
}
