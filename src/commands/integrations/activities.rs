use crate::{
    client::{WekanClientFactory, integrations::IntegrationActivitiesRequest},
    command_result::{
        CommandSuccess, IntegrationActivityAction, IntegrationActivityMutationSuccess,
    },
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct ActivityArgs {
    #[command(subcommand)]
    pub command: ActivityCommand,
}

#[derive(Debug, Subcommand)]
pub enum ActivityCommand {
    /// Add subscriptions; retains existing activities, including all.
    Add(ActivityTarget),
    /// Remove subscriptions directly; removes all matching occurrences.
    Remove(ActivityTarget),
}

#[derive(Debug, Args)]
pub struct ActivityTarget {
    #[command(flatten)]
    target: super::Target,
    /// Activity string; repeat for each subscription. Sent unchanged.
    #[arg(long = "activity", required = true)]
    activities: Vec<String>,
}

pub(super) async fn dispatch(
    command: ActivityCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let (args, action) = match command {
        ActivityCommand::Add(args) => (args, IntegrationActivityAction::Add),
        ActivityCommand::Remove(args) => (args, IntegrationActivityAction::Remove),
    };
    let context = AuthenticatedContext::load(factory, store)?;
    let request = IntegrationActivitiesRequest {
        activities: args.activities,
    };
    let result = match action {
        IntegrationActivityAction::Add => {
            context
                .client()
                .add_integration_activities(
                    &args.target.board_id,
                    &args.target.integration_id,
                    &request,
                    context.record().token(),
                )
                .await
        }
        IntegrationActivityAction::Remove => {
            context
                .client()
                .remove_integration_activities(
                    &args.target.board_id,
                    &args.target.integration_id,
                    &request,
                    context.record().token(),
                )
                .await
        }
    };
    let response = result
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &Redactor::with_secret(context.record().token()),
                "integration activity mutation",
                "the Wekan board was not found",
            )
        })?
        .ok_or_else(super::missing_integration)?;
    Ok(CommandSuccess::IntegrationActivitiesChanged(
        IntegrationActivityMutationSuccess {
            board_id: args.target.board_id,
            integration_id: response.integration_id,
            action,
            requested_activities: request.activities,
            activities: response.activities,
        },
    ))
}
