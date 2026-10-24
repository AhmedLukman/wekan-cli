use super::{client, client_from_url, user_token};
use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};
use wekan_cli::client::{
    ClientError,
    boards::import::{BoardImportOptions, BoardImportRequest, ImportSource, ZipImportTarget},
    cards::import::IcsImportRequest,
    imports::PreparedImportRequest,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_bytes, body_json, header, method, path, query_param},
};

fn counts() -> Value {
    json!({"swimlanes":1,"lists":2,"cards":3,"checklists":4,"comments":5,"attachments":0})
}

fn board_request() -> PreparedImportRequest<BoardImportRequest> {
    PreparedImportRequest::new(&BoardImportRequest {
        options: Default::default(),
        board: json!({}),
        members_mapping: None,
    })
    .unwrap()
}

fn ics_request(ics: &str) -> PreparedImportRequest<IcsImportRequest> {
    PreparedImportRequest::new(&IcsImportRequest { ics: ics.into() }).unwrap()
}

#[test]
fn import_coverage_rows_include_every_parser_option_and_source() {
    use clap::CommandFactory;
    use wekan_cli::cli::Cli;
    let coverage = include_str!("../../docs/api-coverage.md");
    let mut root = Cli::command();
    root.build();
    for (family, command, operation) in [
        ("board", "import", "import_board_from"),
        ("board", "import-zip", "importZip"),
        ("card", "import-ics", "import_ics"),
    ] {
        let row = coverage
            .lines()
            .find(|line| line.contains(&format!("| `{operation}` |")))
            .unwrap();
        let leaf = root
            .find_subcommand(family)
            .unwrap()
            .find_subcommand(command)
            .unwrap();
        assert!(row.ends_with("| ✓ |"));
        for argument in leaf.get_arguments() {
            if let Some(long) = argument.get_long() {
                assert!(
                    row.contains(&format!("--{long}")),
                    "{operation}: missing --{long}"
                );
            }
        }
        if operation == "import_board_from" {
            for source in ImportSource::ALL {
                assert!(row.contains(source.as_str()));
            }
        }
    }
}

#[tokio::test]
async fn all_board_sources_use_native_paths_and_preserve_wrapped_documents() {
    let server = MockServer::start().await;
    let document =
        json!({"board":{"name":"Kanboard"},"tasks":[{"title":"Ship","extension":{"keep":true}}]});
    let body = json!({"board":document,"membersMapping":{"source":"local"}});
    for source in std::iter::once(None).chain(ImportSource::ALL.into_iter().map(Some)) {
        server.reset().await;
        let route = source.map_or_else(
            || "/wekan/api/boards/import".into(),
            |s| format!("/wekan/api/boards/import/{}", s.as_str()),
        );
        Mock::given(method("POST"))
            .and(path(route))
            .and(header("Authorization", "Bearer user-token"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(&body))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"new"})))
            .expect(1)
            .mount(&server)
            .await;
        let result = client_from_url(&format!("{}/wekan/", server.uri()))
            .import_board(
                source,
                PreparedImportRequest::new(&BoardImportRequest {
                    options: Default::default(),
                    board: document.clone(),
                    members_mapping: Some([("source".into(), "local".into())].into()),
                })
                .unwrap(),
                &user_token(),
                None,
            )
            .await
            .unwrap();
        assert_eq!(result.board_id, "new");
    }
}

