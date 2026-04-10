use std::{fs, path::Path};

use anyhow::Result;
use serde::{Deserialize, Serialize};

const LOCK_FILE_NAME: &str = "oxide.lock";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LockFile {
  pub addons: Vec<LockEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LockEntry {
  pub id: String,
  pub version: String,
  pub variant: String,
  pub commands_executed: Vec<String>,
}

impl LockFile {
  pub fn load(project_root: &Path) -> Result<Self> {
    let path = project_root.join(LOCK_FILE_NAME);
    if !path.exists() {
      return Ok(Self::default());
    }
    let contents = fs::read_to_string(&path)?;
    let lock: Self = serde_json::from_str(&contents)?;
    Ok(lock)
  }

  pub fn save(&self, project_root: &Path) -> Result<()> {
    let path = project_root.join(LOCK_FILE_NAME);
    let contents = serde_json::to_string_pretty(self)?;
    fs::write(path, contents)?;
    Ok(())
  }

  pub fn is_command_executed(&self, addon_id: &str, command: &str) -> bool {
    self
      .addons
      .iter()
      .find(|e| e.id == addon_id)
      .map(|e| e.commands_executed.iter().any(|c| c == command))
      .unwrap_or(false)
  }

  pub fn mark_command_executed(&mut self, addon_id: &str, command: &str) {
    if let Some(entry) = self.addons.iter_mut().find(|e| e.id == addon_id)
      && !entry.commands_executed.iter().any(|c| c == command)
    {
      entry.commands_executed.push(command.to_string());
    }
  }

  pub fn upsert_entry(&mut self, entry: LockEntry) {
    if let Some(existing) = self.addons.iter_mut().find(|e| e.id == entry.id) {
      *existing = entry;
    } else {
      self.addons.push(entry);
    }
  }
}
