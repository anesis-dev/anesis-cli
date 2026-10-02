mod common;

use anesis::addons::lock::{LockEntry, LockFile};
use anesis::addons::runner::update_addon;
use common::fixture::{Fixture, build};
use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn mount_addon_url(server: &MockServer, addon_id: &str) {
  Mock::given(method("GET"))
    .and(path(format!("/addon/{addon_id}/url")))
    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
      "archive_url": format!("{}/archive.tar.gz", server.uri()),
      "commit_sha": "deadbeef",
      "subdir": null,
      "version": "2.0.0"
    })))
    .mount(server)
    .await;
}

fn seed_applied_addon(fx: &Fixture, addon_id: &str) {
  let mut lock = LockFile::load(fx.project.path()).unwrap();
  let mut entry = LockEntry::new(addon_id, "1.0.0", "universal");
  entry.upsert_command("install", HashMap::new(), vec![]);
  lock.addons.push(entry);
  lock.save(fx.project.path()).unwrap();
}

fn write_new_cached_manifest(fx: &Fixture, addon_id: &str, manifest: &serde_json::Value) {
  fx.home
    .child(format!(".anesis/cache/addons/{addon_id}/anesis.addon.json"))
    .write_str(&serde_json::to_string_pretty(manifest).unwrap())
    .unwrap();
}

use assert_fs::prelude::*;

#[tokio::test]
async fn preflight_aborts_before_undo_when_command_disappears() {
  let server = MockServer::start().await;
  mount_addon_url(&server, "fixture-addon").await;

  let fx = Fixture::new();
  fx.seed_project();
  fx.install_addon(
    "fixture-addon",
    &build::addon_manifest("fixture-addon", "1.0.0"),
  );

  let x = fx.project.path().canonicalize().unwrap().join("x.txt");
  std::fs::write(&x, "v1-content").unwrap();
  seed_applied_addon(&fx, "fixture-addon");

  let mut new_manifest = build::addon_manifest("fixture-addon", "2.0.0");
  new_manifest["variants"][0]["commands"] = serde_json::json!([]);
  write_new_cached_manifest(&fx, "fixture-addon", &new_manifest);

  let ctx = fx.mock_ctx(&server);
  let err = update_addon(&ctx, "fixture-addon", fx.project.path(), true)
    .await
    .expect_err("a command that no longer exists must abort the update before undo");
  assert!(
    format!("{err:#}").contains("no longer exists"),
    "unexpected error: {err:#}"
  );

  assert_eq!(
    std::fs::read_to_string(&x).unwrap(),
    "v1-content",
    "preflight must fail before undo touches any files"
  );
  let lock = LockFile::load(fx.project.path()).unwrap();
  assert_eq!(
    lock
      .addons
      .iter()
      .find(|e| e.id == "fixture-addon")
      .unwrap()
      .version,
    "1.0.0",
    "the lock entry must still record the old version"
  );
}

#[tokio::test]
async fn preflight_aborts_before_undo_when_variant_no_longer_matches() {
  let server = MockServer::start().await;
  mount_addon_url(&server, "fixture-addon").await;

  let fx = Fixture::new();
  fx.seed_project();
  fx.install_addon(
    "fixture-addon",
    &build::addon_manifest("fixture-addon", "1.0.0"),
  );

  let x = fx.project.path().canonicalize().unwrap().join("x.txt");
  std::fs::write(&x, "v1-content").unwrap();
  seed_applied_addon(&fx, "fixture-addon");

  let mut new_manifest = build::addon_manifest("fixture-addon", "2.0.0");
  new_manifest["variants"][0]["when"] = serde_json::json!("nestjs");
  write_new_cached_manifest(&fx, "fixture-addon", &new_manifest);

  let ctx = fx.mock_ctx(&server);
  let err = update_addon(&ctx, "fixture-addon", fx.project.path(), true)
    .await
    .expect_err("a variant that no longer matches must abort the update before undo");
  assert!(
    format!("{err:#}").contains("no variant"),
    "unexpected error: {err:#}"
  );

  assert_eq!(std::fs::read_to_string(&x).unwrap(), "v1-content");
}

