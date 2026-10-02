mod common;

use anesis::addons::lock::LockFile;
use anesis::addons::runner::{list_addon_commands, run_addon_command, undo_addon};
use anesis::addons::steps::Rollback;
use anesis::context::{AppContext, CleanupState};
use anesis::paths::AnesisPaths;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use common::{
  is_newer_for_tests, rerun_prompt_message_for_tests, step_label_for_tests,
  undo_conflicts_for_tests,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

struct Fixture {
  home: TempDir,
  project: TempDir,
}

impl Fixture {
  fn new() -> Self {
    Self {
      home: TempDir::new().unwrap(),
      project: TempDir::new().unwrap(),
    }
  }

  fn paths(&self) -> AnesisPaths {
    AnesisPaths::under(self.home.path())
  }

  fn ctx(&self) -> AppContext {
    let paths = self.paths();
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

  fn install_addon(&self, id: &str, manifest: serde_json::Value) {
    let addons = self.home.child(".anesis/cache/addons");
    addons
      .child(format!("{id}/anesis.addon.json"))
      .write_str(&serde_json::to_string_pretty(&manifest).unwrap())
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

  fn seed_project(&self) {
    self
      .project
      .child("anesis.json")
      .write_str(r#"{"template_name":"fixture","template_sha":"abc123","addons":[]}"#)
      .unwrap();
  }

  fn lock(&self) -> LockFile {
    LockFile::load(self.project.path()).unwrap()
  }
}

fn reversible_addon(version: &str) -> serde_json::Value {
  serde_json::json!({
    "schema_version": "1",
    "id": "fixture-addon",
    "name": "Fixture Addon",
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
          { "type": "create", "path": "generated.txt", "content": "hello\n", "if_exists": "overwrite" },
          { "type": "append", "target": { "type": "file", "file": "existing.txt" }, "content": "appended\n" }
        ]
      }]
    }]
  })
}

