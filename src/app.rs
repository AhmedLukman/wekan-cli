use crate::{
    client::WekanClientFactory,
    commands::{self, RootCommand},
    credentials::{
        CredentialStore, KeyringCredentialStore, PasswordProvider, SystemPasswordProvider,
    },
    error::AppError,
    output::CommandSuccess,
};

pub struct App<S, P> {
    client_factory: WekanClientFactory,
    credential_store: S,
    password_provider: P,
}

impl App<KeyringCredentialStore, SystemPasswordProvider> {
    pub const fn production(server: Option<String>, allow_insecure_http: bool) -> Self {
        Self::new(
            server,
            allow_insecure_http,
            KeyringCredentialStore,
            SystemPasswordProvider,
        )
    }
}

impl<S, P> App<S, P>
where
    S: CredentialStore,
    P: PasswordProvider,
{
    pub const fn new(
        server: Option<String>,
        allow_insecure_http: bool,
        credential_store: S,
        password_provider: P,
    ) -> Self {
        Self {
            client_factory: WekanClientFactory::new(server, allow_insecure_http),
            credential_store,
            password_provider,
        }
    }

    pub const fn credential_store(&self) -> &S {
        &self.credential_store
    }

    pub async fn execute(&self, command: RootCommand) -> Result<CommandSuccess, AppError> {
        commands::dispatch(
            command,
            &self.client_factory,
            &self.credential_store,
            &self.password_provider,
        )
        .await
    }
}
