use assert_fs::prelude::*;
use oxide_cli::addons::{
  detect::detect_variant,
  manifest::{DetectBlock, DetectRule, MatchMode},
};

#[test]
fn file_exists_matches() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("package.json").write_str("{}").unwrap();

  let detect = vec![DetectBlock {
    id: "node".into(),
    rules: vec![DetectRule::FileExists { file: "package.json".into(), negate: false }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("node".into()));
}

#[test]
fn file_exists_negate() {
  let dir = assert_fs::TempDir::new().unwrap();

  let detect = vec![DetectBlock {
    id: "no-package".into(),
    rules: vec![DetectRule::FileExists { file: "package.json".into(), negate: true }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("no-package".into()));
}

#[test]
fn file_contains_matches() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("package.json")
    .write_str(r#"{"dependencies":{"express":"^4"}}"#)
    .unwrap();

  let detect = vec![DetectBlock {
    id: "express".into(),
    rules: vec![DetectRule::FileContains {
      file: "package.json".into(),
      contains: "express".into(),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("express".into()));
}

#[test]
fn json_contains_key_path() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("package.json")
    .write_str(r#"{"dependencies":{"express":"^4"}}"#)
    .unwrap();

  let detect = vec![DetectBlock {
    id: "express".into(),
    rules: vec![DetectRule::JsonContains {
      file: "package.json".into(),
      key_path: "dependencies.express".into(),
      value: None,
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("express".into()));
}

#[test]
fn match_mode_all_requires_all_rules() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("package.json").write_str("{}").unwrap();

  let detect = vec![DetectBlock {
    id: "both".into(),
    rules: vec![
      DetectRule::FileExists { file: "package.json".into(), negate: false },
      DetectRule::FileExists { file: "tsconfig.json".into(), negate: false },
    ],
    match_mode: MatchMode::All,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), None);
}

#[test]
fn no_matching_block_returns_none() {
  let dir = assert_fs::TempDir::new().unwrap();

  let detect = vec![DetectBlock {
    id: "node".into(),
    rules: vec![DetectRule::FileExists { file: "package.json".into(), negate: false }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), None);
}
