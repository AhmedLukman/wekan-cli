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

use super::BoardCommand;
use crate::{
    cli::Cli,
    client::WekanClientFactory,
    command_result::{BoardListScope, CommandSuccess, DestructiveOperation},
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
                SecretString::from("board-token".to_owned()),
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
        panic!("board commands must not store credentials")
    }

    fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("board commands must not delete credentials")
    }

    fn delete_if_matches(
        &self,
        _target: &CredentialTarget,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("board commands must not delete credentials")
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
        panic!("board commands must not store credentials through a mutation guard")
    }

    fn delete(&self) -> Result<bool, CredentialError> {
        panic!("board commands must not delete credentials through a mutation guard")
    }

    fn delete_if_matches(
        &self,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("board commands must not delete credentials through a mutation guard")
    }
}

fn factory(server: &MockServer) -> WekanClientFactory {
    WekanClientFactory::for_profile(format!("{}/", server.uri()), "default".to_owned(), false)
}

fn board_command(arguments: &[&str]) -> BoardCommand {
    let mut all = vec!["wekan", "--server", "https://wekan.example", "board"];
    all.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(all).unwrap();
    let RootCommand::Board(args) = cli.command else {
        panic!("expected board command")
    };
    args.command
}

#[test]
fn rename_rejects_a_whitespace_only_title() {
    let error = Cli::try_parse_from([
        "wekan",
        "--server",
        "https://wekan.example",
        "board",
        "rename",
        "board-1",
        "--title",
        "   ",
    ])
    .unwrap_err();

    assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
}

async fn dispatch(
    command: BoardCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    super::dispatch(command, client_factory, credential_store, confirmation).await
}

#[test]
fn parser_exposes_the_board_lifecycle_and_validates_inputs() {
    for arguments in [
        vec!["list"],
        vec!["list", "--public"],
        vec!["count"],
        vec!["get", "board-1"],
        vec!["create", "--title", "Board"],
        vec![
            "create",
            "--title",
            "Board",
            "--owner",
            "user-2",
            "--permission",
            "public",
            "--color",
            "cleanlight",
            "--no-comments",
            "--comment-only",
            "--worker",
        ],
        vec!["rename", "board-1", "--title", "Renamed"],
        vec!["delete", "board-1", "--yes"],
    ] {
        board_command(&arguments);
    }

    assert!(Cli::try_parse_from(["wekan", "board", "get", ""]).is_err());
    assert!(Cli::try_parse_from(["wekan", "board", "create", "--title", ""]).is_err());
    assert!(Cli::try_parse_from(["wekan", "board", "create", "--title", "   "]).is_err());
    assert!(
        Cli::try_parse_from([
            "wekan", "board", "create", "--title", "Board", "--owner", "   ",
        ])
        .is_err()
    );
    assert!(
        Cli::try_parse_from([
            "wekan",
            "board",
            "create",
            "--title",
            "Board",
            "--color",
            "not-a-wekan-color",
        ])
        .is_err()
    );
    assert!(Cli::try_parse_from(["wekan", "board", "get", "board-1", "--yes"]).is_err());

    let BoardCommand::Create(defaults) = board_command(&["create", "--title", "Board"]) else {
        panic!("expected create command")
    };
    assert_eq!(defaults.owner, None);
    assert_eq!(
        defaults.permission,
        super::create::BoardPermissionArg::Private
    );
    assert_eq!(defaults.color, super::create::BoardColorArg::Belize);
    assert!(!defaults.no_comments);
    assert!(!defaults.comment_only);
    assert!(!defaults.worker);
}

