use std::path::Path;

use anyhow::{anyhow, Result};

use crate::addons::manifest::MoveStep;

use super::Rollback;

pub fn execute_move(step: &MoveStep, project_root: &Path) -> Result<Vec<Rollback>> {
  let from = super::safe_join(project_root, &step.from, "move source")?;
  let to = super::safe_join(project_root, &step.to, "move destination")?;

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

