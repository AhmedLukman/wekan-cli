use crate::{
    credentials::{CredentialError, CredentialMutation, CredentialStore, CredentialTarget},
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
};

pub(crate) fn map_credential_load_error(error: CredentialError) -> AppError {
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

pub(crate) fn preflight_credentials(
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

pub(crate) fn lock_credential_mutation<'a>(
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

pub(crate) fn credential_target(
    profile: &str,
    credential_namespace: &str,
    server_url: String,
) -> CredentialTarget {
    CredentialTarget::profile_in_store(profile, credential_namespace, server_url)
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
