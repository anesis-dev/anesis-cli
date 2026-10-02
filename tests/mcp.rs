mod common;

use anesis::mcp::{build_argv_for_tests, dispatch_for_tests, tools_list_for_tests};
use common::push_inputs_for_tests;
use serde_json::json;

#[test]
fn push_inputs_expands_object_into_flags() {
  let mut cmd = vec!["use".to_string()];
  push_inputs_for_tests(
    &mut cmd,
    &json!({ "inputs": { "db": "postgres", "port": 5432 } }),
  );
  assert!(
    cmd
      .windows(2)
      .any(|w| w[0] == "--input" && w[1] == "db=postgres")
  );
  assert!(
    cmd
      .windows(2)
      .any(|w| w[0] == "--input" && w[1] == "port=5432")
  );
}

#[test]
fn push_inputs_noop_without_inputs() {
  let mut cmd = vec!["status".to_string()];
  push_inputs_for_tests(&mut cmd, &json!({ "path": "/tmp" }));
  assert_eq!(cmd, vec!["status".to_string()]);
}

#[test]
fn allow_run_is_only_forwarded_when_the_caller_asks_for_it() {
  let mut cmd = Vec::new();
  anesis::mcp::push_allow_run_for_tests(&mut cmd, &serde_json::json!({}));
  assert!(
    cmd.is_empty(),
    "the default must not permit shell execution"
  );

  let mut cmd = Vec::new();
  anesis::mcp::push_allow_run_for_tests(&mut cmd, &serde_json::json!({ "allow_run": false }));
  assert!(cmd.is_empty());

  let mut cmd = Vec::new();
  anesis::mcp::push_allow_run_for_tests(&mut cmd, &serde_json::json!({ "allow_run": "true" }));
  assert!(cmd.is_empty(), "only a JSON boolean should opt in");

  let mut cmd = Vec::new();
  anesis::mcp::push_allow_run_for_tests(&mut cmd, &serde_json::json!({ "allow_run": true }));
  assert_eq!(cmd, vec!["--allow-run".to_string()]);
}

#[test]
fn mutating_tools_advertise_the_allow_run_option() {
  let tools = anesis::mcp::tools_list_for_tests();
  let tools = tools.as_array().expect("tools_list is an array");

  for name in ["scaffold_project", "apply_addon", "apply_stack"] {
    let tool = tools
      .iter()
      .find(|t| t["name"] == name)
      .unwrap_or_else(|| panic!("tool {name} is missing"));

    assert_eq!(
      tool["inputSchema"]["properties"]["allow_run"]["type"], "boolean",
      "{name} must expose allow_run"
    );
    assert!(
      tool["description"].as_str().unwrap().contains("allow_run"),
      "{name}'s description should warn about run steps"
    );
  }
}

#[test]
fn overwrite_is_only_forwarded_when_the_caller_asks_for_it() {
  let mut cmd = Vec::new();
  anesis::mcp::push_overwrite_for_tests(&mut cmd, &serde_json::json!({}));
  assert!(cmd.is_empty(), "the default must not permit overwriting");

  let mut cmd = Vec::new();
  anesis::mcp::push_overwrite_for_tests(&mut cmd, &serde_json::json!({ "overwrite": false }));
  assert!(cmd.is_empty());

  let mut cmd = Vec::new();
  anesis::mcp::push_overwrite_for_tests(&mut cmd, &serde_json::json!({ "overwrite": "true" }));
  assert!(cmd.is_empty(), "only a JSON boolean should opt in");

  let mut cmd = Vec::new();
  anesis::mcp::push_overwrite_for_tests(&mut cmd, &serde_json::json!({ "overwrite": true }));
  assert_eq!(cmd, vec!["--overwrite".to_string()]);
}

#[test]
fn scaffold_project_does_not_pass_overwrite_by_default() {
  let argv = build_argv_for_tests(
    "scaffold_project",
    &serde_json::json!({ "name": "proj", "template": "some-template" }),
  )
  .unwrap();
  assert!(
    !argv.contains(&"--overwrite".to_string()),
    "scaffold_project must not silently permit overwriting: {argv:?}"
  );
}

