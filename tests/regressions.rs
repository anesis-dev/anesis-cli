mod common;

use anesis::addons::lock::{CommandRun, LockEntry, LockFile};
use anesis::addons::manifest::{JsonPatchStep, PackagesStep};
use anesis::addons::runner::update_addon;
use anesis::addons::steps::{Rollback, json_patch::execute_json_patch, packages::execute_packages};
use anesis::utils::errors::{AnesisError, check_response, exit_code, exit_code_for};
use anesis::utils::template_engine::TemplateContext;
use assert_cmd::Command;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use common::fixture::{Fixture, build};
use predicates::prelude::*;
use serde_json::json;
use std::collections::HashMap;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct Cli {
  home: TempDir,
  workdir: TempDir,
}

impl Cli {
  fn new() -> Self {
    Self {
      home: TempDir::new().unwrap(),
      workdir: TempDir::new().unwrap(),
    }
  }

  fn command(&self) -> Command {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("anesis");
    cmd
      .current_dir(self.workdir.path())
      .env("HOME", self.home.path())
      .env("USERPROFILE", self.home.path())
      .env("ANESIS_HOME", self.home.path())
      .env("ANESIS_NO_TELEMETRY", "1")
      .env("ANESIS_BACKEND_URL", "http://127.0.0.1:1")
      .env("ANESIS_RELEASES_API_URL", "http://127.0.0.1:1")
      .env_remove("ANESIS_TOKEN")
      .env_remove("ANESIS_DEBUG")
      .env_remove("ANESIS_ALLOW_RUN");
    cmd
  }

  fn link_addon(&self, id: &str, inputs: &str, steps: &str) {
    let dir = TempDir::new().unwrap();
    dir
      .child("anesis.addon.json")
      .write_str(&format!(
        r#"{{
  "schema_version": "1",
  "id": "{id}",
  "name": "{id}",
  "version": "1.0.0",
  "description": "regression fixture",
  "author": "anesis",
  "requires": [],
  "inputs": {inputs},
  "detect": [],
  "variants": [{{
    "when": null,
    "commands": [{{
      "name": "go",
      "description": "",
      "once": false,
      "requires_commands": [],
      "inputs": [],
      "steps": {steps}
    }}]
  }}]
}}"#
      ))
      .unwrap();
    self
      .command()
      .args([
        "addon",
        "link",
        &dir.path().display().to_string(),
        "--force",
      ])
      .assert()
      .success();
  }

  fn link_template(&self, name: &str, files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    dir
      .child("anesis.template.json")
      .write_str(&format!(
        r#"{{
  "name": "{name}",
  "version": "1.0.0",
  "anesisVersion": ">=0.5.0",
  "author": {{ "name": "anesis", "github": "anesis-dev" }},
  "repository": {{ "url": "https://github.com/anesis-dev/templates" }},
  "metadata": {{ "displayName": "{name}", "description": "fixture", "tags": [] }},
  "inputs": []
}}"#
      ))
      .unwrap();
    for (rel, content) in files {
      dir.child(rel).write_str(content).unwrap();
    }
    self.relink_template(&dir);
    dir
  }

  fn relink_template(&self, dir: &TempDir) {
    self
      .command()
      .args([
        "template",
        "link",
        &dir.path().display().to_string(),
        "--force",
      ])
      .assert()
      .success();
  }
}

const TOUCH_STEP: &str =
  r#"[{ "type": "create", "path": "generated.txt", "content": "hi\n", "if_exists": "overwrite" }]"#;

