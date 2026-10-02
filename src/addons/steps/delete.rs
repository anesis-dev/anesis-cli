use crate::utils::template_engine::TemplateContext;
use std::path::Path;

use crate::addons::manifest::DeleteStep;

use super::{Rollback, StepFailure, StepResult, resolve_target};

pub fn execute_delete(step: &DeleteStep, project_root: &Path, ctx: &TemplateContext) -> StepResult {
  let paths = resolve_target(&step.target, project_root, ctx)?;
  let mut rollbacks = Vec::new();

  for path in paths {
    let meta = match path.symlink_metadata() {
      Ok(m) => m,
      Err(_) => {
        eprintln!(
          "Warning: delete target '{}' does not exist; skipping",
          path.display()
        );
        continue;
      }
    };
    if meta.is_dir() {
      eprintln!(
        "Warning: skipping '{}': deleting directories is not supported",
        path.display()
      );
      continue;
    }

    let is_symlink = meta.file_type().is_symlink();
    let (original, mode) = if is_symlink {
      let target = match std::fs::read_link(&path) {
        Ok(t) => t,
        Err(e) => return Err(StepFailure::new(e, rollbacks)),
      };
      (target.to_string_lossy().into_owned().into_bytes(), None)
    } else {
      let bytes = match std::fs::read(&path) {
        Ok(o) => o,
        Err(e) => return Err(StepFailure::new(e, rollbacks)),
      };
      #[cfg(unix)]
      let mode = {
        use std::os::unix::fs::PermissionsExt;
        Some(meta.permissions().mode() & 0o777)
      };
      #[cfg(not(unix))]
      let mode = None;
      (bytes, mode)
    };

    rollbacks.push(Rollback::RestoreFile {
      path: path.clone(),
      original,
      mode,
      is_symlink,
    });
    if let Err(e) = std::fs::remove_file(&path) {
      return Err(StepFailure::new(e, rollbacks));
    }
  }

  Ok(rollbacks)
}
