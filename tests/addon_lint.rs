use anesis::addons::lint::lint_addon;
use assert_fs::TempDir;
use assert_fs::prelude::*;

fn write_manifest(dir: impl AsRef<std::path::Path>, manifest: &serde_json::Value) {
  std::fs::write(
    dir.as_ref().join("anesis.addon.json"),
    serde_json::to_string_pretty(manifest).unwrap(),
  )
  .unwrap();
}

fn valid_manifest(id: &str) -> serde_json::Value {
  serde_json::json!({
    "schema_version": "1",
    "id": id,
    "name": id,
    "version": "1.0.0",
    "description": "lint fixture",
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
          { "type": "create", "path": "generated.txt", "content": "hi\n", "if_exists": "overwrite" }
        ]
      }]
    }]
  })
}

#[test]
fn a_clean_addon_has_no_findings() {
  let dir = TempDir::new().unwrap();
  let renamed = dir.child("lint-fixture");
  renamed.create_dir_all().unwrap();
  write_manifest(&renamed, &valid_manifest("lint-fixture"));

  let errors = lint_addon(renamed.path()).unwrap();
  assert!(errors.is_empty(), "unexpected findings: {errors:?}");
}

#[test]
fn id_not_matching_directory_is_flagged() {
  let dir = TempDir::new().unwrap();
  let renamed = dir.child("on-disk-name");
  renamed.create_dir_all().unwrap();
  write_manifest(&renamed, &valid_manifest("declared-id"));

  let errors = lint_addon(renamed.path()).unwrap();
  assert!(
    errors
      .iter()
      .any(|e| e.contains("does not match its directory")),
    "{errors:?}"
  );
}

#[test]
fn a_copy_step_pointing_at_a_missing_source_is_flagged() {
  let dir = TempDir::new().unwrap();
  let renamed = dir.child("lint-fixture");
  renamed.create_dir_all().unwrap();
  let mut manifest = valid_manifest("lint-fixture");
  manifest["variants"][0]["commands"][0]["steps"] = serde_json::json!([
    { "type": "copy", "src": "missing-file.txt", "dest": "out.txt" }
  ]);
  write_manifest(&renamed, &manifest);

  let errors = lint_addon(renamed.path()).unwrap();
  assert!(
    errors.iter().any(|e| e.contains("does not exist")),
    "{errors:?}"
  );
}

