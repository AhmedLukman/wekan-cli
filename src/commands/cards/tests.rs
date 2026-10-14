use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

use clap::Parser;
use secrecy::SecretString;
use serde_json::json;
use time::{Duration, OffsetDateTime};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, method, path},
};

use super::CardCommand;
use crate::{
    cli::Cli,
    client::WekanClientFactory,
    command_result::{CardDeleteMode, CardSubmittedField, CommandSuccess, DestructiveOperation},
    commands::RootCommand,
    credentials::{
        CredentialCreateOutcome, CredentialDeleteOutcome, CredentialError, CredentialMutation,
        CredentialRecord, CredentialStore, CredentialTarget,
    },
    error::{AppError, ErrorCode},
    input::{ConfirmationProvider, FakeConfirmationProvider},
};

struct FakeCredentialStore {
    record: Mutex<Option<CredentialRecord>>,
    replacement_on_lock: Mutex<Option<CredentialRecord>>,
    lock_count: AtomicUsize,
}

impl FakeCredentialStore {
    fn authenticated(server: &MockServer) -> Self {
        Self {
            record: Mutex::new(Some(CredentialRecord::new(
                format!("{}/", server.uri()),
                "user-1".to_owned(),
                SecretString::from("card-token".to_owned()),
                OffsetDateTime::now_utc() + Duration::hours(1),
            ))),
            replacement_on_lock: Mutex::new(None),
            lock_count: AtomicUsize::new(0),
        }
    }

    fn replace_on_next_lock(&self, replacement: CredentialRecord) {
        *self.replacement_on_lock.lock().unwrap() = Some(replacement);
    }
}

impl CredentialStore for FakeCredentialStore {
    fn check_available(&self, _target: &CredentialTarget) -> Result<(), CredentialError> {
        Ok(())
    }

    fn exists(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        Ok(self.record.lock().unwrap().is_some())
    }

    fn load(
        &self,
        _target: &CredentialTarget,
    ) -> Result<Option<CredentialRecord>, CredentialError> {
        Ok(self.record.lock().unwrap().clone())
    }

    fn create(
        &self,
        _target: &CredentialTarget,
        _record: &CredentialRecord,
    ) -> Result<CredentialCreateOutcome, CredentialError> {
        panic!("card commands must not store credentials")
    }

    fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("card commands must not delete credentials")
    }

    fn delete_if_matches(
        &self,
        _target: &CredentialTarget,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("card commands must not delete credentials")
    }

    fn lock_mutation(
        &self,
        _target: &CredentialTarget,
    ) -> Result<Box<dyn CredentialMutation + Send + '_>, CredentialError> {
        self.lock_count.fetch_add(1, Ordering::SeqCst);
        if let Some(replacement) = self.replacement_on_lock.lock().unwrap().take() {
            *self.record.lock().unwrap() = Some(replacement);
        }
        Ok(Box::new(FakeCredentialMutation { store: self }))
    }
}

struct FakeCredentialMutation<'a> {
    store: &'a FakeCredentialStore,
}

impl CredentialMutation for FakeCredentialMutation<'_> {
    fn exists(&self) -> Result<bool, CredentialError> {
        Ok(self.store.record.lock().unwrap().is_some())
    }

    fn load(&self) -> Result<Option<CredentialRecord>, CredentialError> {
        Ok(self.store.record.lock().unwrap().clone())
    }

    fn create(
        &self,
        _record: &CredentialRecord,
    ) -> Result<CredentialCreateOutcome, CredentialError> {
        panic!("card commands must not store credentials through a mutation guard")
    }

    fn delete(&self) -> Result<bool, CredentialError> {
        panic!("card commands must not delete credentials through a mutation guard")
    }

    fn delete_if_matches(
        &self,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("card commands must not delete credentials through a mutation guard")
    }
}

fn factory(server: &MockServer) -> WekanClientFactory {
    WekanClientFactory::for_profile(format!("{}/", server.uri()), "default".to_owned(), false)
}

#[tokio::test]
async fn membership_commands_render_lists_and_both_mutation_arrays_without_confirmation() {
    use crate::output::{OutputFormat, render_success};
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    let confirmation = FakeConfirmationProvider::declining();
    for kind in ["member", "assignee"] {
        for action in ["list", "add", "remove"] {
            server.reset().await;
            let response = if action == "list" {
                json!({"_id":"c","title":"Card","boardId":"b","listId":"l","archived":false,"swimlaneId":"s","createdAt":"2030-01-02T03:04:05Z","modifiedAt":"2030-01-02T03:04:05Z","dateLastActivity":"2030-01-02T03:04:05Z","userId":"u","type":"cardType-card","showActivities":false,"members":["u\u{1b}[31m"],"assignees":["a"]})
            } else {
                json!({"_id":"c","members":["u\u{1b}[31m"],"assignees":["a"]})
            };
            Mock::given(method(match action {
                "list" => "GET",
                "add" => "POST",
                _ => "DELETE",
            }))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
            let mut args = vec![kind, action];
            if action != "list" {
                args.push("u");
            }
            args.extend(["--board", "b", "--list", "l", "--card", "c"]);
            let result = dispatch(
                card_command(&args),
                &factory(&server),
                &store,
                &confirmation,
            )
            .await
            .unwrap();
            let data = serde_json::from_str::<serde_json::Value>(&render_success(
                OutputFormat::Json,
                &result,
            ))
            .unwrap()["data"]
                .clone();
            assert_eq!(data["card_id"], "c");
            assert_eq!(data["board_id"], "b");
            assert_eq!(data["list_id"], "l");
            if action != "list" {
                assert_eq!(data["action"], action);
                assert_eq!(data["kind"], kind);
                assert_eq!(data["assignees"], json!(["a"]));
                assert_eq!(data["members"], json!(["u\u{1b}[31m"]));
            }
            assert!(!render_success(OutputFormat::Human, &result).contains('\u{1b}'));
            assert_eq!(server.received_requests().await.unwrap().len(), 1);
        }
    }
    assert!(confirmation.requests().is_empty());
}

