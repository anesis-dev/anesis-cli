use std::path::Path;

use anyhow::{anyhow, Result};

use crate::addons::manifest::MoveStep;

use super::Rollback;

pub fn execute_move(
  step: &MoveStep,
  project_root: &Path,
  ctx: &tera::Context,
) -> Result<Vec<Rollback>> {
  let rendered_from = super::render_string(&step.from, ctx)?;
  let rendered_to = super::render_string(&step.to, ctx)?;
  let from = super::safe_join(project_root, &rendered_from, "move source")?;
  let to = super::safe_join(project_root, &rendered_to, "move destination")?;

  if !from.exists() {
    return Err(anyhow!("{} does not exist", from.display()));
  }
  if to.exists() {
    return Err(anyhow!("{} already exists", to.display()));
  }

  if let Some(parent) = to.parent() {
    std::fs::create_dir_all(parent)?;
  }
  std::fs::rename(&from, &to)?;

  Ok(vec![Rollback::RenameFile { from: to, to: from }])
}

