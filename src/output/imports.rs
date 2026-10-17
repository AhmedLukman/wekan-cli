use super::formatting::escape_terminal_controls as escape;
use crate::command_result::{BoardImportSuccess, IcsImportSuccess, ZipImportSuccess};

pub(super) fn board(data: &BoardImportSuccess) -> String {
    format!(
        "Board import acknowledged: {} (source: {}).\nMutation unconfirmed; no readback performed.",
        escape(&data.board_id),
        escape(&data.source)
    )
}

pub(super) fn zip(data: &ZipImportSuccess) -> String {
    let c = &data.counts;
    let mut output = format!(
        "ZIP import acknowledged into board {}.\nCounts: {} swimlanes, {} lists, {} cards, {} checklists, {} comments, {} attachments.\nMutation unconfirmed; no readback performed.",
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
        "ICS import acknowledged into board {}, list {}, swimlane {}.\nCreated: {}\nCard IDs: {}\nMutation unconfirmed; no readback performed.",
        escape(&data.board_id),
        escape(&data.list_id),
        escape(&data.swimlane_id),
        data.created,
        escape(&data.card_ids.join(", "))
    )
}
