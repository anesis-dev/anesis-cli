use crate::utils::template_engine::TemplateContext;
use std::path::Path;

use anyhow::anyhow;

use crate::addons::manifest::MoveStep;

use super::{Rollback, StepFailure, StepResult};

pub fn execute_move(step: &MoveStep, project_root: &Path, ctx: &TemplateContext) -> StepResult {
  let rendered_from = super::render_string(&step.from, ctx)?;
  let rendered_to = super::render_string(&step.to, ctx)?;
  let from = super::safe_join(project_root, &rendered_from, "move source")?;
  let to = super::safe_join(project_root, &rendered_to, "move destination")?;

  if !from.exists() {
    return Err(StepFailure::without_rollbacks(anyhow!(
      "{} does not exist",
      from.display()
    )));
  }
  if to.exists() {
    return Err(StepFailure::without_rollbacks(anyhow!(
      "{} already exists",
      to.display()
    )));
  }

  if let Some(parent) = to.parent() {
    std::fs::create_dir_all(parent).map_err(StepFailure::without_rollbacks)?;
  }
  std::fs::rename(&from, &to).map_err(StepFailure::without_rollbacks)?;

  Ok(vec![Rollback::RenameFile { from: to, to: from }])
}
