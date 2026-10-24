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
    output::{OutputFormat, write_success},
};
use clap::Parser;
use secrecy::SecretString;
use serde_json::{Value, json};
use std::sync::Mutex;
use time::{Duration, OffsetDateTime};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, method, path},
};

struct Store {
    record: Mutex<Option<CredentialRecord>>,
    change_on_load: bool,
    loads: std::sync::atomic::AtomicUsize,
}
impl Store {
    fn new(server: &MockServer) -> Self {
        Self {
            record: Mutex::new(Some(CredentialRecord::new(
                format!("{}/", server.uri()),
                "user".into(),
                SecretString::from("attachment-token".to_owned()),
                OffsetDateTime::now_utc() + Duration::hours(1),
            ))),
            change_on_load: false,
            loads: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}
impl CredentialStore for Store {
    fn check_available(&self, _: &CredentialTarget) -> Result<(), CredentialError> {
        Ok(())
    }
    fn exists(&self, _: &CredentialTarget) -> Result<bool, CredentialError> {
        Ok(true)
    }
    fn load(&self, _: &CredentialTarget) -> Result<Option<CredentialRecord>, CredentialError> {
        if self.change_on_load && self.loads.fetch_add(1, std::sync::atomic::Ordering::SeqCst) > 0 {
            let previous = self.record.lock().unwrap().clone().unwrap();
            return Ok(Some(CredentialRecord::new(
                previous.server_url().into(),
                "different-user".into(),
                SecretString::from("different-token".to_owned()),
                previous.token_expires(),
            )));
        }
        Ok(self.record.lock().unwrap().clone())
    }
    fn create(
        &self,
        _: &CredentialTarget,
        _: &CredentialRecord,
    ) -> Result<CredentialCreateOutcome, CredentialError> {
        panic!("attachment commands must not change credentials")
    }
    fn delete(&self, _: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("attachment commands must not change credentials")
    }
    fn delete_if_matches(
        &self,
        _: &CredentialTarget,
        _: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("attachment commands must not change credentials")
    }
}
async fn run(
    server: &MockServer,
    store: &Store,
    args: &[&str],
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "attachment"];
    argv.extend(args);
    let RootCommand::Attachment(a) = Cli::try_parse_from(argv).unwrap().command else {
        unreachable!()
    };
    super::dispatch(
        a.command,
        &WekanClientFactory::for_profile(format!("{}/", server.uri()), "default".into(), false),
        store,
        confirmation,
    )
    .await
}
fn download_body() -> Value {
    json!({"success":true,"attachmentId":"a","fileName":"unsafe\nname","fileSize":3,"fileType":"application/octet-stream","storageBackend":"fs","base64Data":"AP8K"})
}

#[tokio::test]
async fn deletion_cancels_and_rejects_changed_credentials_without_http() {
    let server = MockServer::start().await;
    let confirmation = FakeConfirmationProvider::declining();
    let result = run(
        &server,
        &Store::new(&server),
        &["delete", "a"],
        &confirmation,
    )
    .await
    .unwrap();
    let CommandSuccess::Cancelled(result) = result else {
        panic!("expected cancellation")
    };
    assert_eq!(result.operation, DestructiveOperation::AttachmentDelete);
    assert!(
        confirmation.requests()[0]
            .prompt()
            .contains("retained for restoration")
    );
    let mut store = Store::new(&server);
    store.change_on_load = true;
    let error = run(&server, &store, &["delete", "a", "--yes"], &confirmation)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
    assert!(server.received_requests().await.unwrap().is_empty());
}

struct ChangingConfirmation<'a> {
    store: &'a Store,
    replacement: Option<CredentialRecord>,
}

impl ConfirmationProvider for ChangingConfirmation<'_> {
    fn confirm(&self, _: &ConfirmationRequest) -> Result<bool, AppError> {
        *self.store.record.lock().unwrap() = self.replacement.clone();
        Ok(true)
    }
}