#[tokio::test]
async fn binary_board_imports_preserve_native_envelopes_and_member_options() {
    let server = MockServer::start().await;
    let bytes = [0, 255, 10];
    let mapping: Option<BTreeMap<String, String>> =
        Some([("source".into(), "local".into())].into());
    for (source, mut request, envelope) in [
        (
            ImportSource::Excel,
            BoardImportRequest::excel(&bytes, mapping.clone()),
            "excelBase64",
        ),
        (
            ImportSource::Plane,
            BoardImportRequest::xlsx(&bytes, mapping.clone()),
            "xlsxBase64",
        ),
        (
            ImportSource::Plane,
            BoardImportRequest::zip(&bytes, mapping.clone()),
            "zipBase64",
        ),
        (
            ImportSource::Notion,
            BoardImportRequest::zip(&bytes, mapping.clone()),
            "zipBase64",
        ),
        (
            ImportSource::Vikunja,
            BoardImportRequest::zip(&bytes, mapping),
            "zipBase64",
        ),
    ] {
        server.reset().await;
        request.options.members_mode = Some("me".into());
        Mock::given(method("POST"))
            .and(path(format!("/api/boards/import/{}", source.as_str())))
            .and(header("Authorization", "Bearer user-token"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(json!({
                "board": {envelope: "AP8K"},
                "membersMapping": {"source": "local"},
                "membersMode": "me"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"new"})))
            .expect(1)
            .mount(&server)
            .await;
        client(&server)
            .import_board(
                Some(source),
                PreparedImportRequest::new(&request).unwrap(),
                &user_token(),
                None,
            )
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn explicit_wekan_source_preserves_each_member_mode_in_the_body() {
    let server = MockServer::start().await;
    let document = json!({"cards":[],"lists":[]});
    for mode in ["map", "placeholder", "me"] {
        server.reset().await;
        Mock::given(method("POST"))
            .and(path("/api/boards/import/wekan"))
            .and(header("Authorization", "Bearer user-token"))
            .and(header("Content-Type", "application/json"))
            .and(body_json(json!({"board":document,"membersMode":mode})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"new"})))
            .expect(1)
            .mount(&server)
            .await;
        client(&server)
            .import_board(
                Some(ImportSource::Wekan),
                PreparedImportRequest::new(&BoardImportRequest {
                    options: BoardImportOptions {
                        members_mode: Some(mode.into()),
                        ..Default::default()
                    },
                    board: document.clone(),
                    members_mapping: None,
                })
                .unwrap(),
                &user_token(),
                None,
            )
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn zip_preserves_binary_query_scopes_and_explicit_authentication() {
    let server = MockServer::start().await;
    let target = ZipImportTarget {
        board_id: "b".into(),
        fields: vec!["comments".into(), "people".into(), "comments".into()],
        swimlane_id: Some("s".into()),
        list_id: Some("l".into()),
        card_id: Some("c".into()),
        checklist_id: Some("ch".into()),
    };
    for query in [false, true] {
        server.reset().await;
        let bytes = b"PK\0\xff\r\n".to_vec();
        Mock::given(method("POST"))
            .and(path("/api/import/zip"))
            .and(query_param("boardId", "b"))
            .and(query_param("fields", "comments,people,comments"))
            .and(query_param("swimlaneId", "s"))
            .and(query_param("listId", "l"))
            .and(query_param("cardId", "c"))
            .and(query_param("checklistId", "ch"))
            .and(header("Content-Type", "application/zip"))
            .and(header("Content-Length", "6"))
            .and(body_bytes(bytes.clone()))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"ok":true,"counts":counts()})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let response = client(&server)
            .import_zip(
                &target,
                std::io::Cursor::new(bytes.clone()),
                bytes.len() as u64,
                &user_token(),
                query,
                None,
            )
            .await
            .unwrap();
        assert_eq!(response.counts.cards, 3);
        let requests = server.received_requests().await.unwrap();
        let request = &requests[0];
        if query {
            assert!(!request.headers.contains_key("authorization"));
            assert!(
                request
                    .url
                    .query_pairs()
                    .any(|(k, v)| k == "authToken" && v == "user-token")
            );
        } else {
            assert_eq!(request.headers["authorization"], "Bearer user-token");
            assert!(!request.url.query_pairs().any(|(k, _)| k == "authToken"));
        }
    }
}

#[tokio::test]
async fn import_responses_strictly_reject_unknown_missing_invalid_and_empty_fields() {
    let server = MockServer::start().await;
    for response in [
        json!({}),
        json!({"_id":null}),
        json!({"_id":4}),
        json!({"_id":""}),
        json!({"_id":"b","extra":{"secret":"hidden"}}),
    ] {
        server.reset().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;
        for source in [None, Some(ImportSource::Wekan)] {
            assert!(matches!(
                client(&server)
                    .import_board(source, board_request(), &user_token(), None)
                    .await,
                Err(ClientError::Protocol { .. })
            ));
        }
    }
    for response in [
        json!({}),
        json!({"ok":false,"counts":counts()}),
        json!({"ok":true,"counts":counts(),"extra":1}),
        json!({"ok":true,"counts":{"swimlanes":0}}),
        json!({"ok":true,"counts":{"swimlanes":0,"lists":0,"cards":0,"checklists":0,"comments":0,"attachments":0,"extra":1}}),
        json!({"ok":true,"counts":{"swimlanes":0,"lists":0,"cards":-1,"checklists":0,"comments":0,"attachments":0}}),
    ] {
        server.reset().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;
        assert!(matches!(
            client(&server)
                .import_zip(
                    &ZipImportTarget::default(),
                    std::io::Cursor::new(Vec::<u8>::new()),
                    0,
                    &user_token(),
                    false,
                    None
                )
                .await,
            Err(ClientError::Protocol { .. })
        ));
    }
    for response in [
        json!({}),
        json!({"created":"1","cardIds":[]}),
        json!({"created":1,"cardIds":[null]}),
        json!({"created":0,"cardIds":[],"extra":true}),
    ] {
        server.reset().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;
        assert!(matches!(
            client(&server)
                .import_ics("b", "s", "l", ics_request(""), &user_token(), None)
                .await,
            Err(ClientError::Protocol { .. })
        ));
    }
}

#[tokio::test]
async fn ics_path_and_native_error_responses_are_preserved_without_retry() {
    let server = MockServer::start().await;
    let body = json!({"ics":"BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n"});
    Mock::given(method("POST"))
        .and(path(
            "/wekan/api/boards/b%2F1/swimlanes/s%2F1/lists/l%2F1/ics",
        ))
        .and(header("Authorization", "Bearer user-token"))
        .and(body_json(&body))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"created":0,"cardIds":[]})))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        client_from_url(&format!("{}/wekan/", server.uri()))
            .import_ics(
                "b/1",
                "s/1",
                "l/1",
                ics_request("BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n"),
                &user_token(),
                None
            )
            .await
            .unwrap()
            .created,
        0
    );
    for (status, response) in [
        (
            200,
            json!({"error":"invalid-import-source","reason":"Rejected","errorType":"Meteor.Error","isClientSafe":true}),
        ),
        (401, json!({"error":"not-authorized"})),
        (500, json!(null)),
    ] {
        server.reset().await;
        Mock::given(method("POST"))
            .respond_with(if status == 500 {
                ResponseTemplate::new(status).set_body_raw("<!DOCTYPE html>", "text/html")
            } else {
                ResponseTemplate::new(status).set_body_json(response)
            })
            .expect(1)
            .mount(&server)
            .await;
        let error = client(&server)
            .import_board(None, board_request(), &user_token(), None)
            .await
            .unwrap_err();
        if status == 200 {
            assert!(matches!(error, ClientError::EmbeddedProtocol { .. }));
        } else {
            assert!(matches!(error, ClientError::Server { .. }));
        }
    }
    server.reset().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(2))
                .set_body_json(json!({"_id":"late"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        client(&server)
            .import_board(
                None,
                board_request(),
                &user_token(),
                Some(Duration::from_millis(30))
            )
            .await,
        Err(ClientError::Transport(_))
    ));
}

