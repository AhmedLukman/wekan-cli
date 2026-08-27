use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    client::{WekanClient, WekanClientFactory},
    credentials::{CredentialRecord, CredentialStore, CredentialTarget},
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
};

use super::credential_ops::{credential_target, map_credential_load_error, preflight_credentials};

pub(crate) struct AuthenticatedContext {
    client: WekanClient,
    profile: String,
    server: String,
    credential_target: CredentialTarget,
    record: CredentialRecord,
    token_expires: String,
}

impl AuthenticatedContext {
    pub(crate) fn load(
        client_factory: &WekanClientFactory,
        credential_store: &dyn CredentialStore,
    ) -> Result<Self, AppError> {
        let client = client_factory.create()?;
        let server = client.server().as_str().to_owned();
        let profile = client_factory
            .profile()
            .expect("authenticated commands require a named profile")
            .to_owned();
        let credential_target = credential_target(
            &profile,
            client_factory
                .profile_store_namespace()
                .expect("authenticated commands require a profile credential namespace"),
            server.clone(),
        );
        preflight_credentials(credential_store, &credential_target)?;
        let record = credential_store
            .load(&credential_target)
            .map_err(map_credential_load_error)?
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::CredentialNotFound,
                    "no credential is stored for this Wekan profile; run `wekan auth login` or `wekan auth register`",
                    StableExitCode::Server,
                )
            })?;
        let token_expires = record.token_expires().format(&Rfc3339).map_err(|_| {
            AppError::new(
                ErrorCode::InternalError,
                "the validated credential expiry could not be formatted",
                StableExitCode::Internal,
            )
        })?;
        if record.token_expires() <= OffsetDateTime::now_utc() {
            return Err(AppError::new(
                ErrorCode::CredentialExpired,
                format!(
                    "the stored credential expired at {token_expires}; remove it with `wekan auth logout --local-only`, then log in again"
                ),
                StableExitCode::Server,
            )
            .with_details(ErrorDetails {
                token_expires: Some(token_expires),
                ..ErrorDetails::default()
            }));
        }
        Ok(Self {
            client,
            profile,
            server,
            credential_target,
            record,
            token_expires,
        })
    }

    pub(crate) const fn client(&self) -> &WekanClient {
        &self.client
    }

    pub(crate) fn profile(&self) -> &str {
        &self.profile
    }

    pub(crate) fn server(&self) -> &str {
        &self.server
    }

    pub(crate) const fn credential_target(&self) -> &CredentialTarget {
        &self.credential_target
    }

    pub(crate) const fn record(&self) -> &CredentialRecord {
        &self.record
    }

    pub(crate) fn token_expires(&self) -> &str {
        &self.token_expires
    }
}
