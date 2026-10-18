use crate::{
    client::{WekanClientFactory, dependencies::CreateDependencyRequest},
    command_result::{CommandSuccess, DependencyCreateSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
#[command(
    after_help = "Existing target metadata is merged. Omitted, empty or null metadata retains existing/default values.\n\nExample:\n  wekan dependency create target-card-id --board board-id --card source-card-id --type blocks"
)]
pub struct CreateArgs {
    #[command(flatten)]
    target: super::MutationTarget,
    #[command(flatten)]
    metadata: super::MetadataArgs,
}

pub(super) async fn execute(
    args: CreateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let request = CreateDependencyRequest {
        target_card_id: args.target.target_card_id,
        metadata: args.metadata.into_request(),
    };
    let response = context
        .client()
        .create_dependency(
            &args.target.board_id,
            &args.target.card_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &Redactor::with_secret(context.record().token()),
                "dependency creation",
                "the Wekan board or source card was not found",
            )
        })?;
    Ok(CommandSuccess::DependencyCreated(DependencyCreateSuccess {
        board_id: args.target.board_id,
        card_id: response.card_id,
        target_card_id: response.target_card_id,
        dependency_type: response.dependency_type.as_str().to_owned(),
        color: response.color,
        icon: response.icon,
    }))
}
