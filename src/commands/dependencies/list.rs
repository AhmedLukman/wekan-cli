use crate::{
    client::WekanClientFactory,
    command_result::{
        BoardDependencyCollectionSuccess, CardDependencyCollectionSuccess, CommandSuccess,
        DependencyLine, DependencySummary,
    },
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
    after_help = "Examples:\n  wekan dependency list --board board-id\n  wekan dependency list --board board-id --card source-card-id"
)]
pub struct ListArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = super::identifier, help_heading = "Target")]
    board_id: String,
    /// Limit the request to this source card's outgoing dependencies.
    #[arg(long = "card", value_parser = super::identifier, help_heading = "Target")]
    card_id: Option<String>,
}

pub(super) async fn execute(
    args: ListArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let map_error = |error| {
        map_client_error_with_not_found(
            error,
            &redactor,
            "dependency listing",
            "the Wekan board or source card was not found",
        )
    };
    if let Some(card_id) = args.card_id {
        let dependencies = context
            .client()
            .card_dependencies(&args.board_id, &card_id, context.record().token())
            .await
            .map_err(map_error)?;
        Ok(CommandSuccess::CardDependencies(
            CardDependencyCollectionSuccess {
                board_id: args.board_id,
                card_id,
                dependencies: dependencies
                    .into_iter()
                    .map(|d| DependencySummary {
                        target_card_id: d.target_card_id,
                        dependency_type: d.dependency_type.as_str().to_owned(),
                        color: d.color,
                        icon: d.icon,
                    })
                    .collect(),
            },
        ))
    } else {
        let dependencies = context
            .client()
            .board_dependencies(&args.board_id, context.record().token())
            .await
            .map_err(map_error)?;
        Ok(CommandSuccess::BoardDependencies(
            BoardDependencyCollectionSuccess {
                board_id: args.board_id,
                dependencies: dependencies
                    .into_iter()
                    .map(|d| DependencyLine {
                        source_card_id: d.source_card_id,
                        source_title: d.source_title,
                        source_card_number: d.source_card_number,
                        target_card_id: d.target_card_id,
                        target_title: d.target_title,
                        target_card_number: d.target_card_number,
                        dependency_type: d.dependency_type.as_str().to_owned(),
                        color: d.color,
                        icon: d.icon,
                    })
                    .collect(),
            },
        ))
    }
}
