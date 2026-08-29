use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration as StdDuration;

use clap::Parser;
use secrecy::{ExposeSecret, SecretString};
use time::{Duration, OffsetDateTime};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

use super::ApiCommand;
use crate::{
    cli::Cli,
    client::WekanClientFactory,
    command_result::{CommandSuccess, DestructiveOperation},
    commands::RootCommand,
    credentials::{
        CredentialCreateOutcome, CredentialDeleteOutcome, CredentialError, CredentialMutation,
        CredentialRecord, CredentialStore, CredentialTarget,
    },
    error::ErrorCode,
    exit_code::StableExitCode,
    input::{ConfirmationProvider, ConfirmationRequest, FakeConfirmationProvider},
    output::{OutputFormat, write_success},
};

struct FakeCredentialStore {
    record: Mutex<Option<CredentialRecord>>,
    replacement_on_lock: Mutex<Option<CredentialRecord>>,
    mutation_active: std::sync::atomic::AtomicBool,
    load_count: AtomicUsize,
}

struct DelayedConfirmationProvider {
    delay: StdDuration,
    calls: AtomicUsize,
}

struct ReplacingConfirmationProvider<'a> {
    store: &'a FakeCredentialStore,
    replacement: Mutex<Option<CredentialRecord>>,
    calls: AtomicUsize,
}

struct ConcurrentReplacingConfirmationProvider {
    store: Arc<FakeCredentialStore>,
    replacement: Mutex<Option<CredentialRecord>>,
    completed: Arc<std::sync::atomic::AtomicBool>,
    handle: Mutex<Option<std::thread::JoinHandle<()>>>,
}

struct FailingWriter;

impl std::io::Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "closed output",
        ))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl ConfirmationProvider for DelayedConfirmationProvider {
    fn confirm(&self, _request: &ConfirmationRequest) -> Result<bool, crate::error::AppError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(self.delay);
        Ok(true)
    }
}

impl ConfirmationProvider for ReplacingConfirmationProvider<'_> {
    fn confirm(&self, _request: &ConfirmationRequest) -> Result<bool, crate::error::AppError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let replacement = self.replacement.lock().unwrap().take().unwrap();
        *self.store.record.lock().unwrap() = Some(replacement);
        Ok(true)
    }
}

impl ConfirmationProvider for ConcurrentReplacingConfirmationProvider {
    fn confirm(&self, _request: &ConfirmationRequest) -> Result<bool, crate::error::AppError> {
        let store = Arc::clone(&self.store);
        let replacement = self.replacement.lock().unwrap().take().unwrap();
        let completed = Arc::clone(&self.completed);
        let handle = std::thread::spawn(move || {
            while store
                .mutation_active
                .load(std::sync::atomic::Ordering::Acquire)
            {
                std::thread::yield_now();
            }
            *store.record.lock().unwrap() = Some(replacement);
            completed.store(true, std::sync::atomic::Ordering::Release);
        });
        *self.handle.lock().unwrap() = Some(handle);
        Ok(true)
    }
}

impl FakeCredentialStore {
    fn authenticated(server: &MockServer) -> Self {
        Self {
            record: Mutex::new(Some(record(server, "raw-token"))),
            replacement_on_lock: Mutex::new(None),
            mutation_active: std::sync::atomic::AtomicBool::new(false),
            load_count: AtomicUsize::new(0),
        }
    }

    fn absent() -> Self {
        Self {
            record: Mutex::new(None),
            replacement_on_lock: Mutex::new(None),
            mutation_active: std::sync::atomic::AtomicBool::new(false),
            load_count: AtomicUsize::new(0),
        }
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
        self.load_count.fetch_add(1, Ordering::SeqCst);
        Ok(self.record.lock().unwrap().clone())
    }

    fn create(
        &self,
        _target: &CredentialTarget,
        _record: &CredentialRecord,
    ) -> Result<CredentialCreateOutcome, CredentialError> {
        panic!("raw API commands must not create credentials")
    }

    fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("raw API commands must not delete credentials")
    }

    fn delete_if_matches(
        &self,
        _target: &CredentialTarget,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("raw API commands must not delete credentials")
    }

    fn lock_mutation(
        &self,
        _target: &CredentialTarget,
    ) -> Result<Box<dyn CredentialMutation + Send + '_>, CredentialError> {
        if let Some(replacement) = self.replacement_on_lock.lock().unwrap().take() {
            *self.record.lock().unwrap() = Some(replacement);
        }
        self.mutation_active.store(true, Ordering::Release);
        Ok(Box::new(FakeCredentialMutation { store: self }))
    }
}

struct FakeCredentialMutation<'a> {
    store: &'a FakeCredentialStore,
}

impl Drop for FakeCredentialMutation<'_> {
    fn drop(&mut self) {
        self.store.mutation_active.store(false, Ordering::Release);
    }
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
        panic!("raw API commands must not create credentials")
    }

    fn delete(&self) -> Result<bool, CredentialError> {
        panic!("raw API commands must not delete credentials")
    }

    fn delete_if_matches(
        &self,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("raw API commands must not delete credentials")
    }
}

fn record(server: &MockServer, token: &str) -> CredentialRecord {
    CredentialRecord::new(
        format!("{}/", server.uri()),
        "user-1".to_owned(),
        SecretString::from(token.to_owned()),
        OffsetDateTime::now_utc() + Duration::hours(1),
    )
}

fn factory(server: &MockServer) -> WekanClientFactory {
    WekanClientFactory::for_profile(format!("{}/", server.uri()), "default".to_owned(), false)
}

fn subpath_factory(server: &MockServer) -> WekanClientFactory {
    WekanClientFactory::for_profile(
        format!("{}/wekan/", server.uri()),
        "default".to_owned(),
        false,
    )
}

fn command(arguments: &[&str]) -> ApiCommand {
    let mut all = vec!["wekan", "--server", "https://wekan.example", "api"];
    all.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(all).unwrap();
    let RootCommand::Api(args) = cli.command else {
        panic!("expected api command")
    };
    args.command
}

async fn execute(
    command: ApiCommand,
    factory: &WekanClientFactory,
    store: &FakeCredentialStore,
) -> Result<CommandSuccess, crate::error::AppError> {
    super::dispatch(
        command,
        factory,
        store,
        &FakeConfirmationProvider::accepting(),
    )
    .await
}

#[test]
fn parser_supports_extension_methods_and_rejects_conflicting_or_managed_inputs() {
    command(&[
        "request",
        "PROPFIND",
        "/api/items",
        "--query",
        "tag=one",
        "--header",
        "x-test:value",
        "--body",
        "payload",
        "--timeout",
        "90",
        "--yes",
    ]);

    for arguments in [
        vec![
            "request",
            "POST",
            "/api/items",
            "--body",
            "x",
            "--json",
            "{}",
        ],
        vec![
            "request",
            "GET",
            "/api/items",
            "--no-auth",
            "--auth-token-query",
        ],
        vec![
            "request",
            "GET",
            "/api/items",
            "--header",
            "Authorization: secret",
        ],
        vec![
            "request",
            "GET",
            "/api/items",
            "--header",
            "Transfer-Encoding: chunked",
        ],
        vec!["request", "GET", "/api/items", "--query", "missing-equals"],
        vec!["request", "GET", "/api/items", "--timeout", "0"],
        vec!["request", "TRACE", "/api/items"],
        vec!["request", "track", "/api/items"],
        vec!["request", "CONNECT", "/api/items"],
    ] {
        let mut all = vec!["wekan", "api"];
        all.extend(arguments);
        assert!(Cli::try_parse_from(all).is_err());
    }
}

#[tokio::test]
async fn request_timeout_override_is_applied_to_the_complete_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/slow"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(StdDuration::from_millis(1_500))
                .set_body_string("late"),
        )
        .mount(&server)
        .await;
    let store = FakeCredentialStore::absent();

    let error = execute(
        command(&["request", "GET", "/api/slow", "--no-auth", "--timeout", "1"]),
        &factory(&server),
        &store,
    )
    .await
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::TransportError);
}

