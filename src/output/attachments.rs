use crate::command_result::{AttachmentCollectionSuccess, AttachmentScope};

pub(super) fn list(data: &AttachmentCollectionSuccess) -> String {
    let mut lines = Vec::new();
    if data.requested_scope != AttachmentScope::Board {
        lines.push("Warning: Wekan v11.06 ignores swimlane/list/card selectors; these are all board attachments.".to_owned());
    }
    lines.push(format!("{} attachment(s)", data.count));
    for attachment in &data.attachments {
        lines.push(format!(
            "{}  {}  {} bytes",
            super::formatting::escape_terminal_controls(&attachment.attachment_id),
            super::formatting::escape_terminal_controls(&attachment.file_name),
            attachment.file_size
        ));
    }
    lines.join("\n")
}

pub(super) fn detail<T: serde::Serialize>(data: &T) -> String {
    serde_json::to_string_pretty(data).expect("typed attachment output serializes")
}
