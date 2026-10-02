use std::{fs, path::Path};

use anyhow::Result;

use crate::addons::runner::apply_rollback;
use crate::context::{CleanupState, CleanupTask};
use crate::utils::ui;

pub fn setup_ctrlc_handler(cleanup_state: CleanupState) -> Result<()> {
  ctrlc::set_handler(move || {
    println!();
    ui::warn("Interrupted! Cleaning up...");

    let task = {
      let mut guard = cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
      guard.take()
    };

    if let Some(task) = task {
      run_cleanup(&task);
    }

    std::process::exit(130);
  })?;

  Ok(())
}

pub fn run_cleanup(task: &CleanupTask) {
  match task {
    CleanupTask::PartialDownload {
      path,
      prune_root,
      label,
    } => {
      if !path.exists() {
        return;
      }
      let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
      let canonical_root = fs::canonicalize(prune_root).unwrap_or_else(|_| prune_root.clone());
      if !canonical_path.starts_with(&canonical_root) {
        eprintln!(
          "Refusing to remove {}: it is outside {}",
          path.display(),
          prune_root.display()
        );
        return;
      }
      if let Err(e) = fs::remove_dir_all(path) {
        eprintln!("Failed to remove {}: {e}", path.display());
        return;
      }
      prune_empty_parents(path, prune_root);
      ui::success(format!("Removed incomplete {label}"));
    }

    CleanupTask::PartialProject { path } => {
      if !path.exists() {
        return;
      }
      if let Err(e) = fs::remove_dir_all(path) {
        eprintln!("Failed to remove {}: {e}", path.display());
        return;
      }
      ui::success(format!("Removed incomplete project {}", path.display()));
    }

    CleanupTask::PartialProjectFiles { paths } => {
      let removed = paths
        .iter()
        .filter(|p| p.exists() && fs::remove_file(p).is_ok())
        .count();
      if removed > 0 {
        ui::success(format!(
          "Removed {removed} newly-created file(s) from the interrupted generation"
        ));
      }
    }

    CleanupTask::PartialAddon {
      addon_id,
      project_root,
      journal,
    } => {
      let steps = {
        let mut guard = journal.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *guard)
      };

      if steps.is_empty() {
        return;
      }

      let count = steps.len();
      for rollback in steps.into_iter().rev() {
        let _ = apply_rollback(rollback, project_root);
      }
      ui::success(format!(
        "Rolled back {count} step(s) from addon '{addon_id}'"
      ));
    }
  }
}

fn prune_empty_parents(path: &Path, prune_root: &Path) {
  let mut current = path.parent();
  while let Some(parent) = current {
    if parent == prune_root || !parent.starts_with(prune_root) {
      break;
    }
    if fs::remove_dir(parent).is_err() {
      break;
    }
    current = parent.parent();
  }
}

#[doc(hidden)]
pub fn prune_empty_parents_for_tests(path: &Path, prune_root: &Path) {
  prune_empty_parents(path, prune_root);
}
