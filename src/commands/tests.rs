//! Command dispatch and result behavior shared across resource families.

use std::sync::{Arc, Mutex};

use clap::Parser;
use secrecy::SecretString;
use serde_json::json;
use time::{Duration, OffsetDateTime};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, method, path},
};

use super::{RootCommand, boards, cards, lists, swimlanes};
use crate::{
    app::App,
    cli::Cli,
    client::WekanClientFactory,
    config::{
        ServerSelection,
        profiles::{FileProfileStore, Profile, ProfileDocument, ProfileStore},
    },
    credentials::{
        CredentialCreateOutcome, CredentialDeleteOutcome, CredentialError, CredentialRecord,
        CredentialStore, CredentialTarget, LoginSecretMode, LoginSecrets, SecretInputFuture,
        SecretInputProvider, SecretSource,
    },
    error::{AppError, ErrorCode},
    input::FakeConfirmationProvider,
};

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
        panic!("transfer commands must not store credentials")
    }

    fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("transfer commands must not delete credentials")
    }

    fn delete_if_matches(
        &self,
        _target: &CredentialTarget,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("transfer commands must not delete credentials")
    }
}

fn factory(server: &MockServer) -> WekanClientFactory {
    WekanClientFactory::for_profile(format!("{}/", server.uri()), "default".to_owned(), false)
}

struct FileSecrets {
    input: Option<&'static str>,
    reads: Arc<Mutex<Vec<(SecretSource, usize)>>>,
}

impl SecretInputProvider for FileSecrets {
    fn read_storage_secret(&self, source: SecretSource, limit: usize) -> SecretInputFuture<'_> {
        self.reads.lock().unwrap().push((source, limit));
        Box::pin(async {
            self.input
                .map(SecretString::from)
                .ok_or_else(|| AppError::invalid_input("injected credential input unavailable"))
        })
    }

    fn read_new_account_password(&self, _: bool) -> Result<SecretString, AppError> {
        panic!("file credentials must not acquire account passwords")
    }

    fn read_login_secrets(&self, _: LoginSecretMode) -> Result<LoginSecrets, AppError> {
        panic!("file credentials must not acquire login secrets")
    }
}

#[tokio::test]
async fn credential_files_use_app_secret_input_and_stop_on_input_failures() {
    const PAYLOAD: &[u8] = b"{\"source\":\"unchanged\"}\r\n";
    for (args, flag, input, route, response) in [
        (
            vec!["board", "import"],
            "--attachment-auth-file",
            r#"{"token":"file-secret"}"#,
            "/api/boards/import",
            json!({"_id":"new-board"}),
        ),
        (
            vec!["integration", "inbound-email", "submit", "--file"],
            "--headers-file",
            r#"{"X-Provider-Signature":"file-secret"}"#,
            "/api/inbound-email",
            json!({"_id":"new-card"}),
        ),
        (
            vec![
                "integration",
                "inbound-email",
                "bounce",
                "postmark",
                "--file",
            ],
            "--headers-file",
            r#"{"X-Provider-Signature":"file-secret"}"#,
            "/api/inbound-email/bounces/postmark",
            json!({"ok":true,"recorded":1}),
        ),
    ] {
        for supplied in [Some(input), Some("{"), None] {
            let server = MockServer::start().await;
            let directory = tempfile::tempdir().unwrap();
            let payload = directory.path().join("payload.json");
            std::fs::write(&payload, PAYLOAD).unwrap();
            // Only the injected provider can supply this nonexistent file.
            let credential_file = directory.path().join("credentials.json");
            let profiles = FileProfileStore::at(directory.path().join("profiles"));
            let mut document = ProfileDocument::default();
            document.insert("default".into(), Profile::new(format!("{}/", server.uri())));
            profiles.lock_mutation().unwrap().save(document).unwrap();
            let reads = Arc::new(Mutex::new(Vec::new()));
            let app = App::with_profile_store(
                ServerSelection::new(Some(server.uri()), Some("default".into()), false),
                ReadOnlyCredentialStore::authenticated(&server),
                FileSecrets {
                    input: supplied,
                    reads: Arc::clone(&reads),
                },
                profiles,
            );
            Mock::given(method("POST"))
                .and(path(route))
                .respond_with(ResponseTemplate::new(200).set_body_json(response.clone()))
                .expect(u64::from(supplied == Some(input)))
                .mount(&server)
                .await;
            let mut argv = vec!["wekan"];
            argv.extend_from_slice(&args);
            argv.extend([
                payload.to_str().unwrap(),
                flag,
                credential_file.to_str().unwrap(),
            ]);
            let result = app
                .execute(Cli::try_parse_from(argv).unwrap().command)
                .await;
            assert_eq!(
                *reads.lock().unwrap(),
                vec![(
                    SecretSource::File(credential_file),
                    super::imports::MAX_JSON_BYTES
                )]
            );
            let requests = server.received_requests().await.unwrap();
            if supplied == Some(input) {
                let success = result.unwrap();
                assert_eq!(requests.len(), 1);
                let request = &requests[0];
                if flag == "--attachment-auth-file" {
                    assert_eq!(
                        serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
                        json!({"board":{"source":"unchanged"},"attachmentAuth":{"token":"file-secret"}})
                    );
                    assert_eq!(request.headers["authorization"], "Bearer card-token");
                } else {
                    assert_eq!(request.body, PAYLOAD);
                    assert_eq!(request.headers["x-provider-signature"], "file-secret");
                    assert!(!request.headers.contains_key("authorization"));
                }
                assert!(
                    !crate::output::render_success(crate::output::OutputFormat::Json, &success)
                        .contains("file-secret")
                );
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.code(), ErrorCode::InvalidInput);
                if supplied.is_none() {
                    assert_eq!(error.message(), "injected credential input unavailable");
                }
                assert!(requests.is_empty());
            }
            server.verify().await;
        }
    }
}