#[test]
fn use_diff_does_not_print_gitignored_secret_files() {
  let cli = Cli::new();
  cli.link_addon("diff-addon", "[]", TOUCH_STEP);
  cli
    .workdir
    .child(".gitignore")
    .write_str(".env\nnode_modules/\n")
    .unwrap();
  cli
    .workdir
    .child(".env")
    .write_str("SECRET_KEY=hunter2\n")
    .unwrap();
  cli
    .workdir
    .child("node_modules/dep/index.js")
    .write_str("module.exports = 1;\n")
    .unwrap();
  cli
    .workdir
    .child("anesis.json")
    .write_str(r#"{"template_name":"t","template_sha":"s","addons":[]}"#)
    .unwrap();

  cli
    .command()
    .args(["use", "diff-addon", "go", "--diff"])
    .assert()
    .success()
    .stdout(predicate::str::contains("generated.txt"))
    .stdout(predicate::str::contains("hunter2").not())
    .stdout(predicate::str::contains("node_modules").not());
}

#[test]
fn unknown_input_names_are_rejected_with_a_suggestion() {
  let cli = Cli::new();
  cli.link_addon(
    "input-addon",
    r#"[{ "name": "region", "type": "text", "description": "", "default": "eu" }]"#,
    TOUCH_STEP,
  );

  cli
    .command()
    .args(["use", "input-addon", "go", "-y", "--input", "regoin=us"])
    .assert()
    .failure()
    .stderr(predicate::str::contains("did you mean 'region'"));
  assert!(!cli.workdir.path().join("generated.txt").exists());
}

#[test]
fn select_and_boolean_inputs_are_validated() {
  let cli = Cli::new();
  cli.link_addon(
    "typed-addon",
    r#"[
      { "name": "region", "type": "select", "description": "", "default": "eu", "options": ["eu", "us"] },
      { "name": "fancy", "type": "boolean", "description": "", "default": "false" }
    ]"#,
    r#"[{ "type": "create", "path": "out.txt", "content": "{{ region }} {{ fancy }}\n", "if_exists": "overwrite" }]"#,
  );

  cli
    .command()
    .args(["use", "typed-addon", "go", "-y", "--input", "region=mars"])
    .assert()
    .failure()
    .stderr(predicate::str::contains("allowed values: eu, us"));

  cli
    .command()
    .args(["use", "typed-addon", "go", "-y", "--input", "fancy=maybe"])
    .assert()
    .failure()
    .stderr(predicate::str::contains("boolean input 'fancy'"));

  cli
    .command()
    .args([
      "use",
      "typed-addon",
      "go",
      "-y",
      "--input",
      "fancy=yes",
      "--input",
      "region=us",
    ])
    .assert()
    .success();
  assert_eq!(
    std::fs::read_to_string(cli.workdir.path().join("out.txt")).unwrap(),
    "us true\n"
  );
}

