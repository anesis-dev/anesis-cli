use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result, bail};

use crate::{context::AppContext, utils::fs::copy_dir_respecting_gitignore, utils::ui};

use super::{diff::show_diff, runner::run_addon_command};

pub async fn test_addon(
  ctx: &AppContext,
  addon_id: &str,
  command: &str,
  project: Option<String>,
) -> Result<()> {
  let fixture = resolve_fixture(ctx, addon_id, project)?;

  let baseline = tempfile::Builder::new()
    .prefix("anesis-test-base-")
    .tempdir()?;
  let work = tempfile::Builder::new()
    .prefix("anesis-test-work-")
    .tempdir()?;
  copy_dir_respecting_gitignore(&fixture, baseline.path())
    .with_context(|| format!("failed to copy fixture from {}", fixture.display()))?;
  copy_dir_respecting_gitignore(&fixture, work.path())
    .with_context(|| format!("failed to copy fixture from {}", fixture.display()))?;

  println!(
    "Testing {} {} on a copy of {} (original untouched)...\n",
    ui::accent(addon_id),
    ui::accent(command),
    fixture.display()
  );

  run_addon_command(
    ctx,
    addon_id,
    command,
    work.path(),
    &HashMap::new(),
    true,
    false,
  )
  .await?;

  println!("\n{}", ui::bold("Diff (changes the addon made):"));
  show_diff(baseline.path(), work.path());
  Ok(())
}

#[doc(hidden)]
pub fn resolve_fixture_for_tests(
  ctx: &AppContext,
  addon_id: &str,
  project: Option<String>,
) -> Result<PathBuf> {
  resolve_fixture(ctx, addon_id, project)
}

fn resolve_fixture(ctx: &AppContext, addon_id: &str, project: Option<String>) -> Result<PathBuf> {
  if let Some(p) = project {
    let path = PathBuf::from(&p);
    if !path.is_dir() {
      bail!("--project '{p}' is not a directory");
    }
    return Ok(path);
  }
  let fixture = ctx.paths.addons.join(addon_id).join("test-fixture");
  if fixture.is_dir() {
    return Ok(fixture);
  }
  bail!(
    "No fixture project. Pass --project <path> or ship a 'test-fixture/' directory with the addon."
  )
}
