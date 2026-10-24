use clap::{ArgGroup, Args};
use serde_json::Number;

use crate::{
    client::{UpdateCardRequest, WekanClientFactory, cards::is_valid_card_color},
    command_result::{CardSubmittedField, CardUpdateSuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::{map_client_error_with_not_found, protocol_error_details},
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    redaction::Redactor,
};

use super::{date_input, non_empty};
use crate::client::cards::{
    CardCustomFieldInput, CardLocationInput, CardPokerInput, CardStickerInput, CardVoteInput,
};

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(transparent)]
pub struct JsonArray<T>(pub Vec<T>);

#[derive(Debug, Args)]
#[command(
    after_help = "Set at least one field. Set and clear forms of the same field conflict.\n\nExample:\n  wekan card update card-id --board board-id --list list-id --title \"Plan next release\""
)]
#[command(
    next_help_heading = "Fields",
    override_usage = "wekan card update <CARD_ID> --board <BOARD_ID> --list <LIST_ID> <FIELDS> [OPTIONS]"
)]
#[command(group(
    ArgGroup::new("updates")
        .required(true)
        .multiple(true)
        .args([
            "labels", "title", "sort", "parent_id", "description", "color", "label_ids", "clear_labels",
            "requested_by", "assigned_by", "received_at", "clear_received_at", "start_at",
            "clear_start_at", "due_at", "clear_due_at", "end_at", "clear_end_at",
            "spent_time", "is_over_time", "members", "clear_members", "assignees",
            "clear_assignees", "due_complete", "swimlane_id", "destination_list_id",
            "new_board_id", "new_swimlane_id", "new_list_id", "archive", "vote", "poker",
            "custom_fields", "stickers", "locations"
        ])
))]
pub struct UpdateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// Wekan list ID.
    #[arg(long = "list", value_parser = non_empty, help_heading = "Target")]
    pub list_id: String,

    /// Wekan card ID.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    pub card_id: String,

    /// New title. Wekan v12.57 truncates values longer than 1000 UTF-16 code units.
    #[arg(long, help_heading = "Fields")]
    pub title: Option<String>,

    /// Numeric card order. Wekan v12.57 silently ignores zero.
    #[arg(long, value_parser = finite_number, help_heading = "Fields")]
    pub sort: Option<f64>,

    /// Parent card ID.
    #[arg(long, value_parser = non_empty, help_heading = "Fields")]
    pub parent_id: Option<String>,

    /// New card description.
    #[arg(long, help_heading = "Fields")]
    pub description: Option<String>,

    /// Named Wekan card color or a custom #rrggbb color.
    #[arg(long, value_parser = card_color, help_heading = "Fields")]
    pub color: Option<String>,

    /// Label ID; repeat to replace the complete label array.
    #[arg(long = "label", value_parser = non_empty, conflicts_with_all = ["clear_labels", "labels"], help_heading = "People and labels")]
    pub label_ids: Vec<String>,

    /// Replace labels with a JSON array of strings or null elements.
    #[arg(long, value_name = "JSON", value_parser = labels_json, conflicts_with_all = ["label_ids", "clear_labels"], help_heading = "People and labels")]
    pub labels: Option<JsonArray<Option<String>>>,

    /// Replace labels with an empty array.
    #[arg(long, conflicts_with_all = ["label_ids", "labels"], help_heading = "People and labels")]
    pub clear_labels: bool,

    /// Requested-by display name.
    #[arg(long, help_heading = "People and labels")]
    pub requested_by: Option<String>,

    /// Assigned-by display name.
    #[arg(long, help_heading = "People and labels")]
    pub assigned_by: Option<String>,

    /// Received date accepted by Wekan (timestamp, date-only, textual date or epoch milliseconds).
    #[arg(long, value_parser = date_input, allow_hyphen_values = true, conflicts_with = "clear_received_at", help_heading = "Dates")]
    pub received_at: Option<crate::client::cards::CardDateInput>,

    /// Clear the received date.
    #[arg(long, conflicts_with = "received_at", help_heading = "Dates")]
    pub clear_received_at: bool,

    /// Start date accepted by Wekan (timestamp, date-only, textual date or epoch milliseconds).
    #[arg(long, value_parser = date_input, allow_hyphen_values = true, conflicts_with = "clear_start_at", help_heading = "Dates")]
    pub start_at: Option<crate::client::cards::CardDateInput>,

    /// Clear the start date.
    #[arg(long, conflicts_with = "start_at", help_heading = "Dates")]
    pub clear_start_at: bool,

    /// Due date accepted by Wekan (timestamp, date-only, textual date or epoch milliseconds).
    #[arg(long, value_parser = date_input, allow_hyphen_values = true, conflicts_with = "clear_due_at", help_heading = "Dates")]
    pub due_at: Option<crate::client::cards::CardDateInput>,

    /// Clear the due date.
    #[arg(long, conflicts_with = "due_at", help_heading = "Dates")]
    pub clear_due_at: bool,

    /// End date accepted by Wekan (timestamp, date-only, textual date or epoch milliseconds).
    #[arg(long, value_parser = date_input, allow_hyphen_values = true, conflicts_with = "clear_end_at", help_heading = "Dates")]
    pub end_at: Option<crate::client::cards::CardDateInput>,

    /// Clear the end date.
    #[arg(long, conflicts_with = "end_at", help_heading = "Dates")]
    pub clear_end_at: bool,

    /// Spent time in hours, including zero. Wekan rejects negative values.
    #[arg(long, value_parser = finite_number, help_heading = "Fields")]
    pub spent_time: Option<f64>,

    /// Overtime state. Wekan v12.57 ignores false and mishandles true.
    #[arg(long, help_heading = "Fields")]
    pub is_over_time: Option<bool>,

    /// Member ID; repeat to replace the complete member array.
    #[arg(long = "member", value_parser = non_empty, conflicts_with = "clear_members", help_heading = "People and labels")]
    pub members: Vec<String>,

    /// Replace members with an empty array.
    #[arg(long, conflicts_with = "members", help_heading = "People and labels")]
    pub clear_members: bool,

    /// Assignee ID; repeat to replace the complete assignee array.
    #[arg(long = "assignee", value_parser = non_empty, conflicts_with = "clear_assignees", help_heading = "People and labels")]
    pub assignees: Vec<String>,

    /// Replace assignees with an empty array.
    #[arg(long, conflicts_with = "assignees", help_heading = "People and labels")]
    pub clear_assignees: bool,

    /// Whether the due date is complete.
    #[arg(long, help_heading = "Dates")]
    pub due_complete: Option<bool>,

    /// Destination swimlane within the source board.
    #[arg(long, value_parser = non_empty, help_heading = "Movement")]
    pub swimlane_id: Option<String>,

    /// Destination list within the source board (distinct from --list).
    #[arg(long = "list-id", value_parser = non_empty, help_heading = "Movement")]
    pub destination_list_id: Option<String>,

    /// Destination board; send all three --new-*-id fields for a board move.
    #[arg(long, value_parser = non_empty, help_heading = "Movement")]
    pub new_board_id: Option<String>,

    /// Destination swimlane for a board move.
    #[arg(long, value_parser = non_empty, help_heading = "Movement")]
    pub new_swimlane_id: Option<String>,

    /// Destination list for a board move.
    #[arg(long, value_parser = non_empty, help_heading = "Movement")]
    pub new_list_id: Option<String>,

    /// Reason used by native list or board moves; Wekan trims and limits it to 1000 code units.
    #[arg(long, help_heading = "Movement")]
    pub move_reason: Option<String>,

    /// Archive (true) or restore (false) through the update endpoint.
    #[arg(long, help_heading = "Fields")]
    pub archive: Option<bool>,

    /// Replace vote settings with a typed JSON object; Wekan resets votes.
    #[arg(long, value_name = "JSON", value_parser = vote_json, help_heading = "Structured fields")]
    pub vote: Option<CardVoteInput>,

    /// Replace poker settings with a typed JSON object; Wekan resets votes.
    #[arg(long, value_name = "JSON", value_parser = poker_json, help_heading = "Structured fields")]
    pub poker: Option<CardPokerInput>,

    /// Replace custom-field values with a typed JSON array; [] clears them.
    #[arg(long, value_name = "JSON", value_parser = json_array::<CardCustomFieldInput>, help_heading = "Structured fields")]
    pub custom_fields: Option<JsonArray<CardCustomFieldInput>>,

    /// Replace stickers with a typed JSON array; [] clears them.
    #[arg(long, value_name = "JSON", value_parser = json_array::<CardStickerInput>, help_heading = "Structured fields")]
    pub stickers: Option<JsonArray<CardStickerInput>>,

    /// Replace locations with a typed JSON array; [] clears them.
    #[arg(long, value_name = "JSON", value_parser = json_array::<CardLocationInput>, help_heading = "Structured fields")]
    pub locations: Option<JsonArray<CardLocationInput>>,
}

