mod common;

use anesis::context::CleanupTask;
use anesis::utils::cleanup::{prune_empty_parents_for_tests, run_cleanup};
use std::path::{Path, PathBuf};

fn download_task(path: &Path, prune_root: &Path) -> CleanupTask {
  CleanupTask::PartialDownload {
    path: path.to_path_buf(),
    prune_root: prune_root.to_path_buf(),
    label: "template",
  }
}

#[test]
fn removes_target_directory() {
  let root = assert_fs::TempDir::new().unwrap();
  let templates_dir = root.path().join("templates");
  let target = templates_dir.join("my-template");
  std::fs::create_dir_all(&target).unwrap();
  std::fs::write(target.join("file.txt"), "content").unwrap();

  run_cleanup(&download_task(&target, &templates_dir));

  assert!(!target.exists(), "target directory should be removed");
}

#[test]
fn removes_empty_parent_directories_up_to_template_root() {
  let root = assert_fs::TempDir::new().unwrap();
  let templates_dir = root.path().join("templates");
  let group = templates_dir.join("group");
  let target = group.join("my-template");
  std::fs::create_dir_all(&target).unwrap();

  run_cleanup(&download_task(&target, &templates_dir));

  assert!(!target.exists());
  assert!(!group.exists(), "empty intermediate dir should be removed");
  assert!(
    templates_dir.exists(),
    "template root should not be removed"
  );
}

#[test]
fn stops_removing_parents_when_dir_is_not_empty() {
  let root = assert_fs::TempDir::new().unwrap();
  let templates_dir = root.path().join("templates");
  let group = templates_dir.join("group");
  let sibling = group.join("other-template");
  let target = group.join("my-template");
  std::fs::create_dir_all(&target).unwrap();
  std::fs::create_dir_all(&sibling).unwrap();

  run_cleanup(&download_task(&target, &templates_dir));

  assert!(!target.exists());
  assert!(group.exists(), "non-empty parent should not be removed");
  assert!(sibling.exists(), "sibling directory must be untouched");
}

#[test]
fn is_noop_when_target_does_not_exist() {
  let root = assert_fs::TempDir::new().unwrap();
  let templates_dir = root.path().join("templates");
  std::fs::create_dir_all(&templates_dir).unwrap();
  let target = templates_dir.join("nonexistent");

  run_cleanup(&download_task(&target, &templates_dir));
}

#[test]
fn removes_nested_files_inside_target() {
  let root = assert_fs::TempDir::new().unwrap();
  let templates_dir = root.path().join("templates");
  let target = templates_dir.join("half-done");
  let nested = target.join("src").join("index.ts");
  std::fs::create_dir_all(nested.parent().unwrap()).unwrap();
  std::fs::write(&nested, "export {}").unwrap();

  run_cleanup(&download_task(&target, &templates_dir));

  assert!(!target.exists());
}

#[test]
fn pruning_never_escapes_its_own_root() {
  let root = assert_fs::TempDir::new().unwrap();
  let cache = root.path().join("cache");
  let addons_dir = cache.join("addons");
  let target = addons_dir.join("owner").join("addon");
  std::fs::create_dir_all(&target).unwrap();

  run_cleanup(&CleanupTask::PartialDownload {
    path: target.clone(),
    prune_root: addons_dir.clone(),
    label: "addon",
  });

  assert!(!target.exists());
  assert!(
    addons_dir.exists(),
    "the addon cache root itself must survive"
  );
  assert!(cache.exists(), "pruning must not climb above its own root");
  assert!(root.path().exists());
}

#[test]
fn pruning_a_path_outside_the_root_removes_nothing() {
  let root = assert_fs::TempDir::new().unwrap();
  let unrelated = root.path().join("elsewhere").join("deep");
  std::fs::create_dir_all(&unrelated).unwrap();
  let prune_root = root.path().join("cache");
  std::fs::create_dir_all(&prune_root).unwrap();

  prune_empty_parents_for_tests(&unrelated, &prune_root);

  assert!(
    unrelated.parent().unwrap().exists(),
    "a path outside the prune root must be left alone"
  );
}

#[test]
fn partial_download_outside_prune_root_is_refused() {
  let root = assert_fs::TempDir::new().unwrap();
  let addons_dir = root.path().join("cache").join("addons");
  std::fs::create_dir_all(&addons_dir).unwrap();
  let victim = root.path().join("victim");
  std::fs::create_dir_all(victim.join("keep-me")).unwrap();
  std::fs::write(victim.join("keep-me").join("file.txt"), "important").unwrap();

  run_cleanup(&CleanupTask::PartialDownload {
    path: victim.clone(),
    prune_root: addons_dir.clone(),
    label: "addon",
  });

  assert!(
    victim.exists(),
    "a path outside prune_root must never be removed"
  );
  assert!(victim.join("keep-me").join("file.txt").exists());
}

#[test]
fn partial_project_is_removed_wholesale() {
  let root = assert_fs::TempDir::new().unwrap();
  let project: PathBuf = root.path().join("my-app");
  std::fs::create_dir_all(project.join("src")).unwrap();
  std::fs::write(project.join("src").join("main.ts"), "console.log(1)").unwrap();

  run_cleanup(&CleanupTask::PartialProject {
    path: project.clone(),
  });

  assert!(!project.exists(), "half-scaffolded project must be removed");
  assert!(root.path().exists(), "the parent directory must survive");
}
