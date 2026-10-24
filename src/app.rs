use crate::{
    command_result::{CommandSuccess, StartupUpdateOutcome},
    commands::{self, RootCommand},
    config::{ServerSelection, TargetResolver, profiles::FileProfileStore},
    credentials::{
        CredentialStore, KeyringCredentialStore, SecretInputProvider, SystemSecretInputProvider,
    },
    error::AppError,
    input::{ConfirmationProvider, SystemConfirmationProvider},
    output::OutputFormat,
    update::{FileCache, StartupEligibility, UpdateService},
};

pub struct App<S, P, F = FileProfileStore, C = SystemConfirmationProvider> {
    target_resolver: TargetResolver,
    credential_store: S,
    secret_input: P,
    profile_store: F,
    confirmation: C,
    update_service_factory: fn() -> Result<UpdateService, AppError>,
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
            update_service_factory: UpdateService::production,
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
            update_service_factory: self.update_service_factory,
        }
    }

    pub const fn credential_store(&self) -> &S {
        &self.credential_store
    }

    pub(crate) fn check_for_update(
        &self,
        output: OutputFormat,
        command: &RootCommand,
    ) -> impl std::future::Future<Output = Result<StartupUpdateOutcome, AppError>> + '_ {
        let eligible = StartupEligibility::production(
            output,
            matches!(command, RootCommand::SelfManagement(_)),
        );
        // Keep release HTTP state off the process entry future's stack.
        Box::pin(async move {
            if !eligible.eligible() {
                return Ok(StartupUpdateOutcome::Continue);
            }
            // Discovery and cache failures are optional startup work. Failures
            // after affirmative installation consent remain explicit errors.
            let Ok(service) = (self.update_service_factory)() else {
                return Ok(StartupUpdateOutcome::Continue);
            };
            let Ok(mut cache) = FileCache::production() else {
                return Ok(StartupUpdateOutcome::Continue);
            };
            service.automatic(&mut cache, &self.confirmation).await
        })
    }

    pub fn execute(
        &self,
        command: RootCommand,
    ) -> impl std::future::Future<Output = Result<CommandSuccess, AppError>> + '_ {
        // Keep the dispatch future off callers' stacks. The complete native
        // response models make it large enough to overflow Windows test and
        // embedding threads when nested in a multi-command workflow.
        // Select local maintenance before constructing the large API dispatch
        // future. Adding another CommandSuccess-producing await inside that
        // state machine exceeds Windows' process-entry stack budget.
        let task: std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<CommandSuccess, AppError>> + Send + '_>,
        > = if let RootCommand::SelfManagement(args) = command {
            if self.target_resolver.has_explicit_target() {
                Box::pin(async {
                    Err(AppError::invalid_input(
                        "--server and --profile cannot be used with self-management commands",
                    ))
                })
            } else {
                match (self.update_service_factory)() {
                    Ok(service) => Box::pin(commands::self_management::dispatch(
                        args.command,
                        service,
                        &self.confirmation,
                    )),
                    Err(error) => Box::pin(async move { Err(error) }),
                }
            }
        } else {
            Box::pin(self.execute_command(command))
        };
        task
    }

    async fn execute_command(&self, command: RootCommand) -> Result<CommandSuccess, AppError> {
        if let RootCommand::Skill(args) = command {
            return commands::skill::dispatch(args.command);
        }
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
        command.validate_input()?;
        let policy = match &command {
            RootCommand::Auth(args) => args.command.missing_profile_resolution(),
            RootCommand::System(_) => crate::config::MissingProfileResolution::AnonymousRequest,
            _ => crate::config::MissingProfileResolution::Reject,
        };
        let mut target = self.target_resolver.resolve(&self.profile_store, policy)?;
        let profile = target.profile().to_owned();
        let result = match command {
            RootCommand::System(args) => {
                commands::system::dispatch(args.command, target.client_factory()).await
            }
            RootCommand::Admin(args) => {
                commands::admin::dispatch(
                    args.command,
                    target.client_factory(),
                    &self.credential_store,
                    &self.secret_input,
                    &self.confirmation,
                )
                .await
            }
            RootCommand::Rule(args) => {
                commands::rules::dispatch(
                    args.command,
                    target.client_factory(),
                    &self.credential_store,
                    &self.confirmation,
                )
                .await
            }
            RootCommand::Integration(args) => {
                commands::integrations::dispatch(
                    args.command,
                    target.client_factory(),
                    &self.credential_store,
                    &self.secret_input,
                    &self.confirmation,
                )
                .await
            }
            RootCommand::Dependency(args) => {
                commands::dependencies::dispatch(
                    args.command,
                    target.client_factory(),
                    &self.credential_store,
                    &self.confirmation,
                )
                .await
            }
            RootCommand::Attachment(args) => {
                commands::attachments::dispatch(
                    args.command,
                    target.client_factory(),
                    &self.credential_store,
                    &self.confirmation,
                )
                .await
            }
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
                    &self.secret_input,
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
            RootCommand::Checklist(args) => {
                commands::checklists::dispatch(
                    args.command,
                    target.client_factory(),
                    &self.credential_store,
                    &self.confirmation,
                )
                .await
            }
            RootCommand::CustomField(args) => {
                commands::custom_fields::dispatch(
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
            RootCommand::Profile(_) | RootCommand::Skill(_) | RootCommand::SelfManagement(_) => {
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
