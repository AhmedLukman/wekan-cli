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
    exports::{ChartExportOptions, ExportError, ExportFormat, ExportOptions, ExportRequest},
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
async fn chart_exports_use_native_routes_and_preserve_chart_options_and_artifacts() {
    let server = MockServer::start().await;
    for (format, endpoint, media, body) in [
        (
            ExportFormat::Pdf,
            "exportPDF",
            "application/pdf",
            b"%PDF-1.7\n%%EOF".as_slice(),
        ),
        (
            ExportFormat::Xlsx,
            "exportExcel",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            b"PK\x03\x04test-workbook".as_slice(),
        ),
    ] {
        server.reset().await;
        Mock::given(method("GET"))
            .and(path(format!(
                "/api/boards/b%2F1/charts/scrumSprint/{endpoint}"
            )))
            .and(header("Authorization", "Bearer chart-token"))
            .and(query_param("sprintId", "sprint/1"))
            .and(query_param("targetCount", "12"))
            .and(query_param("historyDays", "30"))
            .and(query_param("targetDate", "2026-12-01"))
            .and(query_param("sizeField", "poker"))
            .and(query_param("lang", "fi"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, media))
            .expect(1)
            .mount(&server)
            .await;
        let token = SecretString::from("chart-token");
        let mut opts = options(format);
        opts.token = Some(&token);
        opts.request.chart = Some(ChartExportOptions {
            sprint_id: Some("sprint/1".into()),
            target_count: Some(12),
            history_days: Some(30),
            target_date: Some("2026-12-01".into()),
            size_field: Some("poker".into()),
        });
        opts.request.language = Some("fi".into());
        let mut file = tempfile::tempfile().unwrap();
        let result = client(&server)
            .export_board_chart("b/1", "scrumSprint", opts, &mut file)
            .await
            .unwrap();
        assert_eq!(result.bytes_written, body.len() as u64);
        let mut saved = Vec::new();
        file.read_to_end(&mut saved).unwrap();
        assert_eq!(saved, body);
    }
}

#[tokio::test]
async fn all_board_exports_validate_native_archives_and_workbooks() {
    let server = MockServer::start().await;
    for (format, media, body) in [
        (
            ExportFormat::Wekan,
            "application/zip",
            b"PK\x05\x06empty-archive".as_slice(),
        ),
        (
            ExportFormat::Csv,
            "application/zip",
            b"PK\x03\x04csv-archive".as_slice(),
        ),
        (
            ExportFormat::Excel,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            b"PK\x03\x04test-workbook".as_slice(),
        ),
    ] {
        server.reset().await;
        Mock::given(method("GET"))
            .and(path(format!("/api/export-all-boards/{}", format.as_str())))
            .and(query_param("authToken", "query secret+/?"))
            .and(query_param("boardIds", "b1,b2"))
            .and(query_param("fields", "people,labels"))
            .and(query_param("attachments", "false"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, media))
            .expect(1)
            .mount(&server)
            .await;
        let token = SecretString::from("query secret+/?");
        let mut opts = options(format);
        opts.token = Some(&token);
        opts.query_token = true;
        opts.request.board_ids = vec!["b1".into(), "b2".into()];
        opts.request.fields = vec!["people".into(), "labels".into()];
        opts.request.attachments = Some(false);
        let mut file = tempfile::tempfile().unwrap();
        let result = client(&server)
            .export_all_boards(opts, &mut file)
            .await
            .unwrap();
        assert_eq!(result.bytes_written, body.len() as u64);
        let mut saved = Vec::new();
        file.read_to_end(&mut saved).unwrap();
        assert_eq!(saved, body);
        let requests = server.received_requests().await.unwrap();
        assert!(!requests[0].headers.contains_key("authorization"));
        assert_eq!(requests[0].url.query_pairs().count(), 4);
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

async fn preserve_gitlab_export(document: Value) {
    preserve_external_export("gitlab", document).await;
}

async fn preserve_external_export(format: &str, document: Value) {
    let server = MockServer::start().await;
    let mut body = serde_json::to_vec_pretty(&document).unwrap();
    body.push(b'\n');
    Mock::given(method("GET"))
        .and(path(format!("/api/boards/board/export/{format}")))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body.clone(), "application/json"))
        .expect(1)
        .mount(&server)
        .await;
    let mut file = tempfile::tempfile().unwrap();
    let result = client(&server)
        .export_board("board", options(format.parse().unwrap()), &mut file)
        .await
        .unwrap();
    assert_eq!(result.bytes_written, body.len() as u64);
    let mut saved = Vec::new();
    file.read_to_end(&mut saved).unwrap();
    assert_eq!(saved, body);
}

