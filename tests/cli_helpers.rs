use anesis::cli::commands::{AddonCommands, Commands, StackCommands, TemplateCommands};
use anesis::cli::dispatch::{is_json_mode, parse_inputs, skip_version_notice};
use anesis::utils::picker::{ItemKind, PickItem, search_results_json};

#[test]
fn parse_inputs_splits_on_the_first_equals_sign() {
  let map = parse_inputs(&["name=value".to_string()]).unwrap();
  assert_eq!(map.get("name"), Some(&"value".to_string()));
}

#[test]
fn parse_inputs_keeps_a_second_equals_sign_as_part_of_the_value() {
  let map = parse_inputs(&["url=https://x/y=z".to_string()]).unwrap();
  assert_eq!(map.get("url"), Some(&"https://x/y=z".to_string()));
}

#[test]
fn parse_inputs_allows_an_empty_value() {
  let map = parse_inputs(&["flag=".to_string()]).unwrap();
  assert_eq!(map.get("flag"), Some(&"".to_string()));
}

#[test]
fn parse_inputs_rejects_an_empty_name() {
  let err = parse_inputs(&["=value".to_string()]).unwrap_err();
  assert!(err.to_string().contains("cannot be empty"));
}

#[test]
fn parse_inputs_rejects_a_pair_with_no_equals_sign() {
  let err = parse_inputs(&["not-a-pair".to_string()]).unwrap_err();
  assert!(err.to_string().contains("not-a-pair"));
}

#[test]
fn parse_inputs_last_duplicate_wins() {
  let map = parse_inputs(&["k=first".to_string(), "k=second".to_string()]).unwrap();
  assert_eq!(map.get("k"), Some(&"second".to_string()));
}

fn new_cmd() -> Commands {
  Commands::New {
    name: "app".to_string(),
    template_name: None,
    stack: None,
    installed: false,
    yes: false,
    overwrite: false,
    input: vec![],
    dry_run: false,
  }
}

#[test]
fn is_json_mode_true_for_every_json_flagged_variant() {
  assert!(is_json_mode(&Commands::Account { json: true }));
  assert!(is_json_mode(&Commands::Info { json: true }));
  assert!(is_json_mode(&Commands::Status { json: true }));
  assert!(is_json_mode(&Commands::Search {
    query: None,
    json: true
  }));
  assert!(is_json_mode(&Commands::Outdated { json: true }));
  assert!(is_json_mode(&Commands::Doctor { json: true }));
  assert!(is_json_mode(&Commands::Why {
    path: None,
    json: true
  }));
  assert!(is_json_mode(&Commands::Template {
    command: TemplateCommands::List { json: true }
  }));
  assert!(is_json_mode(&Commands::Template {
    command: TemplateCommands::Info {
      template_name: "t".to_string(),
      json: true
    }
  }));
  assert!(is_json_mode(&Commands::Addon {
    command: AddonCommands::List { json: true }
  }));
  assert!(is_json_mode(&Commands::Addon {
    command: AddonCommands::Info {
      addon_id: "a".to_string(),
      json: true
    }
  }));
  assert!(is_json_mode(&Commands::Stack {
    command: StackCommands::List { json: true }
  }));
  assert!(is_json_mode(&Commands::Stack {
    command: StackCommands::Info {
      stack_id: "s".to_string(),
      json: true
    }
  }));
}

#[test]
fn is_json_mode_false_when_the_json_flag_is_off_or_absent() {
  assert!(!is_json_mode(&Commands::Info { json: false }));
  assert!(!is_json_mode(&Commands::Doctor { json: false }));
  assert!(!is_json_mode(&Commands::Why {
    path: None,
    json: false
  }));
  assert!(!is_json_mode(&Commands::Template {
    command: TemplateCommands::List { json: false }
  }));
  assert!(!is_json_mode(&new_cmd()));
  assert!(!is_json_mode(&Commands::Upgrade));
}

#[test]
fn skip_version_notice_true_when_quiet() {
  assert!(skip_version_notice(&new_cmd(), true));
}

#[test]
fn skip_version_notice_true_in_json_mode() {
  assert!(skip_version_notice(&Commands::Info { json: true }, false));
}

#[test]
fn skip_version_notice_true_for_upgrade_completions_and_man() {
  assert!(skip_version_notice(&Commands::Upgrade, false));
  assert!(skip_version_notice(
    &Commands::Completions {
      shell: anesis::completions::CompletionShell::Bash,
      print: false
    },
    false
  ));
  assert!(skip_version_notice(
    &Commands::Man {
      dir: "/tmp".to_string()
    },
    false
  ));
}

#[test]
fn skip_version_notice_false_for_an_ordinary_command() {
  assert!(!skip_version_notice(&new_cmd(), false));
}

fn item(kind: ItemKind, id: &str, name: &str, description: &str) -> PickItem {
  PickItem {
    kind,
    id: id.to_string(),
    name: name.to_string(),
    meta: String::new(),
    description: description.to_string(),
    haystack: format!("{name} {description}").to_lowercase(),
  }
}

#[test]
fn search_results_json_empty_query_returns_everything() {
  let items = vec![
    item(ItemKind::Template, "t1", "React", "A React app"),
    item(ItemKind::Addon, "a1", "Docker", "Container setup"),
  ];
  let results = search_results_json(&items, None);
  assert_eq!(results.len(), 2);
}

#[test]
fn search_results_json_filters_case_insensitively_on_the_haystack() {
  let items = vec![
    item(ItemKind::Template, "t1", "React", "A frontend framework"),
    item(ItemKind::Addon, "a1", "Docker", "Container setup"),
  ];
  let results = search_results_json(&items, Some("REACT"));
  assert_eq!(results.len(), 1);
  assert_eq!(results[0]["id"], "t1");
}

#[test]
fn search_results_json_maps_each_item_kind_to_its_string_tag() {
  let items = vec![
    item(ItemKind::Template, "t1", "T", "d"),
    item(ItemKind::Addon, "a1", "A", "d"),
    item(ItemKind::Stack, "s1", "S", "d"),
  ];
  let results = search_results_json(&items, None);
  let kinds: Vec<&str> = results
    .iter()
    .map(|r| r["kind"].as_str().unwrap())
    .collect();
  assert_eq!(kinds, vec!["template", "addon", "stack"]);
}

#[test]
fn search_results_json_projects_only_the_documented_fields() {
  let items = vec![item(ItemKind::Template, "t1", "React", "A React app")];
  let results = search_results_json(&items, None);
  let obj = results[0].as_object().unwrap();
  let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
  keys.sort();
  assert_eq!(keys, vec!["description", "id", "kind", "name"]);
}
