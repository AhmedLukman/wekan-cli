use super::*;
use crate::input::ConfirmationRequest;
use crate::output::{OutputFormat, render_success};
use std::{
    sync::Arc,
    time::{Duration as StdDuration, Instant},
};

struct NoTokenInput;
impl SecretInputProvider for NoTokenInput {
    fn read_new_account_password(&self, _: bool) -> Result<SecretString, AppError> {
        panic!("token commands must not read passwords")
    }
    fn read_login_secrets(&self, _: LoginSecretMode) -> Result<LoginSecrets, AppError> {
        panic!("token commands must not read login secrets")
    }
    // The default raw-secret implementation errors, detecting unexpected reads.
}

#[tokio::test]
async fn token_commands_deliver_only_intended_secrets_and_preserve_credentials() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/createtoken/u"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"_id":"u","authToken":"issued-secret"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server, "u");
    let confirmation = FakeConfirmationProvider::accepting();
    let created = dispatch(
        user_command(&["token", "create", "u"]),
        &factory(&server),
        &store,
        &NoTokenInput,
        &confirmation,
    )
    .await
    .unwrap();
    assert!(!format!("{created:?}").contains("issued-secret"));
    for format in [OutputFormat::Json, OutputFormat::Human] {
        let rendered = render_success(format, &created);
        assert!(rendered.contains("issued-secret"));
        assert!(!rendered.contains("user-token"));
    }
    let json: serde_json::Value =
        serde_json::from_str(&render_success(OutputFormat::Json, &created)).unwrap();
    assert_eq!(json["data"]["user_id"], "u");
    assert_eq!(json["data"]["auth_token"], "issued-secret");
    assert_eq!(json["data"]["creation_requested"], true);
    assert_eq!(json["data"]["mutation_confirmed"], false);
    assert!(json["data"].get("token_expires").is_none());
    assert_eq!(json["data"].as_object().unwrap().len(), 4);
    let human = render_success(OutputFormat::Human, &created);
    assert!(human.contains("Creation requested: true"));
    assert!(human.contains("Mutation confirmed: false"));
    Mock::given(method("POST"))
        .and(path("/api/deletetoken"))
        .and(body_json(json!({"userId":"u","token":" target-secret "})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"message":"Delete [ target-secret ] using user-token"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let deleted = dispatch(
        user_command(&["token", "delete", "u", "--token-stdin", "--yes"]),
        &factory(&server),
        &store,
        &FakeSecrets::new(" target-secret \r\n"),
        &confirmation,
    )
    .await
    .unwrap();
    for format in [OutputFormat::Human, OutputFormat::Json] {
        let output = render_success(format, &deleted);
        assert!(!output.contains("target-secret"));
        assert!(!output.contains("user-token"));
        assert!(output.contains("[REDACTED]"));
    }
    let CommandSuccess::UserTokenDeleted(data) = deleted else {
        panic!("expected deletion")
    };
    assert_eq!(
        data.scope,
        crate::command_result::UserTokenDeleteScope::Single
    );
    assert!(data.deletion_requested);
    assert!(!data.mutation_confirmed);
    assert!(store.record.lock().unwrap().is_some());
    assert_eq!(store.lock_count.load(Ordering::SeqCst), 0);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn token_deletion_cancels_without_input_or_http_and_rejects_empty_input() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server, "u");
    let result = dispatch(
        user_command(&["token", "delete", "u", "--token-stdin"]),
        &factory(&server),
        &store,
        &NoTokenInput,
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();
    assert!(matches!(result, CommandSuccess::Cancelled(_)));
    let json: serde_json::Value =
        serde_json::from_str(&render_success(OutputFormat::Json, &result)).unwrap();
    assert_eq!(
        json["data"],
        json!({"cancelled": true, "operation": "user_token_delete"})
    );
    for input in ["", "\n", "\r\n"] {
        let error = dispatch(
            user_command(&["token", "delete", "u", "--token-stdin", "--yes"]),
            &factory(&server),
            &store,
            &FakeSecrets::new(input),
            &FakeConfirmationProvider::accepting(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidInput);
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn token_delete_all_omits_the_token_and_redacts_embedded_failures() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server, "u");
    Mock::given(method("POST"))
        .and(body_json(json!({"userId":"u"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"message":"Delete all token from user: u"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let deleted = dispatch(
        user_command(&["token", "delete", "u", "--all", "--yes"]),
        &factory(&server),
        &store,
        &NoTokenInput,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap();
    let CommandSuccess::UserTokenDeleted(data) = deleted else {
        panic!("expected deletion")
    };
    assert_eq!(data.scope, crate::command_result::UserTokenDeleteScope::All);
    assert!(store.record.lock().unwrap().is_some());
    server.reset().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "error":"bad-token", "reason":"target-secret / user-token", "errorType":"Meteor.Error", "isClientSafe":true
    }))).expect(1).mount(&server).await;
    let error = dispatch(
        user_command(&["token", "delete", "u", "--token-stdin", "--yes"]),
        &factory(&server),
        &store,
        &FakeSecrets::new("target-secret\n"),
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ProtocolError);
    let rendered = crate::output::render_error(OutputFormat::Json, &error);
    assert!(!rendered.contains("target-secret"));
    assert!(!rendered.contains("user-token"));
    assert!(rendered.contains("[REDACTED]"));
    assert!(rendered.contains("http_status"));
}

#[tokio::test]
async fn token_stdin_removes_only_one_final_line_ending() {
    for (input, expected) in [("t", "t"), ("t\n", "t"), ("t\n\n", "t\n"), ("t\r", "t\r")] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_json(json!({"userId":"u", "token":expected})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"message":"ack"})))
            .expect(1)
            .mount(&server)
            .await;
        let store = FakeCredentialStore::authenticated(&server, "u");
        dispatch(
            user_command(&["token", "delete", "u", "--token-stdin", "--yes"]),
            &factory(&server),
            &store,
            &FakeSecrets::new(input),
            &FakeConfirmationProvider::accepting(),
        )
        .await
        .unwrap();
        server.verify().await;
    }
}

