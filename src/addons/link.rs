use std::path::Path;

use anyhow::{Context, Result};
use inquire::Confirm;

use crate::{context::AppContext, utils::fs::copy_dir_respecting_gitignore};

use super::{cache::update_addons_cache, manifest};

pub fn link_addon(ctx: &AppContext, source: &Path, force: bool) -> Result<Option<String>> {
  let manifest_path = source.join("anesis.addon.json");
  let content = std::fs::read_to_string(&manifest_path).with_context(|| {
    format!(
      "No 'anesis.addon.json' found at {}",
      manifest_path.display()
    )
  })?;
  let manifest = manifest::parse(&content)?;

  let dest = ctx.paths.addons.join(&manifest.id);
  if !dest.starts_with(&ctx.paths.addons) {
    anyhow::bail!(
      "addon id '{}' would resolve outside the addons cache directory",
      manifest.id
    );
  }
  if dest.exists() {
    if !force
      && !Confirm::new(&format!(
        "An addon named '{}' is already cached and will be overwritten. Continue?",
        manifest.id
      ))
      .with_default(false)
      .prompt()?
    {
      return Ok(None);
    }
    std::fs::remove_dir_all(&dest).with_context(|| {
      format!(
        "Failed to clear existing cached addon at {}",
        dest.display()
      )
    })?;
  }
  copy_dir_respecting_gitignore(source, &dest)
    .with_context(|| format!("Failed to copy addon from {}", source.display()))?;

  update_addons_cache(&ctx.paths.addons, &manifest.id, &manifest, "local")?;
  Ok(Some(manifest.id))
}
