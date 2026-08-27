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
mod tests {
    use std::fs;

    use super::{
        MissingProfileResolution, ResolvedTarget, ServerEnvironment, ServerSelection,
        TargetResolver,
    };
    use crate::{
        config::profiles::{
            FileProfileStore, Profile, ProfileDocument, ProfileMutation, ProfileSnapshot,
            ProfileStore, ProfileStoreError,
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

    fn resolve<'a>(
        selection: ServerSelection,
        profile_store: &'a dyn ProfileStore,
        environment_server: Option<&str>,
        environment_profile: Option<&str>,
        missing_profile_resolution: MissingProfileResolution,
    ) -> Result<ResolvedTarget<'a>, AppError> {
        TargetResolver::new(selection).resolve_with_environment(
            profile_store,
            ServerEnvironment {
                server: environment_server.map(str::to_owned),
                profile: environment_profile.map(str::to_owned),
            },
            missing_profile_resolution,
        )
    }

    #[test]
    fn profile_and_server_sources_resolve_independently_by_precedence() {
        let store = profiles();
        let target = resolve(
            ServerSelection {
                explicit_server: Some("https://explicit.example".to_owned()),
                explicit_profile: Some("explicit".to_owned()),
                ..ServerSelection::default()
            },
            &store,
            Some("not a URL and ignored"),
            Some("INVALID AND IGNORED"),
            MissingProfileResolution::Reject,
        )
        .unwrap();
        assert_eq!(
            target.client_factory().server_identity().as_str(),
            "https://explicit.example/"
        );
        assert_eq!(target.profile(), "explicit");
    }

    #[test]
    fn environment_profile_and_url_override_the_active_profile_together() {
        let store = profiles();
        let target = resolve(
            ServerSelection::default(),
            &store,
            Some("https://environment.example"),
            Some("environment"),
            MissingProfileResolution::Reject,
        )
        .unwrap();
        assert_eq!(target.profile(), "environment");
    }

    #[test]
    fn active_profile_is_the_final_fallback() {
        let store = profiles();
        let target = resolve(
            ServerSelection::default(),
            &store,
            None,
            None,
            MissingProfileResolution::Reject,
        )
        .unwrap();
        assert_eq!(target.profile(), "active");
    }

    #[test]
    fn missing_selection_falls_back_to_default_and_requires_initializable_auth() {
        let empty = StaticProfileStore(ProfileDocument::default());
        let missing = resolve(
            ServerSelection::default(),
            &empty,
            Some("https://wekan.example"),
            None,
            MissingProfileResolution::Reject,
        )
        .err()
        .unwrap();
        assert_eq!(missing.code(), ErrorCode::ProfileNotFound);
        assert_eq!(missing.details().profile.as_deref(), Some("default"));

        let missing = resolve(
            ServerSelection {
                explicit_profile: Some("missing".to_owned()),
                ..ServerSelection::default()
            },
            &profiles(),
            None,
            None,
            MissingProfileResolution::Initialize,
        )
        .err()
        .unwrap();
        assert_eq!(missing.code(), ErrorCode::ProfileNotFound);
        assert_eq!(missing.details().profile.as_deref(), Some("missing"));
    }

    #[test]
    fn supplied_server_must_match_an_existing_profile_without_mutating_it() {
        let error = resolve(
            ServerSelection {
                explicit_server: Some("https://other.example".to_owned()),
                explicit_profile: Some("explicit".to_owned()),
                ..ServerSelection::default()
            },
            &profiles(),
            None,
            None,
            MissingProfileResolution::Initialize,
        )
        .err()
        .unwrap();
        assert_eq!(error.code(), ErrorCode::ProfileServerMismatch);
        assert_eq!(error.details().profile.as_deref(), Some("explicit"));
    }

    #[test]
    fn successful_initialization_persists_default_and_activates_the_first_profile() {
        let directory =
            std::env::temp_dir().join(format!("wekan-cli-resolver-init-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let store = FileProfileStore::at(directory.clone());
        let mut target = resolve(
            ServerSelection::default(),
            &store,
            Some("https://wekan.example"),
            None,
            MissingProfileResolution::Initialize,
        )
        .unwrap();
        assert_eq!(target.profile(), "default");
        assert!(!target.profile_created());
        target.initialize_profile().unwrap();
        assert!(target.profile_created());
        assert!(target.profile_active());
        drop(target);
        let snapshot = store.read().unwrap();
        assert_eq!(snapshot.document().active_profile(), Some("default"));
        assert_eq!(
            snapshot.document().profile("default").unwrap().server(),
            "https://wekan.example/"
        );
        drop(snapshot);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn missing_profile_local_credential_cleanup_uses_the_supplied_server_without_mutation() {
        let empty = StaticProfileStore(ProfileDocument::default());
        let target = resolve(
            ServerSelection {
                explicit_profile: Some("orphaned".to_owned()),
                ..ServerSelection::default()
            },
            &empty,
            Some("https://wekan.example"),
            None,
            MissingProfileResolution::LocalCredentialCleanup,
        )
        .unwrap();

        assert_eq!(target.profile(), "orphaned");
        assert_eq!(
            target.client_factory().server_identity().as_str(),
            "https://wekan.example/"
        );
        assert!(!target.profile_created());
        assert!(!target.profile_active());

        let error = match resolve(
            ServerSelection {
                explicit_profile: Some("orphaned".to_owned()),
                ..ServerSelection::default()
            },
            &empty,
            None,
            None,
            MissingProfileResolution::LocalCredentialCleanup,
        ) {
            Ok(_) => panic!("a missing profile without a supplied server must be rejected"),
            Err(error) => error,
        };
        assert_eq!(error.code(), ErrorCode::ProfileNotFound);
        assert_eq!(error.details().profile.as_deref(), Some("orphaned"));
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
            MissingProfileResolution::Reject,
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
            MissingProfileResolution::Reject,
        )
        .unwrap();
        assert!(target.client_factory().create().is_ok());
    }
}