#[tokio::test]
async fn applying_a_command_writes_files_and_records_the_lock_entry() {
  let fx = Fixture::new();
  fx.seed_project();
  fx.install_addon("fixture-addon", reversible_addon("1.0.0"));
  fx.project
    .child("existing.txt")
    .write_str("base\n")
    .unwrap();

  run_addon_command(
    &fx.ctx(),
    "fixture-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .unwrap();

  fx.project.child("generated.txt").assert("hello\n");
  fx.project.child("existing.txt").assert("base\nappended\n");

  let lock = fx.lock();
  assert_eq!(lock.addons.len(), 1);
  assert_eq!(lock.addons[0].id, "fixture-addon");
  assert_eq!(lock.addons[0].version, "1.0.0");
  assert_eq!(lock.addons[0].commands_executed(), vec!["install"]);
  assert!(
    lock.addons[0].has_undoable_changes(),
    "the rollback journal must be persisted, or `anesis undo` has nothing to work with"
  );
}

#[tokio::test]
async fn a_dry_run_changes_nothing() {
  let fx = Fixture::new();
  fx.seed_project();
  fx.install_addon("fixture-addon", reversible_addon("1.0.0"));
  fx.project
    .child("existing.txt")
    .write_str("base\n")
    .unwrap();

  run_addon_command(
    &fx.ctx(),
    "fixture-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    false,
    true,
  )
  .await
  .unwrap();

  assert!(!fx.project.path().join("generated.txt").exists());
  fx.project.child("existing.txt").assert("base\n");
  assert!(
    fx.lock().addons.is_empty(),
    "a dry run must not touch anesis.lock"
  );
}

#[tokio::test]
async fn an_unknown_command_is_rejected() {
  let fx = Fixture::new();
  fx.seed_project();
  fx.install_addon("fixture-addon", reversible_addon("1.0.0"));

  let err = run_addon_command(
    &fx.ctx(),
    "fixture-addon",
    "nope",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .unwrap_err();

  assert!(err.to_string().contains("nope"), "{err}");
}

#[tokio::test]
async fn list_addon_commands_reports_none_available_when_no_variant_matches_and_there_is_no_universal_fallback()
 {
  let fx = Fixture::new();
  fx.seed_project();
  let manifest = serde_json::json!({
    "schema_version": "1",
    "id": "fixture-addon",
    "name": "Fixture Addon",
    "version": "1.0.0",
    "description": "",
    "author": "anesis",
    "requires": [],
    "inputs": [],
    "detect": [],
    "variants": [{
      "when": "only-matches-nothing",
      "commands": [{
        "name": "install",
        "description": "",
        "once": false,
        "requires_commands": [],
        "inputs": [],
        "steps": []
      }]
    }]
  });
  fx.install_addon("fixture-addon", manifest);

  list_addon_commands(
    &fx.ctx(),
    "fixture-addon",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .unwrap();
}

#[tokio::test]
async fn a_missing_required_addon_blocks_the_apply() {
  let fx = Fixture::new();
  fx.seed_project();

  let mut manifest = reversible_addon("1.0.0");
  manifest["requires"] = serde_json::json!(["some-base-addon"]);
  fx.install_addon("fixture-addon", manifest);

  let err = run_addon_command(
    &fx.ctx(),
    "fixture-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .unwrap_err();

  assert!(err.to_string().contains("some-base-addon"), "{err}");
  assert!(!fx.project.path().join("generated.txt").exists());
}

#[tokio::test]
async fn a_command_that_requires_another_is_blocked_until_it_has_run() {
  let fx = Fixture::new();
  fx.seed_project();

  let mut manifest = reversible_addon("1.0.0");
  manifest["variants"][0]["commands"]
    .as_array_mut()
    .unwrap()
    .push(serde_json::json!({
      "name": "configure",
      "description": "",
      "once": false,
      "requires_commands": ["install"],
      "inputs": [],
      "steps": [
        { "type": "create", "path": "configured.txt", "content": "ok\n", "if_exists": "overwrite" }
      ]
    }));
  fx.install_addon("fixture-addon", manifest);
  fx.project
    .child("existing.txt")
    .write_str("base\n")
    .unwrap();

  let err = run_addon_command(
    &fx.ctx(),
    "fixture-addon",
    "configure",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .unwrap_err();
  assert!(err.to_string().contains("install"), "{err}");

  run_addon_command(
    &fx.ctx(),
    "fixture-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .unwrap();
  run_addon_command(
    &fx.ctx(),
    "fixture-addon",
    "configure",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .unwrap();

  fx.project.child("configured.txt").assert("ok\n");
}

#[tokio::test]
async fn a_once_command_is_skipped_on_re_run_at_the_same_version() {
  let fx = Fixture::new();
  fx.seed_project();
  fx.install_addon("fixture-addon", reversible_addon("1.0.0"));
  fx.project
    .child("existing.txt")
    .write_str("base\n")
    .unwrap();

  let ctx = fx.ctx();
  for _ in 0..2 {
    run_addon_command(
      &ctx,
      "fixture-addon",
      "install",
      fx.project.path(),
      &HashMap::new(),
      true,
      false,
    )
    .await
    .unwrap();
  }

  fx.project.child("existing.txt").assert("base\nappended\n");
  assert_eq!(fx.lock().addons.len(), 1);
}

#[tokio::test]
async fn a_failing_step_rolls_back_the_earlier_ones() {
  let fx = Fixture::new();
  fx.seed_project();

  let manifest = serde_json::json!({
    "schema_version": "1",
    "id": "failing-addon",
    "name": "Failing Addon",
    "version": "1.0.0",
    "description": "",
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
          { "type": "create", "path": "first.txt", "content": "one\n", "if_exists": "overwrite" },
          { "type": "create", "path": "second.txt", "content": "two\n", "if_exists": "overwrite" },
          { "type": "append", "target": { "type": "file", "file": "does-not-exist.txt" }, "content": "boom\n" }
        ]
      }]
    }]
  });
  fx.install_addon("failing-addon", manifest);

  let err = run_addon_command(
    &fx.ctx(),
    "failing-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .unwrap_err();
  assert!(err.to_string().contains("step 3"), "{err}");

  assert!(
    !fx.project.path().join("first.txt").exists(),
    "step 1 must have been rolled back"
  );
  assert!(
    !fx.project.path().join("second.txt").exists(),
    "step 2 must have been rolled back"
  );
  assert!(
    fx.lock().addons.is_empty(),
    "a failed apply must not leave a lock entry"
  );
}

#[tokio::test]
async fn undo_reverses_every_step_and_clears_the_lock() {
  let fx = Fixture::new();
  fx.seed_project();
  fx.install_addon("fixture-addon", reversible_addon("1.0.0"));
  fx.project
    .child("existing.txt")
    .write_str("base\n")
    .unwrap();

  run_addon_command(
    &fx.ctx(),
    "fixture-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .unwrap();

  undo_addon("fixture-addon", fx.project.path(), true).unwrap();

  assert!(
    !fx.project.path().join("generated.txt").exists(),
    "a created file must be removed"
  );
  fx.project.child("existing.txt").assert("base\n");
  assert!(
    fx.lock().addons.is_empty(),
    "the lock entry must be gone after undo"
  );
}

#[tokio::test]
async fn undo_of_an_unapplied_addon_is_an_error() {
  let fx = Fixture::new();
  fx.seed_project();

  let err = undo_addon("never-applied", fx.project.path(), true).unwrap_err();
  assert!(err.to_string().contains("never-applied"), "{err}");
}

#[tokio::test]
async fn undo_reverses_every_command_that_was_applied() {
  let fx = Fixture::new();
  fx.seed_project();

  let mut manifest = reversible_addon("1.0.0");
  manifest["variants"][0]["commands"]
    .as_array_mut()
    .unwrap()
    .push(serde_json::json!({
      "name": "extra",
      "description": "",
      "once": false,
      "requires_commands": [],
      "inputs": [],
      "steps": [
        { "type": "create", "path": "extra.txt", "content": "extra\n", "if_exists": "overwrite" }
      ]
    }));
  fx.install_addon("fixture-addon", manifest);
  fx.project
    .child("existing.txt")
    .write_str("base\n")
    .unwrap();

  let ctx = fx.ctx();
  for command in ["install", "extra"] {
    run_addon_command(
      &ctx,
      "fixture-addon",
      command,
      fx.project.path(),
      &HashMap::new(),
      true,
      false,
    )
    .await
    .unwrap();
  }

  undo_addon("fixture-addon", fx.project.path(), true).unwrap();

  assert!(!fx.project.path().join("generated.txt").exists());
  assert!(!fx.project.path().join("extra.txt").exists());
  fx.project.child("existing.txt").assert("base\n");
}

#[tokio::test]
async fn preset_inputs_are_rendered_into_steps() {
  let fx = Fixture::new();
  fx.seed_project();

  let manifest = serde_json::json!({
    "schema_version": "1",
    "id": "input-addon",
    "name": "Input Addon",
    "version": "1.0.0",
    "description": "",
    "author": "anesis",
    "requires": [],
    "inputs": [
      { "name": "service_name", "type": "text", "description": "Service name", "default": "svc", "required": false, "options": [] }
    ],
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
          {
            "type": "create",
            "path": "{{ service_name }}.txt",
            "content": "{{ service_name_pascal }}\n",
            "if_exists": "overwrite"
          }
        ]
      }]
    }]
  });
  fx.install_addon("input-addon", manifest);

  let mut presets = HashMap::new();
  presets.insert("service_name".to_string(), "billing-api".to_string());

  run_addon_command(
    &fx.ctx(),
    "input-addon",
    "install",
    fx.project.path(),
    &presets,
    true,
    false,
  )
  .await
  .unwrap();

  fx.project.child("billing-api.txt").assert("BillingApi\n");
}

