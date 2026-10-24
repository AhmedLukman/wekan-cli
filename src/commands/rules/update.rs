use crate::{
    client::{WekanClientFactory, rules::UpdateRuleRequest},
    command_result::{CommandSuccess, RuleSubmittedField as Field, RuleUpdateSuccess},
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
    after_help = "Wekan shallow-merges supplied trigger/action fields; omitted fields remain. Including activityType can reset omitted matching fields to '*'. Empty definitions send {}. Earlier writes may persist after a later failure."
)]
pub struct UpdateArgs {
    #[command(flatten)]
    target: super::Target,
    #[command(flatten)]
    definition: super::definition::DefinitionArgs,
}
pub(super) async fn execute(
    args: UpdateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let request: UpdateRuleRequest = args.definition.read().await?;
    let mut fields = Vec::new();
    for (present, field) in [
        (request.title.is_some(), Field::Title),
        (request.enabled.is_some(), Field::Enabled),
        (request.trigger.is_some(), Field::Trigger),
        (request.action.is_some(), Field::Action),
    ] {
        if present {
            fields.push(field);
        }
    }
    let context = AuthenticatedContext::load(factory, store)?;
    let d = context
        .client()
        .update_rule(
            &args.target.board_id,
            &args.target.rule_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &Redactor::with_secret(context.record().token()),
                "rule update",
                "the Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::RuleUpdated(RuleUpdateSuccess {
        board_id: args.target.board_id,
        rule_id: d.rule_id,
        submitted_fields: fields,
    }))
}
