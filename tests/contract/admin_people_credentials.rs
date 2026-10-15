use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration as StdDuration, Instant},
};

use secrecy::SecretString;
use serde_json::json;
use time::{Duration, OffsetDateTime};
use wekan_cli::{
    credentials::{
        CredentialCreateOutcome, CredentialDeleteOutcome, CredentialError, CredentialMutation,
        CredentialRecord, CredentialStore, CredentialTarget, SystemSecretInputProvider,
    },
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    input::{ConfirmationProvider, ConfirmationRequest},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

use super::admin_commands::run_with_store_and_providers;

#[derive(Clone)]
struct Store {
    record: Arc<Mutex<Option<CredentialRecord>>>,
    mutation_active: Arc<AtomicBool>,
}

impl Store {
    fn new(record: CredentialRecord) -> Self {
        Self {
            record: Arc::new(Mutex::new(Some(record))),
            mutation_active: Arc::new(AtomicBool::new(false)),
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
        panic!("feature updates must not store credentials")
    }

    fn delete(&self, _: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("feature updates must not delete credentials")
    }

    fn delete_if_matches(
        &self,
        _: &CredentialTarget,
        _: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("feature updates must not delete credentials")
    }

    fn lock_mutation(
        &self,
        _: &CredentialTarget,
    ) -> Result<Box<dyn CredentialMutation + Send + '_>, CredentialError> {
        assert!(!self.mutation_active.swap(true, Ordering::AcqRel));
        Ok(Box::new(Mutation(self)))
    }
}

struct Mutation<'a>(&'a Store);

impl Drop for Mutation<'_> {
    fn drop(&mut self) {
        self.0.mutation_active.store(false, Ordering::Release);
    }
}

impl CredentialMutation for Mutation<'_> {
    fn exists(&self) -> Result<bool, CredentialError> {
        Ok(self.0.record.lock().unwrap().is_some())
    }

    fn load(&self) -> Result<Option<CredentialRecord>, CredentialError> {
        assert!(self.0.mutation_active.load(Ordering::Acquire));
        Ok(self.0.record.lock().unwrap().clone())
    }

    fn create(&self, _: &CredentialRecord) -> Result<CredentialCreateOutcome, CredentialError> {
        panic!("feature updates must not store credentials")
    }

    fn delete(&self) -> Result<bool, CredentialError> {
        panic!("feature updates must not delete credentials")
    }

    fn delete_if_matches(
        &self,
        _: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("feature updates must not delete credentials")
    }
}

fn record(server: &MockServer, token: &str, expires: OffsetDateTime) -> CredentialRecord {
    CredentialRecord::new(
        format!("{}/", server.uri()),
        "user-1".into(),
        SecretString::from(token.to_owned()),
        expires,
    )
}

fn arguments(kind: &str) -> [&str; 7] {
    [
        kind,
        "features",
        "update-all",
        "--field",
        "shared-templates",
        "--value",
        "true",
    ]
}

struct ChangingConfirmation {
    store: Store,
    replacement: Option<CredentialRecord>,
}

impl ConfirmationProvider for ChangingConfirmation {
    fn confirm(&self, _: &ConfirmationRequest) -> Result<bool, AppError> {
        assert!(self.store.mutation_active.load(Ordering::Acquire));
        // Inject a stale guarded read even though real providers serialize writes.
        *self.store.record.lock().unwrap() = self.replacement.clone();
        Ok(true)
    }
}

#[tokio::test]
async fn bulk_updates_recheck_replaced_and_removed_credentials_after_confirmation() {
    for kind in ["org", "team"] {
        for removed in [false, true] {
            let server = MockServer::start().await;
            let expires = OffsetDateTime::now_utc() + Duration::hours(1);
            let store = Store::new(record(&server, "original-token", expires));
            let replacement = (!removed).then(|| record(&server, "replacement-token", expires));
            let error = run_with_store_and_providers(
                &server,
                &arguments(kind),
                store.clone(),
                SystemSecretInputProvider,
                ChangingConfirmation {
                    store: store.clone(),
                    replacement: replacement.clone(),
                },
            )
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
            assert_eq!(error.exit_code(), StableExitCode::Credential);
            assert!(error.message().contains("no HTTP request was sent"));
            assert!(server.received_requests().await.unwrap().is_empty());
            assert!(!store.mutation_active.load(Ordering::Acquire));
            let saved = store.record.lock().unwrap();
            match replacement {
                Some(expected) => assert!(saved.as_ref().unwrap().matches(&expected)),
                None => assert!(saved.is_none()),
            }
        }
    }
}

