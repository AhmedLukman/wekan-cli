use crate::{
    cli::Cli,
    client::WekanClientFactory,
    command_result::{CommandSuccess, DestructiveOperation},
    commands::RootCommand,
    credentials::{
        CredentialCreateOutcome, CredentialDeleteOutcome, CredentialError, CredentialRecord,
        CredentialStore, CredentialTarget,
    },
    error::{AppError, ErrorCode},
    input::{ConfirmationProvider, ConfirmationRequest, FakeConfirmationProvider},
    output::{OutputFormat, render_success},
};
use clap::Parser;
use secrecy::SecretString;
use serde_json::{Value, json};
use std::sync::Mutex;
use time::{Duration, OffsetDateTime};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

struct Store {
    record: Mutex<Option<CredentialRecord>>,
}
impl Store {
    fn new(server: &MockServer) -> Self {
        Self {
            record: Mutex::new(Some(CredentialRecord::new(
                format!("{}/", server.uri()),
                "user".into(),
                SecretString::from("dependency-token".to_owned()),
                OffsetDateTime::now_utc() + Duration::hours(1),
            ))),
        }
    }
}
impl CredentialStore for Store {
    fn check_available(&self, _: &CredentialTarget) -> Result<(), CredentialError> {
        Ok(())
    }
    fn exists(&self, _: &CredentialTarget) -> Result<bool, CredentialError> {
        Ok(self.record.lock().unwrap().is_some())
    }
    fn load(&self, _: &CredentialTarget) -> Result<Option<CredentialRecord>, CredentialError> {
        Ok(self.record.lock().unwrap().clone())
    }
    fn create(
        &self,
        _: &CredentialTarget,
        _: &CredentialRecord,
    ) -> Result<CredentialCreateOutcome, CredentialError> {
        panic!("must not store credentials")
    }
    fn delete(&self, _: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("must not delete credentials")
    }
    fn delete_if_matches(
        &self,
        _: &CredentialTarget,
        _: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("must not delete credentials")
    }
}
async fn run(
    server: &MockServer,
    store: &Store,
    args: &[&str],
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "dependency"];
    argv.extend_from_slice(args);
    argv.extend(["--board", "b"]);
    let RootCommand::Dependency(args) = Cli::try_parse_from(argv).unwrap().command else {
        unreachable!()
    };
    super::dispatch(
        args.command,
        &WekanClientFactory::for_profile(format!("{}/", server.uri()), "default".into(), false),
        store,
        confirmation,
    )
    .await
}
fn data(result: &CommandSuccess) -> Value {
    serde_json::from_str::<Value>(&render_success(OutputFormat::Json, result)).unwrap()["data"]
        .clone()
}

#[tokio::test]
async fn dependency_dispatch_maps_all_outputs_without_extra_http() {
    let server = MockServer::start().await;
    let cases = [
        (
            vec!["list", "--card", "c"],
            "GET",
            "/api/boards/b/cards/c/dependencies",
            json!([{"cardId":"t", "type":"blocks", "color":"red", "icon":"lock"}]),
            None,
            json!({"board_id":"b", "card_id":"c", "dependencies":[{"target_card_id":"t", "dependency_type":"blocks", "color":"red", "icon":"lock"}]}),
        ),
        (
            vec!["list"],
            "GET",
            "/api/boards/b/dependencies",
            json!([{"from":"c", "to":"t", "type":"fixes", "color":"red", "icon":"bug"}]),
            None,
            json!({"board_id":"b", "dependencies":[{"source_card_id":"c", "source_title":null, "source_card_number":null, "target_card_id":"t", "target_title":null, "target_card_number":null, "dependency_type":"fixes", "color":"red", "icon":"bug"}]}),
        ),
        (
            vec![
                "create",
                "t",
                "--card",
                "c",
                "--null-type",
                "--color",
                "",
                "--icon",
                " link ",
            ],
            "POST",
            "/api/boards/b/cards/c/dependencies",
            json!({"_id":"c", "cardId":"t", "type":"related-to", "color":"red", "icon":" link "}),
            Some(json!({"cardId":"t", "type":null, "color":"", "icon":" link "})),
            json!({"board_id":"b", "card_id":"c", "target_card_id":"t", "dependency_type":"related-to", "color":"red", "icon":" link "}),
        ),
        (
            vec!["update", "t", "--card", "c", "--type", "", "--null-color"],
            "PUT",
            "/api/boards/b/cards/c/dependencies/t",
            json!({"_id":"c"}),
            Some(json!({"type":"", "color":null})),
            json!({"board_id":"b", "card_id":"c", "target_card_id":"t", "submitted_fields":["type", "color"], "mutation_confirmed":false}),
        ),
        (
            vec!["delete", "t", "--card", "c", "--yes"],
            "DELETE",
            "/api/boards/b/cards/c/dependencies/t",
            json!({"_id":"c"}),
            None,
            json!({"board_id":"b", "card_id":"c", "target_card_id":"t", "deletion_requested":true, "mutation_confirmed":false}),
        ),
    ];
    for (args, verb, url, response, body, expected) in cases {
        server.reset().await;
        let mut mock = Mock::given(method(verb))
            .and(path(url))
            .and(header("authorization", "Bearer dependency-token"));
        if let Some(body) = body {
            mock = mock.and(body_json(body));
        }
        mock.respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        let result = run(
            &server,
            &Store::new(&server),
            &args,
            &FakeConfirmationProvider::declining(),
        )
        .await
        .unwrap();
        assert_eq!(data(&result), expected);
        assert!(!render_success(OutputFormat::Human, &result).is_empty());
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        server.verify().await;
    }
}

