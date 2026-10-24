//! CLI release discovery and update orchestration, independent of Wekan servers.
mod cache;
mod install;
mod release;

pub(crate) use cache::FileCache;
use cache::UpdateCache;
use install::{Installer, SystemInstaller};
use release::{HttpReleases, Release, ReleaseProvider};

use std::{
    io::{self, IsTerminal},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{
    command_result::{
        DestructiveOperation, InstallationMethod, SelfUpdateStatus, SelfUpdateSuccess,
        StartupUpdateOutcome,
    },
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    input::{ConfirmationProvider, ConfirmationRequest},
    output::OutputFormat,
};

const CHECK_INTERVAL: u64 = 24 * 60 * 60;
const AUTOMATIC_TIMEOUT: Duration = Duration::from_secs(2);
const MANUAL_TIMEOUT: Duration = Duration::from_secs(15);
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) struct StartupEligibility {
    pub human: bool,
    pub stdin_terminal: bool,
    pub stdout_terminal: bool,
    pub stderr_terminal: bool,
    pub ci: bool,
    pub disabled: bool,
    pub self_command: bool,
}

impl StartupEligibility {
    pub fn production(output: OutputFormat, self_command: bool) -> Self {
        Self {
            human: output == OutputFormat::Human,
            stdin_terminal: io::stdin().is_terminal(),
            stdout_terminal: io::stdout().is_terminal(),
            stderr_terminal: io::stderr().is_terminal(),
            ci: [
                "CI",
                "CONTINUOUS_INTEGRATION",
                "GITHUB_ACTIONS",
                "BUILD_BUILDID",
                "JENKINS_URL",
                "TEAMCITY_VERSION",
            ]
            .iter()
            .any(|key| std::env::var_os(key).is_some()),
            disabled: std::env::var_os("WEKAN_NO_UPDATE_NOTIFIER").is_some(),
            self_command,
        }
    }

    pub fn eligible(&self) -> bool {
        self.human
            && self.stdin_terminal
            && self.stdout_terminal
            && self.stderr_terminal
            && !self.ci
            && !self.disabled
            && !self.self_command
    }
}

pub(crate) trait Clock {
    fn now(&self) -> u64;
}

pub(crate) struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

pub(crate) struct UpdateService<H = HttpReleases, I = SystemInstaller, T = SystemClock> {
    http: H,
    installer: I,
    clock: T,
    current_version: String,
}

// Keep updater futures independent of the large application-wide success enum.
// Nesting that enum through another async command path exhausts the Windows
// process entry stack in debug builds, including help and parser-error paths.
#[derive(Debug)]
pub(crate) enum ManualOutcome {
    Complete(SelfUpdateSuccess),
    Cancelled,
}

impl UpdateService {
    pub fn production() -> Result<Self, AppError> {
        let http = HttpReleases::production()?;
        Ok(Self {
            installer: SystemInstaller::production(http.clone())?,
            http,
            clock: SystemClock,
            current_version: CURRENT_VERSION.to_owned(),
        })
    }
}

impl<H: ReleaseProvider, I: Installer, T: Clock> UpdateService<H, I, T> {
    fn result(&self, release: &Release, status: SelfUpdateStatus) -> SelfUpdateSuccess {
        SelfUpdateSuccess {
            current_version: self.current_version.clone(),
            latest_version: release.version.to_string(),
            status,
            installation_method: self.installer.method(),
            release_url: release.url(),
        }
    }

    fn newer(&self, release: &Release) -> Result<bool, AppError> {
        let current = semver::Version::parse(&self.current_version)
            .map_err(|_| protocol("the compiled CLI version is not a semantic version"))?;
        Ok(release.version.cmp_precedence(&current).is_gt())
    }

    pub(crate) async fn manual(
        &self,
        check_only: bool,
        yes: bool,
        confirmation: &dyn ConfirmationProvider,
    ) -> Result<ManualOutcome, AppError> {
        let release = self.http.latest(MANUAL_TIMEOUT).await?;
        if !self.newer(&release)? {
            return Ok(ManualOutcome::Complete(
                self.result(&release, SelfUpdateStatus::UpToDate),
            ));
        }
        if check_only {
            return Ok(ManualOutcome::Complete(
                self.result(&release, SelfUpdateStatus::Available),
            ));
        }
        if self.installer.method() == InstallationMethod::Unknown {
            return Err(update_failed(format!(
                "installation method is unrecognized or ambiguous; update using your original installation method; release: {}",
                release.url()
            )));
        }
        if !yes && !confirmation.confirm(&self.request(&release))? {
            return Ok(ManualOutcome::Cancelled);
        }
        self.installer.install(&release).await?;
        Ok(ManualOutcome::Complete(
            self.result(&release, SelfUpdateStatus::Updated),
        ))
    }

    /// An installed update stops the command that originally triggered this check.
    pub(crate) async fn automatic(
        &self,
        cache: &mut impl UpdateCache,
        confirmation: &dyn ConfirmationProvider,
    ) -> Result<StartupUpdateOutcome, AppError> {
        let Ok(mut entry) = cache.load() else {
            return Ok(StartupUpdateOutcome::Continue);
        };
        let now = self.clock.now();
        let release = if entry.last_attempt.is_some_and(|then| recent(then, now)) {
            let Some(release) = entry.release.as_ref() else {
                return Ok(StartupUpdateOutcome::Continue);
            };
            release.clone()
        } else {
            entry.last_attempt = Some(now);
            // Clear a stale result before checking, including when offline.
            entry.release = None;
            if cache.save(&entry).is_err() {
                return Ok(StartupUpdateOutcome::Continue);
            }
            let Ok(release) = self.http.latest(AUTOMATIC_TIMEOUT).await else {
                return Ok(StartupUpdateOutcome::Continue);
            };
            entry.release = Some(release.clone());
            if cache.save(&entry).is_err() {
                return Ok(StartupUpdateOutcome::Continue);
            }
            release
        };
        if !self.newer(&release).unwrap_or(false)
            || entry.last_notified.is_some_and(|then| recent(then, now))
        {
            return Ok(StartupUpdateOutcome::Continue);
        }
        entry.last_notified = Some(now);
        if cache.save(&entry).is_err() {
            return Ok(StartupUpdateOutcome::Continue);
        }
        if self.installer.method() == InstallationMethod::Unknown {
            return Ok(StartupUpdateOutcome::Notice(
                self.result(&release, SelfUpdateStatus::Available),
            ));
        }
        if !confirmation.confirm(&self.request(&release))? {
            return Ok(StartupUpdateOutcome::Continue);
        }
        self.installer.install(&release).await?;
        Ok(StartupUpdateOutcome::Updated)
    }

    fn request(&self, release: &Release) -> ConfirmationRequest {
        ConfirmationRequest::new(
            DestructiveOperation::SelfUpdate,
            format!(
                "A new Wekan CLI version is available: {} → {}\nUpdate now?",
                self.current_version, release.version
            ),
        )
    }
}

fn recent(then: u64, now: u64) -> bool {
    now.checked_sub(then)
        .is_some_and(|age| age < CHECK_INTERVAL)
}

fn protocol(message: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::ProtocolError, message, StableExitCode::Transport)
}

fn update_failed(message: impl Into<String>) -> AppError {
    AppError::new(ErrorCode::UpdateFailed, message, StableExitCode::Transport)
}

fn file_error(action: &str, error: impl std::fmt::Display) -> AppError {
    AppError::new(
        ErrorCode::FileIoError,
        format!("could not {action}: {error}"),
        StableExitCode::Transport,
    )
}

#[cfg(test)]
mod tests;
