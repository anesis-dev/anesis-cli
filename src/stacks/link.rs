use std::path::Path;

use anyhow::{Context, Result};
use inquire::Confirm;

use super::{
  cache::cached_path,
  manifest::{StackManifest, load_stack},
};
use crate::context::AppContext;

pub fn link_stack(ctx: &AppContext, source: &Path, force: bool) -> Result<Option<String>> {
  let manifest: StackManifest = load_stack(source)
    .with_context(|| format!("Could not read a stack manifest at {}", source.display()))?;

  let dest = cached_path(ctx, &manifest.id);
  if !dest.starts_with(&ctx.paths.stacks) {
    anyhow::bail!(
      "stack id '{}' would resolve outside the stacks cache directory",
      manifest.id
    );
  }
  if dest.exists()
    && !force
    && !Confirm::new(&format!(
      "A stack named '{}' is already cached and will be overwritten. Continue?",
      manifest.id
    ))
    .with_default(false)
    .prompt()?
  {
    return Ok(None);
  }

  std::fs::create_dir_all(&ctx.paths.stacks)?;
  std::fs::write(&dest, serde_json::to_string_pretty(&manifest)?)
    .with_context(|| format!("Failed to cache the stack at {}", dest.display()))?;

  Ok(Some(manifest.id))
}
