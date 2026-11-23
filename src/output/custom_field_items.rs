use super::formatting::escape_terminal_controls;
use crate::command_result::{
    CustomFieldItemCreateSuccess, CustomFieldItemDeleteSuccess, CustomFieldItemUpdateSuccess,
};

pub(super) fn created(data: &CustomFieldItemCreateSuccess) -> String {
    format!(
        "Dropdown item creation request accepted for custom field {} on board {}. Use custom-field get to read item IDs.",
        escape_terminal_controls(&data.custom_field_id),
        escape_terminal_controls(&data.board_id)
    )
}
pub(super) fn updated(data: &CustomFieldItemUpdateSuccess) -> String {
    let outcome = if data.submitted_fields.is_empty() {
        "No changes requested for"
    } else {
        "Updated"
    };
    format!(
        "{outcome} dropdown item {} in custom field {} on board {}.",
        escape_terminal_controls(&data.item_id),
        escape_terminal_controls(&data.custom_field_id),
        escape_terminal_controls(&data.board_id)
    )
}
pub(super) fn deleted(data: &CustomFieldItemDeleteSuccess) -> String {
    format!(
        "Deleted dropdown item {} in custom field {} on board {}.",
        escape_terminal_controls(&data.item_id),
        escape_terminal_controls(&data.custom_field_id),
        escape_terminal_controls(&data.board_id)
    )
}