#[tokio::test]
async fn copy_move_commands_send_one_request_and_render_completed_actions() {
    use crate::output::{OutputFormat, render_success};
    for (family, action, id, route) in [
        ("board", "copy", "b", "/api/boards/b/copy"),
        ("card", "copy", "c", "/api/boards/b/lists/l/cards/c/copy"),
        ("list", "copy", "l", "/api/boards/b/lists/l/copy"),
        ("list", "move", "l", "/api/boards/b/lists/l/move"),
        ("swimlane", "copy", "s", "/api/boards/b/swimlanes/s/copy"),
        ("swimlane", "move", "s", "/api/boards/b/swimlanes/s/move"),
    ] {
        for status in [200, 403, 500] {
            let server = MockServer::start().await;
            let store = ReadOnlyCredentialStore::authenticated(&server);
            let confirm = FakeConfirmationProvider::declining();
            let body = if family == "board" {
                json!("new")
            } else {
                json!({"_id":"new"})
            };
            let request = match family {
                "board" => json!({"title":""}),
                "card" => {
                    json!({"toBoardId":"target","toListId":"tl","toSwimlaneId":"ts","position":null})
                }
                "list" => json!({"toBoardId":"target","toSwimlaneId":"ts","position":false}),
                _ => json!({"toBoardId":"target","title":" renamed ","position":false}),
            };
            Mock::given(method("POST"))
                .and(path(route))
                .and(body_json(request))
                .respond_with(if status == 200 {
                    ResponseTemplate::new(status).set_body_json(body)
                } else {
                    ResponseTemplate::new(status).set_body_string("<!DOCTYPE html>secret-value")
                })
                .expect(1)
                .mount(&server)
                .await;
            let mut args = vec!["wekan", family, action, id];
            if family == "board" {
                args.extend(["--title", ""]);
            } else {
                args.extend([
                    "--board",
                    "b",
                    "--target-board",
                    "target",
                    "--position-json",
                    if family == "card" { "null" } else { "false" },
                ]);
            }
            match family {
                "card" => args.extend([
                    "--list",
                    "l",
                    "--target-list",
                    "tl",
                    "--target-swimlane-id",
                    "ts",
                ]),
                "list" => args.extend(["--target-swimlane-id", "ts"]),
                "swimlane" => args.extend(["--title", " renamed "]),
                _ => (),
            }
            let command = Cli::try_parse_from(args).unwrap().command;
            let result = match command {
                RootCommand::Board(a) => {
                    boards::dispatch(
                        a.command,
                        &factory(&server),
                        &store,
                        &crate::credentials::SystemSecretInputProvider,
                        &confirm,
                    )
                    .await
                }
                RootCommand::Card(a) => {
                    cards::dispatch(a.command, &factory(&server), &store, &confirm).await
                }
                RootCommand::List(a) => {
                    lists::dispatch(a.command, &factory(&server), &store, &confirm).await
                }
                RootCommand::Swimlane(a) => {
                    swimlanes::dispatch(a.command, &factory(&server), &store, &confirm).await
                }
                _ => unreachable!(),
            };
            if status == 200 {
                let result = result.unwrap();
                let value: serde_json::Value =
                    serde_json::from_str(&render_success(OutputFormat::Json, &result)).unwrap();
                let expected = match family {
                    "board" => json!({
                        "source_board_id":"b", "board_id":"new"
                    }),
                    "card" => json!({
                        "source_board_id":"b", "source_list_id":"l", "source_card_id":"c",
                        "target_board_id":"target", "target_list_id":"tl",
                        "target_swimlane_id":"ts", "card_id":"new"
                    }),
                    "list" => json!({
                        "source_board_id":"b", "source_list_id":"l", "target_board_id":"target",
                        "target_swimlane_id":"ts", "returned_list_id":"new"
                    }),
                    "swimlane" => json!({
                        "source_board_id":"b", "source_swimlane_id":"s", "target_board_id":"target",
                        "returned_swimlane_id":"new"
                    }),
                    _ => unreachable!(),
                };
                assert_eq!(value["ok"], true);
                assert_eq!(value["data"], expected);
                assert!(value.get("warnings").is_none());
                let completed = if action == "copy" { "copied" } else { "moved" };
                let heading = if family == "list" && action == "move" {
                    "cross-board list move request accepted (source list retained; position ignored):".to_owned()
                } else {
                    format!("{family} {completed}:")
                };
                assert!(
                    render_success(OutputFormat::Human, &result)
                        .to_lowercase()
                        .starts_with(&heading)
                );
            } else {
                let error = result.unwrap_err();
                assert!(
                    serde_json::to_value(error.details())
                        .unwrap()
                        .get("outcome_unknown")
                        .is_none()
                );
                assert_eq!(
                    error.code(),
                    if status == 403 {
                        ErrorCode::PermissionDenied
                    } else {
                        ErrorCode::ServerError
                    }
                );
                assert!(!format!("{error:?}").contains("secret-value"));
            }
        }
    }
}
