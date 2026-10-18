use super::formatting::{escape_terminal_controls, render_named_value_complete, render_object};
use crate::command_result::{
    CardArchiveSuccess, CardCollectionSuccess, CardCreateSuccess, CardCustomFieldQuerySuccess,
    CardCustomFieldSetSuccess, CardDeleteSuccess, CardDetail, CardRestoreSuccess,
    CardUpdateSuccess,
};

pub(super) fn render_batch(batch: &crate::command_result::CardBatchResult) -> String {
    use crate::command_result::CardBatchResult;
    match batch {
        CardBatchResult::CardBulkCreate(data) => format!(
            "Bulk card creation: {} created, {} failed.\n{}",
            data.results
                .iter()
                .filter(|item| item.card_id.is_some())
                .count(),
            data.results
                .iter()
                .filter(|item| item.error.is_some())
                .count(),
            render_object(data)
        ),
        CardBatchResult::CardBulkDelete(data) => format!(
            "Bulk card deletion: {} deleted, {} not found.\n{}",
            data.deleted.len(),
            data.not_found.len(),
            render_object(data)
        ),
        CardBatchResult::CardBulkLabels(data) => format!(
            "Bulk card labels: {} updated, {} not found.\n{}",
            data.updated.len(),
            data.not_found.len(),
            render_object(data)
        ),
    }
}

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

pub(super) fn render_card_count(data: &crate::command_result::CardCountSuccess) -> String {
    match &data.list_id {
        Some(list_id) => format!(
            "{} active cards in list {} on board {}.",
            data.count,
            escape_terminal_controls(list_id),
            escape_terminal_controls(&data.board_id)
        ),
        None => format!(
            "{} active cards on board {}.",
            data.count,
            escape_terminal_controls(&data.board_id)
        ),
    }
}

pub(super) fn render_swimlane_cards(
    data: &crate::command_result::SwimlaneCardCollectionSuccess,
) -> String {
    if data.cards.is_empty() {
        return format!(
            "No cards found in swimlane {} on board {}.",
            escape_terminal_controls(&data.swimlane_id),
            escape_terminal_controls(&data.board_id)
        );
    }
    let mut lines =
        vec!["ID  TITLE  DESCRIPTION  LIST  RECEIVED  START  DUE  END  ASSIGNEES  SORT".to_owned()];
    for card in &data.cards {
        let assignees = card
            .assignees
            .as_ref()
            .map(|values| {
                values
                    .iter()
                    .map(|v| v.as_deref().unwrap_or("null"))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();
        lines.push(
            [
                card.card_id.clone(),
                card.title.clone().unwrap_or_default(),
                card.description.clone().unwrap_or_default(),
                card.list_id.clone().unwrap_or_default(),
                card.received_at.clone().unwrap_or_default(),
                card.start_at.clone().unwrap_or_default(),
                card.due_at.clone().unwrap_or_default(),
                card.end_at.clone().unwrap_or_default(),
                assignees,
                card.sort
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            ]
            .iter()
            .map(|v| escape_terminal_controls(v))
            .collect::<Vec<_>>()
            .join("  "),
        );
    }
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
    let mut rendered = format!(
        "Card update request accepted for {} using source list {} on board {}; submitted fields: {}.",
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id),
        escape_terminal_controls(&fields)
    );
    if data
        .submitted_fields
        .contains(&crate::command_result::CardSubmittedField::IsOverTime)
    {
        rendered.push_str("\nWekan v11.06 ignores is_over_time=false and writes the wrong field for true; the card's actual overtime state is unchanged.");
    }
    rendered
}

pub(super) fn render_card_deleted(data: &CardDeleteSuccess) -> String {
    format!(
        "Card deletion request accepted for {} from list {} on board {}.",
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id)
    )
}

pub(super) fn render_card_archived(data: &CardArchiveSuccess) -> String {
    format!(
        "Archived card {} in list {} on board {} at {}.",
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id),
        escape_terminal_controls(&data.archived_at)
    )
}

pub(super) fn render_card_restored(data: &CardRestoreSuccess) -> String {
    format!(
        "Restored card {} in list {} on board {}.",
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id)
    )
}

pub(super) fn render_card_custom_field_set(data: &CardCustomFieldSetSuccess) -> String {
    let serde_json::Value::Object(fields) =
        serde_json::to_value(data).expect("card custom-field results serialize as objects")
    else {
        unreachable!()
    };
    let mut lines = Vec::new();
    for (name, value) in fields {
        render_named_value_complete(&mut lines, 0, &name, &value);
    }
    lines.join("\n")
}
pub(super) fn render_card_custom_field_query(data: &CardCustomFieldQuerySuccess) -> String {
    if data.cards.is_empty() {
        return format!(
            "No matching cards found on board {}.",
            escape_terminal_controls(&data.board_id)
        );
    }
    data.cards
        .iter()
        .map(render_card_detail)
        .collect::<Vec<_>>()
        .join("\n\n")
}
