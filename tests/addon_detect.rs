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

// ── FileContains negate ───────────────────────────────────────────────────────

#[test]
fn file_contains_negate_matches_when_content_absent() {
  // negate=true means: rule passes when the file does NOT contain the string
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("app.js").write_str("const koa = require('koa');").unwrap();

  let detect = vec![DetectBlock {
    id: "no-express".into(),
    rules: vec![DetectRule::FileContains {
      file: "app.js".into(),
      contains: "express".into(),
      negate: true,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("no-express".into()));
}

#[test]
fn file_contains_negate_no_match_when_content_present() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("app.js").write_str("const express = require('express');").unwrap();

  let detect = vec![DetectBlock {
    id: "no-express".into(),
    rules: vec![DetectRule::FileContains {
      file: "app.js".into(),
      contains: "express".into(),
      negate: true,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), None);
}

// ── JsonContains with value ───────────────────────────────────────────────────

#[test]
fn json_contains_value_matches() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("package.json")
    .write_str(r#"{"type":"module","version":"1.0.0"}"#)
    .unwrap();

  let detect = vec![DetectBlock {
    id: "esm".into(),
    rules: vec![DetectRule::JsonContains {
      file: "package.json".into(),
      key_path: "type".into(),
      value: Some("module".into()),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("esm".into()));
}

#[test]
fn json_contains_value_no_match_when_different() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("package.json")
    .write_str(r#"{"type":"commonjs"}"#)
    .unwrap();

  let detect = vec![DetectBlock {
    id: "esm".into(),
    rules: vec![DetectRule::JsonContains {
      file: "package.json".into(),
      key_path: "type".into(),
      value: Some("module".into()),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), None);
}

#[test]
fn json_contains_missing_file_no_match() {
  let dir = assert_fs::TempDir::new().unwrap();

  let detect = vec![DetectBlock {
    id: "node".into(),
    rules: vec![DetectRule::JsonContains {
      file: "package.json".into(),
      key_path: "dependencies.express".into(),
      value: None,
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), None);
}

// ── TomlContains ──────────────────────────────────────────────────────────────

#[test]
fn toml_contains_key_path_match() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("Cargo.toml")
    .write_str("[package]\nname = \"my-crate\"\n")
    .unwrap();

  let detect = vec![DetectBlock {
    id: "rust".into(),
    rules: vec![DetectRule::TomlContains {
      file: "Cargo.toml".into(),
      key_path: "package.name".into(),
      value: None,
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("rust".into()));
}

#[test]
fn toml_contains_value_matches() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("Cargo.toml")
    .write_str("[package]\nname = \"my-crate\"\n")
    .unwrap();

  let detect = vec![DetectBlock {
    id: "my-crate".into(),
    rules: vec![DetectRule::TomlContains {
      file: "Cargo.toml".into(),
      key_path: "package.name".into(),
      value: Some("my-crate".into()),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("my-crate".into()));
}

#[test]
fn toml_contains_value_no_match_when_different() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("Cargo.toml")
    .write_str("[package]\nname = \"other-crate\"\n")
    .unwrap();

  let detect = vec![DetectBlock {
    id: "my-crate".into(),
    rules: vec![DetectRule::TomlContains {
      file: "Cargo.toml".into(),
      key_path: "package.name".into(),
      value: Some("my-crate".into()),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), None);
}

#[test]
fn toml_contains_missing_file_no_match() {
  let dir = assert_fs::TempDir::new().unwrap();

  let detect = vec![DetectBlock {
    id: "rust".into(),
    rules: vec![DetectRule::TomlContains {
      file: "Cargo.toml".into(),
      key_path: "package.name".into(),
      value: None,
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), None);
}

// ── YamlContains ──────────────────────────────────────────────────────────────

#[test]
fn yaml_contains_key_path_match() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("config.yaml")
    .write_str("database:\n  host: localhost\n")
    .unwrap();

  let detect = vec![DetectBlock {
    id: "has-db".into(),
    rules: vec![DetectRule::YamlContains {
      file: "config.yaml".into(),
      key_path: "database.host".into(),
      value: None,
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("has-db".into()));
}

#[test]
fn yaml_contains_value_matches() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("config.yaml").write_str("env: production\n").unwrap();

  let detect = vec![DetectBlock {
    id: "prod".into(),
    rules: vec![DetectRule::YamlContains {
      file: "config.yaml".into(),
      key_path: "env".into(),
      value: Some("production".into()),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("prod".into()));
}

#[test]
fn yaml_contains_value_no_match_when_different() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("config.yaml").write_str("env: staging\n").unwrap();

  let detect = vec![DetectBlock {
    id: "prod".into(),
    rules: vec![DetectRule::YamlContains {
      file: "config.yaml".into(),
      key_path: "env".into(),
      value: Some("production".into()),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), None);
}

#[test]
fn yaml_contains_missing_file_no_match() {
  let dir = assert_fs::TempDir::new().unwrap();

  let detect = vec![DetectBlock {
    id: "prod".into(),
    rules: vec![DetectRule::YamlContains {
      file: "config.yaml".into(),
      key_path: "env".into(),
      value: None,
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), None);
}

// ── MatchMode::All when all rules pass ────────────────────────────────────────

#[test]
fn match_mode_all_returns_id_when_all_rules_pass() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("package.json").write_str("{}").unwrap();
  dir.child("tsconfig.json").write_str("{}").unwrap();

  let detect = vec![DetectBlock {
    id: "ts-node".into(),
    rules: vec![
      DetectRule::FileExists { file: "package.json".into(), negate: false },
      DetectRule::FileExists { file: "tsconfig.json".into(), negate: false },
    ],
    match_mode: MatchMode::All,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("ts-node".into()));
}

// ── Multiple blocks: first match wins ────────────────────────────────────────

#[test]
fn first_matching_block_is_returned() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("package.json").write_str("{}").unwrap();
  dir.child("tsconfig.json").write_str("{}").unwrap();

  let detect = vec![
    DetectBlock {
      id: "first".into(),
      rules: vec![DetectRule::FileExists { file: "package.json".into(), negate: false }],
      match_mode: MatchMode::Any,
    },
    DetectBlock {
      id: "second".into(),
      rules: vec![DetectRule::FileExists { file: "tsconfig.json".into(), negate: false }],
      match_mode: MatchMode::Any,
    },
  ];

  assert_eq!(detect_variant(&detect, dir.path()), Some("first".into()));
}

#[test]
fn second_block_returned_when_first_doesnt_match() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("tsconfig.json").write_str("{}").unwrap();
  // no package.json → first block misses

  let detect = vec![
    DetectBlock {
      id: "first".into(),
      rules: vec![DetectRule::FileExists { file: "package.json".into(), negate: false }],
      match_mode: MatchMode::Any,
    },
    DetectBlock {
      id: "second".into(),
      rules: vec![DetectRule::FileExists { file: "tsconfig.json".into(), negate: false }],
      match_mode: MatchMode::Any,
    },
  ];

  assert_eq!(detect_variant(&detect, dir.path()), Some("second".into()));
}

// ── Empty detect list ─────────────────────────────────────────────────────────

#[test]
fn empty_detect_list_returns_none() {
  let dir = assert_fs::TempDir::new().unwrap();
  assert_eq!(detect_variant(&[], dir.path()), None);
}
