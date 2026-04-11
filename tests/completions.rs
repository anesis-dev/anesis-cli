use assert_fs::prelude::*;
use oxide_cli::completions::print_dynamic_completions;

// ── print_dynamic_completions ─────────────────────────────────────────────────

#[test]
fn no_index_file_does_not_crash() {
  let dir = assert_fs::TempDir::new().unwrap();
  // Should not panic — silently ignores missing cache
  print_dynamic_completions(dir.path(), None);
}

#[test]
fn with_addon_id_no_manifest_does_not_crash() {
  let dir = assert_fs::TempDir::new().unwrap();
  // Should not panic — silently ignores missing manifest
  print_dynamic_completions(dir.path(), Some("drizzle"));
}

#[test]
fn with_populated_cache_does_not_crash() {
  let dir = assert_fs::TempDir::new().unwrap();
  let cache = serde_json::json!({
    "lastUpdated": "2024-01-01T00:00:00Z",
    "addons": [
      {
        "id": "drizzle",
        "name": "Drizzle ORM",
        "version": "1.0.0",
        "path": "drizzle",
        "commit_sha": "abc123",
        "repo_url": ""
      }
    ]
  });
  dir.child("oxide-addons.json").write_str(&cache.to_string()).unwrap();

  print_dynamic_completions(dir.path(), None);
}

#[test]
fn with_addon_manifest_does_not_crash() {
  let dir = assert_fs::TempDir::new().unwrap();
  let manifest = serde_json::json!({
    "schema_version": "1",
    "id": "drizzle",
    "name": "Drizzle ORM",
    "version": "1.0.0",
    "description": "",
    "author": "test",
    "variants": [
      {
        "when": null,
        "commands": [
          { "name": "install", "description": "", "once": false, "requires_commands": [], "inputs": [], "steps": [] },
          { "name": "migrate", "description": "", "once": false, "requires_commands": [], "inputs": [], "steps": [] }
        ]
      }
    ]
  });
  dir.child("drizzle").child("oxide.addon.json")
    .write_str(&manifest.to_string())
    .unwrap();

  print_dynamic_completions(dir.path(), Some("drizzle"));
}

#[test]
fn malformed_cache_does_not_crash() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("oxide-addons.json").write_str("not valid json {{{{").unwrap();

  // Should silently ignore parse error
  print_dynamic_completions(dir.path(), None);
}

#[test]
fn malformed_manifest_does_not_crash() {
  let dir = assert_fs::TempDir::new().unwrap();
  dir.child("drizzle").child("oxide.addon.json").write_str("{ bad json").unwrap();

  print_dynamic_completions(dir.path(), Some("drizzle"));
}
