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

use super::SwimlaneCommand;
use crate::{
    cli::Cli,
    client::WekanClientFactory,
    command_result::{
        CommandSuccess, DestructiveOperation, SwimlaneDeleteMode, SwimlaneUpdatedField,
    },
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
                SecretString::from("swimlane-token".to_owned()),
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
        panic!("swimlane commands must not store credentials")
    }

    fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("swimlane commands must not delete credentials")
    }

    fn delete_if_matches(
        &self,
        _target: &CredentialTarget,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("swimlane commands must not delete credentials")
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
        panic!("swimlane commands must not store credentials through a mutation guard")
    }

    fn delete(&self) -> Result<bool, CredentialError> {
        panic!("swimlane commands must not delete credentials through a mutation guard")
    }

    fn delete_if_matches(
        &self,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("swimlane commands must not delete credentials through a mutation guard")
    }
}

fn factory(server: &MockServer) -> WekanClientFactory {
    WekanClientFactory::for_profile(format!("{}/", server.uri()), "default".to_owned(), false)
}

fn swimlane_command(arguments: &[&str]) -> SwimlaneCommand {
    let mut all = vec!["wekan", "--server", "https://wekan.example", "swimlane"];
    all.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(all).unwrap();
    let RootCommand::Swimlane(args) = cli.command else {
        panic!("expected swimlane command")
    };
    args.command
}

async fn dispatch(
    command: SwimlaneCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    super::dispatch(command, client_factory, credential_store, confirmation).await
}

