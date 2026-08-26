pub mod profiles;

use std::env;

use crate::{
    client::{ServerUrl, WekanClientFactory},
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
};

use self::profiles::{ProfileSnapshot, ProfileStore, validate_profile_name};

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

    pub fn direct(server: Option<String>, allow_insecure_http: bool) -> Self {
        Self::new(server, None, allow_insecure_http)
    }
}

#[derive(Clone, Debug, Default)]
struct ServerEnvironment {
    server: Option<String>,
    profile: Option<String>,
}

impl ServerEnvironment {
    fn read() -> Result<Self, AppError> {
        let server = read_unicode_environment("WEKAN_URL")?;
        let profile = if server.is_some() {
            None
        } else {
            read_unicode_environment("WEKAN_PROFILE")?
        };
        Ok(Self { server, profile })
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

    pub fn resolve(&self, profile_store: &dyn ProfileStore) -> Result<ResolvedTarget, AppError> {
        if let Some(target) = self.resolve_explicit(profile_store)? {
            return Ok(target);
        }

        let environment = ServerEnvironment::read()?;
        self.resolve_environment_or_active(profile_store, environment)
    }

    #[cfg(test)]
    fn resolve_with_environment(
        &self,
        profile_store: &dyn ProfileStore,
        environment: ServerEnvironment,
    ) -> Result<ResolvedTarget, AppError> {
        if let Some(target) = self.resolve_explicit(profile_store)? {
            return Ok(target);
        }

        self.resolve_environment_or_active(profile_store, environment)
    }

    fn resolve_explicit(
        &self,
        profile_store: &dyn ProfileStore,
    ) -> Result<Option<ResolvedTarget>, AppError> {
        if self.selection.explicit_server.is_some() && self.selection.explicit_profile.is_some() {
            return Err(with_target_context(
                AppError::invalid_input("--server cannot be used with --profile"),
                self.selection.explicit_profile.clone(),
            ));
        }

        if let Some(server) = &self.selection.explicit_server {
            return Ok(Some(ResolvedTarget::direct(
                ServerUrl::parse(server)
                    .map_err(AppError::from)
                    .map_err(|error| with_target_context(error, None))?,
                self.selection.allow_insecure_http,
            )));
        }
        if let Some(profile) = &self.selection.explicit_profile {
            return self
                .resolve_named(profile, profile_store, "--profile")
                .map(Some);
        }

        Ok(None)
    }

    fn resolve_environment_or_active(
        &self,
        profile_store: &dyn ProfileStore,
        environment: ServerEnvironment,
    ) -> Result<ResolvedTarget, AppError> {
        if let Some(server) = &environment.server {
            return Ok(ResolvedTarget::direct(
                ServerUrl::parse(server)
                    .map_err(AppError::from)
                    .map_err(|error| with_target_context(error, None))?,
                self.selection.allow_insecure_http,
            ));
        }
        if let Some(profile) = &environment.profile {
            return self.resolve_named(profile, profile_store, "WEKAN_PROFILE");
        }

        let snapshot = profile_store
            .read()
            .map_err(AppError::from)
            .map_err(|error| with_target_context(error, None))?;
        let Some(active_name) = snapshot.document().active_profile() else {
            return Err(with_target_context(
                AppError::configuration(
                    "a Wekan server is required; pass --server, set WEKAN_URL, pass --profile, set WEKAN_PROFILE, or activate a profile with `wekan profile use <NAME>`",
                ),
                None,
            ));
        };
        let profile = snapshot.document().profile(active_name).ok_or_else(|| {
            with_target_context(
                AppError::configuration(
                    "the active profile does not exist in the profile configuration",
                ),
                Some(active_name.to_owned()),
            )
        })?;
        let credential_namespace = profile_store
            .credential_namespace()
            .map_err(AppError::from)
            .map_err(|error| with_target_context(error, Some(active_name.to_owned())))?;
        Ok(ResolvedTarget::named(
            ServerUrl::parse(profile.server())
                .map_err(AppError::from)
                .map_err(|error| with_target_context(error, Some(active_name.to_owned())))?,
            active_name.to_owned(),
            credential_namespace,
            self.selection.allow_insecure_http,
            snapshot,
        ))
    }

    fn resolve_named(
        &self,
        name: &str,
        profile_store: &dyn ProfileStore,
        source: &str,
    ) -> Result<ResolvedTarget, AppError> {
        validate_profile_name(name).map_err(|message| {
            let error = if source == "--profile" {
                AppError::invalid_input(message)
            } else {
                AppError::configuration(format!("WEKAN_PROFILE is invalid: {message}"))
            };
            with_target_context(error, Some(name.to_owned()))
        })?;

        let snapshot = profile_store
            .read()
            .map_err(AppError::from)
            .map_err(|error| with_target_context(error, Some(name.to_owned())))?;
        let profile = snapshot.document().profile(name).ok_or_else(|| {
            AppError::new(
                ErrorCode::ProfileNotFound,
                format!("profile `{name}` does not exist"),
                StableExitCode::Configuration,
            )
            .with_details(ErrorDetails {
                profile: Some(Some(name.to_owned())),
                ..ErrorDetails::default()
            })
        })?;
        let credential_namespace = profile_store
            .credential_namespace()
            .map_err(AppError::from)
            .map_err(|error| with_target_context(error, Some(name.to_owned())))?;
        Ok(ResolvedTarget::named(
            ServerUrl::parse(profile.server())
                .map_err(AppError::from)
                .map_err(|error| with_target_context(error, Some(name.to_owned())))?,
            name.to_owned(),
            credential_namespace,
            self.selection.allow_insecure_http,
            snapshot,
        ))
    }
}

fn with_target_context(error: AppError, profile: Option<String>) -> AppError {
    if error.details().profile.is_some() {
        error
    } else {
        error.with_profile_context(profile)
    }
}

pub struct ResolvedTarget {
    client_factory: WekanClientFactory,
    // A named target retains the store's shared lease for the entire command.
    _profile_snapshot: Option<ProfileSnapshot>,
}

impl ResolvedTarget {
    fn direct(server: ServerUrl, allow_insecure_http: bool) -> Self {
        Self {
            client_factory: WekanClientFactory::for_server(server, allow_insecure_http),
            _profile_snapshot: None,
        }
    }