#[test]
fn scaffold_project_and_apply_stack_advertise_the_overwrite_option() {
  let tools = anesis::mcp::tools_list_for_tests();
  let tools = tools.as_array().expect("tools_list is an array");

  for name in ["scaffold_project", "apply_stack"] {
    let tool = tools
      .iter()
      .find(|t| t["name"] == name)
      .unwrap_or_else(|| panic!("tool {name} is missing"));

    assert_eq!(
      tool["inputSchema"]["properties"]["overwrite"]["type"], "boolean",
      "{name} must expose overwrite"
    );
  }
}

#[test]
fn tools_list_matches_the_snapshot() {
  insta::assert_json_snapshot!(anesis::mcp::tools_list_for_tests());
}

#[test]
fn dispatch_initialize_reports_protocol_version_and_server_info() {
  let result = dispatch_for_tests("initialize", None).unwrap();
  assert_eq!(result["protocolVersion"], "2024-11-05");
  assert_eq!(result["serverInfo"]["name"], "anesis");
}

#[test]
fn dispatch_ping_is_an_empty_object() {
  let result = dispatch_for_tests("ping", None).unwrap();
  assert_eq!(result, json!({}));
}

#[test]
fn dispatch_tools_list_wraps_the_tool_array() {
  let result = dispatch_for_tests("tools/list", None).unwrap();
  assert!(result["tools"].as_array().unwrap().len() >= 5);
}

#[test]
fn dispatch_unknown_method_is_minus_32601() {
  let err = dispatch_for_tests("not_a_real_method", None).unwrap_err();
  assert_eq!(err.0, -32601);
  assert!(err.1.contains("not_a_real_method"));
}

#[test]
fn dispatch_tools_call_with_missing_params_is_minus_32602() {
  let err = dispatch_for_tests("tools/call", None).unwrap_err();
  assert_eq!(err.0, -32602);
}

#[test]
fn dispatch_tools_call_with_missing_name_is_minus_32602() {
  let err = dispatch_for_tests("tools/call", Some(&json!({ "arguments": {} }))).unwrap_err();
  assert_eq!(err.0, -32602);
}

#[test]
fn dispatch_tools_call_defaults_missing_arguments_to_an_empty_object() {
  let result =
    dispatch_for_tests("tools/call", Some(&json!({ "name": "project_status" }))).unwrap();
  assert_eq!(result["content"][0]["type"], "text");
}

#[test]
fn dispatch_tools_call_reports_an_unknown_tool_as_a_successful_result_with_is_error() {
  let result = dispatch_for_tests(
    "tools/call",
    Some(&json!({ "name": "not_a_real_tool", "arguments": {} })),
  )
  .unwrap();
  assert_eq!(result["isError"], true);
  assert!(
    result["content"][0]["text"]
      .as_str()
      .unwrap()
      .contains("not_a_real_tool")
  );
}

#[test]
fn build_argv_search_registry_with_no_query_lists_everything() {
  let argv = build_argv_for_tests("search_registry", &json!({})).unwrap();
  assert_eq!(argv, vec!["search".to_string(), "--json".to_string()]);
}

#[test]
fn build_argv_search_registry_with_a_query() {
  let argv = build_argv_for_tests("search_registry", &json!({ "query": "docker" })).unwrap();
  assert_eq!(
    argv,
    vec![
      "search".to_string(),
      "docker".to_string(),
      "--json".to_string()
    ]
  );
}

#[test]
fn build_argv_get_manifest_requires_id() {
  let err = build_argv_for_tests("get_manifest", &json!({ "kind": "addon" })).unwrap_err();
  assert!(err.contains("'id' is required"));
}

#[test]
fn build_argv_get_manifest_rejects_an_unknown_kind() {
  let err =
    build_argv_for_tests("get_manifest", &json!({ "kind": "bogus", "id": "x" })).unwrap_err();
  assert!(err.contains("Unknown kind 'bogus'"));
}

#[test]
fn build_argv_get_manifest_covers_all_three_kinds() {
  for (kind, expected_group) in [
    ("template", "template"),
    ("addon", "addon"),
    ("stack", "stack"),
  ] {
    let argv = build_argv_for_tests("get_manifest", &json!({ "kind": kind, "id": "x" })).unwrap();
    assert_eq!(argv[0], expected_group);
    assert_eq!(argv[1], "info");
    assert_eq!(argv[2], "x");
    assert_eq!(argv[3], "--json");
  }
}

