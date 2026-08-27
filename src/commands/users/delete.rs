use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{
        CancellationSuccess, CommandSuccess, DestructiveOperation, UserDeleteSuccess,
    },
    commands::{
        authenticated::AuthenticatedContext,
        credential_ops::{lock_credential_mutation, map_credential_load_error},
    },
    credentials::{CredentialDeleteOutcome, CredentialMutation, CredentialStore},
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    input::{
        ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
        confirm_or_skip,
    },
    redaction::Redactor,
};

use super::{map_client_error, non_empty};

#[derive(Debug, Args)]
pub struct DeleteArgs {
    /// Wekan user ID to delete.
    #[arg(value_parser = non_empty)]
    pub user_id: String,

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
    let is_self = args.user_id == context.record().user_id();
    let credential_mutation: Option<Box<dyn CredentialMutation + Send + '_>> = if is_self {
        let mutation = lock_credential_mutation(credential_store, context.credential_target())?;
        let guarded_record = mutation
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
                "the stored credential changed while preparing self-deletion; no HTTP request was sent",
                StableExitCode::Credential,
            )
            .with_credential_stored(true));
        }
        Some(mutation)
    } else {
        None
    };
    let request = ConfirmationRequest::new(
        DestructiveOperation::UserDelete,
        if is_self {
            format!(
                "Delete the currently authenticated Wekan user `{}` and remove its local credential?",
                args.user_id
            )
        } else {
            format!("Delete Wekan user `{}`?", args.user_id)
        },
    );
    if confirm_or_skip(&args.confirmation, confirmation, &request)?
        == ConfirmationDecision::Cancelled
    {
        return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::UserDelete,
        )));
    }

    let redactor = Redactor::with_secret(context.record().token());
    let deleted_id = context
        .client()
        .delete_user(&args.user_id, context.record().token())
        .await
        .map_err(|error| map_client_error(error, &redactor, "user deletion", true))?;
    if deleted_id != args.user_id {
        return Err(AppError::new(
            ErrorCode::ProtocolError,
            "Wekan reported a different deleted user ID than requested",
            StableExitCode::Server,
        ));
    }

    let deletion = if is_self {
        credential_mutation
            .as_ref()
            .expect("self-delete holds a credential mutation guard")
            .delete_if_matches(context.record())
            .map_err(|error| {
                AppError::new(
                    ErrorCode::CredentialStoreFailed,
                    redactor.redact(&format!(
                        "the Wekan user was deleted, but its local credential could not be removed: {error}"
                    )),
                    StableExitCode::Credential,
                )
                .with_credential_stored(true)
                .with_user_deleted(true)
            })?
    } else {
        CredentialDeleteOutcome::Mismatch
    };
    Ok(CommandSuccess::UserDeleted(UserDeleteSuccess {
        user_id: args.user_id,
        deleted: true,
        deleted_current_user: is_self,
        credential_stored: if is_self {
            deletion == CredentialDeleteOutcome::Mismatch
        } else {
            true
        },
        local_credential_removed: is_self && deletion == CredentialDeleteOutcome::Removed,
    }))
}
