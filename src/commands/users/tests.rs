use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use clap::Parser;
use secrecy::{ExposeSecret, SecretString};
use serde_json::json;
use time::{Duration, OffsetDateTime};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

use super::{UserCommand, cards::validate_range};
use crate::{
    cli::Cli,
    client::{ClientError, WekanClientFactory},
    command_result::{CommandSuccess, DestructiveOperation, UserCreateWarning},
    commands::RootCommand,
    credentials::{
        CredentialCreateOutcome, CredentialDeleteOutcome, CredentialError, CredentialMutation,
        CredentialRecord, CredentialStore, CredentialTarget, LoginSecretMode, LoginSecrets,
        SecretInputProvider,
    },
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    input::{ConfirmationArgs, ConfirmationProvider, FakeConfirmationProvider},
    redaction::Redactor,
};

struct FakeCredentialStore {
    record: Mutex<Option<CredentialRecord>>,
    replacement_on_lock: Mutex<Option<CredentialRecord>>,
    lock_count: AtomicUsize,
    fail_delete: AtomicBool,
}

impl FakeCredentialStore {
    fn authenticated(server: &MockServer, user_id: &str) -> Self {
        Self {
            record: Mutex::new(Some(CredentialRecord::new(
                format!("{}/", server.uri()),
                user_id.to_owned(),
                SecretString::from("user-token".to_owned()),
                OffsetDateTime::now_utc() + Duration::hours(1),
            ))),
            replacement_on_lock: Mutex::new(None),
            lock_count: AtomicUsize::new(0),
            fail_delete: AtomicBool::new(false),
        }
    }

    fn replace_on_next_lock(&self, replacement: CredentialRecord) {
        *self.replacement_on_lock.lock().unwrap() = Some(replacement);
    }

    fn fail_next_delete(&self) {
        self.fail_delete.store(true, Ordering::SeqCst);
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
        panic!("user commands must not store a credential")
    }

    fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("user commands use conditional credential deletion")
    }

    fn delete_if_matches(
        &self,
        _target: &CredentialTarget,
        expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        if self.fail_delete.swap(false, Ordering::SeqCst) {
            return Err(CredentialError::Delete("injected failure".to_owned()));
        }
        let mut record = self.record.lock().unwrap();
        match record.as_ref() {
            None => Ok(CredentialDeleteOutcome::Absent),
            Some(stored) if !stored.matches(expected) => Ok(CredentialDeleteOutcome::Mismatch),
            Some(_) => {
                *record = None;
                Ok(CredentialDeleteOutcome::Removed)
            }
        }
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
        panic!("user commands must not create credentials through a mutation guard")
    }

    fn delete(&self) -> Result<bool, CredentialError> {
        panic!("user commands must conditionally delete credentials")
    }

    fn delete_if_matches(
        &self,
        expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        self.store.delete_if_matches(
            &CredentialTarget::profile_in_store("unused", "unused", "https://unused/".to_owned()),
            expected,
        )
    }
}

struct FakeSecrets {
    password: SecretString,
    modes: Mutex<Vec<bool>>,
}

impl FakeSecrets {
    fn new(password: &str) -> Self {
        Self {
            password: SecretString::from(password.to_owned()),
            modes: Mutex::default(),
        }
    }
}

impl SecretInputProvider for FakeSecrets {
    fn read_new_account_password(&self, from_stdin: bool) -> Result<SecretString, AppError> {
        self.modes.lock().unwrap().push(from_stdin);
        Ok(self.password.clone())
    }

    fn read_login_secrets(&self, _mode: LoginSecretMode) -> Result<LoginSecrets, AppError> {
        panic!("user commands must not request login secrets")
    }
}

fn factory(server: &MockServer) -> WekanClientFactory {
    WekanClientFactory::for_profile(format!("{}/", server.uri()), "default".to_owned(), false)
}

