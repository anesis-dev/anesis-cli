mod common;

use anyhow::anyhow;
use common::{rerun_prompt_message_for_tests, should_fallback_to_cached_manifest_for_tests};
use oxide_cli::utils::errors::OxideError;

#[test]
fn fallback_to_cached_manifest_when_user_is_not_logged_in() {
  let error = anyhow::Error::from(OxideError::NotLoggedIn);
  assert!(should_fallback_to_cached_manifest_for_tests(&error));
}

#[test]
fn do_not_fallback_to_cached_manifest_for_unrelated_errors() {
  let error = anyhow!("oxide.addon.json is malformed");
  assert!(!should_fallback_to_cached_manifest_for_tests(&error));
}

#[test]
fn rerun_prompt_message_is_none_when_versions_match() {
  let prompt = rerun_prompt_message_for_tests("install", Some("1.0.0"), "1.0.0");
  assert!(prompt.is_none());
}

#[test]
fn rerun_prompt_message_mentions_both_versions_when_version_changed() {
  let prompt = rerun_prompt_message_for_tests("install", Some("1.0.0"), "1.1.0");
  assert_eq!(
    prompt.as_deref(),
    Some(
      "Command 'install' was last run with v1.0.0 of this add-on. A new version (v1.1.0) is available. Re-run it now?"
    )
  );
}

#[test]
fn rerun_prompt_message_is_none_when_no_prior_version_recorded() {
  // None means the command has never been executed → no re-run prompt needed.
  let prompt = rerun_prompt_message_for_tests("install", None, "1.0.0");
  assert!(
    prompt.is_none(),
    "should not prompt to re-run on a fresh install"
  );
}

#[test]
fn should_fallback_for_http_unauthorized() {
  let error = anyhow::Error::from(OxideError::HttpUnauthorized);
  assert!(should_fallback_to_cached_manifest_for_tests(&error));
}

#[test]
fn should_not_fallback_for_network_connect_oxide_error() {
  // OxideError::NetworkConnect is NOT in the fallback list — the fallback only
  // checks for raw reqwest::Error is_connect(), not the wrapped enum variant.
  let error = anyhow::Error::from(OxideError::NetworkConnect);
  assert!(!should_fallback_to_cached_manifest_for_tests(&error));
}

#[test]
fn should_not_fallback_for_http_server_error() {
  let error = anyhow::Error::from(OxideError::HttpServerError("addon".to_string()));
  assert!(!should_fallback_to_cached_manifest_for_tests(&error));
}