pub(super) async fn execute(
    args: UpdateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let submitted_fields = submitted_fields(&args);
    let request = UpdateCardRequest {
        move_reason: args.move_reason,
        title: args.title,
        sort: args
            .sort
            .map(|value| Number::from_f64(value).expect("the parser rejects non-finite numbers")),
        parent_id: args.parent_id,
        description: args.description,
        color: args.color,
        label_ids: args.labels.map(|value| value.0).or_else(|| {
            replacement(args.label_ids, args.clear_labels)
                .map(|ids| ids.into_iter().map(Some).collect())
        }),
        requested_by: args.requested_by,
        assigned_by: args.assigned_by,
        received_at: date_value(args.received_at, args.clear_received_at),
        start_at: date_value(args.start_at, args.clear_start_at),
        due_at: date_value(args.due_at, args.clear_due_at),
        end_at: date_value(args.end_at, args.clear_end_at),
        spent_time: args
            .spent_time
            .map(|value| Number::from_f64(value).expect("the parser rejects non-finite numbers")),
        is_over_time: args.is_over_time,
        members: replacement(args.members, args.clear_members),
        assignees: replacement(args.assignees, args.clear_assignees),
        due_complete: args.due_complete,
        swimlane_id: args.swimlane_id,
        list_id: args.destination_list_id,
        new_board_id: args.new_board_id,
        new_swimlane_id: args.new_swimlane_id,
        new_list_id: args.new_list_id,
        archive: args.archive,
        vote: args.vote,
        poker: args.poker,
        custom_fields: args.custom_fields.map(|value| value.0),
        stickers: args.stickers.map(|value| value.0),
        locations: args.locations.map(|value| value.0),
    };
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let updated = context
        .client()
        .update_card(
            &args.board_id,
            &args.list_id,
            &args.card_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "card update",
                "the requested Wekan card was not found in that board list",
            )
        })?;
    if updated.card_id != args.card_id {
        let details = protocol_error_details(true);

        return Err(AppError::new(
            ErrorCode::ProtocolError,
            "Wekan reported a different updated card ID than requested",
            StableExitCode::Transport,
        )
        .with_details(details));
    }
    Ok(CommandSuccess::CardUpdated(CardUpdateSuccess {
        board_id: args.board_id,
        list_id: args.list_id,
        card_id: updated.card_id,
        submitted_fields,
    }))
}

