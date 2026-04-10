use std::path::Path;

use anyhow::Result;
use serde::Deserialize;

use crate::{
  AppContext,
  auth::token::get_auth_user,
  utils::archive::download_and_extract,
};

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
    .await?
    .error_for_status()?
    .json()
    .await?;

  Ok(res)
}

pub async fn install_addon(ctx: &AppContext, addon_id: &str) -> Result<AddonManifest> {
  let info = get_addon_url(ctx, addon_id).await?;

  let dest = ctx.paths.addons.join(addon_id);

  // Skip download if cached commit matches and dir exists
  if let Some(cached) = get_cached_addon(&ctx.paths.addons, addon_id)?
    && cached.commit_sha == info.commit_sha
    && dest.exists()
  {
    let manifest_path = dest.join("oxide.addon.json");
    let content = std::fs::read_to_string(&manifest_path)?;
    let manifest: AddonManifest = serde_json::from_str(&content)?;
    println!("Addon '{}' is already up to date", addon_id);
    return Ok(manifest);
  }

  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(dest.clone());
  }

  download_and_extract(&ctx.client, &info.archive_url, &dest, None).await?;

  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
  }

  let manifest_path = dest.join("oxide.addon.json");
  let content = std::fs::read_to_string(&manifest_path)?;
  let manifest: AddonManifest = serde_json::from_str(&content)?;

  update_addons_cache(&ctx.paths.addons, addon_id, &manifest, &info.commit_sha)?;
  println!("Addon '{}' successfully downloaded", addon_id);

  Ok(manifest)
}

pub fn read_cached_manifest(addons_dir: &Path, addon_id: &str) -> Result<AddonManifest> {
  let manifest_path = addons_dir.join(addon_id).join("oxide.addon.json");
  let content = std::fs::read_to_string(&manifest_path)?;
  Ok(serde_json::from_str(&content)?)
}
