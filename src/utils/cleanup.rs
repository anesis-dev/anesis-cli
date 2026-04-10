use std::{fs, path::PathBuf};

use anyhow::Result;

use crate::CleanupState;

pub fn setup_ctrlc_handler(
  cleanup_state: CleanupState,
  template_path_clone: PathBuf,
) -> Result<()> {
  ctrlc::set_handler(move || {
    println!("\n⚠ Interrupted! Cleaning up...");

    let cleanup_path = {
      let guard = cleanup_state.lock().unwrap_or_else(|e| e.into_inner());

      guard.clone()
    };
    if let Some(path) = cleanup_path
      && path.exists()
    {
      if let Err(e) = fs::remove_dir_all(&path) {
        println!("Failed to remove: {}", e);
      }

      let mut current = path.parent();
      while let Some(parent) = current {
        if parent == template_path_clone {
          break;
        }
        if fs::remove_dir(parent).is_err() {
          break;
        }
        current = parent.parent();
      }
      println!("✓ Removed incomplete template");
    }
    std::process::exit(1);
  })?;

  Ok(())
}
