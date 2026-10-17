use clap::{CommandFactory, Parser};
use secrecy::SecretString;
use serde_json::{Value, json};
use wekan_cli::{
    app::App,
    cli::Cli,
    client::{ClientError, auth::AuthEndpoint},
    command_result::CommandSuccess,
    config::{ServerSelection, profiles::FileProfileStore},
    credentials::{
        CredentialCreateOutcome, CredentialError, CredentialRecord, CredentialStore,
        CredentialTarget, LoginSecretMode, LoginSecrets, SecretInputProvider,
    },
    error::AppError,
    output::{OutputFormat, render_success},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

fn state() -> Value {
    serde_json::from_str(include_str!(
        "../fixtures/responses/schema_upgrade_status.json"
    ))
    .unwrap()
}

#[test]
fn remaining_coverage_rows_match_the_parser() {
    let coverage = include_str!("../../docs/api-coverage.md");
    let mut root = Cli::command();
    root.build();
    for (commands, operation, syntax) in [
        (
            vec!["board", "attachment", "list"],
            "get_board_attachments",
            "wekan board attachment list --board <BOARD_ID>",
        ),
        (
            vec!["system", "schema-upgrade-status"],
            "get_schema_upgrade_status",
            "wekan system schema-upgrade-status",
        ),
        (
            vec!["auth", "preflight"],
            "options_login",
            "wekan auth preflight login",
        ),
        (
            vec!["auth", "preflight"],
            "options_register",
            "wekan auth preflight register",
        ),
        (
            vec!["auth", "preflight"],
            "options_logout",
            "wekan auth preflight logout",
        ),
    ] {
        let row = coverage
            .lines()
            .find(|line| line.contains(&format!("| `{operation}` |")))
            .unwrap();
        assert!(row.contains(syntax), "{operation}");
        assert!(row.ends_with("| ✓ |"));
        let leaf = commands.iter().fold(&root, |node, command| {
            node.find_subcommand(command).unwrap()
        });
        for arg in leaf.get_arguments() {
            if let Some(long) = arg.get_long() {
                assert!(
                    row.contains(&format!("--{long}")),
                    "{operation} missing {long}"
                );
            }
        }
    }
}

#[tokio::test]
async fn public_requests_use_exact_routes_without_credentials_or_bodies() {
    let server = MockServer::start().await;
    let client = super::client_from_url(&format!("{}/wekan/", server.uri()));
    for endpoint in [
        AuthEndpoint::Login,
        AuthEndpoint::Register,
        AuthEndpoint::Logout,
    ] {
        Mock::given(method("OPTIONS"))
            .and(path(format!("/wekan/{}", endpoint.path())))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;
        client.auth_preflight(endpoint).await.unwrap();
    }
    Mock::given(method("GET"))
        .and(path("/wekan/schema-upgrade-status"))
        .and(query_param("json", "1"))
        .and(header("accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(state()))
        .expect(1)
        .mount(&server)
        .await;
    let result = client.schema_upgrade_status().await.unwrap();
    assert_eq!(
        result.steps.keys().map(String::as_str).collect::<Vec<_>>(),
        ["archived-flag-backfill", "swimlane-structure"]
    );
    assert_eq!(
        result.steps["archived-flag-backfill"].error.as_deref(),
        Some("fixture failure")
    );
    for request in server.received_requests().await.unwrap() {
        assert!(!request.headers.contains_key("authorization"));
        assert!(!request.headers.contains_key("content-type"));
        assert!(request.body.is_empty());
        assert!(!request.url.query_pairs().any(|(k, _)| k == "access_token"));
    }
    server.verify().await;
}

#[tokio::test]
async fn preflight_rejects_nonempty_bodies_and_unexpected_statuses() {
    let server = MockServer::start().await;
    for (status, body) in [
        (200, " "),
        (200, "{}"),
        (200, "<html>login</html>"),
        (204, ""),
        (401, ""),
        (500, ""),
    ] {
        server.reset().await;
        Mock::given(method("OPTIONS"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .mount(&server)
            .await;
        let error = super::client(&server)
            .auth_preflight(AuthEndpoint::Login)
            .await
            .unwrap_err();
        if status == 200 {
            assert!(matches!(
                error,
                ClientError::Protocol {
                    success_status_received: true,
                    ..
                }
            ));
        } else {
            assert!(matches!(error, ClientError::Server { status: s, .. } if s.as_u16() == status));
        }
    }
}

#[tokio::test]
async fn schema_state_rejects_unknown_fields_and_missing_or_invalid_known_fields() {
    let server = MockServer::start().await;
    let valid = state();
    let mut invalid = Vec::new();
    for pointer in ["", "/steps/swimlane-structure", "/lastCheck"] {
        let mut value = valid.clone();
        value.pointer_mut(pointer).unwrap()["unmapped"] = json!(true);
        invalid.push(value);
    }
    for pointer in ["", "/steps/swimlane-structure", "/lastCheck"] {
        for field in valid.pointer(pointer).unwrap().as_object().unwrap().keys() {
            let mut value = valid.clone();
            value
                .pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(field);
            invalid.push(value);
        }
    }
    for (pointer, bad) in [
        ("/running", json!("false")),
        ("/product", Value::Null),
        ("/startedAt", json!("not-a-date")),
        ("/finishedAt", json!(false)),
        ("/lastCheck/at", json!("2026-02-30T00:00:00Z")),
        ("/steps/swimlane-structure/status", json!("unknown")),
        ("/steps/swimlane-structure/fixed", json!(1.5)),
        ("/steps/swimlane-structure/unresolved", Value::Null),
        ("/steps/archived-flag-backfill/error", Value::Null),
    ] {
        let mut value = valid.clone();
        *value.pointer_mut(pointer).unwrap() = bad;
        invalid.push(value);
    }
    for value in invalid {
        server.reset().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(value.clone()))
            .mount(&server)
            .await;
        assert!(
            matches!(
                super::client(&server).schema_upgrade_status().await,
                Err(ClientError::Protocol { .. })
            ),
            "accepted {value}"
        );
    }
}

#[tokio::test]
async fn schema_state_accepts_initial_nulls_and_every_native_step_status() {
    let server = MockServer::start().await;
    let mut value = state();
    value["startedAt"] = Value::Null;
    value["finishedAt"] = Value::Null;
    value["lastCheck"] = Value::Null;
    value["steps"] = json!({});
    value["appVersion"] = json!("");
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(value.clone()))
        .mount(&server)
        .await;
    let initial = super::client(&server)
        .schema_upgrade_status()
        .await
        .unwrap();
    assert!(initial.app_version.is_empty());
    assert!(initial.steps.is_empty());
    assert!(initial.started_at.is_none());
    assert!(initial.finished_at.is_none());
    assert!(initial.last_check.is_none());
    server.reset().await;
    value["appVersion"] = json!("v11.06.0");
    for status in ["pending", "checking", "running", "done", "skipped", "error"] {
        value["steps"][status] = json!({"status":status,"fixed":0,"unresolved":0});
    }
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(value))
        .mount(&server)
        .await;
    let decoded = super::client(&server)
        .schema_upgrade_status()
        .await
        .unwrap();
    let output = serde_json::to_value(decoded).unwrap();
    assert_eq!(output["started_at"], Value::Null);
    assert_eq!(output["finished_at"], Value::Null);
    assert_eq!(output["last_check"], Value::Null);
    assert_eq!(output["steps"].as_object().unwrap().len(), 6);
}

#[tokio::test]
async fn board_attachments_preserve_exact_request_projection_and_optionality() {
    let server = MockServer::start().await;
    let client = super::client_from_url(&format!("{}/wekan/", server.uri()));
    let item = json!({"attachmentId":"a", "attachmentName":"report", "attachmentType":"text/plain", "boardId":"b/one", "url":"/file", "urlDownload":"/file?download=true&token=", "swimlaneId":"s", "listId":"l", "cardId":"c"});
    for include_optional in [true, false] {
        server.reset().await;
        let mut value = item.clone();
        if !include_optional {
            for field in ["url", "urlDownload", "swimlaneId", "listId", "cardId"] {
                value.as_object_mut().unwrap().remove(field);
            }
        }
        Mock::given(method("GET"))
            .and(path("/wekan/api/boards/b%2Fone/attachments"))
            .and(header("authorization", "Bearer board-token"))
            .and(header("accept", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([value])))
            .expect(1)
            .mount(&server)
            .await;
        let result = client
            .board_attachments("b/one", &SecretString::from("board-token"))
            .await
            .unwrap();
        assert_eq!(result[0].attachment_name, "report");
        assert_eq!(result[0].url.is_some(), include_optional);
        assert_eq!(result[0].card_id.is_some(), include_optional);
        let request = server.received_requests().await.unwrap().remove(0);
        assert!(request.body.is_empty());
        server.verify().await;
    }
    let mut invalid = vec![json!({"attachments":[]})];
    let mut extra = item.clone();
    extra["unmapped"] = json!({"nested":true});
    invalid.push(json!([extra]));
    for field in [
        "attachmentId",
        "attachmentName",
        "attachmentType",
        "boardId",
    ] {
        let mut missing = item.clone();
        missing.as_object_mut().unwrap().remove(field);
        invalid.push(json!([missing]));
    }
    for field in item.as_object().unwrap().keys() {
        for bad in [Value::Null, json!(42)] {
            let mut wrong = item.clone();
            wrong[field] = bad;
            invalid.push(json!([wrong]));
        }
    }
    for value in invalid {
        server.reset().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(value))
            .mount(&server)
            .await;
        assert!(matches!(
            client
                .board_attachments("b/one", &SecretString::from("board-token"))
                .await,
            Err(ClientError::Protocol { .. })
        ));
    }
}

struct NoCredentials;
impl CredentialStore for NoCredentials {
    fn delete_if_matches(
        &self,
        _: &CredentialTarget,
        _: &CredentialRecord,
    ) -> Result<wekan_cli::credentials::CredentialDeleteOutcome, CredentialError> {
        panic!("public probes must not delete credentials")
    }
    fn check_available(&self, _: &CredentialTarget) -> Result<(), CredentialError> {
        panic!("public probes must not access credentials")
    }
    fn exists(&self, _: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("public probes must not inspect credentials")
    }
    fn load(&self, _: &CredentialTarget) -> Result<Option<CredentialRecord>, CredentialError> {
        panic!("public probes must not load credentials")
    }
    fn create(
        &self,
        _: &CredentialTarget,
        _: &CredentialRecord,
    ) -> Result<CredentialCreateOutcome, CredentialError> {
        panic!("public probes must not save credentials")
    }
    fn delete(&self, _: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("public probes must not delete credentials")
    }
}
struct NoSecrets;
impl SecretInputProvider for NoSecrets {
    fn read_raw_secret(&self) -> Result<SecretString, AppError> {
        panic!("public probes must not prompt")
    }
    fn read_new_account_password(&self, _: bool) -> Result<SecretString, AppError> {
        panic!("public probes must not prompt")
    }
    fn read_login_secrets(&self, _: LoginSecretMode) -> Result<LoginSecrets, AppError> {
        panic!("public probes must not prompt")
    }
}

#[tokio::test]
async fn public_dispatch_never_accesses_credentials_or_creates_profiles() {
    let server = MockServer::start().await;
    let directory = tempfile::tempdir().unwrap();
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.uri()), Some("new-profile".into()), false),
        NoCredentials,
        NoSecrets,
        FileProfileStore::at(directory.path().to_owned()),
    );
    Mock::given(method("OPTIONS"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let mut value = state();
    value["product"] = json!("WeKan\u{001b}[2J");
    value["steps"]["archived-flag-backfill"]["error"] = json!("failed\n\u{001b}");
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(value))
        .mount(&server)
        .await;
    for args in [
        vec!["system", "schema-upgrade-status"],
        vec!["auth", "preflight", "login"],
        vec!["auth", "preflight", "register"],
        vec!["auth", "preflight", "logout"],
    ] {
        let mut argv = vec!["wekan"];
        argv.extend(args);
        let success = app
            .execute(Cli::try_parse_from(argv).unwrap().command)
            .await
            .unwrap();
        let json: Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &success)).unwrap();
        assert_eq!(json["ok"], true);
        let human = render_success(OutputFormat::Human, &success);
        assert!(!human.contains('\u{001b}'));
        if matches!(success, CommandSuccess::SchemaUpgradeStatus(_)) {
            assert_eq!(
                json["data"]["steps"]["archived-flag-backfill"]["error"],
                "failed\n\u{001b}"
            );
            assert!(human.contains("failed\\n\\u{1b}"));
        } else {
            assert_eq!(json["data"]["http_status"], 200);
            assert_eq!(json["data"]["server"], format!("{}/", server.uri()));
        }
        assert!(!directory.path().join("profiles.json").exists());
    }
}