#[tokio::test]
async fn deletion_rechecks_replacement_and_removal_after_confirmation() {
    for removed in [false, true] {
        let server = MockServer::start().await;
        let store = Store::new(&server);
        let replacement = (!removed).then(|| {
            CredentialRecord::new(
                format!("{}/", server.uri()),
                "another-user".into(),
                SecretString::from("another-token".to_owned()),
                OffsetDateTime::now_utc() + Duration::hours(1),
            )
        });
        let confirmation = ChangingConfirmation {
            store: &store,
            replacement,
        };
        let error = run(&server, &store, &["delete", "a"], &confirmation)
            .await
            .unwrap_err();
        assert_eq!(
            error.code(),
            if removed {
                ErrorCode::CredentialNotFound
            } else {
                ErrorCode::CredentialStoreFailed
            }
        );
        assert_eq!(
            error.exit_code(),
            crate::exit_code::StableExitCode::Credential
        );
        assert!(error.message().contains("no HTTP request was sent"));
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn deletion_rechecks_expiry_after_confirmation() {
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
        SecretString::from("attachment-token".to_owned()),
        expires,
    ));
    let confirmation = ExpiringConfirmation {
        expires,
        called: std::sync::atomic::AtomicBool::new(false),
    };
    let error = run(&server, &store, &["delete", "a"], &confirmation)
        .await
        .unwrap_err();
    assert!(
        confirmation
            .called
            .load(std::sync::atomic::Ordering::SeqCst)
    );
    assert_eq!(error.code(), ErrorCode::CredentialExpired);
    assert_eq!(error.exit_code(), crate::exit_code::StableExitCode::Server);
    assert_eq!(
        error.details().token_expires.as_deref(),
        Some(
            expires
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap()
                .as_str()
        )
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn downloads_validate_before_saving_and_preserve_completed_file_context() {
    let server = MockServer::start().await;
    let store = Store::new(&server);
    let confirmation = FakeConfirmationProvider::declining();
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("attachment.bin");
    let destination = file.to_str().unwrap();
    std::fs::write(&file, b"old").unwrap();
    let mut invalid = download_body();
    invalid["future"] = json!(true);
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(invalid))
        .mount(&server)
        .await;
    assert_eq!(
        run(
            &server,
            &store,
            &["download", "a", "--destination", destination, "--overwrite"],
            &confirmation
        )
        .await
        .unwrap_err()
        .code(),
        ErrorCode::ProtocolError
    );
    assert_eq!(std::fs::read(&file).unwrap(), b"old");
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(download_body()))
        .mount(&server)
        .await;
    assert_eq!(
        run(
            &server,
            &store,
            &["download", "a", "--destination", destination],
            &confirmation
        )
        .await
        .unwrap_err()
        .code(),
        ErrorCode::FileIoError
    );
    let result = run(
        &server,
        &store,
        &["download", "a", "--destination", destination, "--overwrite"],
        &confirmation,
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), [0, 255, 10]);
    struct BrokenWriter;
    impl std::io::Write for BrokenWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("output failure"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let error = write_success(
        OutputFormat::Json,
        result,
        &mut BrokenWriter,
        &mut Vec::new(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.details().file_saved, Some(true));
    assert_eq!(error.details().destination.as_deref(), Some(destination));
    let result = run(
        &server,
        &store,
        &["download", "a", "--destination", "-"],
        &confirmation,
    )
    .await
    .unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    write_success(OutputFormat::Raw, result, &mut stdout, &mut stderr)
        .await
        .unwrap();
    assert_eq!(stdout, [0, 255, 10]);
    assert!(stderr.is_empty());
}

#[tokio::test]
async fn upload_preserves_binary_and_explicit_names_and_reports_acknowledgement() {
    let server = MockServer::start().await;
    let store = Store::new(&server);
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("source.bin");
    std::fs::write(&file, [0, 255, 10]).unwrap();
    Mock::given(method("POST")).and(path("/api/attachment/upload-background"))
        .and(body_json(json!({"boardId":"b","fileName":"  named.png  ","fileType":"image/png","fileData":"AP8K"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success":true,"attachmentId":"a","fileName":"named.png","fileSize":3,"storageBackend":"fs","message":"Uploaded","backgroundImageURL":"/background"}))).expect(1).mount(&server).await;
    let result = run(
        &server,
        &store,
        &[
            "upload-background",
            file.to_str().unwrap(),
            "--board",
            "b",
            "--file-name",
            "  named.png  ",
            "--file-type",
            "image/png",
        ],
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();
    let output: Value =
        serde_json::from_str(&crate::output::render_success(OutputFormat::Json, &result)).unwrap();
    assert!(output["data"].get("mutation_confirmed").is_none());
    assert_eq!(output["data"]["background_image_url"], "/background");
    server.reset().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(408)
                .set_body_json(json!({"success":false,"error":"Request timeout"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let error = run(
        &server,
        &store,
        &["upload-background", file.to_str().unwrap(), "--board", "b"],
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.details().http_status, Some(408));
    assert!(
        serde_json::to_value(error.details())
            .unwrap()
            .get("outcome_unknown")
            .is_none()
    );
}

#[tokio::test]
async fn scoped_listing_exposes_board_scope_without_filtering() {
    let server = MockServer::start().await;
    let store = Store::new(&server);
    Mock::given(method("GET")).and(path("/api/attachment/list/b/s/l/c"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success":true,"count":1,"attachments":[{"attachmentId":"other","fileName":"another","fileSize":0,"fileType":"text/plain","storageBackend":"fs","boardId":"b","cardId":"other-card"}]}))).mount(&server).await;
    let result = run(
        &server,
        &store,
        &[
            "list",
            "--board",
            "b",
            "--swimlane-id",
            "s",
            "--list",
            "l",
            "--card",
            "c",
        ],
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();
    let output: Value =
        serde_json::from_str(&crate::output::render_success(OutputFormat::Json, &result)).unwrap();
    assert_eq!(output["data"]["requested_scope"], "card");
    assert_eq!(output["data"]["effective_scope"], "board");
    assert_eq!(output["data"]["attachments"][0]["card_id"], "other-card");
    assert!(crate::output::render_success(OutputFormat::Human, &result).contains("ignores"));
}

#[tokio::test]
async fn upload_receipts_do_not_claim_completed_validation_or_storage() {
    let server = MockServer::start().await;
    let store = Store::new(&server);
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("upload.bin");
    std::fs::write(&file, [0, 255, 10]).unwrap();
    Mock::given(method("POST"))
        .and(path("/api/attachment/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success":true,"attachmentId":"a","fileName":"upload.bin",
            "fileSize":3,"storageBackend":"gridfs","message":"Attachment uploaded successfully"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let result = run(
        &server,
        &store,
        &[
            "upload",
            file.to_str().unwrap(),
            "--board",
            "b",
            "--swimlane-id",
            "s",
            "--list",
            "l",
            "--card",
            "c",
            "--transport",
            "raw",
        ],
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();
    let output: Value =
        serde_json::from_str(&crate::output::render_success(OutputFormat::Json, &result)).unwrap();
    assert_eq!(
        output,
        json!({"ok":true,"data":{
            "attachment_id":"a","file_name":"upload.bin","file_size":3,"storage_backend":"gridfs",
            "server_message":"Attachment uploaded successfully","request_accepted":true,"upload_confirmed":false
        }})
    );
    let human = crate::output::render_success(OutputFormat::Human, &result);
    assert!(human.starts_with("Attachment upload accepted; final availability is unconfirmed:"));
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn background_helper_failures_preserve_native_401_without_credential_replacement_advice() {
    for (server_error, code) in [
        ("t.isBoardAdmin is not a function", ErrorCode::ServerError),
        (
            "currentUser.isBoardAdmin is not a function",
            ErrorCode::ServerError,
        ),
        ("Invalid credentials", ErrorCode::AuthenticationRejected),
    ] {
        let server = MockServer::start().await;
        let store = Store::new(&server);
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("background.png");
        std::fs::write(&file, [0, 255, 10]).unwrap();
        Mock::given(method("POST"))
            .and(path("/api/attachment/upload-background"))
            .respond_with(
                ResponseTemplate::new(401)
                    .insert_header("retry-after", "5")
                    .set_body_json(json!({"success":false,"error":server_error})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let error = run(
            &server,
            &store,
            &[
                "upload-background",
                file.to_str().unwrap(),
                "--board",
                "b",
                "--transport",
                "raw",
            ],
            &FakeConfirmationProvider::declining(),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), code);
        assert_eq!(error.details().http_status, Some(401));
        assert_eq!(error.details().server_error.as_deref(), Some(server_error));
        assert_eq!(error.details().retry_after_seconds, None);
        if code == ErrorCode::ServerError {
            assert!(!error.message().contains("logout"));
            assert!(!error.message().contains("log in again"));
            assert!(error.message().contains("after storing the upload"));
            assert_eq!(error.details().mutation_attempted, Some(true));
            assert_eq!(error.details().retry_safe, Some(false));
        }
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}