#[tokio::test]
async fn list_selects_active_or_public_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/users/user-1/boards"))
        .and(header("authorization", "Bearer board-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"_id": "active-1", "title": "Active"}
        ])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards"))
        .and(header("authorization", "Bearer board-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"_id": "public-1", "title": "Public"}
        ])))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);
    let confirmation = FakeConfirmationProvider::accepting();

    let CommandSuccess::BoardList(active) = dispatch(
        board_command(&["list"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected active board list")
    };
    assert_eq!(active.scope, BoardListScope::Active);
    assert_eq!(active.boards[0].board_id, "active-1");

    let CommandSuccess::BoardList(public) = dispatch(
        board_command(&["list", "--public"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected public board list")
    };
    assert_eq!(public.scope, BoardListScope::Public);
    assert_eq!(public.boards[0].board_id, "public-1");
}

#[tokio::test]
async fn create_sends_every_meaningful_field_and_returns_ids() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/boards"))
        .and(header("authorization", "Bearer board-token"))
        .and(body_json(json!({
            "title": "Board",
            "owner": "user-2",
            "permission": "public",
            "color": "cleanlight",
            "isNoComments": true,
            "isCommentOnly": true,
            "isWorker": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "board-1",
            "defaultSwimlaneId": "swimlane-1"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);

    let CommandSuccess::BoardCreated(created) = dispatch(
        board_command(&[
            "create",
            "--title",
            "Board",
            "--owner",
            "user-2",
            "--permission",
            "public",
            "--color",
            "cleanlight",
            "--no-comments",
            "--comment-only",
            "--worker",
        ]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap() else {
        panic!("expected board creation")
    };
    assert_eq!(created.board_id, "board-1");
    assert_eq!(created.default_swimlane_id, "swimlane-1");
}

#[tokio::test]
async fn count_get_and_rename_return_typed_results() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards_count"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "private": 4,
            "public": 2
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board%2F1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "board/1",
            "title": "Board",
            "slug": "board",
            "createdAt": "2026-08-28T00:00:00.000Z",
            "members": [{
                "userId": "user-1",
                "isAdmin": true,
                "isActive": true
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/boards/board%2F1/title"))
        .and(body_json(json!({"title": "Renamed"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "board/1",
            "title": "Renamed"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);
    let confirmation = FakeConfirmationProvider::accepting();

    let CommandSuccess::BoardCount(counts) = dispatch(
        board_command(&["count"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected board counts")
    };
    assert_eq!((counts.private, counts.public), (4, 2));

    let CommandSuccess::BoardShown(board) = dispatch(
        board_command(&["get", "board/1"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected board")
    };
    assert_eq!(
        board.created_at.as_deref(),
        Some("2026-08-28T00:00:00.000Z")
    );

    let CommandSuccess::BoardRenamed(renamed) = dispatch(
        board_command(&["rename", "board/1", "--title", "  Renamed  "]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected board rename")
    };
    assert_eq!(renamed.board_id, "board/1");
    assert_eq!(renamed.title, "Renamed");
}

#[tokio::test]
async fn get_rejects_unmapped_response_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "board-1",
            "title": "Board",
            "members": [{
                "userId": "user-1",
                "isAdmin": true,
                "isActive": true,
                "futureRole": "observer"
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let error = dispatch(
        board_command(&["get", "board-1"]),
        &factory(&server),
        &FakeCredentialStore::authenticated(&server),
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ProtocolError);
}

#[tokio::test]
async fn get_maps_wekan_v11_06_embedded_404_to_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/missing-board"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "board-not-found",
            "reason": "Board not found",
            "statusCode": 404
        })))
        .expect(1)
        .mount(&server)
        .await;

    let error = dispatch(
        board_command(&["get", "missing-board"]),
        &factory(&server),
        &FakeCredentialStore::authenticated(&server),
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::NotFound);
    assert_eq!(error.message(), "the requested Wekan board was not found");
    assert_eq!(error.details().http_status, Some(200));
    assert_eq!(error.details().wekan_status_code, Some(404));
    assert_eq!(
        error.details().server_error.as_deref(),
        Some("board-not-found")
    );
    assert_eq!(
        error.details().server_reason.as_deref(),
        Some("Board not found")
    );
    assert_eq!(error.details().outcome_unknown, None);
}

#[tokio::test]
async fn delete_confirms_before_http_and_verifies_the_returned_id() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    let declining = FakeConfirmationProvider::declining();
    let unsafe_board_id = "board\u{1b}]52;c;clipboard\u{7}";
    let cancelled = dispatch(
        board_command(&["delete", unsafe_board_id]),
        &factory(&server),
        &store,
        &declining,
    )
    .await
    .unwrap();
    assert_eq!(
        cancelled,
        CommandSuccess::Cancelled(crate::command_result::CancellationSuccess::new(
            DestructiveOperation::BoardDelete
        ))
    );
    let requests = declining.requests();
    assert_eq!(requests[0].operation(), DestructiveOperation::BoardDelete);
    assert!(requests[0].prompt().contains("all of its contents"));
    assert!(!requests[0].prompt().contains('\u{1b}'));
    assert!(!requests[0].prompt().contains('\u{7}'));
    assert!(requests[0].prompt().contains(r"\u{1b}"));

    Mock::given(method("DELETE"))
        .and(path("/api/boards/board-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "other"})))
        .expect(1)
        .mount(&server)
        .await;
    let error = dispatch(
        board_command(&["delete", "board-1", "--yes"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ProtocolError);
    assert_eq!(error.details().outcome_unknown, Some(true));
}

#[tokio::test]
async fn delete_rejects_a_replaced_credential_before_confirmation_or_http() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/api/boards/board-1"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);
    let replacement = CredentialRecord::new(
        format!("{}/", server.uri()),
        "user-2".to_owned(),
        SecretString::from("replacement-token".to_owned()),
        OffsetDateTime::now_utc() + Duration::hours(1),
    );
    store.replace_on_next_lock(replacement.clone());
    let confirmation = FakeConfirmationProvider::accepting();

    let error = dispatch(
        board_command(&["delete", "board-1"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
    assert_eq!(error.details().credential_stored, Some(true));
    assert!(confirmation.requests().is_empty());
    assert_eq!(store.lock_count.load(Ordering::SeqCst), 1);
    assert!(
        store
            .record
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|record| record.matches(&replacement))
    );
}

#[tokio::test]
async fn ambiguous_mutation_failures_report_unknown_outcomes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/boards"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);
    let error = dispatch(
        board_command(&["create", "--title", "Board"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ProtocolError);
    assert_eq!(error.details().outcome_unknown, Some(true));

    Mock::given(method("PUT"))
        .and(path("/api/boards/board-1/title"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "other-board",
            "title": "Renamed"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let error = dispatch(
        board_command(&["rename", "board-1", "--title", "Renamed"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ProtocolError);
    assert_eq!(error.details().outcome_unknown, Some(true));
}