#[tokio::test]
async fn membership_mutation_protocol_failures_have_unknown_outcomes() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    for kind in ["member", "assignee"] {
        for action in ["add", "remove"] {
            server.reset().await;
            Mock::given(method(if action == "add" { "POST" } else { "DELETE" }))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(json!({"_id":"wrong","members":[],"assignees":[]})),
                )
                .expect(1)
                .mount(&server)
                .await;
            let error = dispatch(
                card_command(&[
                    kind, action, "u", "--board", "b", "--list", "l", "--card", "c",
                ]),
                &factory(&server),
                &store,
                &FakeConfirmationProvider::declining(),
            )
            .await
            .unwrap_err();
            assert_eq!(error.code(), ErrorCode::ProtocolError);
            assert_eq!(error.details().outcome_unknown, Some(true));
        }
    }
}

#[tokio::test]
async fn archive_restore_dispatch_typed_results_without_confirmation() {
    let server = MockServer::start().await;
    for (command, action, response) in [
        (
            "archive",
            "archive",
            json!({"_id":"card-1", "archived":true, "archivedAt":"2030-01-02T03:04:05Z"}),
        ),
        (
            "restore",
            "unarchive",
            json!({"_id":"card-1", "archived":false}),
        ),
    ] {
        Mock::given(method("POST"))
            .and(path(format!(
                "/api/boards/board-1/lists/list-1/cards/card-1/{action}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        let confirmation = FakeConfirmationProvider::declining();
        let store = FakeCredentialStore::authenticated(&server);
        let result = dispatch(
            card_command(&[command, "card-1", "--board", "board-1", "--list", "list-1"]),
            &factory(&server),
            &store,
            &confirmation,
        )
        .await
        .unwrap();
        match result {
            CommandSuccess::CardArchived(data) => {
                assert_eq!(
                    (
                        data.board_id.as_str(),
                        data.list_id.as_str(),
                        data.card_id.as_str()
                    ),
                    ("board-1", "list-1", "card-1")
                );
                assert!(data.archived);
                assert_eq!(data.archived_at, "2030-01-02T03:04:05Z");
            }
            CommandSuccess::CardRestored(data) => {
                assert_eq!(
                    (
                        data.board_id.as_str(),
                        data.list_id.as_str(),
                        data.card_id.as_str()
                    ),
                    ("board-1", "list-1", "card-1")
                );
                assert!(!data.archived);
            }
            other => panic!("unexpected result: {other:?}"),
        }
        assert!(confirmation.requests().is_empty());
        assert_eq!(store.lock_count.load(Ordering::SeqCst), 0);
    }
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn archive_restore_failures_preserve_status_uncertainty_and_safe_diagnostics() {
    for (command, action, archived) in [
        ("archive", "archive", true),
        ("restore", "unarchive", false),
    ] {
        for (status, body, expected_code, unknown) in [
            (401, json!(null), ErrorCode::AuthenticationRejected, None),
            (403, json!(null), ErrorCode::PermissionDenied, None),
            (404, json!(null), ErrorCode::NotFound, None),
            (500, json!(null), ErrorCode::ServerError, Some(true)),
            (
                200,
                json!({"_id":"card-1", "archived":archived, "card-token":"secret-value"}),
                ErrorCode::ProtocolError,
                Some(true),
            ),
            (
                200,
                json!({"_id":"other", "archived":archived, "archivedAt":"2030-01-02T03:04:05Z"}),
                ErrorCode::ProtocolError,
                Some(true),
            ),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path(format!(
                    "/api/boards/board-1/lists/list-1/cards/card-1/{action}"
                )))
                .respond_with(if status == 200 {
                    ResponseTemplate::new(status).set_body_json(body)
                } else {
                    ResponseTemplate::new(status)
                        .set_body_string("<!DOCTYPE html><title>Error</title>")
                })
                .expect(1)
                .mount(&server)
                .await;
            let error = dispatch(
                card_command(&[command, "card-1", "--board", "board-1", "--list", "list-1"]),
                &factory(&server),
                &FakeCredentialStore::authenticated(&server),
                &FakeConfirmationProvider::accepting(),
            )
            .await
            .unwrap_err();
            assert_eq!(error.code(), expected_code);
            assert_eq!(error.details().http_status, Some(status));
            assert_eq!(error.details().outcome_unknown, unknown);
            assert!(!format!("{error:?}").contains("card-token"));
            assert!(!format!("{error:?}").contains("secret-value"));
        }
    }
}

#[test]
fn archive_restore_accept_global_options_and_trim_identifiers() {
    for action in ["archive", "restore"] {
        let cli = Cli::try_parse_from([
            "wekan",
            "card",
            action,
            " card-1 ",
            "--board",
            " board-1 ",
            "--list",
            " list-1 ",
            "--server",
            "http://localhost:3000",
            "--profile",
            "local",
            "--output",
            "json",
            "--allow-insecure-http",
        ])
        .unwrap();
        assert_eq!(cli.server.as_deref(), Some("http://localhost:3000"));
        assert_eq!(cli.profile_name.as_deref(), Some("local"));
        assert_eq!(cli.output, crate::output::OutputFormat::Json);
        assert!(cli.allow_insecure_http);
        let RootCommand::Card(args) = cli.command else {
            panic!("expected card command")
        };
        match args.command {
            CardCommand::Archive(args) => assert_eq!(
                (
                    args.board_id.as_str(),
                    args.list_id.as_str(),
                    args.card_id.as_str()
                ),
                ("board-1", "list-1", "card-1")
            ),
            CardCommand::Restore(args) => assert_eq!(
                (
                    args.board_id.as_str(),
                    args.list_id.as_str(),
                    args.card_id.as_str()
                ),
                ("board-1", "list-1", "card-1")
            ),
            _ => panic!("expected archive or restore"),
        }
    }
}

#[tokio::test]
async fn archive_restore_transport_failures_report_unknown_outcomes() {
    for command in ["archive", "restore"] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let server_url = format!("http://{}/", listener.local_addr().unwrap());
        let store = FakeCredentialStore {
            record: Mutex::new(Some(CredentialRecord::new(
                server_url.clone(),
                "user-1".to_owned(),
                SecretString::from("card-token".to_owned()),
                OffsetDateTime::now_utc() + Duration::hours(1),
            ))),
            replacement_on_lock: Mutex::new(None),
            lock_count: AtomicUsize::new(0),
        };
        let factory = WekanClientFactory::for_profile(server_url, "default".to_owned(), false);
        drop(listener);
        let error = dispatch(
            card_command(&[command, "card-1", "--board", "board-1", "--list", "list-1"]),
            &factory,
            &store,
            &FakeConfirmationProvider::accepting(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::TransportError);
        assert_eq!(error.details().outcome_unknown, Some(true));
        assert!(!format!("{error:?}").contains("card-token"));
    }
}

#[tokio::test]
async fn linked_creation_and_extended_updates_send_typed_bodies_once() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/boards/board-1/lists/list-1/cards"))
        .and(body_json(
            json!({"linkedId":"source-card", "swimlaneId":"swimlane-1"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"linked-card"})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/boards/board-1/lists/list-1/cards/card-1"))
        .and(body_json(json!({
            "swimlaneId":"lane-2", "listId":"list-2", "newBoardId":"board-2",
            "newSwimlaneId":"lane-3", "newListId":"list-3", "archive":false,
            "vote":{"question":"Ship?","public":false,"allowNonBoardMembers":true},
            "poker":{"question":true,"oneHundred":["user-1"],"estimation":2.5},
            "customFields":[{"_id":"field-1","value":["a","b"]}],
            "stickers":[{"icon":"star","name":"Release","highlight":"round","position":2}],
            "locations":[{"_id":"location-1","name":"Office","latitude":-1.28,"longitude":36.82}]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"card-1"})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/boards/board-1/lists/list-1/cards"))
        .and(body_json(
            json!({"linkedId":"missing-card", "swimlaneId":"swimlane-1"}),
        ))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(json!({"error":"linkedId card not found"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);
    let confirmation = FakeConfirmationProvider::accepting();
    let result = dispatch(
        card_command(&[
            "create",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "--swimlane-id",
            "swimlane-1",
            "--linked-id",
            "source-card",
        ]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap();
    assert!(matches!(result, CommandSuccess::CardCreated(_)));
    let missing_link = dispatch(
        card_command(&[
            "create",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "--swimlane-id",
            "swimlane-1",
            "--linked-id",
            "missing-card",
        ]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap_err();
    assert_eq!(missing_link.code(), ErrorCode::NotFound);
    let result = dispatch(
        card_command(&[
            "update",
            "card-1",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "--swimlane-id",
            "lane-2",
            "--list-id",
            "list-2",
            "--new-board-id",
            "board-2",
            "--new-swimlane-id",
            "lane-3",
            "--new-list-id",
            "list-3",
            "--archive",
            "false",
            "--vote",
            r#"{"question":"Ship?","public":false,"allowNonBoardMembers":true}"#,
            "--poker",
            r#"{"question":true,"oneHundred":["user-1"],"estimation":2.5}"#,
            "--custom-fields",
            r#"[{"_id":"field-1","value":["a","b"]}]"#,
            "--stickers",
            r#"[{"icon":"star","name":"Release","highlight":"round","position":2}]"#,
            "--locations",
            r#"[{"_id":"location-1","name":"Office","latitude":-1.28,"longitude":36.82}]"#,
        ]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap();
    let CommandSuccess::CardUpdated(result) = result else {
        panic!("expected update");
    };
    assert_eq!(
        result.submitted_fields,
        vec![
            CardSubmittedField::SwimlaneId,
            CardSubmittedField::ListId,
            CardSubmittedField::NewBoardId,
            CardSubmittedField::NewSwimlaneId,
            CardSubmittedField::NewListId,
            CardSubmittedField::Archive,
            CardSubmittedField::Vote,
            CardSubmittedField::Poker,
            CardSubmittedField::CustomFields,
            CardSubmittedField::Stickers,
            CardSubmittedField::Locations,
        ]
    );
}

#[test]
fn structured_inputs_reject_unknown_fields_invalid_shapes_and_types() {
    for (flag, value) in [
        ("--vote", r#"{"future":true}"#),
        ("--vote", r#"{"public":"true"}"#),
        ("--vote", r#"{"public":null}"#),
        ("--poker", r#"{"future":true}"#),
        ("--poker", r#"{"question":"Estimate?"}"#),
        ("--custom-fields", r#"[{"_id":"f","future":true}]"#),
        ("--stickers", r#"[{"future":true}]"#),
        ("--stickers", r#"[{"highlight":"unknown"}]"#),
        ("--stickers", "[[]]"),
        ("--custom-fields", "[[]]"),
        ("--locations", "[[]]"),
        ("--locations", r#"[{"future":true}]"#),
        ("--locations", "{}"),
        ("--vote", "[]"),
        ("--poker", "[]"),
    ] {
        assert!(
            Cli::try_parse_from([
                "wekan", "card", "update", "card-1", "--board", "board-1", "--list", "list-1",
                flag, value
            ])
            .is_err(),
            "{flag} {value}"
        );
    }
    for (flag, value) in [
        ("--archive", "false"),
        ("--swimlane-id", "lane-2"),
        ("--list-id", "list-2"),
        ("--new-board-id", "board-2"),
        ("--new-swimlane-id", "lane-3"),
        ("--new-list-id", "list-3"),
        ("--vote", "{}"),
        ("--poker", "{}"),
        ("--custom-fields", "[]"),
        ("--stickers", "[]"),
        ("--locations", "[]"),
    ] {
        card_command(&[
            "update", "card-1", "--board", "board-1", "--list", "list-1", flag, value,
        ]);
    }
}

async fn dispatch(
    command: CardCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    super::dispatch(command, client_factory, credential_store, confirmation).await
}

fn card_command(arguments: &[&str]) -> CardCommand {
    let mut all = vec!["wekan", "--server", "https://wekan.example", "card"];
    all.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(all).unwrap();
    let RootCommand::Card(args) = cli.command else {
        panic!("expected card command")
    };
    args.command
}

#[tokio::test]
async fn bulk_creation_rejects_oversized_and_nonregular_files_before_http() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("cards.json");
    std::fs::File::create(&path)
        .unwrap()
        .set_len(50 * 1024 * 1024 + 1)
        .unwrap();
    for input in [path.as_path(), directory.path()] {
        let error = dispatch(
            card_command(&[
                "bulk",
                "create",
                "--board",
                "b",
                "--list",
                "l",
                "--cards-file",
                input.to_str().unwrap(),
            ]),
            &factory(&server),
            &store,
            &FakeConfirmationProvider::accepting(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidInput);
        assert!(error.message().contains("no request was sent"));
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn bulk_creation_preserves_files_and_reports_complete_mixed_results() {
    use crate::output::{OutputFormat, render_error, render_success};
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    let request = r#"[{"title":null,"members":null},{"title":"Next"}]"#;
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), request).unwrap();
    for source in ["--cards", "--cards-file"] {
        for failed in [0, 1, 2] {
            server.reset().await;
            let response: Vec<_> = (0..2)
                .map(|index| {
                    if index < failed {
                        json!({"index":index,"error":"card-token\nerror"})
                    } else {
                        json!({"index":index,"_id":format!("c{index}")})
                    }
                })
                .collect();
            Mock::given(method("POST")).and(path("/api/boards/b/lists/l/cards/bulk"))
                .and(body_json(json!({"swimlaneId":"s","cards":[{"title":null,"members":null},{"title":"Next"}]})))
                .respond_with(ResponseTemplate::new(200).set_body_json(response)).expect(1).mount(&server).await;
            let result = dispatch(
                card_command(&[
                    "bulk",
                    "create",
                    "--board",
                    "b",
                    "--list",
                    "l",
                    "--swimlane-id",
                    "s",
                    source,
                    if source == "--cards" {
                        request
                    } else {
                        file.path().to_str().unwrap()
                    },
                ]),
                &factory(&server),
                &store,
                &FakeConfirmationProvider::declining(),
            )
            .await;
            if failed == 0 {
                let success = result.unwrap();
                let value: serde_json::Value =
                    serde_json::from_str(&render_success(OutputFormat::Json, &success)).unwrap();
                assert_eq!(
                    value["data"]["results"][0],
                    json!({"index":0,"card_id":"c0","error":null})
                );
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.code(), ErrorCode::BatchIncomplete);
                assert_eq!(error.exit_code(), crate::exit_code::StableExitCode::Server);
                assert_eq!(error.details().http_status, Some(200));
                let rendered = render_error(OutputFormat::Json, &error);
                assert!(!rendered.contains("card-token"));
                let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
                assert_eq!(
                    value["error"]["details"]["batch_result"]["operation"],
                    "card_bulk_create"
                );
                assert_eq!(
                    value["error"]["details"]["batch_result"]["data"]["results"]
                        .as_array()
                        .unwrap()
                        .len(),
                    2
                );
                let human = render_error(OutputFormat::Human, &error);
                assert!(human.contains("Bulk card creation:"));
                assert!(human.contains("index"));
            }
        }
    }
}

#[tokio::test]
async fn bulk_delete_guards_confirmation_and_preserves_incomplete_results() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    let args = [
        "bulk", "delete", "--board", "b", "--card", "c", "--card", "missing",
    ];
    let prompt = FakeConfirmationProvider::declining();
    let cancelled = dispatch(card_command(&args), &factory(&server), &store, &prompt)
        .await
        .unwrap();
    assert!(
        matches!(cancelled,CommandSuccess::Cancelled(data) if data.operation==DestructiveOperation::CardBulkDelete)
    );
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(prompt.requests().len(), 1);
    assert!(
        prompt.requests()[0]
            .prompt()
            .contains("retains attachment metadata and file bytes")
    );
    let replacement = CredentialRecord::new(
        format!("{}/", server.uri()),
        "user-2".into(),
        SecretString::from("changed".to_owned()),
        OffsetDateTime::now_utc() + Duration::hours(1),
    );
    store.replace_on_next_lock(replacement.clone());
    let prompt = FakeConfirmationProvider::accepting();
    let error = dispatch(card_command(&args), &factory(&server), &store, &prompt)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
    assert_eq!(prompt.requests().len(), 1);
    assert!(
        store
            .record
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .matches(&replacement)
    );
    assert!(server.received_requests().await.unwrap().is_empty());
    let store = FakeCredentialStore::authenticated(&server);
    Mock::given(method("DELETE"))
        .and(path("/api/boards/b/cards/bulk"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"deleted":["c"],"notFound":["missing"]})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let mut confirmed = args.to_vec();
    confirmed.push("--yes");
    let error = dispatch(card_command(&confirmed), &factory(&server), &store, &prompt)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::BatchIncomplete);
    assert!(
        matches!(error.details().batch_result.as_ref(),Some(crate::command_result::CardBatchResult::CardBulkDelete(data)) if data.deleted==["c"] && data.not_found==["missing"])
    );
}

#[tokio::test]
async fn bulk_delete_rechecks_replacement_and_removal_during_confirmation() {
    struct ChangingConfirmation<'a> {
        store: &'a FakeCredentialStore,
        replacement: Option<CredentialRecord>,
    }
    impl ConfirmationProvider for ChangingConfirmation<'_> {
        fn confirm(&self, _: &crate::input::ConfirmationRequest) -> Result<bool, AppError> {
            // Native-vault edits can bypass the CLI's file lock while it prompts.
            std::thread::scope(|scope| {
                scope
                    .spawn(|| {
                        *self.store.record.lock().unwrap() = self.replacement.clone();
                    })
                    .join()
                    .unwrap();
            });
            Ok(true)
        }
    }
    for removed in [false, true] {
        let server = MockServer::start().await;
        let store = FakeCredentialStore::authenticated(&server);
        let replacement = (!removed).then(|| {
            CredentialRecord::new(
                format!("{}/", server.uri()),
                "user-2".into(),
                SecretString::from("replacement-token".to_owned()),
                OffsetDateTime::now_utc() + Duration::hours(1),
            )
        });
        let prompt = ChangingConfirmation {
            store: &store,
            replacement: replacement.clone(),
        };
        let error = dispatch(
            card_command(&["bulk", "delete", "--board", "b", "--card", "c"]),
            &factory(&server),
            &store,
            &prompt,
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.code(),
            if removed {
                ErrorCode::CredentialNotFound
            } else {
                ErrorCode::CredentialStoreFailed
            }
        );
        assert_eq!(
            error.exit_code(),
            crate::exit_code::StableExitCode::Credential
        );
        assert!(error.message().contains("no HTTP request was sent"));
        assert!(server.received_requests().await.unwrap().is_empty());
        let current = store.record.lock().unwrap();
        if let Some(expected) = &replacement {
            assert!(current.as_ref().unwrap().matches(expected));
            assert_eq!(error.details().credential_stored, Some(true));
        } else {
            assert!(current.is_none());
        }
    }
}

#[tokio::test]
async fn bulk_delete_rechecks_expiry_after_confirmation() {
    struct ExpiringConfirmation {
        expires: OffsetDateTime,
    }
    impl ConfirmationProvider for ExpiringConfirmation {
        fn confirm(&self, _: &crate::input::ConfirmationRequest) -> Result<bool, AppError> {
            while OffsetDateTime::now_utc() < self.expires {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Ok(true)
        }
    }
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    let expires = OffsetDateTime::now_utc() + Duration::seconds(2);
    let original = CredentialRecord::new(
        format!("{}/", server.uri()),
        "user-1".into(),
        SecretString::from("card-token".to_owned()),
        expires,
    );
    *store.record.lock().unwrap() = Some(original.clone());
    let error = dispatch(
        card_command(&["bulk", "delete", "--board", "b", "--card", "c"]),
        &factory(&server),
        &store,
        &ExpiringConfirmation { expires },
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::CredentialExpired);
    assert_eq!(error.exit_code(), crate::exit_code::StableExitCode::Server);
    assert_eq!(
        error.details().token_expires.as_deref(),
        Some(
            expires
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap()
                .as_str()
        )
    );
    assert!(error.message().contains("no HTTP request was sent"));
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(
        store
            .record
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .matches(&original)
    );
}

#[tokio::test]
async fn bulk_label_results_render_null_elements_in_success_and_incomplete_batches() {
    use crate::output::{OutputFormat, render_error, render_success};
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    let expected =
        json!({"board_id":"b","updated":[{"card_id":"c","label_ids":[null,"a"]}],"not_found":[]});
    for incomplete in [false, true] {
        server.reset().await;
        let not_found = if incomplete { vec!["missing"] } else { vec![] };
        Mock::given(method("POST"))
            .and(path("/api/boards/b/cards/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "updated":[{"_id":"c","labelIds":[null,"a"]}],"notFound":not_found
            })))
            .expect(1)
            .mount(&server)
            .await;
        let mut args = vec![
            "bulk",
            "labels",
            "--board",
            "b",
            "--card",
            "c",
            "--add-label",
            "a",
        ];
        if incomplete {
            args.extend(["--card", "missing"]);
        }
        let result = dispatch(
            card_command(&args),
            &factory(&server),
            &store,
            &FakeConfirmationProvider::declining(),
        )
        .await;
        let (data, human) = if incomplete {
            let error = result.unwrap_err();
            assert_eq!(error.code(), ErrorCode::BatchIncomplete);
            let value: serde_json::Value =
                serde_json::from_str(&render_error(OutputFormat::Json, &error)).unwrap();
            assert_eq!(
                value["error"]["details"]["batch_result"]["operation"],
                "card_bulk_labels"
            );
            (
                value["error"]["details"]["batch_result"]["data"].clone(),
                render_error(OutputFormat::Human, &error),
            )
        } else {
            let success = result.unwrap();
            let value: serde_json::Value =
                serde_json::from_str(&render_success(OutputFormat::Json, &success)).unwrap();
            (
                value["data"].clone(),
                render_success(OutputFormat::Human, &success),
            )
        };
        let mut expected = expected.clone();
        expected["not_found"] = json!(not_found);
        assert_eq!(data, expected);
        assert!(human.contains("null"));
        assert!(human.contains("1 updated"));
    }
}

#[tokio::test]
async fn bulk_labels_map_partial_invalid_and_uncertain_server_failures() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    for (status, response, code) in [
        (
            200,
            json!({"updated":[{"_id":"c","labelIds":["a"]}],"notFound":["missing"]}),
            ErrorCode::BatchIncomplete,
        ),
        (
            400,
            json!({"error":"invalid labels","invalidLabelIds":["a"]}),
            ErrorCode::ServerError,
        ),
        (
            500,
            json!({"error":"server failed"}),
            ErrorCode::ServerError,
        ),
    ] {
        server.reset().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(status).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        let error = dispatch(
            card_command(&[
                "bulk",
                "labels",
                "--board",
                "b",
                "--card",
                "c",
                "--card",
                "missing",
                "--add-label",
                "a",
            ]),
            &factory(&server),
            &store,
            &FakeConfirmationProvider::declining(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), code);
        assert_eq!(error.details().http_status, Some(status));
        assert_eq!(
            error.details().outcome_unknown,
            (status == 500).then_some(true)
        );
        if status == 400 {
            assert_eq!(
                error.details().invalid_label_ids.as_deref(),
                Some(["a".to_owned()].as_slice())
            );
        }
    }
}

#[tokio::test]
async fn bulk_count_boundaries_send_one_request_or_fail_before_http() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    for count in [1, 500, 501] {
        server.reset().await;
        let cards = serde_json::to_string(&vec![json!({}); count]).unwrap();
        if count <= 500 {
            let results: Vec<_> = (0..count)
                .map(|i| json!({"index":i,"_id":format!("c{i}")}))
                .collect();
            Mock::given(method("POST"))
                .respond_with(ResponseTemplate::new(200).set_body_json(results))
                .expect(1)
                .mount(&server)
                .await;
        }
        let result = dispatch(
            card_command(&[
                "bulk", "create", "--board", "b", "--list", "l", "--cards", &cards,
            ]),
            &factory(&server),
            &store,
            &FakeConfirmationProvider::declining(),
        )
        .await;
        assert_eq!(result.is_ok(), count <= 500);
        if count > 500 {
            assert_eq!(result.unwrap_err().code(), ErrorCode::InvalidInput);
            assert!(server.received_requests().await.unwrap().is_empty());
        }
    }
}

#[tokio::test]
async fn card_label_projection_preserves_null_and_decodes_the_whole_card() {
    use crate::output::{OutputFormat, render_success};
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    let mut card = json!({"_id":"c","boardId":"b","listId":"l","archived":false,"swimlaneId":"s","createdAt":"2030-01-02T03:04:05Z","modifiedAt":"2030-01-02T03:04:05Z","dateLastActivity":"2030-01-02T03:04:05Z","userId":"u","type":"cardType-card","showActivities":false,"labelIds":[null,"a"]});
    for unknown in [false, true] {
        server.reset().await;
        if unknown {
            card["future"] = json!(1);
        }
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&card))
            .mount(&server)
            .await;
        let result = dispatch(
            card_command(&[
                "label", "list", "--board", "b", "--list", "l", "--card", "c",
            ]),
            &factory(&server),
            &store,
            &FakeConfirmationProvider::declining(),
        )
        .await;
        if unknown {
            assert_eq!(result.unwrap_err().code(), ErrorCode::ProtocolError);
        } else {
            let data: serde_json::Value =
                serde_json::from_str(&render_success(OutputFormat::Json, &result.unwrap()))
                    .unwrap();
            assert_eq!(data["data"]["label_ids"], json!([null, "a"]));
        }
    }
}

#[test]
fn parser_exposes_crud_and_enforces_update_contract() {
    for arguments in [
        vec!["list", "--board", "board-1", "--list", "list-1"],
        vec!["get", "--board", "board-1", "--list", "list-1", "card-1"],
        vec![
            "create",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "--title",
            "Todo",
            "--swimlane-id",
            "swimlane-1",
        ],
        vec![
            "update",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "card-1",
            "--clear-due-at",
        ],
        vec![
            "update",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "card-1",
            "--sort",
            "0",
            "--spent-time",
            "0",
            "--is-over-time",
            "false",
        ],
        vec![
            "update",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "card-1",
            "--label",
            "label-1",
            "--member",
            "user-1",
            "--due-complete",
            "false",
        ],
        vec![
            "delete", "--board", "board-1", "--list", "list-1", "card-1", "--yes",
        ],
    ] {
        card_command(&arguments);
    }

    for invalid in [
        vec![
            "wekan", "card", "update", "--board", "board-1", "--list", "list-1", "card-1",
        ],
        vec![
            "wekan",
            "card",
            "update",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "card-1",
            "--due-at",
            "2030-01-01T00:00:00Z",
            "--clear-due-at",
        ],
        vec![
            "wekan",
            "card",
            "update",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "card-1",
            "--label",
            "label-1",
            "--clear-labels",
        ],
        vec![
            "wekan",
            "card",
            "update",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "card-1",
            "--member",
            "user-1",
            "--clear-members",
        ],
        vec![
            "wekan",
            "card",
            "update",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "card-1",
            "--assignee",
            "user-1",
            "--clear-assignees",
        ],
        vec![
            "wekan",
            "card",
            "update",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "card-1",
            "--spent-time",
            "NaN",
        ],
        vec![
            "wekan", "card", "update", "--board", "board-1", "--list", "list-1", "card-1",
            "--sort", "NaN",
        ],
        vec![
            "wekan",
            "card",
            "update",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "card-1",
            "--is-over-time",
            "maybe",
        ],
        vec![
            "wekan", "card", "update", "--board", "board-1", "--list", "list-1", "card-1",
            "--color", "belize",
        ],
        vec![
            "wekan", "card", "get", "--board", "board-1", "--list", "list-1", "card-1", "--yes",
        ],
        vec!["wekan", "card", "move", "board-1", "list-1", "card-1"],
        vec!["wekan", "card", "archive", "board-1", "list-1", "card-1"],
    ] {
        assert!(Cli::try_parse_from(invalid).is_err());
    }
}

#[test]
fn parser_preserves_text_and_delegates_date_interpretation_to_wekan() {
    assert!(
        Cli::try_parse_from(["wekan", "card", "list", "--board", " ", "--list", "list-1"]).is_err()
    );
    for text in ["", " ", "😀"] {
        let CardCommand::Update(args) = card_command(&[
            "update",
            "card-1",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "--description",
            text,
        ]) else {
            panic!("expected update")
        };
        assert_eq!(args.description.as_deref(), Some(text));
    }
    for date in [
        "2030-01-01",
        "January 1, 2030",
        "253402300800000",
        "-62198755200000",
    ] {
        card_command(&[
            "create",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "--swimlane-id",
            "swimlane-1",
            "--due-at",
            date,
        ]);
    }
    let accepted = "😀".repeat(501);
    card_command(&[
        "update", "--board", "board-1", "--list", "list-1", "card-1", "--title", &accepted,
    ]);
}

#[tokio::test]
async fn commands_return_typed_results_and_canonical_submitted_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board%2F1/lists/list%2F1/cards"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "card-1",
            "title": "Todo",
            "assignees": []
        }])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board%2F1/lists/list%2F1/cards/card%2F1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "card/1",
            "title": "Todo",
            "archived": false,
            "swimlaneId": "swimlane-1",
            "createdAt": "2030-01-02T03:04:05Z",
            "modifiedAt": "2030-01-02T03:04:05Z",
            "dateLastActivity": "2030-01-02T03:04:05Z",
            "userId": "user-1",
            "type": "cardType-card",
            "showActivities": false
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/boards/board%2F1/lists/list%2F1/cards"))
        .and(body_json(json!({
            "title": " Todo ",
            "swimlaneId": "swimlane-1",
            "members": ["user-1"],
            "dueAt": "2030-01-03T00:00:00Z"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "card-2"})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/boards/board%2F1/lists/list%2F1/cards/card%2F1"))
        .and(body_json(json!({
            "title": "Doing",
            "sort": 3.5,
            "labelIds": [],
            "dueAt": "",
            "isOverTime": true,
            "members": ["user-2"],
            "assignees": [],
            "dueComplete": false
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "card/1"})))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);
    let confirmation = FakeConfirmationProvider::accepting();

    let CommandSuccess::CardCollection(collection) = dispatch(
        card_command(&["list", "--board", "board/1", "--list", "list/1"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected card collection")
    };
    assert_eq!(collection.board_id, "board/1");
    assert_eq!(collection.list_id, "list/1");
    assert_eq!(collection.cards[0].card_id, "card-1");

    let CommandSuccess::CardShown(card) = dispatch(
        card_command(&["get", "--board", "board/1", "--list", "list/1", "card/1"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected card detail")
    };
    assert_eq!(card.card_id, "card/1");
    assert!(card.members.is_empty());

    let CommandSuccess::CardCreated(created) = dispatch(
        card_command(&[
            "create",
            "--board",
            "board/1",
            "--list",
            "list/1",
            "--title",
            " Todo ",
            "--swimlane-id",
            "swimlane-1",
            "--member",
            "user-1",
            "--due-at",
            "2030-01-03T00:00:00Z",
        ]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected card creation")
    };
    assert_eq!(created.card_id, "card-2");

    let CommandSuccess::CardUpdated(updated) = dispatch(
        card_command(&[
            "update",
            "--board",
            "board/1",
            "--list",
            "list/1",
            "card/1",
            "--due-complete",
            "false",
            "--clear-assignees",
            "--member",
            "user-2",
            "--clear-due-at",
            "--clear-labels",
            "--title",
            "Doing",
            "--sort",
            "3.5",
            "--is-over-time",
            "true",
        ]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected card update")
    };
    assert_eq!(
        updated.submitted_fields,
        [
            CardSubmittedField::Title,
            CardSubmittedField::Sort,
            CardSubmittedField::LabelIds,
            CardSubmittedField::DueAt,
            CardSubmittedField::IsOverTime,
            CardSubmittedField::Members,
            CardSubmittedField::Assignees,
            CardSubmittedField::DueComplete,
        ]
    );
}

#[tokio::test]
async fn card_errors_map_not_found_and_id_mismatches() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/list-1/cards/missing"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/boards/board-1/lists/list-1/cards/card-1"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"error": "not-found"})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/boards/board-1/lists/list-1/cards/card-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "different"})))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);
    let confirmation = FakeConfirmationProvider::accepting();

    let missing = dispatch(
        card_command(&["get", "--board", "board-1", "--list", "list-1", "missing"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::NotFound);
    assert_eq!(missing.details().http_status, Some(200));
    assert_eq!(missing.details().wekan_status_code, Some(404));

    let update_missing = dispatch(
        card_command(&[
            "update",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "card-1",
            "--title",
            "Doing",
            "--parent-id",
            "missing-parent",
        ]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap_err();
    assert_eq!(update_missing.code(), ErrorCode::NotFound);
    assert_eq!(update_missing.details().outcome_unknown, Some(true));

    let mismatch = dispatch(
        card_command(&[
            "update", "--board", "board-1", "--list", "list-1", "card-2", "--title", "Doing",
        ]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap_err();
    assert_eq!(mismatch.code(), ErrorCode::ProtocolError);
    assert_eq!(mismatch.details().outcome_unknown, Some(true));
}

#[tokio::test]
async fn delete_confirms_guards_credentials_and_verifies_the_returned_id() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    let declining = FakeConfirmationProvider::declining();
    let cancelled = dispatch(
        card_command(&[
            "delete",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "card\u{1b}]52;c;x\u{7}",
        ]),
        &factory(&server),
        &store,
        &declining,
    )
    .await
    .unwrap();
    assert_eq!(
        cancelled,
        CommandSuccess::Cancelled(crate::command_result::CancellationSuccess::new(
            DestructiveOperation::CardDelete
        ))
    );
    assert!(!declining.requests()[0].prompt().contains('\u{1b}'));
    assert!(declining.requests()[0].prompt().contains("cascade"));

    let failing = FakeConfirmationProvider::failing(AppError::invalid_input("prompt failed"));
    let prompt_error = dispatch(
        card_command(&[
            "delete",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "card-prompt",
        ]),
        &factory(&server),
        &store,
        &failing,
    )
    .await
    .unwrap_err();
    assert_eq!(prompt_error.code(), ErrorCode::InvalidInput);

    Mock::given(method("DELETE"))
        .and(path("/api/boards/board-1/lists/list-1/cards/card-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "card-1"})))
        .expect(1)
        .mount(&server)
        .await;
    let CommandSuccess::CardDeleted(deleted) = dispatch(
        card_command(&[
            "delete", "--board", "board-1", "--list", "list-1", "card-1", "--yes",
        ]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap() else {
        panic!("expected card deletion")
    };
    assert!(deleted.deleted);
    assert_eq!(deleted.delete_mode, CardDeleteMode::Hard);

    Mock::given(method("DELETE"))
        .and(path("/api/boards/board-1/lists/list-1/cards/card-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "different"})))
        .expect(1)
        .mount(&server)
        .await;
    let mismatch = dispatch(
        card_command(&[
            "delete", "--board", "board-1", "--list", "list-1", "card-2", "--yes",
        ]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap_err();
    assert_eq!(mismatch.code(), ErrorCode::ProtocolError);
    assert_eq!(mismatch.details().outcome_unknown, Some(true));

    let replacement = CredentialRecord::new(
        format!("{}/", server.uri()),
        "user-2".to_owned(),
        SecretString::from("replacement-token".to_owned()),
        OffsetDateTime::now_utc() + Duration::hours(1),
    );
    store.replace_on_next_lock(replacement);
    let confirmation = FakeConfirmationProvider::accepting();
    let changed = dispatch(
        card_command(&["delete", "--board", "board-1", "--list", "list-1", "card-3"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap_err();
    assert_eq!(changed.code(), ErrorCode::CredentialStoreFailed);
    assert!(confirmation.requests().is_empty());
    assert_eq!(store.lock_count.load(Ordering::SeqCst), 5);
}

#[tokio::test]
async fn remaining_card_read_commands_route_and_render_scope() {
    use crate::output::{OutputFormat, render_success};
    for (args, route, body) in [
        (
            vec!["count", "--board", "b"],
            "/api/boards/b/cards_count",
            json!({"board_cards_count":0}),
        ),
        (
            vec!["count", "--board", "b", "--list", "l"],
            "/api/boards/b/lists/l/cards_count",
            json!({"list_cards_count":3}),
        ),
        (
            vec!["list", "--board", "b", "--swimlane", "s"],
            "/api/boards/b/swimlanes/s/cards",
            json!([{"_id":"c","listId":"l","title":"A\u{001b}[31m","assignees":[null,"u"]}]),
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let result = super::dispatch(
            card_command(&args),
            &factory(&server),
            &FakeCredentialStore::authenticated(&server),
            &FakeConfirmationProvider::declining(),
        )
        .await
        .unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &result)).unwrap();
        assert_eq!(value["data"]["board_id"], "b");
        assert!(!render_success(OutputFormat::Human, &result).contains('\u{001b}'));
        if args[0] == "list" {
            assert_eq!(value["data"]["swimlane_id"], "s");
            assert_eq!(value["data"]["cards"][0]["list_id"], "l");
        }
    }
}
