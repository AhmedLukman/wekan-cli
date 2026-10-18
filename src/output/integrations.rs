use super::formatting::{escape_terminal_controls, render_object};
use crate::command_result::{
    IntegrationActivityMutationSuccess, IntegrationCollectionSuccess, IntegrationCreateSuccess,
    IntegrationDeleteSuccess, IntegrationDetail, IntegrationUpdateSuccess,
};

pub(super) fn collection(data: &IntegrationCollectionSuccess) -> String {
    if data.integrations.is_empty() {
        format!(
            "No integrations found on board {}.",
            escape_terminal_controls(&data.board_id)
        )
    } else {
        render_object(data)
    }
}

pub(super) fn detail(data: &IntegrationDetail) -> String {
    render_object(data)
}
pub(super) fn created(data: &IntegrationCreateSuccess) -> String {
    format!("Integration created:\n{}", render_object(data))
}
pub(super) fn updated(data: &IntegrationUpdateSuccess) -> String {
    format!(
        "Integration update request accepted:\n{}",
        render_object(data)
    )
}
pub(super) fn deleted(data: &IntegrationDeleteSuccess) -> String {
    format!(
        "Integration deletion request accepted:\n{}",
        render_object(data)
    )
}
pub(super) fn activities(data: &IntegrationActivityMutationSuccess) -> String {
    format!(
        "Integration subscriptions returned by Wekan:\n{}",
        render_object(data)
    )
}
