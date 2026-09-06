use clap::{ArgGroup, Args};
use serde_json::Number;

use crate::{
    client::{UpdateCardRequest, WekanClientFactory, cards::is_valid_card_color},
    command_result::{CardSubmittedField, CardUpdateSuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::{map_partial_mutation_client_error_with_not_found, protocol_error_details},
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    redaction::Redactor,
};

use super::{non_empty, rfc3339, trimmed_non_empty};

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("updates")
        .required(true)
        .multiple(true)
        .args([
            "title", "sort", "parent_id", "description", "color", "label_ids", "clear_labels",
            "requested_by", "assigned_by", "received_at", "clear_received_at", "start_at",
            "clear_start_at", "due_at", "clear_due_at", "end_at", "clear_end_at",
            "spent_time", "is_over_time", "members", "clear_members", "assignees",
            "clear_assignees", "due_complete"
        ])
))]
pub struct UpdateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    pub board_id: String,

    /// Wekan list ID.
    #[arg(long = "list", value_parser = non_empty)]
    pub list_id: String,

    /// Wekan card ID.
    #[arg(value_parser = non_empty)]
    pub card_id: String,

    /// New title. Wekan v11.06 truncates values longer than 1000 UTF-16 code units.
    #[arg(long, value_parser = trimmed_non_empty)]
    pub title: Option<String>,

    /// Numeric card order. Wekan v11.06 silently ignores zero.
    #[arg(long, value_parser = finite_number)]
    pub sort: Option<f64>,

    /// Parent card ID.
    #[arg(long, value_parser = non_empty)]
    pub parent_id: Option<String>,

    /// New card description.
    #[arg(long, value_parser = trimmed_non_empty)]
    pub description: Option<String>,

    /// Named Wekan card color or a custom #rrggbb color.
    #[arg(long, value_parser = card_color)]
    pub color: Option<String>,

    /// Label ID; repeat to replace the complete label array.
    #[arg(long = "label", value_parser = non_empty, conflicts_with = "clear_labels")]
    pub label_ids: Vec<String>,

    /// Replace labels with an empty array.
    #[arg(long, conflicts_with = "label_ids")]
    pub clear_labels: bool,

    /// Requested-by display name.
    #[arg(long, value_parser = trimmed_non_empty)]
    pub requested_by: Option<String>,

    /// Assigned-by display name.
    #[arg(long, value_parser = trimmed_non_empty)]
    pub assigned_by: Option<String>,

    /// Received date-time in RFC3339 format.
    #[arg(long, value_parser = rfc3339, conflicts_with = "clear_received_at")]
    pub received_at: Option<String>,

    /// Clear the received date.
    #[arg(long, conflicts_with = "received_at")]
    pub clear_received_at: bool,

    /// Start date-time in RFC3339 format.
    #[arg(long, value_parser = rfc3339, conflicts_with = "clear_start_at")]
    pub start_at: Option<String>,

    /// Clear the start date.
    #[arg(long, conflicts_with = "start_at")]
    pub clear_start_at: bool,

    /// Due date-time in RFC3339 format.
    #[arg(long, value_parser = rfc3339, conflicts_with = "clear_due_at")]
    pub due_at: Option<String>,

    /// Clear the due date.
    #[arg(long, conflicts_with = "due_at")]
    pub clear_due_at: bool,

    /// End date-time in RFC3339 format.
    #[arg(long, value_parser = rfc3339, conflicts_with = "clear_end_at")]
    pub end_at: Option<String>,

    /// Clear the end date.
    #[arg(long, conflicts_with = "end_at")]
    pub clear_end_at: bool,

    /// Finite spent-time value. Wekan v11.06 silently ignores zero.
    #[arg(long, value_parser = finite_number)]
    pub spent_time: Option<f64>,

    /// Overtime state. Wekan v11.06 ignores false and mishandles true.
    #[arg(long)]
    pub is_over_time: Option<bool>,

    /// Member ID; repeat to replace the complete member array.
    #[arg(long = "member", value_parser = non_empty, conflicts_with = "clear_members")]
    pub members: Vec<String>,

    /// Replace members with an empty array.
    #[arg(long, conflicts_with = "members")]
    pub clear_members: bool,

    /// Assignee ID; repeat to replace the complete assignee array.
    #[arg(long = "assignee", value_parser = non_empty, conflicts_with = "clear_assignees")]
    pub assignees: Vec<String>,

    /// Replace assignees with an empty array.
    #[arg(long, conflicts_with = "assignees")]
    pub clear_assignees: bool,

    /// Whether the due date is complete.
    #[arg(long)]
    pub due_complete: Option<bool>,
}

pub(super) async fn execute(
    args: UpdateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let submitted_fields = submitted_fields(&args);
    let request = UpdateCardRequest {
        title: args.title,
        sort: args
            .sort
            .map(|value| Number::from_f64(value).expect("the parser rejects non-finite numbers")),
        parent_id: args.parent_id,
        description: args.description,
        color: args.color,
        label_ids: replacement(args.label_ids, args.clear_labels),
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
            map_partial_mutation_client_error_with_not_found(
                error,
                &redactor,
                "card update",
                "the requested Wekan card was not found in that board list",
            )
        })?;
    if updated.card_id != args.card_id {
        let mut details = protocol_error_details(true);
        details.outcome_unknown = Some(true);
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

fn date_value(value: Option<String>, clear: bool) -> Option<String> {
    if clear { Some(String::new()) } else { value }
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
            !args.label_ids.is_empty() || args.clear_labels,
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
    ] {
        if present {
            fields.push(field);
        }
    }
    fields
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
