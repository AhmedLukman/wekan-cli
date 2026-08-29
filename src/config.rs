pub mod profiles;

use std::env;

use crate::{
    client::{ServerUrl, WekanClientFactory},
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
};

use self::profiles::{
    Profile, ProfileMutation, ProfileSnapshot, ProfileStore, canonical_server,
    validate_profile_name,
};

const DEFAULT_PROFILE: &str = "default";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MissingProfileResolution {
    Reject,
    Initialize,
    LocalCredentialCleanup,
}

impl MissingProfileResolution {
    pub(crate) const fn allows_initialization(self) -> bool {
        matches!(self, Self::Initialize)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ServerSelection {
    pub explicit_server: Option<String>,
    pub explicit_profile: Option<String>,
    pub allow_insecure_http: bool,
}

impl ServerSelection {
    pub fn new(
        explicit_server: Option<String>,
        explicit_profile: Option<String>,
        allow_insecure_http: bool,
    ) -> Self {
        Self {
            explicit_server,
            explicit_profile,
            allow_insecure_http,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct ServerEnvironment {
    server: Option<String>,
    profile: Option<String>,
}

impl ServerEnvironment {
    fn read(selection: &ServerSelection) -> Result<Self, AppError> {
        Ok(Self {
            server: if selection.explicit_server.is_some() {
                None
            } else {
                read_unicode_environment("WEKAN_URL")?
            },
            profile: if selection.explicit_profile.is_some() {
                None
            } else {
                read_unicode_environment("WEKAN_PROFILE")?
            },
        })
    }
}

fn read_unicode_environment(name: &str) -> Result<Option<String>, AppError> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(AppError::configuration(format!(
            "{name} must contain valid Unicode"
        ))),
    }
}

#[derive(Clone, Debug)]
pub struct TargetResolver {
    selection: ServerSelection,
}

impl TargetResolver {
    pub const fn new(selection: ServerSelection) -> Self {
        Self { selection }
    }

    pub fn has_explicit_target(&self) -> bool {
        self.selection.explicit_server.is_some() || self.selection.explicit_profile.is_some()
    }

    pub(crate) fn resolve<'a>(
        &self,
        profile_store: &'a dyn ProfileStore,
        missing_profile_resolution: MissingProfileResolution,
    ) -> Result<ResolvedTarget<'a>, AppError> {
        self.resolve_with_environment(
            profile_store,
            ServerEnvironment::read(&self.selection)?,
            missing_profile_resolution,
        )
    }

    fn resolve_with_environment<'a>(
        &self,
        profile_store: &'a dyn ProfileStore,
        environment: ServerEnvironment,
        missing_profile_resolution: MissingProfileResolution,
    ) -> Result<ResolvedTarget<'a>, AppError> {
        let selected_name = self.selected_name_before_active(&environment)?;
        let snapshot = profile_store
            .read()
            .map_err(AppError::from)
            .map_err(|error| with_optional_target_context(error, selected_name.clone()))?;
        let name = selected_name.unwrap_or_else(|| {
            snapshot
                .document()
                .active_profile()
                .unwrap_or(DEFAULT_PROFILE)
                .to_owned()
        });
        let supplied_server = self
            .selection
            .explicit_server
            .as_ref()
            .or(environment.server.as_ref())
            .map(|raw| canonical_server(raw))
            .transpose()
            .map_err(AppError::from)
            .map_err(|error| with_target_context(error, &name))?;
        let credential_namespace = profile_store
            .credential_namespace()
            .map_err(AppError::from)
            .map_err(|error| with_target_context(error, &name))?;

        if let Some(profile) = snapshot.document().profile(&name) {
            let server = verified_server(&name, profile.server(), supplied_server.as_deref())?;
            let active = snapshot.document().active_profile() == Some(name.as_str());
            return Ok(ResolvedTarget::existing(
                server,
                name,
                credential_namespace,
                active,
                self.selection.allow_insecure_http,
                snapshot,
            ));
        }

        let Some(server) = supplied_server else {
            return Err(profile_not_found(&name));
        };

