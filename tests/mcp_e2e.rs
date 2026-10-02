use assert_fs::TempDir;
use assert_fs::prelude::*;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

struct McpSession {
  child: std::process::Child,
  stdin: std::process::ChildStdin,
  stdout: BufReader<std::process::ChildStdout>,
}

impl McpSession {
  fn start(home: &TempDir, workdir: &TempDir) -> Self {
    let mut child = Command::new(env!("CARGO_BIN_EXE_anesis"))
      .arg("mcp")
      .current_dir(workdir.path())
      .env("HOME", home.path())
      .env("USERPROFILE", home.path())
      .env("ANESIS_HOME", home.path())
      .env("ANESIS_NO_TELEMETRY", "1")
      .env("ANESIS_BACKEND_URL", "http://127.0.0.1:1")
      .env("ANESIS_RELEASES_API_URL", "http://127.0.0.1:1")
      .env_remove("ANESIS_TOKEN")
      .env_remove("ANESIS_DEBUG")
      .stdin(Stdio::piped())
      .stdout(Stdio::piped())
      .stderr(Stdio::piped())
      .spawn()
      .unwrap();
    let stdin = child.stdin.take().unwrap();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    Self {
      child,
      stdin,
      stdout,
    }
  }

  fn call(&mut self, request: serde_json::Value) -> serde_json::Value {
    let line = serde_json::to_string(&request).unwrap();
    self.read_reply_to(line.as_bytes())
  }

  fn write_raw_line(&mut self, bytes: &[u8]) {
    self.stdin.write_all(bytes).unwrap();
    self.stdin.write_all(b"\n").unwrap();
    self.stdin.flush().unwrap();
  }

  fn read_reply_to(&mut self, line: &[u8]) -> serde_json::Value {
    self.write_raw_line(line);

    let mut reply_line = String::new();
    self.stdout.read_line(&mut reply_line).unwrap();
    assert!(
      !reply_line.is_empty(),
      "anesis mcp closed stdout without replying to {}",
      String::from_utf8_lossy(line)
    );
    serde_json::from_str(&reply_line)
      .unwrap_or_else(|e| panic!("reply was not valid JSON ({e}): {reply_line}"))
  }

  fn shutdown(mut self) {
    drop(self.stdin);
    let status = self.child.wait().unwrap();
    assert!(status.success(), "anesis mcp exited with {status}");
  }
}

fn write_template(dir: &TempDir, name: &str) {
  dir
    .child("anesis.template.json")
    .write_str(&format!(
      r#"{{
  "name": "{name}",
  "version": "1.0.0",
  "anesisVersion": ">=0.5.0",
  "author": {{ "name": "anesis", "github": "anesis-dev" }},
  "repository": {{ "url": "https://github.com/anesis-dev/templates" }},
  "specialization": "backend",
  "scope": "cli",
  "technologies": [],
  "languages": [],
  "type": "base",
  "metadata": {{ "displayName": "{name}", "description": "MCP e2e fixture", "tags": [] }},
  "inputs": []
}}"#
    ))
    .unwrap();
  dir.child("README.md").write_str("# hello\n").unwrap();
}

fn write_addon(dir: &TempDir, id: &str, steps: &str) {
  dir
    .child("anesis.addon.json")
    .write_str(&format!(
      r#"{{
  "schema_version": "1",
  "id": "{id}",
  "name": "{id}",
  "version": "1.0.0",
  "description": "MCP e2e fixture addon",
  "author": "anesis",
  "requires": [],
  "inputs": [],
  "detect": [],
  "variants": [{{
    "when": null,
    "commands": [{{
      "name": "install",
      "description": "",
      "once": true,
      "requires_commands": [],
      "inputs": [],
      "steps": {steps}
    }}]
  }}]
}}"#
    ))
    .unwrap();
}

#[test]
fn initialize_then_tools_list_over_real_stdio() {
  let home = TempDir::new().unwrap();
  let workdir = TempDir::new().unwrap();
  let mut session = McpSession::start(&home, &workdir);

  let init = session.call(serde_json::json!({
    "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
  }));
  assert_eq!(init["result"]["serverInfo"]["name"], "anesis");

  let list = session.call(serde_json::json!({
    "jsonrpc": "2.0", "id": 2, "method": "tools/list"
  }));
  let tools = list["result"]["tools"].as_array().unwrap();
  assert!(tools.iter().any(|t| t["name"] == "scaffold_project"));

  session.shutdown();
}

