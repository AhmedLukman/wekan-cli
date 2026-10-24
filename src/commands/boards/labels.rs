use crate::{
    client::{
        WekanClientFactory,
        boards::labels::{AddBoardLabelRequest, LABEL_COLORS},
    },
    command_result::{BoardDetailLabel, BoardLabelAdded, BoardLabelCollection, CommandSuccess},
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
pub struct LabelArgs {
    #[command(subcommand)]
    pub command: LabelCommand,
}

#[derive(Debug, Subcommand)]
pub enum LabelCommand {
    /// List labels in stored board order.
    List(Target),
    /// Add or reuse a label with the same name and color. Requires board administration.
    Add(AddArgs),
}

#[derive(Debug, Args)]
pub struct Target {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    board_id: String,
}

#[derive(Debug, Args)]
pub struct AddArgs {
    #[command(flatten)]
    target: Target,
    /// Label name, sent unchanged; omission creates an empty name.
    #[arg(long)]
    name: Option<String>,
    /// Named Wekan REST palette color; omission defaults to green.
    #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(LABEL_COLORS))]
    color: Option<String>,
}

pub(super) async fn dispatch(
    command: LabelCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    match command {
        LabelCommand::List(target) => {
            let board = context
                .client()
                .board(&target.board_id, context.record().token())
                .await
                .map_err(|error| {
                    map_client_error_with_not_found(
                        error,
                        &redactor,
                        "board label listing",
                        "the requested Wekan board was not found",
                    )
                })?;
            Ok(CommandSuccess::BoardLabels(BoardLabelCollection {
                board_id: target.board_id,
                labels: board
                    .labels
                    .into_iter()
                    .map(|label| BoardDetailLabel {
                        due_at: label.due_at,
                        shared_label_id: label.shared_label_id,
                        label_id: label.label_id,
                        name: label.name,
                        color: label.color,
                    })
                    .collect(),
            }))
        }
        LabelCommand::Add(args) => {
            let label_id = context
                .client()
                .add_board_label(
                    &args.target.board_id,
                    &AddBoardLabelRequest {
                        name: args.name,
                        color: args.color,
                    },
                    context.record().token(),
                )
                .await
                .map_err(|error| {
                    map_client_error_with_not_found(
                        error,
                        &redactor,
                        "board label addition",
                        "the requested Wekan board was not found",
                    )
                })?;
            Ok(CommandSuccess::BoardLabelAdded(BoardLabelAdded {
                board_id: args.target.board_id,
                label_id,
            }))
        }
    }
}
