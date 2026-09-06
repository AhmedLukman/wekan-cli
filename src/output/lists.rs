use super::formatting::{escape_terminal_controls, render_named_value};
use crate::command_result::{
    ListCollectionSuccess, ListCreateSuccess, ListDeleteSuccess, ListDetail, ListUpdateSuccess,
};

pub(super) fn render_list_collection(data: &ListCollectionSuccess) -> String {
    if data.lists.is_empty() {
        return format!(
            "No lists found on board {}.",
            escape_terminal_controls(&data.board_id)
        );
    }
    let mut lines = vec!["ID  TITLE  MODIFIED  CARDS MODIFIED".to_owned()];
    lines.extend(data.lists.iter().map(|list| {
        format!(
            "{}  {}  {}  {}",
            escape_terminal_controls(&list.list_id),
            escape_terminal_controls(&list.title),
            escape_terminal_controls(list.modified_at.as_deref().unwrap_or("")),
            escape_terminal_controls(list.cards_modified_at.as_deref().unwrap_or("")),
        )
    }));
    lines.join("\n")
}

pub(super) fn render_list_detail(data: &ListDetail) -> String {
    let value = serde_json::to_value(data).expect("list documents are always serializable");
    let serde_json::Value::Object(mut fields) = value else {
        unreachable!("list documents serialize as objects")
    };
    fields.remove("list_id");
    fields.remove("title");

    let mut lines = vec![
        format!("List ID: {}", escape_terminal_controls(&data.list_id)),
        format!("Title: {}", escape_terminal_controls(&data.title)),
    ];
    for (name, value) in fields {
        if !value.is_null() {
            render_named_value(&mut lines, 0, &name, &value);
        }
    }
    lines.join("\n")
}

pub(super) fn render_list_created(data: &ListCreateSuccess) -> String {
    format!(
        "Created list {} on board {}.",
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id)
    )
}

pub(super) fn render_list_updated(data: &ListUpdateSuccess) -> String {
    let fields = data
        .updated_fields
        .iter()
        .map(|field| {
            serde_json::to_value(field)
                .expect("updated list fields are always serializable")
                .as_str()
                .expect("updated list fields serialize as strings")
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Updated list {} on board {}: {}.",
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id),
        escape_terminal_controls(&fields)
    )
}

pub(super) fn render_list_deleted(data: &ListDeleteSuccess) -> String {
    format!(
        "Soft-deleted list {} on board {} and its live cards.",
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id)
    )
}