#[test]
fn project_commands_resolve_the_root_from_a_subdirectory() {
  let cli = Cli::new();
  cli.link_addon("root-addon", "[]", TOUCH_STEP);
  cli
    .workdir
    .child("anesis.json")
    .write_str(r#"{"template_name":"t","template_sha":"s","addons":[]}"#)
    .unwrap();
  cli.workdir.child("sub/deeper").create_dir_all().unwrap();

  cli
    .command()
    .current_dir(cli.workdir.path().join("sub/deeper"))
    .args(["use", "root-addon", "go", "-y"])
    .assert()
    .success();

  assert!(cli.workdir.path().join("generated.txt").exists());
  assert!(cli.workdir.path().join("anesis.lock").exists());
  assert!(!cli.workdir.path().join("sub/deeper/anesis.lock").exists());
}

#[cfg(unix)]
#[test]
fn generated_projects_keep_executable_permissions() {
  use std::os::unix::fs::PermissionsExt;

  let cli = Cli::new();
  let dir = cli.link_template(
    "exec-template",
    &[("scripts/build.sh", "#!/bin/sh\necho hi\n")],
  );
  std::fs::set_permissions(
    dir.path().join("scripts/build.sh"),
    std::fs::Permissions::from_mode(0o755),
  )
  .unwrap();
  cli.relink_template(&dir);

  cli
    .command()
    .args(["new", "out", "exec-template", "-y"])
    .assert()
    .success();

  let mode = std::fs::metadata(cli.workdir.path().join("out/scripts/build.sh"))
    .unwrap()
    .permissions()
    .mode();
  assert_eq!(mode & 0o111, 0o111, "executable bits were lost: {mode:o}");
}

#[test]
fn a_render_error_removes_the_half_generated_project_so_the_command_can_be_retried() {
  let cli = Cli::new();
  cli.link_template(
    "broken-template",
    &[("a.txt", "a\n"), ("zz.txt.tera", "{{ broken(")],
  );

  for _ in 0..2 {
    cli
      .command()
      .args(["new", "out", "broken-template", "-y"])
      .assert()
      .failure()
      .stderr(predicate::str::contains("already exists").not());
    assert!(
      !cli.workdir.path().join("out").exists(),
      "the partial project must be removed after the failure"
    );
  }
}

#[test]
fn a_malformed_template_manifest_is_reported() {
  let cli = Cli::new();
  cli.link_template("tpl-bad-manifest", &[("a.txt", "a\n")]);
  let cached = cli
    .home
    .path()
    .join(".anesis/cache/templates/tpl-bad-manifest/anesis.template.json");
  let good = std::fs::read_to_string(&cached).unwrap();
  std::fs::write(&cached, good.replace("\"inputs\": []", "\"inputs\": [,]")).unwrap();

  cli
    .command()
    .args(["new", "out", "tpl-bad-manifest", "-y"])
    .assert()
    .failure()
    .stderr(predicate::str::contains("anesis.template.json"));
}

#[test]
fn stack_commands_reject_ids_that_escape_the_cache() {
  let cli = Cli::new();
  cli.link_addon("keep-addon", "[]", TOUCH_STEP);
  let index = cli
    .home
    .path()
    .join(".anesis/cache/addons/anesis-addons.json");
  assert!(index.exists());

  cli
    .command()
    .args(["stack", "remove", "../addons/anesis-addons"])
    .assert()
    .failure();
  assert!(
    index.exists(),
    "the addon index must not be deletable through a stack id"
  );
}

#[cfg(unix)]
#[test]
fn input_values_cannot_inject_shell_commands_into_run_steps() {
  let cli = Cli::new();
  cli.link_addon(
    "shell-addon",
    r#"[{ "name": "name", "type": "text", "description": "", "default": "x" }]"#,
    r#"[{ "type": "run", "command": "echo {{ name }} > out.txt", "description": "" }]"#,
  );

  cli
    .command()
    .args([
      "use",
      "shell-addon",
      "go",
      "-y",
      "--allow-run",
      "--input",
      "name=hello; touch pwned",
    ])
    .assert()
    .success();

  assert!(!cli.workdir.path().join("pwned").exists());
  assert_eq!(
    std::fs::read_to_string(cli.workdir.path().join("out.txt")).unwrap(),
    "hello; touch pwned\n"
  );
}

#[test]
fn json_patch_keeps_key_order_and_indentation() {
  let dir = TempDir::new().unwrap();
  dir
    .child("package.json")
    .write_str("{\n    \"name\": \"app\",\n    \"version\": \"1.0.0\",\n    \"dependencies\": {\n        \"b\": \"1\",\n        \"a\": \"2\"\n    }\n}\n")
    .unwrap();

  let step = JsonPatchStep {
    path: "package.json".into(),
    set: HashMap::from([("scripts.lint".to_string(), json!("eslint ."))]),
    remove: vec![],
  };
  execute_json_patch(&step, dir.path(), &TemplateContext::new()).unwrap();

  assert_eq!(
    std::fs::read_to_string(dir.path().join("package.json")).unwrap(),
    "{\n    \"name\": \"app\",\n    \"version\": \"1.0.0\",\n    \"dependencies\": {\n        \"b\": \"1\",\n        \"a\": \"2\"\n    },\n    \"scripts\": {\n        \"lint\": \"eslint .\"\n    }\n}\n"
  );
}

#[test]
fn json_patch_supports_pointer_paths_and_refuses_to_overwrite_non_objects() {
  let dir = TempDir::new().unwrap();
  dir
    .child("package.json")
    .write_str(r#"{"files":["src"],"exports":{}}"#)
    .unwrap();

  let pointer = JsonPatchStep {
    path: "package.json".into(),
    set: HashMap::from([(
      "/exports/~1package.json".to_string(),
      json!("./package.json"),
    )]),
    remove: vec![],
  };
  execute_json_patch(&pointer, dir.path(), &TemplateContext::new()).unwrap();
  let value: serde_json::Value =
    serde_json::from_str(&std::fs::read_to_string(dir.path().join("package.json")).unwrap())
      .unwrap();
  assert_eq!(value["exports"]["/package.json"], "./package.json");

  let clobber = JsonPatchStep {
    path: "package.json".into(),
    set: HashMap::from([("files.extra.deep".to_string(), json!(1))]),
    remove: vec![],
  };
  let err = execute_json_patch(&clobber, dir.path(), &TemplateContext::new()).unwrap_err();
  assert!(format!("{:#}", err.error).contains("array index"));
  let after: serde_json::Value =
    serde_json::from_str(&std::fs::read_to_string(dir.path().join("package.json")).unwrap())
      .unwrap();
  assert_eq!(after["files"], json!(["src"]));
}

#[test]
fn packages_step_rejects_option_like_specs() {
  let dir = TempDir::new().unwrap();
  dir.child("package.json").write_str("{}").unwrap();
  let step = PackagesStep {
    dependencies: vec!["--registry=https://evil.example".into(), "lodash".into()],
    dev_dependencies: vec![],
  };
  let err = execute_packages(&step, dir.path(), true, true).unwrap_err();
  assert!(format!("{:#}", err.error).contains("must not start with '-'"));
}

#[test]
fn lock_journal_is_base64_encoded_and_legacy_arrays_still_load() {
  let dir = TempDir::new().unwrap();
  let root = dir.path().canonicalize().unwrap();
  let mut lock = LockFile::default();
  let mut entry = LockEntry::new("a", "1.0.0", "universal");
  entry.upsert_command(
    "go",
    HashMap::new(),
    vec![Rollback::restore_file_for_tests(
      root.join("f.txt"),
      b"hello".to_vec(),
    )],
  );
  lock.addons.push(entry);
  lock.save(&root).unwrap();

  let raw = std::fs::read_to_string(root.join("anesis.lock")).unwrap();
  assert!(raw.contains("aGVsbG8="), "expected base64 payload: {raw}");
  assert!(
    !raw.contains("104"),
    "bytes must not be serialised as integers: {raw}"
  );

  let legacy = raw.replace("\"aGVsbG8=\"", "[104, 101, 108, 108, 111]");
  std::fs::write(root.join("anesis.lock"), legacy).unwrap();
  let loaded = LockFile::load(&root).unwrap();
  match &loaded.addons[0].commands[0].journal[0] {
    Rollback::RestoreFile { original, .. } => assert_eq!(original, b"hello"),
    other => panic!("unexpected rollback {other:?}"),
  }
}

#[test]
fn lock_entries_pointing_into_vcs_metadata_are_dropped() {
  let dir = TempDir::new().unwrap();
  let root = dir.path().canonicalize().unwrap();
  std::fs::create_dir_all(root.join(".git/hooks")).unwrap();
  std::fs::write(
    root.join("anesis.lock"),
    r#"{"schema_version":2,"addons":[{"id":"evil","version":"1","variant":"u","commands":[{"name":"go","journal":[
      {"RestoreFile":{"path":".git/hooks/pre-commit","original":"IyEvYmluL3No","mode":493}}]}]}]}"#,
  )
  .unwrap();

  let lock = LockFile::load(&root).unwrap();
  let CommandRun { journal, .. } = &lock.addons[0].commands[0];
  assert!(
    journal.is_empty(),
    "journal entries touching .git must be dropped"
  );
}

