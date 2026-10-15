use crate::{
    client::WekanClientFactory,
    command_result::{
        CancellationSuccess, CommandSuccess, DestructiveOperation, RuleDeleteSuccess,
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
    after_help = "Deletion requires confirmation or --yes and removes the rule together with its referenced trigger/action. Missing or wrongly scoped rules retain Wekan v11.06 HTTP 500 errors."
)]
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
    let context = AuthenticatedContext::load(factory, store)?;
    let guard = lock_credential_mutation(store, context.credential_target())?;
    let request = ConfirmationRequest::new(
        DestructiveOperation::RuleDelete,
        format!(
            "Delete Wekan rule `{}` on board `{}`?",
            escape_terminal_text(&args.target.rule_id),
            escape_terminal_text(&args.target.board_id)
        ),
    );
    if confirm_or_skip(&args.confirmation, confirmation, &request)?
        == ConfirmationDecision::Cancelled
    {
        return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::RuleDelete,
        )));
    }
    let record = guard.load().map_err(map_credential_load_error)?.ok_or_else(|| AppError::new(
        ErrorCode::CredentialNotFound, "the credential used to prepare rule deletion is no longer stored; no HTTP request was sent", StableExitCode::Credential))?;
    if !record.matches(context.record()) {
        return Err(AppError::new(
            ErrorCode::CredentialStoreFailed,
            "the stored credential changed while preparing rule deletion; no HTTP request was sent",
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
    let result = context
        .client()
        .delete_rule(
            &args.target.board_id,
            &args.target.rule_id,
            context.record().token(),
        )
        .await;
    drop(guard);
    let response = result.map_err(|e| {
        map_mutation_client_error_with_not_found(
            e,
            &Redactor::with_secret(context.record().token()),
            "rule deletion",
            "the Wekan board was not found",
        )
    })?;
    Ok(CommandSuccess::RuleDeleted(RuleDeleteSuccess {
        board_id: args.target.board_id,
        rule_id: response.rule_id,
        deleted: true,
    }))
}
