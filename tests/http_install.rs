mod common;

use anesis::addons::lock::{LockEntry, LockFile};
use anesis::addons::runner::update_addon;
use anesis::addons::steps::Rollback;
use common::fixture::Fixture;
use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn a_failed_download_leaves_the_old_version_completely_untouched() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/addon/fixture-addon/url"))
    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
      "archive_url": format!("{}/archive.tar.gz", server.uri()),
      "commit_sha": "newsha",
      "subdir": null,
      "version": "2.0.0"
    })))
    .mount(&server)
    .await;

  let fx = Fixture::new();
  fx.seed_project();

  let x = fx.project.path().canonicalize().unwrap().join("x.txt");
  std::fs::write(&x, "v1-content").unwrap();

  let mut lock = LockFile::load(fx.project.path()).unwrap();
  let mut entry = LockEntry::new("fixture-addon", "1.0.0", "universal");
  entry.upsert_command(
    "install",
    HashMap::new(),
    vec![Rollback::DeleteCreatedFile { path: x.clone() }],
  );
  lock.addons.push(entry);
  lock.save(fx.project.path()).unwrap();

  let ctx = fx.mock_ctx(&server);
  let err = update_addon(&ctx, "fixture-addon", fx.project.path(), true)
    .await
    .expect_err("the archive download must fail its https check");
  assert!(
    format!("{err:#}").contains("https"),
    "unexpected error: {err:#}"
  );

  assert_eq!(
    std::fs::read_to_string(&x).unwrap(),
    "v1-content",
    "the old version's file must be untouched — install runs before undo, so a failed \
     download must never trigger the undo at all"
  );

  let lock = LockFile::load(fx.project.path()).unwrap();
  let entry = lock
    .addons
    .iter()
    .find(|e| e.id == "fixture-addon")
    .expect("the old lock entry must still be present");
  assert_eq!(entry.version, "1.0.0");
  assert!(entry.has_undoable_changes());
}
