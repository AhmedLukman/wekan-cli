use super::formatting::{escape_terminal_controls, render_named_value_complete};
use crate::command_result::{
    BoardDependencyCollectionSuccess, CardDependencyCollectionSuccess, DependencyCreateSuccess,
    DependencyDeleteSuccess, DependencyUpdateSuccess,
};

pub(super) fn card_collection(data: &CardDependencyCollectionSuccess) -> String {
    if data.dependencies.is_empty() {
        return format!(
            "No outgoing dependencies found on card {} on board {}.",
            escape_terminal_controls(&data.card_id),
            escape_terminal_controls(&data.board_id)
        );
    }
    let mut lines = vec!["TARGET  TYPE  COLOR  ICON".to_owned()];
    lines.extend(data.dependencies.iter().map(|d| {
        format!(
            "{}  {}  {}  {}",
            escape_terminal_controls(&d.target_card_id),
            escape_terminal_controls(&d.dependency_type),
            escape_terminal_controls(&d.color),
            escape_terminal_controls(&d.icon)
        )
    }));
    lines.join("\n")
}

pub(super) fn board_collection(data: &BoardDependencyCollectionSuccess) -> String {
    if data.dependencies.is_empty() {
        return format!(
            "No dependency lines found on board {}.",
            escape_terminal_controls(&data.board_id)
        );
    }
    // Render every mapped field, including optional titles and exact card numbers.
    let mut lines = Vec::new();
    for dependency in &data.dependencies {
        let value = serde_json::to_value(dependency).expect("dependency lines are serializable");
        render_named_value_complete(&mut lines, 0, "dependency", &value);
    }
    lines.join("\n")
}

pub(super) fn created(data: &DependencyCreateSuccess) -> String {
    format!(
        "Created or updated dependency from card {} to card {} on board {}.\nType: {}\nColor: {}\nIcon: {}",
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.target_card_id),
        escape_terminal_controls(&data.board_id),
        escape_terminal_controls(&data.dependency_type),
        escape_terminal_controls(&data.color),
        escape_terminal_controls(&data.icon)
    )
}

pub(super) fn updated(data: &DependencyUpdateSuccess) -> String {
    format!(
        "Dependency update request accepted from card {} to card {} on board {}.",
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.target_card_id),
        escape_terminal_controls(&data.board_id)
    )
}

pub(super) fn deleted(data: &DependencyDeleteSuccess) -> String {
    format!(
        "Dependency deletion request accepted from card {} to card {} on board {}.",
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.target_card_id),
        escape_terminal_controls(&data.board_id)
    )
}