#[test]
fn find_project_root_walks_up_to_the_lock_or_manifest() {
  let dir = TempDir::new().unwrap();
  dir.child("anesis.lock").write_str("{}").unwrap();
  dir.child("a/b/c").create_dir_all().unwrap();
  let found = anesis::utils::fs::find_project_root(&dir.path().join("a/b/c")).unwrap();
  assert_eq!(found, dir.path());
}

#[cfg(unix)]
#[test]
fn write_atomic_follows_symlinks_and_keeps_permissions() {
  use std::os::unix::fs::PermissionsExt;

  let dir = TempDir::new().unwrap();
  let real = dir.path().join("dotfiles/zshrc");
  std::fs::create_dir_all(real.parent().unwrap()).unwrap();
  std::fs::write(&real, "old").unwrap();
  std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o644)).unwrap();
  let link = dir.path().join(".zshrc");
  std::os::unix::fs::symlink(&real, &link).unwrap();

  anesis::utils::atomic::write_atomic(&link, b"new").unwrap();

  assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
  assert_eq!(std::fs::read_to_string(&real).unwrap(), "new");
  assert_eq!(
    std::fs::metadata(&real).unwrap().permissions().mode() & 0o777,
    0o644
  );
}

#[cfg(unix)]
#[test]
fn private_atomic_writes_are_never_group_or_world_readable() {
  use std::os::unix::fs::PermissionsExt;

  let dir = TempDir::new().unwrap();
  let target = dir.path().join("auth.json");
  std::fs::write(&target, "old").unwrap();
  std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();

  anesis::utils::atomic::write_private_atomic(&target, b"token").unwrap();

  assert_eq!(
    std::fs::metadata(&target).unwrap().permissions().mode() & 0o777,
    0o600
  );
}

