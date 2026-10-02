use std::path::Path;

use anyhow::{Context, Result};
use tempfile::NamedTempFile;

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
  let dir = path
    .parent()
    .filter(|p| !p.as_os_str().is_empty())
    .unwrap_or_else(|| Path::new("."));

  let mut temp_file = NamedTempFile::new_in(dir)
    .with_context(|| format!("Failed to create a temp file next to '{}'", path.display()))?;

  std::io::Write::write_all(&mut temp_file, bytes).with_context(|| {
    format!(
      "Failed to write to a temp file next to '{}'",
      path.display()
    )
  })?;

  temp_file
    .persist(path)
    .map_err(|e| e.error)
    .with_context(|| format!("Failed to atomically write '{}'", path.display()))?;

  Ok(())
}
