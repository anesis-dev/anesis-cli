use std::path::Path;

use anyhow::Result;

use crate::addons::manifest::AppendStep;

use super::{Rollback, render_lines, resolve_target};

pub fn execute_append(
  step: &AppendStep,
  project_root: &Path,
  ctx: &tera::Context,
) -> Result<Vec<Rollback>> {
  let paths = resolve_target(&step.target, project_root)?;
  let lines: Vec<String> = step.content.lines().map(str::to_string).collect();
  let rendered = render_lines(&lines, ctx)?.join("\n");

  let mut rollbacks = Vec::new();

  for path in paths {
    let original = std::fs::read(&path)?;
    let mut new_content = String::from_utf8_lossy(&original).to_string();

    if !new_content.is_empty() && !new_content.ends_with('\n') {
      new_content.push('\n');
    }
    new_content.push_str(&rendered);

    rollbacks.push(Rollback::RestoreFile { path: path.clone(), original });
    std::fs::write(&path, new_content)?;
  }

  Ok(rollbacks)
}
