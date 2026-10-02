use thiserror::Error;

use crate::utils::ui;

#[derive(Debug, Error)]
pub enum AnesisError {
  #[error("You are not logged in.")]
  NotLoggedIn,
  #[error("Your session has expired.")]
  SessionExpired,
  #[error("Authentication failed. Your session may have expired.")]
  HttpUnauthorized,
  #[error("{0} was not found.")]
  HttpNotFound(String),
  #[error("The server returned an error while fetching {0}.")]
  HttpServerError(String),
  #[error("Could not connect to the server.")]
  NetworkConnect,
  #[error("The request timed out.")]
  NetworkTimeout,
  #[error("{0} needs an interactive terminal.")]
  NotATerminal(String),
  #[error("Aborted. No changes were made.")]
  Aborted,
}

pub mod exit_code {
  pub const FAILURE: i32 = 1;
  pub const USAGE: i32 = 2;
  pub const AUTH: i32 = 3;
  pub const NETWORK: i32 = 4;
  pub const NOT_FOUND: i32 = 5;
  pub const NOT_A_TERMINAL: i32 = 6;
  pub const ABORTED: i32 = 7;
}

pub fn exit_code_for(err: &anyhow::Error) -> i32 {
  for cause in err.chain() {
    if let Some(anesis_err) = cause.downcast_ref::<AnesisError>() {
      return match anesis_err {
        AnesisError::NotLoggedIn | AnesisError::SessionExpired | AnesisError::HttpUnauthorized => {
          exit_code::AUTH
        }
        AnesisError::HttpNotFound(_) => exit_code::NOT_FOUND,
        AnesisError::HttpServerError(_)
        | AnesisError::NetworkConnect
        | AnesisError::NetworkTimeout => exit_code::NETWORK,
        AnesisError::NotATerminal(_) => exit_code::NOT_A_TERMINAL,
        AnesisError::Aborted => exit_code::ABORTED,
      };
    }

    if let Some(reqwest_err) = cause.downcast_ref::<reqwest::Error>() {
      if reqwest_err
        .status()
        .is_some_and(|s| s == reqwest::StatusCode::NOT_FOUND)
      {
        return exit_code::NOT_FOUND;
      }
      if reqwest_err.status().is_some_and(|s| {
        s == reqwest::StatusCode::UNAUTHORIZED || s == reqwest::StatusCode::FORBIDDEN
      }) {
        return exit_code::AUTH;
      }
      return exit_code::NETWORK;
    }

    if is_not_a_tty(cause) {
      return exit_code::NOT_A_TERMINAL;
    }
  }

  exit_code::FAILURE
}

fn is_not_a_tty(cause: &(dyn std::error::Error + 'static)) -> bool {
  matches!(
    cause.downcast_ref::<inquire::InquireError>(),
    Some(inquire::InquireError::NotTTY)
  )
}

pub fn classify_reqwest_error(err: reqwest::Error, resource: &str) -> anyhow::Error {
  if err.is_connect() {
    return AnesisError::NetworkConnect.into();
  }
  if err.is_timeout() {
    return AnesisError::NetworkTimeout.into();
  }
  if let Some(status) = err.status() {
    return classify_status(status, resource).into();
  }
  anyhow::anyhow!("Network error while fetching {resource}")
}

fn classify_status(status: reqwest::StatusCode, resource: &str) -> AnesisError {
  if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
    AnesisError::HttpUnauthorized
  } else if status == reqwest::StatusCode::NOT_FOUND {
    AnesisError::HttpNotFound(resource.to_string())
  } else {
    AnesisError::HttpServerError(resource.to_string())
  }
}

#[derive(serde::Deserialize)]
struct ServerErrorBody {
  message: String,
}

pub async fn check_response(
  response: reqwest::Response,
  resource: &str,
) -> Result<reqwest::Response, anyhow::Error> {
  if response.status().is_success() {
    return Ok(response);
  }

  let status = response.status();
  let body_text = response.text().await.unwrap_or_default();
  let server_message = serde_json::from_str::<ServerErrorBody>(&body_text)
    .ok()
    .map(|b| b.message)
    .filter(|m| !m.trim().is_empty());

  let base: anyhow::Error = classify_status(status, resource).into();
  Err(match server_message {
    Some(msg) => base.context(msg),
    None => base,
  })
}

pub fn print_error(err: &anyhow::Error) {
  if std::env::var("ANESIS_DEBUG").is_ok() {
    ui::error_header(format!("{err:?}"));
    return;
  }

  for cause in err.chain() {
    if let Some(anesis_err) = cause.downcast_ref::<AnesisError>() {
      ui::error_header(err);
      if let Some(hint) = hint_for_anesis_error(anesis_err) {
        ui::hint_line(hint);
      }
      return;
    }
  }

  for cause in err.chain() {
    if is_not_a_tty(cause) {
      ui::error_header("This command needs an interactive terminal, and there isn't one.");
      ui::hint_line(
        "Pass `--yes` to accept defaults, and `--input NAME=VALUE` for each value \
         the command would have asked for.",
      );
      return;
    }
  }

  for cause in err.chain() {
    if let Some(reqwest_err) = cause.downcast_ref::<reqwest::Error>() {
      if err.downcast_ref::<reqwest::Error>().is_some() {
        ui::error_header(friendly_reqwest_message(reqwest_err));
      } else {
        ui::error_header(err);
      }
      if let Some(hint) = hint_for_reqwest_error(reqwest_err) {
        ui::hint_line(hint);
      }
      return;
    }
  }

  ui::error_header(err);
}

fn hint_for_anesis_error(err: &AnesisError) -> Option<&'static str> {
  match err {
    AnesisError::NotLoggedIn => Some("Run `anesis login` to authenticate."),
    AnesisError::SessionExpired => Some("Run `anesis login` to re-authenticate."),
    AnesisError::HttpUnauthorized => Some("Run `anesis login` to re-authenticate."),
    AnesisError::HttpNotFound(_) => Some("Check the name is correct and that you have access."),
    AnesisError::HttpServerError(_) => {
      Some("This is likely a temporary issue. Try again in a moment.")
    }
    AnesisError::NetworkConnect => Some("Check your internet connection and try again."),
    AnesisError::NetworkTimeout => {
      Some("The server may be starting up (cold start). Wait a moment and try again.")
    }
    AnesisError::NotATerminal(_) => Some(
      "Pass `--yes` to accept defaults, and `--input NAME=VALUE` for each value the addon needs.",
    ),
    AnesisError::Aborted => None,
  }
}

fn friendly_reqwest_message(err: &reqwest::Error) -> String {
  if err.is_connect() {
    return "Could not connect to the server.".to_string();
  }
  if err.is_timeout() {
    return "The request timed out.".to_string();
  }
  if let Some(status) = err.status() {
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
      return "Authentication failed. Your session may have expired.".to_string();
    }
    if status == reqwest::StatusCode::NOT_FOUND {
      return "The requested resource was not found.".to_string();
    }
    if status.is_server_error() {
      return "The server returned an error. This is likely a temporary issue.".to_string();
    }
  }
  "An unexpected network error occurred.".to_string()
}

fn hint_for_reqwest_error(err: &reqwest::Error) -> Option<&'static str> {
  if err.is_connect() {
    return Some("Check your internet connection and try again.");
  }
  if err.is_timeout() {
    return Some("The server may be starting up (cold start). Wait a moment and try again.");
  }
  if err
    .status()
    .is_some_and(|s| s == reqwest::StatusCode::UNAUTHORIZED || s == reqwest::StatusCode::FORBIDDEN)
  {
    return Some("Run `anesis login` to re-authenticate.");
  }
  None
}