#[tokio::test]
async fn no_auth_preserves_subpaths_queries_headers_and_json_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/wekan/api/echo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "unknown": {"nested": true}
        })))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::absent();
    let result = execute(
        command(&[
            "request",
            "POST",
            "/api/echo?from=path",
            "--query",
            "tag=one",
            "--query",
            "tag=two",
            "--header",
            "x-test:first",
            "--header",
            "x-test:second",
            "--header",
            "Cookie: caller-session=explicit",
            "--json",
            r#"{"value":true}"#,
            "--no-auth",
            "--yes",
        ]),
        &subpath_factory(&server),
        &store,
    )
    .await
    .unwrap();
    assert_eq!(store.load_count.load(Ordering::SeqCst), 0);
    let requests = server.received_requests().await.unwrap();
    let request = &requests[0];
    assert!(request.headers.get("authorization").is_none());
    assert_eq!(
        request.headers.get("content-type").unwrap(),
        "application/json"
    );
    assert_eq!(request.body, br#"{"value":true}"#);
    let pairs = request.url.query_pairs().collect::<Vec<_>>();
    assert_eq!(pairs[0], ("from".into(), "path".into()));
    assert_eq!(pairs[1], ("tag".into(), "one".into()));
    assert_eq!(pairs[2], ("tag".into(), "two".into()));
    let values = request
        .headers
        .get_all("x-test")
        .iter()
        .map(|value| value.to_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(values, ["first", "second"]);
    assert_eq!(
        request.headers.get("cookie").unwrap(),
        "caller-session=explicit"
    );
    assert!(matches!(result, CommandSuccess::ApiResponse(_)));
}

#[tokio::test]
async fn body_files_stream_with_an_exact_content_length() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .expect(1)
        .mount(&server)
        .await;
    let body_path = std::env::temp_dir().join(format!(
        "wekan-cli-raw-body-{}-{}.bin",
        std::process::id(),
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    std::fs::write(&body_path, [0, 1, 2, 3, 255]).unwrap();
    let store = FakeCredentialStore::absent();
    let result = execute(
        command(&[
            "request",
            "POST",
            "/api/upload",
            "--body-file",
            body_path.to_str().unwrap(),
            "--no-auth",
            "--yes",
        ]),
        &factory(&server),
        &store,
    )
    .await;
    let _ = std::fs::remove_file(&body_path);
    result.unwrap();

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests[0].body, [0, 1, 2, 3, 255]);
    assert_eq!(requests[0].headers["content-length"], "5");
}

#[tokio::test]
async fn managed_auth_supports_bearer_and_secure_query_modes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .expect(2)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);

    execute(
        command(&["request", "GET", "/api/bearer"]),
        &factory(&server),
        &store,
    )
    .await
    .unwrap();
    execute(
        command(&[
            "request",
            "GET",
            "/api/query-auth-on-an-unmodeled-route",
            "--auth-token-query",
        ]),
        &factory(&server),
        &store,
    )
    .await
    .unwrap();

    let requests = server.received_requests().await.unwrap();
    let bearer_request = requests
        .iter()
        .find(|request| request.url.path() == "/api/bearer")
        .expect("bearer request should be captured");
    let query_token_request = requests
        .iter()
        .find(|request| request.url.path() == "/api/query-auth-on-an-unmodeled-route")
        .expect("query-token request should be captured");

    assert_eq!(bearer_request.headers["authorization"], "Bearer raw-token");
    assert!(bearer_request.url.query().is_none());
    assert!(query_token_request.headers.get("authorization").is_none());
    assert_eq!(
        query_token_request.url.query_pairs().collect::<Vec<_>>(),
        [("authToken".into(), "raw-token".into())]
    );
}

