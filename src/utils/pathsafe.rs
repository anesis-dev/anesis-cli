use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};

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

fn deepest_existing_ancestor(path: &Path) -> PathBuf {
  let mut current = path;
  loop {
    if current.symlink_metadata().is_ok() {
      return current.to_path_buf();
    }
    match current.parent() {
      Some(parent) => current = parent,
      None => return current.to_path_buf(),
    }
  }
}

pub fn safe_join(root: &Path, relative: &str, label: &str) -> Result<PathBuf> {
  let canon_root = root
    .canonicalize()
    .with_context(|| format!("Cannot resolve project root '{}'", root.display()))?;

  let candidate = normalize_join(&canon_root, relative);

  if !candidate.starts_with(&canon_root) {
    return Err(anyhow::anyhow!(
      "Path traversal blocked: {} '{}' would escape the root directory",
      label,
      relative
    ));
  }

  let canon_existing = deepest_existing_ancestor(&candidate)
    .canonicalize()
    .with_context(|| format!("Cannot resolve {} '{}'", label, relative))?;
  if !canon_existing.starts_with(&canon_root) {
    return Err(anyhow::anyhow!(
      "Path traversal blocked: {} '{}' resolves outside the root directory via a symlink",
      label,
      relative
    ));
  }

  Ok(candidate)
}