struct ChangingConfirmation<'a> {
    store: &'a Store,
    action: &'static str,
}
impl ConfirmationProvider for ChangingConfirmation<'_> {
    fn confirm(&self, _: &ConfirmationRequest) -> Result<bool, AppError> {
        let mut record = self.store.record.lock().unwrap();
        match self.action {
            "missing" => *record = None,
            "changed" => {
                let old = record.as_ref().unwrap();
                *record = Some(CredentialRecord::new(
                    old.server_url().into(),
                    "other-user".into(),
                    SecretString::from("other-token".to_owned()),
                    old.token_expires(),
                ));
            }
            _ => unreachable!(),
        }
        Ok(true)
    }
}

#[tokio::test]
async fn dependency_deletion_cancels_or_rejects_changed_and_missing_credentials() {
    let server = MockServer::start().await;
    let args = ["delete", "t", "--card", "c"];
    let confirmation = FakeConfirmationProvider::declining();
    let result = run(&server, &Store::new(&server), &args, &confirmation)
        .await
        .unwrap();
    let CommandSuccess::Cancelled(cancelled) = result else {
        panic!("expected cancellation")
    };
    assert_eq!(cancelled.operation, DestructiveOperation::DependencyDelete);
    assert!(
        confirmation.requests()[0]
            .prompt()
            .contains("card `c` to card `t`")
    );
    for (action, code) in [
        ("missing", ErrorCode::CredentialNotFound),
        ("changed", ErrorCode::CredentialStoreFailed),
    ] {
        let store = Store::new(&server);
        let error = run(
            &server,
            &store,
            &args,
            &ChangingConfirmation {
                store: &store,
                action,
            },
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), code);
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn dependency_deletion_rechecks_expiry_after_confirmation() {
    struct ExpiringConfirmation {
        expires: OffsetDateTime,
        called: std::sync::atomic::AtomicBool,
    }
    impl ConfirmationProvider for ExpiringConfirmation {
        fn confirm(&self, _: &ConfirmationRequest) -> Result<bool, AppError> {
            self.called.store(true, std::sync::atomic::Ordering::SeqCst);
            while OffsetDateTime::now_utc() < self.expires {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Ok(true)
        }
    }
    let server = MockServer::start().await;
    let store = Store::new(&server);
    let expires = OffsetDateTime::now_utc() + Duration::seconds(2);
    *store.record.lock().unwrap() = Some(CredentialRecord::new(
        format!("{}/", server.uri()),
        "user".into(),
        SecretString::from("dependency-token".to_owned()),
        expires,
    ));
    let confirmation = ExpiringConfirmation {
        expires,
        called: std::sync::atomic::AtomicBool::new(false),
    };
    let error = run(
        &server,
        &store,
        &["delete", "t", "--card", "c"],
        &confirmation,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::CredentialExpired);
    assert!(
        confirmation
            .called
            .load(std::sync::atomic::Ordering::SeqCst)
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn dependency_errors_preserve_status_redact_secrets_and_report_mutation_outcomes() {
    let server = MockServer::start().await;
    Mock::given(method("PUT")).and(body_json(json!({}))).respond_with(ResponseTemplate::new(500).set_body_json(json!({"error":"bad-request", "reason":"dependency-token cannot update", "errorType":"Meteor.Error", "isClientSafe":true, "message":"dependency-token cannot update"}))).expect(1).mount(&server).await;
    let error = run(
        &server,
        &Store::new(&server),
        &["update", "t", "--card", "c"],
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.details().http_status, Some(500));
    assert!(!format!("{error:?}").contains("dependency-token"));
    server.verify().await;
    server.reset().await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"c", "unmapped":true})))
        .mount(&server)
        .await;
    let error = run(
        &server,
        &Store::new(&server),
        &["update", "t", "--card", "c", "--color", "red"],
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ProtocolError);
    assert_eq!(error.details().outcome_unknown, Some(true));
}

#[test]
fn dependency_renderers_escape_terminal_controls_and_preserve_optional_numbers() {
    use crate::command_result::{BoardDependencyCollectionSuccess, DependencyLine};
    let result = CommandSuccess::BoardDependencies(BoardDependencyCollectionSuccess {
        board_id: "b".into(),
        dependencies: vec![DependencyLine {
            source_card_id: "c\n".into(),
            source_title: Some("Title\u{1b}".into()),
            source_card_number: Some(serde_json::Number::from(9007199254740993_u64)),
            target_card_id: "t".into(),
            target_title: None,
            target_card_number: None,
            dependency_type: "blocks".into(),
            color: "red".into(),
            icon: "lock".into(),
        }],
    });
    let human = render_success(OutputFormat::Human, &result);
    assert!(!human.contains('\u{1b}'));
    assert!(human.contains("9007199254740993"));
    assert_eq!(
        data(&result)["dependencies"][0]["source_title"],
        "Title\u{1b}"
    );
}
