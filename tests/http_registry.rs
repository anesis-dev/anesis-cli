mod common;

use anesis::addons::install::install_addon;
use anesis::templates::install::install_template;
use anesis::utils::errors::AnesisError;
use anesis::utils::http::{MAX_TOTAL_PAGES_FOR_TESTS, fetch_paginated};
use common::fixture::Fixture;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn fetch_paginated_returns_a_single_page_unmodified() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/addon/all"))
    .and(query_param("page", "1"))
    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
      "data": ["a", "b"],
      "total_pages": 1
    })))
    .mount(&server)
    .await;

  let fx = Fixture::new();
  let ctx = fx.mock_ctx(&server);
  let items: Vec<String> = fetch_paginated(&ctx, "/addon/all", "addon catalog")
    .await
    .unwrap();

  assert_eq!(items, vec!["a".to_string(), "b".to_string()]);
}

#[tokio::test]
async fn fetch_paginated_aggregates_every_page() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/addon/all"))
    .and(query_param("page", "1"))
    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
      "data": ["a"],
      "total_pages": 3
    })))
    .mount(&server)
    .await;
  Mock::given(method("GET"))
    .and(path("/addon/all"))
    .and(query_param("page", "2"))
    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
      "data": ["b"],
      "total_pages": 3
    })))
    .mount(&server)
    .await;
  Mock::given(method("GET"))
    .and(path("/addon/all"))
    .and(query_param("page", "3"))
    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
      "data": ["c"],
      "total_pages": 3
    })))
    .mount(&server)
    .await;

  let fx = Fixture::new();
  let ctx = fx.mock_ctx(&server);
  let mut items: Vec<String> = fetch_paginated(&ctx, "/addon/all", "addon catalog")
    .await
    .unwrap();
  items.sort();

  assert_eq!(
    items,
    vec!["a".to_string(), "b".to_string(), "c".to_string()]
  );
}

#[tokio::test]
async fn fetch_paginated_rejects_a_server_reporting_too_many_pages() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/addon/all"))
    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
      "data": ["a"],
      "total_pages": MAX_TOTAL_PAGES_FOR_TESTS + 1
    })))
    .mount(&server)
    .await;

  let fx = Fixture::new();
  let ctx = fx.mock_ctx(&server);
  let err = fetch_paginated::<String>(&ctx, "/addon/all", "addon catalog")
    .await
    .expect_err("a page count above the limit must be rejected through the real function");

  assert!(err.to_string().contains("page"), "{err}");
}

#[tokio::test]
async fn install_template_404_is_classified_as_not_found() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/template/react-vite-ts/url"))
    .respond_with(ResponseTemplate::new(404))
    .mount(&server)
    .await;

  let fx = Fixture::new();
  let ctx = fx.mock_ctx(&server);
  let err = install_template(&ctx, "react-vite-ts").await.unwrap_err();

  assert!(
    err
      .downcast_ref::<AnesisError>()
      .is_some_and(|e| matches!(e, AnesisError::HttpNotFound(_))),
    "expected HttpNotFound, got: {err}"
  );
}

#[tokio::test]
async fn install_template_401_is_classified_as_unauthorized() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/template/react-vite-ts/url"))
    .respond_with(ResponseTemplate::new(401))
    .mount(&server)
    .await;

  let fx = Fixture::new();
  let ctx = fx.mock_ctx(&server);
  let err = install_template(&ctx, "react-vite-ts").await.unwrap_err();

  assert!(
    err
      .downcast_ref::<AnesisError>()
      .is_some_and(|e| matches!(e, AnesisError::HttpUnauthorized)),
    "expected HttpUnauthorized, got: {err}"
  );
}

#[tokio::test]
async fn install_template_malformed_json_body_is_a_parse_error() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/template/react-vite-ts/url"))
    .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
    .mount(&server)
    .await;

  let fx = Fixture::new();
  let ctx = fx.mock_ctx(&server);
  let err = install_template(&ctx, "react-vite-ts").await.unwrap_err();

  assert!(
    err.to_string().contains("Failed to parse"),
    "unexpected error: {err}"
  );
}

