use std::{collections::BTreeSet, fs, sync::Mutex};

use secrecy::SecretString;
use time::{Duration, OffsetDateTime};

use super::{
    AddArgs, ListArgs, ProfileCommand, RemoveArgs, ShowArgs, UpdateArgs, UseArgs,
    dispatch as dispatch_with_confirmation,
};
use crate::{
    command_result::CommandSuccess,
    config::profiles::{FileProfileStore, Profile, ProfileStore},
    credentials::{
        CredentialDeleteOutcome, CredentialError, CredentialRecord, CredentialStore,
        CredentialTarget,
    },
    error::ErrorCode,
    input::{
        ConfirmationArgs, ConfirmationProvider, ConfirmationRequest, FakeConfirmationProvider,
    },
};

struct CallbackConfirmation<F>(F);

impl<F> ConfirmationProvider for CallbackConfirmation<F>
where
    F: Fn() + Send + Sync,
{
    fn confirm(&self, _request: &ConfirmationRequest) -> Result<bool, crate::error::AppError> {
        (self.0)();
        Ok(true)
    }
}

fn dispatch(
    command: ProfileCommand,
    store: &dyn ProfileStore,
    credentials: &dyn CredentialStore,
) -> Result<CommandSuccess, crate::error::AppError> {
    dispatch_with_confirmation(
        command,
        store,
        credentials,
        &FakeConfirmationProvider::accepting(),
    )
}

struct TestDirectory(std::path::PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "wekan-cli-profile-command-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn store(&self) -> FileProfileStore {
        FileProfileStore::at(self.0.clone())
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Default)]
struct FakeCredentialStore {
    present: Mutex<BTreeSet<String>>,
}

impl FakeCredentialStore {
    fn insert(&self, account: &str) {
        self.present.lock().unwrap().insert(account.to_owned());
    }

    fn clear(&self) {
        self.present.lock().unwrap().clear();
    }
}

impl CredentialStore for FakeCredentialStore {
    fn check_available(&self, _target: &CredentialTarget) -> Result<(), CredentialError> {
        Ok(())
    }

    fn load(&self, target: &CredentialTarget) -> Result<Option<CredentialRecord>, CredentialError> {
        Ok(self
            .present
            .lock()
            .unwrap()
            .contains(target.account())
            .then(|| {
                CredentialRecord::new(
                    target.server_url().to_owned(),
                    "user-1".to_owned(),
                    SecretString::from("token".to_owned()),
                    OffsetDateTime::now_utc() + Duration::hours(1),
                )
            }))
    }

    fn save(
        &self,
        _target: &CredentialTarget,
        _record: &CredentialRecord,
    ) -> Result<(), CredentialError> {
        panic!("profile commands must never save credentials")
    }

    fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("profile commands must never delete credentials")
    }

    fn delete_if_matches(
        &self,
        _target: &CredentialTarget,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("profile commands must never delete credentials")
    }
}

fn add(
    store: &FileProfileStore,
    credentials: &FakeCredentialStore,
    name: &str,
    url: &str,
    activate: bool,
) -> Result<CommandSuccess, crate::error::AppError> {
    dispatch(
        ProfileCommand::Add(AddArgs {
            name: name.to_owned(),
            url: url.to_owned(),
            r#use: activate,
        }),
        store,
        credentials,
    )
}

#[test]
fn full_profile_lifecycle_is_deterministic_and_allows_same_server_accounts() {
    let directory = TestDirectory::new();
    let store = directory.store();
    let credentials = FakeCredentialStore::default();

    let CommandSuccess::ProfileList(empty) =
        dispatch(ProfileCommand::List(ListArgs {}), &store, &credentials).unwrap()
    else {
        panic!("expected profile list")
    };
    assert!(empty.profiles.is_empty());
    assert_eq!(empty.active_profile, None);

    let CommandSuccess::ProfileAdded(work) =
        add(&store, &credentials, "work", "https://wekan.example", false).unwrap()
    else {
        panic!("expected added profile")
    };
    assert!(work.active);

    let CommandSuccess::ProfileAdded(local) = add(
        &store,
        &credentials,
        "local",
        "https://wekan.example/",
        false,
    )
    .unwrap() else {
        panic!("expected added profile")
    };
    assert!(!local.active);

    let CommandSuccess::ProfileList(list) =
        dispatch(ProfileCommand::List(ListArgs {}), &store, &credentials).unwrap()
    else {
        panic!("expected profile list")
    };
    assert_eq!(list.active_profile.as_deref(), Some("work"));
    assert_eq!(
        list.profiles
            .iter()
            .map(|profile| profile.name.as_str())
            .collect::<Vec<_>>(),
        ["local", "work"]
    );

    let CommandSuccess::ProfileUsed(used) = dispatch(
        ProfileCommand::Use(UseArgs {
            name: "local".to_owned(),
        }),
        &store,
        &credentials,
    )
    .unwrap() else {
        panic!("expected used profile")
    };
    assert!(used.active);

    let CommandSuccess::ProfileShown(shown) = dispatch(
        ProfileCommand::Show(ShowArgs {
            name: "local".to_owned(),
        }),
        &store,
        &credentials,
    )
    .unwrap() else {
        panic!("expected shown profile")
    };
    assert_eq!(shown.server, "https://wekan.example/");
    assert!(shown.active);
}

