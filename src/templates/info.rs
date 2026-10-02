use std::path::Path;

use anyhow::{Context, Result};

use crate::{
  context::AppContext, templates::AnesisTemplate, templates::install::install_template, utils::ui,
};

use super::cache::get_cached_template;

pub async fn template_info(ctx: &AppContext, template_name: &str, json: bool) -> Result<()> {
  let cached = get_cached_template(ctx, template_name)?;
  let manifest = match cached.filter(|c| ctx.paths.templates.join(&c.path).exists()) {
    Some(cached) => read_manifest(&ctx.paths.templates.join(&cached.path))?,
    None => {
      install_template(ctx, template_name).await?;
      read_manifest(&ctx.paths.templates.join(template_name))?
    }
  };

  if json {
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    return Ok(());
  }

  println!(
    "{} {}",
    ui::bold(&manifest.metadata.display_name),
    ui::muted(format!("({})", manifest.name))
  );
  println!(
    "{} {} anesis {}",
    ui::accent(format!("v{}", manifest.version)),
    ui::muted("·"),
    manifest.anesis_version
  );
  if !manifest.metadata.description.is_empty() {
    println!("{}", manifest.metadata.description);
  }
  if !manifest.repository.url.is_empty() {
    ui::kv("repository", &manifest.repository.url);
  }
  Ok(())
}

fn read_manifest(dir: &Path) -> Result<AnesisTemplate> {
  let path = dir.join("anesis.template.json");
  let content = std::fs::read_to_string(&path)
    .with_context(|| format!("Failed to read template manifest at {}", path.display()))?;
  serde_json::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
}
