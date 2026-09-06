use super::formatting::{escape_terminal_controls, render_named_value, two_column_table};
use crate::command_result::{
    BoardCountSuccess, BoardCreateSuccess, BoardDeleteSuccess, BoardDetail, BoardListScope,
    BoardListSuccess, BoardRenameSuccess,
};

pub(super) fn render_board_list(data: &BoardListSuccess) -> String {
    if data.boards.is_empty() {
        return match data.scope {
            BoardListScope::Active => "No active boards found.".to_owned(),
            BoardListScope::Public => "No public boards found.".to_owned(),
        };
    }
    let rows = data
        .boards
        .iter()
        .map(|board| {
            (
                escape_terminal_controls(&board.board_id),
                escape_terminal_controls(&board.title),
            )
        })
        .collect::<Vec<_>>();
    two_column_table("ID", "TITLE", &rows)
}

pub(super) fn render_board_count(data: &BoardCountSuccess) -> String {
    format!(
        "Private boards: {}\nPublic boards: {}",
        data.private, data.public
    )
}

pub(super) fn render_board_created(data: &BoardCreateSuccess) -> String {
    format!(
        "Created board {}.\nDefault swimlane ID: {}",
        escape_terminal_controls(&data.board_id),
        escape_terminal_controls(&data.default_swimlane_id)
    )
}

pub(super) fn render_board_renamed(data: &BoardRenameSuccess) -> String {
    format!(
        "Renamed board {} to {}.",
        escape_terminal_controls(&data.board_id),
        escape_terminal_controls(&data.title)
    )
}

pub(super) fn render_board_deleted(data: &BoardDeleteSuccess) -> String {
    format!(
        "Deleted board {}.",
        escape_terminal_controls(&data.board_id)
    )
}

pub(super) fn render_board_detail(data: &BoardDetail) -> String {
    let value = serde_json::to_value(data).expect("board documents are always serializable");
    let serde_json::Value::Object(mut fields) = value else {
        unreachable!("board documents serialize as objects")
    };
    fields.remove("board_id");
    fields.remove("title");

    let mut groups = [
        ("Core", Vec::new()),
        ("Sharing", Vec::new()),
        ("Appearance", Vec::new()),
        ("Dates and defaults", Vec::new()),
        ("Settings", Vec::new()),
    ];
    for (name, value) in fields {
        if value.is_null() {
            continue;
        }
        let group = board_field_group(&name);
        groups[group].1.push((name, value));
    }

    let mut lines = vec![
        format!("Board ID: {}", escape_terminal_controls(&data.board_id)),
        format!("Title: {}", escape_terminal_controls(&data.title)),
    ];
    for (heading, values) in groups {
        if values.is_empty() {
            continue;
        }
        lines.push(format!("{heading}:"));
        for (name, value) in values {
            render_named_value(&mut lines, 2, &name, &value);
        }
    }
    lines.join("\n")
}

pub(super) fn board_field_group(name: &str) -> usize {
    if matches!(
        name,
        "members" | "watchers" | "labels" | "orgs" | "teams" | "domains" | "import_usernames"
    ) {
        1
    } else if matches!(
        name,
        "color" | "custom_theme_colors" | "background_image_url" | "background_image_id"
    ) {
        2
    } else if name.contains("_at")
        || name.ends_with("_board_id")
        || name.ends_with("_list_id")
        || matches!(name, "spent_time" | "is_overtime")
    {
        3
    } else if name.starts_with("allows_")
        || name.starts_with("card_aging")
        || matches!(
            name,
            "show_dependencies" | "restrict_comment_editing" | "auto_width" | "present_parent_task"
        )
    {
        4
    } else {
        0
    }
}
