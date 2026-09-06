use clap::Args;

use crate::{
    client::{UpdateSwimlaneRequest, WekanClientFactory},
    command_result::{CommandSuccess, SwimlaneUpdateSuccess, SwimlaneUpdatedField},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::{map_mutation_client_error_with_not_found, protocol_error_details},
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    redaction::Redactor,
};

use super::{non_empty, trimmed_non_empty};

#[derive(Debug, Args)]
#[command(
    after_help = "Example:\n  wekan swimlane update swimlane-id --board board-id --title \"Plan next release\""
)]
#[command(next_help_heading = "Fields")]
pub struct UpdateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// Wekan swimlane ID.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    pub swimlane_id: String,

    /// New swimlane title.
    #[arg(long, value_parser = trimmed_non_empty)]
    pub title: String,
}

pub(super) async fn execute(
    args: UpdateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let updated = context
        .client()
        .update_swimlane(
            &args.board_id,
            &args.swimlane_id,
            &UpdateSwimlaneRequest { title: args.title },
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_mutation_client_error_with_not_found(
                error,
                &redactor,
                "swimlane update",
                "the requested Wekan swimlane was not found on that board",
            )
        })?;
    if updated.swimlane_id != args.swimlane_id {
        let mut details = protocol_error_details(true);
        details.outcome_unknown = Some(true);
        return Err(AppError::new(
            ErrorCode::ProtocolError,
            "Wekan reported a different updated swimlane ID than requested",
            StableExitCode::Transport,
        )
        .with_details(details));
    }
    Ok(CommandSuccess::SwimlaneUpdated(SwimlaneUpdateSuccess {
        board_id: args.board_id,
        swimlane_id: updated.swimlane_id,
        updated_fields: vec![SwimlaneUpdatedField::Title],
    }))
}
