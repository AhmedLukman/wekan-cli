use serde_json::json;
use wekan_cli::client::ClientError;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

use super::{client, register_request};

#[tokio::test]
async fn registration_uses_the_correct_wire_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .and(header("accept", "application/json"))
        .and(header("content-type", "application/json"))
        .and(header("user-agent", "wekan-cli/0.1.0"))
        .and(body_json(json!({
            "username": "alice",
            "password": "correct horse battery staple"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "user-1",
            "token": "server-token",
            "tokenExpires": "2030-01-02T03:04:05Z"
        })))
        .expect(1)
        .mount(&server)
        .await;

    client(&server)
        .register(&register_request())
        .await
        .expect("the matching registration response should decode");
}

#[tokio::test]
async fn registration_redirects_are_not_followed_or_replayed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(307).insert_header("location", "/other-register"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/other-register"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::UnexpectedRedirect { .. }));
}

#[tokio::test]
async fn registration_server_failures_are_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Server { .. }));
}
