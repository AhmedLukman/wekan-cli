use super::formatting::escape_terminal_controls;
use crate::command_result::{
    CustomFieldItemCreateSuccess, CustomFieldItemDeleteSuccess, CustomFieldItemUpdateSuccess,
};

pub(super) fn created(data: &CustomFieldItemCreateSuccess) -> String {
    format!(
        "Wekan acknowledged dropdown item creation for custom field {} on board {}. Use custom-field get to read item IDs.",
        escape_terminal_controls(&data.custom_field_id),
        escape_terminal_controls(&data.board_id)
    )
}
pub(super) fn updated(data: &CustomFieldItemUpdateSuccess) -> String {
    format!(
        "Wekan acknowledged update of dropdown item {} in custom field {} on board {}.",
        escape_terminal_controls(&data.item_id),
        escape_terminal_controls(&data.custom_field_id),
        escape_terminal_controls(&data.board_id)
    )
}
pub(super) fn deleted(data: &CustomFieldItemDeleteSuccess) -> String {
    format!(
        "Wekan acknowledged hard deletion of dropdown item {} in custom field {} on board {}.",
        escape_terminal_controls(&data.item_id),
        escape_terminal_controls(&data.custom_field_id),
        escape_terminal_controls(&data.board_id)
    )
}
