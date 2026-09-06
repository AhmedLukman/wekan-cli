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

use super::ListCommand;
use crate::{
    cli::Cli,
    client::WekanClientFactory,
    command_result::{CommandSuccess, DestructiveOperation, ListDeleteMode, ListUpdatedField},
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
                SecretString::from("list-token".to_owned()),
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
        panic!("list commands must not store credentials")
    }

    fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("list commands must not delete credentials")
    }

    fn delete_if_matches(
        &self,
        _target: &CredentialTarget,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("list commands must not delete credentials")
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
        panic!("list commands must not store credentials through a mutation guard")
    }

    fn delete(&self) -> Result<bool, CredentialError> {
        panic!("list commands must not delete credentials through a mutation guard")
    }

    fn delete_if_matches(
        &self,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("list commands must not delete credentials through a mutation guard")
    }
}

fn factory(server: &MockServer) -> WekanClientFactory {
    WekanClientFactory::for_profile(format!("{}/", server.uri()), "default".to_owned(), false)
}

fn list_command(arguments: &[&str]) -> ListCommand {
    let mut all = vec!["wekan", "--server", "https://wekan.example", "list"];
    all.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(all).unwrap();
    let RootCommand::List(args) = cli.command else {
        panic!("expected list command")
    };
    args.command
}

async fn dispatch(
    command: ListCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    super::dispatch(command, client_factory, credential_store, confirmation).await
}

#[test]
fn parser_exposes_only_crud_and_validates_update_groups() {
    for arguments in [
        vec!["list", "board-1"],
        vec!["get", "board-1", "list-1"],
        vec!["create", "board-1", "--title", "Todo"],
        vec![
            "create",
            "board-1",
            "--title",
            "Todo",
            "--swimlane-id",
            "swimlane-1",
        ],
        vec!["update", "board-1", "list-1", "--title", "Doing"],
        vec![
            "update",
            "board-1",
            "list-1",
            "--color",
            "#12aBcF",
            "--starred",
            "false",
            "--wip-limit",
            "2.5",
            "--wip-enabled",
            "true",
            "--wip-soft",
            "false",
        ],
        vec!["delete", "board-1", "list-1", "--yes"],
    ] {
        list_command(&arguments);
    }

    for invalid in [
        vec!["wekan", "list", "update", "board-1", "list-1"],
        vec![
            "wekan",
            "list",
            "update",
            "board-1",
            "list-1",
            "--wip-limit",
            "2",
        ],
        vec![
            "wekan", "list", "update", "board-1", "list-1", "--color", "belize",
        ],
        vec![
            "wekan",
            "list",
            "update",
            "board-1",
            "list-1",
            "--wip-limit",
            "NaN",
            "--wip-enabled",
            "true",
            "--wip-soft",
            "false",
        ],
        vec!["wekan", "list", "get", "board-1", "list-1", "--yes"],
    ] {
        assert!(Cli::try_parse_from(invalid).is_err());
    }

    let long_title = "x".repeat(1001);
    list_command(&["update", "board-1", "list-1", "--title", &long_title]);

    let five_hundred_emojis = "😀".repeat(500);
    list_command(&[
        "update",
        "board-1",
        "list-1",
        "--title",
        &five_hundred_emojis,
    ]);

    let five_hundred_one_emojis = "😀".repeat(501);
    list_command(&[
        "update",
        "board-1",
        "list-1",
        "--title",
        &five_hundred_one_emojis,
    ]);

    let ListCommand::Create(trimmed) = list_command(&[
        "create",
        " board-1 ",
        "--title",
        " Todo ",
        "--swimlane-id",
        " swimlane-1 ",
    ]) else {
        panic!("expected create-list arguments")
    };
    assert_eq!(trimmed.board_id, "board-1");
    assert_eq!(trimmed.title, "Todo");
    assert_eq!(trimmed.swimlane_id.as_deref(), Some("swimlane-1"));

    for whitespace_only in [
        vec!["wekan", "list", "list", "   "],
        vec!["wekan", "list", "get", "board-1", "   "],
        vec!["wekan", "list", "create", "board-1", "--title", "   "],
        vec![
            "wekan",
            "list",
            "create",
            "board-1",
            "--title",
            "Todo",
            "--swimlane-id",
            "   ",
        ],
    ] {
        assert!(Cli::try_parse_from(whitespace_only).is_err());
    }
}

