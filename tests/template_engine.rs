use anesis::addons::steps::render_string as addon_render_string;
use anesis::utils::template_engine::{TemplateContext, hardened_env, render_named, render_string};

fn ctx() -> TemplateContext {
  TemplateContext::new()
}

#[test]
fn get_env_is_denied_by_hardened_env() {
  let mut env = hardened_env();
  let err = render_named(&mut env, "t", "{{ get_env(name=\"HOME\") }}", &ctx()).unwrap_err();
  assert!(format!("{err:?}").contains("get_env"));
}

#[test]
fn get_env_is_denied_in_render_string() {
  let err = render_string("{{ get_env(name=\"AUDIT_FAKE_SECRET\") }}", &ctx()).unwrap_err();
  assert!(format!("{err:?}").contains("get_env"));
}

#[test]
fn get_env_is_denied_in_addon_step_rendering() {
  let err = addon_render_string("{{ get_env(name=\"AUDIT_FAKE_SECRET\") }}", &ctx()).unwrap_err();
  assert!(format!("{err:?}").contains("get_env"));
}

#[test]
fn get_env_is_denied_in_template_files() {
  use anesis::context::AppContext;
  use anesis::paths::AnesisPaths;
  use anesis::templates::TemplateFile;
  use anesis::templates::generator::extract_template;
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::{Arc, Mutex};

  let out = assert_fs::TempDir::new().unwrap();
  let paths = AnesisPaths::new().unwrap();
  let app_ctx = AppContext::new(paths, reqwest::Client::new(), Arc::new(Mutex::new(None)));

  let files = vec![TemplateFile {
    path: PathBuf::from("leaked.txt.tera"),
    contents: b"{{ get_env(name=\"AUDIT_FAKE_SECRET\") }}".to_vec(),
  }];

  let err = extract_template(
    &files,
    out.path(),
    "my-app",
    &app_ctx,
    &HashMap::new(),
    &std::collections::HashSet::new(),
  )
  .unwrap_err();

  assert!(format!("{err:?}").contains("get_env"));
  assert!(!out.path().join("leaked.txt").exists());
}

#[test]
fn existing_filters_still_work() {
  let mut c = ctx();
  c.insert("name", "world");
  let out = render_string("{{ name | upper }}", &c).unwrap();
  assert_eq!(out, "WORLD");
}

#[test]
fn missing_variable_is_an_error_when_printed() {
  assert!(render_string("{{ nope }}", &ctx()).is_err());
}

#[test]
fn missing_variable_is_falsy_in_conditions() {
  let out = render_string("{% if nope %}yes{% else %}no{% endif %}", &ctx()).unwrap();
  assert_eq!(out, "no");
}

#[test]
fn trailing_newline_is_preserved() {
  let mut c = ctx();
  c.insert("name", "app");
  assert_eq!(render_string("{{ name }}\n", &c).unwrap(), "app\n");
}

#[test]
fn output_is_not_html_escaped() {
  let mut c = ctx();
  c.insert("name", "<a&b>");
  assert_eq!(render_string("{{ name }}", &c).unwrap(), "<a&b>");
}

#[test]
fn github_actions_expressions_are_rejected_not_silently_swallowed() {
  assert!(render_string("${{ matrix.os }}", &ctx()).is_err());
}
