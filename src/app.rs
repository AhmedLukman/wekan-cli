use crate::{
    command_result::CommandSuccess,
    commands::{self, PreparedCommand, RootCommand},
    config::{ServerSelection, TargetResolver, profiles::FileProfileStore},
    credentials::{
        CredentialStore, KeyringCredentialStore, SecretInputProvider, SystemSecretInputProvider,
    },
    error::AppError,
    input::{ConfirmationProvider, SystemConfirmationProvider},
};

pub struct App<S, P, F = FileProfileStore, C = SystemConfirmationProvider> {
    target_resolver: TargetResolver,
    credential_store: S,
    secret_input: P,
    profile_store: F,
    confirmation: C,
}

impl
    App<
        KeyringCredentialStore,
        SystemSecretInputProvider,
        FileProfileStore,
        SystemConfirmationProvider,
    >
{
    pub fn production(selection: ServerSelection) -> Self {
        Self::production_with_confirmation_interactivity(selection, true)
    }

    pub fn production_with_confirmation_interactivity(
        selection: ServerSelection,
        allow_interactive_confirmation: bool,
    ) -> Self {
        Self::with_dependencies(
            selection,
            KeyringCredentialStore,
            SystemSecretInputProvider,
            FileProfileStore::production(),
            SystemConfirmationProvider::for_output(allow_interactive_confirmation),
        )
    }
}

impl<S, P, F> App<S, P, F, SystemConfirmationProvider>
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
        Self::with_dependencies(
            selection,
            credential_store,
            secret_input,
            profile_store,
            SystemConfirmationProvider::interactive(),
        )
    }
}