#[tokio::test]
async fn collection_get_create_and_update_return_stable_typed_results() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board%2F1/lists"))
        .and(header("authorization", "Bearer list-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "list-1",
            "title": "Todo",
            "modifiedAt": "2026-08-28T01:00:00Z",
            "cardsModifiedAt": null
        }])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board%2F1/lists/list%2F1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "list/1",
            "title": "Todo",
            "starred": false,
            "archived": false,
            "boardId": "board/1",
            "swimlaneId": "swimlane-1",
            "createdAt": "2026-08-28T00:00:00Z",
            "modifiedAt": "2026-08-28T01:00:00Z",
            "_updatedAt": "2026-08-28T00:30:00Z",
            "wipLimit": {"value": 2, "enabled": true, "soft": false},
            "color": "#12aBcF",
            "type": "list",
            "width": 220
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/boards/board%2F1/lists"))
        .and(body_json(
            json!({"title": "Todo", "swimlaneId": "swimlane-1"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "list-2"})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/boards/board%2F1/lists/list%2F1"))
        .and(body_json(json!({
            "title": "Doing",
            "color": "silver",
            "starred": false,
            "wipLimit": {"value": 3.0, "enabled": true, "soft": false}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "list/1"})))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);
    let confirmation = FakeConfirmationProvider::accepting();

    let CommandSuccess::ListCollection(collection) = dispatch(
        list_command(&["list", "board/1"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected list collection")
    };
    assert_eq!(collection.board_id, "board/1");
    assert_eq!(collection.lists[0].list_id, "list-1");

    let CommandSuccess::ListShown(list) = dispatch(
        list_command(&["get", "board/1", "list/1"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected list detail")
    };
    assert_eq!(
        list.position_updated_at.as_deref(),
        Some("2026-08-28T00:30:00Z")
    );
    assert_eq!(list.list_type, "list");

    let CommandSuccess::ListCreated(created) = dispatch(
        list_command(&[
            "create",
            "board/1",
            "--title",
            " Todo ",
            "--swimlane-id",
            "swimlane-1",
        ]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected list creation")
    };
    assert_eq!(created.list_id, "list-2");

    let CommandSuccess::ListUpdated(updated) = dispatch(
        list_command(&[
            "update",
            "board/1",
            "list/1",
            "--title",
            "Doing",
            "--color",
            "silver",
            "--starred",
            "false",
            "--wip-limit",
            "3",
            "--wip-enabled",
            "true",
            "--wip-soft",
            "false",
        ]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected list update")
    };
    assert_eq!(
        updated.updated_fields,
        [
            ListUpdatedField::Title,
            ListUpdatedField::Color,
            ListUpdatedField::Starred,
            ListUpdatedField::WipLimit,
        ]
    );
}

#[tokio::test]
async fn get_maps_empty_success_to_not_found_and_rejects_unknown_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/missing"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/future"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "future",
            "title": "Future",
            "archived": false,
            "boardId": "board-1",
            "createdAt": "2026-08-28T00:00:00Z",
            "modifiedAt": "2026-08-28T00:00:00Z",
            "type": "list",
            "futureField": true
        })))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);

    let missing = dispatch(
        list_command(&["get", "board-1", "missing"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::NotFound);
    assert_eq!(missing.details().http_status, Some(200));
    assert_eq!(missing.details().wekan_status_code, Some(404));

    let future = dispatch(
        list_command(&["get", "board-1", "future"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(future.code(), ErrorCode::ProtocolError);
    let json = crate::output::render_error(crate::output::OutputFormat::Json, &future);
    let envelope: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        envelope["error"]["details"]["response_path"],
        "/futureField"
    );
    assert_eq!(
        envelope["error"]["details"]["response_error"],
        "unknown_field"
    );
}

#[tokio::test]
async fn delete_confirms_guards_the_credential_and_verifies_the_id() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    let declining = FakeConfirmationProvider::declining();
    let cancelled = dispatch(
        list_command(&["delete", "board-1", "list\u{1b}]52;c;x\u{7}"]),
        &factory(&server),
        &store,
        &declining,
    )
    .await
    .unwrap();
    assert_eq!(
        cancelled,
        CommandSuccess::Cancelled(crate::command_result::CancellationSuccess::new(
            DestructiveOperation::ListDelete
        ))
    );
    assert!(!declining.requests()[0].prompt().contains('\u{1b}'));
    assert!(declining.requests()[0].prompt().contains("live cards"));

    Mock::given(method("DELETE"))
        .and(path("/api/boards/board-1/lists/list-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "list-1"})))
        .expect(1)
        .mount(&server)
        .await;
    let CommandSuccess::ListDeleted(deleted) = dispatch(
        list_command(&["delete", "board-1", "list-1", "--yes"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap() else {
        panic!("expected list deletion")
    };
    assert!(deleted.deleted);
    assert_eq!(deleted.delete_mode, ListDeleteMode::Soft);

    let replacement = CredentialRecord::new(
        format!("{}/", server.uri()),
        "user-2".to_owned(),
        SecretString::from("replacement-token".to_owned()),
        OffsetDateTime::now_utc() + Duration::hours(1),
    );
    store.replace_on_next_lock(replacement);
    let confirmation = FakeConfirmationProvider::accepting();
    let error = dispatch(
        list_command(&["delete", "board-1", "list-2"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
    assert!(confirmation.requests().is_empty());
    assert_eq!(store.lock_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn update_maps_404_and_mutation_protocol_failures_are_ambiguous() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/api/boards/board-1/lists/missing"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "error": "List not found"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/boards/board-1/lists"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);

    let not_found = dispatch(
        list_command(&["update", "board-1", "missing", "--title", "New"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(not_found.code(), ErrorCode::NotFound);
    assert_eq!(not_found.details().outcome_unknown, None);

    let malformed = dispatch(
        list_command(&["create", "board-1", "--title", "Todo"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(malformed.code(), ErrorCode::ProtocolError);
    assert_eq!(malformed.details().outcome_unknown, Some(true));
}
