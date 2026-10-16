use super::client;
#[path = "../support/export_fixtures.rs"]
mod fixtures;
use secrecy::SecretString;
use serde_json::{Value, json};
use std::{
    io::{Read, Seek, SeekFrom},
    time::Duration,
};
use wekan_cli::client::{
    ClientError,
    exports::{ExportError, ExportFormat, ExportOptions, ExportRequest},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

const JSON_FORMATS: [&str; 12] = [
    "json",
    "kanboard",
    "deck",
    "openproject",
    "github",
    "gitlab",
    "gitea",
    "forgejo",
    "trello",
    "jira",
    "asana",
    "zenkit",
];
fn options<'a>(format: ExportFormat) -> ExportOptions<'a> {
    ExportOptions {
        format,
        request: ExportRequest::default(),
        token: None,
        query_token: false,
        timeout: None,
    }
}

#[tokio::test]
async fn every_format_uses_its_native_route_and_preserves_response_bytes() {
    let server = MockServer::start().await;
    for name in JSON_FORMATS
        .into_iter()
        .chain(["csv", "tsv", "xlsx", "pdf", "zip"])
    {
        server.reset().await;
        let (suffix, media, body) = match name {
            "json" => (
                "export".into(),
                "application/json",
                serde_json::to_vec_pretty(&fixtures::board()).unwrap(),
            ),
            "csv" | "tsv" => (
                "export/csv".into(),
                if name == "csv" {
                    "text/csv; charset=utf-8"
                } else {
                    "text/tsv"
                },
                b"title\nShip\n".to_vec(),
            ),
            "xlsx" => (
                "exportExcel".into(),
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                b"PK\x03\x04test-workbook".to_vec(),
            ),
            "pdf" => (
                "exportPDF".into(),
                "application/pdf",
                b"%PDF-1.7\n%%EOF".to_vec(),
            ),
            "zip" => (
                "exportZip".into(),
                "application/zip",
                b"PK\x05\x06empty-archive".to_vec(),
            ),
            _ => (
                format!("export/{name}"),
                "application/json",
                serde_json::to_vec_pretty(&fixtures::external(name)).unwrap(),
            ),
        };
        let mut mock = Mock::given(method("GET"))
            .and(path(format!("/api/boards/board/{suffix}")))
            .and(header("Authorization", "Bearer test-token"));
        if name == "tsv" {
            mock = mock.and(query_param("delimiter", "\t"));
        } else if name == "csv" {
            mock = mock.and(query_param("delimiter", "; &+"));
        }
        mock.respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", media)
                .set_body_bytes(body.clone()),
        )
        .expect(1)
        .mount(&server)
        .await;
        let mut file = tempfile::tempfile().unwrap();
        let token = SecretString::from("test-token".to_owned());
        let mut opts = options(name.parse().unwrap());
        opts.token = Some(&token);
        if name == "csv" {
            opts.request.delimiter = Some("; &+".into());
        }
        let result = client(&server)
            .export_board("board", opts, &mut file)
            .await
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(result.bytes_written, body.len() as u64);
        let mut saved = Vec::new();
        file.read_to_end(&mut saved).unwrap();
        assert_eq!(saved, body);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests[0].url.query_pairs().count(),
            usize::from(matches!(name, "csv" | "tsv")),
            "{name} exports must omit unset parameters"
        );
    }
}

