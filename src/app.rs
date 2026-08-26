use crate::{
    command_result::CommandSuccess,
    commands::{self, PreparedCommand, RootCommand},
    config::{ServerSelection, TargetResolver, profiles::FileProfileStore},
    credentials::{
        CredentialStore, KeyringCredentialStore, SecretInputProvider, SystemSecretInputProvider,
    },
    error::AppError,
};

pub struct App<S, P, F = FileProfileStore> {
    target_resolver: TargetResolver,
    credential_store: S,
    secret_input: P,
    profile_store: F,
}

impl App<KeyringCredentialStore, SystemSecretInputProvider, FileProfileStore> {
    pub fn production(selection: ServerSelection) -> Self {
        Self::with_profile_store(
            selection,
            KeyringCredentialStore,
            SystemSecretInputProvider,
            FileProfileStore::production(),
        )
    }
}

impl<S, P> App<S, P, FileProfileStore>
where
    S: CredentialStore,
    P: SecretInputProvider,
{
    pub fn new(
        server: Option<String>,
        allow_insecure_http: bool,
        credential_store: S,
        secret_input: P,
    ) -> Self {
        Self::with_profile_store(
            ServerSelection::direct(server, allow_insecure_http),
            credential_store,
            secret_input,
            FileProfileStore::production(),
        )
    }
}

impl<S, P, F> App<S, P, F>
where
    S: CredentialStore,
    P: SecretInputProvider,
    F: crate::config::profiles::ProfileStore,
{
    pub const fn with_profile_store(
        selection: ServerSelection,
        credential_store: S,
        secret_input: P,
        profile_store: F,
    ) -> Self {
        Self {
            target_resolver: TargetResolver::new(selection),
            credential_store,
            secret_input,
            profile_store,
        }
    }

    pub const fn credential_store(&self) -> &S {
        &self.credential_store
    }

    pub async fn execute(&self, command: RootCommand) -> Result<CommandSuccess, AppError> {
        match command {
            RootCommand::Auth(args) => self.execute_auth(args).await,
            RootCommand::Profile(args) => {
                if self.target_resolver.has_explicit_target() {
                    return Err(AppError::invalid_input(
                        "--server and --profile cannot be used with profile-management commands",
                    ));
                }
                commands::dispatch(
                    PreparedCommand::Profile { args },
                    &self.credential_store,
                    &self.secret_input,
                    &self.profile_store,
                )
                .await
            }
        }
    }

    async fn execute_auth(
        &self,
        args: crate::commands::auth::AuthArgs,
    ) -> Result<CommandSuccess, AppError> {
        let target = self.target_resolver.resolve(&self.profile_store)?;
        let profile = target.profile().map(str::to_owned);
        commands::dispatch(
            PreparedCommand::Auth {
                args,
                client_factory: target.client_factory(),
            },
            &self.credential_store,
            &self.secret_input,
            &self.profile_store,
        )
        .await
        .map_err(|error| error.with_profile_context(profile))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::{Mutex, mpsc},
        time::Duration,
    };

    use super::App;
    use crate::{
        commands::{
            RootCommand,
            auth::{AuthArgs, AuthCommand, logout::LogoutArgs, status::StatusArgs},
        },
        config::{
            ServerSelection,
            profiles::{FileProfileStore, Profile, ProfileDocument, ProfileStore},
        },
        credentials::{
            CredentialDeleteOutcome, CredentialError, CredentialRecord, CredentialStore,
            CredentialTarget, SystemSecretInputProvider,
        },
    };

    struct BlockingCredentialStore {
        entered: Mutex<Option<mpsc::Sender<()>>>,
        release: Mutex<mpsc::Receiver<()>>,
    }

    impl CredentialStore for BlockingCredentialStore {
        fn check_available(&self, _target: &CredentialTarget) -> Result<(), CredentialError> {
            self.entered
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .send(())
                .unwrap();
            self.release.lock().unwrap().recv().unwrap();
            Err(CredentialError::Unavailable("test complete".to_owned()))
        }

        fn load(
            &self,
            _target: &CredentialTarget,
        ) -> Result<Option<CredentialRecord>, CredentialError> {
            panic!("preflight must stop before credential loading")
        }

        fn save(
            &self,
            _target: &CredentialTarget,
            _record: &CredentialRecord,
        ) -> Result<(), CredentialError> {
            panic!("status must not save credentials")
        }

        fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
            panic!("status must not delete credentials")
        }

        fn delete_if_matches(
            &self,
            _target: &CredentialTarget,
            _expected: &CredentialRecord,
        ) -> Result<CredentialDeleteOutcome, CredentialError> {
            panic!("status must not delete credentials")
        }
    }

    fn require_send<T: Send>(_: T) {}

    #[test]
    fn execute_future_is_send() {
        let app = App::production(ServerSelection::direct(
            Some("https://wekan.example".to_owned()),
            false,
        ));
        let command = RootCommand::Auth(AuthArgs {
            command: AuthCommand::Logout(LogoutArgs {
                all: false,
                local_only: true,
            }),
        });

        require_send(app.execute(command));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn named_auth_holds_the_profile_lease_for_the_complete_transaction() {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "wekan-cli-auth-profile-lease-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        let profile_store = FileProfileStore::at(directory.clone());
        let mut document = ProfileDocument::default();
        document.insert(
            "work".to_owned(),
            Profile::new("https://wekan.example/".to_owned()),
        );
        document.set_active_profile(Some("work".to_owned()));
        profile_store
            .lock_mutation()
            .unwrap()
            .save(document)
            .unwrap();

        let (entered_sender, entered_receiver) = mpsc::channel();
        let (release_sender, release_receiver) = mpsc::channel();
        let app = App::with_profile_store(
            ServerSelection {
                explicit_profile: Some("work".to_owned()),
                ..ServerSelection::default()
            },
            BlockingCredentialStore {
                entered: Mutex::new(Some(entered_sender)),
                release: Mutex::new(release_receiver),
            },
            SystemSecretInputProvider,
            profile_store.clone(),
        );
        let auth = tokio::spawn(async move {
            app.execute(RootCommand::Auth(AuthArgs {
                command: AuthCommand::Status(StatusArgs {}),
            }))
            .await
        });
        entered_receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap();

        let mutation_profiles = profile_store.clone();
        let (attempted_sender, attempted_receiver) = mpsc::channel();
        let (mutation_sender, mutation_receiver) = mpsc::channel();
        let mutation = std::thread::spawn(move || {
            attempted_sender.send(()).unwrap();
            let _mutation = mutation_profiles.lock_mutation().unwrap();
            mutation_sender.send(()).unwrap();
        });
        attempted_receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        assert!(
            mutation_receiver
                .recv_timeout(Duration::from_millis(150))
                .is_err()
        );

        release_sender.send(()).unwrap();
        assert!(auth.await.unwrap().is_err());
        mutation_receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        mutation.join().unwrap();
        fs::remove_dir_all(directory).unwrap();
    }
}
