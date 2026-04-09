use std::path::Path;

use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

use crate::{
  AppContext, BACKEND_URL,
  auth::token::get_auth_user,
  cache::{get_cached_template, update_templates_cache},
  utils::archive::download_and_extract,
};

#[derive(Deserialize)]
struct TemplateInfoRes {
  archive_url: String,
  commit_sha: String,
  subdir: Option<String>,
}

async fn get_template_info(
  template_name: &str,
  client: &Client,
  auth_path: &Path,
) -> Result<TemplateInfoRes> {
  let user = get_auth_user(auth_path)?;

  let res: TemplateInfoRes = client
    .get(format!("{BACKEND_URL}/template/{template_name}/url"))
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .send()
    .await?
    .error_for_status()?
    .json()
    .await?;

  Ok(res)
}

pub async fn install_template(ctx: &AppContext, template_name: &str) -> Result<()> {
  let info = get_template_info(template_name, &ctx.client, &ctx.paths.auth).await?;

  // Skip download if the cached commit matches the server's latest
  if let Some(cached) = get_cached_template(ctx, template_name)?
    && cached.commit_sha == info.commit_sha
    && ctx.paths.templates.join(template_name).exists()
  {
    println!("Template '{}' is already up to date", template_name);
    return Ok(());
  }

  let dest = ctx.paths.templates.join(template_name);

  {
    let mut guard = ctx.cleanup_state.lock().unwrap();
    *guard = Some(dest.clone());
  }

  download_and_extract(
    &ctx.client,
    &info.archive_url,
    &dest,
    info.subdir.as_deref(),
  )
  .await?;

  {
    let mut guard = ctx.cleanup_state.lock().unwrap();
    *guard = None;
  }

  update_templates_cache(
    &ctx.paths.templates,
    Path::new(template_name),
    &info.commit_sha,
  )?;
  println!("Template successfully downloaded");

  Ok(())
}
