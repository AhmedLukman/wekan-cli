use super::features::{FeatureChanges, UpdateAllArgs, confirm_bulk, identifier};
use crate::{
    client::WekanClientFactory,
    command_result::*,
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    input::ConfirmationProvider,
    redaction::Redactor,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct TeamArgs {
    #[command(subcommand)]
    pub command: TeamCommand,
}
#[derive(Debug, Subcommand)]
pub enum TeamCommand {
    /// List identifying fields and the three feature flags (site admin).
    List,
    /// Update native feature flags on one or all teams.
    Features(FeaturesArgs),
}
#[derive(Debug, Args)]
pub struct FeaturesArgs {
    #[command(subcommand)]
    pub command: FeaturesCommand,
}
#[derive(Debug, Subcommand)]
pub enum FeaturesCommand {
    /// Update at least one feature on a single team; omitted flags are preserved.
    Update(UpdateArgs),
    /// Change one feature across every team, with confirmation or --yes.
    UpdateAll(UpdateAllArgs),
}
#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Wekan team ID.
    #[arg(long = "team", value_parser = identifier)]
    id: String,
    #[command(flatten)]
    changes: FeatureChanges,
}

pub(super) async fn execute(
    command: TeamCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    match command {
        TeamCommand::List => {
            let teams = context
                .client()
                .admin_teams(context.record().token())
                .await
                .map_err(|e| map_client_error(e, &redactor, "admin team list"))?;
            Ok(CommandSuccess::AdminTeams(AdminTeamCollectionSuccess {
                teams,
            }))
        }
        TeamCommand::Features(args) => match args.command {
            FeaturesCommand::Update(args) => {
                let team = context
                    .client()
                    .update_team_features(
                        &args.id,
                        &args.changes.team_request(),
                        context.record().token(),
                    )
                    .await
                    .map_err(|e| map_client_error(e, &redactor, "admin team features update"))?;
                Ok(CommandSuccess::AdminTeamUpdated(Box::new(
                    AdminTeamUpdateSuccess {
                        team,
                        submitted_fields: args.changes.submitted_fields(),
                    },
                )))
            }
            FeaturesCommand::UpdateAll(args) => {
                let operation = DestructiveOperation::AdminTeamFeaturesUpdateAll;
                let Some(guard) = confirm_bulk(&args, confirmation, &context, store, operation)?
                else {
                    return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
                        operation,
                    )));
                };
                let field = args.field.into();
                let result = context
                    .client()
                    .update_all_team_features(field, args.value, context.record().token())
                    .await;
                drop(guard);
                let response = result.map_err(|e| {
                    map_client_error(e, &redactor, "admin team features update-all")
                })?;
                Ok(CommandSuccess::AdminTeamFeaturesUpdatedAll(
                    AdminFeaturesUpdateAllSuccess {
                        field,
                        value: args.value,
                        updated: response.updated,
                    },
                ))
            }
        },
    }
}
