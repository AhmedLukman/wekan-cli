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

struct Secrets(&'static str);
impl crate::credentials::SecretInputProvider for Secrets {
    fn read_raw_secret(&self) -> Result<SecretString, AppError> {
        Ok(SecretString::from(self.0.to_owned()))
    }
    fn read_new_account_password(&self, _: bool) -> Result<SecretString, AppError> {
        panic!("not used")
    }
    fn read_login_secrets(
        &self,
        _: crate::credentials::LoginSecretMode,
    ) -> Result<crate::credentials::LoginSecrets, AppError> {
        panic!("not used")
    }
}

async fn run(
    server: &MockServer,
    store: &Store,
    args: &[&str],
    secrets: &Secrets,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "integration"];
    argv.extend_from_slice(args);
    argv.extend(["--board", "b"]);
    let RootCommand::Integration(args) = Cli::try_parse_from(argv).unwrap().command else {
        unreachable!()
    };
    super::dispatch(
        args.command,
        &WekanClientFactory::for_profile(format!("{}/", server.uri()), "default".into(), false),
        store,
        secrets,
        confirmation,
    )
    .await
}

fn data(result: &CommandSuccess) -> Value {
    serde_json::from_str::<Value>(&render_success(OutputFormat::Json, result)).unwrap()["data"]
        .clone()
}

fn document() -> Value {
    json!({"_id":"i", "enabled":true, "type":"outgoing-webhooks", "activities":["all"], "url":"https://8.8.8.8/hook", "boardId":"b", "userId":"u", "createdAt":"2026-10-14T00:00:00.000Z", "modifiedAt":"2026-10-14T00:00:00.000Z"})
}

#[tokio::test]
async fn all_commands_map_outputs_and_send_one_native_request() {
    let server = MockServer::start().await;
    let detail = json!({"integration_id":"i", "enabled":true, "title":null, "integration_type":"outgoing-webhooks", "activities":["all"], "url":"https://8.8.8.8/hook", "board_id":"b", "user_id":"u", "created_at":"2026-10-14T00:00:00.000Z", "modified_at":"2026-10-14T00:00:00.000Z"});
    for (args, verb, url, body, response, expected) in [
        (
            vec!["list"],
            "GET",
            "/api/boards/b/integrations",
            None,
            json!([document()]),
            json!({"board_id":"b", "integrations":[detail.clone()]}),
        ),
        (
            vec!["get", "i"],
            "GET",
            "/api/boards/b/integrations/i",
            None,
            document(),
            detail,
        ),
        (
            vec!["create", "--url", "https://8.8.8.8/hook"],
            "POST",
            "/api/boards/b/integrations",
            Some(json!({"url":"https://8.8.8.8/hook"})),
            json!({"_id":"i"}),
            json!({"board_id":"b", "integration_id":"i"}),
        ),
        (
            vec![
                "update",
                "i",
                "--enabled",
                "false",
                "--title",
                "",
                "--url",
                "https://8.8.8.8/new",
                "--token-stdin",
                "--activity",
                "a",
                "--activity",
                "a",
            ],
            "PUT",
            "/api/boards/b/integrations/i",
            Some(
                json!({"enabled":false, "title":"", "url":"https://8.8.8.8/new", "token":" webhook-secret ", "activities":["a","a"]}),
            ),
            json!({"_id":"i"}),
            json!({"board_id":"b", "integration_id":"i", "submitted_fields":["enabled","title","url","token","activities"], "mutation_confirmed":false}),
        ),
        (
            vec!["delete", "i", "--yes"],
            "DELETE",
            "/api/boards/b/integrations/i",
            None,
            json!({"_id":"i"}),
            json!({"board_id":"b", "integration_id":"i", "deletion_requested":true, "mutation_confirmed":false}),
        ),
        (
            vec!["activity", "add", "i", "--activity", "a", "--activity", "a"],
            "POST",
            "/api/boards/b/integrations/i/activities",
            Some(json!({"activities":["a","a"]})),
            json!({"_id":"i", "activities":["all","a"]}),
            json!({"board_id":"b", "integration_id":"i", "action":"add", "requested_activities":["a","a"], "activities":["all","a"]}),
        ),
        (
            vec!["activity", "remove", "i", "--activity", "all"],
            "DELETE",
            "/api/boards/b/integrations/i/activities",
            Some(json!({"activities":["all"]})),
            json!({"_id":"i", "activities":["a"]}),
            json!({"board_id":"b", "integration_id":"i", "action":"remove", "requested_activities":["all"], "activities":["a"]}),
        ),
    ] {
        server.reset().await;
        let mut mock = Mock::given(method(verb))
            .and(path(url))
            .and(header("authorization", "Bearer auth-token"));
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
            &Secrets(" webhook-secret \r\n"),
            &FakeConfirmationProvider::declining(),
        )
        .await
        .unwrap();
        assert_eq!(data(&result), expected);
        assert!(!render_success(OutputFormat::Human, &result).is_empty());
        assert!(!format!("{result:?}").contains("webhook-secret"));
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        server.verify().await;
    }
}

