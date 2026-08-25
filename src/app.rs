use crate::{
    client::WekanClientFactory,
    command_result::CommandSuccess,
    commands::{self, RootCommand},
    credentials::{
        CredentialStore, KeyringCredentialStore, SecretInputProvider, SystemSecretInputProvider,
    },
    error::AppError,
};

pub struct App<S, P> {
    client_factory: WekanClientFactory,
    credential_store: S,
    secret_input: P,
}

impl App<KeyringCredentialStore, SystemSecretInputProvider> {
    pub const fn production(server: Option<String>, allow_insecure_http: bool) -> Self {
        Self::new(
            server,
            allow_insecure_http,
            KeyringCredentialStore,
            SystemSecretInputProvider,
        )
    }
}

impl<S, P> App<S, P>
where
    S: CredentialStore,
    P: SecretInputProvider,
{
    pub const fn new(
        server: Option<String>,
        allow_insecure_http: bool,
        credential_store: S,
        secret_input: P,
    ) -> Self {
        Self {
            client_factory: WekanClientFactory::new(server, allow_insecure_http),
            credential_store,
            secret_input,
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
            &self.secret_input,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::App;
    use crate::commands::{
        RootCommand,
        auth::{AuthArgs, AuthCommand, logout::LogoutArgs},
    };

    fn require_send<T: Send>(_: T) {}

    #[test]
    fn execute_future_is_send() {
        let app = App::production(Some("https://wekan.example".to_owned()), false);
        let command = RootCommand::Auth(AuthArgs {
            command: AuthCommand::Logout(LogoutArgs {
                all: false,
                local_only: true,
            }),
        });

        require_send(app.execute(command));
    }
}
