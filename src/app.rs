use crate::{
    command_result::CommandSuccess,
    commands::{self, RootCommand},
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
        if let RootCommand::Profile(args) = command {
            if self.target_resolver.has_explicit_target() {
                return Err(AppError::invalid_input(
                    "--server and --profile cannot be used with profile-management commands",
                ));
            }
            return commands::profile::dispatch(
                args.command,
                &self.profile_store,
                &self.credential_store,
                &self.confirmation,
            );
        }
        if let RootCommand::User(args) = &command {
            args.command.validate()?;
        }
        let policy = match &command {
            RootCommand::Auth(args) => args.command.missing_profile_resolution(),
            _ => crate::config::MissingProfileResolution::Reject,
        };
        let mut target = self.target_resolver.resolve(&self.profile_store, policy)?;
        let profile = target.profile().to_owned();
        let result = match command {
            RootCommand::Api(args) => {
                commands::api::dispatch(
                    args.command,
                    target.client_factory(),
                    &self.credential_store,
                    &self.confirmation,
                )
                .await
            }
            RootCommand::User(args) => {
                commands::users::dispatch(
                    args.command,
                    target.client_factory(),
                    &self.credential_store,
                    &self.secret_input,
                    &self.confirmation,
                )
                .await
            }
            RootCommand::Board(args) => {
                commands::boards::dispatch(
                    args.command,
                    target.client_factory(),
                    &self.credential_store,
                    &self.confirmation,
                )
                .await
            }
            RootCommand::List(args) => {
                commands::lists::dispatch(
                    args.command,
                    target.client_factory(),
                    &self.credential_store,
                    &self.confirmation,
                )
                .await
            }
            RootCommand::Card(args) => {
                commands::cards::dispatch(
                    args.command,
                    target.client_factory(),
                    &self.credential_store,
                    &self.confirmation,
                )
                .await
            }
            RootCommand::Comment(args) => {
                commands::comments::dispatch(
                    args.command,
                    target.client_factory(),
                    &self.credential_store,
                    &self.confirmation,
                )
                .await
            }
            RootCommand::Swimlane(args) => {
                commands::swimlanes::dispatch(
                    args.command,
                    target.client_factory(),
                    &self.credential_store,
                    &self.confirmation,
                )
                .await
            }
            RootCommand::Auth(args) => {
                commands::auth::dispatch(
                    args.command,
                    &mut target,
                    &self.credential_store,
                    &self.secret_input,
                    &self.confirmation,
                )
                .await
            }
            RootCommand::Profile(_) => {
                unreachable!("local commands return before target resolution")
            }
        };
        result.map_err(|error| {
            let error = error.with_profile_context(profile);
            if policy.allows_initialization() {
                error.with_profile_state(target.profile_created(), target.profile_active())
            } else {
                error
            }
        })
    }
}

#[cfg(test)]
mod tests;