fn replacement(values: Vec<String>, clear: bool) -> Option<Vec<String>> {
    if clear || !values.is_empty() {
        Some(values)
    } else {
        None
    }
}

fn date_value(
    value: Option<crate::client::cards::CardDateInput>,
    clear: bool,
) -> Option<crate::client::cards::CardDateInput> {
    if clear {
        Some(String::new().into())
    } else {
        value
    }
}

fn submitted_fields(args: &UpdateArgs) -> Vec<CardSubmittedField> {
    let mut fields = Vec::new();
    for (present, field) in [
        (args.title.is_some(), CardSubmittedField::Title),
        (args.sort.is_some(), CardSubmittedField::Sort),
        (args.parent_id.is_some(), CardSubmittedField::ParentId),
        (args.description.is_some(), CardSubmittedField::Description),
        (args.color.is_some(), CardSubmittedField::Color),
        (
            !args.label_ids.is_empty() || args.clear_labels || args.labels.is_some(),
            CardSubmittedField::LabelIds,
        ),
        (args.requested_by.is_some(), CardSubmittedField::RequestedBy),
        (args.assigned_by.is_some(), CardSubmittedField::AssignedBy),
        (
            args.received_at.is_some() || args.clear_received_at,
            CardSubmittedField::ReceivedAt,
        ),
        (
            args.start_at.is_some() || args.clear_start_at,
            CardSubmittedField::StartAt,
        ),
        (
            args.due_at.is_some() || args.clear_due_at,
            CardSubmittedField::DueAt,
        ),
        (
            args.end_at.is_some() || args.clear_end_at,
            CardSubmittedField::EndAt,
        ),
        (args.spent_time.is_some(), CardSubmittedField::SpentTime),
        (args.is_over_time.is_some(), CardSubmittedField::IsOverTime),
        (
            !args.members.is_empty() || args.clear_members,
            CardSubmittedField::Members,
        ),
        (
            !args.assignees.is_empty() || args.clear_assignees,
            CardSubmittedField::Assignees,
        ),
        (args.due_complete.is_some(), CardSubmittedField::DueComplete),
        (args.swimlane_id.is_some(), CardSubmittedField::SwimlaneId),
        (
            args.destination_list_id.is_some(),
            CardSubmittedField::ListId,
        ),
        (args.new_board_id.is_some(), CardSubmittedField::NewBoardId),
        (
            args.new_swimlane_id.is_some(),
            CardSubmittedField::NewSwimlaneId,
        ),
        (args.new_list_id.is_some(), CardSubmittedField::NewListId),
        (args.move_reason.is_some(), CardSubmittedField::MoveReason),
        (args.archive.is_some(), CardSubmittedField::Archive),
        (args.vote.is_some(), CardSubmittedField::Vote),
        (args.poker.is_some(), CardSubmittedField::Poker),
        (
            args.custom_fields.is_some(),
            CardSubmittedField::CustomFields,
        ),
        (args.stickers.is_some(), CardSubmittedField::Stickers),
        (args.locations.is_some(), CardSubmittedField::Locations),
    ] {
        if present {
            fields.push(field);
        }
    }
    fields
}

