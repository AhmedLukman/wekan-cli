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
pub struct OrganizationArgs {
    #[command(subcommand)]
    pub command: OrganizationCommand,
}
#[derive(Debug, Subcommand)]
pub enum OrganizationCommand {
    /// List identifying fields and the three feature flags (site admin).
    List,
    /// Update native feature flags on one or all organizations.
    Features(FeaturesArgs),
}
#[derive(Debug, Args)]
pub struct FeaturesArgs {
    #[command(subcommand)]
    pub command: FeaturesCommand,
}
#[derive(Debug, Subcommand)]
pub enum FeaturesCommand {
    /// Update at least one feature on a single org; omitted flags are preserved.
    Update(UpdateArgs),
    /// Change one feature across every org, with confirmation or --yes.
    UpdateAll(UpdateAllArgs),
}
#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Wekan org ID.
    #[arg(long = "org", value_parser = identifier)]
    id: String,
    #[command(flatten)]
    changes: FeatureChanges,
}

pub(super) async fn execute(
    command: OrganizationCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    match command {
        OrganizationCommand::List => {
            let organizations = context
                .client()
                .admin_organizations(context.record().token())
                .await
                .map_err(|e| map_client_error(e, &redactor, "admin org list"))?;
            Ok(CommandSuccess::AdminOrganizations(
                AdminOrganizationCollectionSuccess { organizations },
            ))
        }
        OrganizationCommand::Features(args) => match args.command {
            FeaturesCommand::Update(args) => {
                let organization = context
                    .client()
                    .update_organization_features(
                        &args.id,
                        &args.changes.organization_request(),
                        context.record().token(),
                    )
                    .await
                    .map_err(|e| map_client_error(e, &redactor, "admin org features update"))?;
                Ok(CommandSuccess::AdminOrganizationUpdated(Box::new(
                    AdminOrganizationUpdateSuccess {
                        organization,
                        submitted_fields: args.changes.submitted_fields(),
                    },
                )))
            }
            FeaturesCommand::UpdateAll(args) => {
                let operation = DestructiveOperation::AdminOrganizationFeaturesUpdateAll;
                let Some(guard) = confirm_bulk(&args, confirmation, &context, store, operation)?
                else {
                    return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
                        operation,
                    )));
                };
                let field = args.field.into();
                let result = context
                    .client()
                    .update_all_organization_features(field, args.value, context.record().token())
                    .await;
                drop(guard);
                let response = result
                    .map_err(|e| map_client_error(e, &redactor, "admin org features update-all"))?;
                Ok(CommandSuccess::AdminOrganizationFeaturesUpdatedAll(
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
