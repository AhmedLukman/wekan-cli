use super::formatting::escape_terminal_controls;
use crate::command_result::{ProfileItem, ProfileListSuccess, ProfileRemoveSuccess};

pub(super) fn render_profile_item(action: &str, data: &ProfileItem) -> String {
    format!(
        "{action}: {}\nServer: {}\nActive: {}",
        escape_terminal_controls(&data.name),
        escape_terminal_controls(&data.server),
        if data.active { "yes" } else { "no" }
    )
}

pub(super) fn render_profile_list(data: &ProfileListSuccess) -> String {
    if data.profiles.is_empty() {
        return "No profiles configured.".to_owned();
    }
    let name_width = data
        .profiles
        .iter()
        .map(|profile| profile.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let server_width = data
        .profiles
        .iter()
        .map(|profile| profile.server.len())
        .max()
        .unwrap_or(6)
        .max(6);
    let mut lines = vec![format!(
        "{:<name_width$}  {:<server_width$}  ACTIVE",
        "NAME", "SERVER"
    )];
    lines.extend(data.profiles.iter().map(|profile| {
        format!(
            "{:<name_width$}  {:<server_width$}  {}",
            escape_terminal_controls(&profile.name),
            escape_terminal_controls(&profile.server),
            if profile.active { "yes" } else { "no" }
        )
    }));
    lines.join("\n")
}

pub(super) fn render_profile_removed(data: &ProfileRemoveSuccess) -> String {
    let mut message = format!(
        "Removed profile: {}\nServer: {}",
        escape_terminal_controls(&data.name),
        escape_terminal_controls(&data.server)
    );
    if data.active_profile.is_none() {
        message.push_str("\nNo profile is active.");
    }
    message
}
