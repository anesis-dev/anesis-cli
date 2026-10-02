use std::{fs, path::Path};

use anyhow::Result;
use ignore::WalkBuilder;

use crate::templates::TemplateFile;

pub fn copy_dir_respecting_gitignore(source: &Path, dest: &Path) -> Result<()> {
  for entry in WalkBuilder::new(source)
    .hidden(false)
    .require_git(false)
    .filter_entry(|e| e.file_name() != ".git")
    .build()
  {
    let entry = entry?;
    if !entry.file_type().is_some_and(|t| t.is_file()) {
      continue;
    }
    let relative = entry.path().strip_prefix(source)?;
    let out = dest.join(relative);
    if let Some(parent) = out.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::copy(entry.path(), &out)?;
  }
  Ok(())
}

pub fn remove_stale_tmp_siblings(dest: &Path) {
  let (Some(parent), Some(name)) = (dest.parent(), dest.file_name().and_then(|n| n.to_str()))
  else {
    return;
  };
  let prefix = format!("{name}.tmp-");
  let Ok(entries) = fs::read_dir(parent) else {
    return;
  };
  let one_hour = std::time::Duration::from_secs(3600);
  for entry in entries.flatten() {
    let is_stale = entry.file_name().to_string_lossy().starts_with(&prefix)
      && entry
        .metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age > one_hour);
    if is_stale {
      let _ = fs::remove_dir_all(entry.path());
    }
  }
}

pub fn find_project_root(start: &Path) -> Option<std::path::PathBuf> {
  start
    .ancestors()
    .find(|dir| dir.join("anesis.json").is_file() || dir.join("anesis.lock").is_file())
    .map(Path::to_path_buf)
}

pub fn read_dir_to_files(path: &Path) -> Result<Vec<TemplateFile>> {
  let mut files = Vec::new();
  read_dir_recursive(path, path, &mut files)?;
  Ok(files)
}

pub fn read_dir_recursive(
  base: &Path,
  current: &Path,
  files: &mut Vec<TemplateFile>,
) -> Result<()> {
  for entry in fs::read_dir(current)? {
    let entry = entry?;
    let path = entry.path();
    let file_type = entry.file_type()?;

    if file_type.is_file() {
      let contents = fs::read(&path)?;
      let relative_path = path.strip_prefix(base)?.to_path_buf();
      #[cfg(unix)]
      let mode = {
        use std::os::unix::fs::PermissionsExt;
        Some(entry.metadata()?.permissions().mode() & 0o777)
      };
      #[cfg(not(unix))]
      let mode = None;
      files.push(TemplateFile {
        path: relative_path,
        contents,
        mode,
      });
    } else if file_type.is_dir() {
      read_dir_recursive(base, &path, files)?;
    }
  }

  Ok(())
}
