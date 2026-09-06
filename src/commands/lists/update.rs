use clap::{ArgGroup, Args};
use serde_json::Number;

use crate::{
    client::{ListWipLimit, UpdateListRequest, WekanClientFactory, lists::is_valid_list_color},
    command_result::{CommandSuccess, ListUpdateSuccess, ListUpdatedField},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::{map_mutation_client_error_with_not_found, protocol_error_details},
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    redaction::Redactor,
};

use super::{non_empty, trimmed_non_empty};

#[derive(Debug, Args)]
#[command(
    after_help = "Example:\n  wekan list update list-id --board board-id --title \"Plan next release\""
)]
#[command(next_help_heading = "Fields")]
#[command(group(
    ArgGroup::new("updates")
        .required(true)
        .multiple(true)
        .args(["title", "color", "starred", "wip_limit"])
))]
pub struct UpdateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// Wekan list ID.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    pub list_id: String,

    /// New title. Wekan v11.06 truncates values longer than 1000 UTF-16 code units.
    #[arg(long, value_parser = trimmed_non_empty)]
    pub title: Option<String>,

    /// Named Wekan list color or a custom #rrggbb color.
    #[arg(long, value_parser = list_color)]
    pub color: Option<String>,

    /// Whether the list is starred.
    #[arg(long)]
    pub starred: Option<bool>,

    /// Numeric WIP limit value.
    #[arg(long, value_parser = finite_number, requires_all = ["wip_enabled", "wip_soft"])]
    pub wip_limit: Option<f64>,

    /// Whether WIP limiting is enabled.
    #[arg(long, requires_all = ["wip_limit", "wip_soft"])]
    pub wip_enabled: Option<bool>,

    /// Whether exceeding the WIP limit is allowed with a warning.
    #[arg(long, requires_all = ["wip_limit", "wip_enabled"])]
    pub wip_soft: Option<bool>,
}

pub(super) async fn execute(
    args: UpdateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let updated_fields = updated_fields(&args);
    let request = UpdateListRequest {
        title: args.title,
        color: args.color,
        starred: args.starred,
        wip_limit: args.wip_limit.map(|value| ListWipLimit {
            value: Number::from_f64(value).expect("the parser rejects non-finite numbers"),
            enabled: args
                .wip_enabled
                .expect("Clap requires every WIP option together"),
            soft: args
                .wip_soft
                .expect("Clap requires every WIP option together"),
        }),
    };
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let updated = context
        .client()
        .update_list(
            &args.board_id,
            &args.list_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_mutation_client_error_with_not_found(
                error,
                &redactor,
                "list update",
                "the requested Wekan list was not found on that board",
            )
        })?;
    if updated.list_id != args.list_id {
        let mut details = protocol_error_details(true);
        details.outcome_unknown = Some(true);
        return Err(AppError::new(
            ErrorCode::ProtocolError,
            "Wekan reported a different updated list ID than requested",
            StableExitCode::Transport,
        )
        .with_details(details));
    }
    Ok(CommandSuccess::ListUpdated(ListUpdateSuccess {
        board_id: args.board_id,
        list_id: updated.list_id,
        updated_fields,
    }))
}

fn updated_fields(args: &UpdateArgs) -> Vec<ListUpdatedField> {
    let mut fields = Vec::new();
    if args.title.is_some() {
        fields.push(ListUpdatedField::Title);
    }
    if args.color.is_some() {
        fields.push(ListUpdatedField::Color);
    }
    if args.starred.is_some() {
        fields.push(ListUpdatedField::Starred);
    }
    if args.wip_limit.is_some() {
        fields.push(ListUpdatedField::WipLimit);
    }
    fields
}

fn list_color(value: &str) -> Result<String, String> {
    if is_valid_list_color(value) {
        Ok(value.to_owned())
    } else {
        Err("use a Wekan list color name or a custom #rrggbb value".to_owned())
    }
}

fn finite_number(value: &str) -> Result<f64, String> {
    let value = value
        .parse::<f64>()
        .map_err(|_| "value must be a number".to_owned())?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err("value must be finite".to_owned())
    }
}
