use assert_cmd::Command;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use serde_json::Value;

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
      .env_remove("ANESIS_TOKEN")
      .env_remove("ANESIS_BACKEND_URL")
      .env_remove("ANESIS_FRONTEND_URL")
      .env_remove("ANESIS_DEBUG");
    cmd
  }

  fn json(&self, args: &[&str]) -> Value {
    let output = self.command().args(args).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout).into_owned();
    serde_json::from_str(&stdout).unwrap_or_else(|e| {
      panic!(
        "`anesis {}` did not emit JSON ({e}): {stdout}",
        args.join(" ")
      )
    })
  }
}

impl Cli {
  fn seed_template_cache(&self) {
    let templates = self.home.child(".anesis/cache/templates");
    templates
      .child("anesis-templates.json")
      .write_str(
        r#"{
  "lastUpdated": "2026-01-01T00:00:00Z",
  "templates": [
    {
      "name": "react-vite-ts",
      "version": "0.2.0",
      "source": "https://github.com/anesis-dev/templates",
      "path": "react-vite-ts",
      "commit_sha": "0123456789abcdef0123456789abcdef01234567"
    }
  ]
}"#,
      )
      .unwrap();
    templates
      .child("react-vite-ts/anesis.template.json")
      .write_str(
        r#"{
  "name": "react-vite-ts",
  "version": "0.2.0",
  "anesisVersion": ">=1.0.0",
  "author": { "name": "Anesis", "github": "anesis-dev" },
  "repository": { "url": "https://github.com/anesis-dev/templates" },
  "specialization": "frontend",
  "scope": "web",
  "technologies": ["react", "vite"],
  "languages": ["typescript"],
  "type": "base",
  "metadata": {
    "displayName": "React + Vite (TypeScript)",
    "description": "A React SPA scaffolded with Vite.",
    "tags": ["react", "vite", "spa"]
  },
  "inputs": [
    {
      "name": "use_router",
      "type": "boolean",
      "description": "Include React Router",
      "default": "false",
      "required": false
    }
  ]
}"#,
      )
      .unwrap();
  }

  fn seed_addon_cache(&self) {
    let addons = self.home.child(".anesis/cache/addons");
    addons
      .child("anesis-addons.json")
      .write_str(
        r#"{
  "lastUpdated": "2026-01-01T00:00:00Z",
  "addons": [
    {
      "id": "docker-compose",
      "name": "Docker Compose",
      "version": "0.4.1",
      "path": "docker-compose",
      "commit_sha": "89abcdef0123456789abcdef0123456789abcdef",
      "repo_url": "https://github.com/anesis-dev/addons"
    }
  ]
}"#,
      )
      .unwrap();
    addons
      .child("docker-compose/anesis.addon.json")
      .write_str(
        r#"{
  "schema_version": "1",
  "id": "docker-compose",
  "name": "Docker Compose",
  "version": "0.4.1",
  "description": "Adds a docker-compose.yml and a Dockerfile.",
  "author": "anesis",
  "requires": [],
  "inputs": [],
  "detect": [],
  "variants": [
    {
      "when": null,
      "commands": [
        {
          "name": "install",
          "once": true,
          "requires_commands": [],
          "inputs": [],
          "steps": [
            {
              "type": "create",
              "path": "docker-compose.yml",
              "content": "services: {}\n",
              "if_exists": "skip"
            }
          ]
        }
      ]
    }
  ]
}"#,
      )
      .unwrap();
  }
}

#[test]
fn info_json_shape() {
  let cli = Cli::new();

  insta::assert_json_snapshot!(cli.json(&["info", "--json"]), {
    ".version" => "[version]",
    ".paths.home" => "[home]",
    ".paths.templates_cache" => "[templates_cache]",
    ".paths.addons_cache" => "[addons_cache]",
    ".paths.auth" => "[auth]",
  });
}

#[test]
fn doctor_json_reports_a_list_of_checks() {
  let cli = Cli::new();
  let value = cli.json(&["doctor", "--json"]);
  let checks = value["checks"].as_array().expect("checks array");
  assert!(!checks.is_empty());
  for check in checks {
    assert!(check["name"].is_string());
    assert!(check["status"].is_string());
    assert!(check["detail"].is_string());
  }
}

