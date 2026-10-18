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
                SecretString::from("auth-token".to_owned()),
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

#[tokio::test]
async fn deletion_cancels_and_rechecks_credentials_after_confirmation() {
    struct Changing<'a>(&'a Store, bool);
    impl ConfirmationProvider for Changing<'_> {
        fn confirm(&self, _: &ConfirmationRequest) -> Result<bool, AppError> {
            let mut record = self.0.record.lock().unwrap();
            if self.1 {
                *record = None;
            } else {
                *record = Some(CredentialRecord::new(
                    record.as_ref().unwrap().server_url().into(),
                    "other".into(),
                    SecretString::from("changed".to_owned()),
                    OffsetDateTime::now_utc() + Duration::hours(1),
                ));
            }
            Ok(true)
        }
    }
    let server = MockServer::start().await;
    let result = run(
        &server,
        &Store::new(&server),
        &["delete", "r"],
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();
    assert_eq!(
        data(&result),
        json!({"cancelled":true, "operation":"rule_delete"})
    );
    assert_eq!(DestructiveOperation::RuleDelete.as_command(), "rule delete");
    for (remove, code) in [
        (true, ErrorCode::CredentialNotFound),
        (false, ErrorCode::CredentialStoreFailed),
    ] {
        let store = Store::new(&server);
        assert_eq!(
            run(&server, &store, &["delete", "r"], &Changing(&store, remove))
                .await
                .unwrap_err()
                .code(),
            code
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn deletion_rechecks_expiry_after_confirmation_without_sending_http() {
    struct Expiring(OffsetDateTime);
    impl ConfirmationProvider for Expiring {
        fn confirm(&self, _: &ConfirmationRequest) -> Result<bool, AppError> {
            while OffsetDateTime::now_utc() < self.0 {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Ok(true)
        }
    }
    let server = MockServer::start().await;
    let store = Store::new(&server);
    let expires = OffsetDateTime::now_utc() + Duration::milliseconds(200);
    *store.record.lock().unwrap() = Some(CredentialRecord::new(
        format!("{}/", server.uri()),
        "user".into(),
        SecretString::from("auth-token".to_owned()),
        expires,
    ));
    let error = run(&server, &store, &["delete", "r"], &Expiring(expires))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::CredentialExpired);
    assert!(server.received_requests().await.unwrap().is_empty());
}

async fn run(
    server: &MockServer,
    store: &Store,
    args: &[&str],
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "rule"];
    argv.extend_from_slice(args);
    argv.extend(["--board", "b"]);
    let RootCommand::Rule(args) = Cli::try_parse_from(argv).unwrap().command else {
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
async fn commands_map_full_results_and_sparse_updates_with_one_native_request() {
    let server = MockServer::start().await;
    for (args, verb, response, expected, body) in [
        (
            vec!["list"],
            "GET",
            json!([]),
            json!({"board_id":"b","rules":[]}),
            None,
        ),
        (
            vec!["get", "r"],
            "GET",
            json!({"_id":"r","title":"Title","trigger":{"activityType":"button","buttonLabel":"Run","desc":null},"action":null}),
            json!({"board_id":"b","rule_id":"r","title":"Title","trigger":{"activity_type":"button","button_label":"Run","desc":null},"action":null}),
            None,
        ),
        (
            vec![
                "create",
                "--rule",
                r#"{"trigger":{"activityType":"moveCard"},"action":{"actionType":"archive"}}"#,
            ],
            "POST",
            json!({"_id":"r","triggerId":"t","actionId":"a"}),
            json!({"board_id":"b","rule_id":"r","trigger_id":"t","action_id":"a"}),
            Some(json!({"trigger":{"activityType":"moveCard"},"action":{"actionType":"archive"}})),
        ),
        (
            vec!["update", "r", "--rule", "{}"],
            "PUT",
            json!({"_id":"r"}),
            json!({"board_id":"b","rule_id":"r","submitted_fields":[]}),
            Some(json!({})),
        ),
        (
            vec![
                "update",
                "r",
                "--rule",
                r#"{"title":"","trigger":{"listName":null},"action":{}}"#,
            ],
            "PUT",
            json!({"_id":"r"}),
            json!({"board_id":"b","rule_id":"r","submitted_fields":["title","trigger","action"]}),
            Some(json!({"title":"","trigger":{"listName":null},"action":{}})),
        ),
        (
            vec!["delete", "r", "--yes"],
            "DELETE",
            json!({"_id":"r"}),
            json!({"board_id":"b","rule_id":"r","deleted":true}),
            None,
        ),
    ] {
        server.reset().await;
        let mut mock = Mock::given(method(verb)).and(header("authorization", "Bearer auth-token"));
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
        let human = render_success(OutputFormat::Human, &result);
        assert!(!human.is_empty());
        if verb == "PUT" {
            assert!(human.starts_with("Rule updated:"));
        }
        server.verify().await;
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}
#[tokio::test]
async fn invalid_json_and_create_requirements_fail_without_http() {
    let server = MockServer::start().await;
    for (command, json) in [
        ("create", "{}"),
        ("create", r#"{"trigger":{},"action":{}}"#),
        (
            "create",
            r#"{"trigger":{"activityType":""},"action":{"actionType":"archive"}}"#,
        ),
        (
            "create",
            r#"{"trigger":{"activityType":"button"},"action":{"actionType":null}}"#,
        ),
        ("update", "[]"),
        ("update", "null"),
        ("update", "{"),
        ("update", r#"{"unknown":"secret"}"#),
        ("update", r#"{"trigger":[]}"#),
        ("update", r#"{"action":{"username":{"password":"secret"}}}"#),
        ("update", r#"{"title":null}"#),
        ("update", r#"{"action":null}"#),
    ] {
        let mut args = vec![command];
        if command == "update" {
            args.push("r");
        }
        args.extend(["--rule", json]);
        let e = run(
            &server,
            &Store::new(&server),
            &args,
            &FakeConfirmationProvider::declining(),
        )
        .await
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidInput);
        assert!(!e.to_string().contains("secret"));
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}
#[tokio::test]
async fn invalid_timestamps_fail_before_credentials_and_http() {
    let server = MockServer::start().await;
    let store = Store {
        record: Mutex::new(None),
    };
    for date in [
        "not-a-date",
        "",
        "2026-02-30T00:00:00Z",
        "+010001-02-29T00:00:00.000Z",
        "+275760-09-13T00:00:00.001Z",
        "-000000-01-01T00:00:00.000Z",
    ] {
        let definition = json!({
            "trigger": {"activityType":"button", "lastRunAt":date},
            "action": {"actionType":"archive"}
        })
        .to_string();
        for args in [
            vec!["create", "--rule", &definition],
            vec!["update", "r", "--rule", &definition],
        ] {
            let error = run(
                &server,
                &store,
                &args,
                &FakeConfirmationProvider::declining(),
            )
            .await
            .unwrap_err();
            assert_eq!(error.code(), ErrorCode::InvalidInput);
            assert!(!error.to_string().contains(date) || date.is_empty());
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn file_sources_are_bounded_regular_and_typed() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("rule.json");
    std::fs::write(&file, r#"{"title":"new"}"#).unwrap();
    Mock::given(method("PUT"))
        .and(body_json(json!({"title":"new"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"r"})))
        .expect(1)
        .mount(&server)
        .await;
    run(
        &server,
        &Store::new(&server),
        &["update", "r", "--rule-file", file.to_str().unwrap()],
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();
    server.verify().await;
    server.reset().await;
    for path in [dir.path().to_owned(), dir.path().join("missing")] {
        assert!(
            run(
                &server,
                &Store::new(&server),
                &["update", "r", "--rule-file", path.to_str().unwrap()],
                &FakeConfirmationProvider::declining()
            )
            .await
            .is_err()
        );
    }
    let file = std::fs::File::create(&file).unwrap();
    file.set_len(50 * 1024 * 1024 + 1).unwrap();
    assert_eq!(
        run(
            &server,
            &Store::new(&server),
            &[
                "update",
                "r",
                "--rule-file",
                dir.path().join("rule.json").to_str().unwrap()
            ],
            &FakeConfirmationProvider::declining()
        )
        .await
        .unwrap_err()
        .code(),
        ErrorCode::InvalidInput
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}
#[tokio::test]
async fn mutation_failures_preserve_server_status_and_possible_partial_effects() {
    let server = MockServer::start().await;
    for args in [
        vec![
            "create",
            "--rule",
            r#"{"trigger":{"activityType":"button"},"action":{"actionType":"archive"}}"#,
        ],
        vec!["update", "r", "--rule", "{}"],
        vec!["delete", "r", "--yes"],
    ] {
        server.reset().await;
        Mock::given(path(if args[0]=="create" {"/api/boards/b/rules"} else {"/api/boards/b/rules/r"}))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({"error":"not-found","reason":"Rule not found","errorType":"Meteor.Error","isClientSafe":true,"message":"Rule not found [not-found]"}))).expect(1).mount(&server).await;
        let e = run(
            &server,
            &Store::new(&server),
            &args,
            &FakeConfirmationProvider::declining(),
        )
        .await
        .unwrap_err();
        assert_eq!(e.code(), ErrorCode::ServerError);
        assert_eq!(e.details().http_status, Some(500));
        assert_eq!(e.details().server_error.as_deref(), Some("not-found"));
        assert!(
            serde_json::to_value(e.details())
                .unwrap()
                .get("outcome_unknown")
                .is_none()
        );
        server.verify().await;
    }
}
#[tokio::test]
async fn human_output_escapes_untrusted_rule_text() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"r","title":"x\u{1b}[31m","trigger":null,"action":{"emailMsg":"\u{1b}]52;c;secret\u{7}"}}))).mount(&server).await;
    let result = run(
        &server,
        &Store::new(&server),
        &["get", "r"],
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();
    let human = render_success(OutputFormat::Human, &result);
    assert!(!human.contains('\u{1b}'));
    assert!(!human.contains('\u{7}'));
}
