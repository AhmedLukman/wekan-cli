use super::formatting::{escape_terminal_controls, render_named_value};
use crate::command_result::{
    ChecklistItemCreateSuccess, ChecklistItemDeleteSuccess, ChecklistItemDetail,
    ChecklistItemUpdateSuccess,
};
pub(super) fn render_checklist_item_detail(data: &ChecklistItemDetail) -> String {
    let serde_json::Value::Object(mut fields) =
        serde_json::to_value(data).expect("checklist item details are serializable")
    else {
        unreachable!("item details are objects")
    };
    fields.remove("item_id");
    fields.remove("title");
    let mut lines = vec![
        format!("Item ID: {}", escape_terminal_controls(&data.item_id)),
        format!("Title: {}", escape_terminal_controls(&data.title)),
    ];
    for (name, value) in fields {
        if !value.is_null() {
            render_named_value(&mut lines, 0, &name, &value);
        }
    }
    lines.join("\n")
}
pub(super) fn render_checklist_item_created(data: &ChecklistItemCreateSuccess) -> String {
    format!(
        "Created item {} in checklist {} on card {} on board {}.",
        escape_terminal_controls(&data.item_id),
        escape_terminal_controls(&data.checklist_id),
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.board_id)
    )
}
pub(super) fn render_checklist_item_updated(data: &ChecklistItemUpdateSuccess) -> String {
    format!(
        "Wekan acknowledged update of item {} in checklist {} on card {} on board {}.",
        escape_terminal_controls(&data.item_id),
        escape_terminal_controls(&data.checklist_id),
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.board_id)
    )
}
pub(super) fn render_checklist_item_deleted(data: &ChecklistItemDeleteSuccess) -> String {
    format!(
        "Permanently deleted item {} from checklist {} on card {} on board {}.",
        escape_terminal_controls(&data.item_id),
        escape_terminal_controls(&data.checklist_id),
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.board_id)
    )
}
