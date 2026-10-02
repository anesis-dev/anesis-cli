use anesis::utils::pathsafe::safe_join;

#[test]
fn joins_a_relative_path_inside_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let joined = safe_join(dir.path(), "src/main.rs", "target file").unwrap();
  assert_eq!(
    joined,
    dir.path().canonicalize().unwrap().join("src/main.rs")
  );
}

#[test]
fn rejects_a_traversing_relative_path() {
  let dir = assert_fs::TempDir::new().unwrap();
  let err = safe_join(dir.path(), "../../etc/passwd", "target file").unwrap_err();
  assert!(err.to_string().contains("Path traversal blocked"));
}

#[cfg(unix)]
#[test]
fn rejects_a_symlink_that_escapes_the_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let outside = assert_fs::TempDir::new().unwrap();

  std::os::unix::fs::symlink(outside.path(), dir.path().join("escape")).unwrap();
  let err = safe_join(dir.path(), "escape/victim", "target file").unwrap_err();
  assert!(err.to_string().contains("Path traversal blocked"));
}

#[test]
fn allows_a_dot_prefixed_relative_path() {
  let dir = assert_fs::TempDir::new().unwrap();
  let joined = safe_join(dir.path(), "./file.txt", "target file").unwrap();
  assert_eq!(joined, dir.path().canonicalize().unwrap().join("file.txt"));
}
