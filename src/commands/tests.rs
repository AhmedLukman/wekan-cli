//! Command dispatch and result behavior shared across resource families.

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
    cli::Cli,
    client::WekanClientFactory,
    credentials::{
        CredentialCreateOutcome, CredentialDeleteOutcome, CredentialError, CredentialRecord,
        CredentialStore, CredentialTarget,
    },
    error::ErrorCode,
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
                    boards::dispatch(a.command, &factory(&server), &store, &confirm).await
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
