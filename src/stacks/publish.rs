use anyhow::Result;
use serde::Serialize;

use crate::{
  auth::token::get_auth_user,
  context::AppContext,
  utils::{
    errors::check_response,
    ui::{self, spinner},
  },
};

#[derive(Serialize)]
struct PublishStackDto {
  url: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  visibility: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  repo_credential_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  organization_id: Option<String>,
}

pub async fn publish_stack(
  ctx: &AppContext,
  stack_url: &str,
  update: bool,
  visibility: Option<String>,
  credential_id: Option<String>,
  org_id: Option<String>,
) -> Result<()> {
  let user = get_auth_user(&ctx.paths.auth)?;
  let (method, verb) = if update {
    (
      ctx.client.patch(format!("{}/stack/", ctx.backend_url)),
      "Updating",
    )
  } else {
    (
      ctx
        .client
        .post(format!("{}/stack/publish", ctx.backend_url)),
      "Publishing",
    )
  };

  let sp = spinner(format!("{verb} stack to registry..."));
  let response = method
    .bearer_auth(user.token)
    .header("Content-Type", "application/json")
    .json(&PublishStackDto {
      url: stack_url.to_string(),
      visibility,
      repo_credential_id: credential_id,
      organization_id: org_id,
    })
    .send()
    .await
    .inspect_err(|_| sp.finish_and_clear())?;
  let res = check_response(response, "stack")
    .await
    .inspect_err(|_| sp.finish_and_clear())?;
  sp.finish_and_clear();

  let body: serde_json::Value = res.json().await.unwrap_or_default();
  let message = body
    .get("message")
    .and_then(|m| m.as_str())
    .unwrap_or("Done");
  ui::success(message);
  if let Some(id) = body.get("stack_id").and_then(|m| m.as_str()) {
    println!("   Stack: {id}");
  }
  Ok(())
}
