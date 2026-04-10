use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{AppContext, auth::token::get_auth_user};

#[derive(Serialize)]
struct PublishAddonDto {
  url: String,
}

#[derive(Deserialize)]
struct PublishAddonResponse {
  message: String,
  addon_id: String,
}

pub async fn publish_addon(ctx: &AppContext, addon_url: &str) -> Result<()> {
  let user = get_auth_user(&ctx.paths.auth)?;

  let res: PublishAddonResponse = ctx
    .client
    .post(format!("{}/addon/publish", ctx.backend_url))
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .json(&PublishAddonDto {
      url: addon_url.to_string(),
    })
    .send()
    .await?
    .error_for_status()?
    .json()
    .await?;

  println!("✅ {}", res.message);
  println!("   Addon: {}", res.addon_id);
  Ok(())
}
