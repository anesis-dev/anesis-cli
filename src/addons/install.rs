use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::{
  auth::token::get_auth_user,
  context::{AppContext, CleanupTask},
  utils::{
    archive::download_and_extract,
    errors::classify_reqwest_error,
    ui::{self, spinner},
  },
};

use super::{
  cache::{CachedAddon, get_cached_addon, update_addons_cache},
  manifest::AddonManifest,
};

pub async fn record_addon_use(ctx: &AppContext, addon_id: &str) {
  if !ctx.telemetry {
    return;
  }
  let Ok(user) = get_auth_user(&ctx.paths.auth) else {
    return;
  };
  let url = format!("{}/addon/{}/use", ctx.backend_url, addon_id);
  if let Err(e) = ctx
    .client
    .post(&url)
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .timeout(std::time::Duration::from_secs(3))
    .send()
    .await
  {
    log::warn!("Failed to record addon use event: {e:?}");
  }
}

#[derive(Deserialize)]
struct AddonUrlResponse {
  archive_url: String,
  #[serde(default)]
  archive_token: Option<String>,
  commit_sha: String,
  subdir: Option<String>,
  #[serde(default)]
  version: String,
}

#[derive(Debug)]
pub enum AddonInstallResult {
  Installed(AddonManifest),
  Updated(AddonManifest),
  UpToDate(AddonManifest),
}

impl AddonInstallResult {
  pub fn into_manifest(self) -> AddonManifest {
    match self {
      Self::Installed(manifest) | Self::Updated(manifest) | Self::UpToDate(manifest) => manifest,
    }
  }

  pub fn message(&self, addon_id: &str) -> Option<String> {
    match self {
      Self::Installed(_) => Some(format!("Addon '{addon_id}' successfully downloaded")),
      Self::Updated(manifest) => Some(format!(
        "Addon '{addon_id}' updated to v{}",
        manifest.version
      )),
      Self::UpToDate(_) => None,
    }
  }

  pub fn update_message(&self, addon_id: &str) -> Option<String> {
    match self {
      Self::Updated(manifest) => Some(format!(
        "Addon '{addon_id}' updated to v{}",
        manifest.version
      )),
      Self::Installed(_) | Self::UpToDate(_) => None,
    }
  }

  pub fn up_to_date_message(addon_id: &str) -> String {
    format!("Addon '{addon_id}' is already up to date")
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallState {
  Install,
  Update,
  UpToDate,
}

fn classify_install_state(
  cached_addon: Option<&CachedAddon>,
  addon_dir_exists: bool,
  latest_commit_sha: &str,
) -> InstallState {
  let Some(cached_addon) = cached_addon else {
    return InstallState::Install;
  };

  if !addon_dir_exists {
    return InstallState::Install;
  }

  if cached_addon.commit_sha == latest_commit_sha {
    InstallState::UpToDate
  } else {
    InstallState::Update
  }
}

#[doc(hidden)]
pub fn classify_install_state_for_tests(
  cached_addon: Option<&CachedAddon>,
  addon_dir_exists: bool,
  latest_commit_sha: &str,
) -> &'static str {
  match classify_install_state(cached_addon, addon_dir_exists, latest_commit_sha) {
    InstallState::Install => "install",
    InstallState::Update => "update",
    InstallState::UpToDate => "up_to_date",
  }
}

async fn get_addon_url(ctx: &AppContext, addon_id: &str) -> Result<AddonUrlResponse> {
  get_addon_url_raw(&ctx.client, &ctx.backend_url, &ctx.paths.auth, addon_id).await
}

async fn get_addon_url_raw(
  client: &Client,
  backend_url: &str,
  auth_path: &Path,
  addon_id: &str,
) -> Result<AddonUrlResponse> {
  let token = get_auth_user(auth_path).ok().map(|u| u.token);

  let mut req = client
    .get(format!("{backend_url}/addon/{addon_id}/url"))
    .header("Content-Type", "application/json");
  if let Some(token) = token {
    req = req.bearer_auth(token);
  }

  let response = req
    .send()
    .await
    .with_context(|| format!("Failed to fetch download URL for addon '{addon_id}'"))?;

  if !response.status().is_success() {
    let err = response.error_for_status().unwrap_err();
    return Err(classify_reqwest_error(err, &format!("addon '{addon_id}'")));
  }

  let res: AddonUrlResponse = response
    .json()
    .await
    .with_context(|| format!("Failed to parse URL response for addon '{addon_id}'"))?;

  Ok(res)
}

pub async fn install_addon(ctx: &AppContext, addon_id: &str) -> Result<AddonInstallResult> {
  let addon_dir = ctx.paths.addon_dir(addon_id)?;

  let sp = spinner(format!("Fetching info for addon '{addon_id}'..."));
  let info = get_addon_url(ctx, addon_id)
    .await
    .inspect_err(|_| sp.finish_and_clear())?;
  sp.finish_and_clear();

  let addons_dir = &ctx.paths.addons;
  let cached_addon = get_cached_addon(addons_dir, addon_id)
    .with_context(|| format!("Failed to read addons cache while checking '{addon_id}'"))?;
  let install_state =
    classify_install_state(cached_addon.as_ref(), addon_dir.exists(), &info.commit_sha);

  if install_state == InstallState::UpToDate {
    return Ok(AddonInstallResult::UpToDate(read_cached_manifest(
      addons_dir, addon_id,
    )?));
  }

  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(CleanupTask::PartialDownload {
      path: addon_dir.clone(),
      prune_root: ctx.paths.addons.clone(),
      label: "addon",
    });
  }

