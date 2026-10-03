use std::{
  fs,
  path::Path,
  sync::atomic::{AtomicBool, Ordering},
  time::Duration,
};

use anyhow::Result;

use crate::addons::runner::apply_rollback;
use crate::context::{CleanupState, CleanupTask};
use crate::utils::{errors::AnesisError, ui};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

const MAIN_THREAD_GRACE: Duration = Duration::from_secs(2);

pub fn interrupted() -> bool {
  INTERRUPTED.load(Ordering::SeqCst)
}

pub fn check_interrupted() -> Result<()> {
  if interrupted() {
    return Err(AnesisError::Interrupted.into());
  }
  Ok(())
}

pub fn setup_ctrlc_handler(cleanup_state: CleanupState) -> Result<()> {
  ctrlc::set_handler(move || {
    if INTERRUPTED.swap(true, Ordering::SeqCst) {
      return;
    }
    eprintln!();
    ui::warn("Interrupted! Cleaning up...");

    std::thread::sleep(MAIN_THREAD_GRACE);

    let task = {
      let mut guard = cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
      guard.take()
    };

    if let Some(task) = task {
      run_cleanup(&task);
    }

    std::process::exit(crate::utils::errors::exit_code::INTERRUPTED);
  })?;

  Ok(())
}

#[doc(hidden)]
pub fn set_interrupted_for_tests(value: bool) {
  INTERRUPTED.store(value, Ordering::SeqCst);
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
      let mut failed = 0usize;
      for rollback in steps.into_iter().rev() {
        if let Err(err) = apply_rollback(rollback, project_root) {
          failed += 1;
          ui::failure(format!("{err:#}"));
        }
      }
      if failed == 0 {
        ui::success(format!(
          "Rolled back {count} step(s) from addon '{addon_id}'"
        ));
      } else {
        ui::warn_err(format!(
          "Rolled back {} of {count} step(s) from addon '{addon_id}'; the rest could not be reverted",
          count - failed
        ));
      }
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
