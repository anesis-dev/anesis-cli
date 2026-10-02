#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use anesis::context::{AppContext, CleanupState};
use anesis::paths::AnesisPaths;
use assert_fs::TempDir;
use assert_fs::prelude::*;

pub struct Fixture {
  pub home: TempDir,
  pub project: TempDir,
}

impl Fixture {
  pub fn new() -> Self {
    Self {
      home: TempDir::new().unwrap(),
      project: TempDir::new().unwrap(),
    }
  }

  pub fn paths(&self) -> AnesisPaths {
    AnesisPaths::under(self.home.path())
  }

  pub fn offline_ctx(&self) -> AppContext {
    offline_ctx(&self.home)
  }

  pub fn mock_ctx(&self, server: &wiremock::MockServer) -> AppContext {
    mock_ctx(server, &self.home)
  }

  pub fn seed_project(&self) {
    self
      .project
      .child("anesis.json")
      .write_str(r#"{"template_name":"fixture","template_sha":"abc123","addons":[]}"#)
      .unwrap();
  }

  pub fn install_addon(&self, id: &str, manifest: &serde_json::Value) {
    build::write_addon_to_cache(&self.home, id, manifest);
  }
}

impl Default for Fixture {
  fn default() -> Self {
    Self::new()
  }
}

fn base_ctx(home: &TempDir) -> AppContext {
  let paths = AnesisPaths::under(home.path());
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

pub fn offline_ctx(home: &TempDir) -> AppContext {
  base_ctx(home)
}

pub fn mock_ctx(server: &wiremock::MockServer, home: &TempDir) -> AppContext {
  let mut ctx = base_ctx(home);
  ctx.backend_url = server.uri();
  ctx.frontend_url = server.uri();
  ctx
}

pub mod build {
  use super::*;

  pub fn addon_manifest(id: &str, version: &str) -> serde_json::Value {
    serde_json::json!({
      "schema_version": "1",
      "id": id,
      "name": id,
      "version": version,
      "description": "Test fixture",
      "author": "anesis",
      "requires": [],
      "inputs": [],
      "detect": [],
      "variants": [{
        "when": null,
        "commands": [{
          "name": "install",
          "description": "",
          "once": true,
          "requires_commands": [],
          "inputs": [],
          "steps": [
            { "type": "create", "path": "generated.txt", "content": "hello\n", "if_exists": "overwrite" }
          ]
        }]
      }]
    })
  }

  pub fn write_addon_to_cache(home: &TempDir, id: &str, manifest: &serde_json::Value) {
    let addons = home.child(".anesis/cache/addons");
    addons
      .child(format!("{id}/anesis.addon.json"))
      .write_str(&serde_json::to_string_pretty(manifest).unwrap())
      .unwrap();

    let version = manifest["version"].as_str().unwrap_or("0.0.0");
    addons
      .child("anesis-addons.json")
      .write_str(&format!(
        r#"{{
  "lastUpdated": "2026-01-01T00:00:00Z",
  "addons": [
    {{
      "id": "{id}",
      "name": "{id}",
      "version": "{version}",
      "path": "{id}",
      "commit_sha": "deadbeef",
      "repo_url": "https://github.com/anesis-dev/addons"
    }}
  ]
}}"#
      ))
      .unwrap();
  }

  pub fn template_manifest(name: &str, version: &str) -> serde_json::Value {
    serde_json::json!({
      "name": name,
      "version": version,
      "anesisVersion": ">=0.5.0",
      "author": { "name": "anesis", "github": "anesis-dev" },
      "repository": { "url": "https://github.com/anesis-dev/templates" },
      "specialization": "frontend",
      "scope": "web",
      "technologies": [],
      "languages": [],
      "type": "base",
      "metadata": {
        "displayName": name,
        "description": "Test fixture template",
        "tags": []
      },
      "inputs": []
    })
  }

  pub fn write_template_dir(dir: &TempDir, name: &str, version: &str) {
    dir
      .child("anesis.template.json")
      .write_str(&serde_json::to_string_pretty(&template_manifest(name, version)).unwrap())
      .unwrap();
    dir
      .child("README.md")
      .write_str(&format!("# {name}\n"))
      .unwrap();
  }

  pub fn stack_manifest(id: &str, name: &str, template: &str) -> String {
    format!(
      r#"{{
  "schema_version": "1",
  "id": "{id}",
  "name": "{name}",
  "description": "A test stack",
  "version": "1.0.0",
  "author": {{ "name": "anesis", "github": "anesis-dev" }},
  "template": "{template}",
  "addons": []
}}"#
    )
  }

  pub fn targz(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut tar_bytes = Vec::new();
    {
      let mut builder = tar::Builder::new(&mut tar_bytes);
      for (path, contents) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, path, *contents).unwrap();
      }
      builder.finish().unwrap();
    }

    let mut gz_bytes = Vec::new();
    {
      let mut encoder =
        flate2::write::GzEncoder::new(&mut gz_bytes, flate2::Compression::default());
      std::io::Write::write_all(&mut encoder, &tar_bytes).unwrap();
      encoder.finish().unwrap();
    }
    gz_bytes
  }
}