#[test]
fn build_argv_scaffold_project_requires_name() {
  let err = build_argv_for_tests("scaffold_project", &json!({ "template": "t" })).unwrap_err();
  assert!(err.contains("'name' is required"));
}

#[test]
fn build_argv_scaffold_project_requires_template_or_stack() {
  let err = build_argv_for_tests("scaffold_project", &json!({ "name": "proj" })).unwrap_err();
  assert!(err.contains("Provide either 'template' or 'stack'"));
}

#[test]
fn build_argv_scaffold_project_prefers_stack_over_template_when_both_are_given() {
  let argv = build_argv_for_tests(
    "scaffold_project",
    &json!({ "name": "proj", "template": "t", "stack": "s" }),
  )
  .unwrap();
  assert!(argv.contains(&"--stack".to_string()));
  assert!(!argv.contains(&"t".to_string()));
}

#[test]
fn build_argv_apply_addon_requires_addon_id_and_command() {
  let err = build_argv_for_tests("apply_addon", &json!({ "addon_id": "docker" })).unwrap_err();
  assert!(err.contains("'addon_id' and 'command' are required"));
}

#[test]
fn build_argv_apply_stack_requires_name_and_stack() {
  let err = build_argv_for_tests("apply_stack", &json!({ "name": "proj" })).unwrap_err();
  assert!(err.contains("'name' and 'stack' are required"));
}

#[test]
fn build_argv_project_status_needs_no_arguments() {
  let argv = build_argv_for_tests("project_status", &json!({})).unwrap();
  assert_eq!(argv, vec!["status".to_string(), "--json".to_string()]);
}

#[test]
fn build_argv_dry_run_requires_addon_id_and_command() {
  let err = build_argv_for_tests("dry_run", &json!({ "addon_id": "docker" })).unwrap_err();
  assert!(err.contains("'addon_id' and 'command' are required"));
}

#[test]
fn build_argv_dry_run_maps_to_use_with_dry_run_flag() {
  let argv = build_argv_for_tests(
    "dry_run",
    &json!({ "addon_id": "docker", "command": "add-entity" }),
  )
  .unwrap();
  assert_eq!(
    argv,
    vec![
      "use".to_string(),
      "docker".to_string(),
      "add-entity".to_string(),
      "--dry-run".to_string()
    ]
  );
}

#[test]
fn build_argv_dry_run_never_passes_allow_run_or_yes() {
  let argv = build_argv_for_tests(
    "dry_run",
    &json!({ "addon_id": "docker", "command": "add-entity", "allow_run": true }),
  )
  .unwrap();
  assert!(!argv.contains(&"--allow-run".to_string()));
  assert!(!argv.contains(&"--yes".to_string()));
}

#[test]
fn build_argv_undo_addon_requires_addon_id() {
  let err = build_argv_for_tests("undo_addon", &json!({})).unwrap_err();
  assert!(err.contains("'addon_id' is required"));
}

#[test]
fn build_argv_undo_addon_maps_to_undo_with_yes() {
  let argv = build_argv_for_tests("undo_addon", &json!({ "addon_id": "docker" })).unwrap();
  assert_eq!(
    argv,
    vec![
      "undo".to_string(),
      "docker".to_string(),
      "--yes".to_string()
    ]
  );
}

#[test]
fn build_argv_list_outdated_needs_no_arguments() {
  let argv = build_argv_for_tests("list_outdated", &json!({})).unwrap();
  assert_eq!(argv, vec!["outdated".to_string(), "--json".to_string()]);
}

#[test]
fn tools_list_includes_dry_run_undo_addon_and_list_outdated() {
  let tools = tools_list_for_tests();
  let names: Vec<&str> = tools
    .as_array()
    .unwrap()
    .iter()
    .map(|t| t["name"].as_str().unwrap())
    .collect();
  assert!(names.contains(&"dry_run"));
  assert!(names.contains(&"undo_addon"));
  assert!(names.contains(&"list_outdated"));
}

#[test]
fn build_argv_rejects_an_unknown_tool() {
  let err = build_argv_for_tests("not_a_real_tool", &json!({})).unwrap_err();
  assert!(err.contains("Unknown tool 'not_a_real_tool'"));
}
