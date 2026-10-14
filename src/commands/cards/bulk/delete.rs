use crate::{
    client::WekanClientFactory,
    command_result::{
        BulkCardDeleteResult, CancellationSuccess, CardBatchResult, CommandSuccess,
        DestructiveOperation,
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
pub struct DeleteArgs {
    #[command(flatten)]
    target: super::Target,
    #[command(flatten)]
    confirmation: ConfirmationArgs,
}

pub(super) async fn execute(
    args: DeleteArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    super::validate_count(args.target.card_ids.len())?;
    let context = AuthenticatedContext::load(factory, store)?;
    let guard = lock_credential_mutation(store, context.credential_target())?;
    let prompt = ConfirmationRequest::new(
        DestructiveOperation::CardBulkDelete,
        format!(
            "Permanently delete {} requested cards [{}] on board `{}` with their comments, checklists, checklist items, and subcards? Wekan v11.06 retains attachment metadata and file bytes.",
            args.target.card_ids.len(),
            args.target
                .card_ids
                .iter()
                .map(|id| escape_terminal_text(id))
                .collect::<Vec<_>>()
                .join(", "),
            escape_terminal_text(&args.target.board_id)
        ),
    );
    if confirm_or_skip(&args.confirmation, confirmation, &prompt)?
        == ConfirmationDecision::Cancelled
    {
        return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::CardBulkDelete,
        )));
    }
    let record = guard
        .load()
        .map_err(map_credential_load_error)?
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::CredentialNotFound,
                "the credential used to prepare bulk card deletion is no longer stored; no HTTP request was sent",
                StableExitCode::Credential,
            )
        })?;
    if !record.matches(context.record()) {
        return Err(AppError::new(
            ErrorCode::CredentialStoreFailed,
            "the stored credential changed while preparing bulk card deletion; no HTTP request was sent",
            StableExitCode::Credential,
        )
        .with_credential_stored(true));
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
    let redactor = Redactor::with_secret(record.token());
    let result = context
        .client()
        .bulk_delete_cards(&args.target.board_id, &args.target.card_ids, record.token())
        .await;
    drop(guard);
    let result = result.map_err(|error| {
        map_mutation_client_error_with_not_found(
            error,
            &redactor,
            "bulk card deletion",
            "the requested Wekan board was not found",
        )
    })?;
    super::finish(CardBatchResult::CardBulkDelete(BulkCardDeleteResult {
        board_id: args.target.board_id,
        deleted: result.deleted,
        not_found: result.not_found,
    }))
}