#[tokio::test]
async fn preflight_aborts_before_undo_when_a_run_step_needs_allow_run() {
  let server = MockServer::start().await;
  mount_addon_url(&server, "fixture-addon").await;

  let fx = Fixture::new();
  fx.seed_project();
  fx.install_addon(
    "fixture-addon",
    &build::addon_manifest("fixture-addon", "1.0.0"),
  );

  let x = fx.project.path().canonicalize().unwrap().join("x.txt");
  std::fs::write(&x, "v1-content").unwrap();
  seed_applied_addon(&fx, "fixture-addon");

  let mut new_manifest = build::addon_manifest("fixture-addon", "2.0.0");
  new_manifest["variants"][0]["commands"][0]["steps"] = serde_json::json!([
    { "type": "run", "command": "echo hi", "description": "" }
  ]);
  write_new_cached_manifest(&fx, "fixture-addon", &new_manifest);

  let ctx = fx.mock_ctx(&server);
  assert!(!ctx.allow_run);
  let err = update_addon(&ctx, "fixture-addon", fx.project.path(), true)
    .await
    .expect_err("a new run step without --allow-run must abort the update before undo");
  assert!(
    format!("{err:#}").contains("--allow-run"),
    "unexpected error: {err:#}"
  );

  assert_eq!(std::fs::read_to_string(&x).unwrap(), "v1-content");
}

#[tokio::test]
async fn preflight_aborts_before_undo_when_a_required_input_was_not_saved() {
  let server = MockServer::start().await;
  mount_addon_url(&server, "fixture-addon").await;

  let fx = Fixture::new();
  fx.seed_project();
  fx.install_addon(
    "fixture-addon",
    &build::addon_manifest("fixture-addon", "1.0.0"),
  );

  let x = fx.project.path().canonicalize().unwrap().join("x.txt");
  std::fs::write(&x, "v1-content").unwrap();
  seed_applied_addon(&fx, "fixture-addon");

  let mut new_manifest = build::addon_manifest("fixture-addon", "2.0.0");
  new_manifest["inputs"] = serde_json::json!([
    { "name": "dbName", "type": "text", "description": "", "default": null, "required": true }
  ]);
  write_new_cached_manifest(&fx, "fixture-addon", &new_manifest);

  let ctx = fx.mock_ctx(&server);
  let err = update_addon(&ctx, "fixture-addon", fx.project.path(), true)
    .await
    .expect_err("a newly-required input with no saved value must abort the update before undo");
  assert!(
    format!("{err:#}").contains("dbName"),
    "unexpected error: {err:#}"
  );

  assert_eq!(std::fs::read_to_string(&x).unwrap(), "v1-content");
}

#[tokio::test]
async fn preflight_passes_and_update_proceeds_when_everything_still_matches() {
  let server = MockServer::start().await;
  mount_addon_url(&server, "fixture-addon").await;

  let fx = Fixture::new();
  fx.seed_project();
  fx.install_addon(
    "fixture-addon",
    &build::addon_manifest("fixture-addon", "1.0.0"),
  );

  seed_applied_addon(&fx, "fixture-addon");

  let new_manifest = build::addon_manifest("fixture-addon", "2.0.0");
  write_new_cached_manifest(&fx, "fixture-addon", &new_manifest);

  let ctx = fx.mock_ctx(&server);
  update_addon(&ctx, "fixture-addon", fx.project.path(), true)
    .await
    .expect("preflight must pass when nothing incompatible changed");

  let lock = LockFile::load(fx.project.path()).unwrap();
  assert_eq!(
    lock
      .addons
      .iter()
      .find(|e| e.id == "fixture-addon")
      .unwrap()
      .version,
    "2.0.0"
  );
}