fn json_input<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, String> {
    if !value.trim_start().starts_with('{') {
        return Err("expected a JSON object".to_owned());
    }
    serde_json::from_str(value)
        .map_err(|_| "expected JSON with the documented field names and types".to_owned())
}

fn json_array<T: serde::de::DeserializeOwned>(value: &str) -> Result<JsonArray<T>, String> {
    let items: Vec<serde_json::Value> =
        serde_json::from_str(value).map_err(|_| "expected a JSON array of objects".to_owned())?;
    items
        .into_iter()
        .map(|item| {
            if !item.is_object() {
                return Err("expected a JSON array of objects".to_owned());
            }
            serde_json::from_value(item)
                .map_err(|_| "expected JSON with the documented field names and types".to_owned())
        })
        .collect::<Result<Vec<T>, String>>()
        .map(JsonArray)
}

fn vote_json(value: &str) -> Result<CardVoteInput, String> {
    let vote: CardVoteInput = json_input(value)?;
    Ok(vote)
}

fn poker_json(value: &str) -> Result<CardPokerInput, String> {
    let poker: CardPokerInput = json_input(value)?;
    Ok(poker)
}

fn card_color(value: &str) -> Result<String, String> {
    if is_valid_card_color(value) {
        Ok(value.to_owned())
    } else {
        Err("use a Wekan card color name or a custom #rrggbb value".to_owned())
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

fn labels_json(value: &str) -> Result<JsonArray<Option<String>>, String> {
    serde_json::from_str(value)
        .map(JsonArray)
        .map_err(|_| "expected a JSON array of strings or null".to_owned())
}
