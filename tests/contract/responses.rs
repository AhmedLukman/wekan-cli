use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

use secrecy::ExposeSecret;
use serde_json::json;
use wekan_cli::client::{ClientError, ServerUrl, WekanClient};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

use super::{client, login_request, register_request};

#[tokio::test]
async fn login_success_fields_are_decoded() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "user-1",
            "token": "login-token",
            "tokenExpires": "2030-01-02T03:04:05Z"
        })))
        .mount(&server)
        .await;

    let session = client(&server).login(&login_request()).await.unwrap();
    assert_eq!(session.user_id(), "user-1");
    assert_eq!(session.token().expose_secret(), "login-token");
}

#[tokio::test]
async fn login_rejections_preserve_structured_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": "login-failed",
            "reason": "Incorrect username, email address or password."
        })))
        .mount(&server)
        .await;

    let error = client(&server).login(&login_request()).await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::Server {
            status: reqwest::StatusCode::UNAUTHORIZED,
            server_error: Some(ref value),
            server_reason: Some(_),
            retry_after_seconds: None,
        } if value == "login-failed"
    ));
}

#[tokio::test]
async fn login_rate_limits_preserve_retry_after_seconds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "17")
                .set_body_json(json!({
                    "error": "too-many-requests",
                    "reason": "Too many failed login attempts. Try again later."
                })),
        )
        .mount(&server)
        .await;

    let error = client(&server).login(&login_request()).await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::Server {
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            retry_after_seconds: Some(17),
            ..
        }
    ));
}

#[tokio::test]
async fn malformed_login_success_records_that_a_session_was_created() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;

    let error = client(&server).login(&login_request()).await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::Protocol {
            success_status_received: true,
            ..
        }
    ));
}

#[tokio::test]
async fn login_responses_enforce_the_size_limit() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .mount(&server)
        .await;

    let error = client(&server).login(&login_request()).await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::ResponseTooLarge {
            status: reqwest::StatusCode::OK,
            ..
        }
    ));
}

#[tokio::test]
async fn oversized_login_rate_limits_preserve_status_and_retry_after() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "19")
                .set_body_bytes(vec![b'x'; 1024 * 1024 + 1]),
        )
        .mount(&server)
        .await;

    let error = client(&server).login(&login_request()).await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::ResponseTooLarge {
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            retry_after_seconds: Some(19),
            ..
        }
    ));
}

#[tokio::test]
async fn registration_success_fields_are_decoded() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "user-1",
            "token": "server-token",
            "tokenExpires": "2030-01-02T03:04:05Z"
        })))
        .mount(&server)
        .await;

    let session = client(&server).register(&register_request()).await.unwrap();
    assert_eq!(session.user_id(), "user-1");
    assert_eq!(session.token().expose_secret(), "server-token");
    assert_eq!(
        session
            .token_expires()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap(),
        "2030-01-02T03:04:05Z"
    );
}

#[tokio::test]
async fn registration_disabled_accepts_an_empty_error_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(403))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Server {
            status: reqwest::StatusCode::FORBIDDEN,
            server_error: None,
            server_reason: None,
            ..
        }
    ));
}

#[tokio::test]
async fn structured_registration_errors_are_preserved() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "username-already-exists",
            "reason": "Username already exists."
        })))
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    match error {
        ClientError::Server {
            status,
            server_error,
            server_reason,
            ..
        } => {
            assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
            assert_eq!(server_error.as_deref(), Some("username-already-exists"));
            assert_eq!(server_reason.as_deref(), Some("Username already exists."));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn numeric_meteor_error_codes_are_preserved() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": 403,
            "reason": "Username already exists."
        })))
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    match error {
        ClientError::Server {
            status,
            server_error,
            server_reason,
            ..
        } => {
            assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
            assert_eq!(server_error.as_deref(), Some("403"));
            assert_eq!(server_reason.as_deref(), Some("Username already exists."));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn non_json_registration_errors_are_classified_by_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(500).set_body_string("upstream unavailable"))
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Server {
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            server_error: None,
            server_reason: None,
            ..
        }
    ));
}

#[tokio::test]
async fn malformed_success_is_a_possible_partial_mutation() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Protocol {
            success_status_received: true,
            ..
        }
    ));
}

#[tokio::test]
async fn malformed_success_does_not_echo_a_response_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "user-1",
            "token": 123456,
            "tokenExpires": "2030-01-02T03:04:05Z"
        })))
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();

    assert!(!error.to_string().contains("123456"));
    assert!(matches!(
        error,
        ClientError::Protocol {
            success_status_received: true,
            ..
        }
    ));
}

#[tokio::test]
async fn registration_responses_enforce_the_size_limit() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::ResponseTooLarge {
            status: reqwest::StatusCode::OK,
            ..
        }
    ));
}

#[tokio::test]
async fn oversized_registration_errors_preserve_the_http_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(400).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::ResponseTooLarge {
            status: reqwest::StatusCode::BAD_REQUEST,
            ..
        }
    ));
}

#[tokio::test]
async fn truncated_registration_errors_preserve_the_http_status() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request).unwrap();
        stream
            .write_all(
                b"HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{\"error\":",
            )
            .unwrap();
    });
    let server_url = ServerUrl::parse(&format!("http://{address}"), false).unwrap();
    let client = WekanClient::new(server_url).unwrap();

    let error = client.register(&register_request()).await.unwrap_err();
    server.join().unwrap();

    assert!(matches!(
        error,
        ClientError::ResponseBody {
            status: reqwest::StatusCode::BAD_REQUEST,
            ..
        }
    ));
}
