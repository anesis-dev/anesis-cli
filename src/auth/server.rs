use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, anyhow};
use axum::{
  Router,
  extract::{Query, State},
  response::Redirect,
  routing::get,
};
use serde::{Deserialize, Serialize};
use tokio::{
  sync::{Mutex, Notify, oneshot},
  time::Duration,
};

use crate::FRONTEND_URL;

type SharedTx = Arc<Mutex<Option<oneshot::Sender<User>>>>;

#[derive(Serialize, Deserialize)]
pub struct User {
  pub token: String,
  pub name: String,
}

pub async fn run_local_auth_server() -> Result<User> {
  let notify = Arc::new(Notify::new());
  let notify_clone = notify.clone();
  let (tx, rx) = oneshot::channel::<User>();

  let shared_tx: SharedTx = Arc::new(Mutex::new(Some(tx)));

  let app = Router::new()
    .route("/callback", get(callback))
    .with_state(shared_tx);

  let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;

  let server = axum::serve(listener, app).with_graceful_shutdown(async move {
    notify_clone.notified().await;
  });

  tokio::select! {
    result = server => {
      result?;
      Err(anyhow!("Server stopped unexpectedly"))
    }
    user = rx => {
      notify.notify_one();
      Ok(user?)
    }
    _ = tokio::time::sleep(Duration::from_secs(300)) => {
      notify.notify_one();
      Err(anyhow!("Login timed out after 5 minutes. Please try again."))
    }
  }
}

async fn callback(
  State(shared_tx): State<SharedTx>,
  Query(params): Query<HashMap<String, String>>,
) -> Redirect {
  if let Some(token) = params.get("token")
    && let Some(user_name) = params.get("name")
  {
    let mut guard = shared_tx.lock().await;

    if let Some(tx) = guard.take() {
      let _ = tx.send(User {
        name: user_name.to_string(),
        token: token.to_string(),
      });
    }

    Redirect::to(&format!("{}/cli/success", FRONTEND_URL))
  } else {
    Redirect::to(&format!("{}/cli/error", FRONTEND_URL))
  }
}