#[tokio::test]
async fn path_and_safe_method_validation_happen_before_credentials_or_http() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::absent();
    for arguments in [
        vec!["request", "GET", "https://evil.example/api"],
        vec!["request", "GET", "//evil.example/api"],
        vec!["request", "GET", "/api/../outside"],
        vec!["request", "GET", "/api\\..\\outside"],
        vec!["request", "GET", "/api\\items"],
        vec!["request", "GET", "/api/items#fragment"],
        vec!["request", "GET", "/api/items?authToken=secret"],
        vec!["request", "GET", "/api/items", "--yes"],
        vec![
            "request",
            "POST",
            "/api/items",
            "--auth-token-query",
            "--yes",
        ],
    ] {
        let error = execute(command(&arguments), &factory(&server), &store)
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidInput);
    }
    assert_eq!(store.load_count.load(Ordering::SeqCst), 0);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn unsafe_requests_can_cancel_and_detect_a_changed_credential() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore::authenticated(&server);
    let cancelled = super::dispatch(
        command(&["request", "DELETE", "/api/items/1"]),
        &factory(&server),
        &store,
        &FakeConfirmationProvider::declining(),
    )
    .await
    .unwrap();
    assert_eq!(
        cancelled,
        CommandSuccess::Cancelled(crate::command_result::CancellationSuccess::new(
            DestructiveOperation::ApiRequest
        ))
    );
    assert!(server.received_requests().await.unwrap().is_empty());

    *store.replacement_on_lock.lock().unwrap() = Some(record(&server, "replacement"));
    let confirmation = FakeConfirmationProvider::accepting();
    let error = super::dispatch(
        command(&["request", "POST", "/api/items"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
    assert_eq!(confirmation.requests().len(), 1);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn unsafe_requests_recheck_a_credential_replaced_during_confirmation() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/items"))
        .respond_with(ResponseTemplate::new(200).set_body_string("must not send"))
        .expect(0)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::authenticated(&server);
    let confirmation = ReplacingConfirmationProvider {
        store: &store,
        replacement: Mutex::new(Some(record(&server, "replacement"))),
        calls: AtomicUsize::new(0),
    };

    let error = super::dispatch(
        command(&["request", "POST", "/api/items"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
    assert_eq!(confirmation.calls.load(Ordering::SeqCst), 1);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn unsafe_requests_hold_the_credential_mutation_guard_through_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/items"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(StdDuration::from_millis(300))
                .set_body_string("sent while guarded"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let store = Arc::new(FakeCredentialStore::authenticated(&server));
    let completed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let confirmation = ConcurrentReplacingConfirmationProvider {
        store: Arc::clone(&store),
        replacement: Mutex::new(Some(record(&server, "replacement"))),
        completed: Arc::clone(&completed),
        handle: Mutex::new(None),
    };
    let client_factory = factory(&server);

    let mut operation = Box::pin(super::dispatch(
        command(&["request", "POST", "/api/items"]),
        &client_factory,
        store.as_ref(),
        &confirmation,
    ));
    let mut request_observed = false;
    let result = loop {
        tokio::select! {
            result = &mut operation => break result,
            _ = tokio::time::sleep(StdDuration::from_millis(10)) => {
                if !server.received_requests().await.unwrap().is_empty() {
                    request_observed = true;
                    assert!(!completed.load(std::sync::atomic::Ordering::Acquire));
                    break operation.await;
                }
            }
        }
    };
    result.unwrap();
    confirmation
        .handle
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .join()
        .unwrap();

    assert!(request_observed);
    assert!(completed.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(
        store
            .record
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .token()
            .expose_secret(),
        "replacement"
    );
}

#[tokio::test]
async fn unsafe_requests_recheck_expiry_after_confirmation() {
    let server = MockServer::start().await;
    let store = FakeCredentialStore {
        record: Mutex::new(Some(CredentialRecord::new(
            format!("{}/", server.uri()),
            "user-1".to_owned(),
            SecretString::from("raw-token".to_owned()),
            OffsetDateTime::now_utc() + Duration::milliseconds(500),
        ))),
        replacement_on_lock: Mutex::new(None),
        mutation_active: std::sync::atomic::AtomicBool::new(false),
        load_count: AtomicUsize::new(0),
    };
    let confirmation = DelayedConfirmationProvider {
        delay: StdDuration::from_millis(750),
        calls: AtomicUsize::new(0),
    };

    let error = super::dispatch(
        command(&["request", "POST", "/api/items"]),
        &factory(&server),
        &store,
        &confirmation,
    )
    .await
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::CredentialExpired);
    assert_eq!(confirmation.calls.load(Ordering::SeqCst), 1);
    assert!(error.details().token_expires.is_some());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn raw_output_is_byte_exact_and_routes_http_errors_to_stderr() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/binary"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes([0, 1, 2, 255]))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/denied"))
        .respond_with(ResponseTemplate::new(403).set_body_bytes(b"denied"))
        .mount(&server)
        .await;
    let store = FakeCredentialStore::absent();

    let success = execute(
        command(&["request", "GET", "/api/binary", "--no-auth"]),
        &factory(&server),
        &store,
    )
    .await
    .unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = write_success(OutputFormat::Raw, success, &mut stdout, &mut stderr)
        .await
        .unwrap();
    assert_eq!(exit, std::process::ExitCode::SUCCESS);
    assert_eq!(stdout, [0, 1, 2, 255]);
    assert!(stderr.is_empty());

    let failure = execute(
        command(&["request", "GET", "/api/denied", "--no-auth"]),
        &factory(&server),
        &store,
    )
    .await
    .unwrap();
    stdout.clear();
    stderr.clear();
    let exit = write_success(OutputFormat::Raw, failure, &mut stdout, &mut stderr)
        .await
        .unwrap();
    assert_ne!(exit, std::process::ExitCode::SUCCESS);
    assert!(stdout.is_empty());
    assert_eq!(stderr, b"denied");
}

#[tokio::test]
async fn raw_output_write_failures_use_the_transport_exit_status() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/output-failure"))
        .respond_with(ResponseTemplate::new(200).set_body_string("response"))
        .mount(&server)
        .await;
    let store = FakeCredentialStore::absent();
    let response = execute(
        command(&["request", "GET", "/api/output-failure", "--no-auth"]),
        &factory(&server),
        &store,
    )
    .await
    .unwrap();
    let mut stdout = FailingWriter;
    let mut stderr = Vec::new();

    let exit = write_success(OutputFormat::Raw, response, &mut stdout, &mut stderr)
        .await
        .unwrap();

    assert_eq!(
        exit,
        std::process::ExitCode::from(StableExitCode::Transport)
    );
    assert!(stderr.is_empty());
}

#[tokio::test]
async fn json_output_preserves_unmodeled_response_fields_and_non_success_bodies() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/unknown"))
        .respond_with(
            ResponseTemplate::new(200)
                .append_header("x-repeat", "one")
                .append_header("x-repeat", "two")
                .set_body_json(serde_json::json!({
                    "statusCode": 401,
                    "error": "embedded-but-uninterpreted",
                    "future": {"field": [1, 2, 3]}
                })),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/missing"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "unmapped": true
        })))
        .mount(&server)
        .await;
    let store = FakeCredentialStore::absent();

    let success = execute(
        command(&["request", "GET", "/api/unknown", "--no-auth"]),
        &factory(&server),
        &store,
    )
    .await
    .unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    write_success(OutputFormat::Json, success, &mut stdout, &mut stderr)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["body"]["encoding"], "json");
    assert_eq!(value["data"]["mutation_attempted"], false);
    assert_eq!(value["data"]["mutation_confirmed"], false);
    assert_eq!(value["data"]["body"]["value"]["statusCode"], 401);
    assert_eq!(value["data"]["body"]["value"]["future"]["field"][2], 3);
    assert_eq!(
        value["data"]["headers"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|header| header["name"] == "x-repeat")
            .count(),
        2
    );

    let failure = execute(
        command(&["request", "GET", "/api/missing", "--no-auth"]),
        &factory(&server),
        &store,
    )
    .await
    .unwrap();
    stdout.clear();
    stderr.clear();
    let exit = write_success(OutputFormat::Json, failure, &mut stdout, &mut stderr)
        .await
        .unwrap();
    assert_ne!(exit, std::process::ExitCode::SUCCESS);
    assert!(stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
    assert_eq!(value["error"]["code"], "server_error");
    assert_eq!(value["error"]["details"]["http_status"], 404);
    assert_eq!(
        value["error"]["details"]["response"]["body"]["value"]["unmapped"],
        true
    );
}

