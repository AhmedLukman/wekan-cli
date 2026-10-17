use super::admin_fixtures as fixtures;
use clap::Parser;
use secrecy::SecretString;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use time::{Duration, OffsetDateTime};
use wekan_cli::{
    app::App,
    config::{
        ServerSelection,
        profiles::{FileProfileStore, Profile, ProfileDocument, ProfileStore},
    },
    credentials::{
        CredentialCreateOutcome, CredentialDeleteOutcome, CredentialError, CredentialRecord,
        CredentialStore, CredentialTarget, LoginSecretMode, LoginSecrets, SecretInputFuture,
        SecretInputProvider, SecretSource, SystemSecretInputProvider,
    },
};
use wekan_cli::{
    cli::Cli,
    error::ErrorCode,
    output::{OutputFormat, render_success},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, method, path},
};

pub(super) async fn run(
    server: &MockServer,
    args: &[&str],
) -> Result<wekan_cli::command_result::CommandSuccess, wekan_cli::error::AppError> {
    run_with_secrets(server, args, SystemSecretInputProvider).await
}

async fn run_with_secrets(
    server: &MockServer,
    args: &[&str],
    secrets: impl SecretInputProvider,
) -> Result<wekan_cli::command_result::CommandSuccess, wekan_cli::error::AppError> {
    run_with_providers(
        server,
        args,
        secrets,
        wekan_cli::input::SystemConfirmationProvider::for_output(false),
    )
    .await
}

pub(super) async fn run_with_confirmation(
    server: &MockServer,
    args: &[&str],
    confirmation: impl wekan_cli::input::ConfirmationProvider,
) -> Result<wekan_cli::command_result::CommandSuccess, wekan_cli::error::AppError> {
    run_with_providers(server, args, SystemSecretInputProvider, confirmation).await
}

async fn run_with_providers(
    server: &MockServer,
    args: &[&str],
    secrets: impl SecretInputProvider,
    confirmation: impl wekan_cli::input::ConfirmationProvider,
) -> Result<wekan_cli::command_result::CommandSuccess, wekan_cli::error::AppError> {
    run_with_store_and_providers(
        server,
        args,
        ReadOnlyCredentialStore::authenticated(server),
        secrets,
        confirmation,
    )
    .await
}

pub(super) async fn run_with_store_and_providers(
    server: &MockServer,
    args: &[&str],
    store: impl CredentialStore,
    secrets: impl SecretInputProvider,
    confirmation: impl wekan_cli::input::ConfirmationProvider,
) -> Result<wekan_cli::command_result::CommandSuccess, wekan_cli::error::AppError> {
    let mut argv = vec!["wekan", "admin"];
    argv.extend_from_slice(args);
    let directory = tempfile::tempdir().unwrap();
    let profiles = FileProfileStore::at(directory.path().to_owned());
    {
        let mut mutation = profiles.lock_mutation().unwrap();
        let mut document = ProfileDocument::default();
        document.insert("default".into(), Profile::new(format!("{}/", server.uri())));
        mutation.save(document).unwrap();
    }
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.uri()), Some("default".into()), false),
        store,
        secrets,
        profiles,
    );
    app.with_confirmation_provider(confirmation)
        .execute(Cli::try_parse_from(argv).unwrap().command)
        .await
}
pub(super) fn json_data(result: &wekan_cli::command_result::CommandSuccess) -> Value {
    serde_json::from_str::<Value>(&render_success(OutputFormat::Json, result)).unwrap()["data"]
        .clone()
}

struct InjectedStorageSecrets {
    reads: Arc<Mutex<Vec<(SecretSource, usize)>>>,
}

impl SecretInputProvider for InjectedStorageSecrets {
    fn read_storage_secret(&self, source: SecretSource, limit: usize) -> SecretInputFuture<'_> {
        self.reads.lock().unwrap().push((source, limit));
        Box::pin(async { Ok(SecretString::from("injected-storage-secret")) })
    }

    fn read_new_account_password(
        &self,
        _: bool,
    ) -> Result<SecretString, wekan_cli::error::AppError> {
        panic!("settings must not acquire an account password")
    }

    fn read_login_secrets(
        &self,
        _: LoginSecretMode,
    ) -> Result<LoginSecrets, wekan_cli::error::AppError> {
        panic!("settings must not acquire login secrets")
    }
}

