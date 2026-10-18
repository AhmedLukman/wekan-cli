use crate::{
    client::{WekanClientFactory, users::tokens::DeleteUserTokenRequest},
    command_result::{
        CancellationSuccess, CommandSuccess, DestructiveOperation, UserTokenDeleteScope,
        UserTokenDeleteSuccess,
    },
    commands::{
        authenticated::AuthenticatedContext,
        client_error::map_client_error,
        credential_ops::{lock_credential_mutation, map_credential_load_error},
    },
    credentials::{CredentialStore, SecretInputProvider},
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    input::{
        ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
        confirm_or_skip, escape_terminal_text,
    },
    redaction::Redactor,
};
use clap::Args;
use secrecy::{ExposeSecret, SecretString};
use time::OffsetDateTime;

#[derive(Debug, Args)]
#[command(
    after_help = "The native v11.06 operation receives the token unchanged. Local credentials are retained."
)]
pub struct DeleteArgs {
    /// Wekan user ID whose tokens should be deleted.
    #[arg(value_parser = super::super::non_empty)]
    user_id: String,
    /// Read the target token from UTF-8 stdin, removing one final LF or CRLF.
    #[arg(long, required_unless_present = "all", conflicts_with = "all")]
    token_stdin: bool,
    /// Request deletion of every token for this user.
    #[arg(long)]
    all: bool,
    #[command(flatten)]
    confirmation: ConfirmationArgs,
}

pub(super) async fn execute(
    args: DeleteArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    secrets: &dyn SecretInputProvider,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    // Deleting all of our sessions must not revoke a replacement credential
    // installed by a concurrent logout/login while confirmation is pending.
    let guard = if args.all && args.user_id == context.record().user_id() {
        Some(lock_credential_mutation(
            store,
            context.credential_target(),
        )?)
    } else {
        None
    };
    let prompt = ConfirmationRequest::new(
        DestructiveOperation::UserTokenDelete,
        format!(
            "Request deletion of {} for Wekan user `{}`?",
            if args.all {
                "all tokens"
            } else {
                "the supplied token"
            },
            escape_terminal_text(&args.user_id)
        ),
    );
    if confirm_or_skip(&args.confirmation, confirmation, &prompt)?
        == ConfirmationDecision::Cancelled
    {
        return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::UserTokenDelete,
        )));
    }
    let token = if args.token_stdin {
        let raw = secrets.read_raw_secret()?;
        let value = raw.expose_secret();
        let value = value
            .strip_suffix("\r\n")
            .or_else(|| value.strip_suffix('\n'))
            .unwrap_or(value);
        if value.is_empty() {
            return Err(AppError::invalid_input(
                "token stdin must not be empty; use --all to delete all tokens; no request was sent",
            ));
        }
        Some(SecretString::from(value.to_owned()))
    } else {
        None
    };
    if let Some(guard) = &guard {
        let record = guard
            .load()
            .map_err(map_credential_load_error)?
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::CredentialNotFound,
                    "the credential used to prepare token deletion is no longer stored; no HTTP request was sent",
                    StableExitCode::Credential,
                )
            })?;
        if !record.matches(context.record()) {
            return Err(AppError::new(
                ErrorCode::CredentialStoreFailed,
                "the stored credential changed while preparing token deletion; no HTTP request was sent",
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
    }
    let request = DeleteUserTokenRequest {
        user_id: args.user_id,
        token,
    };
    let mut redactor = Redactor::with_secret(context.record().token());
    if let Some(token) = &request.token {
        redactor = redactor.and_secret(token);
    }
    let response = context
        .client()
        .delete_user_token(&request, context.record().token())
        .await;
    drop(guard);
    let response = response.map_err(|e| map_client_error(e, &redactor, "token deletion"))?;
    Ok(CommandSuccess::UserTokenDeleted(UserTokenDeleteSuccess {
        user_id: request.user_id.clone(),
        scope: if args.all {
            UserTokenDeleteScope::All
        } else {
            UserTokenDeleteScope::Single
        },
        message: redactor.redact(response.message.expose_secret()),
    }))
}