#[test]
fn parser_exposes_only_crud_and_validates_inputs() {
    for arguments in [
        vec!["list", "board-1"],
        vec!["get", "board-1", "swimlane-1"],
        vec!["create", "board-1", "--title", "Delivery"],
        vec!["create", "board-1", "--title", "Delivery", "--sort", "2.5"],
        vec!["update", "board-1", "swimlane-1", "--title", "Operations"],
        vec!["delete", "board-1", "swimlane-1", "--yes"],
    ] {
        swimlane_command(&arguments);
    }

    for invalid in [
        vec!["wekan", "swimlane", "update", "board-1", "swimlane-1"],
        vec![
            "wekan", "swimlane", "create", "board-1", "--title", "Delivery", "--sort", "NaN",
        ],
        vec![
            "wekan", "swimlane", "create", "board-1", "--title", "Delivery", "--sort", "inf",
        ],
        vec!["wekan", "swimlane", "get", "board-1", "swimlane-1", "--yes"],
        vec!["wekan", "swimlane", "copy", "board-1", "swimlane-1"],
        vec!["wekan", "swimlane", "move", "board-1", "swimlane-1"],
    ] {
        assert!(Cli::try_parse_from(invalid).is_err());
    }

    let SwimlaneCommand::Create(trimmed) = swimlane_command(&[
        "create",
        " board-1 ",
        "--title",
        " Delivery ",
        "--sort",
        "-3.25",
    ]) else {
        panic!("expected create-swimlane arguments")
    };
    assert_eq!(trimmed.board_id, "board-1");
    assert_eq!(trimmed.title, "Delivery");
    assert_eq!(trimmed.sort, Some(-3.25));

    for whitespace_only in [
        vec!["wekan", "swimlane", "list", "   "],
        vec!["wekan", "swimlane", "get", "board-1", "   "],
        vec!["wekan", "swimlane", "create", "board-1", "--title", "   "],
        vec![
            "wekan",
            "swimlane",
            "update",
            "board-1",
            "swimlane-1",
            "--title",
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
        .and(path("/api/boards/board%2F1/swimlanes"))
        .and(header("authorization", "Bearer swimlane-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "swimlane-1",
            "title": "Delivery"
        }])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board%2F1/swimlanes/swimlane%2F1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "swimlane/1",
            "title": "Delivery",
            "archived": false,
            "archivedAt": null,
            "boardId": "board/1",
            "createdAt": "2026-08-28T00:00:00Z",
            "sort": 2.5,
            "color": "#12aBcF",
            "updatedAt": "2026-08-28T00:30:00Z",
            "modifiedAt": "2026-08-28T01:00:00Z",
            "type": "swimlane",
            "height": 250
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/boards/board%2F1/swimlanes"))
        .and(body_json(json!({"title": "Delivery", "sort": 2.5})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "swimlane-2"})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/boards/board%2F1/swimlanes/swimlane%2F1"))
        .and(body_json(json!({"title": "Operations"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "swimlane/1"})))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);
    let confirmation = FakeConfirmationProvider::accepting();

    let CommandSuccess::SwimlaneCollection(collection) = dispatch(
        swimlane_command(&["list", "board/1"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected swimlane collection")
    };
    assert_eq!(collection.board_id, "board/1");
    assert_eq!(collection.swimlanes[0].swimlane_id, "swimlane-1");

    let CommandSuccess::SwimlaneShown(swimlane) = dispatch(
        swimlane_command(&["get", "board/1", "swimlane/1"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected swimlane detail")
    };
    assert_eq!(swimlane.swimlane_type, "swimlane");
    assert_eq!(swimlane.height, Some(250.into()));

    let CommandSuccess::SwimlaneCreated(created) = dispatch(
        swimlane_command(&[
            "create",
            "board/1",
            "--title",
            " Delivery ",
            "--sort",
            "2.5",
        ]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected swimlane creation")
    };
    assert_eq!(created.swimlane_id, "swimlane-2");

    let CommandSuccess::SwimlaneUpdated(updated) = dispatch(
        swimlane_command(&["update", "board/1", "swimlane/1", "--title", "Operations"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected swimlane update")
    };
    assert_eq!(updated.updated_fields, [SwimlaneUpdatedField::Title]);
}

#[tokio::test]
async fn get_maps_empty_success_to_not_found_and_rejects_unknown_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/swimlanes/missing"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/swimlanes/future"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "future",
            "title": "Future",
            "archived": false,
            "boardId": "board-1",
            "createdAt": "2026-08-28T00:00:00Z",
            "modifiedAt": "2026-08-28T00:00:00Z",
            "type": "swimlane",
            "futureField": true
        })))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);

    let missing = dispatch(
        swimlane_command(&["get", "board-1", "missing"]),
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
        swimlane_command(&["get", "board-1", "future"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(future.code(), ErrorCode::ProtocolError);
}

#[tokio::test]
async fn delete_confirms_guards_the_credential_and_verifies_the_id() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    let declining = FakeConfirmationProvider::declining();
    let cancelled = dispatch(
        swimlane_command(&["delete", "board-1", "swimlane\u{1b}]52;c;x\u{7}"]),
        &factory(&server),
        &store,
        &declining,
    )
    .await
    .unwrap();
    assert_eq!(
        cancelled,
        CommandSuccess::Cancelled(crate::command_result::CancellationSuccess::new(
            DestructiveOperation::SwimlaneDelete
        ))
    );
    assert!(!declining.requests()[0].prompt().contains('\u{1b}'));
    assert!(declining.requests()[0].prompt().contains("lists or cards"));

    Mock::given(method("DELETE"))
        .and(path("/api/boards/board-1/swimlanes/swimlane-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "swimlane-1"})))
        .expect(1)
        .mount(&server)
        .await;
    let CommandSuccess::SwimlaneDeleted(deleted) = dispatch(
        swimlane_command(&["delete", "board-1", "swimlane-1", "--yes"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap() else {
        panic!("expected swimlane deletion")
    };
    assert!(deleted.deleted);
    assert_eq!(deleted.delete_mode, SwimlaneDeleteMode::Hard);

    Mock::given(method("DELETE"))
        .and(path("/api/boards/board-1/swimlanes/swimlane-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "other"})))
        .expect(1)
        .mount(&server)
        .await;
    let mismatch = dispatch(
        swimlane_command(&["delete", "board-1", "swimlane-2", "--yes"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap_err();
    assert_eq!(mismatch.code(), ErrorCode::ProtocolError);
    assert_eq!(mismatch.details().outcome_unknown, Some(true));

    let requests_before_replacement = server.received_requests().await.unwrap().len();
    let replacement = CredentialRecord::new(
        format!("{}/", server.uri()),
        "user-2".to_owned(),
        SecretString::from("replacement-token".to_owned()),
        OffsetDateTime::now_utc() + Duration::hours(1),
    );
    Mock::given(method("DELETE"))
        .and(path("/api/boards/board-1/swimlanes/swimlane-3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "swimlane-3"})))
        .expect(0)
        .mount(&server)
        .await;
    store.replace_on_next_lock(replacement.clone());
    let confirmation = FakeConfirmationProvider::accepting();
    let error = dispatch(
        swimlane_command(&["delete", "board-1", "swimlane-3"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
    assert_eq!(error.details().credential_stored, Some(true));
    assert!(confirmation.requests().is_empty());
    assert_eq!(store.lock_count.load(Ordering::SeqCst), 4);
    assert!(
        store
            .record
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|record| record.matches(&replacement))
    );
    server.verify().await;
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        requests_before_replacement,
        "a replaced credential must abort before any additional HTTP request"
    );
}

#[tokio::test]
async fn update_maps_not_found_and_mutation_protocol_failures_are_ambiguous() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/api/boards/board-1/swimlanes/missing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "not-found",
            "reason": "Swimlane not found"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/boards/board-1/swimlanes"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/boards/board-1/swimlanes/swimlane-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "other"})))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);

    let not_found = dispatch(
        swimlane_command(&["update", "board-1", "missing", "--title", "New"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(not_found.code(), ErrorCode::NotFound);
    assert_eq!(not_found.details().outcome_unknown, None);

    let malformed = dispatch(
        swimlane_command(&["create", "board-1", "--title", "Delivery"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(malformed.code(), ErrorCode::ProtocolError);
    assert_eq!(malformed.details().outcome_unknown, Some(true));

    let mismatch = dispatch(
        swimlane_command(&["update", "board-1", "swimlane-1", "--title", "Operations"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(mismatch.code(), ErrorCode::ProtocolError);
    assert_eq!(mismatch.details().outcome_unknown, Some(true));
}
