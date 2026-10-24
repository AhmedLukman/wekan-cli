use super::super::custom_fields::non_empty;
use crate::{
    client::{
        WekanClientFactory,
        boards::presentation::{BoardViewVisibilityPatch, UpdateBoardViewSettingsRequest},
    },
    command_result::{BoardCardFieldOrderSuccess, BoardViewSettingsSuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::{ArgGroup, Args, Subcommand};
use std::collections::BTreeMap;

#[derive(Debug, Args)]
pub struct ViewSettingsArgs {
    #[command(subcommand)]
    pub command: ViewSettingsCommand,
}
#[derive(Debug, Subcommand)]
pub enum ViewSettingsCommand {
    /// Read visibility, defaults, and order for every board view.
    Get(Target),
    /// Update view visibility, defaults, or menu order.
    Update(ViewUpdateArgs),
}
#[derive(Debug, Args)]
pub struct Target {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    board_id: String,
}
#[derive(Debug, Args)]
#[command(group(ArgGroup::new("updates").required(true).multiple(true).args(["view_settings", "default_public_view", "default_private_view", "view_order"])))]
pub struct ViewUpdateArgs {
    #[command(flatten)]
    target: Target,
    /// JSON object of view keys and camelCase showOnPublic/showOnPrivate booleans.
    #[arg(long, value_parser = visibility_json, value_name = "JSON")]
    view_settings: Option<BTreeMap<String, BoardViewVisibilityPatch>>,
    /// View to open for public visitors.
    #[arg(long, value_parser = non_empty)]
    default_public_view: Option<String>,
    /// View to open for private visitors.
    #[arg(long, value_parser = non_empty)]
    default_private_view: Option<String>,
    /// View key in menu order; repeat to supply the sequence.
    #[arg(long, value_parser = non_empty)]
    view_order: Vec<String>,
}
fn visibility_json(value: &str) -> Result<BTreeMap<String, BoardViewVisibilityPatch>, String> {
    serde_json::from_str(value).map_err(|_| {
        "expected an object of view keys with showOnPublic/showOnPrivate booleans".to_owned()
    })
}
pub(crate) async fn views(
    command: ViewSettingsCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let (board_id, result) = match command {
        ViewSettingsCommand::Get(target) => {
            let result = context
                .client()
                .board_view_settings(&target.board_id, context.record().token())
                .await;
            (target.board_id, result)
        }
        ViewSettingsCommand::Update(args) => {
            let request = UpdateBoardViewSettingsRequest {
                board_view_settings: args.view_settings,
                default_public_board_view: args.default_public_view,
                default_private_board_view: args.default_private_view,
                board_view_order: (!args.view_order.is_empty()).then_some(args.view_order),
            };
            let result = context
                .client()
                .update_board_view_settings(
                    &args.target.board_id,
                    &request,
                    context.record().token(),
                )
                .await;
            (args.target.board_id, result)
        }
    };
    let settings = result.map_err(|e| {
        map_client_error_with_not_found(
            e,
            &redactor,
            "board view settings",
            "the requested Wekan board was not found",
        )
    })?;
    Ok(CommandSuccess::BoardViewSettings(
        BoardViewSettingsSuccess { board_id, settings },
    ))
}

#[derive(Debug, Args)]
pub struct CardFieldOrderArgs {
    #[command(subcommand)]
    pub command: CardFieldOrderCommand,
}
#[derive(Debug, Subcommand)]
pub enum CardFieldOrderCommand {
    /// Read the normalized card section order and available keys.
    Get(Target),
    /// Set a card section order or restore the default order.
    Update(OrderUpdateArgs),
}
#[derive(Debug, Args)]
#[command(group(ArgGroup::new("order").required(true).args(["fields","reset"])))]
pub struct OrderUpdateArgs {
    #[command(flatten)]
    target: Target,
    /// Card section key in desired order; repeat to supply the sequence.
    #[arg(long="field",value_parser=non_empty)]
    fields: Vec<String>,
    /// Reset to Wekan's default section order.
    #[arg(long)]
    reset: bool,
}
pub(crate) async fn order(
    command: CardFieldOrderCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let (board_id, result) = match command {
        CardFieldOrderCommand::Get(target) => {
            let result = context
                .client()
                .board_card_field_order(&target.board_id, context.record().token())
                .await;
            (target.board_id, result)
        }
        CardFieldOrderCommand::Update(args) => {
            let fields = if args.reset { Vec::new() } else { args.fields };
            let result = context
                .client()
                .update_board_card_field_order(
                    &args.target.board_id,
                    &fields,
                    context.record().token(),
                )
                .await;
            (args.target.board_id, result)
        }
    };
    let settings = result.map_err(|e| {
        map_client_error_with_not_found(
            e,
            &redactor,
            "board card field order",
            "the requested Wekan board was not found",
        )
    })?;
    Ok(CommandSuccess::BoardCardFieldOrder(
        BoardCardFieldOrderSuccess { board_id, settings },
    ))
}
