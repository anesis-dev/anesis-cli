use crate::utils::template_engine::TemplateContext;
use std::path::Path;

use anyhow::anyhow;
use inquire::Select;

use crate::addons::manifest::{IfNotFound, InjectStep};

use super::{Rollback, StepFailure, StepResult, render_string, resolve_target};

fn normalize_whitespace(s: &str) -> String {
  s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn execute_inject(
  step: &InjectStep,
  project_root: &Path,
  ctx: &TemplateContext,
  non_interactive: bool,
) -> StepResult {
  if step.after.is_some() && step.before.is_some() {
    return Err(StepFailure::without_rollbacks(anyhow!(
      "inject step cannot set both 'after' and 'before'; choose one marker"
    )));
  }

  let paths = resolve_target(&step.target, project_root, ctx)?;
  let rendered: Vec<String> = render_string(&step.content, ctx)?
    .lines()
    .map(str::to_string)
    .collect();

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
            "{} is not valid UTF-8 (binary file); refusing to inject",
            path.display()
          ),
          rollbacks,
        ));
      }
    };
    let mut file_lines: Vec<String> = text.lines().map(str::to_string).collect();

    let marker = step.after.as_deref().or(step.before.as_deref());

    if let Some(marker) = marker {
      let normalized_marker = normalize_whitespace(marker);
      match file_lines
        .iter()
        .position(|l| normalize_whitespace(l).contains(&normalized_marker))
      {
        Some(idx) => {
          let insert_idx = if step.after.is_some() { idx + 1 } else { idx };
          for (i, line) in rendered.iter().enumerate() {
            file_lines.insert(insert_idx + i, line.clone());
          }
        }
        None => match step.if_not_found {
          IfNotFound::Skip => continue,
          IfNotFound::Error => {
            return Err(StepFailure::new(
              anyhow!("Marker {:?} not found in {}", marker, path.display()),
              rollbacks,
            ));
          }
          IfNotFound::WarnAndAsk => {
            eprintln!(
              "Warning: marker {:?} not found in {}",
              marker,
              path.display()
            );
            if non_interactive {
              continue;
            }
            let choice =
              match Select::new("How would you like to proceed?", vec!["Continue", "Abort"])
                .prompt()
              {
                Ok(c) => c,
                Err(e) => return Err(StepFailure::new(e, rollbacks)),
              };
            if choice == "Abort" {
              return Err(StepFailure::new(anyhow!("Aborted by user"), rollbacks));
            }
            continue;
          }
        },
      }
    } else {
      let mut new_lines = rendered.clone();
      new_lines.extend(file_lines);
      file_lines = new_lines;
    }

    let had_trailing_newline = original.last().copied() == Some(b'\n');
    let line_ending = if text.contains("\r\n") { "\r\n" } else { "\n" };
    rollbacks.push(Rollback::restore_file(path.clone(), original));
    let mut new_content = file_lines.join(line_ending);
    if had_trailing_newline {
      new_content.push_str(line_ending);
    }
    if let Err(e) = crate::utils::atomic::write_file_atomic(&path, new_content.as_bytes()) {
      return Err(StepFailure::new(e, rollbacks));
    }
  }

  Ok(rollbacks)
}