#[test]
fn scaffold_project_over_real_stdio_actually_scaffolds_via_run_self() {
  let home = TempDir::new().unwrap();
  let workdir = TempDir::new().unwrap();

  let template_dir = TempDir::new().unwrap();
  write_template(&template_dir, "mcp-e2e-template");
  let status = Command::new(env!("CARGO_BIN_EXE_anesis"))
    .args([
      "template",
      "link",
      &template_dir.path().display().to_string(),
      "--force",
    ])
    .env("HOME", home.path())
    .env("USERPROFILE", home.path())
    .env("ANESIS_HOME", home.path())
    .env("ANESIS_NO_TELEMETRY", "1")
    .env_remove("ANESIS_TOKEN")
    .status()
    .unwrap();
  assert!(status.success());

  let mut session = McpSession::start(&home, &workdir);
  session.call(serde_json::json!({
    "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
  }));

  let result = session.call(serde_json::json!({
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "scaffold_project",
      "arguments": {
        "name": "mcp-proj",
        "template": "mcp-e2e-template",
        "path": workdir.path().display().to_string()
      }
    }
  }));

  assert_eq!(
    result["result"]["isError"], false,
    "scaffold_project call failed: {result}"
  );
  assert!(
    workdir.path().join("mcp-proj/README.md").exists(),
    "run_self must have actually invoked `anesis new` as a real subprocess"
  );

  session.shutdown();
}

#[test]
fn scaffold_project_into_a_nonempty_directory_refuses_to_clobber_by_default() {
  let home = TempDir::new().unwrap();
  let workdir = TempDir::new().unwrap();

  workdir
    .child("README.md")
    .write_str("# the user's own project\n")
    .unwrap();

  let template_dir = TempDir::new().unwrap();
  write_template(&template_dir, "mcp-e2e-clobber-template");
  let status = Command::new(env!("CARGO_BIN_EXE_anesis"))
    .args([
      "template",
      "link",
      &template_dir.path().display().to_string(),
      "--force",
    ])
    .env("HOME", home.path())
    .env("USERPROFILE", home.path())
    .env("ANESIS_HOME", home.path())
    .env("ANESIS_NO_TELEMETRY", "1")
    .env_remove("ANESIS_TOKEN")
    .status()
    .unwrap();
  assert!(status.success());

  let mut session = McpSession::start(&home, &workdir);
  session.call(serde_json::json!({
    "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
  }));

  let result = session.call(serde_json::json!({
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "scaffold_project",
      "arguments": {
        "name": ".",
        "template": "mcp-e2e-clobber-template",
        "path": workdir.path().display().to_string()
      }
    }
  }));

  assert_eq!(
    result["result"]["isError"], true,
    "scaffolding into a directory with an unrelated README.md must fail without --overwrite: {result}"
  );
  assert_eq!(
    std::fs::read_to_string(workdir.path().join("README.md")).unwrap(),
    "# the user's own project\n",
    "the user's file must be left untouched"
  );

  session.shutdown();
}

fn link_template_and_addon(home: &TempDir, workdir: &TempDir) {
  let template_dir = TempDir::new().unwrap();
  write_template(&template_dir, "mcp-e2e-lifecycle-template");
  let status = Command::new(env!("CARGO_BIN_EXE_anesis"))
    .args([
      "template",
      "link",
      &template_dir.path().display().to_string(),
      "--force",
    ])
    .env("HOME", home.path())
    .env("USERPROFILE", home.path())
    .env("ANESIS_HOME", home.path())
    .env("ANESIS_NO_TELEMETRY", "1")
    .env_remove("ANESIS_TOKEN")
    .status()
    .unwrap();
  assert!(status.success());

  let addon_dir = TempDir::new().unwrap();
  write_addon(
    &addon_dir,
    "mcp-e2e-addon",
    r#"[
      { "type": "create", "path": "addon-file.txt", "content": "from addon\n", "if_exists": "overwrite" }
    ]"#,
  );
  let status = Command::new(env!("CARGO_BIN_EXE_anesis"))
    .args([
      "addon",
      "link",
      &addon_dir.path().display().to_string(),
      "--force",
    ])
    .env("HOME", home.path())
    .env("USERPROFILE", home.path())
    .env("ANESIS_HOME", home.path())
    .env("ANESIS_NO_TELEMETRY", "1")
    .env_remove("ANESIS_TOKEN")
    .status()
    .unwrap();
  assert!(status.success());

  let status = Command::new(env!("CARGO_BIN_EXE_anesis"))
    .args(["new", "proj", "mcp-e2e-lifecycle-template", "--yes"])
    .current_dir(workdir.path())
    .env("HOME", home.path())
    .env("USERPROFILE", home.path())
    .env("ANESIS_HOME", home.path())
    .env("ANESIS_NO_TELEMETRY", "1")
    .env_remove("ANESIS_TOKEN")
    .status()
    .unwrap();
  assert!(status.success());
}