#[tokio::test]
async fn structured_success_reports_unsafe_mutation_as_unconfirmed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/mutation"))
        .respond_with(ResponseTemplate::new(202).set_body_json(serde_json::json!({
            "accepted": true
        })))
        .mount(&server)
        .await;
    let store = FakeCredentialStore::absent();
    let response = execute(
        command(&["request", "POST", "/api/mutation", "--no-auth", "--yes"]),
        &factory(&server),
        &store,
    )
    .await
    .unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let exit = write_success(OutputFormat::Json, response, &mut stdout, &mut stderr)
        .await
        .unwrap();

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
    let value: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(value["data"]["http_status"], 202);
    assert_eq!(value["data"]["mutation_attempted"], true);
    assert_eq!(value["data"]["mutation_confirmed"], false);
}

#[tokio::test]
async fn structured_output_maps_redirect_auth_and_server_statuses() {
    let server = MockServer::start().await;
    for (endpoint, status) in [
        ("/api/moved", 302),
        ("/api/auth", 401),
        ("/api/failure", 500),
    ] {
        Mock::given(path(endpoint))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_json(serde_json::json!({"endpoint": endpoint})),
            )
            .mount(&server)
            .await;
    }
    let store = FakeCredentialStore::absent();

    for (arguments, expected_code, expected_status, outcome_unknown) in [
        (
            vec!["request", "GET", "/api/moved", "--no-auth"],
            "unexpected_redirect",
            302,
            None,
        ),
        (
            vec!["request", "GET", "/api/auth", "--no-auth"],
            "authentication_rejected",
            401,
            None,
        ),
        (
            vec!["request", "POST", "/api/failure", "--no-auth", "--yes"],
            "server_error",
            500,
            Some(true),
        ),
    ] {
        let response = execute(command(&arguments), &factory(&server), &store)
            .await
            .unwrap();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let exit = write_success(OutputFormat::Json, response, &mut stdout, &mut stderr)
            .await
            .unwrap();
        assert_ne!(exit, std::process::ExitCode::SUCCESS);
        assert!(stdout.is_empty());

        let value: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(value["error"]["code"], expected_code);
        assert_eq!(value["error"]["details"]["http_status"], expected_status);
        assert_eq!(
            value["error"]["details"]["response"]["http_status"],
            expected_status
        );
        assert_eq!(
            value["error"]["details"]["outcome_unknown"].as_bool(),
            outcome_unknown
        );
    }
}

