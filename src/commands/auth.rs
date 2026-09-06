pub mod login;
pub mod logout;
pub mod register;
pub mod status;

use clap::{Args, Subcommand};
use time::format_description::well_known::Rfc3339;

use crate::{
    client::AuthSession,
    command_result::{AuthSuccess, CommandSuccess},
    config::{MissingProfileResolution, ResolvedTarget},
    credentials::{
        CredentialCreateOutcome, CredentialMutation, CredentialRecord, CredentialStore,
        SecretInputProvider,
    },
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    input::ConfirmationProvider,
    redaction::Redactor,
};

use super::credential_ops::map_credential_load_error;

#[derive(Debug, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommand,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Log in to Wekan and securely store the returned token.
    Login(login::LoginArgs),

    /// Revoke Wekan login tokens and remove the stored credential.
    Logout(logout::LogoutArgs),

    /// Register a Wekan account and securely store its login token.
    Register(register::RegisterArgs),

    /// Validate and display the stored authentication session.
    Status(status::StatusArgs),
}

impl AuthCommand {
    pub(crate) const fn missing_profile_resolution(&self) -> MissingProfileResolution {
        match self {
            Self::Login(_) | Self::Register(_) => MissingProfileResolution::Initialize,
            Self::Logout(args) if args.local_only => {
                MissingProfileResolution::LocalCredentialCleanup
            }
            Self::Logout(_) | Self::Status(_) => MissingProfileResolution::Reject,
        }
    }
}

pub(crate) async fn dispatch(
    command: AuthCommand,
    target: &mut ResolvedTarget<'_>,
    credential_store: &dyn CredentialStore,
    secret_input: &dyn SecretInputProvider,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        AuthCommand::Login(args) => {
            login::execute(args, target, credential_store, secret_input).await
        }
        AuthCommand::Register(args) => {
            register::execute(args, target, credential_store, secret_input).await
        }
        AuthCommand::Logout(args) => {
            logout::execute(
                args,
                target.client_factory(),
                credential_store,
                confirmation,
            )
            .await
        }
        AuthCommand::Status(args) => {
            status::execute(args, target.client_factory(), credential_store).await
        }
    }
}

pub(super) fn persist_session(
    target: &mut ResolvedTarget<'_>,
    credential_mutation: &dyn CredentialMutation,
    server_url: String,
    session: AuthSession,
) -> Result<AuthSuccess, AppError> {
    let (user_id, token, token_expires) = session.into_parts();
    let token_expires_text = token_expires.format(&Rfc3339).map_err(|_| {
        AppError::new(
            ErrorCode::InternalError,
            "the validated token expiry could not be formatted",
            StableExitCode::Internal,
        )
    })?;
    let record = CredentialRecord::new(server_url.clone(), user_id.clone(), token, token_expires);
    let token_redactor = Redactor::with_secret(record.token());

    target.initialize_profile()?;
    let profile_created = target.profile_created();
    let profile_active = target.profile_active();
    match credential_mutation.create(&record).map_err(|error| {
        AppError::new(
            ErrorCode::CredentialStoreFailed,
            token_redactor.redact(&error.to_string()),
            StableExitCode::Credential,
        )
        .with_profile_state(profile_created, profile_active)
    })? {
        CredentialCreateOutcome::Created => {}
        CredentialCreateOutcome::AlreadyExists => {
            return Err(AppError::new(
                ErrorCode::CredentialAlreadyExists,
                format!(
                    "profile `{}` acquired a stored credential concurrently; the existing entry was preserved, and the remote session or account may already have been created; inspect Wekan, then run `wekan --server {} --profile {} auth logout --local-only --yes` before authenticating again",
                    target.profile(),
                    server_url,
                    target.profile()
                ),
                StableExitCode::Configuration,
            )
            .with_credential_stored(true)
            .with_profile_state(profile_created, profile_active));
        }
    }

    Ok(AuthSuccess {
        server: server_url,
        profile: target.profile().to_owned(),
        user_id,
        token_expires: token_expires_text,
        credential_stored: true,
        profile_created,
        profile_active,
    })
}

pub(super) fn ensure_credential_absent(
    credential_mutation: &dyn CredentialMutation,
    target: &ResolvedTarget<'_>,
) -> Result<(), AppError> {
    let exists = credential_mutation
        .exists()
        .map_err(map_credential_load_error)?;
    if exists {
        Err(credential_already_exists(target))
    } else {
        Ok(())
    }
}

fn credential_already_exists(target: &ResolvedTarget<'_>) -> AppError {
    AppError::new(
        ErrorCode::CredentialAlreadyExists,
        format!(
            "profile `{}` already has a stored credential; run `wekan --server {} --profile {} auth logout --local-only --yes` before authenticating again",
            target.profile(),
            target.client_factory().server_identity().as_str(),
            target.profile()
        ),
        StableExitCode::Configuration,
    )
    .with_credential_stored(true)
    .with_profile_state(target.profile_created(), target.profile_active())
}

pub(super) fn non_empty_identity(value: &str) -> Result<String, String> {
    if value.is_empty() {
        Err("value must not be empty".to_owned())
    } else {
        Ok(value.to_owned())
    }
}