#[test]
fn github_urls_must_name_exactly_an_owner_and_a_repo() {
  use anesis::utils::validate::is_valid_github_repo_url;
  assert!(is_valid_github_repo_url("https://github.com/owner/repo").is_ok());
  assert!(is_valid_github_repo_url("https://github.com/owner/repo.git").is_ok());
  for bad in [
    "https://github.com/owner/",
    "https://github.com/owner",
    "https://github.com/owner/repo/tree/main/sub",
    "https://user:token@github.com/owner/repo",
    "https://github.com/owner/repo?x=1",
    "https://github.com/owner/repo#frag",
  ] {
    assert!(
      is_valid_github_repo_url(bad).is_err(),
      "{bad} must be rejected"
    );
  }
}

#[test]
fn windows_reserved_names_are_complete() {
  use anesis::utils::validate::{is_windows_reserved_name, validate_project_name};
  for name in ["COM0", "lpt0", "CONIN$", "con.txt", "COM\u{b9}", "nul"] {
    assert!(is_windows_reserved_name(name), "{name}");
  }
  assert!(!is_windows_reserved_name("console"));
  assert!(validate_project_name("aux").is_err());
}

#[tokio::test]
async fn client_errors_do_not_map_to_the_network_exit_code() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/thing"))
    .respond_with(ResponseTemplate::new(422).set_body_json(json!({ "message": "bad payload" })))
    .mount(&server)
    .await;

  let response = reqwest::get(format!("{}/thing", server.uri()))
    .await
    .unwrap();
  let err = check_response(response, "thing").await.unwrap_err();
  assert_eq!(exit_code_for(&err), exit_code::FAILURE);
  assert_eq!(
    exit_code_for(&AnesisError::HttpServerError("x".into()).into()),
    exit_code::NETWORK
  );
  assert_eq!(
    exit_code_for(&AnesisError::Interrupted.into()),
    exit_code::INTERRUPTED
  );
}

#[test]
fn prompt_cancellation_maps_to_the_conventional_exit_codes() {
  let interrupted = anyhow::Error::from(inquire::InquireError::OperationInterrupted);
  let cancelled = anyhow::Error::from(inquire::InquireError::OperationCanceled);
  assert_eq!(exit_code_for(&interrupted), exit_code::INTERRUPTED);
  assert_eq!(exit_code_for(&cancelled), exit_code::ABORTED);
}

