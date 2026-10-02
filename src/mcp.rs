use std::io::{BufRead, Write};
use std::process::{Command, Stdio};

use anyhow::Result;
use serde_json::{Value, json};

pub fn run_mcp() -> Result<()> {
  let stdin = std::io::stdin();
  let stdout = std::io::stdout();
  let mut out = stdout.lock();

  for line in stdin.lock().lines() {
    let line = match line {
      Ok(line) => line,
      Err(e) => {
        eprintln!("anesis mcp: skipping unreadable line on stdin: {e}");
        continue;
      }
    };
    if line.trim().is_empty() {
      continue;
    }
    let req: Value = match serde_json::from_str(&line) {
      Ok(req) => req,
      Err(e) => {
        let reply = json!({
          "jsonrpc": "2.0",
          "id": null,
          "error": { "code": -32700, "message": format!("Parse error: {e}") }
        });
        writeln!(out, "{reply}")?;
        out.flush()?;
        continue;
      }
    };

    let Some(id) = req.get("id").cloned() else {
      continue;
    };
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");

    let reply = match dispatch(method, req.get("params")) {
      Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
      Err((code, message)) => {
        json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
      }
    };

    writeln!(out, "{reply}")?;
    out.flush()?;
  }

  Ok(())
}

fn dispatch(method: &str, params: Option<&Value>) -> Result<Value, (i64, String)> {
  match method {
    "initialize" => Ok(json!({
      "protocolVersion": "2024-11-05",
      "capabilities": { "tools": {} },
      "serverInfo": { "name": "anesis", "version": env!("CARGO_PKG_VERSION") }
    })),
    "ping" => Ok(json!({})),
    "tools/list" => Ok(json!({ "tools": tools_list() })),
    "tools/call" => call_tool(params),
    other => Err((-32601, format!("Method not found: {other}"))),
  }
}

#[doc(hidden)]
pub fn dispatch_for_tests(method: &str, params: Option<&Value>) -> Result<Value, (i64, String)> {
  dispatch(method, params)
}

fn call_tool(params: Option<&Value>) -> Result<Value, (i64, String)> {
  let params = params.ok_or((-32602, "Missing params".to_string()))?;
  let name = params
    .get("name")
    .and_then(Value::as_str)
    .ok_or((-32602, "Missing tool name".to_string()))?;
  let args = params
    .get("arguments")
    .cloned()
    .unwrap_or_else(|| json!({}));

  let (text, is_error) = run_tool(name, &args);
  Ok(json!({
    "content": [{ "type": "text", "text": text }],
    "isError": is_error
  }))
}

fn build_argv(name: &str, args: &Value) -> Result<Vec<String>, String> {
  let s = |k: &str| {
    args
      .get(k)
      .and_then(Value::as_str)
      .unwrap_or("")
      .to_string()
  };

  match name {
    "search_registry" => {
      let mut v = vec!["search".to_string()];
      let q = s("query");
      if !q.is_empty() {
        v.push(q);
      }
      v.push("--json".to_string());
      Ok(v)
    }
    "get_manifest" => {
      let kind = s("kind");
      let id = s("id");
      if id.is_empty() {
        return Err("'id' is required".to_string());
      }
      match kind.as_str() {
        "template" => Ok(vec!["template".into(), "info".into(), id, "--json".into()]),
        "addon" => Ok(vec!["addon".into(), "info".into(), id, "--json".into()]),
        "stack" => Ok(vec!["stack".into(), "info".into(), id, "--json".into()]),
        other => Err(format!("Unknown kind '{other}'; use template|addon|stack")),
      }
    }
    "scaffold_project" => {
      let project = s("name");
      if project.is_empty() {
        return Err("'name' is required".to_string());
      }
      let mut v = vec!["new".to_string(), project];
      let stack = s("stack");
      let template = s("template");
      if !stack.is_empty() {
        v.push("--stack".into());
        v.push(stack);
      } else if !template.is_empty() {
        v.push(template);
      } else {
        return Err("Provide either 'template' or 'stack'".to_string());
      }
      v.push("--yes".into());
      push_overwrite(&mut v, args);
      push_allow_run(&mut v, args);
      push_inputs(&mut v, args);
      Ok(v)
    }
    "apply_addon" => {
      let id = s("addon_id");
      let command = s("command");
      if id.is_empty() || command.is_empty() {
        return Err("'addon_id' and 'command' are required".to_string());
      }
      let mut v = vec!["use".to_string(), id, command, "--yes".into()];
      push_allow_run(&mut v, args);
      push_inputs(&mut v, args);
      Ok(v)
    }
    "apply_stack" => {
      let project = s("name");
      let stack = s("stack");
      if project.is_empty() || stack.is_empty() {
        return Err("'name' and 'stack' are required".to_string());
      }
      let mut v = vec![
        "new".to_string(),
        project,
        "--stack".into(),
        stack,
        "--yes".into(),
      ];
      push_overwrite(&mut v, args);
      push_allow_run(&mut v, args);
      push_inputs(&mut v, args);
      Ok(v)
    }
    "project_status" => Ok(vec!["status".into(), "--json".into()]),
    "dry_run" => {
      let id = s("addon_id");
      let command = s("command");
      if id.is_empty() || command.is_empty() {
        return Err("'addon_id' and 'command' are required".to_string());
      }
      let mut v = vec!["use".to_string(), id, command, "--dry-run".into()];
      push_inputs(&mut v, args);
      Ok(v)
    }
    "undo_addon" => {
      let id = s("addon_id");
      if id.is_empty() {
        return Err("'addon_id' is required".to_string());
      }
      Ok(vec!["undo".to_string(), id, "--yes".into()])
    }
    "list_outdated" => Ok(vec!["outdated".into(), "--json".into()]),
    other => Err(format!("Unknown tool '{other}'")),
  }
}

