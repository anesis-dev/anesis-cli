use anesis::addons::{
  detect::detect_variant,
  manifest::{DetectBlock, DetectRule, MatchMode},
};
use assert_fs::prelude::*;

#[test]
fn file_exists_matches() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("package.json").write_str("{}").unwrap();

  let detect = vec![DetectBlock {
    id: "node".into(),
    rules: vec![DetectRule::FileExists {
      file: "package.json".into(),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("node".into()));
}

#[test]
fn file_exists_negate() {
  let dir = assert_fs::TempDir::new().unwrap();

  let detect = vec![DetectBlock {
    id: "no-package".into(),
    rules: vec![DetectRule::FileExists {
      file: "package.json".into(),
      negate: true,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(
    detect_variant(&detect, dir.path()),
    Some("no-package".into())
  );
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
      DetectRule::FileExists {
        file: "package.json".into(),
        negate: false,
      },
      DetectRule::FileExists {
        file: "tsconfig.json".into(),
        negate: false,
      },
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
    rules: vec![DetectRule::FileExists {
      file: "package.json".into(),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), None);
}

#[test]
fn file_contains_negate_matches_when_content_absent() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("app.js")
    .write_str("const koa = require('koa');")
    .unwrap();

  let detect = vec![DetectBlock {
    id: "no-express".into(),
    rules: vec![DetectRule::FileContains {
      file: "app.js".into(),
      contains: "express".into(),
      negate: true,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(
    detect_variant(&detect, dir.path()),
    Some("no-express".into())
  );
}

#[test]
fn file_contains_negate_no_match_when_content_present() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("app.js")
    .write_str("const express = require('express');")
    .unwrap();

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
  dir
    .child("config.yaml")
    .write_str("env: production\n")
    .unwrap();

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
  dir
    .child("config.yaml")
    .write_str("env: staging\n")
    .unwrap();

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
fn yaml_contains_matches_a_boolean_value_by_its_string_form() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("config.yaml")
    .write_str("enabled: true\n")
    .unwrap();

  let detect = vec![DetectBlock {
    id: "enabled".into(),
    rules: vec![DetectRule::YamlContains {
      file: "config.yaml".into(),
      key_path: "enabled".into(),
      value: Some("true".into()),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(
    detect_variant(&detect, dir.path()),
    Some("enabled".into()),
    "the YAML backend's bool Display formatting must stay pinned to plain 'true'/'false'"
  );
}

#[test]
fn yaml_contains_matches_an_integer_value_by_its_string_form() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("config.yaml").write_str("port: 3000\n").unwrap();

  let detect = vec![DetectBlock {
    id: "port".into(),
    rules: vec![DetectRule::YamlContains {
      file: "config.yaml".into(),
      key_path: "port".into(),
      value: Some("3000".into()),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(
    detect_variant(&detect, dir.path()),
    Some("port".into()),
    "the YAML backend's number Display formatting must stay pinned to a plain integer, \
     with no trailing '.0' or scientific notation"
  );
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

#[test]
fn match_mode_all_returns_id_when_all_rules_pass() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("package.json").write_str("{}").unwrap();
  dir.child("tsconfig.json").write_str("{}").unwrap();

  let detect = vec![DetectBlock {
    id: "ts-node".into(),
    rules: vec![
      DetectRule::FileExists {
        file: "package.json".into(),
        negate: false,
      },
      DetectRule::FileExists {
        file: "tsconfig.json".into(),
        negate: false,
      },
    ],
    match_mode: MatchMode::All,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("ts-node".into()));
}

#[test]
fn first_matching_block_is_returned() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("package.json").write_str("{}").unwrap();
  dir.child("tsconfig.json").write_str("{}").unwrap();

  let detect = vec![
    DetectBlock {
      id: "first".into(),
      rules: vec![DetectRule::FileExists {
        file: "package.json".into(),
        negate: false,
      }],
      match_mode: MatchMode::Any,
    },
    DetectBlock {
      id: "second".into(),
      rules: vec![DetectRule::FileExists {
        file: "tsconfig.json".into(),
        negate: false,
      }],
      match_mode: MatchMode::Any,
    },
  ];

  assert_eq!(detect_variant(&detect, dir.path()), Some("first".into()));
}

#[test]
fn second_block_returned_when_first_doesnt_match() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("tsconfig.json").write_str("{}").unwrap();

  let detect = vec![
    DetectBlock {
      id: "first".into(),
      rules: vec![DetectRule::FileExists {
        file: "package.json".into(),
        negate: false,
      }],
      match_mode: MatchMode::Any,
    },
    DetectBlock {
      id: "second".into(),
      rules: vec![DetectRule::FileExists {
        file: "tsconfig.json".into(),
        negate: false,
      }],
      match_mode: MatchMode::Any,
    },
  ];

  assert_eq!(detect_variant(&detect, dir.path()), Some("second".into()));
}

#[test]
fn empty_detect_list_returns_none() {
  let dir = assert_fs::TempDir::new().unwrap();
  assert_eq!(detect_variant(&[], dir.path()), None);
}

#[test]
fn match_mode_all_requires_all_rules_part2() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("package.json").write_str("{}").unwrap();

  let detect = vec![DetectBlock {
    id: "ts-node".into(),
    rules: vec![
      DetectRule::FileExists {
        file: "package.json".into(),
        negate: false,
      },
      DetectRule::FileExists {
        file: "tsconfig.json".into(),
        negate: false,
      },
    ],
    match_mode: MatchMode::All,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), None);
}

#[test]
fn json_contains_negate_matches_when_key_absent() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("package.json")
    .write_str(r#"{"name":"my-app"}"#)
    .unwrap();

  let detect = vec![DetectBlock {
    id: "no-express".into(),
    rules: vec![DetectRule::JsonContains {
      file: "package.json".into(),
      key_path: "dependencies.express".into(),
      value: None,
      negate: true,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(
    detect_variant(&detect, dir.path()),
    Some("no-express".into())
  );
}

#[test]
fn json_contains_negate_no_match_when_key_present() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("package.json")
    .write_str(r#"{"dependencies":{"express":"^4.0.0"}}"#)
    .unwrap();

  let detect = vec![DetectBlock {
    id: "no-express".into(),
    rules: vec![DetectRule::JsonContains {
      file: "package.json".into(),
      key_path: "dependencies.express".into(),
      value: None,
      negate: true,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), None);
}

#[test]
fn toml_contains_negate_matches_when_key_absent() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("Cargo.toml")
    .write_str("[package]\nname = \"my-crate\"\n")
    .unwrap();

  let detect = vec![DetectBlock {
    id: "no-tokio".into(),
    rules: vec![DetectRule::TomlContains {
      file: "Cargo.toml".into(),
      key_path: "dependencies.tokio".into(),
      value: None,
      negate: true,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("no-tokio".into()));
}

#[test]
fn toml_contains_negate_no_match_when_key_present() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("Cargo.toml")
    .write_str("[package]\nname = \"my-crate\"\n\n[dependencies]\ntokio = \"1\"\n")
    .unwrap();

  let detect = vec![DetectBlock {
    id: "no-tokio".into(),
    rules: vec![DetectRule::TomlContains {
      file: "Cargo.toml".into(),
      key_path: "dependencies.tokio".into(),
      value: None,
      negate: true,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), None);
}

#[test]
fn yaml_contains_negate_matches_when_key_absent() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("config.yaml")
    .write_str("name: my-app\n")
    .unwrap();

  let detect = vec![DetectBlock {
    id: "no-database".into(),
    rules: vec![DetectRule::YamlContains {
      file: "config.yaml".into(),
      key_path: "database.host".into(),
      value: None,
      negate: true,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(
    detect_variant(&detect, dir.path()),
    Some("no-database".into())
  );
}

#[test]
fn yaml_contains_negate_no_match_when_key_present() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("config.yaml")
    .write_str("database:\n  host: localhost\n")
    .unwrap();

  let detect = vec![DetectBlock {
    id: "no-database".into(),
    rules: vec![DetectRule::YamlContains {
      file: "config.yaml".into(),
      key_path: "database.host".into(),
      value: None,
      negate: true,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), None);
}

#[test]
fn json_contains_numeric_value_matches_as_string() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir
    .child("package.json")
    .write_str(r#"{"engines":{"node":18}}"#)
    .unwrap();

  let detect = vec![DetectBlock {
    id: "node18".into(),
    rules: vec![DetectRule::JsonContains {
      file: "package.json".into(),
      key_path: "engines.node".into(),
      value: Some("18".into()),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("node18".into()));
}

#[test]
fn file_exists_negate_matches_when_file_absent() {
  let dir = assert_fs::TempDir::new().unwrap();

  let detect = vec![DetectBlock {
    id: "no-file".into(),
    rules: vec![DetectRule::FileExists {
      file: "does-not-exist.txt".into(),
      negate: true,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("no-file".into()));
}

#[test]
fn file_contains_negate_matches_when_file_missing() {
  let dir = assert_fs::TempDir::new().unwrap();

  let detect = vec![DetectBlock {
    id: "no-match".into(),
    rules: vec![DetectRule::FileContains {
      file: "missing.txt".into(),
      contains: "anything".into(),
      negate: true,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("no-match".into()));
}

#[test]
fn file_exists_rejects_absolute_path_outside_root() {
  let dir = assert_fs::TempDir::new().unwrap();

  let detect = vec![DetectBlock {
    id: "leaked".into(),
    rules: vec![DetectRule::FileExists {
      file: "/etc/passwd".into(),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(
    detect_variant(&detect, dir.path()),
    None,
    "an absolute path must never be treated as a match"
  );
}

#[test]
fn file_exists_rejects_parent_traversal_outside_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let outside = dir
    .path()
    .parent()
    .unwrap()
    .join("anesis-detect-outside.txt");
  std::fs::write(&outside, "secret").unwrap();

  let detect = vec![DetectBlock {
    id: "leaked".into(),
    rules: vec![DetectRule::FileExists {
      file: "../anesis-detect-outside.txt".into(),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  let result = detect_variant(&detect, dir.path());
  std::fs::remove_file(&outside).ok();

  assert_eq!(
    result, None,
    "a '..' path escaping the project root must never be treated as a match"
  );
}

#[test]
fn file_contains_rejects_parent_traversal_outside_root() {
  let dir = assert_fs::TempDir::new().unwrap();
  let outside = dir
    .path()
    .parent()
    .unwrap()
    .join("anesis-detect-secret.txt");
  std::fs::write(&outside, "super-secret-value").unwrap();

  let detect = vec![DetectBlock {
    id: "leaked".into(),
    rules: vec![DetectRule::FileContains {
      file: "../anesis-detect-secret.txt".into(),
      contains: "super-secret-value".into(),
      negate: false,
    }],
    match_mode: MatchMode::Any,
  }];

  let result = detect_variant(&detect, dir.path());
  std::fs::remove_file(&outside).ok();

  assert_eq!(
    result, None,
    "content of a file outside the project root must never be readable via detect rules"
  );
}

#[test]
fn file_exists_negate_does_not_flip_for_out_of_root_path() {
  let dir = assert_fs::TempDir::new().unwrap();

  let detect = vec![DetectBlock {
    id: "negated".into(),
    rules: vec![DetectRule::FileExists {
      file: "/etc/passwd".into(),
      negate: true,
    }],
    match_mode: MatchMode::Any,
  }];

  assert_eq!(detect_variant(&detect, dir.path()), Some("negated".into()));
}
