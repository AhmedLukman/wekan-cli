use super::formatting::{escape_terminal_controls, render_named_value};
use crate::command_result::{
    CustomFieldCollectionSuccess, CustomFieldCreateSuccess, CustomFieldDeleteSuccess,
    CustomFieldDetail, CustomFieldUpdateSuccess,
};
fn fields<T: serde::Serialize>(data: &T) -> String {
    let serde_json::Value::Object(fields) =
        serde_json::to_value(data).expect("custom-field results serialize as objects")
    else {
        unreachable!()
    };
    let mut lines = Vec::new();
    for (name, value) in fields {
        if !value.is_null() {
            render_named_value(&mut lines, 0, &name, &value);
        }
    }
    lines.join("\n")
}
pub(super) fn collection(data: &CustomFieldCollectionSuccess) -> String {
    if data.custom_fields.is_empty() {
        return format!(
            "No custom fields found on board {}.",
            escape_terminal_controls(&data.board_id)
        );
    }
    let mut lines = vec!["ID  NAME  TYPE".to_owned()];
    lines.extend(data.custom_fields.iter().map(|field| {
        format!(
            "{}  {}  {}",
            escape_terminal_controls(&field.custom_field_id),
            escape_terminal_controls(&field.name),
            escape_terminal_controls(&field.field_type)
        )
    }));
    lines.join("\n")
}
pub(super) fn detail(data: &CustomFieldDetail) -> String {
    fields(data)
}
pub(super) fn created(data: &CustomFieldCreateSuccess) -> String {
    format!(
        "Created custom field {} on board {}.",
        escape_terminal_controls(&data.custom_field_id),
        escape_terminal_controls(&data.board_id)
    )
}
pub(super) fn updated(data: &CustomFieldUpdateSuccess) -> String {
    format!(
        "Wekan acknowledged update of custom field {} on board {}.",
        escape_terminal_controls(&data.custom_field_id),
        escape_terminal_controls(&data.board_id)
    )
}
pub(super) fn deleted(data: &CustomFieldDeleteSuccess) -> String {
    format!(
        "Wekan acknowledged hard deletion of custom field {} through board {}.",
        escape_terminal_controls(&data.custom_field_id),
        escape_terminal_controls(&data.board_id)
    )
}
