mod common;

#[cfg(unix)]
use anesis::addons::lock::LockEntry;
use anesis::addons::lock::LockFile;
use anesis::addons::runner::run_addon_command;
#[cfg(unix)]
use anesis::addons::runner::undo_addon;
#[cfg(unix)]
use anesis::addons::steps::Rollback;
use assert_fs::prelude::*;
use common::fixture::{Fixture, build};
use std::collections::HashMap;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn reversible_addon_with_steps(id: &str, steps: serde_json::Value) -> serde_json::Value {
  let mut manifest = build::addon_manifest(id, "1.0.0");
  manifest["variants"][0]["commands"][0]["steps"] = steps;
  manifest
}

#[tokio::test]
async fn a_glob_step_that_partially_succeeds_is_rolled_back_through_the_runner() {
  let fx = Fixture::new();
  fx.seed_project();

  fx.project
    .child("files/a.txt")
    .write_str("start\nMARKER\nend\n")
    .unwrap();
  std::fs::write(
    fx.project.path().join("files/b.bin"),
    [0xffu8, 0xfe, 0x00, 0x01],
  )
  .unwrap();

  let manifest = reversible_addon_with_steps(
    "fixture-addon",
    serde_json::json!([
      { "type": "create", "path": "created.txt", "content": "hello\n", "if_exists": "overwrite" },
      {
        "type": "inject",
        "target": { "type": "glob", "glob": "files/*" },
        "content": "INJECTED",
        "after": "MARKER",
        "if_not_found": "error"
      }
    ]),
  );
  fx.install_addon("fixture-addon", &manifest);

  let err = run_addon_command(
    &fx.offline_ctx(),
    "fixture-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .expect_err("the inject step must fail on the binary file");

  assert!(
    err.to_string().contains("step 2"),
    "the error should identify which step failed: {err:#}"
  );
  assert!(
    !fx.project.path().join("created.txt").exists(),
    "step 1's create must have been rolled back"
  );
  let a_txt = std::fs::read_to_string(fx.project.path().join("files/a.txt")).unwrap();
  assert_eq!(
    a_txt, "start\nMARKER\nend\n",
    "step 2's partial rewrite of a.txt must have been rolled back too — this is the \
     defect-1 fix: a bare `?` inside the glob loop used to drop the partial rollback, \
     leaving a.txt permanently modified even though the whole command reported failure"
  );

  let lock = LockFile::load(fx.project.path()).unwrap();
  assert!(
    lock.addons.is_empty(),
    "a fully rolled-back command must not leave a lock entry behind"
  );
}

#[tokio::test]
#[cfg(unix)]
async fn a_lock_save_failure_rolls_back_the_commands_file_changes() {
  let fx = Fixture::new();
  fx.seed_project();

  let lock_path = fx.project.path().join("anesis.lock");
  std::fs::write(&lock_path, r#"{"addons":[]}"#).unwrap();

  let subdir = fx.project.path().join("subdir");
  std::fs::create_dir_all(&subdir).unwrap();

  let manifest = reversible_addon_with_steps(
    "fixture-addon",
    serde_json::json!([
      { "type": "create", "path": "subdir/created.txt", "content": "hello\n", "if_exists": "overwrite" }
    ]),
  );
  fx.install_addon("fixture-addon", &manifest);

  std::fs::set_permissions(fx.project.path(), std::fs::Permissions::from_mode(0o555)).unwrap();

  let result = run_addon_command(
    &fx.offline_ctx(),
    "fixture-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await;

  std::fs::set_permissions(fx.project.path(), std::fs::Permissions::from_mode(0o755)).unwrap();

  let err = result.expect_err("a read-only project root must fail the lock save");
  assert!(
    format!("{err:#}").contains("Failed to save anesis.lock"),
    "unexpected error: {err:#}"
  );
  assert!(
    !subdir.join("created.txt").exists(),
    "a lock-save failure must roll back the fully-applied file changes — otherwise the \
     project ends up with created.txt on disk but no lock entry to undo it with"
  );

  let lock = LockFile::load(fx.project.path()).unwrap();
  assert!(
    lock.addons.is_empty(),
    "the pre-existing empty lock file must be unchanged, since the save that would have \
     recorded the addon never succeeded"
  );
}

#[test]
#[cfg(unix)]
fn undo_makes_progress_on_every_entry_it_can_and_keeps_the_rest_for_retry() {
  let fx = Fixture::new();
  fx.seed_project();

  let root = fx.project.path().canonicalize().unwrap();
  let x = root.join("x.txt");
  let y = root.join("y.txt");
  let z = root.join("z.txt");
  std::fs::write(&x, "modified-x").unwrap();
  std::fs::write(&y, "modified-y").unwrap();
  std::fs::write(&z, "modified-z").unwrap();

  std::fs::set_permissions(&y, std::fs::Permissions::from_mode(0o444)).unwrap();

  let mut lock = LockFile::load(fx.project.path()).unwrap();
  let mut entry = LockEntry::new("fixture-addon", "1.0.0", "universal");
  entry.upsert_command(
    "install",
    HashMap::new(),
    vec![
      Rollback::RestoreFile {
        path: x.clone(),
        original: b"orig-x".to_vec(),
        mode: None,
        is_symlink: false,
      },
      Rollback::RestoreFile {
        path: y.clone(),
        original: b"orig-y".to_vec(),
        mode: None,
        is_symlink: false,
      },
      Rollback::RestoreFile {
        path: z.clone(),
        original: b"orig-z".to_vec(),
        mode: None,
        is_symlink: false,
      },
    ],
  );
  lock.addons.push(entry);
  lock.save(fx.project.path()).unwrap();

  let err = undo_addon("fixture-addon", fx.project.path(), true)
    .expect_err("y.txt's read-only permission must make the undo partially fail");
  assert!(
    err.to_string().contains("partially reverted"),
    "unexpected error: {err:#}"
  );
  assert!(err.to_string().contains("1 change"), "{err:#}");

  assert_eq!(std::fs::read_to_string(&x).unwrap(), "orig-x");
  assert_eq!(std::fs::read_to_string(&z).unwrap(), "orig-z");
  assert_eq!(
    std::fs::read_to_string(&y).unwrap(),
    "modified-y",
    "y.txt's restore failed, so it must be untouched"
  );

  let lock = LockFile::load(fx.project.path()).unwrap();
  let entry = lock
    .addons
    .iter()
    .find(|e| e.id == "fixture-addon")
    .expect("a partially-undone addon must still be recorded in the lock");
  let remaining_journal: Vec<&Rollback> = entry
    .commands
    .iter()
    .flat_map(|c| c.journal.iter())
    .collect();
  assert_eq!(
    remaining_journal.len(),
    1,
    "only the failed entry should remain — x.txt and z.txt already succeeded and must not \
     be retried (RenameFile rollbacks are not idempotent on a second application)"
  );
  assert!(matches!(
    remaining_journal[0],
    Rollback::RestoreFile { path, .. } if path == &y
  ));

  std::fs::set_permissions(&y, std::fs::Permissions::from_mode(0o644)).unwrap();
  undo_addon("fixture-addon", fx.project.path(), true).unwrap();
  assert_eq!(std::fs::read_to_string(&y).unwrap(), "orig-y");

  let lock = LockFile::load(fx.project.path()).unwrap();
  assert!(
    !lock.addons.iter().any(|e| e.id == "fixture-addon"),
    "a fully-undone addon must be removed from the lock"
  );
}

#[test]
#[cfg(unix)]
fn undo_restores_file_mode_and_recreates_symlinks() {
  let fx = Fixture::new();
  fx.seed_project();

  let root = fx.project.path().canonicalize().unwrap();
  let script = root.join("script.sh");
  let link = root.join("link.txt");

  let mut lock = LockFile::load(fx.project.path()).unwrap();
  let mut entry = LockEntry::new("fixture-addon", "1.0.0", "universal");
  entry.upsert_command(
    "install",
    HashMap::new(),
    vec![
      Rollback::RestoreFile {
        path: script.clone(),
        original: b"#!/bin/sh\necho hi".to_vec(),
        mode: Some(0o755),
        is_symlink: false,
      },
      Rollback::RestoreFile {
        path: link.clone(),
        original: b"real.txt".to_vec(),
        mode: None,
        is_symlink: true,
      },
    ],
  );
  lock.addons.push(entry);
  lock.save(fx.project.path()).unwrap();

  undo_addon("fixture-addon", fx.project.path(), true).unwrap();

  let meta = std::fs::metadata(&script).unwrap();
  assert_eq!(meta.permissions().mode() & 0o777, 0o755);

  let link_meta = std::fs::symlink_metadata(&link).unwrap();
  assert!(link_meta.file_type().is_symlink());
  assert_eq!(
    std::fs::read_link(&link).unwrap(),
    std::path::Path::new("real.txt")
  );
}