#[test]
fn duplicate_names_and_missing_profiles_have_stable_errors() {
    let directory = TestDirectory::new();
    let store = directory.store();
    let credentials = FakeCredentialStore::default();
    add(
        &store,
        &credentials,
        "local",
        "http://localhost:3000",
        false,
    )
    .unwrap();

    let duplicate = add(
        &store,
        &credentials,
        "local",
        "http://localhost:4000",
        false,
    )
    .unwrap_err();
    assert_eq!(duplicate.code(), ErrorCode::ProfileAlreadyExists);

    let missing = dispatch(
        ProfileCommand::Show(ShowArgs {
            name: "missing".to_owned(),
        }),
        &store,
        &credentials,
    )
    .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::ProfileNotFound);
}

#[test]
fn update_and_remove_require_the_profile_to_be_logged_out() {
    let directory = TestDirectory::new();
    let store = directory.store();
    let credentials = FakeCredentialStore::default();
    add(
        &store,
        &credentials,
        "local",
        "http://localhost:3000",
        false,
    )
    .unwrap();
    let namespace = store.credential_namespace().unwrap();
    let target = CredentialTarget::profile_in_store(
        "local",
        &namespace,
        "http://localhost:3000/".to_owned(),
    );
    credentials.insert(target.account());

    let CommandSuccess::ProfileUpdated(unchanged) = dispatch(
        ProfileCommand::Update(UpdateArgs {
            name: "local".to_owned(),
            url: "http://localhost:3000/".to_owned(),
        }),
        &store,
        &credentials,
    )
    .unwrap() else {
        panic!("expected idempotent update")
    };
    assert_eq!(unchanged.server, "http://localhost:3000/");

    let update = dispatch(
        ProfileCommand::Update(UpdateArgs {
            name: "local".to_owned(),
            url: "http://localhost:4000".to_owned(),
        }),
        &store,
        &credentials,
    )
    .unwrap_err();
    assert_eq!(update.code(), ErrorCode::ProfileHasCredential);

    let active = dispatch(
        ProfileCommand::Remove(RemoveArgs {
            name: "local".to_owned(),
            force: false,
            confirmation: ConfirmationArgs::assume_yes(),
        }),
        &store,
        &credentials,
    )
    .unwrap_err();
    assert_eq!(active.code(), ErrorCode::ProfileInUse);

    let credential = dispatch(
        ProfileCommand::Remove(RemoveArgs {
            name: "local".to_owned(),
            force: true,
            confirmation: ConfirmationArgs::assume_yes(),
        }),
        &store,
        &credentials,
    )
    .unwrap_err();
    assert_eq!(credential.code(), ErrorCode::ProfileHasCredential);

    credentials.clear();
    let CommandSuccess::ProfileRemoved(removed) = dispatch(
        ProfileCommand::Remove(RemoveArgs {
            name: "local".to_owned(),
            force: true,
            confirmation: ConfirmationArgs::assume_yes(),
        }),
        &store,
        &credentials,
    )
    .unwrap() else {
        panic!("expected removed profile")
    };
    assert!(removed.removed);
    assert_eq!(removed.active_profile, None);
}

#[test]
fn removal_decline_and_confirmation_failure_leave_the_profile_unchanged() {
    for (confirmation, expected_error) in [
        (FakeConfirmationProvider::declining(), false),
        (
            FakeConfirmationProvider::failing(crate::error::AppError::invalid_input(
                "test confirmation unavailable",
            )),
            true,
        ),
    ] {
        let directory = TestDirectory::new();
        let store = directory.store();
        let credentials = FakeCredentialStore::default();
        add(
            &store,
            &credentials,
            "local",
            "http://localhost:3000",
            false,
        )
        .unwrap();

        let result = dispatch_with_confirmation(
            ProfileCommand::Remove(RemoveArgs {
                name: "local".to_owned(),
                force: true,
                confirmation: ConfirmationArgs::default(),
            }),
            &store,
            &credentials,
            &confirmation,
        );

        assert!(store.read().unwrap().document().profile("local").is_some());
        assert_eq!(confirmation.requests().len(), 1);
        assert!(
            confirmation.requests()[0]
                .prompt()
                .contains("clear the active selection")
        );
        if expected_error {
            assert_eq!(result.unwrap_err().code(), ErrorCode::InvalidInput);
        } else {
            let success = result.unwrap();
            let CommandSuccess::Cancelled(cancelled) = success else {
                panic!("expected cancellation")
            };
            assert!(cancelled.cancelled);
        }
    }
}