        match missing_profile_resolution {
            MissingProfileResolution::Reject => return Err(profile_not_found(&name)),
            MissingProfileResolution::LocalCredentialCleanup => {
                let server = ServerUrl::parse(&server)
                    .map_err(AppError::from)
                    .map_err(|error| with_target_context(error, &name))?;
                return Ok(ResolvedTarget::missing_profile_cleanup(
                    server,
                    name,
                    credential_namespace,
                    self.selection.allow_insecure_http,
                    snapshot,
                ));
            }
            MissingProfileResolution::Initialize => {}
        }

        drop(snapshot);
        let mutation = profile_store
            .lock_mutation()
            .map_err(AppError::from)
            .map_err(|error| with_target_context(error, &name))?;
        if let Some(profile) = mutation.document().profile(&name) {
            let server = verified_server(&name, profile.server(), Some(&server))?;
            let active = mutation.document().active_profile() == Some(name.as_str());
            return Ok(ResolvedTarget::under_mutation(
                server,
                name,
                credential_namespace,
                active,
                false,
                self.selection.allow_insecure_http,
                mutation,
            ));
        }

        let server = ServerUrl::parse(&server)
            .map_err(AppError::from)
            .map_err(|error| with_target_context(error, &name))?;
        Ok(ResolvedTarget::under_mutation(
            server,
            name,
            credential_namespace,
            false,
            true,
            self.selection.allow_insecure_http,
            mutation,
        ))
    }

    fn selected_name_before_active(
        &self,
        environment: &ServerEnvironment,
    ) -> Result<Option<String>, AppError> {
        if let Some(name) = &self.selection.explicit_profile {
            validate_profile_name(name)
                .map_err(AppError::invalid_input)
                .map_err(|error| with_target_context(error, name))?;
            return Ok(Some(name.clone()));
        }
        if let Some(name) = &environment.profile {
            validate_profile_name(name)
                .map_err(|message| {
                    AppError::configuration(format!("WEKAN_PROFILE is invalid: {message}"))
                })
                .map_err(|error| with_target_context(error, name))?;
            return Ok(Some(name.clone()));
        }
        Ok(None)
    }
}

fn verified_server(
    name: &str,
    stored_server: &str,
    supplied_server: Option<&str>,
) -> Result<ServerUrl, AppError> {
    if supplied_server.is_some_and(|supplied| supplied != stored_server) {
        return Err(AppError::new(
            ErrorCode::ProfileServerMismatch,
            format!(
                "the supplied server URL does not match profile `{name}`; the stored profile was not changed"
            ),
            StableExitCode::Configuration,
        )
        .with_details(ErrorDetails {
            profile: Some(name.to_owned()),
            ..ErrorDetails::default()
        }));
    }
    ServerUrl::parse(stored_server)
        .map_err(AppError::from)
        .map_err(|error| with_target_context(error, name))
}

fn profile_not_found(name: &str) -> AppError {
    AppError::new(
        ErrorCode::ProfileNotFound,
        format!(
            "profile `{name}` does not exist; add it with `wekan profile add`, supply --server to auth login or auth register, or run `wekan --server <URL> --profile {name} auth logout --local-only --yes` to clear an orphaned local credential"
        ),
        StableExitCode::Configuration,
    )
    .with_details(ErrorDetails {
        profile: Some(name.to_owned()),
        ..ErrorDetails::default()
    })
}

fn with_target_context(error: AppError, profile: &str) -> AppError {
    if error.details().profile.is_some() {
        error
    } else {
        error.with_profile_context(profile.to_owned())
    }
}

fn with_optional_target_context(error: AppError, profile: Option<String>) -> AppError {
    match profile {
        Some(profile) => with_target_context(error, &profile),
        None => error,
    }
}

enum ProfileGuard<'a> {
    Existing {
        _snapshot: ProfileSnapshot,
    },
    Mutation {
        mutation: Box<dyn ProfileMutation + Send + 'a>,
        needs_creation: bool,
    },
}

pub struct ResolvedTarget<'a> {
    client_factory: WekanClientFactory,
    profile: String,
    credential_namespace: String,
    profile_created: bool,
    profile_active: bool,
    guard: ProfileGuard<'a>,
}

