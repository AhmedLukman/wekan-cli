use super::{client, client_from_url, user_token};
use serde_json::json;
use std::{io::Cursor, time::Duration};
use wekan_cli::client::{
    ClientError,
    boards::{
        import::BoardImportRequest,
        trello_import::{TrelloImportAuth, TrelloImportBoardName},
    },
    imports::PreparedImportRequest,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_bytes, body_json, header, method, path},
};

fn document() -> PreparedImportRequest<BoardImportRequest> {
    PreparedImportRequest::new(&BoardImportRequest {
        board: json!({"name":"Delivery","extension":{"keep":true}}),
        members_mapping: Some([("source".into(), "local".into())].into()),
    })
    .unwrap()
}

#[tokio::test]
async fn exact_requests_preserve_prefix_content_mapping_bytes_and_each_auth_form() {
    let server = MockServer::start().await;
    let token = secrecy::SecretString::from("token ;&=+/%?".to_owned());
    for (auth, name, value) in [
        (
            TrelloImportAuth::Bearer,
            "authorization",
            "Bearer token ;&=+/%?",
        ),
        (
            TrelloImportAuth::XAuthToken,
            "x-auth-token",
            "token ;&=+/%?",
        ),
        (
            TrelloImportAuth::MeteorCookie,
            "cookie",
            "meteor_login_token=token%20%3B%26%3D%2B%2F%25%3F",
        ),
        (
            TrelloImportAuth::WekanCookie,
            "cookie",
            "wekan_login_token=token%20%3B%26%3D%2B%2F%25%3F",
        ),
    ] {
        server.reset().await;
        Mock::given(method("POST")).and(path("/wekan/import-trello"))
            .and(header(name, value)).and(header("accept", "application/json"))
            .and(header("content-type", "application/json"))
            .and(body_json(json!({"board":{"name":"Delivery","extension":{"keep":true}},"membersMapping":{"source":"local"}})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"boardIds":["b"]}))).expect(1).mount(&server).await;
        let result = client_from_url(&format!("{}/wekan/", server.uri()))
            .import_trello_json(document(), &token, auth, None)
            .await
            .unwrap();
        assert_eq!(result.board_ids, ["b"]);
        assert!(result.errors.is_none());
        let requests = server.received_requests().await.unwrap();
        let request = &requests[0];
        assert!(request.url.query().is_none());
        for other in ["authorization", "x-auth-token", "cookie"] {
            assert_eq!(request.headers.contains_key(other), other == name);
        }
        server.verify().await;
        server.reset().await;
        let bytes = vec![b'P', b'K', 0, 255, 13, 10];
        Mock::given(method("POST"))
            .and(path("/wekan/import-trello"))
            .and(header(name, value))
            .and(header("content-type", "application/zip"))
            .and(header("content-length", "6"))
            .and(body_bytes(bytes.clone()))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"boardIds":["b"],"errors":[]})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let result = client_from_url(&format!("{}/wekan/", server.uri()))
            .import_trello_zip(Cursor::new(bytes), 6, &token, auth, None)
            .await
            .unwrap();
        assert_eq!(result.errors, Some(vec![]));
        let request = server.received_requests().await.unwrap().remove(0);
        assert!(request.url.query().is_none());
        assert!(!request.headers.contains_key("transfer-encoding"));
        server.verify().await;
    }
}

