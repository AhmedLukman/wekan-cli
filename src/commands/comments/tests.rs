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

use super::CommentCommand;
use crate::{
    cli::Cli,
    client::WekanClientFactory,
    command_result::{CommandSuccess, CommentDeleteMode, DestructiveOperation},
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
                SecretString::from("comment-token".to_owned()),
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
        panic!("comment commands must not store credentials")
    }

    fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("comment commands must not delete credentials")
    }

    fn delete_if_matches(
        &self,
        _target: &CredentialTarget,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("comment commands must not delete credentials")
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
        panic!("comment commands must not store credentials through a mutation guard")
    }

    fn delete(&self) -> Result<bool, CredentialError> {
        panic!("comment commands must not delete credentials through a mutation guard")
    }

    fn delete_if_matches(
        &self,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("comment commands must not delete credentials through a mutation guard")
    }
}

fn factory(server: &MockServer) -> WekanClientFactory {
    WekanClientFactory::for_profile(format!("{}/", server.uri()), "default".to_owned(), false)
}

fn comment_command(arguments: &[&str]) -> CommentCommand {
    let mut all = vec!["wekan", "--server", "https://wekan.example", "comment"];
    all.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(all).unwrap();
    let RootCommand::Comment(args) = cli.command else {
        panic!("expected comment command")
    };
    args.command
}

async fn dispatch(
    command: CommentCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    super::dispatch(command, client_factory, credential_store, confirmation).await
}

#[test]
fn parser_exposes_the_supported_lifecycle_and_validates_inputs() {
    for arguments in [
        vec!["list", "board-1", "card-1"],
        vec!["get", "board-1", "card-1", "comment-1"],
        vec!["create", "board-1", "card-1", "--text", "Hello"],
        vec!["delete", "board-1", "card-1", "comment-1"],
        vec!["delete", "board-1", "card-1", "comment-1", "--yes"],
    ] {
        comment_command(&arguments);
    }

    for invalid in [
        vec![
            "wekan",
            "comment",
            "update",
            "board-1",
            "card-1",
            "comment-1",
        ],
        vec!["wekan", "comment", "create", "board-1", "card-1"],
        vec![
            "wekan", "comment", "create", "board-1", "card-1", "--text", "   ",
        ],
        vec!["wekan", "comment", "list", "   ", "card-1"],
        vec!["wekan", "comment", "get", "board-1", "card-1", "   "],
        vec![
            "wekan",
            "comment",
            "get",
            "board-1",
            "card-1",
            "comment-1",
            "--yes",
        ],
    ] {
        assert!(Cli::try_parse_from(invalid).is_err());
    }

    let CommentCommand::Create(args) =
        comment_command(&["create", " board-1 ", " card-1 ", "--text", " Hello world "])
    else {
        panic!("expected comment-create arguments")
    };
    assert_eq!(args.board_id, "board-1");
    assert_eq!(args.card_id, "card-1");
    assert_eq!(args.text, "Hello world");
}

