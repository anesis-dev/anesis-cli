use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{AppContext, BACKEND_URL, auth::token::get_auth_user};

#[derive(Deserialize, Serialize)]
pub struct PublishTemplateDto {
  pub url: String,
}

pub async fn publish(ctx: &AppContext, template_url: &str) -> Result<()> {
  let user = get_auth_user(&ctx.paths.auth)?;

  let res = ctx
    .client
    .post(format!("{}/template/publish", BACKEND_URL))
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .json(&PublishTemplateDto {
      url: template_url.to_string(),
    })
    .send()
    .await?
    .error_for_status()?;

  let body: serde_json::Value = res.json().await?;
  println!("{}", body);
  Ok(())
}
