use std::fmt;

use oxide_cli::utils::errors::OxideError;

// ── OxideError Display ────────────────────────────────────────────────────────

#[test]
fn not_logged_in_display() {
  let err = OxideError::NotLoggedIn;
  assert_eq!(err.to_string(), "You are not logged in.");
}

#[test]
fn http_unauthorized_display() {
  let err = OxideError::HttpUnauthorized;
  assert_eq!(
    err.to_string(),
    "Authentication failed. Your session may have expired."
  );
}

#[test]
fn http_not_found_display() {
  let err = OxideError::HttpNotFound("template 'react-vite'".to_string());
  assert_eq!(err.to_string(), "template 'react-vite' was not found.");
}

#[test]
fn http_server_error_display() {
  let err = OxideError::HttpServerError("template list".to_string());
  assert_eq!(
    err.to_string(),
    "The server returned an error while fetching template list."
  );
}

#[test]
fn network_connect_display() {
  let err = OxideError::NetworkConnect;
  assert_eq!(err.to_string(), "Could not connect to the server.");
}

#[test]
fn network_timeout_display() {
  let err = OxideError::NetworkTimeout;
  assert_eq!(err.to_string(), "The request timed out.");
}

// ── OxideError in anyhow chain ────────────────────────────────────────────────

#[test]
fn oxide_error_wrapped_in_anyhow_is_downcastable() {
  let err: anyhow::Error = OxideError::NotLoggedIn.into();
  assert!(err.downcast_ref::<OxideError>().is_some());
}

#[test]
fn oxide_error_survives_anyhow_context_chain() {
  let err: anyhow::Error = OxideError::HttpUnauthorized.into();
  let wrapped = err.context("fetching user account");
  let found = wrapped
    .chain()
    .any(|c| c.downcast_ref::<OxideError>().is_some());
  assert!(found, "OxideError should be discoverable in the error chain");
}

// ── OxideError is Debug + Display (both trait bounds required by the codebase) ──

#[test]
fn oxide_error_implements_debug() {
  let err = OxideError::HttpNotFound("foo".into());
  let debug = format!("{err:?}");
  assert!(!debug.is_empty());
}

#[test]
fn all_variants_have_non_empty_messages() {
  let variants: Vec<Box<dyn fmt::Display>> = vec![
    Box::new(OxideError::NotLoggedIn),
    Box::new(OxideError::HttpUnauthorized),
    Box::new(OxideError::HttpNotFound("x".into())),
    Box::new(OxideError::HttpServerError("x".into())),
    Box::new(OxideError::NetworkConnect),
    Box::new(OxideError::NetworkTimeout),
  ];

  for v in variants {
    assert!(
      !v.to_string().is_empty(),
      "OxideError variant has empty message: {v}"
    );
  }
}
