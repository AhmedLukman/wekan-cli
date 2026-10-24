use clap::{ArgGroup, Args};
use serde_json::Number;

use crate::{
    client::{
        UpdateListRequest, WekanClientFactory, lists::ListWipLimitInput, lists::is_valid_list_color,
    },
    command_result::{CommandSuccess, ListUpdateSuccess, ListUpdatedField},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::{map_client_error_with_not_found, protocol_error_details},
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
        .args(["title", "color", "starred", "clear_starred", "wip_limit", "wip_enabled", "wip_soft"])
))]
pub struct UpdateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// Wekan list ID.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    pub list_id: String,

    /// New title. Wekan v12.57 truncates values longer than 1000 UTF-16 code units.
    #[arg(long, value_parser = trimmed_non_empty)]
    pub title: Option<String>,

    /// Named Wekan list color or a custom #rrggbb color.
    #[arg(long, value_parser = list_color)]
    pub color: Option<String>,

    /// Whether the list is starred.
    #[arg(long, conflicts_with = "clear_starred")]
    pub starred: Option<bool>,

    /// Set starred to null.
    #[arg(long, conflicts_with = "starred")]
    pub clear_starred: bool,

    /// Numeric WIP limit value.
    #[arg(long, value_parser = finite_number)]
    pub wip_limit: Option<f64>,

    /// Whether WIP limiting is enabled.
    #[arg(long)]
    pub wip_enabled: Option<bool>,

    /// Whether exceeding the WIP limit is allowed with a warning.
    #[arg(long)]
    pub wip_soft: Option<bool>,
}

pub(super) async fn execute(
    args: UpdateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let updated_fields = updated_fields(&args);
    let wip_limit = has_wip(&args).then(|| ListWipLimitInput {
        value: args
            .wip_limit
            .map(|value| Number::from_f64(value).expect("finite number")),
        enabled: args.wip_enabled,
        soft: args.wip_soft,
    });
    let request = UpdateListRequest {
        title: args.title,
        color: args.color,
        starred: if args.clear_starred {
            Some(None)
        } else {
            args.starred.map(Some)
        },
        wip_limit,
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
            map_client_error_with_not_found(
                error,
                &redactor,
                "list update",
                "the requested Wekan list was not found on that board",
            )
        })?;
    if updated.list_id != args.list_id {
        let details = protocol_error_details(true);

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
    if args.starred.is_some() || args.clear_starred {
        fields.push(ListUpdatedField::Starred);
    }
    if has_wip(args) {
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

fn has_wip(args: &UpdateArgs) -> bool {
    args.wip_limit.is_some() || args.wip_enabled.is_some() || args.wip_soft.is_some()
}
