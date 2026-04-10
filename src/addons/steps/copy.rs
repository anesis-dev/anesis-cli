use std::path::Path;

use anyhow::{Context, Result};
use inquire::Confirm;

use crate::addons::manifest::{CopyStep, IfExists};

use super::Rollback;

pub fn execute_copy(step: &CopyStep, addon_dir: &Path, project_root: &Path) -> Result<Vec<Rollback>> {
  let src = super::safe_join(addon_dir, &step.src, "addon source")?;
  let dest = super::safe_join(project_root, &step.dest, "copy destination")?;

  let mut rollbacks = Vec::new();

  if dest.exists() {
    match step.if_exists {
      IfExists::Skip => return Ok(rollbacks),
      IfExists::Ask => {
        let overwrite = Confirm::new(&format!("{} already exists. Overwrite?", step.dest))
          .with_default(false)
          .prompt()?;
        if !overwrite {
          return Ok(rollbacks);
        }
        rollbacks.push(Rollback::RestoreFile { path: dest.clone(), original: std::fs::read(&dest)? });
      }
      IfExists::Overwrite => {
        rollbacks.push(Rollback::RestoreFile { path: dest.clone(), original: std::fs::read(&dest)? });
      }
    }
  } else {
    rollbacks.push(Rollback::DeleteCreatedFile { path: dest.clone() });
  }

  if let Some(parent) = dest.parent() {
    std::fs::create_dir_all(parent)?;
  }
  std::fs::copy(&src, &dest)
    .with_context(|| format!("Failed to copy {} to {}", src.display(), dest.display()))?;

  Ok(rollbacks)
}