#[tokio::test]
async fn list_get_and_create_return_stable_typed_results() {
    let server = MockServer::start().await;
    let collection_path = "/api/boards/board%2F1/cards/card%2F1/comments";
    let comment_path = "/api/boards/board%2F1/cards/card%2F1/comments/comment%2F1";
    Mock::given(method("GET"))
        .and(path(collection_path))
        .and(header("authorization", "Bearer comment-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "comment/1",
            "comment": "Hello",
            "authorId": "user-1"
        }])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(comment_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "comment/1",
            "boardId": "board/1",
            "cardId": "card/1",
            "text": "Hello",
            "parentId": "",
            "createdAt": "2030-01-02T03:04:05Z",
            "modifiedAt": "2030-01-02T03:05:05Z",
            "userId": "user-1"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(collection_path))
        .and(body_json(json!({"comment": "Hello"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "comment-2"})))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);
    let confirmation = FakeConfirmationProvider::accepting();

    let CommandSuccess::CommentCollection(collection) = dispatch(
        comment_command(&["list", "board/1", "card/1"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected comment collection")
    };
    assert_eq!(collection.board_id, "board/1");
    assert_eq!(collection.card_id, "card/1");
    assert_eq!(collection.comments[0].comment_id, "comment/1");
    assert_eq!(collection.comments[0].author_id, "user-1");

    let CommandSuccess::CommentShown(comment) = dispatch(
        comment_command(&["get", "board/1", "card/1", "comment/1"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected comment detail")
    };
    assert_eq!(comment.text, "Hello");
    assert_eq!(comment.parent_id, None);

    let CommandSuccess::CommentCreated(created) = dispatch(
        comment_command(&["create", "board/1", "card/1", "--text", " Hello "]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap() else {
        panic!("expected comment creation")
    };
    assert_eq!(created.comment_id, "comment-2");
}

#[tokio::test]
async fn get_maps_empty_success_to_not_found_and_rejects_protocol_drift() {
    let server = MockServer::start().await;
    let base = "/api/boards/board-1/cards/card-1/comments";
    Mock::given(method("GET"))
        .and(path(format!("{base}/missing")))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{base}/future")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "future",
            "boardId": "board-1",
            "cardId": "card-1",
            "text": "Future",
            "createdAt": "2030-01-02T03:04:05Z",
            "modifiedAt": "2030-01-02T03:04:05Z",
            "userId": "user-1",
            "futureField": true
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{base}/wrong-scope")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "other",
            "boardId": "board-1",
            "cardId": "card-1",
            "text": "Other",
            "createdAt": "2030-01-02T03:04:05Z",
            "modifiedAt": "2030-01-02T03:04:05Z",
            "userId": "user-1"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);
    let confirmation = FakeConfirmationProvider::accepting();

    let missing = dispatch(
        comment_command(&["get", "board-1", "card-1", "missing"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::NotFound);
    assert_eq!(missing.details().http_status, Some(200));
    assert_eq!(missing.details().wekan_status_code, Some(404));

    for comment_id in ["future", "wrong-scope"] {
        let error = dispatch(
            comment_command(&["get", "board-1", "card-1", comment_id]),
            &factory(&server),
            &store,
            &confirmation,
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::ProtocolError);
    }
}

#[tokio::test]
async fn delete_confirms_guards_credentials_and_verifies_the_card_id() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    let declining = FakeConfirmationProvider::declining();
    let cancelled = dispatch(
        comment_command(&["delete", "board-1", "card-1", "comment\u{1b}]52;c;x\u{7}"]),
        &factory(&server),
        &store,
        &declining,
    )
    .await
    .unwrap();
    assert_eq!(
        cancelled,
        CommandSuccess::Cancelled(crate::command_result::CancellationSuccess::new(
            DestructiveOperation::CommentDelete
        ))
    );
    assert!(!declining.requests()[0].prompt().contains('\u{1b}'));

    Mock::given(method("DELETE"))
        .and(path("/api/boards/board-1/cards/card-1/comments/comment-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "card-1"})))
        .expect(1)
        .mount(&server)
        .await;
    let CommandSuccess::CommentDeleted(deleted) = dispatch(
        comment_command(&["delete", "board-1", "card-1", "comment-1", "--yes"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap() else {
        panic!("expected comment deletion")
    };
    assert!(deleted.deleted);
    assert_eq!(deleted.comment_id, "comment-1");
    assert_eq!(deleted.delete_mode, CommentDeleteMode::Hard);

    let replacement = CredentialRecord::new(
        format!("{}/", server.uri()),
        "user-2".to_owned(),
        SecretString::from("replacement-token".to_owned()),
        OffsetDateTime::now_utc() + Duration::hours(1),
    );
    let requests_before_replacement = server.received_requests().await.unwrap().len();
    store.replace_on_next_lock(replacement);
    let confirmation = FakeConfirmationProvider::accepting();
    let error = dispatch(
        comment_command(&["delete", "board-1", "card-1", "comment-2"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
    assert!(confirmation.requests().is_empty());
    assert_eq!(store.lock_count.load(Ordering::SeqCst), 3);
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        requests_before_replacement,
        "a replaced credential must abort before any additional HTTP request"
    );
}

#[tokio::test]
async fn delete_maps_a_missing_board_or_comment_to_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path(
            "/api/boards/board-1/cards/card-1/comments/missing-comment",
        ))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(json!({"error": "Comment not found"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);

    let error = dispatch(
        comment_command(&["delete", "board-1", "card-1", "missing-comment", "--yes"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::NotFound);
    assert_eq!(error.details().http_status, Some(404));
    assert_eq!(error.details().outcome_unknown, None);
}

#[tokio::test]
async fn mutation_protocol_failures_report_unknown_outcomes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/boards/board-1/cards/card-1/comments"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api/boards/board-1/cards/card-1/comments/comment-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": "other-card"})))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);

    let malformed = dispatch(
        comment_command(&["create", "board-1", "card-1", "--text", "Hello"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(malformed.code(), ErrorCode::ProtocolError);
    assert_eq!(malformed.details().outcome_unknown, Some(true));

    let mismatch = dispatch(
        comment_command(&["delete", "board-1", "card-1", "comment-1", "--yes"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(mismatch.code(), ErrorCode::ProtocolError);
    assert_eq!(mismatch.details().outcome_unknown, Some(true));
}