fn user_command(arguments: &[&str]) -> UserCommand {
    let mut all = vec!["wekan", "--server", "https://wekan.example", "user"];
    all.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(all).unwrap();
    let RootCommand::User(args) = cli.command else {
        panic!("expected user command")
    };
    args.command
}

async fn dispatch(
    command: UserCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    secret_input: &dyn SecretInputProvider,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    command.validate()?;
    super::dispatch(
        command,
        client_factory,
        credential_store,
        secret_input,
        confirmation,
    )
    .await
}

#[test]
fn parser_exposes_every_user_operation_and_validates_inputs() {
    for arguments in [
        vec!["current"],
        vec!["cards", "--due"],
        vec!["list"],
        vec!["get", "alice"],
        vec!["create", "--username", "alice", "--email", "a@example.com"],
        vec!["boards", "user-1"],
        vec!["take-ownership", "user-1", "--yes"],
        vec!["disable-login", "user-1", "--yes"],
        vec!["enable-login", "user-1"],
        vec!["delete", "user-1", "--yes"],
    ] {
        user_command(&arguments);
    }

    assert!(Cli::try_parse_from(["wekan", "user", "get", ""]).is_err());
    assert!(Cli::try_parse_from(["wekan", "user", "cards", "--from", "not-a-date"]).is_err());
    assert!(Cli::try_parse_from(["wekan", "user", "create", "--username", "alice",]).is_err());
    assert!(Cli::try_parse_from(["wekan", "user", "enable-login", "user-1", "--yes"]).is_err());
}

#[test]
fn card_date_range_must_be_ordered() {
    let error =
        validate_range(Some("2026-08-28T00:00:00Z"), Some("2026-08-27T00:00:00Z")).unwrap_err();
    assert_eq!(error.code(), ErrorCode::InvalidInput);
    validate_range(Some("2026-08-27T00:00:00Z"), None).unwrap();
}

#[tokio::test]
async fn current_projects_the_shared_typed_user_record() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/user"))
        .and(header("authorization", "Bearer user-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "user-1",
            "username": "alice",
            "profile": { "fullname": "Alice", "language": "en" },
            "emails": [{ "address": "alice@example.com", "verified": true }],
            "isAdmin": true
        })))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server, "user-1");

    let success = dispatch(
        user_command(&["current"]),
        &factory(&server),
        &store,
        &FakeSecrets::new("unused"),
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap();

    let CommandSuccess::UserCurrent(user) = success else {
        panic!("expected current user")
    };
    assert_eq!(user.user_id, "user-1");
    assert_eq!(user.full_name.as_deref(), Some("Alice"));
    assert_eq!(user.emails.len(), 1);
    assert_eq!(user.login_disabled, None);
}

