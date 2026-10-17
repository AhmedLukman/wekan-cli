use super::client_from_url;
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};
use wekan_cli::client::{ClientError, users::tokens::DeleteUserTokenRequest};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

fn credential() -> SecretString {
    SecretString::from("admin-token".to_owned())
}

#[test]
fn token_coverage_rows_list_every_parser_option() {
    use clap::CommandFactory;
    let coverage = include_str!("../../docs/api-coverage.md");
    let mut root = wekan_cli::cli::Cli::command();
    root.build();
    let token = root
        .find_subcommand("user")
        .unwrap()
        .find_subcommand("token")
        .unwrap();
    for (verb, operation) in [("create", "post_user"), ("delete", "post_deletetoken")] {
        let row = coverage
            .lines()
            .find(|line| line.contains(&format!("| `{operation}` |")))
            .unwrap();
        assert!(row.contains(&format!("wekan user token {verb} <USER_ID>")));
        assert!(row.ends_with("| ✓ |"));
        for arg in token.find_subcommand(verb).unwrap().get_arguments() {
            if let Some(long) = arg.get_long() {
                assert!(
                    row.contains(&format!("--{long}")),
                    "missing {long} in token {verb}"
                );
            }
        }
    }
}

#[tokio::test]
async fn user_token_requests_preserve_exact_native_contracts() {
    let server = MockServer::start().await;
    let client = client_from_url(&format!("{}/wekan/", server.uri()));
    Mock::given(method("POST"))
        .and(path("/wekan/api/createtoken/user%2Fone"))
        .and(header("authorization", "Bearer admin-token"))
        .and(header("accept", "application/json"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"_id":"user/one", "authToken":"issued-token"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let created = client
        .create_user_token("user/one", &credential())
        .await
        .unwrap();
    assert_eq!(created.user_id, "user/one");
    assert!(created.auth_token.expose_secret() == "issued-token");
    assert!(!format!("{created:?}").contains("issued-token"));
    let requests = server.received_requests().await.unwrap();
    assert!(requests[0].body.is_empty());

    for token in [Some("unchanged token"), None] {
        let body = token.map_or_else(
            || json!({"userId":"u"}),
            |t| json!({"userId":"u","token":t}),
        );
        Mock::given(method("POST"))
            .and(path("/wekan/api/deletetoken"))
            .and(header("authorization", "Bearer admin-token"))
            .and(header("accept", "application/json"))
            .and(header("content-type", "application/json"))
            .and(body_json(body))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"message":"ack unchanged token"})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let request = DeleteUserTokenRequest {
            user_id: "u".into(),
            token: token.map(|s| SecretString::from(s.to_owned())),
        };
        assert!(!format!("{request:?}").contains("unchanged token"));
        let result = client
            .delete_user_token(&request, &credential())
            .await
            .unwrap();
        assert!(result.message.expose_secret() == "ack unchanged token");
        assert!(!format!("{result:?}").contains("unchanged token"));
    }
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
    server.verify().await;
}

async fn response_error(create: bool, response: Value) -> ClientError {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .expect(1)
        .mount(&server)
        .await;
    let client = super::client(&server);
    let error = if create {
        client
            .create_user_token("u", &credential())
            .await
            .unwrap_err()
    } else {
        client
            .delete_user_token(
                &DeleteUserTokenRequest {
                    user_id: "u".into(),
                    token: None,
                },
                &credential(),
            )
            .await
            .unwrap_err()
    };
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    error
}

#[tokio::test]
async fn user_token_responses_are_strict_and_diagnostics_never_include_values() {
    for value in [
        json!({"_id":"u","authToken":"new-secret","extra":{"nested":"new-secret"}}),
        json!({"_id":"u"}),
        json!({"authToken":"new-secret"}),
        json!({"_id":"u","authToken":123}),
        json!({"_id":"u","authToken":null}),
        json!({"_id":"u","authToken":""}),
        json!({"_id":"other-secret","authToken":"new-secret"}),
        json!({"_id":"","authToken":"new-secret"}),
        json!({}),
        json!([]),
    ] {
        let error = response_error(true, value).await;
        assert!(matches!(error, ClientError::Protocol { .. }));
        assert!(!error.to_string().contains("new-secret"));
        assert!(!error.to_string().contains("other-secret"));
    }
    for value in [
        json!({"message":"target-secret","extra":{"nested":true}}),
        json!({}),
        json!({"message":null}),
        json!({"message":42}),
        json!({"message":{"nested":"target-secret"}}),
    ] {
        let error = response_error(false, value).await;
        assert!(matches!(error, ClientError::Protocol { .. }));
        assert!(!error.to_string().contains("target-secret"));
    }
    for create in [false, true] {
        for value in [
            json!({"error":"Forbidden", "reason":"Forbidden", "statusCode":403}),
            json!({"error":"bad-token", "reason":"rejected", "errorType":"Meteor.Error", "isClientSafe":true}),
        ] {
            assert!(matches!(
                response_error(create, value).await,
                ClientError::EmbeddedServer { .. } | ClientError::EmbeddedProtocol { .. }
            ));
        }
        assert!(matches!(
            response_error(
                create,
                json!({"error":"Forbidden", "statusCode":403, "extra":{"nested":true}})
            )
            .await,
            ClientError::Protocol { .. }
        ));
    }
}

#[tokio::test]
async fn user_token_mutations_do_not_retry_or_follow_redirects() {
    for create in [false, true] {
        for status in [200, 302, 503] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(status)
                        .insert_header("location", "/redirect-target")
                        .set_body_string("not json"),
                )
                .expect(1)
                .mount(&server)
                .await;
            let client = super::client(&server);
            if create {
                assert!(client.create_user_token("u", &credential()).await.is_err());
            } else {
                assert!(
                    client
                        .delete_user_token(
                            &DeleteUserTokenRequest {
                                user_id: "u".into(),
                                token: None
                            },
                            &credential()
                        )
                        .await
                        .is_err()
                );
            }
            assert_eq!(server.received_requests().await.unwrap().len(), 1);
        }
    }
}
