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
    matchers::{body_json, method, path},
};

use crate::{
    cli::Cli,
    client::WekanClientFactory,
    command_result::{CommandSuccess, CustomFieldSubmittedField, DestructiveOperation},
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
                SecretString::from("custom-field-token".to_owned()),
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
        panic!("custom-field commands must not store credentials")
    }

    fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("custom-field commands must not delete credentials")
    }

    fn delete_if_matches(
        &self,
        _target: &CredentialTarget,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("custom-field commands must not delete credentials")
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
        panic!("custom-field commands must not store credentials through a mutation guard")
    }

    fn delete(&self) -> Result<bool, CredentialError> {
        panic!("custom-field commands must not delete credentials through a mutation guard")
    }

    fn delete_if_matches(
        &self,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("custom-field commands must not delete credentials through a mutation guard")
    }
}

fn factory(server: &MockServer) -> WekanClientFactory {
    WekanClientFactory::for_profile(format!("{}/", server.uri()), "default".to_owned(), false)
}

fn command(family: &str, arguments: &[&str]) -> RootCommand {
    let mut argv = vec!["wekan", "--server", "https://wekan.example", family];
    argv.extend_from_slice(arguments);
    Cli::try_parse_from(argv).unwrap().command
}
async fn dispatch(
    command: RootCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        RootCommand::CustomField(args) => {
            super::dispatch(args.command, factory, store, confirmation).await
        }
        RootCommand::CustomFieldItem(args) => {
            crate::commands::custom_field_items::dispatch(
                args.command,
                factory,
                store,
                confirmation,
            )
            .await
        }
        RootCommand::Card(args) => {
            crate::commands::cards::dispatch(args.command, factory, store, confirmation).await
        }
        _ => panic!("expected custom-field command"),
    }
}

fn deletion(family: &str, yes: bool) -> RootCommand {
    let mut args = vec![
        "delete",
        if family == "custom-field" {
            "field-1"
        } else {
            "item-1"
        },
        "--board",
        "board-1",
    ];
    if family == "custom-field-item" {
        args.extend(["--custom-field", "field-1"]);
    }
    if yes {
        args.push("--yes");
    }
    command(family, &args)
}

#[tokio::test]
async fn deletions_bind_confirmation_to_credentials_and_validate_parent_acknowledgements() {
    for (family, url, operation) in [
        (
            "custom-field",
            "/api/boards/board-1/custom-fields/field-1",
            DestructiveOperation::CustomFieldDelete,
        ),
        (
            "custom-field-item",
            "/api/boards/board-1/custom-fields/field-1/dropdown-items/item-1",
            DestructiveOperation::CustomFieldItemDelete,
        ),
    ] {
        let server = MockServer::start().await;
        let store = FakeCredentialStore::authenticated(&server);
        let confirmation = FakeConfirmationProvider::declining();
        let result = dispatch(
            deletion(family, false),
            &factory(&server),
            &store,
            &confirmation,
        )
        .await
        .unwrap();
        let CommandSuccess::Cancelled(result) = result else {
            panic!("expected cancellation")
        };
        assert_eq!(result.operation, operation);
        let requests = confirmation.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].operation(), operation);
        assert!(requests[0].prompt().contains("all associated boards"));
        assert!(requests[0].prompt().contains("requested board `board-1`"));
        assert!(requests[0].prompt().contains("custom field `field-1`"));
        if family == "custom-field-item" {
            assert!(requests[0].prompt().contains("dropdown item `item-1`"));
        }
        assert_eq!(store.lock_count.load(Ordering::SeqCst), 1);
        assert!(server.received_requests().await.unwrap().is_empty());
        let error = dispatch(
            deletion(family, false),
            &factory(&server),
            &store,
            &crate::input::SystemConfirmationProvider::for_output(false),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidInput);
        store.replace_on_next_lock(CredentialRecord::new(
            format!("{}/", server.uri()),
            "user-2".to_owned(),
            SecretString::from("changed".to_owned()),
            OffsetDateTime::now_utc() + Duration::hours(1),
        ));
        assert_eq!(
            dispatch(
                deletion(family, true),
                &factory(&server),
                &store,
                &FakeConfirmationProvider::accepting()
            )
            .await
            .unwrap_err()
            .code(),
            ErrorCode::CredentialStoreFailed
        );
        assert!(server.received_requests().await.unwrap().is_empty());
        for body in [
            json!({"_id":"field-1"}),
            json!({"_id":"item-1"}),
            json!({"_id":"field-1","future":true}),
        ] {
            server.reset().await;
            Mock::given(method("DELETE"))
                .and(path(url))
                .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
                .expect(1)
                .mount(&server)
                .await;
            let result = dispatch(
                deletion(family, true),
                &factory(&server),
                &store,
                &FakeConfirmationProvider::declining(),
            )
            .await;
            if body == json!({"_id":"field-1"}) {
                let rendered = crate::output::render_success(
                    crate::output::OutputFormat::Json,
                    &result.unwrap(),
                );
                let data: serde_json::Value = serde_json::from_str(&rendered).unwrap();
                let mut expected = json!({
                    "board_id": "board-1",
                    "custom_field_id": "field-1",
                    "deletion_requested": true,
                    "mutation_confirmed": false,
                    "delete_mode": "hard",
                });
                if family == "custom-field-item" {
                    expected["item_id"] = json!("item-1");
                }
                assert_eq!(data, json!({"ok":true,"data":expected}));
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.code(), ErrorCode::ProtocolError);
                assert_eq!(error.details().outcome_unknown, Some(true));
            }
        }
    }
}