// v12.57 externalExportFormatters.js:187-226 conditionally emits planning
// descriptions/dates and links each release-assigned card to its version.
fn openproject_planning_document() -> Value {
    let mut document = fixtures::external("openproject");
    document["_embedded"]["versions"] = json!([{
        "_type":"Version", "id":1, "name":"Release", "status":"open",
        "description":{"raw":"Release notes"}, "startDate":"2030-01-02", "endDate":"2030-01-09"
    }]);
    document["_embedded"]["sprints"] = json!([{
        "_type":"Sprint", "id":1, "name":"Sprint", "description":{"raw":"Sprint goal"},
        "startDate":"2030-01-02", "finishDate":"2030-01-09",
        "_links":{"self":{"href":"/api/v3/sprints/1","title":"Sprint"},
            "status":{"href":"urn:openproject-org:api:v3:sprints:status:in_planning","title":"In planning"}}
    }]);
    document["_embedded"]["elements"][0]["_links"]["version"] =
        json!({"href":"/api/v3/versions/1","title":"Release"});
    document
}

#[tokio::test]
async fn openproject_export_preserves_optional_planning_metadata_and_release_links() {
    for (collection, fields) in [
        ("versions", ["description", "startDate", "endDate"]),
        ("sprints", ["description", "startDate", "finishDate"]),
    ] {
        for selected in 0..8 {
            let mut document = openproject_planning_document();
            for (index, field) in fields.iter().enumerate() {
                if selected & (1 << index) == 0 {
                    document["_embedded"][collection][0]
                        .as_object_mut()
                        .unwrap()
                        .remove(*field);
                }
            }
            preserve_external_export("openproject", document).await;
        }
    }
}

