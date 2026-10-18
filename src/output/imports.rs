use super::formatting::escape_terminal_controls as escape;
use crate::command_result::{BoardImportSuccess, IcsImportSuccess, ZipImportSuccess};

pub(super) fn trello(data: &crate::command_result::TrelloImportResult) -> String {
    let mut output = format!(
        "Imported Trello boards.\nBoard IDs: {}",
        escape(&data.board_ids.join(", "))
    );
    if let Some(errors) = &data.errors {
        output.push_str(&format!("\nFailed boards: {}", errors.len()));
        for error in errors {
            let name = match &error.board {
                crate::client::boards::trello_import::TrelloImportBoardName::String(name) => {
                    name.clone()
                }
                other => serde_json::to_string(other).expect("scalar names are serializable"),
            };
            output.push_str(&format!("\n{}: {}", escape(&name), escape(&error.error)));
        }
    }
    output
}

pub(super) fn board(data: &BoardImportSuccess) -> String {
    format!(
        "Imported board: {} (source: {}).",
        escape(&data.board_id),
        escape(&data.source)
    )
}

pub(super) fn zip(data: &ZipImportSuccess) -> String {
    let c = &data.counts;
    let mut output = format!(
        "Imported ZIP into board {}.\nCounts: {} swimlanes, {} lists, {} cards, {} checklists, {} comments, {} attachments.",
        escape(&data.board_id),
        c.swimlanes,
        c.lists,
        c.cards,
        c.checklists,
        c.comments,
        c.attachments
    );
    for (label, value) in [
        ("Requested swimlane", &data.swimlane_id),
        ("Requested list", &data.list_id),
        ("Requested card", &data.card_id),
        (
            "Requested checklist (ignored by Wekan v11.06)",
            &data.checklist_id,
        ),
    ] {
        if let Some(value) = value {
            output.push_str(&format!("\n{label}: {}", escape(value)));
        }
    }
    output
}

pub(super) fn ics(data: &IcsImportSuccess) -> String {
    format!(
        "Imported ICS into board {}, list {}, swimlane {}.\nCreated: {}\nCard IDs: {}",
        escape(&data.board_id),
        escape(&data.list_id),
        escape(&data.swimlane_id),
        data.created,
        escape(&data.card_ids.join(", "))
    )
}
