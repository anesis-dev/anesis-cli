use std::path::PathBuf;

use oxide_cli::templates::{TemplateFile, generator::{extract_dir_contents, to_kebab_case, to_snake_case}};
use tera::{Context, Tera};

fn make_context(project_name: &str) -> Context {
  let mut ctx = Context::new();
  ctx.insert("project_name", project_name);
  ctx.insert("project_name_kebab", &to_kebab_case(project_name));
  ctx.insert("project_name_snake", &to_snake_case(project_name));
  ctx
}

// ── case helpers ──────────────────────────────────────────────────────────────

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

// ── extract_dir_contents ──────────────────────────────────────────────────────

#[test]
fn renders_tera_file_and_strips_extension() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("README.md.tera"),
    contents: b"# {{ project_name }}".to_vec(),
  }];

  let mut tera = Tera::default();
  extract_dir_contents(&files, dir.path(), &mut tera, &make_context("my-app")).unwrap();

  let content = std::fs::read_to_string(dir.path().join("README.md")).unwrap();
  assert_eq!(content, "# my-app");
}

#[test]
fn copies_non_tera_file_unchanged() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("src/index.ts"),
    contents: b"console.log('hello')".to_vec(),
  }];

  let mut tera = Tera::default();
  extract_dir_contents(&files, dir.path(), &mut tera, &make_context("my-app")).unwrap();

  let content = std::fs::read_to_string(dir.path().join("src").join("index.ts")).unwrap();
  assert_eq!(content, "console.log('hello')");
}

#[test]
fn template_vars_kebab_and_snake() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("out.txt.tera"),
    contents: b"{{ project_name_kebab }} {{ project_name_snake }}".to_vec(),
  }];

  let mut tera = Tera::default();
  extract_dir_contents(&files, dir.path(), &mut tera, &make_context("My_Project")).unwrap();

  let content = std::fs::read_to_string(dir.path().join("out.txt")).unwrap();
  assert_eq!(content, "my-project my_project");
}

#[test]
fn creates_nested_output_directories() {
  let dir = assert_fs::TempDir::new().unwrap();
  let files = vec![TemplateFile {
    path: PathBuf::from("src/components/Button.tsx"),
    contents: b"export default () => null".to_vec(),
  }];

  let mut tera = Tera::default();
  extract_dir_contents(&files, dir.path(), &mut tera, &make_context("app")).unwrap();

  assert!(dir.path().join("src/components/Button.tsx").exists());
}