#[tokio::test]
async fn zip_timeout_diagnostics_do_not_contain_query_credentials() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&server)
        .await;
    let token = secrecy::SecretString::from("secret&?=/+%".to_owned());
    let error = client(&server)
        .import_zip(
            &ZipImportTarget {
                board_id: "b".into(),
                ..Default::default()
            },
            std::io::Cursor::new(b"ZIP".to_vec()),
            3,
            &token,
            true,
            Some(Duration::from_millis(30)),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::Transport(_)));
    let diagnostic = format!("{error:?}");
    assert!(!diagnostic.contains("authToken"));
    assert!(!diagnostic.contains("secret"));
}

#[tokio::test]
async fn new_board_zip_import_preserves_selectors_and_rejects_unmapped_or_invalid_acknowledgments()
{
    use wekan_cli::client::boards::import::NewBoardZipImportTarget;
    let server = MockServer::start().await;
    let target = NewBoardZipImportTarget {
        fields: vec!["people".into(), "attachments".into()],
        members_mode: Some("me".into()),
    };
    let responses = [
        json!({"ok":true,"boardId":"created"}),
        json!({"ok":true,"boardId":"created","future":{}}),
        json!({"ok":true,"boardId":"created","counts":counts()}),
        json!({"ok":true}),
        json!({"ok":true,"boardId":7}),
        json!({"ok":false,"boardId":"created"}),
        json!({"ok":true,"boardId":""}),
    ];
    for (index, response) in responses.into_iter().enumerate() {
        server.reset().await;
        Mock::given(method("POST"))
            .and(path("/api/import/zip"))
            .and(query_param("newBoard", "1"))
            .and(query_param("membersMode", "me"))
            .and(query_param("fields", "people,attachments"))
            .and(header("Authorization", "Bearer user-token"))
            .and(header("Content-Type", "application/zip"))
            .and(body_bytes(b"PK\0\xff".to_vec()))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        let response = client(&server)
            .import_zip_new_board(
                &target,
                std::io::Cursor::new(b"PK\0\xff".to_vec()),
                4,
                &user_token(),
                false,
                None,
            )
            .await;
        if index == 0 {
            assert_eq!(response.unwrap().board_id, "created");
        } else {
            assert!(
                matches!(response, Err(ClientError::Protocol { .. })),
                "{index}"
            );
        }
        let request = server.received_requests().await.unwrap().pop().unwrap();
        assert!(!request.url.query_pairs().any(|(key, _)| key == "boardId"));
    }
}
