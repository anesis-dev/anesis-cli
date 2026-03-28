use std::{fs, path::Path};

use anyhow::Result;

use crate::templates::TemplateFile;

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
      files.push(TemplateFile {
        path: relative_path,
        contents,
      });
    } else if file_type.is_dir() {
      read_dir_recursive(base, &path, files)?;
    }
  }

  Ok(())
}
