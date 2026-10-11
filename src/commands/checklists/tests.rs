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
    matchers::{body_json, header, method, path},
};

use crate::{
    cli::Cli,
    client::WekanClientFactory,
    command_result::{ChecklistItemSubmittedField, CommandSuccess, DestructiveOperation},
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
                SecretString::from("checklist-token".to_owned()),
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
        panic!("checklist commands must not store credentials")
    }

    fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("checklist commands must not delete credentials")
    }

    fn delete_if_matches(
        &self,
        _target: &CredentialTarget,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("checklist commands must not delete credentials")
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
        panic!("checklist commands must not store credentials through a mutation guard")
    }

    fn delete(&self) -> Result<bool, CredentialError> {
        panic!("checklist commands must not delete credentials through a mutation guard")
    }

    fn delete_if_matches(
        &self,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("checklist commands must not delete credentials through a mutation guard")
    }
}

fn factory(server: &MockServer) -> WekanClientFactory {
    WekanClientFactory::for_profile(format!("{}/", server.uri()), "default".to_owned(), false)
}

fn command(family: &str, arguments: &[&str]) -> RootCommand {
    let mut argv = vec!["wekan", "--server", "https://wekan.example", family];
    argv.extend_from_slice(arguments);
    Cli::try_parse_from(argv).unwrap().command
}
async fn dispatch(
    command: RootCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        RootCommand::Checklist(args) => {
            super::dispatch(args.command, factory, store, confirmation).await
        }
        RootCommand::ChecklistItem(args) => {
            crate::commands::checklist_items::dispatch(args.command, factory, store, confirmation)
                .await
        }
        _ => panic!("expected checklist command"),
    }
}

#[tokio::test]
async fn creation_preserves_text_and_trims_only_scope_identifiers() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/boards/board-1/cards/card-1/checklists"))
        .and(header("authorization", "Bearer checklist-token"))
        .and(body_json(
            json!({"title":"  Release  ","items":["  Verify  ","Ship"]}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"checklist-1"})))
        .expect(1)
        .mount(&server)
        .await;
    let result = dispatch(
        command(
            "checklist",
            &[
                "create",
                "--board",
                " board-1 ",
                "--card",
                " card-1 ",
                "--title",
                "  Release  ",
                "--item",
                "  Verify  ",
                "--item",
                "Ship",
            ],
        ),
        &factory(&server),
        &FakeCredentialStore::authenticated(&server),
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();
    let CommandSuccess::ChecklistCreated(result) = result else {
        panic!("expected creation");
    };
    assert_eq!(result.board_id, "board-1");
    assert_eq!(result.checklist_id, "checklist-1");
}

#[tokio::test]
async fn item_updates_preserve_wire_types_and_report_ordered_submitted_fields() {
    for (arguments, body, expected) in [
        (vec![], json!({}), vec![]),
        (
            vec!["--is-finished-json", r#""TRUE""#, "--title", "  Renamed  "],
            json!({"title":"  Renamed  ","isFinished":"TRUE"}),
            vec![
                ChecklistItemSubmittedField::Title,
                ChecklistItemSubmittedField::IsFinished,
            ],
        ),
        (
            vec!["--is-finished", "false"],
            json!({"isFinished":false}),
            vec![ChecklistItemSubmittedField::IsFinished],
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path(
                "/api/boards/board-1/cards/card-1/checklists/checklist-1/items/item-1",
            ))
            .and(body_json(body))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"item-1"})))
            .expect(1)
            .mount(&server)
            .await;
        let mut args = vec![
            "update",
            "item-1",
            "--board",
            "board-1",
            "--card",
            "card-1",
            "--checklist",
            "checklist-1",
        ];
        args.extend(arguments);
        let result = dispatch(
            command("checklist-item", &args),
            &factory(&server),
            &FakeCredentialStore::authenticated(&server),
            &FakeConfirmationProvider::declining(),
        )
        .await
        .unwrap();
        let CommandSuccess::ChecklistItemUpdated(result) = result else {
            panic!("expected update");
        };
        assert_eq!(result.submitted_fields, expected);
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

fn deletion(family: &str, yes: bool) -> RootCommand {
    let mut args = vec![
        "delete",
        "checklist-1",
        "--board",
        "board-1",
        "--card",
        "card-1",
    ];
    if family == "checklist-item" {
        args[1] = "item-1";
        args.extend(["--checklist", "checklist-1"]);
    }
    if yes {
        args.push("--yes");
    }
    command(family, &args)
}

