use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};

pub mod append;
pub mod copy;
pub mod create;
pub mod delete;
pub mod inject;
pub mod move_step;
pub mod rename;
pub mod replace;

pub enum Rollback {
  DeleteCreatedFile { path: PathBuf },
  RestoreFile { path: PathBuf, original: Vec<u8> },
  RenameFile { from: PathBuf, to: PathBuf },
}

/// Renders content lines with Tera one_off — substitutes {{ var }} from user inputs.
pub fn render_lines(lines: &[String], ctx: &tera::Context) -> Result<Vec<String>> {
  lines
    .iter()
    .map(|line| tera::Tera::one_off(line, ctx, false).map_err(Into::into))
    .collect()
}

/// Normalises `root.join(relative)` without touching the filesystem by
/// resolving `.` and `..` components lexically.
fn normalize_join(root: &Path, relative: &str) -> PathBuf {
  let joined = root.join(relative);
  let mut out = PathBuf::new();
  for component in joined.components() {
    match component {
      Component::ParentDir => {
        out.pop();
      }
      Component::CurDir => {}
      c => out.push(c),
    }
  }
  out
}

/// Normalises `root` itself lexically (no filesystem I/O).
fn normalize_path(path: &Path) -> PathBuf {
  let mut out = PathBuf::new();
  for component in path.components() {
    match component {
      Component::ParentDir => {
        out.pop();
      }
      Component::CurDir => {}
      c => out.push(c),
    }
  }
  out
}

/// Joins `root` with `relative`, normalises the result lexically, then
/// verifies the resulting path starts with `root`.  Returns the normalised
/// path or an error if the path would escape `root`.
///
/// This prevents path-traversal attacks in addon manifests (e.g. `../../etc/passwd`).
pub(super) fn safe_join(root: &Path, relative: &str, label: &str) -> Result<PathBuf> {
  let norm_root = normalize_path(root);
  let path = normalize_join(root, relative);
  if !path.starts_with(&norm_root) {
    return Err(anyhow::anyhow!(
      "Path traversal blocked: {} '{}' would escape the root directory",
      label,
      relative
    ));
  }
  Ok(path)
}

pub(super) fn resolve_target(
  target: &crate::addons::manifest::Target,
  project_root: &Path,
) -> Result<Vec<PathBuf>> {
  use crate::addons::manifest::Target;
  match target {
    Target::File { file } => {
      let path = safe_join(project_root, file, "target file")?;
      Ok(vec![path])
    }
    Target::Glob { glob } => {
      // Validate the pattern itself doesn't traverse outside root
      safe_join(project_root, glob, "glob pattern")?;
      let pattern = project_root.join(glob).to_string_lossy().to_string();
      let canonical_root = project_root
        .canonicalize()
        .with_context(|| format!("Cannot resolve project root '{}'", project_root.display()))?;
      let paths = glob::glob(&pattern)?
        .filter_map(|e| e.ok())
        // Filter out any results that escape root via symlinks
        .filter(|p| {
          p.canonicalize()
            .map(|cp| cp.starts_with(&canonical_root))
            .unwrap_or(false)
        })
        .collect();
      Ok(paths)
    }
  }
}