#[tokio::test]
async fn openproject_planning_metadata_and_links_remain_strict() {
    let document = openproject_planning_document();
    for pointer in [
        "/_embedded/versions/0",
        "/_embedded/versions/0/description",
        "/_embedded/sprints/0",
        "/_embedded/sprints/0/description",
        "/_embedded/elements/0/_links/version",
    ] {
        let mut unknown = document.clone();
        unknown
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unmapped".into(), json!("private-value"));
        reject("openproject", unknown).await;
    }
    for (collection, fields) in [
        ("versions", ["description", "startDate", "endDate"]),
        ("sprints", ["description", "startDate", "finishDate"]),
    ] {
        for field in fields {
            for value in [
                Value::Null,
                json!(1),
                json!(false),
                json!([]),
                json!({}),
                json!(["description"]),
            ] {
                let mut invalid = document.clone();
                invalid["_embedded"][collection][0][field] = value;
                reject("openproject", invalid).await;
            }
        }
        for field in [
            "_type",
            "id",
            "name",
            if collection == "versions" {
                "status"
            } else {
                "_links"
            },
        ] {
            let mut missing = document.clone();
            missing["_embedded"][collection][0]
                .as_object_mut()
                .unwrap()
                .remove(field);
            reject("openproject", missing).await;
        }
        for value in [Value::Null, json!(1), json!(false), json!([]), json!({})] {
            let mut invalid = document.clone();
            invalid["_embedded"][collection][0]["description"]["raw"] = value;
            reject("openproject", invalid).await;
        }
    }
    for value in [
        Value::Null,
        json!(1),
        json!(false),
        json!([]),
        json!("Release"),
        json!(["/api/v3/versions/1", "Release"]),
    ] {
        let mut invalid = document.clone();
        invalid["_embedded"]["elements"][0]["_links"]["version"] = value;
        reject("openproject", invalid).await;
    }
    for field in ["href", "title"] {
        let mut missing = document.clone();
        missing["_embedded"]["elements"][0]["_links"]["version"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        reject("openproject", missing).await;
        for value in [Value::Null, json!(1), json!(false), json!([]), json!({})] {
            let mut invalid = document.clone();
            invalid["_embedded"]["elements"][0]["_links"]["version"][field] = value;
            reject("openproject", invalid).await;
        }
    }
}

// v12.57 externalExporters.js:108,152 cannot resolve a deleted user's
// username. The formatters omit only the identity property, keeping comments.
fn external_comment_documents() -> Vec<(&'static str, Value, &'static str)> {
    let at = "2026-10-22T04:55:09.823Z";
    [
        ("kanboard", "/tasks/0", "comments",
            json!({"comment":"History","username":"reviewer","date_creation":at}),
            "/tasks/0/comments/0/username"),
        ("deck", "/stacks/0/cards/0", "comments",
            json!({"message":"History","actorId":"reviewer","creationDateTime":at}),
            "/stacks/0/cards/0/comments/0/actorId"),
        ("openproject", "/_embedded/elements/0", "_embedded",
            json!({"activities":[{"_type":"Activity::Comment","comment":{"raw":"History"},
                "createdAt":at,"_links":{"user":{"title":"reviewer"}}}]}),
            "/_embedded/elements/0/_embedded/activities/0/_links/user/title"),
        ("jira", "/issues/0/fields", "comment",
            json!({"comments":[{"body":"History","author":{"name":"reviewer"},"created":at}]}),
            "/issues/0/fields/comment/comments/0/author/name"),
        ("asana", "/data/0", "stories",
            json!({"resource_subtype":"comment_added","text":"History","created_by":{"name":"reviewer"},"created_at":at}),
            "/data/0/stories/0/created_by/name"),
        ("zenkit", "/items/0", "comments",
            json!({"text":"History","author":"reviewer","date":at}),
            "/items/0/comments/0/author"),
    ].into_iter().map(|(format, parent, key, comment, identity)| {
        let mut document = fixtures::external(format);
        let comments = if matches!(format, "openproject" | "jira") { comment } else { json!([comment]) };
        document.pointer_mut(parent).unwrap().as_object_mut().unwrap().insert(key.into(), comments);
        (format, document, identity)
    }).collect()
}

#[tokio::test]
async fn external_exports_preserve_comments_with_present_or_deleted_authors() {
    for (format, document, identity) in external_comment_documents() {
        preserve_external_export(format, document.clone()).await;
        let mut absent = document;
        let (parent, field) = identity.rsplit_once('/').unwrap();
        absent
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(field);
        preserve_external_export(format, absent).await;
    }
}

#[tokio::test]
async fn external_comment_author_identities_remain_strict() {
    for (format, document, identity) in external_comment_documents() {
        for value in [Value::Null, json!(1), json!(false), json!([]), json!({})] {
            let mut invalid = document.clone();
            *invalid.pointer_mut(identity).unwrap() = value;
            reject(format, invalid).await;
        }
        let (parent, _) = identity.rsplit_once('/').unwrap();
        let mut unknown = document.clone();
        unknown
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unmapped".into(), json!("private-value"));
        reject(format, unknown).await;
        if matches!(format, "openproject" | "jira" | "asana") {
            let (ancestor, field) = parent.rsplit_once('/').unwrap();
            let mut missing = document.clone();
            missing
                .pointer_mut(ancestor)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(field);
            reject(format, missing).await;
            for value in [
                Value::Null,
                json!(1),
                json!(false),
                json!("reviewer"),
                json!([]),
                json!(["reviewer"]),
            ] {
                let mut invalid = document.clone();
                *invalid.pointer_mut(parent).unwrap() = value;
                reject(format, invalid).await;
            }
        }
    }
}

// Wekan v12.57 models/lib/externalExportFormatters.js:253-258 emits these
// release/sprint records, omitting dates that server/scrum.js permits unset.
fn gitlab_planning_document(field: &str) -> Value {
    let mut document = fixtures::external("gitlab");
    document[0][field] = json!({
        "id":1, "iid":1, "title":"Release", "description":"",
        "state":if field == "milestone" { json!("active") } else { json!(1) },
        "start_date":"2026-01-01", "due_date":"2026-02-01"
    });
    document
}

#[tokio::test]
async fn gitlab_export_milestones_preserve_optional_planned_dates_and_bytes() {
    for dates in 0..4 {
        let mut document = gitlab_planning_document("milestone");
        for (bit, field) in [(1, "start_date"), (2, "due_date")] {
            if dates & bit == 0 {
                document[0]["milestone"]
                    .as_object_mut()
                    .unwrap()
                    .remove(field);
            }
        }
        preserve_gitlab_export(document).await;
    }
    let mut released = gitlab_planning_document("milestone");
    released[0]["milestone"]["state"] = json!("closed");
    preserve_gitlab_export(released).await;
}

#[tokio::test]
async fn gitlab_export_iterations_preserve_optional_planned_dates_and_bytes() {
    for dates in 0..4 {
        let mut document = gitlab_planning_document("iteration");
        for (bit, field) in [(1, "start_date"), (2, "due_date")] {
            if dates & bit == 0 {
                document[0]["iteration"]
                    .as_object_mut()
                    .unwrap()
                    .remove(field);
            }
        }
        preserve_gitlab_export(document).await;
    }
}

#[tokio::test]
async fn gitlab_export_notes_preserve_deleted_authors_and_bytes() {
    // v12.57 retains comments after account deletion; externalExporters.js
    // cannot resolve their username, so the formatter emits author: {}.
    for author in [json!({}), json!({"username":"reviewer"})] {
        let mut document = fixtures::external("gitlab");
        document[0]["notes"] = json!([{
            "body":"Historic comment", "author":author,
            "created_at":"2026-10-22T04:55:09.823Z", "system":false
        }]);
        preserve_gitlab_export(document).await;
    }
}

#[tokio::test]
async fn gitlab_planning_export_fields_remain_strict() {
    for planning in ["milestone", "iteration"] {
        let document = gitlab_planning_document(planning);
        for pointer in ["/0".to_owned(), format!("/0/{planning}")] {
            let mut unknown = document.clone();
            unknown
                .pointer_mut(&pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("unmapped".into(), json!("private-value"));
            reject("gitlab", unknown).await;
        }
        for field in ["id", "iid", "title", "description", "state"] {
            let mut missing = document.clone();
            missing[0][planning].as_object_mut().unwrap().remove(field);
            reject("gitlab", missing).await;
            let numeric =
                matches!(field, "id" | "iid") || (field == "state" && planning == "iteration");
            let mut invalid_values = vec![Value::Null, json!(true), json!([]), json!({})];
            invalid_values.push(if numeric { json!("1") } else { json!(1) });
            for value in invalid_values {
                let mut invalid = document.clone();
                invalid[0][planning][field] = value;
                reject("gitlab", invalid).await;
            }
        }
        for field in ["start_date", "due_date"] {
            for value in [Value::Null, json!(1), json!(false), json!([]), json!({})] {
                let mut invalid = document.clone();
                invalid[0][planning][field] = value;
                reject("gitlab", invalid).await;
            }
        }
        for value in [
            Value::Null,
            json!(1),
            json!(false),
            json!("Release"),
            json!([]),
        ] {
            let mut invalid = document.clone();
            invalid[0][planning] = value;
            reject("gitlab", invalid).await;
        }
    }
}

#[tokio::test]
async fn gitlab_export_deleted_comment_authors_keep_strict_note_and_user_shapes() {
    let mut document = fixtures::external("gitlab");
    document[0]["notes"] = json!([{
        "body":"Historic comment", "author":{},
        "created_at":"2026-10-22T04:55:09.823Z", "system":false
    }]);
    for pointer in ["/0", "/0/notes/0", "/0/notes/0/author"] {
        let mut unknown = document.clone();
        unknown
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unmapped".into(), json!("private-value"));
        reject("gitlab", unknown).await;
    }
    for field in ["body", "author", "created_at", "system"] {
        let mut missing = document.clone();
        missing[0]["notes"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        reject("gitlab", missing).await;
    }
    for value in [Value::Null, json!(1), json!(false), json!([]), json!({})] {
        let mut invalid = document.clone();
        invalid[0]["notes"][0]["author"]["username"] = value;
        reject("gitlab", invalid).await;
    }
    for value in [Value::Null, json!(1), json!(false), json!("reviewer")] {
        let mut invalid = document.clone();
        invalid[0]["notes"][0]["author"] = value;
        reject("gitlab", invalid).await;
    }
    // Only a note's author can lose its username after account deletion.
    // Issue authors and assignees still require their mapped username.
    for (field, value) in [("author", json!({})), ("assignees", json!([{}]))] {
        let mut invalid = fixtures::external("gitlab");
        invalid[0][field] = value;
        reject("gitlab", invalid).await;
    }
}

#[test]
#[ignore = "requires WEKAN_CORRECTED_SPEC pointing to a freshly generated corrected contract"]
fn corrected_contract_maps_external_planning_and_deleted_comment_authors() {
    let path = std::env::var_os("WEKAN_CORRECTED_SPEC").expect("set WEKAN_CORRECTED_SPEC");
    let spec: serde_yaml_ng::Value =
        serde_yaml_ng::from_reader(std::fs::File::open(path).unwrap()).unwrap();
    let definitions = &spec["definitions"];
    let issue = &definitions["VerifiedExportsExternal_documentsNativeGitlabExportItem"];
    assert_eq!(issue["additionalProperties"], false);
    assert_eq!(
        issue["properties"]["milestone"]["$ref"],
        "#/definitions/VerifiedExportsExternal_documentsNativeGitlabExportItemMilestone"
    );
    for (name, state_type) in [("Milestone", "string"), ("Iteration", "number")] {
        let record =
            &definitions[format!("VerifiedExportsExternal_documentsNativeGitlabExportItem{name}")];
        assert_eq!(record["additionalProperties"], false);
        let required: Vec<_> = record["required"]
            .as_sequence()
            .unwrap()
            .iter()
            .map(|field| field.as_str().unwrap())
            .collect();
        assert_eq!(required, ["id", "iid", "title", "description", "state"]);
        for (field, kind) in [
            ("id", "number"),
            ("iid", "number"),
            ("title", "string"),
            ("description", "string"),
            ("state", state_type),
            ("start_date", "string"),
            ("due_date", "string"),
        ] {
            assert_eq!(record["properties"][field]["type"], kind);
            assert!(record["properties"][field]["x-nullable"].is_null());
        }
    }
    let note_author =
        &definitions["VerifiedExportsExternal_documentsNativeGitlabExportItemNotesItemAuthor"];
    assert_eq!(note_author["additionalProperties"], false);
    assert!(note_author["required"].is_null());
    assert_eq!(note_author["properties"]["username"]["type"], "string");
    assert!(note_author["properties"]["username"]["x-nullable"].is_null());
    for name in ["Author", "AssigneesItem"] {
        let record =
            &definitions[format!("VerifiedExportsExternal_documentsNativeGitlabExportItem{name}")];
        assert_eq!(
            record["required"].as_sequence().unwrap(),
            &vec![serde_yaml_ng::Value::from("username")]
        );
    }
    for (suffix, identity) in [
        ("NativeKanboardExportTasksItemCommentsItem", "username"),
        ("NativeDeckExportStacksItemCardsItemCommentsItem", "actorId"),
        (
            "NativeOpenprojectExportEmbeddedElementsItemFixedEmbeddedActivitiesItemLinksUser",
            "title",
        ),
        (
            "NativeJiraExportIssuesItemFieldsFixedCommentCommentsItemAuthor",
            "name",
        ),
        ("NativeAsanaExportDataItemStoriesItemCreatedBy", "name"),
        ("NativeZenkitExportItemsItemCommentsItem", "author"),
    ] {
        let record = &definitions[format!("VerifiedExportsExternal_documents{suffix}")];
        assert_eq!(record["additionalProperties"], false);
        assert_eq!(record["properties"][identity]["type"], "string");
        assert!(record["properties"][identity]["x-nullable"].is_null());
        assert!(
            !record["required"]
                .as_sequence()
                .is_some_and(|fields| fields.contains(&serde_yaml_ng::Value::from(identity)))
        );
    }
    let prefix = "VerifiedExportsExternal_documentsNativeOpenprojectExportEmbedded";
    for (suffix, required, end_date) in [
        (
            "VersionsItem",
            vec!["_type", "id", "name", "status"],
            "endDate",
        ),
        (
            "SprintsItem",
            vec!["_type", "id", "name", "_links"],
            "finishDate",
        ),
    ] {
        let record = &definitions[format!("{prefix}{suffix}")];
        assert_eq!(record["additionalProperties"], false);
        assert_eq!(
            record["required"].as_sequence().unwrap(),
            &required
                .into_iter()
                .map(serde_yaml_ng::Value::from)
                .collect::<Vec<_>>()
        );
        for field in ["startDate", end_date] {
            assert_eq!(record["properties"][field]["type"], "string");
            assert!(record["properties"][field]["x-nullable"].is_null());
        }
        assert_eq!(
            record["properties"]["description"]["$ref"],
            format!("#/definitions/{prefix}VersionsItemDescription")
        );
    }
    let links = &definitions[format!("{prefix}ElementsItemFixedLinks")];
    let version_name = format!("{prefix}ElementsItemFixedLinksVersion");
    assert_eq!(
        links["properties"]["version"]["$ref"],
        format!("#/definitions/{version_name}")
    );
    let version = &definitions[version_name];
    assert_eq!(version["additionalProperties"], false);
    assert_eq!(
        version["required"].as_sequence().unwrap(),
        &vec![
            serde_yaml_ng::Value::from("href"),
            serde_yaml_ng::Value::from("title")
        ]
    );
    for field in ["href", "title"] {
        assert_eq!(version["properties"][field]["type"], "string");
        assert!(version["properties"][field]["x-nullable"].is_null());
    }
}

#[tokio::test]
async fn custom_field_export_metadata_is_optional_strict_and_preserves_native_bytes() {
    let server = MockServer::start().await;
    let mut board = fixtures::board();
    board["customFields"] = json!([{
        "_id":"field", "name":"Score", "type":"number", "settings":{},
        "showOnCard":false, "automaticallyOnCard":false, "alwaysOnCard":false,
        "showLabelOnMiniCard":false, "showSumAtTopOfList":false,
        "modifiedAt":"2026-10-21T00:00:00Z"
    }]);
    for metadata in [
        json!({}),
        json!({"readOnly":false,"adminOnly":false}),
        json!({"readOnly":true,"adminOnly":true,"cardKinds":["story","task"],"sort":2.5}),
        json!({"readOnly":false,"adminOnly":true,"cardKinds":[],"sort":0}),
        json!({"type":"dropdownMultiSelect","settings":{"dropdownItems":[{"_id":"a","name":"A"}]}}),
        json!({"settings":{"jiraTimeField":"original"}}),
        json!({"settings":{"jiraEstimateFieldId":"customfield_10001","jiraEstimateUnit":"points"}}),
        json!({"settings":{"jiraTimeField":null,"jiraEstimateFieldId":null,"jiraEstimateUnit":null}}),
    ] {
        server.reset().await;
        let mut document = board.clone();
        document["customFields"][0]
            .as_object_mut()
            .unwrap()
            .extend(metadata.as_object().unwrap().clone());
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
    for (field, values) in [
        (
            "readOnly",
            vec![Value::Null, json!(0), json!("false"), json!([]), json!({})],
        ),
        (
            "adminOnly",
            vec![Value::Null, json!(0), json!("true"), json!([]), json!({})],
        ),
        (
            "cardKinds",
            vec![
                Value::Null,
                json!("story"),
                json!([null]),
                json!([1]),
                json!([{}]),
            ],
        ),
        (
            "sort",
            vec![
                Value::Null,
                json!("2.5"),
                json!(false),
                json!([]),
                json!({}),
            ],
        ),
    ] {
        for value in values {
            let mut document = board.clone();
            document["customFields"][0][field] = value;
            reject("json", document).await;
        }
    }
    let mut unknown = board.clone();
    unknown["customFields"][0]["unmapped"] = json!("private-value");
    reject("json", unknown).await;
    for settings in [
        json!({"jiraTimeField":true}),
        json!({"jiraEstimateFieldId":42}),
        json!({"jiraEstimateUnit":[]}),
        json!({"jiraTimeField":"original","unmapped":"private-value"}),
        json!({"dropdownItems":[{"_id":"a","name":"A","unmapped":true}]}),
    ] {
        let mut invalid = board.clone();
        invalid["customFields"][0]["settings"] = settings;
        reject("json", invalid).await;
    }
    board["customFields"][0]
        .as_object_mut()
        .unwrap()
        .remove("name");
    reject("json", board).await;
}

#[tokio::test]
async fn movement_activity_export_reasons_are_optional_strict_and_preserve_native_bytes() {
    let server = MockServer::start().await;
    for reason in [None, Some(""), Some("Ready for review")] {
        server.reset().await;
        let mut board = fixtures::board();
        board["activities"] = json!([
            {"_id":"ordinary", "activityType":"createCard"},
            {"_id":"move", "activityType":"moveCard"},
            {"_id":"transfer", "activityType":"moveCardBoard"}
        ]);
        if let Some(reason) = reason {
            for activity in board["activities"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .skip(1)
            {
                activity["moveReason"] = json!(reason);
            }
        }
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
    }
    for reason in [Value::Null, json!(42), json!(false), json!([]), json!({})] {
        let mut board = fixtures::board();
        board["activities"] =
            json!([{"_id":"move", "activityType":"moveCard", "moveReason":reason}]);
        reject("json", board).await;
    }
    for invalid in [
        json!({"_id":"move", "moveReason":"Review", "unmapped":"private-value"}),
        json!({"activityType":"moveCard", "moveReason":"Review"}),
    ] {
        let mut board = fixtures::board();
        board["activities"] = json!([invalid]);
        reject("json", board).await;
    }
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

#[tokio::test]
async fn native_exports_preserve_strict_label_member_and_item_metadata() {
    let server = MockServer::start().await;
    let mut board = fixtures::board();
    board["labels"] = json!([{"_id":"label","name":"Milestone","color":"red"}]);
    board["members"] = json!([{"userId":"member","isAdmin":false,"isActive":true}]);
    board["checklistItems"] = json!([{"_id":"item","title":"Verify","sort":0,"isFinished":false,"checklistId":"checklist","cardId":"card","modifiedAt":"2030-01-02T03:04:05Z"}]);
    for present in [false, true] {
        server.reset().await;
        let mut document = board.clone();
        if present {
            document["labels"][0]["dueAt"] = json!("2030-01-02T03:04:05Z");
            document["labels"][0]["sharedLabelId"] = json!("shared");
            document["members"][0]["grantedBy"] = json!(["team:team", "org:org"]);
            document["members"][0]["customRoleId"] = json!("role");
            document["checklistItems"][0]["assigneeId"] = json!("member");
            document["checklistItems"][0]["linkedCardId"] = json!("subtask");
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
    for (collection, field, invalid) in [
        (
            "labels",
            "dueAt",
            vec![
                Value::Null,
                json!(1),
                json!(true),
                json!([]),
                json!({}),
                json!("not-a-date"),
            ],
        ),
        (
            "labels",
            "sharedLabelId",
            vec![Value::Null, json!(1), json!(true), json!([]), json!({})],
        ),
        (
            "members",
            "grantedBy",
            vec![
                Value::Null,
                json!(1),
                json!(true),
                json!("team:team"),
                json!([null]),
                json!([1]),
                json!({}),
            ],
        ),
        (
            "members",
            "customRoleId",
            vec![Value::Null, json!(1), json!(true), json!([]), json!({})],
        ),
        (
            "checklistItems",
            "assigneeId",
            vec![Value::Null, json!(1), json!(true), json!([]), json!({})],
        ),
        (
            "checklistItems",
            "linkedCardId",
            vec![Value::Null, json!(1), json!(true), json!([]), json!({})],
        ),
    ] {
        for value in invalid {
            let mut document = board.clone();
            document[collection][0][field] = value;
            reject("json", document).await;
        }
    }
    for collection in ["labels", "members", "checklistItems"] {
        let mut document = board.clone();
        document[collection][0]["unmapped"] = json!("secret-value");
        reject("json", document).await;
    }
}
