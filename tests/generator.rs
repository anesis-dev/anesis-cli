use std::{
  collections::HashMap,
  path::PathBuf,
  sync::{Arc, Mutex},
};

use anesis::{
  context::AppContext,
  paths::AnesisPaths,
  templates::{
    ExcludeBlock, TemplateFile,
    generator::{
      eval_when, excluded_paths, extract_dir_contents, extract_template, overwritten_paths,
      to_camel_case, to_kebab_case, to_pascal_case, to_snake_case,
    },
  },
  utils::template_engine::{TemplateContext, hardened_env},
};

fn inputs(pairs: &[(&str, &str)]) -> HashMap<String, String> {
  pairs
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

fn make_context(project_name: &str) -> TemplateContext {
  let mut ctx = TemplateContext::new();
  ctx.insert("project_name", project_name);
  ctx.insert("project_name_kebab", &to_kebab_case(project_name));
  ctx.insert("project_name_snake", &to_snake_case(project_name));
  ctx
}

fn make_app_context() -> AppContext {
  let paths = AnesisPaths::new().unwrap();
  AppContext::new(paths, reqwest::Client::new(), Arc::new(Mutex::new(None)))
}

#[test]
fn kebab_case_replaces_underscores_and_spaces() {
  assert_eq!(to_kebab_case("My_Project Name"), "my-project-name");
  assert_eq!(to_kebab_case("hello"), "hello");
  assert_eq!(to_kebab_case("Hello_World"), "hello-world");
}

#[test]
fn snake_case_replaces_hyphens_and_spaces() {
  assert_eq!(to_snake_case("my-project-name"), "my_project_name");
  assert_eq!(to_snake_case("hello"), "hello");
  assert_eq!(to_snake_case("Hello-World"), "hello_world");
}

#[test]
fn renders_tera_file_and_strips_extension() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("README.md.tera"),
    contents: b"# {{ project_name }}".to_vec(),
    mode: None,
  }];

  let mut env = hardened_env();
  extract_dir_contents(
    &files,
    dir.path(),
    &mut env,
    &make_context("my-app"),
    &make_app_context(),
    &std::collections::HashSet::new(),
  )
  .unwrap();

  let content = std::fs::read_to_string(dir.path().join("README.md")).unwrap();
  assert_eq!(content, "# my-app");
}

#[test]
fn copies_non_tera_file_unchanged() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("src/index.ts"),
    contents: b"console.log('hello')".to_vec(),
    mode: None,
  }];

  let mut env = hardened_env();
  extract_dir_contents(
    &files,
    dir.path(),
    &mut env,
    &make_context("my-app"),
    &make_app_context(),
    &std::collections::HashSet::new(),
  )
  .unwrap();

  let content = std::fs::read_to_string(dir.path().join("src").join("index.ts")).unwrap();
  assert_eq!(content, "console.log('hello')");
}

#[test]
fn template_vars_kebab_and_snake() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("out.txt.tera"),
    contents: b"{{ project_name_kebab }} {{ project_name_snake }}".to_vec(),
    mode: None,
  }];

  let mut env = hardened_env();
  extract_dir_contents(
    &files,
    dir.path(),
    &mut env,
    &make_context("My_Project"),
    &make_app_context(),
    &std::collections::HashSet::new(),
  )
  .unwrap();

  let content = std::fs::read_to_string(dir.path().join("out.txt")).unwrap();
  assert_eq!(content, "my-project my_project");
}

#[test]
fn creates_nested_output_directories() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("src/components/Button.tsx"),
    contents: b"export default () => null".to_vec(),
    mode: None,
  }];

  let mut env = hardened_env();
  extract_dir_contents(
    &files,
    dir.path(),
    &mut env,
    &make_context("app"),
    &make_app_context(),
    &std::collections::HashSet::new(),
  )
  .unwrap();

  assert!(dir.path().join("src/components/Button.tsx").exists());
}

#[test]
fn pascal_case_from_kebab() {
  assert_eq!(to_pascal_case("my-project-name"), "MyProjectName");
}

#[test]
fn pascal_case_from_snake() {
  assert_eq!(to_pascal_case("my_project_name"), "MyProjectName");
}

#[test]
fn pascal_case_single_word() {
  assert_eq!(to_pascal_case("hello"), "Hello");
}

#[test]
fn pascal_case_already_pascal() {
  assert_eq!(to_pascal_case("MyProject"), "MyProject");
}

#[test]
fn pascal_case_empty_string() {
  assert_eq!(to_pascal_case(""), "");
}

#[test]
fn pascal_case_with_spaces() {
  assert_eq!(to_pascal_case("my project name"), "MyProjectName");
}

#[test]
fn camel_case_from_kebab() {
  assert_eq!(to_camel_case("my-project-name"), "myProjectName");
}

#[test]
fn camel_case_from_snake() {
  assert_eq!(to_camel_case("my_project_name"), "myProjectName");
}