#[test]
fn empty_template_list_json() {
  let cli = Cli::new();
  insta::assert_json_snapshot!(cli.json(&["template", "list", "--json"]));
}

#[test]
fn empty_addon_list_json() {
  let cli = Cli::new();
  insta::assert_json_snapshot!(cli.json(&["addon", "list", "--json"]));
}

#[test]
fn empty_stack_list_json() {
  let cli = Cli::new();
  insta::assert_json_snapshot!(cli.json(&["stack", "list", "--json"]));
}

#[test]
fn populated_template_list_json() {
  let cli = Cli::new();
  cli.seed_template_cache();
  insta::assert_json_snapshot!(cli.json(&["template", "list", "--json"]));
}

#[test]
fn populated_addon_list_json() {
  let cli = Cli::new();
  cli.seed_addon_cache();
  insta::assert_json_snapshot!(cli.json(&["addon", "list", "--json"]));
}

#[test]
fn template_info_json() {
  let cli = Cli::new();
  cli.seed_template_cache();
  insta::assert_json_snapshot!(cli.json(&["template", "info", "react-vite-ts", "--json"]));
}

#[test]
fn addon_info_json() {
  let cli = Cli::new();
  cli.seed_addon_cache();
  insta::assert_json_snapshot!(cli.json(&["addon", "info", "docker-compose", "--json"]));
}

#[test]
fn status_json_for_a_fresh_project() {
  let cli = Cli::new();
  cli
    .workdir
    .child("anesis.json")
    .write_str(
      r#"{
  "template_name": "react-vite-ts",
  "template_sha": "0123456789abcdef0123456789abcdef01234567",
  "addons": []
}"#,
    )
    .unwrap();

  insta::assert_json_snapshot!(cli.json(&["status", "--json"]));
}

#[test]
fn status_json_with_applied_addons() {
  let cli = Cli::new();
  cli
    .workdir
    .child("anesis.json")
    .write_str(
      r#"{
  "template_name": "nest-express",
  "template_sha": "89abcdef0123456789abcdef0123456789abcdef",
  "addons": ["nest-prisma-v7", "docker-compose"]
}"#,
    )
    .unwrap();
  cli
    .workdir
    .child("anesis.lock")
    .write_str(
      r#"{
  "addons": [
    {
      "id": "nest-prisma-v7",
      "version": "1.2.0",
      "variant": "nest",
      "commands_executed": ["install", "generate"]
    },
    {
      "id": "docker-compose",
      "version": "0.4.1",
      "variant": "universal",
      "commands_executed": ["install"]
    }
  ]
}"#,
    )
    .unwrap();

  insta::assert_json_snapshot!(cli.json(&["status", "--json"]));
}

#[test]
fn status_json_outside_a_project_fails() {
  let cli = Cli::new();
  let assertion = cli
    .command()
    .args(["status", "--json"])
    .assert()
    .failure()
    .code(anesis::utils::errors::exit_code::FAILURE);

  let stderr = String::from_utf8_lossy(&assertion.get_output().stderr).into_owned();
  assert!(
    stderr.contains("not an Anesis project"),
    "the error should say what is wrong: {stderr}"
  );
}

#[test]
fn account_json_when_logged_out_exits_with_the_auth_code() {
  let cli = Cli::new();
  cli
    .command()
    .args(["account", "--json"])
    .assert()
    .failure()
    .code(anesis::utils::errors::exit_code::AUTH);
}

#[test]
fn json_output_is_the_only_thing_on_stdout() {
  let cli = Cli::new();

  for args in [
    ["info", "--json"].as_slice(),
    ["template", "list", "--json"].as_slice(),
    ["addon", "list", "--json"].as_slice(),
    ["stack", "list", "--json"].as_slice(),
  ] {
    let output = cli.command().args(args).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout).into_owned();

    serde_json::from_str::<Value>(&stdout).unwrap_or_else(|e| {
      panic!(
        "`anesis {}` mixed non-JSON into stdout ({e}): {stdout:?}",
        args.join(" ")
      )
    });
  }
}