#[test]
fn anesis_debug_zero_does_not_enable_debug_output() {
  let cli = Cli::new();
  cli
    .command()
    .env("ANESIS_DEBUG", "0")
    .args(["use", "no-such-addon", "go"])
    .assert()
    .failure()
    .stderr(predicate::str::contains("Backtrace").not());
}

async fn mount_update_url(server: &MockServer, id: &str) {
  Mock::given(method("GET"))
    .and(path(format!("/addon/{id}/url")))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "archive_url": format!("{}/archive.tar.gz", server.uri()),
      "commit_sha": "deadbeef",
      "subdir": null,
      "version": "2.0.0"
    })))
    .mount(server)
    .await;
}

#[tokio::test]
async fn update_reuses_the_addon_level_inputs_chosen_originally() {
  let server = MockServer::start().await;
  mount_update_url(&server, "region-addon").await;

  let fx = Fixture::new();
  fx.seed_project();

  let mut v2 = build::addon_manifest("region-addon", "2.0.0");
  v2["inputs"] = json!([{ "name": "region", "type": "text", "description": "", "default": "eu" }]);
  v2["variants"][0]["commands"][0]["steps"] = json!([
    { "type": "create", "path": "region.txt", "content": "{{ region }}\n", "if_exists": "overwrite" }
  ]);
  fx.install_addon("region-addon", &v2);

  let mut lock = LockFile::load(fx.project.path()).unwrap();
  let mut entry = LockEntry::new("region-addon", "1.0.0", "universal");
  entry.inputs.insert("region".into(), "us".into());
  entry.upsert_command("install", HashMap::new(), vec![]);
  lock.addons.push(entry);
  lock.save(fx.project.path()).unwrap();

  let ctx = fx.mock_ctx(&server);
  update_addon(&ctx, "region-addon", fx.project.path(), true)
    .await
    .unwrap();

  assert_eq!(
    std::fs::read_to_string(fx.project.path().join("region.txt")).unwrap(),
    "us\n"
  );
}

#[tokio::test]
async fn a_failed_update_restores_the_previous_version() {
  let server = MockServer::start().await;
  mount_update_url(&server, "restore-addon").await;

  let fx = Fixture::new();
  fx.seed_project();
  let root = fx.project.path().canonicalize().unwrap();
  let old_file = root.join("old.txt");
  std::fs::write(&old_file, "from v1\n").unwrap();

  let mut v2 = build::addon_manifest("restore-addon", "2.0.0");
  v2["variants"][0]["commands"][0]["steps"] = json!([
    { "type": "create", "path": "new.txt", "content": "from v2\n", "if_exists": "overwrite" },
    {
      "type": "inject",
      "target": { "type": "file", "file": "new.txt" },
      "content": "x",
      "after": "marker-that-does-not-exist",
      "if_not_found": "error"
    }
  ]);
  fx.install_addon("restore-addon", &v2);

  let mut lock = LockFile::load(&root).unwrap();
  let mut entry = LockEntry::new("restore-addon", "1.0.0", "universal");
  entry.upsert_command(
    "install",
    HashMap::new(),
    vec![Rollback::DeleteCreatedFile {
      path: old_file.clone(),
    }],
  );
  lock.addons.push(entry);
  lock.save(&root).unwrap();

  let ctx = fx.mock_ctx(&server);
  let err = update_addon(&ctx, "restore-addon", &root, true)
    .await
    .unwrap_err();
  assert!(
    format!("{err:#}").contains("previous v1.0.0 was restored"),
    "{err:#}"
  );

  assert_eq!(std::fs::read_to_string(&old_file).unwrap(), "from v1\n");
  assert!(!root.join("new.txt").exists());
  let lock = LockFile::load(&root).unwrap();
  let entry = lock
    .addons
    .iter()
    .find(|e| e.id == "restore-addon")
    .unwrap();
  assert_eq!(entry.version, "1.0.0");
  assert!(entry.has_undoable_changes());
}