#[tokio::test]
async fn item_creation_keeps_numeric_and_explicit_null_sort_distinct_from_omission() {
    for (flags, body) in [
        (vec![], json!({"title":"  Item  "})),
        (
            vec!["--sort", "-1.5"],
            json!({"title":"  Item  ","sort":-1.5}),
        ),
        (
            vec!["--sort-json", "null"],
            json!({"title":"  Item  ","sort":null}),
        ),
        (
            vec!["--sort-json", r#""""#],
            json!({"title":"  Item  ","sort":""}),
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/api/boards/board-1/cards/card-1/checklists/checklist-1/items",
            ))
            .and(body_json(body))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"item-1"})))
            .expect(1)
            .mount(&server)
            .await;
        let mut args = vec![
            "create",
            "--board",
            "board-1",
            "--card",
            "card-1",
            "--checklist",
            "checklist-1",
            "--title",
            "  Item  ",
        ];
        args.extend(flags);
        let CommandSuccess::ChecklistItemCreated(created) = dispatch(
            command("checklist-item", &args),
            &factory(&server),
            &FakeCredentialStore::authenticated(&server),
            &FakeConfirmationProvider::declining(),
        )
        .await
        .unwrap() else {
            panic!("expected item creation");
        };
        assert_eq!(created.item_id, "item-1");
    }
}

#[tokio::test]
async fn checklist_detail_output_maps_all_metadata_and_escapes_nested_terminal_controls() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "_id":"checklist-1","cardId":"card-1","boardId":"board-1","title":"Title\n\u{1b}",
        "sort":1.5,"createdAt":"2030-01-01T00:00:00Z","modifiedAt":"2030-01-01T00:00:00Z",
        "finishedAt":"+010000-01-01T00:00:00.000Z","hideCheckedChecklistItems":false,
        "hideAllChecklistItems":true,"showChecklistAtMinicard":false,
        "items":[{"_id":"second","title":"Nested\u{1b}","isFinished":true},{"_id":"first","title":"First","isFinished":false}]
    }))).mount(&server).await;
    let result = dispatch(
        command(
            "checklist",
            &[
                "get",
                "checklist-1",
                "--board",
                "board-1",
                "--card",
                "card-1",
            ],
        ),
        &factory(&server),
        &FakeCredentialStore::authenticated(&server),
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();
    let human = crate::output::render_success(crate::output::OutputFormat::Human, &result);
    assert!(!human.contains('\u{1b}'));
    assert!(human.contains(r"\u{1b}"));
    let json: serde_json::Value = serde_json::from_str(&crate::output::render_success(
        crate::output::OutputFormat::Json,
        &result,
    ))
    .unwrap();
    assert_eq!(json["data"]["items"][0]["item_id"], "second");
    assert_eq!(json["data"]["items"][1]["item_id"], "first");
    assert_eq!(json["data"]["hide_checked_checklist_items"], false);
    assert_eq!(json["data"]["hide_all_checklist_items"], true);
    assert_eq!(json["data"]["show_checklist_at_minicard"], false);
    assert_eq!(json["data"]["finished_at"], "+010000-01-01T00:00:00.000Z");
    assert_eq!(json["data"]["title"], "Title\n\u{1b}");
}

#[tokio::test]
async fn both_detail_commands_preserve_optional_creator_ids_in_human_and_json_output() {
    for family in ["checklist", "checklist-item"] {
        for creator in [None, Some("creator-2\u{1b}")] {
            let server = MockServer::start().await;
            let (id, url, mut response) = if family == "checklist" {
                (
                    "checklist-1",
                    "/api/boards/board-1/cards/card-1/checklists/checklist-1",
                    json!({"_id":"checklist-1","cardId":"card-1","boardId":"board-1",
                        "title":"Release","sort":0,"createdAt":"2030-01-01T00:00:00Z",
                        "modifiedAt":"2030-01-01T00:00:00Z","items":[]}),
                )
            } else {
                (
                    "item-1",
                    "/api/boards/board-1/cards/card-1/checklists/checklist-1/items/item-1",
                    json!({"_id":"item-1","cardId":"card-1","boardId":"board-1",
                        "checklistId":"checklist-1","title":"Verify","sort":0,
                        "isFinished":false,"modifiedAt":"2030-01-01T00:00:00Z"}),
                )
            };
            if let Some(creator) = creator {
                response["userId"] = json!(creator);
            }
            Mock::given(method("GET"))
                .and(path(url))
                .respond_with(ResponseTemplate::new(200).set_body_json(response))
                .expect(1)
                .mount(&server)
                .await;
            let mut args = vec!["get", id, "--board", "board-1", "--card", "card-1"];
            if family == "checklist-item" {
                args.extend(["--checklist", "checklist-1"]);
            }
            let result = dispatch(
                command(family, &args),
                &factory(&server),
                &FakeCredentialStore::authenticated(&server),
                &FakeConfirmationProvider::declining(),
            )
            .await
            .unwrap();
            let output: serde_json::Value = serde_json::from_str(&crate::output::render_success(
                crate::output::OutputFormat::Json,
                &result,
            ))
            .unwrap();
            assert_eq!(output["data"]["user_id"], json!(creator));
            assert!(output["data"].get("userId").is_none());
            let human = crate::output::render_success(crate::output::OutputFormat::Human, &result);
            assert!(!human.contains('\u{1b}'));
            if creator.is_some() {
                assert!(human.contains(r"user_id: creator-2\u{1b}"));
            } else {
                assert!(!human.contains("user_id:"));
            }
            server.verify().await;
        }
    }
}

