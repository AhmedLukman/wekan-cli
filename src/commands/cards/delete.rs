use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{
        CancellationSuccess, CardDeleteMode, CardDeleteSuccess, CommandSuccess,
        DestructiveOperation,
    },
    commands::{
        authenticated::AuthenticatedContext,
        client_error::{map_client_error, protocol_error_details},
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
pub struct DeleteArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    pub board_id: String,

    /// Wekan list ID. Supplying the wrong list can trigger a Wekan v11.06 cascade defect.
    #[arg(long = "list", value_parser = non_empty)]
    pub list_id: String,

    /// Wekan card ID to permanently delete.
    #[arg(value_parser = non_empty)]
    pub card_id: String,

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
            "the stored credential changed while preparing card deletion; no HTTP request was sent",
            StableExitCode::Credential,
        )
        .with_credential_stored(true));
    }
    let request = ConfirmationRequest::new(
        DestructiveOperation::CardDelete,
        format!(
            "Permanently delete Wekan card `{}` from list `{}` on board `{}` and cascade its child resources?",
            escape_terminal_text(&args.card_id),
            escape_terminal_text(&args.list_id),
            escape_terminal_text(&args.board_id)
        ),
    );
    if confirm_or_skip(&args.confirmation, confirmation, &request)?
        == ConfirmationDecision::Cancelled
    {
        return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::CardDelete,
        )));
    }

    let redactor = Redactor::with_secret(context.record().token());
    let deletion = context
        .client()
        .delete_card(
            &args.board_id,
            &args.list_id,
            &args.card_id,
            context.record().token(),
        )
        .await;
    drop(credential_mutation);
    let deleted =
        deletion.map_err(|error| map_client_error(error, &redactor, "card deletion", true))?;
    if deleted.card_id != args.card_id {
        let mut details = protocol_error_details(true);
        details.outcome_unknown = Some(true);
        return Err(AppError::new(
            ErrorCode::ProtocolError,
            "Wekan reported a different deleted card ID than requested",
            StableExitCode::Transport,
        )
        .with_details(details));
    }
    Ok(CommandSuccess::CardDeleted(CardDeleteSuccess {
        board_id: args.board_id,
        list_id: args.list_id,
        card_id: deleted.card_id,
        deleted: true,
        delete_mode: CardDeleteMode::Hard,
    }))
}
