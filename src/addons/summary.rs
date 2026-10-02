use super::steps::Rollback;
use crate::utils::ui::symbols;

const RENDER_BLOCK_LIMIT: usize = 20;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ChangeSummary {
  pub created: usize,
  pub modified: usize,
  pub renamed: usize,
  pub irreversible_runs: usize,
}

impl ChangeSummary {
  pub fn from_rollbacks(rollbacks: &[Rollback]) -> Self {
    let mut summary = Self::default();
    for rollback in rollbacks {
      match rollback {
        Rollback::DeleteCreatedFile { .. } => summary.created += 1,
        Rollback::RestoreFile { .. } => summary.modified += 1,
        Rollback::RenameFile { .. } => summary.renamed += 1,
        Rollback::IrreversibleRun { .. } => summary.irreversible_runs += 1,
      }
    }
    summary
  }

  pub fn files_changed(&self) -> usize {
    self.created + self.modified + self.renamed
  }

  pub fn is_empty(&self) -> bool {
    self.files_changed() == 0 && self.irreversible_runs == 0
  }

  pub fn render(&self) -> String {
    if self.is_empty() {
      return "No changes.".to_string();
    }

    let mut parts = Vec::new();
    let files = self.files_changed();
    if files > 0 {
      parts.push(format!(
        "{files} file{} changed",
        if files == 1 { "" } else { "s" }
      ));
    }
    if self.created > 0 {
      parts.push(format!("{} created", self.created));
    }
    if self.modified > 0 {
      parts.push(format!("{} modified", self.modified));
    }
    if self.renamed > 0 {
      parts.push(format!("{} renamed", self.renamed));
    }
    if self.irreversible_runs > 0 {
      parts.push(format!(
        "{} shell command{} run (not reversible)",
        self.irreversible_runs,
        if self.irreversible_runs == 1 { "" } else { "s" }
      ));
    }
    parts.join(", ")
  }

  pub fn render_block(&self, rollbacks: &[Rollback]) -> String {
    if self.is_empty() {
      return "No changes.".to_string();
    }

    let files = self.files_changed();
    let mut lines = vec![format!(
      "{} {files} file{} changed",
      symbols::ok(),
      if files == 1 { "" } else { "s" }
    )];

    let mut shown = 0usize;
    for rollback in rollbacks {
      if shown >= RENDER_BLOCK_LIMIT {
        break;
      }
      let (marker, path) = match rollback {
        Rollback::DeleteCreatedFile { path } => ("+", path.display().to_string()),
        Rollback::RestoreFile { path, .. } => ("~", path.display().to_string()),
        Rollback::RenameFile { to, .. } => (symbols::arrow(), to.display().to_string()),
        Rollback::IrreversibleRun { .. } => continue,
      };
      lines.push(format!("  {marker} {path}"));
      shown += 1;
    }

    let remaining = files.saturating_sub(shown);
    if remaining > 0 {
      lines.push(format!("  {}and {remaining} more", symbols::ellipsis()));
    }

    lines.join("\n")
  }
}
