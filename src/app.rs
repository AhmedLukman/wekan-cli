use crate::{
    command_result::CommandSuccess,
    commands::{
        self, PreparedCommand, RootCommand,
        auth::{AuthCommand, PreparedAuthCommand},
    },
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
            RootCommand::Api(args) => self.execute_api(args).await,
            RootCommand::Auth(args) => self.execute_auth(args).await,
            RootCommand::User(args) => self.execute_user(args).await,
            RootCommand::Board(args) => self.execute_board(args).await,
            RootCommand::List(args) => self.execute_list(args).await,
            RootCommand::Card(args) => self.execute_card(args).await,
            RootCommand::Comment(args) => self.execute_comment(args).await,
            RootCommand::Swimlane(args) => self.execute_swimlane(args).await,
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

    async fn execute_api(
        &self,
        args: crate::commands::api::ApiArgs,
    ) -> Result<CommandSuccess, AppError> {
        let target = self.target_resolver.resolve(
            &self.profile_store,
            crate::config::MissingProfileResolution::Reject,
        )?;
        let profile = target.profile().to_owned();
        commands::dispatch(
            PreparedCommand::Api {
                args,
                client_factory: target.client_factory(),
            },
            &self.credential_store,
            &self.secret_input,
            &self.profile_store,
            &self.confirmation,
        )
        .await
        .map_err(|error| error.with_profile_context(profile))
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
        let prepared = match args.command {
            AuthCommand::Login(args) => Ok(PreparedAuthCommand::Login {
                args,
                target: &mut target,
            }),
            AuthCommand::Logout(args) => Ok(PreparedAuthCommand::Logout {
                args,
                client_factory: target.client_factory(),
            }),
            AuthCommand::Register(args) => Ok(PreparedAuthCommand::Register {
                args,
                target: &mut target,
            }),
            AuthCommand::Status(args) => Ok(PreparedAuthCommand::Status {
                args,
                client_factory: target.client_factory(),
            }),
        };
        let result = match prepared {
            Ok(command) => {
                commands::dispatch(
                    PreparedCommand::Auth { command },
                    &self.credential_store,
                    &self.secret_input,
                    &self.profile_store,
                    &self.confirmation,
                )
                .await
            }
            Err(error) => Err(error),
        };
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

    async fn execute_user(
        &self,
        args: crate::commands::users::UserArgs,
    ) -> Result<CommandSuccess, AppError> {
        args.command.validate()?;
        let missing_profile_resolution = args.command.missing_profile_resolution();
        let target = self
            .target_resolver
            .resolve(&self.profile_store, missing_profile_resolution)?;
        let profile = target.profile().to_owned();
        commands::dispatch(
            PreparedCommand::User {
                args,
                client_factory: target.client_factory(),
            },
            &self.credential_store,
            &self.secret_input,
            &self.profile_store,
            &self.confirmation,
        )
        .await
        .map_err(|error| error.with_profile_context(profile))
    }

    async fn execute_board(
        &self,
        args: crate::commands::boards::BoardArgs,
    ) -> Result<CommandSuccess, AppError> {
        let missing_profile_resolution = args.command.missing_profile_resolution();
        let target = self
            .target_resolver
            .resolve(&self.profile_store, missing_profile_resolution)?;
        let profile = target.profile().to_owned();
        commands::dispatch(
            PreparedCommand::Board {
                args,
                client_factory: target.client_factory(),
            },
            &self.credential_store,
            &self.secret_input,
            &self.profile_store,
            &self.confirmation,
        )
        .await
        .map_err(|error| error.with_profile_context(profile))
    }

    async fn execute_list(
        &self,
        args: crate::commands::lists::ListArgs,
    ) -> Result<CommandSuccess, AppError> {
        let missing_profile_resolution = args.command.missing_profile_resolution();
        let target = self
            .target_resolver
            .resolve(&self.profile_store, missing_profile_resolution)?;
        let profile = target.profile().to_owned();
        commands::dispatch(
            PreparedCommand::List {
                args,
                client_factory: target.client_factory(),
            },
            &self.credential_store,
            &self.secret_input,
            &self.profile_store,
            &self.confirmation,
        )
        .await
        .map_err(|error| error.with_profile_context(profile))
    }

    async fn execute_card(
        &self,
        args: Box<crate::commands::cards::CardArgs>,
    ) -> Result<CommandSuccess, AppError> {
        let missing_profile_resolution = args.command.missing_profile_resolution();
        let target = self
            .target_resolver
            .resolve(&self.profile_store, missing_profile_resolution)?;
        let profile = target.profile().to_owned();
        commands::dispatch(
            PreparedCommand::Card {
                args,
                client_factory: target.client_factory(),
            },
            &self.credential_store,
            &self.secret_input,
            &self.profile_store,
            &self.confirmation,
        )
        .await
        .map_err(|error| error.with_profile_context(profile))
    }

    async fn execute_swimlane(
        &self,
        args: crate::commands::swimlanes::SwimlaneArgs,
    ) -> Result<CommandSuccess, AppError> {
        let missing_profile_resolution = args.command.missing_profile_resolution();
        let target = self
            .target_resolver
            .resolve(&self.profile_store, missing_profile_resolution)?;
        let profile = target.profile().to_owned();
        commands::dispatch(
            PreparedCommand::Swimlane {
                args,
                client_factory: target.client_factory(),
            },
            &self.credential_store,
            &self.secret_input,
            &self.profile_store,
            &self.confirmation,
        )
        .await
        .map_err(|error| error.with_profile_context(profile))
    }

    async fn execute_comment(
        &self,
        args: crate::commands::comments::CommentArgs,
    ) -> Result<CommandSuccess, AppError> {
        let missing_profile_resolution = args.command.missing_profile_resolution();
        let target = self
            .target_resolver
            .resolve(&self.profile_store, missing_profile_resolution)?;
        let profile = target.profile().to_owned();
        commands::dispatch(
            PreparedCommand::Comment {
                args,
                client_factory: target.client_factory(),
            },
            &self.credential_store,
            &self.secret_input,
            &self.profile_store,
            &self.confirmation,
        )
        .await
        .map_err(|error| error.with_profile_context(profile))
    }
}

#[cfg(test)]
mod tests;
