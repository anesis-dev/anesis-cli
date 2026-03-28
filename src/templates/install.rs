use std::path::Path;

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{
  AppContext, BACKEND_URL,
  auth::token::get_auth_user,
  cache::{is_template_installed, update_templates_cache},
  utils::git::download_dir,
};

#[derive(Serialize)]
struct GetTemplateUrl {
  template_name: String,
}

#[derive(Deserialize)]
struct GetTemplateUrlRes {
  url: String,
}

async fn get_template_url(
  template_name: &str,
  client: &Client,
  auth_path: &Path,
) -> Result<String> {
  let user = get_auth_user(auth_path)?;

  let res: GetTemplateUrlRes = client
    .post(format!("{}/template/url", BACKEND_URL))
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .body(serde_json::to_string(&GetTemplateUrl {
      template_name: template_name.to_string(),
    })?)
    .send()
    .await?
    .error_for_status()?
    .json()
    .await?;

  Ok(res.url)
}

pub async fn install_template(ctx: &AppContext, template_name: &str) -> Result<()> {
  let template_path = &ctx.paths.templates;
  let is_installed = is_template_installed(ctx, template_name)?;

  if !is_installed {
    let url = get_template_url(template_name, &ctx.client, &ctx.paths.auth).await?;

    let path: &Path = Path::new(template_name);

    let cleanup_path = template_path.join(path);

    {
      let mut guard = ctx.cleanup_state.lock().unwrap();
      *guard = Some(cleanup_path.clone());
    }

    download_dir(&ctx.client, &url, &template_path.join(path)).await?;

    {
      let mut guard = ctx.cleanup_state.lock().unwrap();
      *guard = None;
    }

    update_templates_cache(template_path, path)?;
    println!("Template successfully downloaded");

    Ok(())
  } else {
    println!("Template '{}' is already installed", template_name);
    Ok(())
  }
}
