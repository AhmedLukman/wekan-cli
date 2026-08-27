pub mod login;
pub mod logout;
pub mod register;
pub mod status;

use clap::{Args, Subcommand};
use reqwest::StatusCode;
use time::format_description::well_known::Rfc3339;

use crate::{
    client::AuthSession,
    command_result::{AuthSuccess, CommandSuccess},
    config::{MissingProfileResolution, ResolvedTarget},
    credentials::{
        CredentialCreateOutcome, CredentialError, CredentialMutation, CredentialRecord,
        CredentialStore, CredentialTarget, SecretInputProvider,
    },
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    input::ConfirmationProvider,
    redaction::Redactor,
};

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
        AuthCommand::Logout(args) => {
            logout::execute(
                args,
                target.client_factory(),
                credential_store,
                confirmation,
            )
            .await
        }
        AuthCommand::Register(args) => {
            register::execute(args, target, credential_store, secret_input).await
        }
        AuthCommand::Status(args) => {
            status::execute(args, target.client_factory(), credential_store).await
        }
    }
}

pub(super) fn map_credential_load_error(error: CredentialError) -> AppError {
    let (code, message) = match error {
        CredentialError::Unavailable(message) => (
            ErrorCode::CredentialStoreUnavailable,
            format!("the operating-system credential store is unavailable: {message}"),
        ),
        error => (
            ErrorCode::CredentialStoreFailed,
            format!("the stored credential could not be loaded: {error}"),
        ),
    };
    AppError::new(code, message, StableExitCode::Credential)
}

pub(super) fn preflight_credentials(
    credential_store: &dyn CredentialStore,
    target: &CredentialTarget,
) -> Result<(), AppError> {
    credential_store.check_available(target).map_err(|error| {
        AppError::new(
            ErrorCode::CredentialStoreUnavailable,
            error.to_string(),
            StableExitCode::Credential,
        )
    })
}

pub(super) fn lock_credential_mutation<'a>(
    credential_store: &'a dyn CredentialStore,
    target: &CredentialTarget,
) -> Result<Box<dyn CredentialMutation + Send + 'a>, AppError> {
    credential_store.lock_mutation(target).map_err(|error| {
        AppError::new(
            ErrorCode::CredentialStoreFailed,
            format!("the stored credential could not be synchronized: {error}"),
            StableExitCode::Credential,
        )
    })
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

pub(super) fn credential_target(
    profile: &str,
    credential_namespace: &str,
    server_url: String,
) -> CredentialTarget {
    CredentialTarget::profile_in_store(profile, credential_namespace, server_url)
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

pub(super) fn protocol_error_details(success_status_received: bool) -> ErrorDetails {
    ErrorDetails {
        http_status: success_status_received.then_some(StatusCode::OK.as_u16()),
        ..ErrorDetails::default()
    }
}

pub(super) fn response_error_details(
    status: StatusCode,
    retry_after_seconds: Option<u64>,
) -> ErrorDetails {
    ErrorDetails {
        http_status: Some(status.as_u16()),
        retry_after_seconds: retry_after_for_status(status, retry_after_seconds),
        ..ErrorDetails::default()
    }
}

fn retry_after_for_status(status: StatusCode, retry_after_seconds: Option<u64>) -> Option<u64> {
    (status == StatusCode::TOO_MANY_REQUESTS)
        .then_some(retry_after_seconds)
        .flatten()
}

pub(super) fn server_error_details(
    status: StatusCode,
    server_error: Option<String>,
    server_reason: Option<String>,
    retry_after_seconds: Option<u64>,
    redactor: &Redactor<'_>,
) -> ErrorDetails {
    let mut details = response_error_details(status, retry_after_seconds);
    details.server_error = server_error.map(|value| redactor.redact(&value));
    details.server_reason = server_reason.map(|value| redactor.redact(&value));
    details
}

pub(super) fn embedded_server_error_details(
    http_status: reqwest::StatusCode,
    wekan_status_code: u16,
    server_error: Option<String>,
    server_reason: Option<String>,
    redactor: &Redactor<'_>,
) -> ErrorDetails {
    ErrorDetails {
        http_status: Some(http_status.as_u16()),
        wekan_status_code: Some(wekan_status_code),
        server_error: server_error.map(|value| redactor.redact(&value)),
        server_reason: server_reason.map(|value| redactor.redact(&value)),
        ..ErrorDetails::default()
    }
}

#[cfg(test)]
mod tests {
    use super::credential_target;

    #[test]
    fn namespaced_profile_factory_creates_an_isolated_credential_target() {
        let target = credential_target("work", "store-one", "https://wekan.example/".to_owned());
        assert_eq!(target.account(), "profile:store-one:work");
    }
}
