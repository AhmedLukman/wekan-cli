use crate::{
    client::WekanClientFactory,
    command_result::{
        CancellationSuccess, CommandSuccess, DependencyDeleteSuccess, DestructiveOperation,
    },
    commands::{
        authenticated::AuthenticatedContext,
        client_error::map_mutation_client_error_with_not_found,
        credential_ops::{lock_credential_mutation, map_credential_load_error},
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    input::{
        ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
        confirm_or_skip, escape_terminal_text,
    },
    redaction::Redactor,
};
use clap::Args;
use time::OffsetDateTime;

#[derive(Debug, Args)]
#[command(
    after_help = "Removal requires confirmation or --yes. Wekan can acknowledge a missing edge; use dependency list to inspect the result."
)]
pub struct DeleteArgs {
    #[command(flatten)]
    target: super::MutationTarget,
    #[command(flatten)]
    confirmation: ConfirmationArgs,
}

pub(super) async fn execute(
    args: DeleteArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let guard = lock_credential_mutation(store, context.credential_target())?;
    let request = ConfirmationRequest::new(
        DestructiveOperation::DependencyDelete,
        format!(
            "Remove the dependency from card `{}` to card `{}` on board `{}`?",
            escape_terminal_text(&args.target.card_id),
            escape_terminal_text(&args.target.target_card_id),
            escape_terminal_text(&args.target.board_id)
        ),
    );
    if confirm_or_skip(&args.confirmation, confirmation, &request)?
        == ConfirmationDecision::Cancelled
    {
        return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::DependencyDelete,
        )));
    }
    let record = guard.load().map_err(map_credential_load_error)?.ok_or_else(|| AppError::new(
        ErrorCode::CredentialNotFound, "the credential used to prepare dependency deletion is no longer stored; no HTTP request was sent", StableExitCode::Credential))?;
    if !record.matches(context.record()) {
        return Err(AppError::new(ErrorCode::CredentialStoreFailed, "the stored credential changed while preparing dependency deletion; no HTTP request was sent", StableExitCode::Credential).with_credential_stored(true));
    }
    if record.token_expires() <= OffsetDateTime::now_utc() {
        return Err(AppError::new(
            ErrorCode::CredentialExpired,
            format!(
                "the stored credential expired at {}; no HTTP request was sent",
                context.token_expires()
            ),
            StableExitCode::Server,
        )
        .with_details(ErrorDetails {
            token_expires: Some(context.token_expires().to_owned()),
            ..ErrorDetails::default()
        }));
    }
    let response = context
        .client()
        .delete_dependency(
            &args.target.board_id,
            &args.target.card_id,
            &args.target.target_card_id,
            context.record().token(),
        )
        .await;
    drop(guard);
    let response = response.map_err(|error| {
        map_mutation_client_error_with_not_found(
            error,
            &Redactor::with_secret(context.record().token()),
            "dependency deletion",
            "the Wekan board or source card was not found",
        )
    })?;
    Ok(CommandSuccess::DependencyDeleted(DependencyDeleteSuccess {
        board_id: args.target.board_id,
        card_id: response.card_id,
        target_card_id: args.target.target_card_id,
        deletion_requested: true,
        mutation_confirmed: false,
    }))
}
