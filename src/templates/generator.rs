use std::{
  fs,
  path::{Path, PathBuf},
};

use anyhow::Result;
use tera::{Context, Tera};

use crate::templates::TemplateFile;

pub fn extract_template(files: &[TemplateFile], project_name: &str) -> Result<()> {
  let output_path = PathBuf::from(project_name);
  fs::create_dir_all(&output_path)?;

  let mut context = Context::new();
  context.insert("project_name", project_name);
  context.insert("project_name_kebab", &to_kebab_case(project_name));
  context.insert("project_name_snake", &to_snake_case(project_name));
  context.insert("tauri_user_name", "tauri");

  let mut tera = Tera::default();

  extract_dir_contents(files, &output_path, &mut tera, &context)?;

  Ok(())
}

fn to_kebab_case(s: &str) -> String {
  s.chars()
    .map(|c| match c {
      '_' | ' ' => '-',
      _ => c,
    })
    .collect::<String>()
    .to_lowercase()
}

fn to_snake_case(s: &str) -> String {
  s.chars()
    .map(|c| match c {
      '-' | ' ' => '_',
      _ => c,
    })
    .collect::<String>()
    .to_lowercase()
}

pub fn extract_dir_contents(
  files: &[TemplateFile],
  base_path: &Path,
  tera: &mut Tera,
  context: &Context,
) -> Result<()> {
  for file in files {
    let file_name = file
      .path
      .file_name()
      .ok_or_else(|| anyhow::anyhow!("Invalid file path: {}", file.path.display()))?;
    let file_name_str = file_name.to_string_lossy();
    let template_key = file.path.to_string_lossy();

    let output_path = base_path.join(&file.path);
    if let Some(parent) = output_path.parent() {
      fs::create_dir_all(parent)?;
    }

    if file_name_str.ends_with(".tera") {
      let output_name = file_name_str.trim_end_matches(".tera");
      let output_path = output_path.with_file_name(output_name);

      let template_content = std::str::from_utf8(&file.contents)?;
      tera.add_raw_template(&template_key, template_content)?;
      let rendered = tera.render(&template_key, context)?;

      fs::write(&output_path, rendered)?;
      println!("  ✓ {}", output_path.display());
    } else {
      fs::write(&output_path, &file.contents)?;
      println!("  ✓ {}", output_path.display());
    }
  }
  Ok(())
}