#[tokio::test]
async fn current_maps_a_remote_user_id_mismatch_to_a_credential_failure() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/user"))
        .and(header("authorization", "Bearer user-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "_id": "other-user" })))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server, "user-1");

    let error = dispatch(
        user_command(&["current"]),
        &factory(&server),
        &store,
        &FakeSecrets::new("unused"),
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
    assert_eq!(error.exit_code(), StableExitCode::Credential);
}

#[tokio::test]
async fn create_reads_the_shared_account_password_and_reports_the_v11_defect() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/users"))
        .and(header("authorization", "Bearer user-token"))
        .and(body_json(json!({
            "username": "bob",
            "email": "bob@example.com",
            "password": "new-user-password"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "_id": {} })))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server, "admin-1");
    let secrets = FakeSecrets::new("new-user-password");

    let success = dispatch(
        user_command(&[
            "create",
            "--username",
            "bob",
            "--email",
            "bob@example.com",
            "--password-stdin",
        ]),
        &factory(&server),
        &store,
        &secrets,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap();

    let CommandSuccess::UserCreated(created) = success else {
        panic!("expected user creation")
    };
    assert!(created.created);
    assert_eq!(created.user_id, None);
    assert_eq!(
        created.warning,
        Some(UserCreateWarning::UserIdUnavailableInWekanV1106)
    );
    assert_eq!(*secrets.modes.lock().unwrap(), [true]);
}

#[tokio::test]
async fn destructive_decline_sends_no_http_request() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server, "admin-1");
    let confirmation = FakeConfirmationProvider::declining();

    let success = dispatch(
        user_command(&["delete", "user-2"]),
        &factory(&server),
        &store,
        &FakeSecrets::new("unused"),
        &confirmation,
    )
    .await
    .unwrap();

    assert_eq!(
        success,
        CommandSuccess::Cancelled(crate::command_result::CancellationSuccess::new(
            DestructiveOperation::UserDelete
        ))
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn self_guards_run_before_confirmation_or_http() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server, "admin-1");
    let confirmation = FakeConfirmationProvider::accepting();

    for command in [
        user_command(&["take-ownership", "admin-1"]),
        user_command(&["disable-login", "admin-1"]),
    ] {
        let error = dispatch(
            command,
            &factory(&server),
            &store,
            &FakeSecrets::new("unused"),
            &confirmation,
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidInput);
    }
    assert!(confirmation.requests().is_empty());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn deleting_self_removes_only_the_matching_local_credential() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/api/users/admin-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "_id": "admin-1" })))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server, "admin-1");

    let success = dispatch(
        user_command(&["delete", "admin-1", "--yes"]),
        &factory(&server),
        &store,
        &FakeSecrets::new("unused"),
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();

    let CommandSuccess::UserDeleted(deleted) = success else {
        panic!("expected user deletion")
    };
    assert!(deleted.deleted);
    assert!(deleted.deleted_current_user);
    assert!(deleted.local_credential_removed);
    assert!(!deleted.credential_stored);
    assert!(store.record.lock().unwrap().is_none());
    assert_eq!(store.lock_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn self_delete_cleanup_failure_reports_the_completed_remote_deletion() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/api/users/admin-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "_id": "admin-1" })))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server, "admin-1");
    store.fail_next_delete();

    let error = dispatch(
        user_command(&["delete", "admin-1", "--yes"]),
        &factory(&server),
        &store,
        &FakeSecrets::new("unused"),
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
    assert_eq!(error.details().user_deleted, Some(true));
    assert_eq!(error.details().credential_stored, Some(true));
    assert!(store.record.lock().unwrap().is_some());
}

#[tokio::test]
async fn self_delete_aborts_before_confirmation_and_http_if_the_credential_was_replaced() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server, "admin-1");
    store.replace_on_next_lock(CredentialRecord::new(
        format!("{}/", server.uri()),
        "admin-1".to_owned(),
        SecretString::from("replacement-token".to_owned()),
        OffsetDateTime::now_utc() + Duration::hours(1),
    ));
    let confirmation = FakeConfirmationProvider::accepting();

    let error = dispatch(
        user_command(&["delete", "admin-1", "--yes"]),
        &factory(&server),
        &store,
        &FakeSecrets::new("unused"),
        &confirmation,
    )
    .await
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
    assert_eq!(store.lock_count.load(Ordering::SeqCst), 1);
    assert!(confirmation.requests().is_empty());
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(
        store
            .record
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .token()
            .expose_secret(),
        "replacement-token"
    );
}

#[tokio::test]
async fn embedded_permission_is_stable_and_unverified_not_found_is_preserved() {
    for (status, expected) in [
        (403, ErrorCode::PermissionDenied),
        (404, ErrorCode::ServerError),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/users/missing"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "statusCode": status,
                "error": "test",
                "reason": "test"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let store = FakeCredentialStore::authenticated(&server, "admin-1");
        let error = dispatch(
            user_command(&["get", "missing"]),
            &factory(&server),
            &store,
            &FakeSecrets::new("unused"),
            &FakeConfirmationProvider::accepting(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), expected);
        assert_eq!(error.details().http_status, Some(200));
        assert_eq!(error.details().wekan_status_code, Some(status));
    }
}

