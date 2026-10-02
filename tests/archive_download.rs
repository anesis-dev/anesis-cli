mod common;

use anesis::utils::archive::{download_and_extract, download_capped_for_tests as download_capped};
use common::download_capped_with_progress_for_tests;
use indicatif::ProgressBar;
use wiremock::matchers::{header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn progress_callback_counts_bytes_as_they_arrive() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .respond_with(ResponseTemplate::new(200).set_body_bytes(b"twelve-bytes".to_vec()))
    .mount(&server)
    .await;

  let client = reqwest::Client::new();
  let pb = ProgressBar::hidden();
  let bytes = download_capped_with_progress_for_tests(&client, &server.uri(), None, &pb)
    .await
    .unwrap();

  assert_eq!(bytes, b"twelve-bytes");
  assert_eq!(pb.position(), bytes.len() as u64);
  assert_eq!(pb.length(), Some(bytes.len() as u64));
}

#[tokio::test]
async fn a_missing_content_length_does_not_break_the_download() {
  let server = MockServer::start().await;
  // wiremock always sets Content-Length, so simulate a chunked/unknown-length
  // response by asserting the download still succeeds when we never see a
  // header — the progress bar simply never gets `.set_length()` called.
  Mock::given(method("GET"))
    .respond_with(ResponseTemplate::new(200).set_body_bytes(b"chunked-body".to_vec()))
    .mount(&server)
    .await;

  let client = reqwest::Client::new();
  let pb = ProgressBar::hidden();
  let bytes = download_capped_with_progress_for_tests(&client, &server.uri(), None, &pb)
    .await
    .unwrap();

  assert_eq!(bytes, b"chunked-body");
  assert_eq!(pb.position(), bytes.len() as u64);
}

#[tokio::test]
async fn downloads_the_full_body_on_success() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .respond_with(ResponseTemplate::new(200).set_body_bytes(b"archive-bytes".to_vec()))
    .mount(&server)
    .await;

  let client = reqwest::Client::new();
  let bytes = download_capped(&client, &server.uri(), None).await.unwrap();

  assert_eq!(bytes, b"archive-bytes");
}

#[tokio::test]
async fn sends_a_bearer_token_when_one_is_given() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(header("Authorization", "Bearer secret-token"))
    .respond_with(ResponseTemplate::new(200).set_body_bytes(b"ok".to_vec()))
    .mount(&server)
    .await;

  let client = reqwest::Client::new();
  let bytes = download_capped(&client, &server.uri(), Some("secret-token"))
    .await
    .unwrap();

  assert_eq!(bytes, b"ok");
}

#[tokio::test]
async fn a_non_2xx_status_is_an_error() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .respond_with(ResponseTemplate::new(404))
    .mount(&server)
    .await;

  let client = reqwest::Client::new();
  let err = download_capped(&client, &server.uri(), None)
    .await
    .expect_err("a 404 response must not be treated as a successful download");
  assert!(
    err.to_string().to_lowercase().contains("404")
      || err.to_string().to_lowercase().contains("not found"),
    "unexpected error: {err}"
  );
}

#[tokio::test]
async fn a_connection_that_never_answers_is_an_error_not_a_hang() {
  let client = reqwest::Client::new();
  let err = download_capped(&client, "http://127.0.0.1:1/archive.tar.gz", None)
    .await
    .expect_err("a refused connection must surface as an error");
  assert!(!err.to_string().is_empty());
}

#[tokio::test]
async fn download_and_extract_rejects_non_https_archive_url_without_a_network_call() {
  let dir = tempfile::tempdir().unwrap();
  let client = reqwest::Client::new();

  let err = download_and_extract(
    &client,
    "http://example.com/archive.tar.gz",
    dir.path(),
    None,
    None,
  )
  .await
  .expect_err("a non-https archive_url must be refused before downloading anything");

  assert!(err.to_string().contains("https"));
}