#[test]
fn coverage_rows_list_every_parser_option() {
    use clap::CommandFactory;
    let coverage = include_str!("../../docs/api-coverage.md");
    let mut root = Cli::command();
    root.build();
    for (family, operation) in [
        ("settings", "global_settings"),
        ("attachment-settings", "attachment_settings"),
    ] {
        for verb in ["get", "update"] {
            let row = coverage
                .lines()
                .find(|line| line.contains(&format!("| `{verb}_{operation}` |")))
                .unwrap();
            let leaf = root
                .find_subcommand("admin")
                .unwrap()
                .find_subcommand(family)
                .unwrap()
                .find_subcommand(verb)
                .unwrap();
            assert!(row.contains(&format!("wekan admin {family} {verb}")));
            assert!(row.ends_with("| ✓ |"));
            for arg in leaf.get_arguments() {
                if let Some(long) = arg.get_long() {
                    assert!(
                        row.contains(&format!("--{long}")),
                        "missing {long} in {family} {verb}"
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn storage_secret_sources_use_the_injected_provider_for_every_credential_flag() {
    for (flag, provider, wire) in [
        ("s3-secret-access-key", "s3", "secretAccessKey"),
        ("azure-account-key", "azure", "accountKey"),
        ("azure-connection-string", "azure", "connectionString"),
        ("gcs-credentials", "gcs", "credentials"),
    ] {
        for mode in ["file", "stdin"] {
            let server = MockServer::start().await;
            let mut expected = json!({"storageConfig":{}});
            expected["storageConfig"][provider] = json!({});
            expected["storageConfig"][provider][wire] = json!("injected-storage-secret");
            Mock::given(method("PUT"))
                .and(path("/api/admin/attachment-settings"))
                .and(body_json(expected))
                .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::attachment()))
                .expect(1)
                .mount(&server)
                .await;
            let option = format!("--{flag}-{mode}");
            let mut args = vec!["attachment-settings", "update", option.as_str()];
            let source = if mode == "file" {
                // This path is intentionally not created; only the provider may read it.
                args.push("provider-only-credential.txt");
                SecretSource::File("provider-only-credential.txt".into())
            } else {
                SecretSource::Stdin
            };
            let reads = Arc::new(Mutex::new(Vec::new()));
            let result = run_with_secrets(
                &server,
                &args,
                InjectedStorageSecrets {
                    reads: reads.clone(),
                },
            )
            .await
            .unwrap();
            assert_eq!(
                *reads.lock().unwrap(),
                vec![(source, wekan_cli::client::admin::MAX_SETTINGS_BYTES)]
            );
            assert_eq!(
                json_data(&result)["submitted_fields"],
                json!(["storage_config"])
            );
            assert!(
                !render_success(OutputFormat::Json, &result).contains("injected-storage-secret")
            );
        }
    }
}

#[tokio::test]
async fn every_global_flag_maps_to_native_json_and_complete_stable_output() {
    for boolean in [false, true] {
        let server = MockServer::start().await;
        let mut expected = json!({});
        let mut args = vec!["settings".to_owned(), "update".into()];
        for (wire, _, flag, is_bool) in fixtures::GLOBAL_FIELDS {
            let value = if *is_bool {
                json!(boolean)
            } else {
                json!("value\u{1b}[31m")
            };
            expected[*wire] = value.clone();
            args.extend([
                (*flag).into(),
                if *is_bool {
                    boolean.to_string()
                } else {
                    "value\u{1b}[31m".into()
                },
            ]);
        }
        let mut response = expected.clone();
        response["_id"] = json!("global-settings");
        Mock::given(method("PUT"))
            .and(path("/api/settings"))
            .and(body_json(expected))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        let result = run(
            &server,
            &args.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .await
        .unwrap();
        let data = json_data(&result);
        assert_eq!(
            data["submitted_fields"],
            json!(
                fixtures::GLOBAL_FIELDS
                    .iter()
                    .map(|(_, s, _, _)| *s)
                    .collect::<Vec<_>>()
            )
        );
        let human = render_success(OutputFormat::Human, &result);
        for (_, snake, _, is_bool) in fixtures::GLOBAL_FIELDS {
            assert_eq!(
                data["settings"][*snake],
                if *is_bool {
                    json!(boolean)
                } else {
                    json!("value\u{1b}[31m")
                }
            );
            assert!(human.contains(snake));
        }
        assert!(!human.contains('\u{1b}'));
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn every_attachment_flag_and_multiple_secret_files_send_one_typed_request() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let mut expected = fixtures::attachment();
    for key in ["_id", "createdAt", "updatedAt", "createdBy", "updatedBy"] {
        expected.as_object_mut().unwrap().remove(key);
    }
    for (provider, keys) in [
        ("s3", vec!["secretAccessKey"]),
        ("azure", vec!["accountKey", "connectionString"]),
        ("gcs", vec!["credentials"]),
    ] {
        for key in keys {
            expected["storageConfig"][provider][key] = json!("replacement-secret");
            expected["storageConfig"][provider]
                .as_object_mut()
                .unwrap()
                .remove(&format!("{key}Set"));
        }
    }
    let mut args = vec![
        "attachment-settings".to_owned(),
        "update".into(),
        "--default-storage".into(),
        "fs".into(),
    ];
    for (wire, _, flag, kind) in fixtures::ATTACHMENT_FIELDS {
        let value = expected.pointer(wire).unwrap();
        args.extend([
            (*flag).into(),
            if *kind == "Vec<String>" {
                value[0].as_str().unwrap().into()
            } else if *kind == "String" {
                value.as_str().unwrap().into()
            } else {
                value.to_string()
            },
        ]);
    }
    for field in [
        "s3-secret-access-key",
        "azure-account-key",
        "azure-connection-string",
        "gcs-credentials",
    ] {
        let file = dir.path().join(field);
        std::fs::write(&file, "replacement-secret\r\n").unwrap();
        args.extend([format!("--{field}-file"), file.to_str().unwrap().into()]);
    }
    Mock::given(method("PUT"))
        .and(path("/api/admin/attachment-settings"))
        .and(body_json(expected))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::attachment()))
        .expect(1)
        .mount(&server)
        .await;
    let result = run(
        &server,
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
    )
    .await
    .unwrap();
    let data = json_data(&result);
    assert_eq!(
        data["submitted_fields"],
        json!([
            "default_storage",
            "storage_config",
            "upload_settings",
            "limit_settings",
            "migration_settings"
        ])
    );
    let human = render_success(OutputFormat::Human, &result);
    for (_, stable, _, _) in fixtures::ATTACHMENT_FIELDS {
        assert!(data["settings"].pointer(stable).is_some(), "{stable}");
    }
    assert!(!human.contains("replacement-secret"));
    assert!(
        data["settings"]["storage_config"]["s3"]
            .get("secret_access_key")
            .is_none()
    );
}

#[tokio::test]
async fn json_files_are_strict_and_fail_before_network_requests() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("settings.json");
    for (family, body) in [
        ("settings", "{}"),
        ("settings", "[]"),
        ("settings", "{broken-secret"),
        ("settings", "{\"productName\":true}"),
        ("settings", "{\"productName\":null}"),
        (
            "settings",
            "{\"productName\":\"one\",\"productName\":\"two\"}",
        ),
        ("settings", "{\"mailServer\":{\"password\":\"secret\"}}"),
        ("settings", "{\"_id\":\"readonly\"}"),
        ("attachment-settings", "{\"createdAt\":\"readonly\"}"),
        ("attachment-settings", "{\"storageConfig\":{\"future\":{}}}"),
        (
            "attachment-settings",
            "{\"storageConfig\":{\"s3\":{\"future\":true}}}",
        ),
        (
            "attachment-settings",
            "{\"storageConfig\":{\"s3\":{\"secretAccessKeySet\":true}}}",
        ),
        ("attachment-settings", "{\"storageConfig\":[]}"),
        (
            "attachment-settings",
            "{\"limitSettings\":{\"apiUploadMaxBytes\":-1}}",
        ),
        (
            "attachment-settings",
            "{\"migrationSettings\":{\"batchSize\":101}}",
        ),
        (
            "attachment-settings",
            "{\"migrationSettings\":{\"delayMs\":99}}",
        ),
        (
            "attachment-settings",
            "{\"migrationSettings\":{\"cpuThreshold\":91}}",
        ),
        ("attachment-settings", "{\"defaultStorage\":\"unknown\"}"),
    ] {
        std::fs::write(&file, body).unwrap();
        let error = run(
            &server,
            &[family, "update", "--settings-file", file.to_str().unwrap()],
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidInput, "{body}: {error:?}");
        assert!(!error.message().contains("broken-secret"));
    }
    std::fs::write(&file, [0xff]).unwrap();
    assert_eq!(
        run(
            &server,
            &[
                "settings",
                "update",
                "--settings-file",
                file.to_str().unwrap()
            ]
        )
        .await
        .unwrap_err()
        .code(),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        run(
            &server,
            &[
                "settings",
                "update",
                "--settings-file",
                dir.path().to_str().unwrap()
            ]
        )
        .await
        .unwrap_err()
        .code(),
        ErrorCode::InvalidInput
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn failed_mutations_preserve_outcome_metadata_and_redact_credential_values() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("settings.json");
    std::fs::write(&file,r#"{"storageConfig":{"s3":{"secretAccessKey":"secret-input"},"gcs":{"credentials":{"private_key":"private-key-value"}}}}"#).unwrap();
    Mock::given(method("PUT")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"error":"validation-error","reason":"secret-input private-key-value","message":"private-key-value","errorType":"Meteor.Error","isClientSafe":true}))).expect(1).mount(&server).await;
    let error = run(
        &server,
        &[
            "attachment-settings",
            "update",
            "--settings-file",
            file.to_str().unwrap(),
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ProtocolError);
    assert_eq!(error.details().http_status, Some(200));
    assert_eq!(error.details().outcome_unknown, Some(true));
    let output = wekan_cli::output::render_error(OutputFormat::Json, &error);
    assert!(!output.contains("secret-input"));
    assert!(!output.contains("private-key-value"));
}

#[tokio::test]
async fn empty_mime_list_and_nested_json_objects_preserve_native_requests() {
    for body in [
        json!({"uploadSettings":{"allowedMimeTypes":[]}}),
        json!({"migrationSettings":{}}),
        json!({"storageConfig":{"s3":{"secretAccessKey":""}}}),
    ] {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        std::fs::write(&file, body.to_string()).unwrap();
        Mock::given(method("PUT"))
            .and(body_json(&body))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::attachment()))
            .expect(1)
            .mount(&server)
            .await;
        run(
            &server,
            &[
                "attachment-settings",
                "update",
                "--settings-file",
                file.to_str().unwrap(),
            ],
        )
        .await
        .unwrap();
    }
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(body_json(json!({"uploadSettings":{"allowedMimeTypes":[]}})))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::attachment()))
        .expect(1)
        .mount(&server)
        .await;
    run(
        &server,
        &[
            "attachment-settings",
            "update",
            "--clear-upload-allowed-mime-types",
        ],
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn reads_render_all_mapped_fields() {
    for (family, route, response) in [
        ("settings", "/api/settings", fixtures::global()),
        (
            "attachment-settings",
            "/api/admin/attachment-settings",
            fixtures::attachment(),
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        let result = run(&server, &[family, "get"]).await.unwrap();
        assert!(json_data(&result)["settings"].is_object());
        assert!(render_success(OutputFormat::Human, &result).contains("settings_id"));
    }
}

struct ReadOnlyCredentialStore {
    record: CredentialRecord,
}

impl ReadOnlyCredentialStore {
    fn authenticated(server: &MockServer) -> Self {
        Self {
            record: CredentialRecord::new(
                format!("{}/", server.uri()),
                "user-1".to_owned(),
                SecretString::from("card-token".to_owned()),
                OffsetDateTime::now_utc() + Duration::hours(1),
            ),
        }
    }
}

impl CredentialStore for ReadOnlyCredentialStore {
    fn check_available(&self, _target: &CredentialTarget) -> Result<(), CredentialError> {
        Ok(())
    }

    fn exists(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        Ok(true)
    }

    fn load(
        &self,
        _target: &CredentialTarget,
    ) -> Result<Option<CredentialRecord>, CredentialError> {
        Ok(Some(self.record.clone()))
    }

    fn create(
        &self,
        _target: &CredentialTarget,
        _record: &CredentialRecord,
    ) -> Result<CredentialCreateOutcome, CredentialError> {
        panic!("settings commands must not store credentials")
    }

    fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("settings commands must not delete credentials")
    }

    fn delete_if_matches(
        &self,
        _target: &CredentialTarget,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("settings commands must not delete credentials")
    }
}
