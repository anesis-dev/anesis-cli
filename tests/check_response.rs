use anesis::utils::errors::check_response;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn surfaces_the_server_message_on_a_client_error() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
      "status": 409,
      "error": "CONFLICT",
      "message": "addon 'acme/tool' already belongs to another owner"
    })))
    .mount(&server)
    .await;

  let response = reqwest::get(server.uri()).await.unwrap();
  let err = check_response(response, "addon").await.unwrap_err();

  assert_eq!(
    err.to_string(),
    "addon 'acme/tool' already belongs to another owner"
  );
}

#[tokio::test]
async fn falls_back_to_a_classified_message_when_body_has_no_message_field() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .respond_with(ResponseTemplate::new(404).set_body_string("not json at all"))
    .mount(&server)
    .await;

  let response = reqwest::get(server.uri()).await.unwrap();
  let err = check_response(response, "template").await.unwrap_err();

  assert!(err.to_string().contains("template"));
  assert!(err.to_string().to_lowercase().contains("not found"));
}

#[tokio::test]
async fn success_status_passes_the_response_through_unchanged() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"message": "ok"})))
    .mount(&server)
    .await;

  let response = reqwest::get(server.uri()).await.unwrap();
  let response = check_response(response, "template").await.unwrap();

  assert_eq!(response.status(), reqwest::StatusCode::OK);
}

#[tokio::test]
async fn whitespace_only_message_falls_back_to_the_classified_message() {
  let server = MockServer::start().await;
  Mock::given(method("GET"))
    .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({"message": "   "})))
    .mount(&server)
    .await;

  let response = reqwest::get(server.uri()).await.unwrap();
  let err = check_response(response, "addon catalog").await.unwrap_err();

  assert!(
    err.to_string().contains("addon catalog"),
    "a whitespace-only server message must not replace the classified fallback: {err}"
  );
}
