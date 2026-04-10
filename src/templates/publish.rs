use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{AppContext, auth::token::get_auth_user};

#[derive(Deserialize, Serialize)]
pub struct PublishTemplateDto {
  pub url: String,
}

#[derive(Deserialize)]
struct PublishTemplateResponse {
  message: String,
  name: String,
}

pub async fn publish(ctx: &AppContext, template_url: &str) -> Result<()> {
  let user = get_auth_user(&ctx.paths.auth)?;

  let res: PublishTemplateResponse = ctx
    .client
    .post(format!("{}/template/publish", ctx.backend_url))
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .json(&PublishTemplateDto {
      url: template_url.to_string(),
    })
    .send()
    .await?
    .error_for_status()?
    .json()
    .await?;

  println!("✅ {}", res.message);
  println!("   Template: {}", res.name);
  Ok(())
}
