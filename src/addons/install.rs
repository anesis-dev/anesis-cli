use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{AppContext, auth::token::get_auth_user, utils::archive::download_and_extract};

use super::{
  cache::{get_cached_addon, update_addons_cache},
  manifest::AddonManifest,
};

#[derive(Deserialize)]
struct AddonUrlResponse {
  archive_url: String,
  commit_sha: String,
}

async fn get_addon_url(ctx: &AppContext, addon_id: &str) -> Result<AddonUrlResponse> {
  let user = get_auth_user(&ctx.paths.auth)?;

  let res: AddonUrlResponse = ctx
    .client
    .get(format!("{}/addon/{addon_id}/url", ctx.backend_url))
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .send()
    .await
    .with_context(|| format!("Failed to fetch download URL for addon '{addon_id}'"))?
    .error_for_status()
    .with_context(|| format!("Server returned error for addon '{addon_id}'"))?
    .json()
    .await
    .with_context(|| format!("Failed to parse URL response for addon '{addon_id}'"))?;

  Ok(res)
}

pub async fn install_addon(ctx: &AppContext, addon_id: &str) -> Result<AddonManifest> {
  let info = get_addon_url(ctx, addon_id).await?;

  // The archive already contains a top-level `{addon_id}/` directory,
  // so we extract into the addons root — files land at addons/{addon_id}/...
  let addons_dir = &ctx.paths.addons;
  let addon_dir = addons_dir.join(addon_id);

  // Skip download if cached commit matches and addon dir exists
  if let Some(cached) = get_cached_addon(addons_dir, addon_id)
    .with_context(|| format!("Failed to read addons cache while checking '{addon_id}'"))?
    && cached.commit_sha == info.commit_sha
    && addon_dir.exists()
  {
    let manifest_path = addon_dir.join("oxide.addon.json");
    let content = std::fs::read_to_string(&manifest_path)
      .with_context(|| format!("Failed to read cached manifest at {}", manifest_path.display()))?;
    let manifest: AddonManifest = serde_json::from_str(&content)
      .with_context(|| format!("Failed to parse cached manifest at {}", manifest_path.display()))?;
    println!("Addon '{}' is already up to date", addon_id);
    return Ok(manifest);
  }

  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(addon_dir.clone());
  }

  download_and_extract(&ctx.client, &info.archive_url, addons_dir, None)
    .await
    .with_context(|| {
      format!(
        "Failed to download and extract addon '{addon_id}' from {}",
        info.archive_url
      )
    })?;

  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
  }

  let manifest_path = addon_dir.join("oxide.addon.json");
  let content = std::fs::read_to_string(&manifest_path).with_context(|| {
    format!(
      "Addon '{addon_id}' was extracted but 'oxide.addon.json' was not found at {}. \
       Make sure the addon archive contains a top-level '{addon_id}/' directory with 'oxide.addon.json' inside.",
      manifest_path.display()
    )
  })?;

  let manifest: AddonManifest = serde_json::from_str(&content)
    .with_context(|| format!("Failed to parse oxide.addon.json for addon '{addon_id}'"))?;

  update_addons_cache(addons_dir, addon_id, &manifest, &info.commit_sha)
    .with_context(|| format!("Failed to update addons cache after installing '{addon_id}'"))?;

  println!("Addon '{}' successfully downloaded", addon_id);

  Ok(manifest)
}

pub fn read_cached_manifest(addons_dir: &Path, addon_id: &str) -> Result<AddonManifest> {
  let manifest_path = addons_dir.join(addon_id).join("oxide.addon.json");
  let content = std::fs::read_to_string(&manifest_path)
    .with_context(|| format!("Failed to read manifest for addon '{addon_id}' at {}", manifest_path.display()))?;
  serde_json::from_str(&content)
    .with_context(|| format!("Failed to parse manifest for addon '{addon_id}'"))
}
