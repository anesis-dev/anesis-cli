use std::collections::HashMap;

use anesis::stacks::manifest::*;

#[test]
fn parses_and_defaults_command() {
  let stack: StackManifest = serde_json::from_str(
    r#"{"schema_version":"1","id":"s","name":"S","version":"1.0.0",
        "author":{"name":"Maksym Zhuk","github":"anesis-dev"},"template":"nest-express",
        "addons":[{"id":"nest-config"},{"id":"nest-prisma","command":"generate","inputs":{"db":"postgres"}}]}"#,
  )
  .unwrap();
  validate(&stack).unwrap();
  assert_eq!(stack.version, "1.0.0");
  assert_eq!(stack.author.github, "anesis-dev");
  assert_eq!(stack.addons[0].command, "install");
  assert_eq!(stack.addons[1].command, "generate");
  assert_eq!(stack.addons[1].inputs.get("db").unwrap(), "postgres");
}

#[test]
fn rejects_manifest_missing_version_or_author() {
  let missing_version = serde_json::from_str::<StackManifest>(
    r#"{"schema_version":"1","id":"s","name":"S","template":"nest"}"#,
  );
  assert!(missing_version.is_err());
}

#[test]
fn rejects_empty_template_and_addon_id() {
  let mut stack: StackManifest = serde_json::from_str(
    r#"{"schema_version":"1","id":"s","name":"S","version":"1.0.0",
        "author":{"name":"Maksym Zhuk","github":"anesis-dev"},"template":"nest"}"#,
  )
  .unwrap();
  stack.template = " ".into();
  assert!(validate(&stack).is_err());
  stack.template = "nest".into();
  stack.addons.push(StackAddon {
    id: "".into(),
    command: "install".into(),
    inputs: HashMap::new(),
  });
  assert!(validate(&stack).is_err());
}

#[test]
fn rejects_a_traversing_template_name() {
  let mut stack: StackManifest = serde_json::from_str(
    r#"{"schema_version":"1","id":"s","name":"S","version":"1.0.0",
        "author":{"name":"Maksym Zhuk","github":"anesis-dev"},"template":"nest"}"#,
  )
  .unwrap();
  stack.template = "../../etc/passwd".into();
  assert!(validate(&stack).is_err());
}

#[test]
fn rejects_a_traversing_addon_id() {
  let mut stack: StackManifest = serde_json::from_str(
    r#"{"schema_version":"1","id":"s","name":"S","version":"1.0.0",
        "author":{"name":"Maksym Zhuk","github":"anesis-dev"},"template":"nest"}"#,
  )
  .unwrap();
  stack.addons.push(StackAddon {
    id: "../../../tmp/pwned".into(),
    command: "install".into(),
    inputs: HashMap::new(),
  });
  assert!(validate(&stack).is_err());
}
