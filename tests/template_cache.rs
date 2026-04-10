use assert_fs::prelude::*;
use oxide_cli::cache::{TemplatesCache, get_installed_templates, remove_template_from_cache, update_templates_cache};

fn write_oxide_template_json(dir: &assert_fs::TempDir, subdir: &str, name: &str) {
  let json = serde_json::json!({
    "name": name,
    "version": "1.0.0",
    "oxideVersion": "0.5.0",
    "official": true,
    "repository": { "url": "https://github.com/example/repo" },
    "metadata": { "displayName": name, "description": "test" }
  });
  dir
    .child(subdir)
    .child("oxide.template.json")
    .write_str(&json.to_string())
    .unwrap();
}

#[test]
fn update_cache_adds_entry() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_oxide_template_json(&dir, "react-vite", "react-vite");

  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "abc123").unwrap();

  let content = std::fs::read_to_string(dir.path().join("oxide-templates.json")).unwrap();
  let cache: TemplatesCache = serde_json::from_str(&content).unwrap();

  assert_eq!(cache.templates.len(), 1);
  assert_eq!(cache.templates[0].name, "react-vite");
  assert_eq!(cache.templates[0].commit_sha, "abc123");
}

#[test]
fn update_cache_replaces_duplicate() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_oxide_template_json(&dir, "react-vite", "react-vite");

  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "aaa").unwrap();
  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "bbb").unwrap();

  let content = std::fs::read_to_string(dir.path().join("oxide-templates.json")).unwrap();
  let cache: TemplatesCache = serde_json::from_str(&content).unwrap();

  assert_eq!(cache.templates.len(), 1);
  assert_eq!(cache.templates[0].commit_sha, "bbb");
}

#[test]
fn remove_template_removes_entry_and_dir() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_oxide_template_json(&dir, "react-vite", "react-vite");
  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "abc").unwrap();
  dir.child("react-vite").child("index.js").write_str("").unwrap();

  remove_template_from_cache(dir.path(), "react-vite").unwrap();

  let content = std::fs::read_to_string(dir.path().join("oxide-templates.json")).unwrap();
  let cache: TemplatesCache = serde_json::from_str(&content).unwrap();

  assert!(cache.templates.is_empty());
  assert!(!dir.path().join("react-vite").exists());
}

#[test]
fn remove_template_not_installed_is_err() {
  let dir = assert_fs::TempDir::new().unwrap();
  write_oxide_template_json(&dir, "react-vite", "react-vite");
  update_templates_cache(dir.path(), std::path::Path::new("react-vite"), "abc").unwrap();

  assert!(remove_template_from_cache(dir.path(), "nonexistent").is_err());
}

#[test]
fn get_installed_templates_no_file_is_ok() {
  let dir = assert_fs::TempDir::new().unwrap();
  assert!(get_installed_templates(dir.path()).is_ok());
}
