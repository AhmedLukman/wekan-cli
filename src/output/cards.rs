use super::formatting::{escape_terminal_controls, render_named_value_complete};
use crate::command_result::{
    CardCollectionSuccess, CardCreateSuccess, CardDeleteSuccess, CardDetail, CardUpdateSuccess,
};

pub(super) fn render_card_collection(data: &CardCollectionSuccess) -> String {
    if data.cards.is_empty() {
        return format!(
            "No cards found in list {} on board {}.",
            escape_terminal_controls(&data.list_id),
            escape_terminal_controls(&data.board_id)
        );
    }
    let mut lines = vec![
        "ID  TITLE  DESCRIPTION  SWIMLANE  RECEIVED  START  DUE  END  ASSIGNEES  SORT".to_owned(),
    ];
    lines.extend(data.cards.iter().map(|card| {
        format!(
            "{}  {}  {}  {}  {}  {}  {}  {}  {}  {}",
            escape_terminal_controls(&card.card_id),
            escape_terminal_controls(card.title.as_deref().unwrap_or("")),
            escape_terminal_controls(card.description.as_deref().unwrap_or("")),
            escape_terminal_controls(card.swimlane_id.as_deref().unwrap_or("")),
            escape_terminal_controls(card.received_at.as_deref().unwrap_or("")),
            escape_terminal_controls(card.start_at.as_deref().unwrap_or("")),
            escape_terminal_controls(card.due_at.as_deref().unwrap_or("")),
            escape_terminal_controls(card.end_at.as_deref().unwrap_or("")),
            escape_terminal_controls(&card.assignees.join(",")),
            card.sort
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )
    }));
    lines.join("\n")
}

pub(super) fn render_card_detail(data: &CardDetail) -> String {
    let value = serde_json::to_value(data).expect("card documents are always serializable");
    let serde_json::Value::Object(mut fields) = value else {
        unreachable!("card documents serialize as objects")
    };
    fields.remove("card_id");

    let mut lines = vec![format!(
        "Card ID: {}",
        escape_terminal_controls(&data.card_id)
    )];
    for (name, value) in fields {
        render_named_value_complete(&mut lines, 0, &name, &value);
    }
    lines.join("\n")
}

pub(super) fn render_card_created(data: &CardCreateSuccess) -> String {
    format!(
        "Created card {} in list {} on board {}.",
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id)
    )
}

pub(super) fn render_card_updated(data: &CardUpdateSuccess) -> String {
    let fields = data
        .submitted_fields
        .iter()
        .map(|field| {
            serde_json::to_value(field)
                .expect("submitted card fields are always serializable")
                .as_str()
                .expect("submitted card fields serialize as strings")
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Updated card {} in list {} on board {}; submitted fields: {}.",
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id),
        escape_terminal_controls(&fields)
    )
}

pub(super) fn render_card_deleted(data: &CardDeleteSuccess) -> String {
    format!(
        "Permanently deleted card {} from list {} on board {}.",
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id)
    )
}