#[test]
fn a_copy_step_pointing_at_a_real_file_is_accepted() {
  let dir = TempDir::new().unwrap();
  let renamed = dir.child("lint-fixture");
  renamed.create_dir_all().unwrap();
  renamed.child("real-file.txt").write_str("hi\n").unwrap();
  let mut manifest = valid_manifest("lint-fixture");
  manifest["variants"][0]["commands"][0]["steps"] = serde_json::json!([
    { "type": "copy", "src": "real-file.txt", "dest": "out.txt" }
  ]);
  write_manifest(&renamed, &manifest);

  let errors = lint_addon(renamed.path()).unwrap();
  assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn requires_commands_naming_an_unknown_command_is_flagged() {
  let dir = TempDir::new().unwrap();
  let renamed = dir.child("lint-fixture");
  renamed.create_dir_all().unwrap();
  let mut manifest = valid_manifest("lint-fixture");
  manifest["variants"][0]["commands"][0]["requires_commands"] =
    serde_json::json!(["does-not-exist"]);
  write_manifest(&renamed, &manifest);

  let errors = lint_addon(renamed.path()).unwrap();
  assert!(
    errors.iter().any(|e| e.contains("does-not-exist")),
    "{errors:?}"
  );
}

#[test]
fn requires_commands_naming_a_real_command_is_accepted() {
  let dir = TempDir::new().unwrap();
  let renamed = dir.child("lint-fixture");
  renamed.create_dir_all().unwrap();
  let mut manifest = valid_manifest("lint-fixture");
  manifest["variants"][0]["commands"] = serde_json::json!([
    {
      "name": "install",
      "description": "",
      "once": true,
      "requires_commands": [],
      "inputs": [],
      "steps": []
    },
    {
      "name": "configure",
      "description": "",
      "once": false,
      "requires_commands": ["install"],
      "inputs": [],
      "steps": []
    }
  ]);
  write_manifest(&renamed, &manifest);

  let errors = lint_addon(renamed.path()).unwrap();
  assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn a_fixture_missing_the_injection_anchor_is_flagged() {
  let dir = TempDir::new().unwrap();
  let renamed = dir.child("lint-fixture");
  renamed.create_dir_all().unwrap();
  let mut manifest = valid_manifest("lint-fixture");
  manifest["variants"][0]["commands"][0]["steps"] = serde_json::json!([
    {
      "type": "inject",
      "target": { "type": "file", "file": "README.md" },
      "content": "extra",
      "after": "MISSING ANCHOR",
      "if_not_found": "error"
    }
  ]);
  write_manifest(&renamed, &manifest);
  renamed
    .child("test-fixture/README.md")
    .write_str("# hello\n")
    .unwrap();

  let errors = lint_addon(renamed.path()).unwrap();
  assert!(errors.iter().any(|e| e.contains("no anchor")), "{errors:?}");
}

#[test]
fn a_fixture_with_the_anchor_present_is_accepted() {
  let dir = TempDir::new().unwrap();
  let renamed = dir.child("lint-fixture");
  renamed.create_dir_all().unwrap();
  let mut manifest = valid_manifest("lint-fixture");
  manifest["variants"][0]["commands"][0]["steps"] = serde_json::json!([
    {
      "type": "inject",
      "target": { "type": "file", "file": "README.md" },
      "content": "extra",
      "after": "hello",
      "if_not_found": "error"
    }
  ]);
  write_manifest(&renamed, &manifest);
  renamed
    .child("test-fixture/README.md")
    .write_str("# hello\n")
    .unwrap();

  let errors = lint_addon(renamed.path()).unwrap();
  assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn a_fixture_missing_the_target_file_entirely_is_flagged() {
  let dir = TempDir::new().unwrap();
  let renamed = dir.child("lint-fixture");
  renamed.create_dir_all().unwrap();
  let mut manifest = valid_manifest("lint-fixture");
  manifest["variants"][0]["commands"][0]["steps"] = serde_json::json!([
    {
      "type": "inject",
      "target": { "type": "file", "file": "does-not-exist.md" },
      "content": "extra",
      "after": "hello",
      "if_not_found": "error"
    }
  ]);
  write_manifest(&renamed, &manifest);
  renamed.child("test-fixture").create_dir_all().unwrap();

  let errors = lint_addon(renamed.path()).unwrap();
  assert!(
    errors.iter().any(|e| e.contains("is missing")),
    "{errors:?}"
  );
}

#[test]
fn a_command_with_prerequisites_is_not_checked_against_the_bare_fixture() {
  let dir = TempDir::new().unwrap();
  let renamed = dir.child("lint-fixture");
  renamed.create_dir_all().unwrap();
  let mut manifest = valid_manifest("lint-fixture");
  manifest["variants"][0]["commands"] = serde_json::json!([
    {
      "name": "install",
      "description": "",
      "once": true,
      "requires_commands": [],
      "inputs": [],
      "steps": []
    },
    {
      "name": "configure",
      "description": "",
      "once": false,
      "requires_commands": ["install"],
      "inputs": [],
      "steps": [
        {
          "type": "inject",
          "target": { "type": "file", "file": "generated-by-install.txt" },
          "content": "extra",
          "after": "anchor",
          "if_not_found": "error"
        }
      ]
    }
  ]);
  write_manifest(&renamed, &manifest);
  renamed.child("test-fixture").create_dir_all().unwrap();

  let errors = lint_addon(renamed.path()).unwrap();
  assert!(
    errors.is_empty(),
    "a command gated on requires_commands must be skipped against the bare fixture: {errors:?}"
  );
}

#[test]
fn a_manifest_that_fails_schema_validation_is_an_error_not_a_finding() {
  let dir = TempDir::new().unwrap();
  let renamed = dir.child("lint-fixture");
  renamed.create_dir_all().unwrap();
  let mut manifest = valid_manifest("../escaping-id");
  manifest["id"] = serde_json::json!("../escaping-id");
  write_manifest(&renamed, &manifest);

  assert!(lint_addon(renamed.path()).is_err());
}

#[test]
fn a_missing_manifest_is_an_error() {
  let dir = TempDir::new().unwrap();
  assert!(lint_addon(dir.path()).is_err());
}
