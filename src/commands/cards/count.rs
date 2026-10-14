use crate::{
    client::WekanClientFactory,
    command_result::{CardCountSuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
        transfer::identifier,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;
#[derive(Debug, Args)]
pub struct CountArgs {
    /// Board whose active cards are counted.
    #[arg(long = "board", value_parser = identifier)]
    pub board_id: String,
    /// Restrict the count to a list. A nonexistent list ID returns zero.
    #[arg(long = "list", value_parser = identifier)]
    pub list_id: Option<String>,
}
pub(super) async fn execute(
    args: CountArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let count = match &args.list_id {
        Some(list) => {
            context
                .client()
                .list_card_count(&args.board_id, list, context.record().token())
                .await
        }
        None => {
            context
                .client()
                .board_card_count(&args.board_id, context.record().token())
                .await
        }
    }
    .map_err(|e| {
        map_client_error_with_not_found(
            e,
            &Redactor::with_secret(context.record().token()),
            "card count",
            "the requested board was not found",
        )
    })?;
    Ok(CommandSuccess::CardCount(CardCountSuccess {
        board_id: args.board_id,
        list_id: args.list_id,
        count,
    }))
}
