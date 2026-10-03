use crate::utils::template_engine::TemplateContext;
use std::path::Path;

use inquire::Confirm;

use crate::addons::manifest::{CreateStep, IfExists};

use super::{Rollback, StepFailure, StepResult};

pub fn execute_create(
  step: &CreateStep,
  project_root: &Path,
  ctx: &TemplateContext,
  non_interactive: bool,
) -> StepResult {
  let rendered_path = super::render_string(&step.path, ctx)?;
  let path = super::safe_join(project_root, &rendered_path, "create path")?;
  let mut content = super::render_string(&step.content, ctx)?;
  if !content.is_empty() && !content.ends_with('\n') {
    content.push('\n');
  }

  let mut rollbacks = Vec::new();

  if path.exists() {
    match step.if_exists {
      IfExists::Skip => return Ok(rollbacks),
      IfExists::Ask => {
        if non_interactive {
          eprintln!("  {rendered_path} already exists — keeping it (pass no --yes to be asked)");
          return Ok(rollbacks);
        }
        let overwrite = Confirm::new(&format!("{rendered_path} already exists. Overwrite?"))
          .with_default(false)
          .prompt()
          .map_err(StepFailure::without_rollbacks)?;
        if !overwrite {
          return Ok(rollbacks);
        }
        let original = std::fs::read(&path).map_err(StepFailure::without_rollbacks)?;
        rollbacks.push(Rollback::restore_file(path.clone(), original));
      }
      IfExists::Overwrite => {
        let original = std::fs::read(&path).map_err(StepFailure::without_rollbacks)?;
        rollbacks.push(Rollback::restore_file(path.clone(), original));
      }
    }
  } else {
    rollbacks.push(Rollback::DeleteCreatedFile { path: path.clone() });
  }

  if let Some(parent) = path.parent()
    && let Err(e) = std::fs::create_dir_all(parent)
  {
    return Err(StepFailure::new(e, rollbacks));
  }
  if let Err(e) = crate::utils::atomic::write_file_atomic(&path, content.as_bytes()) {
    return Err(StepFailure::new(e, rollbacks));
  }

  Ok(rollbacks)
}
