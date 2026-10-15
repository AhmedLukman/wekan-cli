use crate::{
    client::{WekanClientFactory, rules::CreateRuleRequest},
    command_result::{CommandSuccess, RuleCreateSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_mutation_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;
#[derive(Debug, Args)]
#[command(
    after_help = "Creation requires trigger.activityType and action.actionType. Wekan defaults omitted/empty titles to API rule and activity matching fields to '*'. Rule creation is sequential and failures may leave trigger/action documents behind."
)]
pub struct CreateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = super::identifier)]
    board_id: String,
    #[command(flatten)]
    definition: super::definition::DefinitionArgs,
}
pub(super) async fn execute(
    args: CreateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let request: CreateRuleRequest = args.definition.read().await?;
    if request
        .trigger
        .activity_type
        .as_ref()
        .and_then(Option::as_ref)
        .is_none_or(|v| v.is_empty())
        || request
            .action
            .action_type
            .as_ref()
            .and_then(Option::as_ref)
            .is_none_or(|v| v.is_empty())
    {
        return Err(AppError::invalid_input(
            "rule creation requires nonempty string trigger.activityType and action.actionType; no request was sent",
        ));
    }
    let context = AuthenticatedContext::load(factory, store)?;
    let d = context
        .client()
        .create_rule(&args.board_id, &request, context.record().token())
        .await
        .map_err(|e| {
            map_mutation_client_error_with_not_found(
                e,
                &Redactor::with_secret(context.record().token()),
                "rule creation",
                "the Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::RuleCreated(RuleCreateSuccess {
        board_id: args.board_id,
        rule_id: d.rule_id,
        trigger_id: d.trigger_id,
        action_id: d.action_id,
    }))
}