#[doc(hidden)]
pub fn build_argv_for_tests(name: &str, args: &Value) -> Result<Vec<String>, String> {
  build_argv(name, args)
}

fn run_tool(name: &str, args: &Value) -> (String, bool) {
  let cwd = args.get("path").and_then(Value::as_str).map(String::from);
  match build_argv(name, args) {
    Ok(mut cmd) => run_self(&mut cmd, cwd.as_deref()),
    Err(message) => (message, true),
  }
}

fn push_allow_run(cmd: &mut Vec<String>, args: &Value) {
  if args.get("allow_run").and_then(Value::as_bool) == Some(true) {
    cmd.push("--allow-run".to_string());
  }
}

fn push_overwrite(cmd: &mut Vec<String>, args: &Value) {
  if args.get("overwrite").and_then(Value::as_bool) == Some(true) {
    cmd.push("--overwrite".to_string());
  }
}

fn push_inputs(cmd: &mut Vec<String>, args: &Value) {
  if let Some(obj) = args.get("inputs").and_then(Value::as_object) {
    for (k, v) in obj {
      let value = match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
      };
      cmd.push("--input".to_string());
      cmd.push(format!("{k}={value}"));
    }
  }
}

fn run_self(args: &mut [String], cwd: Option<&str>) -> (String, bool) {
  let exe = match std::env::current_exe() {
    Ok(exe) => exe,
    Err(e) => return (format!("cannot locate anesis binary: {e}"), true),
  };

  let mut cmd = Command::new(exe);
  cmd.args(args.iter()).stdin(Stdio::null());
  if let Some(dir) = cwd {
    cmd.current_dir(dir);
  }

  match cmd.output() {
    Ok(o) => {
      let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
      let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
      let text = json!({
        "exit_code": o.status.code(),
        "stdout": stdout,
        "stderr": stderr
      })
      .to_string();
      (text, !o.status.success())
    }
    Err(e) => (format!("failed to run anesis: {e}"), true),
  }
}

pub fn push_inputs_for_tests(cmd: &mut Vec<String>, args: &Value) {
  push_inputs(cmd, args)
}

#[doc(hidden)]
pub fn push_allow_run_for_tests(cmd: &mut Vec<String>, args: &Value) {
  push_allow_run(cmd, args)
}

#[doc(hidden)]
pub fn push_overwrite_for_tests(cmd: &mut Vec<String>, args: &Value) {
  push_overwrite(cmd, args)
}

#[doc(hidden)]
pub fn tools_list_for_tests() -> Value {
  tools_list()
}

