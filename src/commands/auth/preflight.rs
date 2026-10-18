use clap::Args;

use crate::commands::client_error::map_client_error;
use crate::{
    client::{WekanClientFactory, auth::AuthEndpoint},
    command_result::{AuthPreflightSuccess, CommandSuccess},
    error::AppError,
    redaction::Redactor,
};

#[derive(Debug, Args)]
pub struct PreflightArgs {
    /// Authentication route to probe anonymously: login, register, or logout.
    #[arg(value_parser = parse_endpoint)]
    pub endpoint: AuthEndpoint,
}

fn parse_endpoint(value: &str) -> Result<AuthEndpoint, String> {
    match value {
        "login" => Ok(AuthEndpoint::Login),
        "register" => Ok(AuthEndpoint::Register),
        "logout" => Ok(AuthEndpoint::Logout),
        _ => Err("endpoint must be login, register, or logout".into()),
    }
}

pub(super) async fn execute(
    args: PreflightArgs,
    factory: &WekanClientFactory,
) -> Result<CommandSuccess, AppError> {
    factory
        .create()
        .map_err(AppError::from)?
        .auth_preflight(args.endpoint)
        .await
        .map_err(|e| map_client_error(e, &Redactor::default(), "authentication preflight"))?;
    Ok(CommandSuccess::AuthPreflight(AuthPreflightSuccess {
        server: factory.server_identity().as_str().to_owned(),
        endpoint: args.endpoint,
        http_status: 200,
    }))
}