    fn named(
        server: ServerUrl,
        profile: String,
        credential_namespace: String,
        allow_insecure_http: bool,
        snapshot: ProfileSnapshot,
    ) -> Self {
        Self {
            client_factory: WekanClientFactory::for_resolved_profile(
                server,
                profile,
                credential_namespace,
                allow_insecure_http,
            ),
            _profile_snapshot: Some(snapshot),
        }
    }

    pub const fn client_factory(&self) -> &WekanClientFactory {
        &self.client_factory
    }

    pub fn profile(&self) -> Option<&str> {
        self.client_factory.profile()
    }
}

#[cfg(test)]
mod tests {
    use super::{ResolvedTarget, ServerEnvironment, ServerSelection, TargetResolver};
    use crate::{
        config::profiles::{
            Profile, ProfileDocument, ProfileMutation, ProfileSnapshot, ProfileStore,
            ProfileStoreError,
        },
        error::{AppError, ErrorCode},
    };

    struct StaticProfileStore(ProfileDocument);

    impl ProfileStore for StaticProfileStore {
        fn read(&self) -> Result<ProfileSnapshot, ProfileStoreError> {
            Ok(ProfileSnapshot::new_unlocked(self.0.clone()))
        }

        fn lock_mutation(&self) -> Result<Box<dyn ProfileMutation + Send + '_>, ProfileStoreError> {
            panic!("target resolution must not mutate profiles")
        }

        fn credential_namespace(&self) -> Result<String, ProfileStoreError> {
            Ok("test-store".to_owned())
        }
    }

    struct PanicProfileStore;

    impl ProfileStore for PanicProfileStore {
        fn read(&self) -> Result<ProfileSnapshot, ProfileStoreError> {
            panic!("direct server resolution must not read profiles")
        }

        fn lock_mutation(&self) -> Result<Box<dyn ProfileMutation + Send + '_>, ProfileStoreError> {
            panic!("target resolution must not mutate profiles")
        }

