use std::{
  collections::HashMap,
  fs,
  path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::steps::Rollback;

const LOCK_FILE_NAME: &str = "anesis.lock";
const CURRENT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Default)]
pub struct LockFile {
  pub addons: Vec<LockEntry>,
}

#[derive(Debug, Clone)]
pub struct LockEntry {
  pub id: String,
  pub version: String,
  pub variant: String,
  pub inputs: HashMap<String, String>,
  pub commands: Vec<CommandRun>,
}

#[derive(Debug, Clone)]
pub struct CommandRun {
  pub name: String,
  pub inputs: HashMap<String, String>,
  pub journal: Vec<Rollback>,
}

impl LockEntry {
  pub fn new(
    id: impl Into<String>,
    version: impl Into<String>,
    variant: impl Into<String>,
  ) -> Self {
    Self {
      id: id.into(),
      version: version.into(),
      variant: variant.into(),
      inputs: HashMap::new(),
      commands: Vec::new(),
    }
  }

  pub fn commands_executed(&self) -> Vec<String> {
    self.commands.iter().map(|c| c.name.clone()).collect()
  }

  pub fn has_undoable_changes(&self) -> bool {
    self.commands.iter().any(|c| !c.journal.is_empty())
  }

  /// Replaces the tracked inputs/journal for `name` with the latest run's data
  /// (rather than accumulating across reruns), so a later command with an
  /// unrelated input of the same name can never corrupt this command's replay.
  pub fn upsert_command(
    &mut self,
    name: &str,
    inputs: HashMap<String, String>,
    journal: Vec<Rollback>,
  ) {
    if let Some(existing) = self.commands.iter_mut().find(|c| c.name == name) {
      existing.inputs = inputs;
      existing.journal = journal;
    } else {
      self.commands.push(CommandRun {
        name: name.to_string(),
        inputs,
        journal,
      });
    }
  }
}

impl LockFile {
  pub fn load(project_root: &Path) -> Result<Self> {
    let path = project_root.join(LOCK_FILE_NAME);
    if !path.exists() {
      return Ok(Self::default());
    }
    let contents = fs::read_to_string(&path)?;
    let raw: serde_json::Value = serde_json::from_str(&contents)?;
    let schema_version = raw
      .get("schema_version")
      .and_then(|v| v.as_u64())
      .unwrap_or(1);
    let canon_root = project_root.canonicalize().ok();

    let mut lock = if schema_version >= 2 {
      let wire: WireLockFile = serde_json::from_value(raw)?;
      from_wire(wire, canon_root.as_deref())
    } else {
      let legacy: LegacyLockFile = serde_json::from_value(raw)?;
      migrate_v1(legacy, canon_root.as_deref())
    };
    lock.sanitize_journal_paths(project_root);
    Ok(lock)
  }

  fn sanitize_journal_paths(&mut self, project_root: &Path) {
    let Ok(canon_root) = project_root.canonicalize() else {
      return;
    };

    let is_inside = |path: &Path| -> bool {
      resolve_for_containment(path).is_some_and(|resolved| resolved.starts_with(&canon_root))
    };

    for entry in &mut self.addons {
      let addon_id = entry.id.clone();
      for cmd in &mut entry.commands {
        cmd.journal.retain(|rollback| {
          let (keep, offending) = match rollback {
            Rollback::DeleteCreatedFile { path } => (is_inside(path), path.display().to_string()),
            Rollback::RestoreFile { path, .. } => (is_inside(path), path.display().to_string()),
            Rollback::RenameFile { from, to } => (
              is_inside(from) && is_inside(to),
              format!("{} -> {}", from.display(), to.display()),
            ),
            Rollback::IrreversibleRun { .. } => (true, String::new()),
          };
          if !keep {
            eprintln!(
              "Warning: anesis.lock for '{addon_id}' references a rollback path outside the project root ('{offending}') \
               -- dropping this rollback entry so the addon can still be loaded."
            );
          }
          keep
        });
      }
    }
  }