struct ExpiringConfirmation {
    store: Store,
    expires: OffsetDateTime,
}

impl ConfirmationProvider for ExpiringConfirmation {
    fn confirm(&self, _: &ConfirmationRequest) -> Result<bool, AppError> {
        assert!(self.store.mutation_active.load(Ordering::Acquire));
        while OffsetDateTime::now_utc() < self.expires {
            std::thread::sleep(StdDuration::from_millis(10));
        }
        Ok(true)
    }
}

#[tokio::test]
async fn bulk_updates_recheck_expiry_after_confirmation() {
    for kind in ["org", "team"] {
        let server = MockServer::start().await;
        let expires = OffsetDateTime::now_utc() + Duration::seconds(2);
        let original = record(&server, "original-token", expires);
        let store = Store::new(original.clone());
        let error = run_with_store_and_providers(
            &server,
            &arguments(kind),
            store.clone(),
            SystemSecretInputProvider,
            ExpiringConfirmation {
                store: store.clone(),
                expires,
            },
        )
        .await
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::CredentialExpired);
        assert_eq!(error.exit_code(), StableExitCode::Server);
        assert!(server.received_requests().await.unwrap().is_empty());
        assert!(!store.mutation_active.load(Ordering::Acquire));
        assert!(
            store
                .record
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .matches(&original)
        );
    }
}

struct ConcurrentConfirmation {
    store: Store,
    replacement: CredentialRecord,
    worker: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
}

impl ConfirmationProvider for ConcurrentConfirmation {
    fn confirm(&self, _: &ConfirmationRequest) -> Result<bool, AppError> {
        assert!(self.store.mutation_active.load(Ordering::Acquire));
        let store = self.store.clone();
        let replacement = self.replacement.clone();
        *self.worker.lock().unwrap() = Some(std::thread::spawn(move || {
            let deadline = Instant::now() + StdDuration::from_secs(5);
            while store.mutation_active.load(Ordering::Acquire) {
                assert!(
                    Instant::now() < deadline,
                    "credential guard was never released"
                );
                std::thread::sleep(StdDuration::from_millis(1));
            }
            *store.record.lock().unwrap() = Some(replacement);
        }));
        Ok(true)
    }
}

#[tokio::test]
async fn bulk_updates_hold_the_credential_guard_through_http_and_preserve_a_waiting_replacement() {
    for kind in ["org", "team"] {
        let server = MockServer::start().await;
        let resource = if kind == "org" { "orgs" } else { "teams" };
        Mock::given(method("PUT"))
            .and(path(format!("/api/admin/{resource}/features")))
            .and(header("authorization", "Bearer original-token"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(StdDuration::from_millis(300))
                    .set_body_json(json!({"updated":2})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let expires = OffsetDateTime::now_utc() + Duration::hours(1);
        let original = record(&server, "original-token", expires);
        let replacement = record(&server, "replacement-token", expires);
        let store = Store::new(original.clone());
        let worker = Arc::new(Mutex::new(None));
        let args = arguments(kind);
        let operation = run_with_store_and_providers(
            &server,
            &args,
            store.clone(),
            SystemSecretInputProvider,
            ConcurrentConfirmation {
                store: store.clone(),
                replacement: replacement.clone(),
                worker: worker.clone(),
            },
        );
        let operation = tokio::time::timeout(StdDuration::from_secs(5), operation);
        tokio::pin!(operation);
        let mut request_observed = false;
        loop {
            tokio::select! {
                result = &mut operation => {
                    result.unwrap().unwrap();
                    break;
                }
                _ = tokio::time::sleep(StdDuration::from_millis(10)) => {
                    if !server.received_requests().await.unwrap().is_empty() {
                        request_observed = true;
                        assert!(store.mutation_active.load(Ordering::Acquire));
                        assert!(store.record.lock().unwrap().as_ref().unwrap().matches(&original));
                    }
                }
            }
        }
        worker.lock().unwrap().take().unwrap().join().unwrap();
        assert!(request_observed);
        assert!(!store.mutation_active.load(Ordering::Acquire));
        assert!(
            store
                .record
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .matches(&replacement)
        );
    }
}
