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

type SharedTx = Arc<Mutex<Option<oneshot::Sender<User>>>>;
type AppState = (SharedTx, String, String);

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
  pub token: String,
  pub name: String,
}

pub const CALLBACK_PORTS: &[u16] = &[8080, 8081, 8082, 8083, 8084, 8085, 8086, 8087, 8088, 8089];

pub struct AuthListener {
  listener: tokio::net::TcpListener,
  port: u16,
  expected_state: String,
  frontend_url: String,
}

impl AuthListener {
  pub fn port(&self) -> u16 {
    self.port
  }
}

pub async fn bind_local_auth_server(
  expected_state: String,
  frontend_url: &str,
) -> Result<AuthListener> {
  let mut last_err = None;

  for &port in CALLBACK_PORTS {
    match try_bind(port) {
      Ok(listener) => {
        return Ok(AuthListener {
          listener,
          port,
          expected_state,
          frontend_url: frontend_url.to_string(),
        });
      }
      Err(err) => last_err = Some(err),
    }
  }

  let first = CALLBACK_PORTS.first().copied().unwrap_or(0);
  let last = CALLBACK_PORTS.last().copied().unwrap_or(0);
  Err(anyhow!(
    "Could not start the login callback listener: every port from {first} to {last} is in use. \
     Stop whatever is using them and try again.{}",
    last_err
      .map(|e| format!(" (last error: {e})"))
      .unwrap_or_default()
  ))
}

fn try_bind(port: u16) -> Result<tokio::net::TcpListener> {
  let socket = tokio::net::TcpSocket::new_v4()?;
  #[cfg(not(windows))]
  socket.set_reuseaddr(true)?;
  socket.bind(format!("127.0.0.1:{port}").parse()?)?;
  Ok(socket.listen(1024)?)
}

pub async fn serve_local_auth_server(bound: AuthListener) -> Result<User> {
  let AuthListener {
    listener,
    expected_state,
    frontend_url,
    ..
  } = bound;

  let notify = Arc::new(Notify::new());
  let notify_clone = notify.clone();
  let (tx, rx) = oneshot::channel::<User>();

  let shared_tx: SharedTx = Arc::new(Mutex::new(Some(tx)));
  let state: AppState = (shared_tx, expected_state, frontend_url);

  let app = Router::new()
    .route("/callback", get(callback))
    .with_state(state);

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
    _ = tokio::time::sleep(Duration::from_secs(600)) => {
      notify.notify_one();
      Err(anyhow!("Login timed out after 10 minutes. Please try again."))
    }
  }
}

async fn callback(
  State((shared_tx, expected_state, frontend_url)): State<AppState>,
  Query(params): Query<HashMap<String, String>>,
) -> Redirect {
  match params.get("state") {
    Some(state) if state == &expected_state => {}
    Some(_) => return Redirect::to(&format!("{}/cli/error?reason=invalid_state", frontend_url)),
    None => return Redirect::to(&format!("{}/cli/error?reason=missing_state", frontend_url)),
  }

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

    Redirect::to(&format!("{}/cli/success", frontend_url))
  } else {
    Redirect::to(&format!("{}/cli/error", frontend_url))
  }
}