#[tokio::test]
async fn a_required_input_with_no_value_fails_non_interactively() {
  let fx = Fixture::new();
  fx.seed_project();

  let manifest = serde_json::json!({
    "schema_version": "1",
    "id": "input-addon",
    "name": "Input Addon",
    "version": "1.0.0",
    "description": "",
    "author": "anesis",
    "requires": [],
    "inputs": [
      { "name": "api_key", "type": "text", "description": "API key", "default": null, "required": true, "options": [] }
    ],
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
          { "type": "create", "path": "out.txt", "content": "{{ api_key }}\n", "if_exists": "overwrite" }
        ]
      }]
    }]
  });
  fx.install_addon("input-addon", manifest);

  let err = run_addon_command(
    &fx.ctx(),
    "input-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .unwrap_err();

  assert!(err.to_string().contains("api_key"), "{err}");
  assert!(!fx.project.path().join("out.txt").exists());
}

#[test]
fn is_newer_compares_semver_not_strings() {
  assert!(is_newer_for_tests("0.2.0", "0.1.0"));
  assert!(is_newer_for_tests("0.10.0", "0.9.0"));
  assert!(!is_newer_for_tests("0.1.0", "0.1.0"));
  assert!(!is_newer_for_tests("0.1.0", "0.2.0"));
  assert!(is_newer_for_tests("2024-05", "2024-04"));
}