#[tokio::test]
async fn install_template_a_valid_response_reaches_the_https_only_download_gate() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/template/react-vite-ts/url"))
    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
      "archive_url": format!("{}/archive.tar.gz", server.uri()),
      "commit_sha": "abc123",
      "subdir": null
    })))
    .mount(&server)
    .await;

  let fx = Fixture::new();
  let ctx = fx.mock_ctx(&server);
  let err = install_template(&ctx, "react-vite-ts").await.unwrap_err();

  assert!(
    err.to_string().contains("https"),
    "expected the download stage's https gate to be what stops this, got: {err}"
  );
}

#[tokio::test]
async fn install_template_update_leaves_the_old_cached_template_untouched_on_download_failure() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/template/react-vite-ts/url"))
    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
      "archive_url": format!("http://{}/archive.tar.gz", server.address()),
      "commit_sha": "new-sha",
      "subdir": null
    })))
    .mount(&server)
    .await;

  let fx = Fixture::new();
  let paths = fx.paths();
  paths.ensure_directories().unwrap();
  let template_dir = paths.templates.join("react-vite-ts");
  std::fs::create_dir_all(&template_dir).unwrap();
  std::fs::write(template_dir.join("marker.txt"), "old-content").unwrap();
  std::fs::write(
    template_dir.join("anesis.template.json"),
    serde_json::to_string(&common::fixture::build::template_manifest(
      "react-vite-ts",
      "1.0.0",
    ))
    .unwrap(),
  )
  .unwrap();
  anesis::templates::cache::update_templates_cache(
    &paths.templates,
    std::path::Path::new("react-vite-ts"),
    "old-sha",
  )
  .unwrap();

  let ctx = fx.mock_ctx(&server);
  let err = install_template(&ctx, "react-vite-ts").await.unwrap_err();
  assert!(
    format!("{err:#}").contains("https"),
    "expected the download stage's https gate to fail this update: {err:#}"
  );

  assert!(
    template_dir.join("marker.txt").exists(),
    "a failed update download must leave the previously-cached template on disk"
  );
  assert_eq!(
    std::fs::read_to_string(template_dir.join("marker.txt")).unwrap(),
    "old-content"
  );
}

#[tokio::test]
async fn install_template_skips_the_network_entirely_for_a_locally_linked_template() {
  let server = MockServer::start().await;

  let fx = Fixture::new();
  let paths = fx.paths();
  paths.ensure_directories().unwrap();
  let template_dir = paths.templates.join("linked-template");
  std::fs::create_dir_all(&template_dir).unwrap();
  std::fs::write(
    template_dir.join("anesis.template.json"),
    serde_json::to_string(&common::fixture::build::template_manifest(
      "linked-template",
      "1.0.0",
    ))
    .unwrap(),
  )
  .unwrap();
  anesis::templates::cache::update_templates_cache(
    &paths.templates,
    std::path::Path::new("linked-template"),
    "local",
  )
  .unwrap();

  let ctx = fx.mock_ctx(&server);
  let result = install_template(&ctx, "linked-template").await.unwrap();
  assert_eq!(result, anesis::templates::install::InstallResult::UpToDate);
}

#[tokio::test]
async fn install_addon_404_is_classified_as_not_found() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/addon/docker-compose/url"))
    .respond_with(ResponseTemplate::new(404))
    .mount(&server)
    .await;

  let fx = Fixture::new();
  let ctx = fx.mock_ctx(&server);
  let err = install_addon(&ctx, "docker-compose").await.unwrap_err();

  assert!(
    err
      .downcast_ref::<AnesisError>()
      .is_some_and(|e| matches!(e, AnesisError::HttpNotFound(_))),
    "expected HttpNotFound, got: {err}"
  );
}

#[tokio::test]
async fn install_addon_a_valid_response_reaches_the_https_only_download_gate() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .and(path("/addon/docker-compose/url"))
    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
      "archive_url": format!("{}/archive.tar.gz", server.uri()),
      "commit_sha": "abc123",
      "subdir": null,
      "version": "1.0.0"
    })))
    .mount(&server)
    .await;

  let fx = Fixture::new();
  let ctx = fx.mock_ctx(&server);
  let err = install_addon(&ctx, "docker-compose").await.unwrap_err();

  assert!(
    format!("{err:#}").contains("https"),
    "expected the download stage's https gate to be what stops this, got: {err:#}"
  );
}
