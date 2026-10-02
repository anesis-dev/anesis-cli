use crate::utils::template_engine::TemplateContext;
use std::path::Path;

use anyhow::Context;
use inquire::Confirm;

use crate::addons::manifest::{CopyStep, IfExists};

use super::{Rollback, StepFailure, StepResult};

pub fn execute_copy(
  step: &CopyStep,
  addon_dir: &Path,
  project_root: &Path,
  ctx: &TemplateContext,
  non_interactive: bool,
) -> StepResult {
  let rendered_src = super::render_string(&step.src, ctx)?;
  let rendered_dest = super::render_string(&step.dest, ctx)?;
  let src = super::safe_join(addon_dir, &rendered_src, "addon source")?;
  let dest = super::safe_join(project_root, &rendered_dest, "copy destination")?;

  let mut rollbacks = Vec::new();

  if dest.exists() {
    match step.if_exists {
      IfExists::Skip => return Ok(rollbacks),
      IfExists::Ask => {
        if non_interactive {
          println!("  {rendered_dest} already exists — keeping it (pass no --yes to be asked)");
          return Ok(rollbacks);
        }
        let overwrite = Confirm::new(&format!("{} already exists. Overwrite?", rendered_dest))
          .with_default(false)
          .prompt()
          .map_err(StepFailure::without_rollbacks)?;
        if !overwrite {
          return Ok(rollbacks);
        }
        let original = std::fs::read(&dest).map_err(StepFailure::without_rollbacks)?;
        rollbacks.push(Rollback::restore_file(dest.clone(), original));
      }
      IfExists::Overwrite => {
        let original = std::fs::read(&dest).map_err(StepFailure::without_rollbacks)?;
        rollbacks.push(Rollback::restore_file(dest.clone(), original));
      }
    }
  } else {
    rollbacks.push(Rollback::DeleteCreatedFile { path: dest.clone() });
  }

  if let Some(parent) = dest.parent()
    && let Err(e) = std::fs::create_dir_all(parent)
  {
    return Err(StepFailure::new(e, rollbacks));
  }

  if step.render {
    let bytes = std::fs::read(&src)
      .with_context(|| format!("Failed to read addon source {}", src.display()))
      .map_err(|e| StepFailure::new(e, rollbacks.clone()))?;
    if let Ok(text) = std::str::from_utf8(&bytes) {
      let rendered = render_string_or_fail(text, ctx, &rollbacks)?;
      std::fs::write(&dest, rendered)
        .with_context(|| format!("Failed to write {}", dest.display()))
        .map_err(|e| StepFailure::new(e, rollbacks.clone()))?;
      return Ok(rollbacks);
    }
  }

  let bytes = std::fs::read(&src)
    .with_context(|| format!("Failed to read addon source {}", src.display()))
    .map_err(|e| StepFailure::new(e, rollbacks.clone()))?;
  std::fs::write(&dest, bytes)
    .with_context(|| format!("Failed to write {}", dest.display()))
    .map_err(|e| StepFailure::new(e, rollbacks.clone()))?;

  Ok(rollbacks)
}

fn render_string_or_fail(
  text: &str,
  ctx: &TemplateContext,
  rollbacks: &[Rollback],
) -> Result<String, StepFailure> {
  super::render_string(text, ctx).map_err(|e| StepFailure::new(e, rollbacks.to_vec()))
}
