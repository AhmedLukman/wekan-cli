use super::formatting::{escape_terminal_controls, render_named_value};
use crate::command_result::{
    SwimlaneCollectionSuccess, SwimlaneCreateSuccess, SwimlaneDeleteSuccess, SwimlaneDetail,
    SwimlaneUpdateSuccess,
};

pub(super) fn render_swimlane_collection(data: &SwimlaneCollectionSuccess) -> String {
    if data.swimlanes.is_empty() {
        return format!(
            "No swimlanes found on board {}.",
            escape_terminal_controls(&data.board_id)
        );
    }
    let mut lines = vec!["ID  TITLE".to_owned()];
    lines.extend(data.swimlanes.iter().map(|swimlane| {
        format!(
            "{}  {}",
            escape_terminal_controls(&swimlane.swimlane_id),
            escape_terminal_controls(&swimlane.title),
        )
    }));
    lines.join("\n")
}

pub(super) fn render_swimlane_detail(data: &SwimlaneDetail) -> String {
    let value = serde_json::to_value(data).expect("swimlane documents are always serializable");
    let serde_json::Value::Object(mut fields) = value else {
        unreachable!("swimlane documents serialize as objects")
    };
    fields.remove("swimlane_id");
    fields.remove("title");

    let mut lines = vec![
        format!(
            "Swimlane ID: {}",
            escape_terminal_controls(&data.swimlane_id)
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

pub(super) fn render_swimlane_created(data: &SwimlaneCreateSuccess) -> String {
    format!(
        "Created swimlane {} on board {}.",
        escape_terminal_controls(&data.swimlane_id),
        escape_terminal_controls(&data.board_id)
    )
}

pub(super) fn render_swimlane_updated(data: &SwimlaneUpdateSuccess) -> String {
    let fields = data
        .updated_fields
        .iter()
        .map(|field| {
            serde_json::to_value(field)
                .expect("updated swimlane fields are always serializable")
                .as_str()
                .expect("updated swimlane fields serialize as strings")
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Updated swimlane {} on board {}: {}.",
        escape_terminal_controls(&data.swimlane_id),
        escape_terminal_controls(&data.board_id),
        escape_terminal_controls(&fields)
    )
}

pub(super) fn render_swimlane_deleted(data: &SwimlaneDeleteSuccess) -> String {
    format!(
        "Permanently deleted swimlane {} on board {}.",
        escape_terminal_controls(&data.swimlane_id),
        escape_terminal_controls(&data.board_id)
    )
}