impl<S, P, F, C> App<S, P, F, C>
where
    S: CredentialStore,
    P: SecretInputProvider,
    F: crate::config::profiles::ProfileStore,
    C: ConfirmationProvider,
{
    pub const fn with_dependencies(
        selection: ServerSelection,
        credential_store: S,
        secret_input: P,
        profile_store: F,
        confirmation: C,
    ) -> Self {
        Self {
            target_resolver: TargetResolver::new(selection),
            credential_store,
            secret_input,
            profile_store,
            confirmation,
        }
    }

    pub fn with_confirmation_provider<C2>(self, confirmation: C2) -> App<S, P, F, C2>
    where
        C2: ConfirmationProvider,
    {
        App {
            target_resolver: self.target_resolver,
            credential_store: self.credential_store,
            secret_input: self.secret_input,
            profile_store: self.profile_store,
            confirmation,
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
                    &self.confirmation,
                )
                .await
            }
        }
    }

    async fn execute_auth(
        &self,
        args: crate::commands::auth::AuthArgs,
    ) -> Result<CommandSuccess, AppError> {
        let missing_profile_resolution = args.command.missing_profile_resolution();
        let allow_initialization = missing_profile_resolution.allows_initialization();
        let mut target = self
            .target_resolver
            .resolve(&self.profile_store, missing_profile_resolution)?;
        let profile = target.profile().to_owned();
        let result = commands::dispatch(
            PreparedCommand::Auth {
                args,
                target: &mut target,
            },
            &self.credential_store,
            &self.secret_input,
            &self.profile_store,
            &self.confirmation,
        )
        .await;
        let profile_created = target.profile_created();
        let profile_active = target.profile_active();
        result.map_err(|error| {
            let error = error.with_profile_context(profile);
            if allow_initialization {
                error.with_profile_state(profile_created, profile_active)
            } else {
                error
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs, io,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, AtomicUsize, Ordering},
            mpsc,
        },
        time::Duration,
    };

    use secrecy::SecretString;
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::App;
    use crate::{
        command_result::{CommandSuccess, LogoutScope},
        commands::{
            RootCommand,
            auth::{
                AuthArgs, AuthCommand, login::LoginArgs, logout::LogoutArgs,
                register::RegisterArgs, status::StatusArgs,
            },
        },
        config::{
            ServerSelection,
            profiles::{
                FileProfileStore, Profile, ProfileDocument, ProfileMutation, ProfileSnapshot,
                ProfileStore, ProfileStoreError,
            },
        },
        credentials::{
            CredentialCreateOutcome, CredentialDeleteOutcome, CredentialError, CredentialRecord,
            CredentialStore, CredentialTarget, LoginSecretMode, LoginSecrets, SecretInputProvider,
            SystemSecretInputProvider,
        },
        error::{AppError, ErrorCode},
        input::ConfirmationArgs,
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

        fn exists(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
            panic!("preflight must stop before credential inspection")
        }

        fn load(
            &self,
            _target: &CredentialTarget,
        ) -> Result<Option<CredentialRecord>, CredentialError> {
            panic!("preflight must stop before credential loading")
        }

        fn create(
            &self,
            _target: &CredentialTarget,
            _record: &CredentialRecord,
        ) -> Result<CredentialCreateOutcome, CredentialError> {
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

    #[derive(Clone, Default)]
    struct RecordingCredentialStore {
        state: Arc<RecordingCredentialState>,
    }

    #[derive(Default)]
    struct RecordingCredentialState {
        accounts: Mutex<Vec<String>>,
        create_calls: AtomicUsize,
        fail_create: AtomicBool,
        late_conflict: AtomicBool,
    }

    impl CredentialStore for RecordingCredentialStore {
        fn check_available(&self, _target: &CredentialTarget) -> Result<(), CredentialError> {
            Ok(())
        }

        fn exists(&self, target: &CredentialTarget) -> Result<bool, CredentialError> {
            Ok(self
                .state
                .accounts
                .lock()
                .unwrap()
                .iter()
                .any(|account| account == target.account()))
        }

        fn load(
            &self,
            _target: &CredentialTarget,
        ) -> Result<Option<CredentialRecord>, CredentialError> {
            Ok(None)
        }

        fn create(
            &self,
            target: &CredentialTarget,
            _record: &CredentialRecord,
        ) -> Result<CredentialCreateOutcome, CredentialError> {
            self.state.create_calls.fetch_add(1, Ordering::SeqCst);
            if self.state.fail_create.load(Ordering::SeqCst) {
                return Err(CredentialError::Store("injected failure".to_owned()));
            }
            if self.state.late_conflict.load(Ordering::SeqCst) {
                self.state
                    .accounts
                    .lock()
                    .unwrap()
                    .push(target.account().to_owned());
                return Ok(CredentialCreateOutcome::AlreadyExists);
            }
            let mut accounts = self.state.accounts.lock().unwrap();
            if accounts.iter().any(|account| account == target.account()) {
                return Ok(CredentialCreateOutcome::AlreadyExists);
            }
            accounts.push(target.account().to_owned());
            Ok(CredentialCreateOutcome::Created)
        }

        fn delete(&self, target: &CredentialTarget) -> Result<bool, CredentialError> {
            let mut accounts = self.state.accounts.lock().unwrap();
            let Some(index) = accounts
                .iter()
                .position(|account| account == target.account())
            else {
                return Ok(false);
            };
            accounts.remove(index);
            Ok(true)
        }

        fn delete_if_matches(
            &self,
            _target: &CredentialTarget,
            _expected: &CredentialRecord,
        ) -> Result<CredentialDeleteOutcome, CredentialError> {
            Ok(CredentialDeleteOutcome::Absent)
        }
    }

    #[derive(Clone, Default)]
    struct FixedSecretInput {
        reads: Arc<AtomicUsize>,
    }

    impl SecretInputProvider for FixedSecretInput {
        fn read_registration_password(&self, _from_stdin: bool) -> Result<SecretString, AppError> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            Ok(SecretString::from("test-password".to_owned()))
        }

        fn read_login_secrets(&self, _mode: LoginSecretMode) -> Result<LoginSecrets, AppError> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            Ok(LoginSecrets::new(
                SecretString::from("test-password".to_owned()),
                None,
            ))
        }
    }

    struct FailingProfileStore {
        after_install: bool,
    }

    struct FailingProfileMutation {
        document: ProfileDocument,
        after_install: bool,
    }

    impl ProfileMutation for FailingProfileMutation {
        fn document(&self) -> &ProfileDocument {
            &self.document
        }

        fn save(&mut self, _document: ProfileDocument) -> Result<(), ProfileStoreError> {
            let error = io::Error::other("injected profile save failure");
            if self.after_install {
                Err(ProfileStoreError::WriteAfterInstall(error))
            } else {
                Err(ProfileStoreError::Write(error))
            }
        }
    }

    impl ProfileStore for FailingProfileStore {
        fn read(&self) -> Result<ProfileSnapshot, ProfileStoreError> {
            Ok(ProfileSnapshot::new_unlocked(ProfileDocument::default()))
        }

        fn lock_mutation(&self) -> Result<Box<dyn ProfileMutation + Send + '_>, ProfileStoreError> {
            Ok(Box::new(FailingProfileMutation {
                document: ProfileDocument::default(),
                after_install: self.after_install,
            }))
        }

        fn credential_namespace(&self) -> Result<String, ProfileStoreError> {
            Ok("failing-store".to_owned())
        }
    }

    fn test_directory(label: &str) -> std::path::PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "wekan-cli-app-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn login_command() -> RootCommand {
        RootCommand::Auth(AuthArgs {
            command: AuthCommand::Login(LoginArgs {
                username: Some("alice".to_owned()),
                email: None,
                password_stdin: false,
                code: false,
                code_stdin: false,
            }),
        })
    }

    fn registration_command() -> RootCommand {
        RootCommand::Auth(AuthArgs {
            command: AuthCommand::Register(RegisterArgs {
                username: Some("alice".to_owned()),
                email: Some("alice@example.com".to_owned()),
                password_stdin: false,
            }),
        })
    }

    fn local_only_logout_command() -> RootCommand {
        RootCommand::Auth(AuthArgs {
            command: AuthCommand::Logout(LogoutArgs {
                all: false,
                local_only: true,
                confirmation: ConfirmationArgs::assume_yes(),
            }),
        })
    }

    async fn mount_auth_success(server: &MockServer, endpoint: &str) {
        Mock::given(method("POST"))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "user-1",
                "token": "server-token",
                "tokenExpires": "2030-01-02T03:04:05Z"
            })))
            .mount(server)
            .await;
    }

    fn require_send<T: Send>(_: T) {}

    #[test]
    fn execute_future_is_send() {
        let app = App::production(ServerSelection::new(
            Some("https://wekan.example".to_owned()),
            None,
            false,
        ));
        let command = RootCommand::Auth(AuthArgs {
            command: AuthCommand::Logout(LogoutArgs {
                all: false,
                local_only: true,
                confirmation: ConfirmationArgs::assume_yes(),
            }),
        });

        require_send(app.execute(command));
    }

    #[tokio::test]
    async fn local_only_logout_removes_an_orphaned_credential_without_recreating_the_profile() {
        let directory = test_directory("orphaned-local-only-logout");
        let profile_store = FileProfileStore::at(directory.clone());
        let credential_namespace = profile_store.credential_namespace().unwrap();
        let orphaned_account = format!("profile:{credential_namespace}:orphaned");
        let credentials = RecordingCredentialStore::default();
        credentials
            .state
            .accounts
            .lock()
            .unwrap()
            .push(orphaned_account.clone());
        let app = App::with_profile_store(
            ServerSelection::new(
                Some("https://wekan.example".to_owned()),
                Some("orphaned".to_owned()),
                false,
            ),
            credentials.clone(),
            FixedSecretInput::default(),
            profile_store.clone(),
        );

        let success = app.execute(local_only_logout_command()).await.unwrap();
        let CommandSuccess::Logout(success) = success else {
            panic!("expected logout success")
        };
        assert_eq!(success.server, "https://wekan.example/");
        assert_eq!(success.profile, "orphaned");
        assert_eq!(success.logout_scope, LogoutScope::LocalOnly);
        assert!(!success.remote_logout_completed);
        assert!(success.local_credential_removed);
        assert!(
            !credentials
                .state
                .accounts
                .lock()
                .unwrap()
                .contains(&orphaned_account)
        );
        assert!(profile_store.read().unwrap().document().is_empty());

        drop(app);
        fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn remote_logout_cannot_target_an_orphaned_credential() {
        let directory = test_directory("orphaned-remote-logout");
        let profile_store = FileProfileStore::at(directory.clone());
        let credentials = RecordingCredentialStore::default();
        let app = App::with_profile_store(
            ServerSelection::new(
                Some("https://wekan.example".to_owned()),
                Some("orphaned".to_owned()),
                false,
            ),
            credentials,
            FixedSecretInput::default(),
            profile_store.clone(),
        );

        let error = app
            .execute(RootCommand::Auth(AuthArgs {
                command: AuthCommand::Logout(LogoutArgs {
                    all: false,
                    local_only: false,
                    confirmation: ConfirmationArgs::assume_yes(),
                }),
            }))
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::ProfileNotFound);
        assert_eq!(error.details().profile.as_deref(), Some("orphaned"));
        assert!(profile_store.read().unwrap().document().is_empty());

        drop(app);
        fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn successful_login_initializes_and_activates_the_first_default_profile() {
        let server = MockServer::start().await;
        mount_auth_success(&server, "/users/login").await;
        let directory = test_directory("first-default");
        let profile_store = FileProfileStore::at(directory.clone());
        let credentials = RecordingCredentialStore::default();
        let app = App::with_profile_store(
            ServerSelection::new(Some(server.uri()), None, false),
            credentials.clone(),
            FixedSecretInput::default(),
            profile_store.clone(),
        );

        let success = app.execute(login_command()).await.unwrap();
        let CommandSuccess::Login(success) = success else {
            panic!("expected login success")
        };
        assert_eq!(success.profile, "default");
        assert!(success.profile_created);
        assert!(success.profile_active);
        let snapshot = profile_store.read().unwrap();
        assert_eq!(snapshot.document().active_profile(), Some("default"));
        assert_eq!(
            snapshot.document().profile("default").unwrap().server(),
            format!("{}/", server.uri())
        );
        assert_eq!(credentials.state.accounts.lock().unwrap().len(), 1);

        drop(snapshot);
        drop(app);
        fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn successful_registration_creates_a_later_profile_without_activating_it() {
        let server = MockServer::start().await;
        mount_auth_success(&server, "/users/register").await;
        let directory = test_directory("later-registration");
        let profile_store = FileProfileStore::at(directory.clone());
        let mut document = ProfileDocument::default();
        document.insert(
            "personal".to_owned(),
            Profile::new("https://personal.example/".to_owned()),
        );
        document.set_active_profile(Some("personal".to_owned()));
        profile_store
            .lock_mutation()
            .unwrap()
            .save(document)
            .unwrap();
        let credentials = RecordingCredentialStore::default();
        let app = App::with_profile_store(
            ServerSelection::new(Some(server.uri()), Some("work".to_owned()), false),
            credentials.clone(),
            FixedSecretInput::default(),
            profile_store.clone(),
        );

        let success = app.execute(registration_command()).await.unwrap();
        let CommandSuccess::Registration(success) = success else {
            panic!("expected registration success")
        };
        assert_eq!(success.profile, "work");
        assert!(success.profile_created);
        assert!(!success.profile_active);
        let snapshot = profile_store.read().unwrap();
        assert_eq!(snapshot.document().active_profile(), Some("personal"));
        assert_eq!(
            snapshot.document().profile("work").unwrap().server(),
            format!("{}/", server.uri())
        );
        assert!(
            credentials
                .state
                .accounts
                .lock()
                .unwrap()
                .iter()
                .any(|account| account.ends_with(":work"))
        );

        drop(snapshot);
        drop(app);
        fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn rejected_malformed_and_transport_failures_do_not_create_profiles() {
        let rejected_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/users/login"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": "login-failed",
                "reason": "rejected"
            })))
            .mount(&rejected_server)
            .await;
        let malformed_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/users/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "incomplete"})))
            .mount(&malformed_server)
            .await;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let transport_url = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);

        for (label, server_url, expected_code) in [
            ("rejected", rejected_server.uri(), ErrorCode::LoginRejected),
            (
                "malformed",
                malformed_server.uri(),
                ErrorCode::ProtocolError,
            ),
            ("transport", transport_url, ErrorCode::TransportError),
        ] {
            let directory = test_directory(label);
            let profile_store = FileProfileStore::at(directory.clone());
            let credentials = RecordingCredentialStore::default();
            let app = App::with_profile_store(
                ServerSelection::new(Some(server_url), None, false),
                credentials.clone(),
                FixedSecretInput::default(),
                profile_store.clone(),
            );

            let error = app.execute(login_command()).await.unwrap_err();
            assert_eq!(error.code(), expected_code);
            assert_eq!(error.details().profile_created, Some(false));
            assert_eq!(error.details().profile_active, Some(false));
            assert!(profile_store.read().unwrap().document().is_empty());
            assert_eq!(credentials.state.create_calls.load(Ordering::SeqCst), 0);

            drop(app);
            fs::remove_dir_all(directory).unwrap();
        }
    }

    #[tokio::test]
    async fn credential_save_failure_retains_the_new_profile_and_reports_partial_state() {
        let server = MockServer::start().await;
        mount_auth_success(&server, "/users/login").await;
        let directory = test_directory("credential-save-failure");
        let profile_store = FileProfileStore::at(directory.clone());
        let credentials = RecordingCredentialStore::default();
        credentials.state.fail_create.store(true, Ordering::SeqCst);
        let app = App::with_profile_store(
            ServerSelection::new(Some(server.uri()), None, false),
            credentials.clone(),
            FixedSecretInput::default(),
            profile_store.clone(),
        );

        let error = app.execute(login_command()).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
        assert_eq!(error.details().session_created, Some(true));
        assert_eq!(error.details().profile_created, Some(true));
        assert_eq!(error.details().profile_active, Some(true));
        let snapshot = profile_store.read().unwrap();
        assert!(snapshot.document().profile("default").is_some());
        assert_eq!(snapshot.document().active_profile(), Some("default"));

        drop(snapshot);
        drop(app);
        fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn late_credential_conflict_never_replaces_the_entry_and_reports_remote_success() {
        let server = MockServer::start().await;
        mount_auth_success(&server, "/users/login").await;
        let directory = test_directory("late-credential-conflict");
        let profile_store = FileProfileStore::at(directory.clone());
        let credentials = RecordingCredentialStore::default();
        credentials
            .state
            .late_conflict
            .store(true, Ordering::SeqCst);
        let app = App::with_profile_store(
            ServerSelection::new(Some(server.uri()), None, false),
            credentials.clone(),
            FixedSecretInput::default(),
            profile_store.clone(),
        );

        let error = app.execute(login_command()).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::CredentialAlreadyExists);
        assert_eq!(error.details().session_created, Some(true));
        assert_eq!(error.details().credential_stored, Some(true));
        assert_eq!(error.details().profile_created, Some(true));
        assert_eq!(error.details().profile_active, Some(true));
        assert!(error.message().contains("--local-only"));
        assert!(error.message().contains("may already have been created"));
        assert!(
            profile_store
                .read()
                .unwrap()
                .document()
                .profile("default")
                .is_some()
        );
        assert_eq!(credentials.state.accounts.lock().unwrap().len(), 1);

        drop(app);
        fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn profile_save_failure_prevents_credential_storage() {
        let server = MockServer::start().await;
        mount_auth_success(&server, "/users/login").await;
        let credentials = RecordingCredentialStore::default();
        let app = App::with_profile_store(
            ServerSelection::new(Some(server.uri()), None, false),
            credentials.clone(),
            FixedSecretInput::default(),
            FailingProfileStore {
                after_install: false,
            },
        );

        let error = app.execute(login_command()).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::ConfigurationError);
        assert_eq!(error.details().session_created, Some(true));
        assert_eq!(error.details().profile_created, Some(false));
        assert_eq!(error.details().profile_active, Some(false));
        assert_eq!(credentials.state.create_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn profile_finalization_failure_reports_the_installed_profile_state() {
        let server = MockServer::start().await;
        mount_auth_success(&server, "/users/login").await;
        let credentials = RecordingCredentialStore::default();
        let app = App::with_profile_store(
            ServerSelection::new(Some(server.uri()), None, false),
            credentials.clone(),
            FixedSecretInput::default(),
            FailingProfileStore {
                after_install: true,
            },
        );

        let error = app.execute(login_command()).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::ConfigurationError);
        assert_eq!(error.details().session_created, Some(true));
        assert_eq!(error.details().profile_created, Some(true));
        assert_eq!(error.details().profile_active, Some(true));
        assert!(error.message().contains("installed"));
        assert_eq!(credentials.state.create_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn simultaneous_missing_profile_logins_create_one_profile_and_one_credential() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/users/login"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(100))
                    .set_body_json(json!({
                        "id": "user-1",
                        "token": "server-token",
                        "tokenExpires": "2030-01-02T03:04:05Z"
                    })),
            )
            .expect(1)
            .mount(&server)
            .await;
        let directory = test_directory("simultaneous-initialization");
        let profile_store = FileProfileStore::at(directory.clone());
        let credentials = RecordingCredentialStore::default();
        let first = App::with_profile_store(
            ServerSelection::new(Some(server.uri()), None, false),
            credentials.clone(),
            FixedSecretInput::default(),
            profile_store.clone(),
        );
        let second = App::with_profile_store(
            ServerSelection::new(Some(server.uri()), None, false),
            credentials.clone(),
            FixedSecretInput::default(),
            profile_store.clone(),
        );

        let first = tokio::spawn(async move { first.execute(login_command()).await });
        let second = tokio::spawn(async move { second.execute(login_command()).await });
        let (first, second) = tokio::join!(first, second);
        let first = first.unwrap();
        let second = second.unwrap();
        let successes = [&first, &second]
            .into_iter()
            .filter(|result| result.is_ok())
            .count();
        assert_eq!(successes, 1);
        let conflict = [first, second].into_iter().find_map(Result::err).unwrap();
        assert_eq!(conflict.code(), ErrorCode::CredentialAlreadyExists);
        assert_eq!(conflict.details().session_created, Some(false));
        assert_eq!(profile_store.read().unwrap().document().profiles().len(), 1);
        assert_eq!(credentials.state.accounts.lock().unwrap().len(), 1);
        server.verify().await;

        fs::remove_dir_all(directory).unwrap();
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
