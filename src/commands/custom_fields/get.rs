use super::non_empty;
use crate::{
    client::WekanClientFactory,
    command_result::CommandSuccess,
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    redaction::Redactor,
};
use clap::Args;
#[derive(Debug, Args)]
#[command(next_help_heading = "Fields")]
pub struct GetArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,
    /// Wekan custom-field ID.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    pub custom_field_id: String,
}
pub(super) async fn execute(
    args: GetArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let value = context
        .client()
        .custom_field(
            &args.board_id,
            &args.custom_field_id,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "custom-field lookup",
                "the custom field was not found on that board",
            )
        })?
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::NotFound,
                "the custom field was not found on that board",
                StableExitCode::Server,
            )
            .with_details(crate::commands::client_error::response_error_details(
                reqwest::StatusCode::OK,
                None,
            ))
        })?;
    Ok(CommandSuccess::CustomFieldShown(super::detail(value)))
}