fn replacement_record(server: &MockServer) -> CredentialRecord {
    CredentialRecord::new(
        format!("{}/", server.uri()),
        "u".into(),
        SecretString::from("replacement-token".to_owned()),
        OffsetDateTime::now_utc() + Duration::hours(1),
    )
}

struct ChangingConfirmation<'a> {
    store: &'a FakeCredentialStore,
    replacement: Option<CredentialRecord>,
}

impl ConfirmationProvider for ChangingConfirmation<'_> {
    fn confirm(&self, _: &ConfirmationRequest) -> Result<bool, AppError> {
        assert!(self.store.mutation_active.load(Ordering::Acquire));
        // Simulate stale guarded data to verify the post-confirmation read.
        *self.store.record.lock().unwrap() = self.replacement.clone();
        Ok(true)
    }
}

#[tokio::test]
async fn self_delete_all_rejects_replaced_or_removed_credentials_without_http() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server, "u");
    let replacement = replacement_record(&server);
    store.replace_on_next_lock(replacement.clone());
    let error = dispatch(
        user_command(&["token", "delete", "u", "--all", "--yes"]),
        &factory(&server),
        &store,
        &NoTokenInput,
        &FakeConfirmationProvider::accepting(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
    assert!(
        store
            .record
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .matches(&replacement)
    );
    assert!(!store.mutation_active.load(Ordering::Acquire));

    for replacement in [Some(replacement), None] {
        let store = FakeCredentialStore::authenticated(&server, "u");
        let error = dispatch(
            user_command(&["token", "delete", "u", "--all"]),
            &factory(&server),
            &store,
            &NoTokenInput,
            &ChangingConfirmation {
                store: &store,
                replacement: replacement.clone(),
            },
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.code(),
            if replacement.is_some() {
                ErrorCode::CredentialStoreFailed
            } else {
                ErrorCode::CredentialNotFound
            }
        );
        assert_eq!(error.exit_code(), StableExitCode::Credential);
        assert!(error.message().contains("no HTTP request was sent"));
        assert!(!store.mutation_active.load(Ordering::Acquire));
        let saved = store.record.lock().unwrap();
        if let Some(expected) = replacement {
            assert!(saved.as_ref().unwrap().matches(&expected));
        } else {
            assert!(saved.is_none());
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

struct ExpiringConfirmation<'a> {
    store: &'a FakeCredentialStore,
    expires: OffsetDateTime,
}

impl ConfirmationProvider for ExpiringConfirmation<'_> {
    fn confirm(&self, _: &ConfirmationRequest) -> Result<bool, AppError> {
        assert!(self.store.mutation_active.load(Ordering::Acquire));
        while OffsetDateTime::now_utc() < self.expires {
            std::thread::sleep(StdDuration::from_millis(10));
        }
        Ok(true)
    }
}

#[tokio::test]
async fn self_delete_all_rechecks_expiry_and_releases_the_guard_on_cancellation() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server, "u");
    let cancelled = dispatch(
        user_command(&["token", "delete", "u", "--all"]),
        &factory(&server),
        &store,
        &NoTokenInput,
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();
    assert!(matches!(cancelled, CommandSuccess::Cancelled(_)));
    assert!(!store.mutation_active.load(Ordering::Acquire));
    let expires = OffsetDateTime::now_utc() + Duration::seconds(2);
    let record = CredentialRecord::new(
        format!("{}/", server.uri()),
        "u".into(),
        SecretString::from("user-token".to_owned()),
        expires,
    );
    *store.record.lock().unwrap() = Some(record.clone());
    let error = dispatch(
        user_command(&["token", "delete", "u", "--all"]),
        &factory(&server),
        &store,
        &NoTokenInput,
        &ExpiringConfirmation {
            store: &store,
            expires,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::CredentialExpired);
    assert_eq!(error.exit_code(), StableExitCode::Server);
    assert!(!store.mutation_active.load(Ordering::Acquire));
    assert!(
        store
            .record
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .matches(&record)
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

struct ConcurrentConfirmation {
    store: Arc<FakeCredentialStore>,
    replacement: CredentialRecord,
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl ConfirmationProvider for ConcurrentConfirmation {
    fn confirm(&self, _: &ConfirmationRequest) -> Result<bool, AppError> {
        assert!(self.store.mutation_active.load(Ordering::Acquire));
        let store = self.store.clone();
        let replacement = self.replacement.clone();
        *self.worker.lock().unwrap() = Some(std::thread::spawn(move || {
            let deadline = Instant::now() + StdDuration::from_secs(5);
            while store.mutation_active.load(Ordering::Acquire) {
                assert!(
                    Instant::now() < deadline,
                    "credential guard was never released"
                );
                std::thread::sleep(StdDuration::from_millis(1));
            }
            *store.record.lock().unwrap() = Some(replacement);
        }));
        Ok(true)
    }
}

#[tokio::test]
async fn self_delete_all_holds_the_guard_through_http_and_preserves_a_waiting_replacement() {
    for status in [200, 503] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/deletetoken"))
            .and(header("authorization", "Bearer user-token"))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_delay(StdDuration::from_millis(300))
                    .set_body_json(json!({"message":"ack"})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let store = Arc::new(FakeCredentialStore::authenticated(&server, "u"));
        let original = store.record.lock().unwrap().clone().unwrap();
        let replacement = replacement_record(&server);
        let confirmation = ConcurrentConfirmation {
            store: store.clone(),
            replacement: replacement.clone(),
            worker: Mutex::new(None),
        };
        let factory = factory(&server);
        let operation = dispatch(
            user_command(&["token", "delete", "u", "--all"]),
            &factory,
            store.as_ref(),
            &NoTokenInput,
            &confirmation,
        );
        let operation = tokio::time::timeout(StdDuration::from_secs(5), operation);
        tokio::pin!(operation);
        let mut observed_request = false;
        loop {
            tokio::select! {
                result = &mut operation => {
                    assert_eq!(result.unwrap().is_ok(), status == 200);
                    break;
                }
                _ = tokio::time::sleep(StdDuration::from_millis(10)) => {
                    if !server.received_requests().await.unwrap().is_empty() {
                        observed_request = true;
                        assert!(store.mutation_active.load(Ordering::Acquire));
                        assert!(store.record.lock().unwrap().as_ref().unwrap().matches(&original));
                    }
                }
            }
        }
        confirmation
            .worker
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .join()
            .unwrap();
        assert!(observed_request);
        assert!(!store.mutation_active.load(Ordering::Acquire));
        assert!(
            store
                .record
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .matches(&replacement)
        );
        server.verify().await;
    }
}
