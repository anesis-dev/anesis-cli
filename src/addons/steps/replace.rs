use std::path::Path;

use anyhow::{anyhow, Result};
use inquire::Select;

use crate::addons::manifest::{IfNotFound, ReplaceStep};

use super::{Rollback, render_lines, resolve_target};

pub fn execute_replace(
  step: &ReplaceStep,
  project_root: &Path,
  ctx: &tera::Context,
) -> Result<Vec<Rollback>> {
  let paths = resolve_target(&step.target, project_root)?;
  let replace_lines: Vec<String> = step.replace.lines().map(str::to_string).collect();
  let rendered_replace = render_lines(&replace_lines, ctx)?.join("\n");

  let mut rollbacks = Vec::new();

  for path in paths {
    let original = std::fs::read(&path)?;
    let content = String::from_utf8_lossy(&original).to_string();

    if !content.contains(&step.find) {
      match step.if_not_found {
        IfNotFound::Skip => continue,
        IfNotFound::Error => {
          return Err(anyhow!("Pattern {:?} not found in {}", step.find, path.display()));
        }
        IfNotFound::WarnAndAsk => {
          eprintln!("Warning: {:?} not found in {}", step.find, path.display());
          let choice =
            Select::new("How would you like to proceed?", vec!["Continue", "Abort"]).prompt()?;
          if choice == "Abort" {
            return Err(anyhow!("Aborted by user"));
          }
          continue;
        }
      }
    }

    let new_content = content.replace(&step.find, &rendered_replace);
    rollbacks.push(Rollback::RestoreFile { path: path.clone(), original });
    std::fs::write(&path, new_content)?;
  }

  Ok(rollbacks)
}