fn inactive_profile(store: &FileProfileStore, credentials: &FakeCredentialStore) -> String {
    add(store, credentials, "active", "http://localhost:3000", false).unwrap();
    add(store, credentials, "target", "http://localhost:4000", false).unwrap();
    "http://localhost:4000/".to_owned()
}

#[test]
fn profile_changed_while_awaiting_confirmation_is_not_removed() {
    let directory = TestDirectory::new();
    let store = directory.store();
    let credentials = FakeCredentialStore::default();
    inactive_profile(&store, &credentials);
    let confirmation = CallbackConfirmation(|| {
        let mut mutation = store.lock_mutation().unwrap();
        let mut document = mutation.document().clone();
        document.insert(
            "target".to_owned(),
            Profile::new("http://localhost:5000/".to_owned()),
        );
        mutation.save(document).unwrap();
    });

    let error = dispatch_with_confirmation(
        ProfileCommand::Remove(RemoveArgs {
            name: "target".to_owned(),
            force: false,
            confirmation: ConfirmationArgs::default(),
        }),
        &store,
        &credentials,
        &confirmation,
    )
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::ConfigurationError);
    assert_eq!(
        store
            .read()
            .unwrap()
            .document()
            .profile("target")
            .unwrap()
            .server(),
        "http://localhost:5000/"
    );
}

#[test]
fn profile_recreated_with_the_same_values_while_awaiting_confirmation_is_not_removed() {
    let directory = TestDirectory::new();
    let store = directory.store();
    let credentials = FakeCredentialStore::default();
    let server = inactive_profile(&store, &credentials);
    let confirmation = CallbackConfirmation(|| {
        {
            let mut mutation = store.lock_mutation().unwrap();
            let mut document = mutation.document().clone();
            document.remove("target");
            mutation.save(document).unwrap();
        }

        let mut mutation = store.lock_mutation().unwrap();
        let mut document = mutation.document().clone();
        document.insert("target".to_owned(), Profile::new(server.clone()));
        mutation.save(document).unwrap();
    });

    let error = dispatch_with_confirmation(
        ProfileCommand::Remove(RemoveArgs {
            name: "target".to_owned(),
            force: false,
            confirmation: ConfirmationArgs::default(),
        }),
        &store,
        &credentials,
        &confirmation,
    )
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::ConfigurationError);
    assert_eq!(
        store
            .read()
            .unwrap()
            .document()
            .profile("target")
            .unwrap()
            .server(),
        server
    );
}

#[test]
fn profile_removed_or_activated_while_awaiting_confirmation_is_not_deleted_again() {
    for activate in [false, true] {
        let directory = TestDirectory::new();
        let store = directory.store();
        let credentials = FakeCredentialStore::default();
        inactive_profile(&store, &credentials);
        let confirmation = CallbackConfirmation(|| {
            let mut mutation = store.lock_mutation().unwrap();
            let mut document = mutation.document().clone();
            if activate {
                document.set_active_profile(Some("target".to_owned()));
            } else {
                document.remove("target");
            }
            mutation.save(document).unwrap();
        });

        let error = dispatch_with_confirmation(
            ProfileCommand::Remove(RemoveArgs {
                name: "target".to_owned(),
                force: false,
                confirmation: ConfirmationArgs::default(),
            }),
            &store,
            &credentials,
            &confirmation,
        )
        .unwrap_err();

        assert_eq!(
            error.code(),
            if activate {
                ErrorCode::ConfigurationError
            } else {
                ErrorCode::ProfileNotFound
            }
        );
        assert_eq!(
            store.read().unwrap().document().profile("target").is_some(),
            activate
        );
    }
}

#[test]
fn credential_added_while_awaiting_confirmation_blocks_profile_removal() {
    let directory = TestDirectory::new();
    let store = directory.store();
    let credentials = FakeCredentialStore::default();
    let server = inactive_profile(&store, &credentials);
    let namespace = store.credential_namespace().unwrap();
    let target = CredentialTarget::profile_in_store("target", &namespace, server);
    let confirmation = CallbackConfirmation(|| credentials.insert(target.account()));

    let error = dispatch_with_confirmation(
        ProfileCommand::Remove(RemoveArgs {
            name: "target".to_owned(),
            force: false,
            confirmation: ConfirmationArgs::default(),
        }),
        &store,
        &credentials,
        &confirmation,
    )
    .unwrap_err();

    assert_eq!(error.code(), ErrorCode::ProfileHasCredential);
    assert!(store.read().unwrap().document().profile("target").is_some());
}

#[test]
fn remote_plaintext_urls_are_saved_as_identity_without_network_permission() {
    let directory = TestDirectory::new();
    let store = directory.store();
    let credentials = FakeCredentialStore::default();
    let result = add(
        &store,
        &credentials,
        "insecure",
        "http://wekan.example",
        false,
    )
    .unwrap();

    let CommandSuccess::ProfileAdded(profile) = result else {
        panic!("expected added profile")
    };
    assert_eq!(profile.server, "http://wekan.example/");
}
