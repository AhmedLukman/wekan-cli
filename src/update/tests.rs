use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

use super::cache::CacheEntry;
use super::*;
use crate::input::FakeConfirmationProvider;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

struct FixedClock(u64);
impl Clock for FixedClock {
    fn now(&self) -> u64 {
        self.0
    }
}

struct Releases {
    response: Result<Release, AppError>,
    requests: RefCell<Vec<Duration>>,
}
impl ReleaseProvider for Releases {
    async fn latest(&self, timeout: Duration) -> Result<Release, AppError> {
        self.requests.borrow_mut().push(timeout);
        self.response.clone()
    }
}

struct Installation {
    method: InstallationMethod,
    installed: Cell<bool>,
    fail: bool,
}
impl Installer for Installation {
    fn method(&self) -> InstallationMethod {
        self.method
    }
    async fn install(&self, _: &Release) -> Result<(), AppError> {
        self.installed.set(true);
        if self.fail {
            Err(update_failed("test installation failed"))
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
struct Cache {
    entry: CacheEntry,
    fail_load: bool,
    fail_save: bool,
}
impl UpdateCache for Cache {
    fn load(&self) -> Result<CacheEntry, AppError> {
        if self.fail_load {
            Err(file_error("load", "test failure"))
        } else {
            Ok(self.entry.clone())
        }
    }
    fn save(&mut self, entry: &CacheEntry) -> Result<(), AppError> {
        if self.fail_save {
            return Err(file_error("save", "test failure"));
        }
        self.entry = entry.clone();
        Ok(())
    }
}

pub(super) fn release(version: &str) -> Release {
    Release {
        tag: format!("v{version}"),
        version: semver::Version::parse(version).unwrap(),
        assets: vec![],
    }
}

fn service(version: &str) -> UpdateService<Releases, Installation, FixedClock> {
    UpdateService {
        http: Releases {
            response: Ok(release(version)),
            requests: RefCell::default(),
        },
        installer: Installation {
            method: InstallationMethod::Release,
            installed: Cell::new(false),
            fail: false,
        },
        clock: FixedClock(200_000),
        current_version: "2.0.0".to_owned(),
    }
}

#[test]
fn startup_requires_human_output_and_all_terminal_streams_without_opt_out_or_ci() {
    fn eligible() -> StartupEligibility {
        StartupEligibility {
            human: true,
            stdin_terminal: true,
            stdout_terminal: true,
            stderr_terminal: true,
            ci: false,
            disabled: false,
            self_command: false,
        }
    }
    assert!(eligible().eligible());
    for field in 0..7 {
        let mut gate = eligible();
        match field {
            0 => gate.human = false,
            1 => gate.stdin_terminal = false,
            2 => gate.stdout_terminal = false,
            3 => gate.stderr_terminal = false,
            4 => gate.ci = true,
            5 => gate.disabled = true,
            _ => gate.self_command = true,
        }
        assert!(!gate.eligible(), "field {field}");
    }
}

#[tokio::test]
async fn checks_compare_semver_and_never_downgrade_or_treat_build_metadata_as_newer() {
    for (version, expected) in [
        ("1.99.0", SelfUpdateStatus::UpToDate),
        ("2.0.0", SelfUpdateStatus::UpToDate),
        ("2.0.0+build.1", SelfUpdateStatus::UpToDate),
        ("2.0.1", SelfUpdateStatus::Available),
        ("10.0.0", SelfUpdateStatus::Available),
    ] {
        let service = service(version);
        let confirmation = FakeConfirmationProvider::accepting();
        let ManualOutcome::Complete(data) =
            service.manual(true, false, &confirmation).await.unwrap()
        else {
            panic!()
        };
        assert_eq!(data.status, expected);
        assert!(!service.installer.installed.get());
        assert!(confirmation.requests().is_empty());
        assert_eq!(*service.http.requests.borrow(), [MANUAL_TIMEOUT]);
    }
}

#[tokio::test]
async fn manual_confirmation_cancellation_yes_and_unknown_installation() {
    let service = service("2.1.0");
    let no = FakeConfirmationProvider::declining();
    assert!(matches!(
        service.manual(false, false, &no).await.unwrap(),
        ManualOutcome::Cancelled
    ));
    assert!(!service.installer.installed.get());
    assert!(no.requests()[0].prompt().contains("2.0.0 → 2.1.0"));
    let never = FakeConfirmationProvider::failing(AppError::invalid_input("must not prompt"));
    let ManualOutcome::Complete(data) = service.manual(false, true, &never).await.unwrap() else {
        panic!()
    };
    assert_eq!(data.status, SelfUpdateStatus::Updated);
    assert!(service.installer.installed.get());
    assert!(never.requests().is_empty());
    let mut unknown = super::tests::service("2.1.0");
    unknown.installer.method = InstallationMethod::Unknown;
    assert_eq!(
        unknown
            .manual(false, true, &never)
            .await
            .unwrap_err()
            .code(),
        ErrorCode::UpdateFailed
    );
    assert!(unknown.manual(true, false, &never).await.is_ok());
    assert!(!unknown.installer.installed.get());
}

#[tokio::test]
async fn automatic_decline_continues_and_acceptance_stops_the_original_command() {
    for accepted in [false, true] {
        let service = service("2.1.0");
        let confirmation = if accepted {
            FakeConfirmationProvider::accepting()
        } else {
            FakeConfirmationProvider::declining()
        };
        let mut cache = Cache::default();
        let outcome = service.automatic(&mut cache, &confirmation).await.unwrap();
        assert_eq!(outcome.should_exit(), accepted);
        assert_eq!(service.installer.installed.get(), accepted);
        assert_eq!(confirmation.requests().len(), 1);
        assert_eq!(*service.http.requests.borrow(), [AUTOMATIC_TIMEOUT]);
        assert_eq!(cache.entry.last_notified, Some(service.clock.now()));
        assert!(
            !service
                .automatic(&mut cache, &confirmation)
                .await
                .unwrap()
                .should_exit()
        );
        assert_eq!(confirmation.requests().len(), 1);
        assert_eq!(service.http.requests.borrow().len(), 1);
    }
}

#[tokio::test]
async fn automatic_failures_are_quiet_but_accepted_installation_failure_is_returned() {
    for failure in 0..3 {
        let mut service = service("2.1.0");
        let mut cache = Cache::default();
        match failure {
            0 => service.http.response = Err(protocol("offline")),
            1 => cache.fail_load = true,
            _ => cache.fail_save = true,
        }
        let confirmation = FakeConfirmationProvider::accepting();
        assert!(matches!(
            service.automatic(&mut cache, &confirmation).await.unwrap(),
            StartupUpdateOutcome::Continue
        ));
        assert!(confirmation.requests().is_empty());
        assert!(!service.installer.installed.get());
        if failure == 0 {
            assert!(matches!(
                service.automatic(&mut cache, &confirmation).await.unwrap(),
                StartupUpdateOutcome::Continue
            ));
            assert_eq!(service.http.requests.borrow().len(), 1);
        }
    }
    let mut service = service("2.1.0");
    service.installer.fail = true;
    let error = service
        .automatic(
            &mut Cache::default(),
            &FakeConfirmationProvider::accepting(),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::UpdateFailed);
    assert_eq!(error.exit_code(), StableExitCode::Transport);
}

#[tokio::test]
async fn cache_expires_at_24_hours_and_future_timestamps_are_rechecked() {
    for timestamp in [200_000 - CHECK_INTERVAL, 200_001] {
        let service = service("2.1.0");
        let mut cache = Cache {
            entry: CacheEntry {
                last_attempt: Some(timestamp),
                last_notified: Some(timestamp),
                release: Some(release("2.0.0")),
            },
            ..Cache::default()
        };
        let no = FakeConfirmationProvider::declining();
        service.automatic(&mut cache, &no).await.unwrap();
        assert_eq!(service.http.requests.borrow().len(), 1);
        assert_eq!(no.requests().len(), 1);
    }
}

#[tokio::test]
async fn fresh_cached_availability_is_used_without_network_and_unknown_installs_only_get_a_notice()
{
    let mut service = service("2.1.0");
    service.installer.method = InstallationMethod::Unknown;
    let mut cache = Cache {
        entry: CacheEntry {
            last_attempt: Some(199_999),
            last_notified: None,
            release: Some(release("2.1.0")),
        },
        ..Cache::default()
    };
    let no = FakeConfirmationProvider::accepting();
    let outcome = service.automatic(&mut cache, &no).await.unwrap();
    assert!(!outcome.should_exit());
    let StartupUpdateOutcome::Notice(data) = outcome else {
        panic!("unknown installations receive a notice");
    };
    assert!(service.http.requests.borrow().is_empty());
    assert!(no.requests().is_empty());
    assert_eq!(data.current_version, "2.0.0");
    assert_eq!(data.latest_version, "2.1.0");
    assert_eq!(data.installation_method, InstallationMethod::Unknown);
}

#[test]
fn disk_cache_is_atomic_recovers_from_invalid_data_and_excludes_concurrent_starts() {
    let dir = tempfile::tempdir().unwrap();
    let mut cache = FileCache::open(dir.path().to_owned()).unwrap();
    assert!(FileCache::open(dir.path().to_owned()).is_err());
    assert!(cache.load().unwrap().last_attempt.is_none());
    std::fs::write(dir.path().join("check.json"), "invalid").unwrap();
    assert!(cache.load().unwrap().last_attempt.is_none());
    let entry = CacheEntry {
        last_attempt: Some(42),
        last_notified: Some(42),
        release: Some(release("2.1.0")),
    };
    cache.save(&entry).unwrap();
    cache.save(&entry).unwrap();
    assert_eq!(cache.load().unwrap().last_notified, Some(42));
    drop(cache);
    assert!(FileCache::open(dir.path().to_owned()).is_ok());
}

fn github_json(tag: &str) -> serde_json::Value {
    serde_json::json!({"tag_name": tag, "draft": false, "prerelease": false, "assets": [], "unrelated_github_field": true})
}

#[tokio::test]
async fn github_discovery_handles_tags_invalid_responses_and_http_errors() {
    for (tag, draft, prerelease, status) in [
        ("v2.1.0", false, false, 200),
        ("2.1.0", false, false, 200),
        ("v2.2.0-rc.1", false, false, 200),
        ("v2.1.0", true, false, 200),
        ("v2.1.0", false, true, 200),
        ("invalid", false, false, 200),
        ("v2.1.0", false, false, 403),
        ("v2.1.0", false, false, 429),
        ("v2.1.0", false, false, 404),
    ] {
        let server = MockServer::start().await;
        let mut body = github_json(tag);
        body["draft"] = draft.into();
        body["prerelease"] = prerelease.into();
        Mock::given(method("GET"))
            .and(path("/latest"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let http = HttpReleases {
            client: reqwest::Client::new(),
            endpoint: format!("{}/latest", server.uri()),
        };
        let result = http.latest(Duration::from_secs(1)).await;
        if status == 200 && !draft && !prerelease && matches!(tag, "v2.1.0" | "2.1.0") {
            assert_eq!(result.unwrap().version.to_string(), "2.1.0");
        } else {
            assert!(result.is_err(), "{tag}, {status}");
        }
    }
}

#[tokio::test]
async fn discovery_timeout_is_bounded_and_errors_do_not_include_response_bodies() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(github_json("v2.1.0"))
                .set_delay(Duration::from_secs(1)),
        )
        .mount(&server)
        .await;
    let http = HttpReleases {
        client: reqwest::Client::new(),
        endpoint: server.uri(),
    };
    let start = std::time::Instant::now();
    let error = http.latest(Duration::from_millis(20)).await.err().unwrap();
    assert_eq!(error.code(), ErrorCode::TransportError);
    assert!(error.message().contains("timed out"));
    assert!(start.elapsed() < Duration::from_secs(1));
}
