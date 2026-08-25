use serde_json::json;
use wekan_cli::client::ClientError;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

use secrecy::SecretString;
use wekan_cli::client::LoginRequest;

use super::{client, login_request, logout_request, logout_token, register_request, status_token};

#[tokio::test]
async fn current_token_logout_uses_the_authenticated_json_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/logout"))
        .and(header("accept", "application/json"))
        .and(header("content-type", "application/json"))
        .and(header("authorization", "Bearer logout-token"))
        .and(header("user-agent", "wekan-cli/0.1.0"))
        .and(body_json(json!({ "all": false })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "message": "You've been logged out!"
        })))
        .expect(1)
        .mount(&server)
        .await;

    client(&server)
        .logout(&logout_request(false), &logout_token())
        .await
        .expect("current-token logout should succeed");
}

#[tokio::test]
async fn all_tokens_logout_sets_the_all_request_field() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/logout"))
        .and(body_json(json!({ "all": true })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "message": "All login tokens have been invalidated."
        })))
        .expect(1)
        .mount(&server)
        .await;

    client(&server)
        .logout(&logout_request(true), &logout_token())
        .await
        .expect("all-token logout should succeed");
}

#[tokio::test]
async fn logout_redirects_are_not_followed_or_replayed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/logout"))
        .respond_with(ResponseTemplate::new(307).insert_header("location", "/other-logout"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/other-logout"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let error = client(&server)
        .logout(&logout_request(false), &logout_token())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::UnexpectedRedirect { .. }));
}

#[tokio::test]
async fn logout_server_failures_are_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/logout"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .logout(&logout_request(false), &logout_token())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Server { .. }));
}

#[tokio::test]
async fn authentication_status_uses_the_current_user_endpoint_and_bearer_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/user"))
        .and(header("accept", "application/json"))
        .and(header("authorization", "Bearer status-token"))
        .and(header("user-agent", "wekan-cli/0.1.0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "user-1"
        })))
        .expect(1)
        .mount(&server)
        .await;

    client(&server)
        .current_user(&status_token())
        .await
        .expect("the current-user response should decode");
}

#[tokio::test]
async fn authentication_status_redirects_are_not_followed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/user"))
        .respond_with(ResponseTemplate::new(307).insert_header("location", "/other-user"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/other-user"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let error = client(&server)
        .current_user(&status_token())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::UnexpectedRedirect { .. }));
}

#[tokio::test]
async fn authentication_status_server_failures_are_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/user"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .current_user(&status_token())
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Server { .. }));
}

#[tokio::test]
async fn login_with_username_uses_the_correct_wire_request_and_omits_code() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
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
        .login(&login_request())
        .await
        .expect("the matching login response should decode");
}

#[tokio::test]
async fn login_with_email_and_two_factor_code_uses_the_correct_wire_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .and(body_json(json!({
            "email": "alice@example.com",
            "password": "test-password",
            "code": "123456"
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
        .login(&LoginRequest {
            username: None,
            email: Some("alice@example.com".to_owned()),
            password: SecretString::from("test-password".to_owned()),
            code: Some(SecretString::from("123456".to_owned())),
        })
        .await
        .expect("email and two-factor login should decode");
}

#[tokio::test]
async fn login_redirects_are_not_followed_or_replayed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(307).insert_header("location", "/other-login"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/other-login"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let error = client(&server).login(&login_request()).await.unwrap_err();
    assert!(matches!(error, ClientError::UnexpectedRedirect { .. }));
}

#[tokio::test]
async fn login_server_failures_are_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server).login(&login_request()).await.unwrap_err();
    assert!(matches!(error, ClientError::Server { .. }));
}

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