#[tokio::test]
async fn structured_output_accepts_head_representation_content_length_over_the_body_limit() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/api/large-head"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-length", (1024 * 1024 + 1).to_string()),
        )
        .mount(&server)
        .await;
    let store = FakeCredentialStore::absent();
    let response = execute(
        command(&["request", "HEAD", "/api/large-head", "--no-auth"]),
        &factory(&server),
        &store,
    )
    .await
    .unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let exit = write_success(OutputFormat::Json, response, &mut stdout, &mut stderr)
        .await
        .unwrap();

    assert_eq!(exit, std::process::ExitCode::SUCCESS);
    assert!(stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(value["data"]["http_status"], 200);
    assert_eq!(value["data"]["body"]["encoding"], "text");
    assert_eq!(value["data"]["body"]["value"], "");
}

#[tokio::test]
async fn structured_output_rejects_large_bodies_and_directs_callers_to_raw() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/large"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/large-mutation"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .mount(&server)
        .await;
    let store = FakeCredentialStore::absent();
    let response = execute(
        command(&["request", "GET", "/api/large", "--no-auth"]),
        &factory(&server),
        &store,
    )
    .await
    .unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = write_success(OutputFormat::Json, response, &mut stdout, &mut stderr)
        .await
        .unwrap();
    assert_ne!(exit, std::process::ExitCode::SUCCESS);
    assert!(stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
    assert_eq!(value["error"]["code"], "api_response_too_large");
    assert_eq!(value["error"]["details"]["http_status"], 200);
    assert_eq!(value["error"]["details"]["http_success"], true);
    assert_eq!(
        value["error"]["details"]["response_limit_bytes"],
        1024 * 1024
    );
    assert!(value["error"]["details"]["outcome_unknown"].is_null());
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("--output raw")
    );

    let response = execute(
        command(&[
            "request",
            "POST",
            "/api/large-mutation",
            "--no-auth",
            "--yes",
        ]),
        &factory(&server),
        &store,
    )
    .await
    .unwrap();
    stdout.clear();
    stderr.clear();
    write_success(OutputFormat::Json, response, &mut stdout, &mut stderr)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
    assert_eq!(value["error"]["code"], "api_response_too_large");
    assert_eq!(value["error"]["details"]["http_success"], true);
    assert_eq!(value["error"]["details"]["retry_safe"], false);
    assert_eq!(value["error"]["details"]["mutation_attempted"], true);
    assert_eq!(value["error"]["details"]["mutation_confirmed"], false);
    assert_eq!(value["error"]["details"]["outcome_unknown"], true);
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("do not retry")
    );
}
