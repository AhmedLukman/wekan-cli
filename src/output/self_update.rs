use std::io::Write;

use crate::{
    command_result::{
        InstallationMethod, SelfUpdateStatus, SelfUpdateSuccess, StartupUpdateOutcome,
    },
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
};

const UPDATED: &str = "Updated successfully; rerun your command.";

pub(super) fn render(data: &SelfUpdateSuccess) -> String {
    match data.status {
        SelfUpdateStatus::UpToDate => {
            format!("Wekan CLI is up to date ({}).", data.current_version)
        }
        SelfUpdateStatus::Updated => UPDATED.to_owned(),
        SelfUpdateStatus::Available => {
            let mut message = format!(
                "A new Wekan CLI version is available: {} → {}\n{}",
                data.current_version, data.latest_version, data.release_url
            );
            if data.installation_method == InstallationMethod::Unknown {
                message.push_str("\nInstallation method is unrecognized; update using your original installation method.");
            } else {
                message.push_str("\nRun `wekan self update` to install.");
            }
            message
        }
    }
}

pub(crate) fn write_update_notice(
    outcome: &StartupUpdateOutcome,
    output: &mut dyn Write,
) -> Result<(), AppError> {
    match outcome {
        StartupUpdateOutcome::Continue => {}
        StartupUpdateOutcome::Notice(data) => {
            // An optional notice must not prevent the requested command.
            let _ = writeln!(
                output,
                "A new Wekan CLI version is available: {} → {}\nInstallation method is unrecognized; update using your original installation method.\n{}",
                data.current_version, data.latest_version, data.release_url
            );
        }
        StartupUpdateOutcome::Updated => {
            writeln!(output, "{UPDATED}").map_err(|error| {
                AppError::new(
                    ErrorCode::FileIoError,
                    format!("could not write update confirmation: {error}"),
                    StableExitCode::Transport,
                )
            })?;
        }
    }
    Ok(())
}
