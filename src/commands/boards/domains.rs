use crate::{
    client::WekanClientFactory,
    command_result::{
        BoardDomainAction, BoardDomainCollectionSuccess, BoardDomainMutationSuccess, CommandSuccess,
    },
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
        custom_fields::non_empty,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct DomainArgs {
    #[command(subcommand)]
    pub command: DomainCommand,
}

#[derive(Debug, Subcommand)]
pub enum DomainCommand {
    /// List domains, including inactive records. Requires a site administrator.
    List(Target),
    /// Request sharing with a domain. Wekan v11.06 fails with not-authorized.
    Add(MutationArgs),
    /// Request removal of domain sharing. Wekan v11.06 fails with not-authorized.
    Remove(RemoveArgs),
}

#[derive(Debug, Args)]
pub struct Target {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    board_id: String,
}

#[derive(Debug, Args)]
pub struct MutationArgs {
    #[command(flatten)]
    target: Target,
    /// Email domain, sent unchanged for Wekan to normalize and validate.
    domain: String,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    #[command(flatten)]
    target: Target,
    /// Email domain, sent unchanged as one encoded path segment.
    #[arg(value_parser = domain_path)]
    domain: String,
}

fn domain_path(value: &str) -> Result<String, String> {
    if value.is_empty() || matches!(value, "." | "..") {
        Err("domain path values must not be empty or dot-only".into())
    } else {
        Ok(value.to_owned())
    }
}

pub(super) async fn dispatch(
    command: DomainCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    if let DomainCommand::List(target) = command {
        let domains = context
            .client()
            .board_domains(&target.board_id, context.record().token())
            .await
            .map_err(|e| {
                map_client_error_with_not_found(
                    e,
                    &redactor,
                    "board domain list",
                    "the requested Wekan board was not found",
                )
            })?;
        return Ok(CommandSuccess::BoardDomains(BoardDomainCollectionSuccess {
            board_id: target.board_id,
            domains,
        }));
    }
    let (target, domain, action, response) = match command {
        DomainCommand::Add(args) => {
            let response = context
                .client()
                .add_board_domain(
                    &args.target.board_id,
                    &args.domain,
                    context.record().token(),
                )
                .await;
            (args.target, args.domain, BoardDomainAction::Add, response)
        }
        DomainCommand::Remove(args) => {
            let response = context
                .client()
                .remove_board_domain(
                    &args.target.board_id,
                    &args.domain,
                    context.record().token(),
                )
                .await;
            (
                args.target,
                args.domain,
                BoardDomainAction::Remove,
                response,
            )
        }
        DomainCommand::List(_) => unreachable!(),
    };
    let operation = match action {
        BoardDomainAction::Add => "board domain add",
        BoardDomainAction::Remove => "board domain remove",
    };
    let domains = response.map_err(|e| {
        map_client_error_with_not_found(
            e,
            &redactor,
            operation,
            "the requested Wekan board was not found",
        )
    })?;
    Ok(CommandSuccess::BoardDomainChanged(
        BoardDomainMutationSuccess {
            board_id: target.board_id,
            domain,
            action,
            domains,
        },
    ))
}
