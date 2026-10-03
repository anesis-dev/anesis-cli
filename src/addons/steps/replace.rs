use crate::utils::template_engine::TemplateContext;
use std::path::Path;

use anyhow::anyhow;
use inquire::Select;

use crate::addons::manifest::{IfNotFound, ReplaceStep};

use super::{Rollback, StepFailure, StepResult, render_string, resolve_target};

pub fn execute_replace(
  step: &ReplaceStep,
  project_root: &Path,
  ctx: &TemplateContext,
  non_interactive: bool,
) -> StepResult {
  let paths = resolve_target(&step.target, project_root, ctx)?;
  let rendered_replace = render_string(&step.replace, ctx)?;

  let mut rollbacks = Vec::new();

  for path in paths {
    let original = match std::fs::read(&path) {
      Ok(o) => o,
      Err(e) => return Err(StepFailure::new(e, rollbacks)),
    };
    let content = match std::str::from_utf8(&original) {
      Ok(c) => c.to_string(),
      Err(_) => {
        return Err(StepFailure::new(
          anyhow!(
            "{} is not valid UTF-8 (binary file); refusing to replace",
            path.display()
          ),
          rollbacks,
        ));
      }
    };

    if !content.contains(&step.find) {
      match step.if_not_found {
        IfNotFound::Skip => continue,
        IfNotFound::Error => {
          return Err(StepFailure::new(
            anyhow!("Pattern {:?} not found in {}", step.find, path.display()),
            rollbacks,
          ));
        }
        IfNotFound::WarnAndAsk => {
          eprintln!("Warning: {:?} not found in {}", step.find, path.display());
          if non_interactive {
            continue;
          }
          let choice =
            match Select::new("How would you like to proceed?", vec!["Continue", "Abort"]).prompt()
            {
              Ok(c) => c,
              Err(e) => return Err(StepFailure::new(e, rollbacks)),
            };
          if choice == "Abort" {
            return Err(StepFailure::new(anyhow!("Aborted by user"), rollbacks));
          }
          continue;
        }
      }
    }

    let new_content = content.replace(&step.find, &rendered_replace);
    rollbacks.push(Rollback::restore_file(path.clone(), original));
    if let Err(e) = crate::utils::atomic::write_file_atomic(&path, new_content.as_bytes()) {
      return Err(StepFailure::new(e, rollbacks));
    }
  }

  Ok(rollbacks)
}