impl<'a> ResolvedTarget<'a> {
    fn existing(
        server: ServerUrl,
        profile: String,
        credential_namespace: String,
        active: bool,
        allow_insecure_http: bool,
        snapshot: ProfileSnapshot,
    ) -> Self {
        Self::with_snapshot(
            server,
            profile,
            credential_namespace,
            active,
            allow_insecure_http,
            snapshot,
        )
    }

    fn missing_profile_cleanup(
        server: ServerUrl,
        profile: String,
        credential_namespace: String,
        allow_insecure_http: bool,
        snapshot: ProfileSnapshot,
    ) -> Self {
        Self::with_snapshot(
            server,
            profile,
            credential_namespace,
            false,
            allow_insecure_http,
            snapshot,
        )
    }

    fn with_snapshot(
        server: ServerUrl,
        profile: String,
        credential_namespace: String,
        active: bool,
        allow_insecure_http: bool,
        snapshot: ProfileSnapshot,
    ) -> Self {
        Self {
            client_factory: WekanClientFactory::for_resolved_profile(
                server,
                profile.clone(),
                credential_namespace.clone(),
                allow_insecure_http,
            ),
            profile,
            credential_namespace,
            profile_created: false,
            profile_active: active,
            guard: ProfileGuard::Existing {
                _snapshot: snapshot,
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn under_mutation(
        server: ServerUrl,
        profile: String,
        credential_namespace: String,
        active: bool,
        needs_creation: bool,
        allow_insecure_http: bool,
        mutation: Box<dyn ProfileMutation + Send + 'a>,
    ) -> Self {
        Self {
            client_factory: WekanClientFactory::for_resolved_profile(
                server,
                profile.clone(),
                credential_namespace.clone(),
                allow_insecure_http,
            ),
            profile,
            credential_namespace,
            profile_created: false,
            profile_active: active,
            guard: ProfileGuard::Mutation {
                mutation,
                needs_creation,
            },
        }
    }

    pub const fn client_factory(&self) -> &WekanClientFactory {
        &self.client_factory
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    pub fn credential_namespace(&self) -> &str {
        &self.credential_namespace
    }

    pub const fn profile_created(&self) -> bool {
        self.profile_created
    }

    pub const fn profile_active(&self) -> bool {
        self.profile_active
    }

    pub fn initialize_profile(&mut self) -> Result<(), AppError> {
        let profile = self.profile.clone();
        let ProfileGuard::Mutation {
            mutation,
            needs_creation,
        } = &mut self.guard
        else {
            return Ok(());
        };
        if !*needs_creation {
            return Ok(());
        }

        let mut document = mutation.document().clone();
        let active = document.is_empty();
        document.insert(
            self.profile.clone(),
            Profile::new(self.client_factory.server_identity().as_str().to_owned()),
        );
        if active {
            document.set_active_profile(Some(self.profile.clone()));
        }
        if let Err(error) = mutation.save(document) {
            if error.profile_was_installed() {
                *needs_creation = false;
                self.profile_created = true;
                self.profile_active = active;
            }
            return Err(with_target_context(AppError::from(error), &profile)
                .with_profile_state(self.profile_created, self.profile_active));
        }
        *needs_creation = false;
        self.profile_created = true;
        self.profile_active = active;
        Ok(())
    }
}

#[cfg(test)]
impl ResolvedTarget<'static> {
    pub(crate) fn for_test(client_factory: &WekanClientFactory) -> Self {
        Self {
            client_factory: WekanClientFactory::for_resolved_profile(
                client_factory.server_identity().clone(),
                client_factory
                    .profile()
                    .unwrap_or(DEFAULT_PROFILE)
                    .to_owned(),
                client_factory
                    .profile_store_namespace()
                    .unwrap_or("test-store")
                    .to_owned(),
                false,
            ),
            profile: client_factory
                .profile()
                .unwrap_or(DEFAULT_PROFILE)
                .to_owned(),
            credential_namespace: client_factory
                .profile_store_namespace()
                .unwrap_or("test-store")
                .to_owned(),
            profile_created: false,
            profile_active: false,
            guard: ProfileGuard::Existing {
                _snapshot: ProfileSnapshot::new_unlocked(Default::default()),
            },
        }
    }
}

#[cfg(test)]
mod tests;
