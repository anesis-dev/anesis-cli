mod common;

use common::prune_empty_dirs_for_tests;

#[test]
fn stops_exactly_at_the_project_root() {
  let root = assert_fs::TempDir::new().unwrap();
  let nested = root.path().join("src").join("components");
  std::fs::create_dir_all(&nested).unwrap();

  prune_empty_dirs_for_tests(Some(&nested), root.path());

  assert!(
    !root.path().join("src").exists(),
    "empty dirs under the project root should be pruned"
  );
  assert!(root.path().exists(), "the project root itself must survive");
}

#[test]
fn does_not_climb_above_a_path_outside_the_root() {
  let root = assert_fs::TempDir::new().unwrap();
  let unrelated = root.path().join("elsewhere").join("deep");
  std::fs::create_dir_all(&unrelated).unwrap();
  let project_root = root.path().join("project");
  std::fs::create_dir_all(&project_root).unwrap();

  prune_empty_dirs_for_tests(Some(&unrelated), &project_root);

  assert!(
    unrelated.parent().unwrap().exists(),
    "a path outside the project root must be left alone"
  );
  assert!(
    root.path().exists(),
    "must never climb above the temp root itself"
  );
}

#[cfg(unix)]
#[test]
fn prunes_even_when_project_root_is_reached_through_a_symlink() {
  let real_root = assert_fs::TempDir::new().unwrap();
  let parent = assert_fs::TempDir::new().unwrap();
  let symlinked_root = parent.path().join("project-via-symlink");
  std::os::unix::fs::symlink(real_root.path(), &symlinked_root).unwrap();

  let nested = symlinked_root.join("src").join("components");
  std::fs::create_dir_all(&nested).unwrap();

  // `nested` was built through the symlink and so canonicalizes to a path
  // under `real_root`, while `project_root` here is the raw (uncanonicalized)
  // symlinked path — the same mismatch that occurs in production between a
  // lock-file path (absolutized against a canonicalized root) and a raw
  // project_root argument. A lexical `starts_with` check would reject this
  // as "outside the root" and prune nothing.
  prune_empty_dirs_for_tests(Some(&nested), &symlinked_root);

  assert!(
    !real_root.path().join("src").exists(),
    "empty dirs must still be pruned when the root is reached through a symlink"
  );
}