  if install_state == InstallState::Update && addon_dir.exists() {
    std::fs::remove_dir_all(&addon_dir)
      .with_context(|| format!("Failed to clear stale addon at {}", addon_dir.display()))?;
  }

  let action = if install_state == InstallState::Update {
    "Updating"
  } else {
    "Downloading"
  };
  if !ui::is_quiet() {
    println!("{action} addon '{addon_id}'...");
  }
  let download_result = download_and_extract(
    &ctx.client,
    &info.archive_url,
    &addon_dir,
    info.subdir.as_deref(),
    info.archive_token.as_deref(),
  )
  .await
  .with_context(|| {
    format!(
      "Failed to download and extract addon '{addon_id}' from {}",
      info.archive_url
    )
  });

  {
    let mut guard = ctx.cleanup_state.lock().unwrap_or_else(|e| e.into_inner());
    *guard = None;
  }

  download_result?;

  let manifest_path = addon_dir.join("anesis.addon.json");
  let content = std::fs::read_to_string(&manifest_path).with_context(|| {
    format!(
      "Addon '{addon_id}' was extracted but 'anesis.addon.json' was not found at {}. \
       Make sure the addon archive contains a top-level '{addon_id}/' directory with 'anesis.addon.json' inside.",
      manifest_path.display()
    )
  })?;

  let manifest: AddonManifest = super::manifest::parse(&content)
    .with_context(|| format!("Failed to parse anesis.addon.json for addon '{addon_id}'"))?;

  update_addons_cache(addons_dir, addon_id, &manifest, &info.commit_sha)
    .with_context(|| format!("Failed to update addons cache after installing '{addon_id}'"))?;

  Ok(match install_state {
    InstallState::Install => AddonInstallResult::Installed(manifest),
    InstallState::Update => AddonInstallResult::Updated(manifest),
    InstallState::UpToDate => unreachable!("up-to-date addons should return early"),
  })
}

pub async fn check_addon_update(
  client: Client,
  backend_url: String,
  auth_path: PathBuf,
  addon_id: String,
  cached_sha: String,
) -> Option<String> {
  if cached_sha == "local" {
    return None;
  }
  let info = get_addon_url_raw(&client, &backend_url, &auth_path, &addon_id)
    .await
    .ok()?;
  (info.commit_sha != cached_sha).then_some(info.commit_sha)
}

pub async fn fetch_latest_version(ctx: &AppContext, addon_id: &str) -> Result<String> {
  Ok(get_addon_url(ctx, addon_id).await?.version)
}

pub fn read_cached_manifest(addons_dir: &Path, addon_id: &str) -> Result<AddonManifest> {
  let manifest_path = addons_dir.join(addon_id).join("anesis.addon.json");
  let content = std::fs::read_to_string(&manifest_path).with_context(|| {
    format!(
      "Failed to read manifest for addon '{addon_id}' at {}",
      manifest_path.display()
    )
  })?;
  super::manifest::parse(&content)
    .with_context(|| format!("Failed to parse manifest for addon '{addon_id}'"))
}