#[test]
fn undo_conflicts_does_not_false_positive_on_a_freshly_applied_rename() {
  // Rollback::RenameFile{from, to} stores the *undo* direction: `from` is
  // where the renamed step's execute_rename() left the file (its current,
  // post-apply location), `to` is where undo will move it back to (the
  // original location, which is expected to be empty right after apply).
  let dir = TempDir::new().unwrap();
  let current_location = dir.path().join("renamed.txt");
  let original_location = dir.path().join("original.txt");
  std::fs::write(&current_location, "content").unwrap();

  let tagged = vec![(
    0usize,
    Rollback::RenameFile {
      from: current_location,
      to: original_location,
    },
  )];
  let conflicts = undo_conflicts_for_tests(&tagged);
  assert!(
    conflicts.is_empty(),
    "a normal, untouched rename must never report a conflict — the original location is \
     expected to be empty right after the rename was applied: {conflicts:?}"
  );
}

#[test]
fn undo_conflicts_flags_a_rename_whose_current_location_is_missing() {
  let dir = TempDir::new().unwrap();
  let current_location = dir.path().join("renamed.txt");
  let original_location = dir.path().join("original.txt");
  // `current_location` was never created (or was since deleted) — the file
  // the rollback needs to rename back is genuinely gone.

  let tagged = vec![(
    0usize,
    Rollback::RenameFile {
      from: current_location,
      to: original_location,
    },
  )];
  let conflicts = undo_conflicts_for_tests(&tagged);
  assert_eq!(conflicts.len(), 1, "{conflicts:?}");
}

#[test]
fn is_newer_never_treats_an_empty_latest_version_as_an_upgrade() {
  assert!(
    !is_newer_for_tests("", "1.0.0"),
    "a blank/missing version from the registry must never trigger a destructive update cycle"
  );
  assert!(!is_newer_for_tests("   ", "1.0.0"));
}

#[test]
fn rerun_prompt_message_is_none_when_versions_match() {
  let prompt = rerun_prompt_message_for_tests("install", Some("1.0.0"), "1.0.0");
  assert!(prompt.is_none());
}

#[test]
fn rerun_prompt_message_mentions_both_versions_when_version_changed() {
  let prompt = rerun_prompt_message_for_tests("install", Some("1.0.0"), "1.1.0");
  assert_eq!(
    prompt.as_deref(),
    Some(
      "Command 'install' was last run with v1.0.0 of this add-on. A new version (v1.1.0) is available. Re-run it now?"
    )
  );
}

#[test]
fn rerun_prompt_message_is_none_when_no_prior_version_recorded() {
  let prompt = rerun_prompt_message_for_tests("install", None, "1.0.0");
  assert!(
    prompt.is_none(),
    "should not prompt to re-run on a fresh install"
  );
}

#[test]
fn step_label_strips_control_characters() {
  use anesis::addons::manifest::Step;

  let step: Step = serde_json::from_value(serde_json::json!({
    "type": "create",
    "path": "safe\r\x1b[2Kmalicious redraw",
    "content": ""
  }))
  .unwrap();

  let label = step_label_for_tests(&step);
  assert!(!label.contains('\r'));
  assert!(!label.contains('\u{1b}'));
  assert!(label.contains("malicious redraw"));
}

