mod common;

use anesis::doctor::{CheckStatus, has_failure, run_checks};
use assert_fs::prelude::*;
use common::fixture::{Fixture, build};
use common::{
  check_addon_cache_for_tests, check_home_writable_for_tests, check_project_consistency_for_tests,
  check_template_cache_for_tests,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn home_writable_check_passes_for_a_normal_temp_dir() {
  let fx = Fixture::new();
  let ctx = fx.offline_ctx();
  let check = check_home_writable_for_tests(&ctx);
  assert_eq!(check.status, CheckStatus::Ok);
}

#[cfg(unix)]
#[test]
fn home_writable_check_fails_when_the_directory_is_read_only() {
  use std::os::unix::fs::PermissionsExt;

  let fx = Fixture::new();
  let ctx = fx.offline_ctx();
  let mut perms = std::fs::metadata(&ctx.paths.home).unwrap().permissions();
  perms.set_mode(0o500);
  std::fs::set_permissions(&ctx.paths.home, perms.clone()).unwrap();

  let check = check_home_writable_for_tests(&ctx);

  perms.set_mode(0o700);
  std::fs::set_permissions(&ctx.paths.home, perms).unwrap();

  assert_eq!(check.status, CheckStatus::Fail);
}

#[test]
fn project_consistency_is_ok_outside_a_project() {
  let fx = Fixture::new();
  let check = check_project_consistency_for_tests(fx.project.path());
  assert_eq!(check.status, CheckStatus::Ok);
  assert!(check.detail.contains("not inside"));
}

#[test]
fn project_consistency_is_ok_when_manifest_and_lock_agree() {
  let fx = Fixture::new();
  fx.seed_project();
  let check = check_project_consistency_for_tests(fx.project.path());
  assert_eq!(check.status, CheckStatus::Ok);
}

#[test]
fn project_consistency_warns_when_manifest_and_lock_drift_apart() {
  let fx = Fixture::new();
  fx.project
    .child("anesis.json")
    .write_str(r#"{"template_name":"fixture","template_sha":"abc123","addons":["ghost-addon"]}"#)
    .unwrap();
  let check = check_project_consistency_for_tests(fx.project.path());
  assert_eq!(check.status, CheckStatus::Warn);
  assert!(check.detail.contains("ghost-addon"));
}

#[test]
fn addon_cache_is_ok_when_nothing_is_installed() {
  let fx = Fixture::new();
  let ctx = fx.offline_ctx();
  let check = check_addon_cache_for_tests(&ctx);
  assert_eq!(check.status, CheckStatus::Ok);
}

#[test]
fn addon_cache_warns_when_indexed_but_missing_on_disk() {
  let fx = Fixture::new();
  fx.install_addon("ghost", &build::addon_manifest("ghost", "1.0.0"));
  std::fs::remove_dir_all(fx.home.path().join(".anesis/cache/addons/ghost")).unwrap();

  let ctx = fx.offline_ctx();
  let check = check_addon_cache_for_tests(&ctx);
  assert_eq!(check.status, CheckStatus::Warn);
  assert!(check.detail.contains("ghost"));
}

#[test]
fn template_cache_is_ok_when_nothing_is_installed() {
  let fx = Fixture::new();
  let ctx = fx.offline_ctx();
  let check = check_template_cache_for_tests(&ctx);
  assert_eq!(check.status, CheckStatus::Ok);
}

#[tokio::test]
async fn run_checks_aggregates_every_check_and_flags_overall_failure() {
  let fx = Fixture::new();
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/"))
    .respond_with(ResponseTemplate::new(200))
    .mount(&server)
    .await;
  let ctx = fx.mock_ctx(&server);
  unsafe {
    std::env::set_var("ANESIS_RELEASES_API_URL", "http://127.0.0.1:1");
  }

  let checks = run_checks(&ctx, fx.project.path()).await;
  assert!(checks.len() >= 6);
  assert!(checks.iter().any(|c| c.name == "Backend"));
  assert!(checks.iter().any(|c| c.name == "Authentication"));
  assert!(!has_failure(&checks), "{checks:?}");
}
