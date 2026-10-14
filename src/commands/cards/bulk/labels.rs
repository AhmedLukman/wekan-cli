use crate::{
    client::{
        WekanClientFactory,
        cards::bulk::{BulkLabelsError, BulkLabelsRequest},
    },
    command_result::{BulkCardLabelUpdate, BulkCardLabelsResult, CardBatchResult, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_mutation_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    redaction::Redactor,
};
use clap::{ArgGroup, Args};

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("label_changes").args(["add_label_ids", "remove_label_ids"]).required(true).multiple(true)))]
pub struct LabelsArgs {
    #[command(flatten)]
    target: super::Target,
    /// Label ID to add; may be repeated. Added IDs must exist on the board.
    #[arg(long = "add-label", value_parser = super::body_id)]
    add_label_ids: Vec<String>,
    /// Label ID to remove; may be repeated. Removal precedes addition.
    #[arg(long = "remove-label", value_parser = super::body_id)]
    remove_label_ids: Vec<String>,
}

pub(super) async fn execute(
    args: LabelsArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    super::validate_count(args.target.card_ids.len())?;
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let request = BulkLabelsRequest {
        card_ids: args.target.card_ids,
        add_label_ids: args.add_label_ids,
        remove_label_ids: args.remove_label_ids,
    };
    let result = context
        .client()
        .bulk_card_labels(&args.target.board_id, &request, context.record().token())
        .await
        .map_err(|error| match error {
            BulkLabelsError::Client(error) => map_mutation_client_error_with_not_found(
                error,
                &redactor,
                "bulk card labels",
                "the requested Wekan board was not found",
            ),
            BulkLabelsError::InvalidLabels {
                server_error,
                invalid_label_ids,
            } => AppError::new(
                ErrorCode::ServerError,
                "Wekan rejected labels absent from the board",
                StableExitCode::Server,
            )
            .with_details(ErrorDetails {
                http_status: Some(400),
                server_error: Some(redactor.redact(&server_error)),
                invalid_label_ids: Some(
                    invalid_label_ids
                        .iter()
                        .map(|id| redactor.redact(id))
                        .collect(),
                ),
                ..ErrorDetails::default()
            }),
        })?;
    super::finish(CardBatchResult::CardBulkLabels(BulkCardLabelsResult {
        board_id: args.target.board_id,
        updated: result
            .updated
            .into_iter()
            .map(|item| BulkCardLabelUpdate {
                card_id: item.card_id,
                label_ids: item.label_ids,
            })
            .collect(),
        not_found: result.not_found,
    }))
}
