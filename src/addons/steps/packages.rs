use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use inquire::Confirm;

use crate::{addons::manifest::PackagesStep, utils::ui};

use super::{Rollback, StepFailure, StepResult};

enum PackageManager {
  Npm,
  Bun,
  Pnpm,
  Yarn,
  Cargo,
}

impl PackageManager {
  fn program(&self) -> &'static str {
    match self {
      PackageManager::Npm => "npm",
      PackageManager::Bun => "bun",
      PackageManager::Pnpm => "pnpm",
      PackageManager::Yarn => "yarn",
      PackageManager::Cargo => "cargo",
    }
  }

  fn add_args(&self) -> &'static [&'static str] {
    match self {
      PackageManager::Npm => &["install"],
      _ => &["add"],
    }
  }

  fn dev_flag(&self) -> &'static str {
    match self {
      PackageManager::Npm | PackageManager::Pnpm => "--save-dev",
      _ => "--dev",
    }
  }

  fn snapshot_files(&self) -> &'static [&'static str] {
    match self {
      PackageManager::Cargo => &["Cargo.toml", "Cargo.lock"],
      PackageManager::Npm => &["package.json", "package-lock.json"],
      PackageManager::Bun => &["package.json", "bun.lock", "bun.lockb"],
      PackageManager::Pnpm => &["package.json", "pnpm-lock.yaml"],
      PackageManager::Yarn => &["package.json", "yarn.lock"],
    }
  }
}

fn describe_install(pm: &PackageManager, step: &PackagesStep) -> String {
  let mut commands = Vec::new();
  if !step.dependencies.is_empty() {
    commands.push(format!(
      "{} {} {}",
      pm.program(),
      pm.add_args().join(" "),
      step.dependencies.join(" ")
    ));
  }
  if !step.dev_dependencies.is_empty() {
    commands.push(format!(
      "{} {} {} {}",
      pm.program(),
      pm.add_args().join(" "),
      pm.dev_flag(),
      step.dev_dependencies.join(" ")
    ));
  }
  commands.join(" && ")
}

fn detect_pm(root: &Path) -> Result<PackageManager> {
  if root.join("bun.lock").exists() || root.join("bun.lockb").exists() {
    Ok(PackageManager::Bun)
  } else if root.join("pnpm-lock.yaml").exists() {
    Ok(PackageManager::Pnpm)
  } else if root.join("yarn.lock").exists() {
    Ok(PackageManager::Yarn)
  } else if root.join("package.json").exists() {
    Ok(PackageManager::Npm)
  } else if root.join("Cargo.toml").exists() {
    Ok(PackageManager::Cargo)
  } else {
    bail!("No package manager detected (no package.json or Cargo.toml in the project root)")
  }
}

fn missing_pm_error(program: &str, err: which::Error) -> anyhow::Error {
  anyhow::Error::new(err).context(format!(
    "'{program}' was not found on PATH — is it installed?"
  ))
}

pub fn execute_packages(
  step: &PackagesStep,
  project_root: &Path,
  non_interactive: bool,
  allow_run: bool,
) -> StepResult {
  execute_packages_inner(step, project_root, non_interactive, allow_run)
    .map_err(StepFailure::without_rollbacks)
}

fn execute_packages_inner(
  step: &PackagesStep,
  project_root: &Path,
  non_interactive: bool,
  allow_run: bool,
) -> Result<Vec<Rollback>> {
  if step.dependencies.is_empty() && step.dev_dependencies.is_empty() {
    return Ok(Vec::new());
  }
  let pm = detect_pm(project_root)?;

  if !allow_run {
    let summary = describe_install(&pm, step);
    println!(
      "  {} {}",
      ui::muted("will run:"),
      ui::yellow(crate::utils::sanitize::sanitize_for_display(&summary))
    );

    if non_interactive {
      bail!(
        "this addon wants to install packages, and there is nobody to ask:\n  {summary}\n\n\
         Re-run with --allow-run (or set ANESIS_ALLOW_RUN=1) if you trust this addon. \
         `--yes` deliberately does not cover package installation."
      );
    }

    if !Confirm::new("Install these packages?")
      .with_default(false)
      .prompt()?
    {
      bail!("packages step declined: '{summary}'");
    }
  }

  let mut rollbacks = Vec::new();
  for name in pm.snapshot_files() {
    let path = project_root.join(name);
    if path.exists() {
      rollbacks.push(Rollback::restore_file(path.clone(), std::fs::read(&path)?));
    } else {
      rollbacks.push(Rollback::DeleteCreatedFile { path });
    }
  }

  let program = which::which(pm.program()).map_err(|e| missing_pm_error(pm.program(), e))?;

  let run = |extra: &[&str], specs: &[String]| -> Result<()> {
    let status = Command::new(&program)
      .args(pm.add_args())
      .args(extra)
      .args(specs)
      .current_dir(project_root)
      .status()
      .with_context(|| {
        format!(
          "failed to run '{}' — is it installed and on PATH?",
          pm.program()
        )
      })?;
    if !status.success() {
      bail!("'{}' exited with {}", pm.program(), status);
    }
    Ok(())
  };

  let result = (|| {
    if !step.dependencies.is_empty() {
      run(&[], &step.dependencies)?;
    }
    if !step.dev_dependencies.is_empty() {
      run(&[pm.dev_flag()], &step.dev_dependencies)?;
    }
    Ok(())
  })();

  if let Err(err) = result {
    for rb in rollbacks.iter().rev() {
      match rb {
        Rollback::RestoreFile { path, original, .. } => {
          let _ = std::fs::write(path, original);
        }
        Rollback::DeleteCreatedFile { path } => {
          let _ = std::fs::remove_file(path);
        }
        _ => {}
      }
    }
    return Err(err);
  }

  Ok(rollbacks)
}

pub fn detect_pm_for_tests(root: &Path) -> Result<&'static str> {
  detect_pm(root).map(|pm| pm.program())
}

#[doc(hidden)]
pub fn missing_pm_error_for_tests(program: &str, err: which::Error) -> anyhow::Error {
  missing_pm_error(program, err)
}