#[tokio::test]
async fn empty_definition_response_maps_not_found_without_inventing_server_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/custom-fields/field-1"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let error = dispatch(
        command("custom-field", &["get", "field-1", "--board", "board-1"]),
        &factory(&server),
        &FakeCredentialStore::authenticated(&server),
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::NotFound);
    assert_eq!(error.details().http_status, Some(200));
    assert!(error.details().wekan_status_code.is_none());
    assert!(error.details().server_error.is_none());
    assert!(error.details().server_reason.is_none());
}

#[tokio::test]
async fn commands_preserve_inputs_and_report_submitted_fields_and_unknown_outcomes() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    Mock::given(method("PUT"))
        .and(path("/api/boards/board-1/custom-fields/field-1"))
        .and(body_json(
            json!({"name":"  Name  ","settings":{},"alwaysOnCard":false}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"field-1"})))
        .expect(1)
        .mount(&server)
        .await;
    let result = dispatch(
        command(
            "custom-field",
            &[
                "update",
                " field-1 ",
                "--board",
                " board-1 ",
                "--always-on-card",
                "false",
                "--settings",
                "{}",
                "--name",
                "  Name  ",
            ],
        ),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();
    let CommandSuccess::CustomFieldUpdated(result) = result else {
        panic!("expected update")
    };
    assert_eq!(
        result.submitted_fields,
        vec![
            CustomFieldSubmittedField::Name,
            CustomFieldSubmittedField::Settings,
            CustomFieldSubmittedField::AlwaysOnCard
        ]
    );
    for value in [Some("null"), Some(r#"{"arbitrary":true}"#), None] {
        server.reset().await;
        let mut args = vec![
            "set-custom-field",
            "card-1",
            "--board",
            "board-1",
            "--list",
            "list-1",
            "--custom-field",
            "field-1",
        ];
        let mut body = json!({});
        if let Some(value) = value {
            args.extend(["--value-json", value]);
            body["value"] = serde_json::from_str(value).unwrap();
        }
        Mock::given(method("POST"))
            .and(body_json(body))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"_id":"card-1","customFields":[{"value":{"unmapped":true}}]}),
            ))
            .expect(1)
            .mount(&server)
            .await;
        let error = dispatch(
            command("card", &args),
            &factory(&server),
            &store,
            &FakeConfirmationProvider::declining(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::ProtocolError);
        assert_eq!(error.details().outcome_unknown, Some(true));
    }
    for status in [401, 403, 404, 500] {
        server.reset().await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(status))
            .expect(1)
            .mount(&server)
            .await;
        let error = dispatch(
            command("custom-field", &["update", "field-1", "--board", "board-1"]),
            &factory(&server),
            &store,
            &FakeConfirmationProvider::declining(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.details().http_status, Some(status));
        assert_eq!(
            error.details().outcome_unknown,
            if status == 500 { Some(true) } else { None }
        );
    }
}

#[tokio::test]
async fn detail_maps_every_setting_and_escapes_nested_terminal_controls() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"field-1","boardIds":["board-1"],"name":"Name\n\u{1b}","type":"dropdown","settings":{"currencyCode":"USD","dropdownItems":[{"_id":"item-1","name":"N\u{1b}"}],"stringtemplateFormat":"x","stringtemplateSeparator":" "},"showOnCard":false,"automaticallyOnCard":true,"alwaysOnCard":false,"showLabelOnMiniCard":true,"showSumAtTopOfList":false,"modifiedAt":"2030-01-01T00:00:00Z"}))).mount(&server).await;
    let result = dispatch(
        command("custom-field", &["get", "field-1", "--board", "board-1"]),
        &factory(&server),
        &FakeCredentialStore::authenticated(&server),
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();
    let data: serde_json::Value = serde_json::from_str(&crate::output::render_success(
        crate::output::OutputFormat::Json,
        &result,
    ))
    .unwrap();
    assert_eq!(data["data"]["field_type"], "dropdown");
    assert_eq!(
        data["data"]["settings"]["dropdown_items"][0]["item_id"],
        "item-1"
    );
    assert_eq!(data["data"]["created_at"], serde_json::Value::Null);
    let human = crate::output::render_success(crate::output::OutputFormat::Human, &result);
    assert!(!human.contains('\u{1b}'));
    assert!(human.contains("\\u{1b}"));
    assert!(human.contains("currency_code"));
}
