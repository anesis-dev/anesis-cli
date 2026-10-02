use std::io::Write as _;

use anesis::utils::archive::{
  MAX_ENTRIES_FOR_TESTS as MAX_ENTRIES, copy_with_cap_for_tests as copy_with_cap,
  extract_tar_gz_for_tests as extract_tar_gz,
};
use flate2::{Compression, write::GzEncoder};
use tar::{Builder, EntryType, Header};

fn build_tar_gz(entries: &[(&str, &[u8])]) -> Vec<u8> {
  let mut builder = Builder::new(Vec::new());
  for (path, data) in entries {
    let mut header = Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder.append_data(&mut header, path, *data).unwrap();
  }
  finish_gz(builder)
}

fn finish_gz(builder: Builder<Vec<u8>>) -> Vec<u8> {
  let tar_bytes = builder.into_inner().unwrap();
  let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
  encoder.write_all(&tar_bytes).unwrap();
  encoder.finish().unwrap()
}

#[test]
fn copy_with_cap_allows_data_within_budget() {
  let mut remaining = 10u64;
  let mut out = Vec::new();
  copy_with_cap(&b"hello"[..], &mut out, &mut remaining).unwrap();
  assert_eq!(out, b"hello");
  assert_eq!(remaining, 5);
}

#[test]
fn copy_with_cap_rejects_data_exceeding_budget() {
  let mut remaining = 3u64;
  let mut out = Vec::new();
  let err = copy_with_cap(&b"hello"[..], &mut out, &mut remaining).unwrap_err();
  assert!(err.to_string().contains("uncompressed size limit"));
}

#[test]
fn extract_tar_gz_writes_files_within_budget() {
  let gz_bytes = build_tar_gz(&[("root/a.txt", b"hello world")]);
  let tmp = tempfile::tempdir().unwrap();
  extract_tar_gz(gz_bytes, tmp.path(), None).unwrap();
  let content = std::fs::read_to_string(tmp.path().join("a.txt")).unwrap();
  assert_eq!(content, "hello world");
}

#[test]
fn extract_tar_gz_rejects_archive_exceeding_entry_count() {
  let entries: Vec<(&str, &[u8])> = (0..=MAX_ENTRIES).map(|_| ("root/x", &b""[..])).collect();
  let gz_bytes = build_tar_gz(&entries);
  let tmp = tempfile::tempdir().unwrap();

  let err = extract_tar_gz(gz_bytes, tmp.path(), None)
    .expect_err("an archive with more than MAX_ENTRIES entries must be rejected");
  assert!(err.to_string().contains("more than"), "{err}");
}

#[test]
fn extract_tar_gz_rejects_path_traversal() {
  let mut builder = Builder::new(Vec::new());
  let mut header = Header::new_gnu();
  let name = b"root/../../etc/passwd";
  header.as_gnu_mut().unwrap().name[..name.len()].copy_from_slice(name);
  header.set_size(5);
  header.set_mode(0o644);
  header.set_cksum();
  builder.append(&header, &b"pwned"[..]).unwrap();
  let gz_bytes = finish_gz(builder);
  let tmp = tempfile::tempdir().unwrap();

  let err =
    extract_tar_gz(gz_bytes, tmp.path(), None).expect_err("a traversal entry must be rejected");
  assert!(err.to_string().contains("unsafe path"), "{err}");
  assert!(
    !tmp.path().parent().unwrap().join("etc/passwd").exists(),
    "the traversal target must never be written"
  );
}

#[test]
fn extract_tar_gz_skips_symlink_entries() {
  let mut builder = Builder::new(Vec::new());
  let mut header = Header::new_gnu();
  header.set_entry_type(EntryType::Symlink);
  header.set_size(0);
  header.set_mode(0o777);
  header.set_cksum();
  builder
    .append_link(&mut header, "root/evil-link", "/etc/passwd")
    .unwrap();
  let gz_bytes = finish_gz(builder);
  let tmp = tempfile::tempdir().unwrap();

  extract_tar_gz(gz_bytes, tmp.path(), None).unwrap();
  assert!(
    !tmp.path().join("evil-link").exists(),
    "a symlink entry must be skipped, never created on disk"
  );
}

#[test]
fn extract_tar_gz_skips_hardlink_entries() {
  let mut builder = Builder::new(Vec::new());
  let mut header = Header::new_gnu();
  header.set_entry_type(EntryType::Link);
  header.set_size(0);
  header.set_mode(0o644);
  header.set_cksum();
  builder
    .append_link(&mut header, "root/evil-hardlink", "/etc/passwd")
    .unwrap();
  let gz_bytes = finish_gz(builder);
  let tmp = tempfile::tempdir().unwrap();

  extract_tar_gz(gz_bytes, tmp.path(), None).unwrap();
  assert!(
    !tmp.path().join("evil-hardlink").exists(),
    "a hardlink entry must be skipped, never created on disk"
  );
}

#[test]
fn extract_tar_gz_filters_by_subdir() {
  let gz_bytes = build_tar_gz(&[
    ("root/keep/a.txt", b"keep me"),
    ("root/skip/b.txt", b"skip me"),
  ]);
  let tmp = tempfile::tempdir().unwrap();

  extract_tar_gz(gz_bytes, tmp.path(), Some("keep")).unwrap();

  assert_eq!(
    std::fs::read_to_string(tmp.path().join("a.txt")).unwrap(),
    "keep me"
  );
  assert!(!tmp.path().join("b.txt").exists());
  assert!(!tmp.path().join("skip").exists());
}

#[test]
#[cfg(unix)]
fn extract_tar_gz_masks_setuid_from_the_tar_header() {
  let mut builder = Builder::new(Vec::new());
  let mut header = Header::new_gnu();
  header.set_size(b"hi".len() as u64);
  header.set_mode(0o7777);
  header.set_cksum();
  builder
    .append_data(&mut header, "root/evil", &b"hi"[..])
    .unwrap();
  let gz_bytes = finish_gz(builder);
  let tmp = tempfile::tempdir().unwrap();

  extract_tar_gz(gz_bytes, tmp.path(), None).unwrap();

  use std::os::unix::fs::PermissionsExt;
  let mode = std::fs::metadata(tmp.path().join("evil"))
    .unwrap()
    .permissions()
    .mode();
  assert_eq!(
    mode & 0o7000,
    0,
    "setuid/setgid/sticky bits from the tar header must not survive extraction, got mode {mode:o}"
  );
}