#[tokio::test]
async fn token_input_preserves_bytes_and_empty_updates_and_clear_forms_are_explicit() {
    let server = MockServer::start().await;
    for (args, input, body) in [
        (vec!["update", "i"], "", json!({})),
        (
            vec!["update", "i", "--clear-token", "--clear-activities"],
            "",
            json!({"token":"", "activities":[]}),
        ),
        (
            vec!["update", "i", "--token-stdin"],
            " a\nb \n",
            json!({"token":" a\nb "}),
        ),
        (
            vec!["update", "i", "--token-stdin"],
            "token",
            json!({"token":"token"}),
        ),
    ] {
        server.reset().await;
        Mock::given(method("PUT"))
            .and(body_json(body))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"i"})))
            .expect(1)
            .mount(&server)
            .await;
        run(
            &server,
            &Store::new(&server),
            &args,
            &Secrets(input),
            &FakeConfirmationProvider::declining(),
        )
        .await
        .unwrap();
        server.verify().await;
    }
    server.reset().await;
    for input in ["", "\n", "\r\n"] {
        let error = run(
            &server,
            &Store::new(&server),
            &["update", "i", "--token-stdin"],
            &Secrets(input),
            &FakeConfirmationProvider::declining(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidInput);
    }
    assert!(server.received_requests().await.unwrap().is_empty());
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
        &["delete", "i"],
        &Secrets(""),
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();
    assert_eq!(
        data(&result),
        json!({"cancelled":true, "operation":"integration_delete"})
    );
    assert_eq!(
        DestructiveOperation::IntegrationDelete.as_command(),
        "integration delete"
    );
    for (remove, code) in [
        (true, ErrorCode::CredentialNotFound),
        (false, ErrorCode::CredentialStoreFailed),
    ] {
        let store = Store::new(&server);
        assert_eq!(
            run(
                &server,
                &store,
                &["delete", "i"],
                &Secrets(""),
                &Changing(&store, remove)
            )
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
    let error = run(
        &server,
        &store,
        &["delete", "i"],
        &Secrets(""),
        &Expiring(expires),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::CredentialExpired);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn errors_redact_both_tokens_and_mark_partial_update_outcomes() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(400).set_body_json(
            json!({"error":"invalid-webhook-url", "reason":"auth-token webhook-secret"}),
        ))
        .mount(&server)
        .await;
    let error = run(
        &server,
        &Store::new(&server),
        &[
            "update",
            "i",
            "--enabled",
            "false",
            "--token-stdin",
            "--url",
            "http://localhost",
        ],
        &Secrets("webhook-secret\n"),
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.details().http_status, Some(400));
    assert_eq!(error.details().outcome_unknown, Some(true));
    for output in [
        format!("{error:?}"),
        crate::output::render_error(OutputFormat::Human, &error),
        crate::output::render_error(OutputFormat::Json, &error),
    ] {
        assert!(!output.contains("auth-token"));
        assert!(!output.contains("webhook-secret"));
    }
    let request = crate::client::UpdateIntegrationRequest {
        token: Some(SecretString::from("webhook-secret".to_owned())),
        ..Default::default()
    };
    assert!(!format!("{request:?}").contains("webhook-secret"));
    server.reset().await;
    Mock::given(method("PUT"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"_id":"i", "unknown":"webhook-secret"})),
        )
        .mount(&server)
        .await;
    let error = run(
        &server,
        &Store::new(&server),
        &["update", "i"],
        &Secrets(""),
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ProtocolError);
    assert_eq!(error.details().outcome_unknown, Some(true));
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let error = run(
        &server,
        &Store::new(&server),
        &["get", "i"],
        &Secrets(""),
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::NotFound);
    assert_eq!(error.details().http_status, Some(200));
}

#[test]
fn human_detail_renders_every_field_and_escapes_controls() {
    let mut value = document();
    value["title"] = json!("Hook\u{1b}");
    value["activities"] = json!(["all", "future\u{1b}"]);
    let result = CommandSuccess::IntegrationShown(
        serde_json::from_value::<crate::client::IntegrationDocument>(value)
            .unwrap()
            .into(),
    );
    let human = render_success(OutputFormat::Human, &result);
    assert!(!human.contains('\u{1b}'));
    let output = data(&result);
    for field in output.as_object().unwrap().keys() {
        assert!(human.contains(field), "missing {field}");
    }
    assert_eq!(output["title"], "Hook\u{1b}");
}