#[tokio::test]
async fn typed_board_requests_preserve_scope_field_order_and_optional_booleans() {
    let server = MockServer::start().await;
    for attachments in [None, Some(false), Some(true)] {
        server.reset().await;
        Mock::given(method("GET"))
            .and(path("/api/boards/b/export"))
            .and(query_param("fields", "people,labels,people"))
            .and(query_param("swimlaneId", "s"))
            .and(query_param("listId", "l"))
            .and(query_param("cardId", "c"))
            .and(query_param("checklistId", "check"))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::board()))
            .expect(1)
            .mount(&server)
            .await;
        let mut opts = options(ExportFormat::Json);
        opts.request = ExportRequest {
            fields: vec!["people".into(), "labels".into(), "people".into()],
            swimlane_id: Some("s".into()),
            list_id: Some("l".into()),
            card_id: Some("c".into()),
            checklist_id: Some("check".into()),
            attachments,
            ..ExportRequest::default()
        };
        client(&server)
            .export_board("b", opts, &mut tempfile::tempfile().unwrap())
            .await
            .unwrap();
        let requests = server.received_requests().await.unwrap();
        let query = requests[0].url.query_pairs().collect::<Vec<_>>();
        assert_eq!(query.len(), 5 + usize::from(attachments.is_some()));
        assert_eq!(
            query
                .iter()
                .find(|(key, _)| key == "attachments")
                .map(|(_, value)| value.as_ref()),
            attachments.map(|value| if value { "true" } else { "false" })
        );
    }
}