#[tokio::test]
async fn embedded_error_without_status_code_is_a_metadata_preserving_protocol_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/users/missing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "isClientSafe": true,
            "error": "user-not-found",
            "reason": "No such user",
            "message": "No such user [user-not-found]",
            "errorType": "Meteor.Error"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server, "admin-1");

    let error = dispatch(
        user_command(&["get", "missing"]),
        &factory(&server),
        &store,
        &FakeSecrets::new("unused"),
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::ProtocolError);
    assert_eq!(error.details().http_status, Some(200));
    assert_eq!(
        error.details().server_error.as_deref(),
        Some("user-not-found")
    );
    assert_eq!(
        error.details().server_reason.as_deref(),
        Some("No such user")
    );
    assert_eq!(
        error.details().server_message.as_deref(),
        Some("No such user [user-not-found]")
    );
    assert_eq!(
        error.details().server_error_type.as_deref(),
        Some("Meteor.Error")
    );
    assert_eq!(error.details().server_is_client_safe, Some(true));
}

#[test]
fn response_failures_use_http_status_and_transport_exit_codes() {
    let token = SecretString::from("user-token".to_owned());
    let redactor = Redactor::with_secret(&token);

    let rejected = super::map_client_error(
        ClientError::Server {
            status: reqwest::StatusCode::UNAUTHORIZED,
            server_error: None,
            server_reason: None,
            retry_after_seconds: None,
        },
        &redactor,
        "user lookup",
        false,
    );
    assert_eq!(rejected.code(), ErrorCode::AuthenticationRejected);
    assert!(
        rejected
            .message()
            .contains("auth logout --local-only --yes")
    );
    assert!(rejected.message().contains("then log in again"));

    let forbidden = super::map_client_error(
        ClientError::ResponseTooLarge {
            limit_bytes: 1024,
            status: reqwest::StatusCode::FORBIDDEN,
            retry_after_seconds: None,
        },
        &redactor,
        "user lookup",
        false,
    );
    assert_eq!(forbidden.code(), ErrorCode::PermissionDenied);
    assert_eq!(forbidden.exit_code(), StableExitCode::Server);

    let success_body = super::map_client_error(
        ClientError::ResponseTooLarge {
            limit_bytes: 1024,
            status: reqwest::StatusCode::OK,
            retry_after_seconds: None,
        },
        &redactor,
        "user lookup",
        false,
    );
    assert_eq!(success_body.code(), ErrorCode::ProtocolError);
    assert_eq!(success_body.exit_code(), StableExitCode::Transport);

    let redirect = super::map_client_error(
        ClientError::UnexpectedRedirect {
            status: reqwest::StatusCode::FOUND,
        },
        &redactor,
        "user lookup",
        false,
    );
    assert_eq!(redirect.exit_code(), StableExitCode::Transport);

    let protocol = super::map_client_error(
        ClientError::Protocol {
            message: "invalid JSON".to_owned(),
            success_status_received: true,
        },
        &redactor,
        "user lookup",
        false,
    );
    assert_eq!(protocol.exit_code(), StableExitCode::Transport);
}

#[test]
fn mutation_argument_types_keep_explicit_action_names() {
    assert!(matches!(
        user_command(&["take-ownership", "user-1", "--yes"]),
        UserCommand::TakeOwnership(args)
            if args.user_id == "user-1" && args.confirmation.yes()
    ));
    assert!(matches!(
        user_command(&["disable-login", "user-1", "--yes"]),
        UserCommand::DisableLogin(args)
            if args.user_id == "user-1" && args.confirmation.yes()
    ));
    assert!(matches!(
        user_command(&["delete", "user-1", "--yes"]),
        UserCommand::Delete(args)
            if args.user_id == "user-1" && args.confirmation.yes()
    ));
    let _ = ConfirmationArgs::default();
}
