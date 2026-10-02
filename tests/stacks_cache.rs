use anesis::context::{AppContext, CleanupState};
use anesis::paths::AnesisPaths;
use anesis::stacks::cache::{read_installed_stacks, remove_cached_stack, resolve_stack};
use assert_fs::TempDir;
use assert_fs::prelude::*;
use std::sync::{Arc, Mutex};

struct Fixture {
  home: TempDir,
  workdir: TempDir,
}

impl Fixture {
  fn new() -> Self {
    Self {
      home: TempDir::new().unwrap(),
      workdir: TempDir::new().unwrap(),
    }
  }

  fn ctx(&self) -> AppContext {
    let paths = AnesisPaths::under(self.home.path());
    paths.ensure_directories().unwrap();

    AppContext {
      paths,
      client: reqwest::Client::new(),
      cleanup_state: Arc::new(Mutex::new(None)) as CleanupState,
      backend_url: "http://127.0.0.1:1".to_string(),
      frontend_url: "http://127.0.0.1:1".to_string(),
      telemetry: false,
      allow_run: false,
    }
  }

  fn write_cached_stack(&self, id: &str, body: &str) {
    self
      .home
      .child(format!(".anesis/cache/stacks/{id}.json"))
      .write_str(body)
      .unwrap();
  }
}

fn stack_json(id: &str, name: &str, template: &str) -> String {
  format!(
    r#"{{
  "schema_version": "1",
  "id": "{id}",
  "name": "{name}",
  "description": "A test stack",
  "version": "1.0.0",
  "author": {{ "name": "anesis", "github": "anesis-dev" }},
  "template": "{template}",
  "addons": [
    {{ "id": "docker-compose", "command": "install", "inputs": {{}} }}
  ]
}}"#
  )
}

#[tokio::test]
async fn a_local_path_wins_over_the_cache_and_the_registry() {
  let fx = Fixture::new();
  fx.write_cached_stack(
    "my-stack",
    &stack_json("my-stack", "Cached", "cached-template"),
  );

  let local = fx.workdir.child("anesis.stack.json");
  local
    .write_str(&stack_json("my-stack", "Local", "local-template"))
    .unwrap();

  let resolved = resolve_stack(&fx.ctx(), local.path().to_str().unwrap())
    .await
    .unwrap();

  assert_eq!(
    resolved.name, "Local",
    "an explicit path must win, so local edits are actually used"
  );
  assert_eq!(resolved.template, "local-template");
}

#[tokio::test]
async fn a_directory_path_resolves_its_anesis_stack_json() {
  let fx = Fixture::new();
  fx.workdir
    .child("anesis.stack.json")
    .write_str(&stack_json("dir-stack", "From Directory", "some-template"))
    .unwrap();

  let resolved = resolve_stack(&fx.ctx(), fx.workdir.path().to_str().unwrap())
    .await
    .unwrap();

  assert_eq!(resolved.name, "From Directory");
}

#[tokio::test]
async fn a_same_named_directory_without_a_manifest_does_not_shadow_the_cache() {
  let fx = Fixture::new();
  fx.write_cached_stack("rust-api", &stack_json("rust-api", "Rust API", "axum"));
  fx.workdir.child("rust-api/src/main.rs").touch().unwrap();

  let previous = std::env::current_dir().unwrap();
  std::env::set_current_dir(fx.workdir.path()).unwrap();
  let resolved = resolve_stack(&fx.ctx(), "rust-api").await;
  std::env::set_current_dir(previous).unwrap();

  assert_eq!(resolved.unwrap().name, "Rust API");
}

#[tokio::test]
async fn the_cache_is_used_when_there_is_no_local_path() {
  let fx = Fixture::new();
  fx.write_cached_stack(
    "nest-saas",
    &stack_json("nest-saas", "Nest SaaS", "nest-express"),
  );

  let resolved = resolve_stack(&fx.ctx(), "nest-saas").await.unwrap();

  assert_eq!(resolved.name, "Nest SaaS");
  assert_eq!(resolved.template, "nest-express");
  assert_eq!(resolved.addons.len(), 1);
  assert_eq!(resolved.addons[0].id, "docker-compose");
}

#[tokio::test]
async fn an_unknown_stack_falls_through_to_the_registry_and_fails() {
  let fx = Fixture::new();

  let err = resolve_stack(&fx.ctx(), "does-not-exist")
    .await
    .unwrap_err();

  assert!(
    err.to_string().contains("does-not-exist"),
    "the error should name the stack: {err}"
  );
}

#[tokio::test]
async fn a_corrupt_cache_entry_is_an_error() {
  let fx = Fixture::new();
  fx.write_cached_stack("broken", "{ not json");

  let err = resolve_stack(&fx.ctx(), "broken").await.unwrap_err();
  assert!(
    err.to_string().contains("broken"),
    "the error should name the file: {err}"
  );
}

#[tokio::test]
async fn a_stack_without_a_template_is_rejected() {
  let fx = Fixture::new();
  fx.write_cached_stack(
    "empty",
    r#"{
  "schema_version": "1",
  "id": "empty",
  "name": "Empty",
  "version": "1.0.0",
  "author": { "name": "anesis", "github": "anesis-dev" },
  "template": "",
  "addons": []
}"#,
  );

  let err = resolve_stack(&fx.ctx(), "empty").await.unwrap_err();
  assert!(err.to_string().contains("no template"), "{err}");
}

