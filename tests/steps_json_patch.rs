use anesis::addons::{
  manifest::JsonPatchStep,
  steps::{Rollback, json_patch::execute_json_patch},
};
use anesis::utils::template_engine::TemplateContext;
use assert_fs::prelude::*;
use serde_json::json;
use std::collections::HashMap;

fn empty_ctx() -> TemplateContext {
  TemplateContext::new()
}

fn write_json(dir: &assert_fs::TempDir, name: &str, value: &serde_json::Value) {
  dir
    .child(name)
    .write_str(&serde_json::to_string_pretty(value).unwrap())
    .unwrap();
}

fn read_json(dir: &assert_fs::TempDir, name: &str) -> serde_json::Value {
  let content = std::fs::read_to_string(dir.path().join(name)).unwrap();
  serde_json::from_str(&content).unwrap()
}

#[test]
fn set_overwrites_a_scalar_value() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_json(
    &dir,
    "package.json",
    &json!({ "name": "app", "version": "1.0.0" }),
  );

  let mut set = HashMap::new();
  set.insert("version".to_string(), json!("2.0.0"));
  let step = JsonPatchStep {
    path: "package.json".into(),
    set,
    remove: Vec::new(),
  };
  execute_json_patch(&step, dir.path(), &empty_ctx()).unwrap();

  let result = read_json(&dir, "package.json");
  assert_eq!(result["version"], "2.0.0");
  assert_eq!(result["name"], "app");
}

#[test]
fn set_creates_intermediate_objects_for_a_dotted_path() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_json(&dir, "config.json", &json!({}));

  let mut set = HashMap::new();
  set.insert("scripts.build".to_string(), json!("tsc"));
  let step = JsonPatchStep {
    path: "config.json".into(),
    set,
    remove: Vec::new(),
  };
  execute_json_patch(&step, dir.path(), &empty_ctx()).unwrap();

  let result = read_json(&dir, "config.json");
  assert_eq!(result["scripts"]["build"], "tsc");
}

#[test]
fn set_merges_an_object_value_into_an_existing_object() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_json(
    &dir,
    "package.json",
    &json!({ "scripts": { "test": "jest", "build": "tsc" } }),
  );

  let mut set = HashMap::new();
  set.insert("scripts".to_string(), json!({ "build": "vite build" }));
  let step = JsonPatchStep {
    path: "package.json".into(),
    set,
    remove: Vec::new(),
  };
  execute_json_patch(&step, dir.path(), &empty_ctx()).unwrap();

  let result = read_json(&dir, "package.json");
  assert_eq!(result["scripts"]["build"], "vite build");
  assert_eq!(
    result["scripts"]["test"], "jest",
    "merging must not drop sibling keys that were not part of the patch"
  );
}

#[test]
fn remove_deletes_a_key() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_json(
    &dir,
    "package.json",
    &json!({ "name": "app", "private": true }),
  );

  let step = JsonPatchStep {
    path: "package.json".into(),
    set: HashMap::new(),
    remove: vec!["private".to_string()],
  };
  execute_json_patch(&step, dir.path(), &empty_ctx()).unwrap();

  let result = read_json(&dir, "package.json");
  assert!(result.get("private").is_none());
  assert_eq!(result["name"], "app");
}

#[test]
fn remove_of_a_missing_key_is_a_no_op() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_json(&dir, "package.json", &json!({ "name": "app" }));

  let step = JsonPatchStep {
    path: "package.json".into(),
    set: HashMap::new(),
    remove: vec!["nonexistent".to_string()],
  };
  execute_json_patch(&step, dir.path(), &empty_ctx()).unwrap();

  let result = read_json(&dir, "package.json");
  assert_eq!(result["name"], "app");
}

#[test]
fn rollback_restores_the_original_bytes() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_json(&dir, "package.json", &json!({ "name": "app" }));
  let original_bytes = std::fs::read(dir.path().join("package.json")).unwrap();

  let mut set = HashMap::new();
  set.insert("name".to_string(), json!("renamed"));
  let step = JsonPatchStep {
    path: "package.json".into(),
    set,
    remove: Vec::new(),
  };
  let rollbacks = execute_json_patch(&step, dir.path(), &empty_ctx()).unwrap();

  assert_eq!(rollbacks.len(), 1);
  match &rollbacks[0] {
    Rollback::RestoreFile { original, .. } => assert_eq!(original, &original_bytes),
    other => panic!("expected RestoreFile, got {other:?}"),
  }
}

#[test]
fn invalid_json_is_refused() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("broken.json").write_str("{ not json").unwrap();

  let mut set = HashMap::new();
  set.insert("name".to_string(), json!("app"));
  let step = JsonPatchStep {
    path: "broken.json".into(),
    set,
    remove: Vec::new(),
  };
  let err = execute_json_patch(&step, dir.path(), &empty_ctx()).unwrap_err();
  assert!(err.to_string().contains("not valid JSON"));
  assert_eq!(
    std::fs::read_to_string(dir.path().join("broken.json")).unwrap(),
    "{ not json",
    "an invalid file must be left untouched"
  );
}

#[test]
fn missing_target_file_is_an_error() {
  let dir = assert_fs::TempDir::new().unwrap();
  let step = JsonPatchStep {
    path: "does-not-exist.json".into(),
    set: HashMap::new(),
    remove: Vec::new(),
  };
  assert!(execute_json_patch(&step, dir.path(), &empty_ctx()).is_err());
}