#[tokio::test]
async fn scope_query_authentication_and_scoped_resource_routes_are_exact() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/b%2F1/lists/l%2F1/cards/c%2F1/exportPDF"))
        .and(query_param("authToken", "query secret+/?"))
        .and(query_param("fields", "people,board-info"))
        .and(query_param("lang", "fi"))
        .and(query_param("dateFormat", "DD-MM-YYYY"))
        .and(query_param("tz", "Africa/Nairobi"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Type", "application/pdf")
                .set_body_bytes(b"%PDF-1.7".to_vec()),
        )
        .mount(&server)
        .await;
    let token = SecretString::from("query secret+/?".to_owned());
    let mut opts = options(ExportFormat::Pdf);
    opts.token = Some(&token);
    opts.query_token = true;
    opts.request = ExportRequest {
        fields: vec!["people".into(), "board-info".into()],
        language: Some("fi".into()),
        date_format: Some("DD-MM-YYYY".into()),
        timezone: Some("Africa/Nairobi".into()),
        ..ExportRequest::default()
    };
    client(&server)
        .export_card(
            "b/1",
            "l/1",
            "c/1",
            opts,
            &mut tempfile::tempfile().unwrap(),
        )
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    assert!(!requests[0].headers.contains_key("authorization"));
    server.reset().await;
    for body in [json!({}), json!({"_id":"a","name":"data","file":null})] {
        server.reset().await;
        Mock::given(method("GET"))
            .and(path("/api/boards/b/attachments/a/export"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        client(&server)
            .export_attachment(
                "b",
                "a",
                options(ExportFormat::Json),
                &mut tempfile::tempfile().unwrap(),
            )
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn attachment_exports_require_the_projected_shape_or_an_empty_object() {
    let server = MockServer::start().await;
    for body in [
        json!({"_id":"a"}),
        json!({"_id":"a","file":12}),
        json!({"_id":"a","file":"not-base64!"}),
        json!({"unmapped":true}),
        json!({"_id":"a","file":null,"unmapped":true}),
    ] {
        server.reset().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
        assert!(matches!(
            client(&server)
                .export_attachment(
                    "b",
                    "a",
                    options(ExportFormat::Json),
                    &mut tempfile::tempfile().unwrap()
                )
                .await,
            Err(ExportError::Client(ClientError::Protocol { .. }))
        ));
    }
}

async fn reject(format: &str, value: Value) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(value))
        .mount(&server)
        .await;
    let result = client(&server)
        .export_board(
            "b",
            options(format.parse().unwrap()),
            &mut tempfile::tempfile().unwrap(),
        )
        .await;
    assert!(
        matches!(
            result,
            Err(ExportError::Client(ClientError::Protocol { .. }))
        ),
        "{format}: {result:?}"
    );
}

#[tokio::test]
async fn card_and_subtask_watchers_are_optional_strict_and_preserve_native_bytes() {
    let server = MockServer::start().await;
    let card = json!({
        "_id":"card", "title":"Watched card", "archived":false,
        "swimlaneId":"swimlane", "createdAt":"2026-10-16T00:00:00Z",
        "modifiedAt":"2026-10-16T00:00:00Z", "dateLastActivity":"2026-10-16T00:00:00Z",
        "userId":"creator", "type":"cardType-card", "showActivities":false
    });
    for collection in ["cards", "subtaskItems"] {
        for watchers in [None, Some(json!([])), Some(json!(["watcher", "other"]))] {
            server.reset().await;
            let mut document = fixtures::board();
            document[collection] = json!([card.clone()]);
            if let Some(watchers) = watchers {
                document[collection][0]["watchers"] = watchers;
            }
            let body = serde_json::to_vec_pretty(&document).unwrap();
            Mock::given(method("GET"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_raw(body.clone(), "application/json"),
                )
                .expect(1)
                .mount(&server)
                .await;
            let mut file = tempfile::tempfile().unwrap();
            client(&server)
                .export_board("board", options(ExportFormat::Json), &mut file)
                .await
                .unwrap();
            let mut saved = Vec::new();
            file.read_to_end(&mut saved).unwrap();
            assert_eq!(saved, body);
        }
        for watchers in [
            Value::Null,
            json!("watcher"),
            json!({}),
            json!([null]),
            json!([42]),
            json!([true]),
            json!([[]]),
            json!([{"userId":"watcher"}]),
        ] {
            let mut document = fixtures::board();
            document[collection] = json!([card.clone()]);
            document[collection][0]["watchers"] = watchers;
            reject("json", document).await;
        }
        let mut unknown = fixtures::board();
        unknown[collection] = json!([card.clone()]);
        unknown[collection][0]["watchers"] = json!(["watcher"]);
        unknown[collection][0]["unmapped"] = json!(true);
        reject("json", unknown).await;
        let mut missing = fixtures::board();
        missing[collection] = json!([card.clone()]);
        missing[collection][0]["watchers"] = json!(["watcher"]);
        missing[collection][0]
            .as_object_mut()
            .unwrap()
            .remove("_id");
        reject("json", missing).await;
    }
}

#[tokio::test]
async fn checklist_creators_are_optional_typed_and_preserved_in_native_exports() {
    let server = MockServer::start().await;
    let mut board = fixtures::board();
    board["checklists"] = json!([{
        "_id":"checklist", "cardId":"card", "title":"Release", "sort":0,
        "createdAt":"2026-10-16T00:00:00Z", "modifiedAt":"2026-10-16T00:00:00Z"
    }]);
    for creator in [None, Some(json!("creator")), Some(Value::Null)] {
        server.reset().await;
        let mut document = board.clone();
        if let Some(creator) = creator {
            document["checklists"][0]["userId"] = creator;
        }
        let body = serde_json::to_vec_pretty(&document).unwrap();
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body.clone(), "application/json"))
            .expect(1)
            .mount(&server)
            .await;
        let mut file = tempfile::tempfile().unwrap();
        client(&server)
            .export_board("board", options(ExportFormat::Json), &mut file)
            .await
            .unwrap();
        let mut saved = Vec::new();
        file.read_to_end(&mut saved).unwrap();
        assert_eq!(saved, body);
    }

    for creator in [json!(42), json!(true), json!([]), json!({})] {
        let mut document = board.clone();
        document["checklists"][0]["userId"] = creator;
        reject("json", document).await;
    }
    let mut unknown = board.clone();
    unknown["checklists"][0]["userId"] = json!("creator");
    unknown["checklists"][0]["unmapped"] = json!(true);
    reject("json", unknown).await;
    board["checklists"][0]
        .as_object_mut()
        .unwrap()
        .remove("cardId");
    reject("json", board).await;
}

#[tokio::test]
async fn import_activity_sources_are_strictly_mapped_and_preserve_native_bytes() {
    let server = MockServer::start().await;
    let mut board = fixtures::board();
    board["activities"] = json!([
        {"_id":"ordinary","activityType":"createBoard"},
        {"_id":"csv","activityType":"importBoard","source":{"id":"board","system":"CSV/TSV"}},
        {"_id":"trello","activityType":"importBoard","source":{"id":"external","system":"Trello","url":"https://trello.com/b/external"}},
        {"_id":"wekan","activityType":"importBoard","source":{"system":"Wekan"}}
    ]);
    let body = serde_json::to_vec_pretty(&board).unwrap();
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body.clone(), "application/json"))
        .expect(1)
        .mount(&server)
        .await;
    let mut file = tempfile::tempfile().unwrap();
    client(&server)
        .export_board("board", options(ExportFormat::Json), &mut file)
        .await
        .unwrap();
    let mut saved = Vec::new();
    file.read_to_end(&mut saved).unwrap();
    assert_eq!(saved, body);

    for source in [
        Value::Null,
        json!("CSV/TSV"),
        json!([]),
        json!({}),
        json!({"id":"board"}),
        json!({"system":null}),
        json!({"system":42}),
        json!({"id":null,"system":"CSV/TSV"}),
        json!({"id":42,"system":"CSV/TSV"}),
        json!({"system":"Trello","url":null}),
        json!({"system":"Trello","url":42}),
        json!({"system":"Trello","url":"https://trello.com/b/external","unmapped":true}),
    ] {
        let mut board = fixtures::board();
        board["activities"] = json!([{"_id":"import","source":source}]);
        reject("json", board).await;
    }
}

