use std::fs;

use super::{
    MissingProfileResolution, ResolvedTarget, ServerEnvironment, ServerSelection, TargetResolver,
};
use crate::{
    config::profiles::{
        FileProfileStore, Profile, ProfileDocument, ProfileMutation, ProfileSnapshot, ProfileStore,
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
