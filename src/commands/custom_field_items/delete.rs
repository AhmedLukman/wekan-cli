use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{
        CancellationSuccess, CommandSuccess, CustomFieldItemDeleteMode,
        CustomFieldItemDeleteSuccess, DestructiveOperation,
    },
    commands::{
        authenticated::AuthenticatedContext,
        client_error::map_client_error_with_not_found,
        credential_ops::{lock_credential_mutation, map_credential_load_error},
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    input::{
        ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
        confirm_or_skip, escape_terminal_text,
    },
    redaction::Redactor,
};

use super::non_empty;

#[derive(Debug, Args)]
#[command(after_help = "Deletion requires confirmation or --yes")]
pub struct DeleteArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// Wekan custom-field ID to permanently delete.
    #[arg(long = "custom-field", value_parser = non_empty, help_heading = "Target")]
    pub custom_field_id: String,

    /// Wekan dropdown item ID.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    pub item_id: String,

    #[command(flatten)]
    pub confirmation: ConfirmationArgs,
}

pub(super) async fn execute(
    args: DeleteArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let credential_mutation =
        lock_credential_mutation(credential_store, context.credential_target())?;
    let guarded_record = credential_mutation
        .load()
        .map_err(map_credential_load_error)?
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::CredentialNotFound,
                "the credential used to prepare this request is no longer stored",
                StableExitCode::Credential,
            )
        })?;
    if !guarded_record.matches(context.record()) {
        return Err(AppError::new(
            ErrorCode::CredentialStoreFailed,
            "the stored credential changed while preparing custom-field deletion; no HTTP request was sent",
            StableExitCode::Credential,
        )
        .with_credential_stored(true));
    }
    let request = ConfirmationRequest::new(
        DestructiveOperation::CustomFieldItemDelete,
        format!(
            "Permanently delete dropdown item `{}` from custom field `{}` for all associated boards (requested board `{}`)?",
            escape_terminal_text(&args.item_id),
            escape_terminal_text(&args.custom_field_id),
            escape_terminal_text(&args.board_id)
        ),
    );
    if confirm_or_skip(&args.confirmation, confirmation, &request)?
        == ConfirmationDecision::Cancelled
    {
        return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::CustomFieldItemDelete,
        )));
    }

    let redactor = Redactor::with_secret(context.record().token());
    let deletion = context
        .client()
        .delete_custom_field_item(
            &args.board_id,
            &args.custom_field_id,
            &args.item_id,
            context.record().token(),
        )
        .await;
    drop(credential_mutation);
    let deleted = deletion.map_err(|error| {
        map_client_error_with_not_found(
            error,
            &redactor,
            "custom-field deletion",
            "the requested Wekan board or custom field was not found",
        )
    })?;
    Ok(CommandSuccess::CustomFieldItemDeleted(
        CustomFieldItemDeleteSuccess {
            board_id: args.board_id,
            custom_field_id: deleted.id,
            item_id: args.item_id,
            delete_mode: CustomFieldItemDeleteMode::Hard,
        },
    ))
}