#[tokio::test]
async fn response_shapes_are_strict_and_scalar_failed_names_keep_their_types() {
    let server = MockServer::start().await;
    for (zip, value) in [
        (false, json!({})),
        (false, json!({"boardIds":null})),
        (false, json!({"boardIds":[1]})),
        (false, json!({"boardIds":[],"errors":[]})),
        (false, json!({"boardIds":[],"unmapped":true})),
        (true, json!({"boardIds":[]})),
        (true, json!({"errors":[]})),
        (true, json!({"boardIds":[],"errors":null})),
        (true, json!({"boardIds":[],"errors":[],"unmapped":true})),
        (
            true,
            json!({"boardIds":[],"errors":[{"board":"b","error":"bad","unmapped":true}]}),
        ),
        (true, json!({"boardIds":[],"errors":[{"error":"bad"}]})),
        (true, json!({"boardIds":[],"errors":[{"board":"b"}]})),
        (
            true,
            json!({"boardIds":[],"errors":[{"board":"b","error":1}]}),
        ),
        (
            true,
            json!({"boardIds":[],"errors":[{"board":null,"error":"bad"}]}),
        ),
        (
            true,
            json!({"boardIds":[],"errors":[{"board":{"unmapped":true},"error":"bad"}]}),
        ),
        (
            true,
            json!({"boardIds":[],"errors":[{"board":[],"error":"bad"}]}),
        ),
    ] {
        server.reset().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&value))
            .mount(&server)
            .await;
        let result = if zip {
            client(&server)
                .import_trello_zip(
                    Cursor::new(vec![0]),
                    1,
                    &user_token(),
                    TrelloImportAuth::Bearer,
                    None,
                )
                .await
        } else {
            client(&server)
                .import_trello_json(document(), &user_token(), TrelloImportAuth::Bearer, None)
                .await
        };
        assert!(
            matches!(result, Err(ClientError::Protocol { .. })),
            "accepted {value}"
        );
    }
    server.reset().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"boardIds":["good"],"errors":[{"board":"bad","error":"failed"},{"board":42,"error":"failed"},{"board":true,"error":"failed"}]}))).mount(&server).await;
    let result = client(&server)
        .import_trello_zip(
            Cursor::new(vec![0]),
            1,
            &user_token(),
            TrelloImportAuth::Bearer,
            None,
        )
        .await
        .unwrap();
    let errors = result.errors.unwrap();
    assert!(matches!(errors[0].board, TrelloImportBoardName::String(_)));
    assert!(matches!(errors[1].board, TrelloImportBoardName::Number(_)));
    assert_eq!(errors[2].board, TrelloImportBoardName::Boolean(true));
}

#[tokio::test]
async fn failures_preserve_native_errors_reject_unknown_error_fields_and_never_retry() {
    let server = MockServer::start().await;
    for status in [400, 401, 413, 500] {
        server.reset().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(status).set_body_json(json!({"error":"native-error"})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let error = client(&server)
            .import_trello_json(document(), &user_token(), TrelloImportAuth::Bearer, None)
            .await
            .unwrap_err();
        assert!(
            matches!(error, ClientError::Server { status: s, server_error: Some(ref e), .. } if s.as_u16() == status && e == "native-error")
        );
        server.verify().await;
    }
    for body in [
        json!({"error":"bad","unmapped":true}),
        json!({"error":{"nested":true}}),
        json!({}),
        json!({"error":null}),
        json!({"error":42}),
    ] {
        server.reset().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(400).set_body_json(body))
            .mount(&server)
            .await;
        assert!(matches!(
            client(&server)
                .import_trello_json(document(), &user_token(), TrelloImportAuth::Bearer, None)
                .await,
            Err(ClientError::Protocol { .. })
        ));
    }
    server.reset().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_raw("<!DOCTYPE html>", "text/html"))
        .mount(&server)
        .await;
    assert!(matches!(
        client(&server)
            .import_trello_json(document(), &user_token(), TrelloImportAuth::Bearer, None)
            .await,
        Err(ClientError::Server { .. })
    ));
    server.reset().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        client(&server)
            .import_trello_json(
                document(),
                &user_token(),
                TrelloImportAuth::Bearer,
                Some(Duration::from_millis(30))
            )
            .await,
        Err(ClientError::Transport(_))
    ));
    server.verify().await;
}

#[test]
fn coverage_row_lists_every_parser_option_and_variant() {
    use clap::CommandFactory;
    let mut root = wekan_cli::cli::Cli::command();
    root.build();
    let leaf = root
        .find_subcommand("board")
        .unwrap()
        .find_subcommand("import-trello")
        .unwrap();
    let coverage = include_str!("../../docs/api-coverage.md");
    let row = coverage
        .lines()
        .find(|line| line.contains("| `import_trello` |"))
        .unwrap();
    assert!(row.ends_with("| ✓ |"));
    for arg in leaf.get_arguments() {
        if let Some(long) = arg.get_long() {
            assert!(row.contains(&format!("--{long}")), "missing {long}");
        }
        if matches!(arg.get_long(), Some("format" | "auth"))
            && let Some(values) = arg.get_value_parser().possible_values()
        {
            for value in values {
                assert!(
                    row.contains(value.get_name()),
                    "missing {}",
                    value.get_name()
                );
            }
        }
    }
    assert!(coverage.contains("144 of 144 operations implemented (100%); 0 remaining"));
    assert_eq!(
        coverage
            .lines()
            .filter(|line| line.starts_with("| ") && line.ends_with("| ✓ |"))
            .count(),
        144
    );
}
