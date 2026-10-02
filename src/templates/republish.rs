use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{
  auth::token::get_auth_user,
  context::AppContext,
  utils::{
    errors::check_response,
    ui::{self, spinner},
  },
};

#[derive(Deserialize, Serialize)]
pub struct RepublishTemplateDto {
  pub url: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub visibility: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub repo_credential_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub organization_id: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct RepublishTemplateResponse {
  pub message: String,
}

pub async fn republish(
  ctx: &AppContext,
  template_url: &str,
  visibility: Option<String>,
  credential_id: Option<String>,
  org_id: Option<String>,
) -> Result<()> {
  let user = get_auth_user(&ctx.paths.auth)?;

  let sp = spinner("Republishing template to registry...");
  let response = ctx
    .client
    .patch(format!("{}/template", ctx.backend_url))
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .json(&RepublishTemplateDto {
      url: template_url.to_string(),
      visibility,
      repo_credential_id: credential_id,
      organization_id: org_id,
    })
    .send()
    .await
    .inspect_err(|_| sp.finish_and_clear())?;
  let response = check_response(response, "template")
    .await
    .inspect_err(|_| sp.finish_and_clear())?;
  let res: RepublishTemplateResponse = response
    .json()
    .await
    .inspect_err(|_| sp.finish_and_clear())?;
  sp.finish_and_clear();

  ui::success(&res.message);
  Ok(())
}