#[tokio::test]
async fn an_addon_with_an_empty_id_is_rejected() {
  let fx = Fixture::new();
  fx.write_cached_stack(
    "bad-addon",
    r#"{
  "schema_version": "1",
  "id": "bad-addon",
  "name": "Bad Addon",
  "version": "1.0.0",
  "author": { "name": "anesis", "github": "anesis-dev" },
  "template": "some-template",
  "addons": [{ "id": "  ", "command": "install", "inputs": {} }]
}"#,
  );

  let err = resolve_stack(&fx.ctx(), "bad-addon").await.unwrap_err();
  assert!(err.to_string().contains("addon #1"), "{err}");
}

#[tokio::test]
async fn the_addon_command_defaults_to_install() {
  let fx = Fixture::new();
  fx.write_cached_stack(
    "defaults",
    r#"{
  "schema_version": "1",
  "id": "defaults",
  "name": "Defaults",
  "version": "1.0.0",
  "author": { "name": "anesis", "github": "anesis-dev" },
  "template": "some-template",
  "addons": [{ "id": "docker-compose" }]
}"#,
  );

  let resolved = resolve_stack(&fx.ctx(), "defaults").await.unwrap();
  assert_eq!(resolved.addons[0].command, "install");
}

#[test]
fn read_installed_stacks_is_empty_when_nothing_is_cached() {
  let fx = Fixture::new();
  assert!(read_installed_stacks(&fx.ctx()).unwrap().is_empty());
}

#[test]
fn read_installed_stacks_lists_every_valid_cached_manifest() {
  let fx = Fixture::new();
  let ctx = fx.ctx();
  fx.write_cached_stack("one", &stack_json("one", "One", "t1"));
  fx.write_cached_stack("two", &stack_json("two", "Two", "t2"));

  let mut names: Vec<String> = read_installed_stacks(&ctx)
    .unwrap()
    .into_iter()
    .map(|s| s.name)
    .collect();
  names.sort();

  assert_eq!(names, vec!["One", "Two"]);
}

#[test]
fn read_installed_stacks_skips_unparseable_files() {
  let fx = Fixture::new();
  let ctx = fx.ctx();
  fx.write_cached_stack("good", &stack_json("good", "Good", "t1"));
  fx.write_cached_stack("bad", "{ not json");

  let stacks = read_installed_stacks(&ctx).unwrap();
  assert_eq!(stacks.len(), 1);
  assert_eq!(stacks[0].name, "Good");
}

#[test]
fn read_installed_stacks_ignores_other_files() {
  let fx = Fixture::new();
  let ctx = fx.ctx();
  fx.write_cached_stack("good", &stack_json("good", "Good", "t1"));
  fx.home
    .child(".anesis/cache/stacks/README.md")
    .write_str("# not a stack")
    .unwrap();

  assert_eq!(read_installed_stacks(&ctx).unwrap().len(), 1);
}

#[test]
fn removing_a_cached_stack_deletes_it() {
  let fx = Fixture::new();
  let ctx = fx.ctx();
  fx.write_cached_stack("gone", &stack_json("gone", "Gone", "t1"));

  remove_cached_stack(&ctx, "gone").unwrap();

  assert!(read_installed_stacks(&ctx).unwrap().is_empty());
}

#[test]
fn removing_a_stack_that_is_not_installed_is_an_error() {
  let fx = Fixture::new();
  let err = remove_cached_stack(&fx.ctx(), "never-installed").unwrap_err();
  assert!(err.to_string().contains("never-installed"), "{err}");
}

#[tokio::test]
async fn link_caches_a_local_stack_under_its_declared_id() {
  use anesis::stacks::link::link_stack;

  let fx = Fixture::new();
  let ctx = fx.ctx();

  fx.workdir
    .child("anesis.stack.json")
    .write_str(&stack_json("linked-stack", "Linked", "react-vite"))
    .unwrap();

  let id = link_stack(&ctx, fx.workdir.path(), false).unwrap();
  assert_eq!(id.as_deref(), Some("linked-stack"));

  let resolved = resolve_stack(&ctx, "linked-stack").await.unwrap();
  assert_eq!(resolved.name, "Linked");
  assert_eq!(resolved.template, "react-vite");
}

#[tokio::test]
async fn link_accepts_the_manifest_path_directly() {
  use anesis::stacks::link::link_stack;

  let fx = Fixture::new();
  let ctx = fx.ctx();

  let manifest = fx.workdir.child("anesis.stack.json");
  manifest
    .write_str(&stack_json("direct-stack", "Direct", "react-vite"))
    .unwrap();

  let id = link_stack(&ctx, manifest.path(), false).unwrap();
  assert_eq!(id.as_deref(), Some("direct-stack"));
}

#[tokio::test]
async fn link_rejects_a_directory_without_a_manifest() {
  use anesis::stacks::link::link_stack;

  let fx = Fixture::new();
  let err = link_stack(&fx.ctx(), fx.workdir.path(), false).unwrap_err();
  assert!(
    err.to_string().contains("stack manifest"),
    "unexpected error: {err:#}"
  );
}

#[tokio::test]
async fn a_future_schema_version_is_refused_rather_than_applied() {
  let fx = Fixture::new();
  fx.write_cached_stack(
    "future-stack",
    &stack_json("future-stack", "Future", "react-vite")
      .replace(r#""schema_version": "1""#, r#""schema_version": "99""#),
  );

  let err = resolve_stack(&fx.ctx(), "future-stack").await.unwrap_err();
  let message = format!("{err:#}");
  assert!(message.contains("future-stack"), "{message}");
  assert!(message.contains("schema version 99"), "{message}");
  assert!(message.contains("anesis upgrade"), "{message}");
}