#[test]
fn camel_case_single_word() {
  assert_eq!(to_camel_case("hello"), "hello");
  assert_eq!(to_camel_case("Hello"), "hello");
}

#[test]
fn camel_case_empty_string() {
  assert_eq!(to_camel_case(""), "");
}

#[test]
fn camel_case_with_spaces() {
  assert_eq!(to_camel_case("my project"), "myProject");
}

#[test]
fn path_traversal_blocked_by_extract_dir_contents() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("../../etc/passwd"),
    contents: b"evil".to_vec(),
    mode: None,
  }];

  let mut env = hardened_env();
  let result = extract_dir_contents(
    &files,
    dir.path(),
    &mut env,
    &make_context("app"),
    &make_app_context(),
    &std::collections::HashSet::new(),
  );
  assert!(result.is_err(), "path traversal should be blocked");
}

#[test]
fn path_traversal_with_tera_file_blocked() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("../sibling.txt"),
    contents: b"escaped".to_vec(),
    mode: None,
  }];

  let mut env = hardened_env();
  let result = extract_dir_contents(
    &files,
    dir.path(),
    &mut env,
    &make_context("app"),
    &make_app_context(),
    &std::collections::HashSet::new(),
  );
  assert!(
    result.is_err(),
    "single-level traversal should also be blocked"
  );
}

#[test]
fn renders_all_three_case_variables() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("vars.txt.tera"),
    contents: b"{{ project_name }} {{ project_name_kebab }} {{ project_name_snake }}".to_vec(),
    mode: None,
  }];

  let mut env = hardened_env();
  extract_dir_contents(
    &files,
    dir.path(),
    &mut env,
    &make_context("My_App"),
    &make_app_context(),
    &std::collections::HashSet::new(),
  )
  .unwrap();

  let content = std::fs::read_to_string(dir.path().join("vars.txt")).unwrap();
  assert_eq!(content, "My_App my-app my_app");
}

#[test]
fn multiple_tera_files_rendered_independently() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![
    TemplateFile {
      path: PathBuf::from("a.txt.tera"),
      contents: b"A: {{ project_name }}".to_vec(),
      mode: None,
    },
    TemplateFile {
      path: PathBuf::from("b.txt.tera"),
      contents: b"B: {{ project_name_kebab }}".to_vec(),
      mode: None,
    },
  ];

  let mut env = hardened_env();
  extract_dir_contents(
    &files,
    dir.path(),
    &mut env,
    &make_context("MyApp"),
    &make_app_context(),
    &std::collections::HashSet::new(),
  )
  .unwrap();

  let a = std::fs::read_to_string(dir.path().join("a.txt")).unwrap();
  let b = std::fs::read_to_string(dir.path().join("b.txt")).unwrap();
  assert_eq!(a, "A: MyApp");
  assert_eq!(b, "B: myapp");
}

#[test]
fn eval_when_handles_negation_and_missing() {
  let on = inputs(&[("use_docker", "true")]);
  let off = inputs(&[("use_docker", "false")]);
  assert!(eval_when("use_docker", &on));
  assert!(!eval_when("use_docker", &off));
  assert!(eval_when("!use_docker", &off));
  assert!(!eval_when("!use_docker", &on));
  assert!(!eval_when("use_docker", &HashMap::new()));
  assert!(eval_when("!use_docker", &HashMap::new()));
}

fn tempdir_app_context() -> (
  assert_fs::TempDir,
  AppContext,
  anesis::context::CleanupState,
) {
  let home = assert_fs::TempDir::new().unwrap();
  let paths = AnesisPaths::under(home.path());
  paths.ensure_directories().unwrap();
  let cleanup_state: anesis::context::CleanupState = Arc::new(Mutex::new(None));
  let ctx = AppContext::new(paths, reqwest::Client::new(), cleanup_state.clone());
  (home, ctx, cleanup_state)
}

#[test]
fn extract_template_renders_pascal_and_camel_case_project_name() {
  let (_home, ctx, _cleanup) = tempdir_app_context();
  let out = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("vars.txt.tera"),
    contents: b"{{ project_name_pascal }} {{ project_name_camel }}".to_vec(),
    mode: None,
  }];

  extract_template(
    &files,
    out.path(),
    "my-cool-app",
    &ctx,
    &HashMap::new(),
    &std::collections::HashSet::new(),
  )
  .unwrap();

  let content = std::fs::read_to_string(out.path().join("vars.txt")).unwrap();
  assert_eq!(
    content, "MyCoolApp myCoolApp",
    "project_name_pascal/_camel must be available to templates, matching \
     insert_with_derived's convention for addon inputs"
  );
}

