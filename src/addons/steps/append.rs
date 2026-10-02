use crate::utils::template_engine::TemplateContext;
use std::path::Path;

use anyhow::anyhow;

use crate::addons::manifest::AppendStep;

use super::{Rollback, StepFailure, StepResult, render_string, resolve_target};

pub fn execute_append(step: &AppendStep, project_root: &Path, ctx: &TemplateContext) -> StepResult {
  let paths = resolve_target(&step.target, project_root, ctx)?;
  let rendered = render_string(&step.content, ctx)?;

  let mut rollbacks = Vec::new();

  for path in paths {
    let original = match std::fs::read(&path) {
      Ok(o) => o,
      Err(e) => return Err(StepFailure::new(e, rollbacks)),
    };
    let text = match std::str::from_utf8(&original) {
      Ok(t) => t,
      Err(_) => {
        return Err(StepFailure::new(
          anyhow!(
            "{} is not valid UTF-8 (binary file); refusing to append",
            path.display()
          ),
          rollbacks,
        ));
      }
    };
    let mut new_content = text.to_string();

    if !new_content.is_empty() && !new_content.ends_with('\n') {
      new_content.push('\n');
    }
    new_content.push_str(&rendered);

    rollbacks.push(Rollback::restore_file(path.clone(), original));
    if let Err(e) = std::fs::write(&path, new_content) {
      return Err(StepFailure::new(e, rollbacks));
    }
  }

  Ok(rollbacks)
}
