use std::{
  collections::HashMap,
  path::{Path, PathBuf},
};

use anyhow::Result;
use serde::Serialize;

use crate::{
  addons::{
    lock::{LockFile, lexically_normalize},
    steps::Rollback,
  },
  utils::ui,
};

#[derive(Debug, Clone, Serialize)]
pub struct WhyEntry {
  pub addon_id: String,
  pub version: String,
  pub command: String,
  pub kind: String,
}

fn kind_of(rollback: &Rollback) -> Option<(PathBuf, &'static str)> {
  match rollback {
    Rollback::DeleteCreatedFile { path } => Some((path.clone(), "created")),
    Rollback::RestoreFile { path, .. } => Some((path.clone(), "modified")),
    Rollback::RenameFile { to, .. } => Some((to.clone(), "renamed")),
    Rollback::IrreversibleRun { .. } => None,
  }
}

fn kind_label(kind: &str) -> &'static str {
  match kind {
    "created" => "created by ",
    "modified" => "modified by",
    "renamed" => "renamed by ",
    _ => "changed by ",
  }
}

pub fn build_index(lock: &LockFile) -> HashMap<PathBuf, Vec<WhyEntry>> {
  let mut index: HashMap<PathBuf, Vec<WhyEntry>> = HashMap::new();
  for entry in &lock.addons {
    for cmd in &entry.commands {
      for rollback in &cmd.journal {
        let Some((path, kind)) = kind_of(rollback) else {
          continue;
        };
        index
          .entry(lexically_normalize(&path))
          .or_default()
          .push(WhyEntry {
            addon_id: entry.id.clone(),
            version: entry.version.clone(),
            command: cmd.name.clone(),
            kind: kind.to_string(),
          });
      }
    }
  }
  index
}

fn resolve_query(project_root: &Path, query: &str) -> PathBuf {
  let raw = PathBuf::from(query);
  let absolute = if raw.is_absolute() {
    raw
  } else {
    project_root.join(raw)
  };
  lexically_normalize(&absolute)
}

fn display_path<'a>(project_root: &Path, path: &'a Path) -> std::borrow::Cow<'a, str> {
  path
    .strip_prefix(project_root)
    .unwrap_or(path)
    .to_string_lossy()
}

pub fn why(project_root: &Path, query: Option<&str>, json: bool) -> Result<()> {
  let lock = LockFile::load(project_root)?;
  let index = build_index(&lock);

  if let Some(q) = query {
    let key = resolve_query(project_root, q);
    let entries = index.get(&key).cloned().unwrap_or_default();

    if json {
      println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({ "path": q, "entries": entries }))?
      );
      return Ok(());
    }

    println!("{q}");
    if entries.is_empty() {
      println!(
        "  {}",
        ui::muted("not created by any addon (probably from the template)")
      );
    } else {
      for e in &entries {
        print_entry(e);
      }
    }
    return Ok(());
  }

  if json {
    let all: Vec<serde_json::Value> = index
      .iter()
      .map(|(path, entries)| {
        serde_json::json!({
          "path": display_path(project_root, path),
          "entries": entries,
        })
      })
      .collect();
    println!("{}", serde_json::to_string_pretty(&all)?);
    return Ok(());
  }

  let mut paths: Vec<&PathBuf> = index.keys().collect();
  paths.sort();
  if paths.is_empty() {
    println!("No files tracked in anesis.lock yet.");
    return Ok(());
  }
  for path in paths {
    println!("{}", display_path(project_root, path));
    for e in &index[path] {
      print_entry(e);
    }
  }
  Ok(())
}

fn print_entry(e: &WhyEntry) {
  println!(
    "  {}  {} v{}  ·  command: {}",
    kind_label(&e.kind),
    ui::accent(&e.addon_id),
    e.version,
    e.command
  );
}