#[test]
fn dry_run_reports_planned_steps_without_touching_disk() {
  let home = TempDir::new().unwrap();
  let workdir = TempDir::new().unwrap();
  link_template_and_addon(&home, &workdir);
  let project_root = workdir.path().join("proj");

  let mut session = McpSession::start(&home, &workdir);
  session.call(serde_json::json!({
    "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
  }));

  let result = session.call(serde_json::json!({
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "dry_run",
      "arguments": {
        "addon_id": "mcp-e2e-addon",
        "command": "install",
        "path": project_root.display().to_string()
      }
    }
  }));

  assert_eq!(
    result["result"]["isError"], false,
    "dry_run call failed: {result}"
  );
  assert!(
    !project_root.join("addon-file.txt").exists(),
    "dry_run must not write any files"
  );

  session.shutdown();
}

#[test]
fn undo_addon_reverts_over_real_stdio() {
  let home = TempDir::new().unwrap();
  let workdir = TempDir::new().unwrap();
  link_template_and_addon(&home, &workdir);
  let project_root = workdir.path().join("proj");

  let status = Command::new(env!("CARGO_BIN_EXE_anesis"))
    .args(["use", "mcp-e2e-addon", "install", "--yes"])
    .current_dir(&project_root)
    .env("HOME", home.path())
    .env("USERPROFILE", home.path())
    .env("ANESIS_HOME", home.path())
    .env("ANESIS_NO_TELEMETRY", "1")
    .env_remove("ANESIS_TOKEN")
    .status()
    .unwrap();
  assert!(status.success());
  assert!(project_root.join("addon-file.txt").exists());

  let mut session = McpSession::start(&home, &workdir);
  session.call(serde_json::json!({
    "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
  }));

  let result = session.call(serde_json::json!({
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "undo_addon",
      "arguments": {
        "addon_id": "mcp-e2e-addon",
        "path": project_root.display().to_string()
      }
    }
  }));

  assert_eq!(
    result["result"]["isError"], false,
    "undo_addon call failed: {result}"
  );
  assert!(
    !project_root.join("addon-file.txt").exists(),
    "undo_addon must have reverted the addon's changes"
  );

  session.shutdown();
}

#[test]
fn invalid_json_on_stdin_gets_a_parse_error_reply_and_the_server_keeps_running() {
  let home = TempDir::new().unwrap();
  let workdir = TempDir::new().unwrap();
  let mut session = McpSession::start(&home, &workdir);

  let reply = session.read_reply_to(b"{not valid json");
  assert_eq!(reply["error"]["code"], -32700);
  assert_eq!(reply["id"], serde_json::Value::Null);

  let init = session.call(serde_json::json!({
    "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
  }));
  assert_eq!(
    init["result"]["serverInfo"]["name"], "anesis",
    "the server must still answer requests after a parse error: {init}"
  );

  session.shutdown();
}

#[test]
fn a_single_invalid_utf8_byte_on_stdin_does_not_kill_the_server() {
  let home = TempDir::new().unwrap();
  let workdir = TempDir::new().unwrap();
  let mut session = McpSession::start(&home, &workdir);

  session.write_raw_line(&[0xff, 0xfe, 0x00, 0x01]);

  let init = session.call(serde_json::json!({
    "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
  }));
  assert_eq!(
    init["result"]["serverInfo"]["name"], "anesis",
    "a non-UTF-8 line must be skipped, not kill the server or drop in-flight requests: {init}"
  );

  session.shutdown();
}