#[tokio::test]
async fn both_deletions_bind_confirmation_to_credentials_and_cancel_without_http() {
    for (family, operation) in [
        ("checklist", DestructiveOperation::ChecklistDelete),
        ("checklist-item", DestructiveOperation::ChecklistItemDelete),
    ] {
        let server = MockServer::start().await;
        let store = FakeCredentialStore::authenticated(&server);
        let declining = FakeConfirmationProvider::declining();
        let result = dispatch(
            deletion(family, false),
            &factory(&server),
            &store,
            &declining,
        )
        .await
        .unwrap();
        let CommandSuccess::Cancelled(result) = result else {
            panic!("expected cancellation");
        };
        assert_eq!(result.operation, operation);
        assert_eq!(store.lock_count.load(Ordering::SeqCst), 1);
        assert!(server.received_requests().await.unwrap().is_empty());
        store.replace_on_next_lock(CredentialRecord::new(
            format!("{}/", server.uri()),
            "user-2".to_owned(),
            SecretString::from("changed-token".to_owned()),
            OffsetDateTime::now_utc() + Duration::hours(1),
        ));
        let error = dispatch(
            deletion(family, true),
            &factory(&server),
            &store,
            &FakeConfirmationProvider::accepting(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn both_deletions_require_confirmation_or_yes_and_verify_acknowledgements() {
    for (family, url, id) in [
        (
            "checklist",
            "/api/boards/board-1/cards/card-1/checklists/checklist-1",
            "checklist-1",
        ),
        (
            "checklist-item",
            "/api/boards/board-1/cards/card-1/checklists/checklist-1/items/item-1",
            "item-1",
        ),
    ] {
        for body in [
            json!({"_id":id}),
            json!({"_id":"wrong"}),
            json!({"_id":id,"unknown":true}),
        ] {
            let server = MockServer::start().await;
            let store = FakeCredentialStore::authenticated(&server);
            let error = dispatch(
                deletion(family, false),
                &factory(&server),
                &store,
                &crate::input::SystemConfirmationProvider::for_output(false),
            )
            .await
            .unwrap_err();
            assert_eq!(error.code(), ErrorCode::InvalidInput);
            assert!(server.received_requests().await.unwrap().is_empty());
            Mock::given(method("DELETE"))
                .and(path(url))
                .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
                .expect(1)
                .mount(&server)
                .await;
            let result = dispatch(
                deletion(family, true),
                &factory(&server),
                &store,
                &FakeConfirmationProvider::declining(),
            )
            .await;
            if body == json!({"_id":id}) {
                let result = result.unwrap();
                let rendered =
                    crate::output::render_success(crate::output::OutputFormat::Json, &result);
                let output: serde_json::Value = serde_json::from_str(&rendered).unwrap();
                assert_eq!(output["data"]["deleted"], true);
                assert_eq!(output["data"]["delete_mode"], "hard");
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.code(), ErrorCode::ProtocolError);
                assert_eq!(error.details().outcome_unknown, Some(true));
            }
        }
    }
}

#[tokio::test]
async fn mutations_keep_pinned_http_failures_and_uncertain_outcomes() {
    for (status, expected, unknown) in [
        (401, ErrorCode::AuthenticationRejected, None),
        (403, ErrorCode::PermissionDenied, None),
        (404, ErrorCode::NotFound, None),
        (500, ErrorCode::ServerError, Some(true)),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(status))
            .mount(&server)
            .await;
        let error = dispatch(
            command(
                "checklist",
                &["create", "--board", "board-1", "--card", "card-1"],
            ),
            &factory(&server),
            &FakeCredentialStore::authenticated(&server),
            &FakeConfirmationProvider::declining(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), expected);
        assert_eq!(error.details().outcome_unknown, unknown);
    }
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let error = dispatch(
        command(
            "checklist",
            &[
                "get",
                "checklist-1",
                "--board",
                "board-1",
                "--card",
                "card-1",
            ],
        ),
        &factory(&server),
        &FakeCredentialStore::authenticated(&server),
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ServerError);
    assert_eq!(error.details().http_status, Some(500));
    assert_eq!(error.details().outcome_unknown, None);
}
