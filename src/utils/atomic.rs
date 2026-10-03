use std::{fs::File, path::Path};

use anyhow::{Context, Result};
use tempfile::NamedTempFile;

#[derive(Clone, Copy, PartialEq)]
enum Perms {
  Preserve,
  Shared,
  Private,
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
  write_atomic_with(path, bytes, Perms::Preserve)
}

pub fn write_file_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
  write_atomic_with(path, bytes, Perms::Shared)
}

pub fn write_private_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
  write_atomic_with(path, bytes, Perms::Private)
}

fn write_atomic_with(path: &Path, bytes: &[u8], perms: Perms) -> Result<()> {
  let target = match path.symlink_metadata() {
    Ok(meta) if meta.file_type().is_symlink() => path
      .canonicalize()
      .with_context(|| format!("Failed to resolve symlink '{}'", path.display()))?,
    _ => path.to_path_buf(),
  };
  let path = target.as_path();

  let dir = path
    .parent()
    .filter(|p| !p.as_os_str().is_empty())
    .unwrap_or_else(|| Path::new("."));

  let mut builder = tempfile::Builder::new();
  #[cfg(unix)]
  if perms == Perms::Shared {
    use std::os::unix::fs::PermissionsExt;
    builder.permissions(std::fs::Permissions::from_mode(0o666));
  }
  #[cfg(not(unix))]
  let _ = perms;
  let mut temp_file: NamedTempFile = builder
    .tempfile_in(dir)
    .with_context(|| format!("Failed to create a temp file next to '{}'", path.display()))?;

  std::io::Write::write_all(&mut temp_file, bytes).with_context(|| {
    format!(
      "Failed to write to a temp file next to '{}'",
      path.display()
    )
  })?;

  if perms != Perms::Private
    && let Ok(meta) = std::fs::metadata(path)
  {
    let _ = temp_file.as_file().set_permissions(meta.permissions());
  }

  temp_file
    .persist(path)
    .map_err(|e| e.error)
    .with_context(|| format!("Failed to atomically write '{}'", path.display()))?;

  Ok(())
}

pub fn with_file_lock<T>(path: &Path, f: impl FnOnce() -> Result<T>) -> Result<T> {
  let mut lock_path = path.as_os_str().to_owned();
  lock_path.push(".lock");
  let file = File::options()
    .create(true)
    .truncate(false)
    .write(true)
    .open(&lock_path)
    .with_context(|| {
      format!(
        "Failed to open lock file '{}'",
        Path::new(&lock_path).display()
      )
    })?;
  file
    .lock()
    .with_context(|| format!("Failed to lock '{}'", Path::new(&lock_path).display()))?;
  let result = f();
  let _ = file.unlock();
  result
}
