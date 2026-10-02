use crate::utils::template_engine::TemplateContext;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use inquire::Confirm;

use crate::{addons::manifest::RunStep, utils::ui};

use super::{Rollback, StepFailure, StepResult};

pub fn execute_run(
  step: &RunStep,
  project_root: &Path,
  ctx: &TemplateContext,
  non_interactive: bool,
  allow_run: bool,
) -> StepResult {
  execute_run_inner(step, project_root, ctx, non_interactive, allow_run)
    .map_err(StepFailure::without_rollbacks)
}

fn execute_run_inner(
  step: &RunStep,
  project_root: &Path,
  ctx: &TemplateContext,
  non_interactive: bool,
  allow_run: bool,
) -> Result<Vec<Rollback>> {
  let command = super::render_string(&step.command, ctx)?;

  if !step.description.is_empty() {
    println!(
      "  {}",
      ui::muted(crate::utils::sanitize::sanitize_for_display(
        &step.description
      ))
    );
  }
  println!(
    "  {} {}",
    ui::muted("will run:"),
    ui::yellow(crate::utils::sanitize::sanitize_for_display(&command))
  );

  if !allow_run {
    if non_interactive {
      bail!(
        "this addon wants to run a shell command, and there is nobody to ask:\n  {command}\n\n\
         Re-run with --allow-run (or set ANESIS_ALLOW_RUN=1) if you trust this addon. \
         `--yes` deliberately does not cover shell execution."
      );
    }

    if !Confirm::new("Run this command?")
      .with_default(false)
      .prompt()?
    {
      bail!("run step declined: '{command}'");
    }
  }

  let status = shell_command(&command)
    .current_dir(project_root)
    .status()
    .with_context(|| format!("failed to launch shell for '{command}'"))?;
  if !status.success() {
    bail!("command '{command}' exited with {status}");
  }

  Ok(vec![Rollback::IrreversibleRun { command }])
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
  let mut cmd = Command::new("cmd");
  cmd.arg("/C").arg(command);
  cmd
}

#[cfg(not(windows))]
fn shell_command(command: &str) -> Command {
  let mut cmd = Command::new("sh");
  cmd.arg("-c").arg(command);
  cmd
}