fn tools_list() -> Value {
  json!([
    {
      "name": "search_registry",
      "description": "Search the Anesis registry for templates, addons and stacks. Returns matching entries as JSON.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "query": { "type": "string", "description": "Filter text (optional; empty lists everything)" }
        }
      }
    },
    {
      "name": "get_manifest",
      "description": "Show the full manifest of a template, addon or stack (description, version, variants, commands, inputs, steps).",
      "inputSchema": {
        "type": "object",
        "properties": {
          "kind": { "type": "string", "enum": ["template", "addon", "stack"] },
          "id": { "type": "string", "description": "Template name / addon id / stack id" }
        },
        "required": ["kind", "id"]
      }
    },
    {
      "name": "scaffold_project",
      "description": "Create a new project from a template (or a stack). Non-interactive. Addon 'run'/'packages' steps are refused unless allow_run is true.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "name": { "type": "string", "description": "Project directory to create (use '.' for the current dir)" },
          "template": { "type": "string", "description": "Template name (omit if using a stack)" },
          "stack": { "type": "string", "description": "Stack id (alternative to template)" },
          "inputs": { "type": "object", "description": "Template input values by name", "additionalProperties": true },
          "allow_run": { "type": "boolean", "description": "Permit addon 'run' steps (arbitrary shell) and 'packages' steps (package installs, which run lifecycle scripts) to execute. Defaults to false; ask the user before setting it." },
          "overwrite": { "type": "boolean", "description": "Allow overwriting existing files in the destination directory. Defaults to false, which fails instead of silently clobbering files; ask the user before setting it." },
          "path": { "type": "string", "description": "Working directory to run in (defaults to the server's cwd)" }
        },
        "required": ["name"]
      }
    },
    {
      "name": "apply_addon",
      "description": "Run an addon command in an existing project. Non-interactive. If the command has a 'run' or 'packages' step it will fail unless allow_run is true.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "addon_id": { "type": "string" },
          "command": { "type": "string", "description": "Addon command to run" },
          "inputs": { "type": "object", "description": "Addon input values by name", "additionalProperties": true },
          "allow_run": { "type": "boolean", "description": "Permit addon 'run' steps (arbitrary shell) and 'packages' steps (package installs, which run lifecycle scripts) to execute. Defaults to false; ask the user before setting it." },
          "path": { "type": "string", "description": "Project directory to run in" }
        },
        "required": ["addon_id", "command"]
      }
    },
    {
      "name": "apply_stack",
      "description": "Scaffold a new project from a stack (template + ordered addons). Non-interactive. Addon 'run'/'packages' steps are refused unless allow_run is true.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "name": { "type": "string", "description": "Project directory to create" },
          "stack": { "type": "string", "description": "Stack id" },
          "inputs": { "type": "object", "additionalProperties": true },
          "allow_run": { "type": "boolean", "description": "Permit addon 'run' steps (arbitrary shell) and 'packages' steps (package installs, which run lifecycle scripts) to execute. Defaults to false; ask the user before setting it." },
          "overwrite": { "type": "boolean", "description": "Allow overwriting existing files in the destination directory. Defaults to false, which fails instead of silently clobbering files; ask the user before setting it." },
          "path": { "type": "string", "description": "Working directory to run in" }
        },
        "required": ["name", "stack"]
      }
    },
    {
      "name": "project_status",
      "description": "Report the current project's template and applied addons as JSON.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "path": { "type": "string", "description": "Project directory to inspect" }
        }
      }
    },
    {
      "name": "dry_run",
      "description": "Preview an addon command's plan (variant, inputs, steps) without changing any files. Use this before apply_addon to see what would happen.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "addon_id": { "type": "string" },
          "command": { "type": "string", "description": "Addon command to preview" },
          "inputs": { "type": "object", "description": "Addon input values by name", "additionalProperties": true },
          "path": { "type": "string", "description": "Project directory to run in" }
        },
        "required": ["addon_id", "command"]
      }
    },
    {
      "name": "undo_addon",
      "description": "Revert an applied addon's changes in the current project, using the recorded rollback journal.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "addon_id": { "type": "string", "description": "Addon id to revert" },
          "path": { "type": "string", "description": "Project directory to run in" }
        },
        "required": ["addon_id"]
      }
    },
    {
      "name": "list_outdated",
      "description": "List applied addons that have a newer version in the registry, as JSON.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "path": { "type": "string", "description": "Project directory to inspect" }
        }
      }
    }
  ])
}