#[tokio::test]
async fn trello_exports_preserve_nullable_label_ids_and_reject_invalid_elements() {
    let server = MockServer::start().await;
    let mut document = fixtures::external("trello");
    document["cards"] = json!([{
        "id":"card", "name":"Ship", "desc":"Details", "idList":"list",
        "due":null, "closed":false, "idLabels":[], "idMembers":[], "idChecklists":[]
    }]);
    for labels in [json!([]), json!(["label"]), json!([null, "label", null])] {
        server.reset().await;
        document["cards"][0]["idLabels"] = labels;
        let body = serde_json::to_vec_pretty(&document).unwrap();
        Mock::given(method("GET"))
            .and(path("/api/boards/board/export/trello"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body.clone(), "application/json"))
            .expect(1)
            .mount(&server)
            .await;
        let mut file = tempfile::tempfile().unwrap();
        client(&server)
            .export_board("board", options(ExportFormat::Trello), &mut file)
            .await
            .unwrap();
        let mut saved = Vec::new();
        file.read_to_end(&mut saved).unwrap();
        assert_eq!(saved, body);
    }

    for value in [json!(42), json!(true), json!([]), json!({})] {
        let mut invalid = document.clone();
        invalid["cards"][0]["idLabels"] = json!([value]);
        reject("trello", invalid).await;
    }
    let mut null_array = document.clone();
    null_array["cards"][0]["idLabels"] = Value::Null;
    reject("trello", null_array).await;
    let mut missing = document.clone();
    missing["cards"][0]
        .as_object_mut()
        .unwrap()
        .remove("idLabels");
    reject("trello", missing).await;
    document["cards"][0]["unmapped"] = json!(true);
    reject("trello", document).await;
}

#[tokio::test]
async fn json_exports_reject_unmapped_fields_missing_required_fields_and_wrong_types() {
    for format in JSON_FORMATS {
        let valid = fixtures::external(format);
        let mut unknown = valid.clone();
        if unknown.is_array() {
            unknown[0]["unmapped"] = json!("never echo this");
        } else {
            unknown["unmapped"] = json!(true);
        }
        reject(format, unknown).await;
        let mut invalid = valid.clone();
        if invalid.is_array() {
            invalid[0]["title"] = json!({});
        } else {
            let key = invalid.as_object().unwrap().keys().next().unwrap().clone();
            invalid[&key] = json!(17);
        }
        reject(format, invalid).await;
        let mut missing = valid;
        if missing.is_array() {
            missing[0].as_object_mut().unwrap().remove("title");
        } else {
            let key = missing.as_object().unwrap().keys().next().unwrap().clone();
            missing.as_object_mut().unwrap().remove(&key);
        }
        reject(format, missing).await;
    }
    let mut board = fixtures::board();
    board["members"] = json!([{"userId":"u","unmapped":true}]);
    reject("json", board).await;
    let mut board = fixtures::board();
    board["triggers"] = json!([{"_id":"t","activityType":"createCard","extraPayload":1}]);
    reject("json", board).await;
    let mut board = fixtures::board();
    board["actions"] = json!([{"_id":"a","actionType":42}]);
    reject("json", board).await;
    let mut board = fixtures::board();
    board["activities"] = json!([{"_id":"a","createdAt":"invalid"}]);
    reject("json", board).await;
    let mut board = fixtures::board();
    board["users"] = json!([{"_id":"u","profile":{"unknown":true}}]);
    reject("json", board).await;
    let mut nested = fixtures::external("deck");
    nested["stacks"][0]["cards"][0]["unknown"] = json!(true);
    reject("deck", nested).await;
}