        fn credential_namespace(&self) -> Result<String, ProfileStoreError> {
            panic!("direct server resolution must not inspect profile credentials")
        }
    }

    fn profiles() -> StaticProfileStore {
        let mut document = ProfileDocument::default();
        document.insert(
            "active".to_owned(),
            Profile::new("https://active.example/".to_owned()),
        );
        document.insert(
            "explicit".to_owned(),
            Profile::new("https://explicit.example/".to_owned()),
        );
        document.insert(
            "environment".to_owned(),
            Profile::new("https://environment.example/".to_owned()),
        );
        document.set_active_profile(Some("active".to_owned()));
        StaticProfileStore(document)
    }

    fn resolve(
        selection: ServerSelection,
        profile_store: &dyn ProfileStore,
        environment_server: Option<&str>,
        environment_profile: Option<&str>,
    ) -> Result<ResolvedTarget, AppError> {
        TargetResolver::new(selection).resolve_with_environment(
            profile_store,
            ServerEnvironment {
                server: environment_server.map(str::to_owned),
                profile: environment_profile.map(str::to_owned),
            },
        )
    }

    #[test]
    fn explicit_server_wins_without_reading_profiles() {
        let target = resolve(
            ServerSelection {
                explicit_server: Some("https://explicit-server.example".to_owned()),
                ..ServerSelection::default()
            },
            &PanicProfileStore,
            Some("https://environment-server.example"),
            Some("environment"),
        )
        .unwrap();
        assert_eq!(
            target.client_factory().server_identity().as_str(),
            "https://explicit-server.example/"
        );
        assert_eq!(target.profile(), None);
    }

    #[test]
    fn explicit_profile_wins_over_environment_url() {
        let target = resolve(
            ServerSelection {
                explicit_profile: Some("explicit".to_owned()),
                ..ServerSelection::default()
            },
            &profiles(),
            Some("https://environment-server.example"),
            None,
        )
        .unwrap();
        assert_eq!(target.profile(), Some("explicit"));
        assert_eq!(
            target.client_factory().server_identity().as_str(),
            "https://explicit.example/"
        );
    }

    #[test]
    fn environment_url_wins_over_environment_profile() {
        let target = resolve(
            ServerSelection::default(),
            &PanicProfileStore,
            Some("https://environment-server.example"),
            Some("environment"),
        )
        .unwrap();
        assert_eq!(target.profile(), None);
    }

    #[test]
    fn environment_profile_wins_over_active_profile() {
        let target = resolve(
            ServerSelection::default(),
            &profiles(),
            None,
            Some("environment"),
        )
        .unwrap();
        assert_eq!(target.profile(), Some("environment"));
    }

    #[test]
    fn active_profile_is_the_final_fallback() {
        let target = resolve(ServerSelection::default(), &profiles(), None, None).unwrap();
        assert_eq!(target.profile(), Some("active"));
    }

    #[test]
    fn missing_profile_and_missing_selection_use_stable_errors() {
        let missing = resolve(
            ServerSelection {
                explicit_profile: Some("missing".to_owned()),
                ..ServerSelection::default()
            },
            &profiles(),
            None,
            None,
        )
        .err()
        .unwrap();
        assert_eq!(missing.code(), ErrorCode::ProfileNotFound);
        assert_eq!(missing.details().profile, Some(Some("missing".to_owned())));

        let empty = StaticProfileStore(ProfileDocument::default());
        let missing = resolve(ServerSelection::default(), &empty, None, None)
            .err()
            .unwrap();
        assert_eq!(missing.code(), ErrorCode::ConfigurationError);
        assert!(missing.message().contains("WEKAN_PROFILE"));
    }

    #[test]
    fn resolving_remote_http_does_not_grant_network_permission() {
        let mut document = ProfileDocument::default();
        document.insert(
            "remote".to_owned(),
            Profile::new("http://wekan.example/".to_owned()),
        );
        let profiles = StaticProfileStore(document);

        let target = resolve(
            ServerSelection {
                explicit_profile: Some("remote".to_owned()),
                ..ServerSelection::default()
            },
            &profiles,
            None,
            None,
        )
        .unwrap();
        assert!(target.client_factory().create().is_err());

        let target = resolve(
            ServerSelection {
                explicit_profile: Some("remote".to_owned()),
                allow_insecure_http: true,
                ..ServerSelection::default()
            },
            &profiles,
            None,
            None,
        )
        .unwrap();
        assert!(target.client_factory().create().is_ok());
    }
}
