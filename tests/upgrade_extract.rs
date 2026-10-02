use anesis::upgrade::{
  extract_from_targz_for_tests as extract_targz, extract_from_zip_for_tests as extract_zip,
};
use std::io::Write;

fn build_targz(entries: &[(&str, &[u8])]) -> Vec<u8> {
  let mut builder = tar::Builder::new(Vec::new());
  for (path, data) in entries {
    let mut header = tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    builder.append_data(&mut header, path, *data).unwrap();
  }
  let tar_bytes = builder.into_inner().unwrap();
  let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
  encoder.write_all(&tar_bytes).unwrap();
  encoder.finish().unwrap()
}

fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
  let mut buf = Vec::new();
  {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
    let options = zip::write::SimpleFileOptions::default();
    for (name, data) in entries {
      writer.start_file(*name, options).unwrap();
      writer.write_all(data).unwrap();
    }
    writer.finish().unwrap();
  }
  buf
}

#[test]
fn extract_from_targz_finds_the_binary_at_the_top_level() {
  let archive = build_targz(&[("anesis", b"binary-bytes")]);
  let binary = extract_targz(&archive).unwrap();
  assert_eq!(binary, b"binary-bytes");
}

#[test]
fn extract_from_targz_finds_the_binary_inside_a_wrapping_directory() {
  let archive = build_targz(&[
    ("anesis-linux-x86_64/README.md", b"docs"),
    ("anesis-linux-x86_64/anesis", b"binary-bytes"),
  ]);
  let binary = extract_targz(&archive).unwrap();
  assert_eq!(binary, b"binary-bytes");
}

#[test]
fn extract_from_targz_errors_when_the_binary_is_absent() {
  let archive = build_targz(&[("anesis-linux-x86_64/README.md", b"docs")]);
  let err = extract_targz(&archive).unwrap_err();
  assert!(err.to_string().contains("not found"));
}

#[test]
fn extract_from_zip_finds_anesis_exe_at_the_top_level() {
  let archive = build_zip(&[("anesis.exe", b"binary-bytes")]);
  let binary = extract_zip(&archive).unwrap();
  assert_eq!(binary, b"binary-bytes");
}

#[test]
fn extract_from_zip_finds_the_binary_inside_a_wrapping_directory() {
  let archive = build_zip(&[
    ("anesis-windows-x86_64/README.md", b"docs"),
    ("anesis-windows-x86_64/anesis.exe", b"binary-bytes"),
  ]);
  let binary = extract_zip(&archive).unwrap();
  assert_eq!(binary, b"binary-bytes");
}

#[test]
fn extract_from_zip_also_accepts_the_unsuffixed_name() {
  let archive = build_zip(&[("anesis", b"binary-bytes")]);
  let binary = extract_zip(&archive).unwrap();
  assert_eq!(binary, b"binary-bytes");
}

#[test]
fn extract_from_zip_errors_when_the_binary_is_absent() {
  let archive = build_zip(&[("README.md", b"docs")]);
  let err = extract_zip(&archive).unwrap_err();
  assert!(err.to_string().contains("not found"));
}
