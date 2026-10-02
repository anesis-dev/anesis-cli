use crate::utils::template_engine::TemplateContext;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub mod append;
pub mod copy;
pub mod create;
pub mod delete;
pub mod inject;
pub mod json_patch;
pub mod move_step;
pub mod packages;
pub mod rename;
pub mod replace;
pub mod run;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Rollback {
  DeleteCreatedFile {
    path: PathBuf,
  },
  RestoreFile {
    path: PathBuf,
    original: Vec<u8>,
    #[serde(default)]
    mode: Option<u32>,
    #[serde(default)]
    is_symlink: bool,
  },
  RenameFile {
    from: PathBuf,
    to: PathBuf,
  },
  IrreversibleRun {
    command: String,
  },
}

impl Rollback {
  pub(super) fn restore_file(path: PathBuf, original: Vec<u8>) -> Self {
    Rollback::RestoreFile {
      path,
      original,
      mode: None,
      is_symlink: false,
    }
  }

  #[doc(hidden)]
  pub fn restore_file_for_tests(path: PathBuf, original: Vec<u8>) -> Self {
    Rollback::restore_file(path, original)
  }
}

#[derive(Debug)]
pub struct StepFailure {
  pub error: anyhow::Error,
  pub rollbacks: Vec<Rollback>,
}

impl StepFailure {
  pub fn new(error: impl Into<anyhow::Error>, rollbacks: Vec<Rollback>) -> Self {
    Self {
      error: error.into(),
      rollbacks,
    }
  }

  pub fn without_rollbacks(error: impl Into<anyhow::Error>) -> Self {
    Self::new(error, Vec::new())
  }
}

impl std::fmt::Display for StepFailure {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Display::fmt(&self.error, f)
  }
}

impl std::error::Error for StepFailure {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    self.error.source()
  }
}

impl From<anyhow::Error> for StepFailure {
  fn from(error: anyhow::Error) -> Self {
    Self::without_rollbacks(error)
  }
}

pub type StepResult = std::result::Result<Vec<Rollback>, StepFailure>;

pub fn render_string(s: &str, ctx: &TemplateContext) -> Result<String> {
  crate::utils::template_engine::render_string(s, ctx)
}

pub(super) fn safe_join(root: &Path, relative: &str, label: &str) -> Result<PathBuf> {
  crate::utils::pathsafe::safe_join(root, relative, label)
}

pub(super) fn resolve_target(
  target: &crate::addons::manifest::Target,
  project_root: &Path,
  ctx: &TemplateContext,
) -> Result<Vec<PathBuf>> {
  use crate::addons::manifest::Target;
  match target {
    Target::File { file } => {
      let rendered = render_string(file, ctx)?;
      let path = safe_join(project_root, &rendered, "target file")?;
      Ok(vec![path])
    }
    Target::Glob { glob } => {
      let glob = render_string(glob, ctx)?;
      safe_join(project_root, &glob, "glob pattern")?;
      let canonical_root = project_root
        .canonicalize()
        .with_context(|| format!("Cannot resolve project root '{}'", project_root.display()))?;
      let escaped_root = glob::Pattern::escape(&canonical_root.to_string_lossy());
      let pattern = format!("{escaped_root}/{glob}");
      let paths = glob::glob(&pattern)?
        .filter_map(|e| e.ok())
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