#[test]
fn extract_template_removes_the_partial_project_when_rendering_fails() {
  let (_home, ctx, cleanup_state) = tempdir_app_context();
  let out = assert_fs::TempDir::new().unwrap();
  let fresh_dir = out.path().join("does-not-exist-yet");
  let files = vec![
    TemplateFile {
      path: PathBuf::from("a.txt"),
      contents: b"written before the failure".to_vec(),
      mode: None,
    },
    TemplateFile {
      path: PathBuf::from("bad.txt.tera"),
      contents: b"{% if x %}".to_vec(),
      mode: None,
    },
  ];

  let result = extract_template(
    &files,
    &fresh_dir,
    "app",
    &ctx,
    &HashMap::new(),
    &std::collections::HashSet::new(),
  );
  assert!(result.is_err(), "the malformed tera template must fail");

  assert!(
    !fresh_dir.exists(),
    "a half-generated project must be removed so the command can be retried"
  );
  assert!(cleanup_state.lock().unwrap().is_none());
}

#[test]
fn extract_template_removes_only_new_files_when_the_directory_already_existed() {
  let (_home, ctx, cleanup_state) = tempdir_app_context();
  let out = assert_fs::TempDir::new().unwrap();
  std::fs::create_dir_all(out.path()).unwrap();
  std::fs::write(out.path().join("pre-existing.txt"), "keep me").unwrap();

  let files = vec![
    TemplateFile {
      path: PathBuf::from("new-file.txt"),
      contents: b"new content".to_vec(),
      mode: None,
    },
    TemplateFile {
      path: PathBuf::from("bad.txt.tera"),
      contents: b"{% if x %}".to_vec(),
      mode: None,
    },
  ];

  let result = extract_template(
    &files,
    out.path(),
    "app",
    &ctx,
    &HashMap::new(),
    &std::collections::HashSet::new(),
  );
  assert!(result.is_err());

  assert!(
    !out.path().join("new-file.txt").exists(),
    "files created by the failed generation must be removed"
  );
  assert_eq!(
    std::fs::read_to_string(out.path().join("pre-existing.txt")).unwrap(),
    "keep me",
    "a file that predates this generation must never be deleted"
  );
  assert!(cleanup_state.lock().unwrap().is_none());
}

#[test]
fn extract_template_clears_the_cleanup_task_on_success() {
  let (_home, ctx, cleanup_state) = tempdir_app_context();
  let out = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("ok.txt"),
    contents: b"fine".to_vec(),
    mode: None,
  }];

  extract_template(
    &files,
    out.path(),
    "app",
    &ctx,
    &HashMap::new(),
    &std::collections::HashSet::new(),
  )
  .unwrap();

  assert!(
    cleanup_state.lock().unwrap().is_none(),
    "a successful generation must not leave a cleanup task registered"
  );
}

#[test]
fn overwritten_paths_and_is_excluded_agree_on_the_tera_stripped_name() {
  let out = assert_fs::TempDir::new().unwrap();
  std::fs::write(out.path().join("config.txt"), "old").unwrap();

  let files = vec![TemplateFile {
    path: PathBuf::from("config.txt.tera"),
    contents: b"{{ project_name }}".to_vec(),
    mode: None,
  }];

  let hits = overwritten_paths(&files, out.path(), &std::collections::HashSet::new()).unwrap();
  assert_eq!(
    hits,
    vec![out.path().join("config.txt")],
    "overwritten_paths must resolve the .tera file against its stripped output name, \
     the same way extract_dir_contents actually writes it"
  );
}

#[test]
#[cfg(unix)]
fn extract_template_blocks_a_template_path_that_escapes_through_a_pre_existing_symlink() {
  let (_home, ctx, _cleanup) = tempdir_app_context();
  let out = assert_fs::TempDir::new().unwrap();
  let escape_target = assert_fs::TempDir::new().unwrap();
  std::os::unix::fs::symlink(escape_target.path(), out.path().join("escape")).unwrap();

  let files = vec![TemplateFile {
    path: PathBuf::from("escape/evil.txt"),
    contents: b"pwned".to_vec(),
    mode: None,
  }];

  let result = extract_template(
    &files,
    out.path(),
    "app",
    &ctx,
    &HashMap::new(),
    &std::collections::HashSet::new(),
  );

  assert!(
    result.is_err(),
    "writing through a pre-existing symlink must be blocked"
  );
  assert!(
    !escape_target.path().join("evil.txt").exists(),
    "the file must never be written outside the output directory"
  );
}

#[test]
fn excluded_paths_collects_only_active_blocks() {
  let blocks = vec![
    ExcludeBlock {
      when: "!use_docker".into(),
      paths: vec!["Dockerfile".into(), "docker-compose.yml".into()],
    },
    ExcludeBlock {
      when: "use_swagger".into(),
      paths: vec!["swagger.ts".into()],
    },
  ];
  let set = excluded_paths(
    &blocks,
    &inputs(&[("use_docker", "false"), ("use_swagger", "false")]),
  );
  assert!(set.contains(&PathBuf::from("Dockerfile")));
  assert!(set.contains(&PathBuf::from("docker-compose.yml")));
  assert!(!set.contains(&PathBuf::from("swagger.ts")));
}
