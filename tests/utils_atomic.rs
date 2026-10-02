use anesis::utils::atomic::write_atomic;

#[test]
fn writes_a_new_file() {
  let dir = assert_fs::TempDir::new().unwrap();
  let path = dir.path().join("out.txt");
  write_atomic(&path, b"hello").unwrap();
  assert_eq!(std::fs::read(&path).unwrap(), b"hello");
}

#[test]
fn overwrites_an_existing_file_completely() {
  let dir = assert_fs::TempDir::new().unwrap();
  let path = dir.path().join("out.txt");
  std::fs::write(&path, b"this is a much longer original body").unwrap();

  write_atomic(&path, b"short").unwrap();

  assert_eq!(std::fs::read(&path).unwrap(), b"short");
}

#[test]
fn leaves_no_temp_file_behind_on_success() {
  let dir = assert_fs::TempDir::new().unwrap();
  let path = dir.path().join("out.txt");
  write_atomic(&path, b"hello").unwrap();

  let entries: Vec<_> = std::fs::read_dir(dir.path())
    .unwrap()
    .map(|e| e.unwrap().file_name())
    .collect();
  assert_eq!(entries, vec![std::ffi::OsString::from("out.txt")]);
}
