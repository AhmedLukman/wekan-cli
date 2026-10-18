use super::formatting::{escape_terminal_controls, render_object};
use crate::command_result::{
    RuleCollectionSuccess, RuleCreateSuccess, RuleDeleteSuccess, RuleDetail, RuleUpdateSuccess,
};
pub(super) fn collection(data: &RuleCollectionSuccess) -> String {
    if data.rules.is_empty() {
        format!(
            "No rules found on board {}.",
            escape_terminal_controls(&data.board_id)
        )
    } else {
        render_object(data)
    }
}
pub(super) fn detail(data: &RuleDetail) -> String {
    render_object(data)
}
pub(super) fn created(data: &RuleCreateSuccess) -> String {
    format!("Rule created:\n{}", render_object(data))
}
pub(super) fn updated(data: &RuleUpdateSuccess) -> String {
    format!("Rule updated:\n{}", render_object(data))
}
pub(super) fn deleted(data: &RuleDeleteSuccess) -> String {
    format!(
        "Rule and its referenced trigger/action deleted:\n{}",
        render_object(data)
    )
}