  pub fn save(&self, project_root: &Path) -> Result<()> {
    let path = project_root.join(LOCK_FILE_NAME);
    let canon_root = project_root
      .canonicalize()
      .unwrap_or_else(|_| project_root.to_path_buf());
    let wire = to_wire(self, &canon_root);
    let contents = serde_json::to_string_pretty(&wire)?;
    crate::utils::atomic::write_atomic(&path, contents.as_bytes())?;
    Ok(())
  }

  pub fn is_command_executed(&self, addon_id: &str, command: &str) -> bool {
    self
      .addons
      .iter()
      .find(|e| e.id == addon_id)
      .map(|e| e.commands.iter().any(|c| c.name == command))
      .unwrap_or(false)
  }

  pub fn addon_version(&self, addon_id: &str) -> Option<&str> {
    self
      .addons
      .iter()
      .find(|e| e.id == addon_id)
      .map(|entry| entry.version.as_str())
  }

  pub fn remove_addon(&mut self, addon_id: &str) {
    self.addons.retain(|e| e.id != addon_id);
  }

  pub fn upsert_entry(&mut self, entry: LockEntry) {
    if let Some(existing) = self.addons.iter_mut().find(|e| e.id == entry.id) {
      *existing = entry;
    } else {
      self.addons.push(entry);
    }
  }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct WireLockFile {
  schema_version: u32,
  #[serde(default)]
  addons: Vec<WireLockEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WireLockEntry {
  id: String,
  version: String,
  variant: String,
  #[serde(default, skip_serializing_if = "HashMap::is_empty")]
  inputs: HashMap<String, String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  commands: Vec<WireCommandRun>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WireCommandRun {
  name: String,
  #[serde(default, skip_serializing_if = "HashMap::is_empty")]
  inputs: HashMap<String, String>,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  journal: Vec<WireRollback>,
}

#[derive(Debug, Serialize, Deserialize)]
enum WireRollback {
  DeleteCreatedFile {
    path: String,
  },
  RestoreFile {
    path: String,
    original: Vec<u8>,
    #[serde(default)]
    mode: Option<u32>,
    #[serde(default)]
    is_symlink: bool,
  },
  RenameFile {
    from: String,
    to: String,
  },
  IrreversibleRun {
    command: String,
  },
}

fn resolve_for_containment(path: &Path) -> Option<PathBuf> {
  let components: Vec<_> = path.components().collect();
  for split in (1..=components.len()).rev() {
    let prefix: PathBuf = components[..split].iter().collect();
    match prefix.canonicalize() {
      Ok(real) => {
        let joined = components[split..]
          .iter()
          .fold(real, |acc, component| acc.join(component));
        return Some(lexically_normalize(&joined));
      }
      Err(_) if prefix.symlink_metadata().is_ok() => return None,
      Err(_) => {}
    }
  }
  Some(lexically_normalize(path))
}

pub(crate) fn lexically_normalize(p: &Path) -> PathBuf {
  let mut out = PathBuf::new();
  for component in p.components() {
    match component {
      std::path::Component::ParentDir => {
        out.pop();
      }
      std::path::Component::CurDir => {}
      c => out.push(c),
    }
  }
  out
}

fn relativize(root: &Path, p: &Path) -> String {
  p.strip_prefix(root)
    .unwrap_or(p)
    .to_string_lossy()
    .into_owned()
}

fn absolutize(root: &Path, s: &str) -> PathBuf {
  root.join(s)
}

fn rollback_to_wire(root: &Path, r: &Rollback) -> WireRollback {
  match r {
    Rollback::DeleteCreatedFile { path } => WireRollback::DeleteCreatedFile {
      path: relativize(root, path),
    },
    Rollback::RestoreFile {
      path,
      original,
      mode,
      is_symlink,
    } => WireRollback::RestoreFile {
      path: relativize(root, path),
      original: original.clone(),
      mode: *mode,
      is_symlink: *is_symlink,
    },
    Rollback::RenameFile { from, to } => WireRollback::RenameFile {
      from: relativize(root, from),
      to: relativize(root, to),
    },
    Rollback::IrreversibleRun { command } => WireRollback::IrreversibleRun {
      command: command.clone(),
    },
  }
}

fn rollback_from_wire(root: &Path, r: WireRollback) -> Rollback {
  match r {
    WireRollback::DeleteCreatedFile { path } => Rollback::DeleteCreatedFile {
      path: absolutize(root, &path),
    },
    WireRollback::RestoreFile {
      path,
      original,
      mode,
      is_symlink,
    } => Rollback::RestoreFile {
      path: absolutize(root, &path),
      original,
      mode,
      is_symlink,
    },
    WireRollback::RenameFile { from, to } => Rollback::RenameFile {
      from: absolutize(root, &from),
      to: absolutize(root, &to),
    },
    WireRollback::IrreversibleRun { command } => Rollback::IrreversibleRun { command },
  }
}

fn to_wire(lock: &LockFile, canon_root: &Path) -> WireLockFile {
  WireLockFile {
    schema_version: CURRENT_SCHEMA_VERSION,
    addons: lock
      .addons
      .iter()
      .map(|e| WireLockEntry {
        id: e.id.clone(),
        version: e.version.clone(),
        variant: e.variant.clone(),
        inputs: e.inputs.clone(),
        commands: e
          .commands
          .iter()
          .map(|c| WireCommandRun {
            name: c.name.clone(),
            inputs: c.inputs.clone(),
            journal: c
              .journal
              .iter()
              .map(|r| rollback_to_wire(canon_root, r))
              .collect(),
          })
          .collect(),
      })
      .collect(),
  }
}

fn from_wire(wire: WireLockFile, canon_root: Option<&Path>) -> LockFile {
  let root = canon_root.unwrap_or_else(|| Path::new(""));
  LockFile {
    addons: wire
      .addons
      .into_iter()
      .map(|e| LockEntry {
        id: e.id,
        version: e.version,
        variant: e.variant,
        inputs: e.inputs,
        commands: e
          .commands
          .into_iter()
          .map(|c| CommandRun {
            name: c.name,
            inputs: c.inputs,
            journal: c
              .journal
              .into_iter()
              .map(|r| rollback_from_wire(root, r))
              .collect(),
          })
          .collect(),
      })
      .collect(),
  }
}

#[derive(Debug, Default, Deserialize)]
struct LegacyLockFile {
  #[serde(default)]
  addons: Vec<LegacyLockEntry>,
}

#[derive(Debug, Deserialize)]
struct LegacyLockEntry {
  id: String,
  version: String,
  variant: String,
  #[serde(default)]
  commands_executed: Vec<String>,
  #[serde(default)]
  journal: Vec<Rollback>,
  #[serde(default)]
  inputs: HashMap<String, String>,
}

fn migrate_v1(legacy: LegacyLockFile, _canon_root: Option<&Path>) -> LockFile {
  LockFile {
    addons: legacy
      .addons
      .into_iter()
      .map(|e| {
        let mut commands: Vec<CommandRun> = e
          .commands_executed
          .iter()
          .map(|name| CommandRun {
            name: name.clone(),
            inputs: e.inputs.clone(),
            journal: Vec::new(),
          })
          .collect();
        if let Some(first) = commands.first_mut() {
          first.journal = e.journal;
        } else if !e.journal.is_empty() {
          commands.push(CommandRun {
            name: "(unknown)".to_string(),
            inputs: e.inputs.clone(),
            journal: e.journal,
          });
        }
        LockEntry {
          id: e.id,
          version: e.version,
          variant: e.variant,
          inputs: e.inputs,
          commands,
        }
      })
      .collect(),
  }
}