#[tokio::test]
async fn errors_redirects_timeouts_and_invalid_binary_signatures_are_not_artifacts() {
    let server = MockServer::start().await;
    for response in [
        ResponseTemplate::new(200).set_body_string("Board not found"),
        ResponseTemplate::new(200)
            .insert_header("Content-Type", "text/plain")
            .set_body_string("Permission Error"),
        ResponseTemplate::new(200)
            .insert_header("Content-Type", "application/pdf")
            .set_body_string("Permission Error"),
        ResponseTemplate::new(200)
            .insert_header("Content-Type", "text/html")
            .set_body_string("<html>login</html>"),
        ResponseTemplate::new(500).set_body_json(json!({"error":"Export failed"})),
        ResponseTemplate::new(302).insert_header("Location", "http://example.test/secret"),
    ] {
        server.reset().await;
        Mock::given(method("GET"))
            .respond_with(response)
            .mount(&server)
            .await;
        assert!(
            client(&server)
                .export_board(
                    "b",
                    options(ExportFormat::Pdf),
                    &mut tempfile::tempfile().unwrap()
                )
                .await
                .is_err()
        );
    }
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(300)))
        .mount(&server)
        .await;
    let mut opts = options(ExportFormat::Pdf);
    opts.timeout = Some(Duration::from_millis(20));
    assert!(matches!(
        client(&server)
            .export_board("b", opts, &mut tempfile::tempfile().unwrap())
            .await,
        Err(ExportError::Client(ClientError::Transport(_)))
    ));
}

#[tokio::test]
async fn exports_larger_than_64_mib_have_no_aggregate_limit() {
    let server = MockServer::start().await;
    // Many modest records exercise reader-based array validation without one
    // enormous record masking accidental whole-document buffering.
    let row = "{\"title\":\"Ship\",\"body\":\"".to_owned()
        + &"x".repeat(1000)
        + "\",\"state\":\"open\",\"labels\":[]}";
    let body = "[".to_owned()
        + &std::iter::repeat_n(row.as_str(), 66000)
            .collect::<Vec<_>>()
            .join(",")
        + "]";
    assert!(body.len() > 64 * 1024 * 1024);
    let length = body.len();
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
        .mount(&server)
        .await;
    let mut file = tempfile::tempfile().unwrap();
    let result = client(&server)
        .export_board("b", options(ExportFormat::Github), &mut file)
        .await
        .unwrap();
    assert_eq!(result.bytes_written, length as u64);
    assert_eq!(file.seek(SeekFrom::End(0)).unwrap(), length as u64);
}

#[tokio::test]
async fn interrupted_response_bodies_fail_instead_of_validating_partial_files() {
    use std::io::Write;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let thread = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut request = [0; 4096];
        let _ = socket.read(&mut request);
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/pdf\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n%PDF-1.7\n").unwrap();
    });
    let client = super::client_from_url(&format!("http://{address}"));
    let error = client
        .export_board(
            "b",
            options(ExportFormat::Pdf),
            &mut tempfile::tempfile().unwrap(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ExportError::Client(ClientError::ResponseBody { .. })
    ));
    thread.join().unwrap();
}
