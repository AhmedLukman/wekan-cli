use super::formatting::{escape_terminal_controls, render_named_value};
use crate::command_result::{
    ChecklistCollectionSuccess, ChecklistCreateSuccess, ChecklistDeleteSuccess, ChecklistDetail,
};

pub(super) fn render_checklist_collection(data: &ChecklistCollectionSuccess) -> String {
    if data.checklists.is_empty() {
        return format!(
            "No checklists found on card {} on board {}.",
            escape_terminal_controls(&data.card_id),
            escape_terminal_controls(&data.board_id)
        );
    }
    let mut lines = vec!["ID  TITLE".to_owned()];
    lines.extend(data.checklists.iter().map(|checklist| {
        format!(
            "{}  {}",
            escape_terminal_controls(&checklist.checklist_id),
            escape_terminal_controls(&checklist.title)
        )
    }));
    lines.join("\n")
}

pub(super) fn render_checklist_detail(data: &ChecklistDetail) -> String {
    let value = serde_json::to_value(data).expect("checklist documents are always serializable");
    let serde_json::Value::Object(mut fields) = value else {
        unreachable!("checklist documents serialize as objects")
    };
    fields.remove("checklist_id");
    fields.remove("title");

    let mut lines = vec![
        format!(
            "Checklist ID: {}",
            escape_terminal_controls(&data.checklist_id)
        ),
        format!("Title: {}", escape_terminal_controls(&data.title)),
    ];
    for (name, value) in fields {
        if !value.is_null() {
            render_named_value(&mut lines, 0, &name, &value);
        }
    }
    lines.join("\n")
}

pub(super) fn render_checklist_created(data: &ChecklistCreateSuccess) -> String {
    format!(
        "Created checklist {} on card {} on board {}.",
        escape_terminal_controls(&data.checklist_id),
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.board_id)
    )
}

pub(super) fn render_checklist_deleted(data: &ChecklistDeleteSuccess) -> String {
    format!(
        "Permanently deleted checklist {} from card {} on board {}.",
        escape_terminal_controls(&data.checklist_id),
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.board_id)
    )
}
