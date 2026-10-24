use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationMethod {
    Release,
    Cargo,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelfUpdateStatus {
    UpToDate,
    Available,
    Updated,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SelfUpdateSuccess {
    /// Version running when the command started, including after replacement.
    pub current_version: String,
    pub latest_version: String,
    pub status: SelfUpdateStatus,
    pub installation_method: InstallationMethod,
    pub release_url: String,
}

/// Startup update checks can continue, display a notice, or stop after installing.
#[derive(Debug)]
pub(crate) enum StartupUpdateOutcome {
    Continue,
    Notice(SelfUpdateSuccess),
    Updated,
}

impl StartupUpdateOutcome {
    pub const fn should_exit(&self) -> bool {
        matches!(self, Self::Updated)
    }
}
