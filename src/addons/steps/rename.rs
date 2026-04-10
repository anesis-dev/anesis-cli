use std::path::Path;

use anyhow::{anyhow, Result};

use crate::addons::manifest::RenameStep;

use super::Rollback;

pub fn execute_rename(step: &RenameStep, project_root: &Path) -> Result<Vec<Rollback>> {
  let from = super::safe_join(project_root, &step.from, "rename source")?;
  let to = super::safe_join(project_root, &step.to, "rename destination")?;

  if !from.exists() {
    return Err(anyhow!("{} does not exist", from.display()));
  }
  if to.exists() {
    return Err(anyhow!("{} already exists", to.display()));
  }

  std::fs::rename(&from, &to)?;

  Ok(vec![Rollback::RenameFile { from: to, to: from }])
}

