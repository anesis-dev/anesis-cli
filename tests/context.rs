use std::sync::{Arc, Mutex};

use anesis::context::{AppContext, CleanupState, CleanupTask};
use anesis::paths::AnesisPaths;
use assert_fs::TempDir;
use reqwest::Client;

fn make_paths(tmp: &TempDir) -> AnesisPaths {
  AnesisPaths::under(tmp.path())
}

#[test]
fn new_sets_default_backend_and_frontend_urls() {
  let tmp = TempDir::new().unwrap();
  let cleanup_state: CleanupState = Arc::new(Mutex::new(None));
  let ctx = AppContext::new(make_paths(&tmp), Client::new(), cleanup_state);

  assert_eq!(ctx.backend_url, "https://api.anesis.tech");
  assert_eq!(ctx.frontend_url, "https://anesis.tech");
}

#[test]
fn new_preserves_supplied_paths() {
  let tmp = TempDir::new().unwrap();
  let paths = make_paths(&tmp);
  let expected_auth = paths.auth.clone();
  let cleanup_state: CleanupState = Arc::new(Mutex::new(None));

  let ctx = AppContext::new(paths, Client::new(), cleanup_state);

  assert_eq!(ctx.paths.home, tmp.path().join(".anesis"));
  assert_eq!(ctx.paths.auth, expected_auth);
}

#[test]
fn new_starts_with_empty_cleanup_state() {
  let tmp = TempDir::new().unwrap();
  let cleanup_state: CleanupState = Arc::new(Mutex::new(None));
  let ctx = AppContext::new(make_paths(&tmp), Client::new(), cleanup_state);

  assert!(ctx.cleanup_state.lock().unwrap().is_none());
}

#[test]
fn cleanup_state_is_shared_via_arc() {
  let tmp = TempDir::new().unwrap();
  let cleanup_state: CleanupState = Arc::new(Mutex::new(None));
  let ctx = AppContext::new(make_paths(&tmp), Client::new(), cleanup_state.clone());

  let in_progress = tmp.path().join("in-progress");
  *cleanup_state.lock().unwrap() = Some(CleanupTask::PartialProject {
    path: in_progress.clone(),
  });

  let observed = ctx.cleanup_state.lock().unwrap();
  match observed.as_ref() {
    Some(CleanupTask::PartialProject { path }) => assert_eq!(path, &in_progress),
    other => panic!(
      "expected the registered project task to be visible through the context, got {}",
      match other {
        Some(_) => "a different task",
        None => "nothing",
      }
    ),
  }
}

#[test]
fn cli_flags_only_tighten_the_defaults() {
  let tmp = TempDir::new().unwrap();
  let make = || {
    AppContext::new(
      make_paths(&tmp),
      Client::new(),
      Arc::new(Mutex::new(None)) as CleanupState,
    )
  };

  let base = make();
  assert!(base.telemetry, "install counts are reported by default");
  assert!(
    !base.allow_run,
    "remote shell execution must be opt-in, not opt-out"
  );

  let opted_out = make().with_cli_flags(true, false);
  assert!(!opted_out.telemetry);
  assert!(!opted_out.allow_run);

  let permissive = make().with_cli_flags(false, true);
  assert!(permissive.telemetry);
  assert!(permissive.allow_run);
}

#[test]
fn env_flags_treat_falsey_values_as_unset() {
  use anesis::context::env_flag_for_tests;

  for value in ["0", "false", "no", "off", "OFF", " ", ""] {
    unsafe { std::env::set_var("ANESIS_TEST_FLAG", value) };
    assert!(
      !env_flag_for_tests("ANESIS_TEST_FLAG"),
      "{value:?} should read as unset"
    );
  }

  for value in ["1", "true", "yes", "anything"] {
    unsafe { std::env::set_var("ANESIS_TEST_FLAG", value) };
    assert!(
      env_flag_for_tests("ANESIS_TEST_FLAG"),
      "{value:?} should read as set"
    );
  }

  unsafe { std::env::remove_var("ANESIS_TEST_FLAG") };
  assert!(!env_flag_for_tests("ANESIS_TEST_FLAG"));
}
