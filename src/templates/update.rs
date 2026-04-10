use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{AppContext, auth::token::get_auth_user};

#[derive(Deserialize, Serialize)]
pub struct UpdateTemplateDto {
  pub url: String,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateTemplateRsponse {
  pub message: String,
}

pub async fn update(ctx: &AppContext, template_url: &str) -> Result<()> {
  let user = get_auth_user(&ctx.paths.auth)?;

  let res: UpdateTemplateRsponse = ctx
    .client
    .patch(format!("{}/template", ctx.backend_url))
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .json(&UpdateTemplateDto {
      url: template_url.to_string(),
    })
    .send()
    .await?
    .error_for_status()?
    .json()
    .await?;

  println!("✅ {}", res.message);
  Ok(())
}
