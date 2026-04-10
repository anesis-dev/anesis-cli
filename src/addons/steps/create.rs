use std::path::Path;

use anyhow::Result;
use inquire::Confirm;

use crate::addons::manifest::{CreateStep, IfExists};

use super::{Rollback, render_lines};

pub fn execute_create(
  step: &CreateStep,
  project_root: &Path,
  ctx: &tera::Context,
) -> Result<Vec<Rollback>> {
  let path = super::safe_join(project_root, &step.path, "create path")?;
  let lines: Vec<String> = step.content.lines().map(str::to_string).collect();
  let content = render_lines(&lines, ctx)?.join("\n");

  let mut rollbacks = Vec::new();

  if path.exists() {
    match step.if_exists {
      IfExists::Skip => return Ok(rollbacks),
      IfExists::Ask => {
        let overwrite = Confirm::new(&format!("{} already exists. Overwrite?", step.path))
          .with_default(false)
          .prompt()?;
        if !overwrite {
          return Ok(rollbacks);
        }
        rollbacks.push(Rollback::RestoreFile { path: path.clone(), original: std::fs::read(&path)? });
      }
      IfExists::Overwrite => {
        rollbacks.push(Rollback::RestoreFile { path: path.clone(), original: std::fs::read(&path)? });
      }
    }
  } else {
    rollbacks.push(Rollback::DeleteCreatedFile { path: path.clone() });
  }

  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)?;
  }
  std::fs::write(&path, content)?;

  Ok(rollbacks)
}
