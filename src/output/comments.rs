use super::formatting::{escape_terminal_controls, render_named_value};
use crate::command_result::{
    CommentCollectionSuccess, CommentCreateSuccess, CommentDeleteSuccess, CommentDetail,
};

pub(super) fn render_comment_collection(data: &CommentCollectionSuccess) -> String {
    if data.comments.is_empty() {
        return format!(
            "No comments found on card {} on board {}.",
            escape_terminal_controls(&data.card_id),
            escape_terminal_controls(&data.board_id)
        );
    }
    let mut lines = vec!["ID  TEXT  AUTHOR".to_owned()];
    lines.extend(data.comments.iter().map(|comment| {
        format!(
            "{}  {}  {}",
            escape_terminal_controls(&comment.comment_id),
            escape_terminal_controls(&comment.text),
            escape_terminal_controls(&comment.author_id)
        )
    }));
    lines.join("\n")
}

pub(super) fn render_comment_detail(data: &CommentDetail) -> String {
    let value = serde_json::to_value(data).expect("comment documents are always serializable");
    let serde_json::Value::Object(mut fields) = value else {
        unreachable!("comment documents serialize as objects")
    };
    fields.remove("comment_id");
    fields.remove("text");

    let mut lines = vec![
        format!("Comment ID: {}", escape_terminal_controls(&data.comment_id)),
        format!("Text: {}", escape_terminal_controls(&data.text)),
    ];
    for (name, value) in fields {
        if !value.is_null() {
            render_named_value(&mut lines, 0, &name, &value);
        }
    }
    lines.join("\n")
}

pub(super) fn render_comment_created(data: &CommentCreateSuccess) -> String {
    format!(
        "Created comment {} on card {} on board {}.",
        escape_terminal_controls(&data.comment_id),
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.board_id)
    )
}

pub(super) fn render_comment_deleted(data: &CommentDeleteSuccess) -> String {
    format!(
        "Permanently deleted comment {} from card {} on board {}.",
        escape_terminal_controls(&data.comment_id),
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.board_id)
    )
}