#[test]
fn step_label_for_run_step_strips_control_characters() {
  use anesis::addons::manifest::Step;

  let step: Step = serde_json::from_value(serde_json::json!({
    "type": "run",
    "command": "echo hi\r\x1b[2Krm -rf /",
  }))
  .unwrap();

  let label = step_label_for_tests(&step);
  assert!(!label.contains('\r'));
  assert!(!label.contains('\u{1b}'));
}

fn when_addon() -> serde_json::Value {
  serde_json::json!({
    "schema_version": "1",
    "id": "when-addon",
    "name": "When Addon",
    "version": "1.0.0",
    "description": "Test fixture for step 'when'",
    "author": "anesis",
    "requires": [],
    "inputs": [
      { "name": "with_extra", "type": "boolean", "default": "false" }
    ],
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
          { "type": "create", "path": "always.txt", "content": "always\n", "if_exists": "overwrite" },
          {
            "type": "create",
            "path": "extra.txt",
            "content": "extra\n",
            "if_exists": "overwrite",
            "when": "with_extra"
          },
          {
            "type": "create",
            "path": "no-extra.txt",
            "content": "no extra\n",
            "if_exists": "overwrite",
            "when": "!with_extra"
          }
        ]
      }]
    }]
  })
}

#[tokio::test]
async fn step_with_false_when_is_skipped() {
  let fx = Fixture::new();
  fx.seed_project();
  fx.install_addon("when-addon", when_addon());

  run_addon_command(
    &fx.ctx(),
    "when-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .unwrap();

  fx.project.child("always.txt").assert("always\n");
  assert!(!fx.project.path().join("extra.txt").exists());
  fx.project.child("no-extra.txt").assert("no extra\n");
}

#[tokio::test]
async fn negated_when() {
  let fx = Fixture::new();
  fx.seed_project();
  fx.install_addon("when-addon", when_addon());

  let mut presets = HashMap::new();
  presets.insert("with_extra".to_string(), "true".to_string());

  run_addon_command(
    &fx.ctx(),
    "when-addon",
    "install",
    fx.project.path(),
    &presets,
    true,
    false,
  )
  .await
  .unwrap();

  fx.project.child("extra.txt").assert("extra\n");
  assert!(!fx.project.path().join("no-extra.txt").exists());
}

#[tokio::test]
async fn missing_input_in_when_is_an_error() {
  let fx = Fixture::new();
  fx.seed_project();

  let mut manifest = when_addon();
  manifest["variants"][0]["commands"][0]["steps"][1]["when"] =
    serde_json::json!("nonexistent_input");
  fx.install_addon("when-addon", manifest);

  let err = run_addon_command(
    &fx.ctx(),
    "when-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .unwrap_err();

  assert!(
    err.to_string().contains("nonexistent_input"),
    "error should name the unknown input: {err}"
  );
  assert!(!fx.project.path().join("always.txt").exists());
}

#[tokio::test]
async fn a_step_skipped_by_when_does_not_enter_the_journal() {
  let fx = Fixture::new();
  fx.seed_project();
  fx.install_addon("when-addon", when_addon());

  run_addon_command(
    &fx.ctx(),
    "when-addon",
    "install",
    fx.project.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await
  .unwrap();

  let lock = fx.lock();
  let entry = &lock.addons[0];
  let journal: Vec<_> = entry
    .commands
    .iter()
    .flat_map(|c| c.journal.iter())
    .collect();
  assert!(
    journal.iter().all(|rb| !matches!(
      rb,
      Rollback::DeleteCreatedFile { path } if path.ends_with("extra.txt")
    )),
    "a step skipped by 'when' must not be journaled: {journal:?}"
  );

  undo_addon("when-addon", fx.project.path(), true).unwrap();
  assert!(!fx.project.path().join("always.txt").exists());
  assert!(!fx.project.path().join("no-extra.txt").exists());
}
