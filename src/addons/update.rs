use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{AppContext, auth::token::get_auth_user};

#[derive(Serialize)]
struct UpdateAddonDto {
  url: String,
}

#[derive(Deserialize)]
struct UpdateAddonResponse {
  message: String,
}

pub async fn update_addon(ctx: &AppContext, addon_url: &str) -> Result<()> {
  let user = get_auth_user(&ctx.paths.auth)?;

  let res: UpdateAddonResponse = ctx
    .client
    .patch(format!("{}/addon", ctx.backend_url))
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .json(&UpdateAddonDto {
      url: addon_url.to_string(),
    })
    .send()
    .await?
    .error_for_status()?
    .json()
    .await?;

  println!("✅ {}", res.message);
  Ok(())
}
