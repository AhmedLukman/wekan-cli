use super::formatting::escape_terminal_controls;
use crate::command_result::{AuthStatusSuccess, AuthSuccess, LogoutScope, LogoutSuccess};

pub(super) fn render_auth_success(action: &str, data: &AuthSuccess) -> String {
    let mut lines = vec![format!(
        "{action} {} on {}",
        escape_terminal_controls(&data.user_id),
        escape_terminal_controls(&data.server)
    )];
    lines.push(format!(
        "Profile: {}",
        escape_terminal_controls(&data.profile)
    ));
    lines.push(format!(
        "Profile created: {}",
        if data.profile_created { "yes" } else { "no" }
    ));
    lines.push(format!(
        "Profile active: {}",
        if data.profile_active { "yes" } else { "no" }
    ));
    lines.push(format!(
        "Token expires: {}",
        escape_terminal_controls(&data.token_expires)
    ));
    lines.push("Credentials stored securely.".to_owned());
    lines.join("\n")
}

pub(super) fn render_logout(data: &LogoutSuccess) -> String {
    let server = escape_terminal_controls(&data.server);
    let mut rendered = match data.logout_scope {
        LogoutScope::CurrentToken => format!(
            "Logged out from {server}.\nCurrent login token revoked.\n{}",
            render_local_credential_state(data)
        ),
        LogoutScope::AllTokens => format!(
            "Logged out from {server}.\nAll login tokens revoked.\n{}",
            render_local_credential_state(data)
        ),
        LogoutScope::LocalOnly => format!(
            "{} for {server}.\nNo Wekan login tokens were revoked.",
            if data.local_credential_removed {
                "Removed the stored credential"
            } else {
                "No stored credential was present"
            }
        ),
    };
    let first_newline = rendered.find('\n').unwrap_or(rendered.len());
    rendered.insert_str(
        first_newline,
        &format!("\nProfile: {}", escape_terminal_controls(&data.profile)),
    );
    rendered
}

pub(super) fn render_local_credential_state(data: &LogoutSuccess) -> &'static str {
    if data.local_credential_removed {
        "Stored credential removed."
    } else if data.credential_stored {
        match data.logout_scope {
            LogoutScope::AllTokens => {
                "A concurrently changed stored credential was preserved; verify it before use."
            }
            LogoutScope::CurrentToken | LogoutScope::LocalOnly => {
                "A newer stored credential was preserved."
            }
        }
    } else {
        "Stored credential was already absent."
    }
}

pub(super) fn render_auth_status(data: &AuthStatusSuccess) -> String {
    let identity = data.user.username.as_deref().unwrap_or(&data.user.user_id);
    let mut lines = vec![
        format!(
            "Authenticated as {} on {}",
            escape_terminal_controls(identity),
            escape_terminal_controls(&data.server)
        ),
        format!("User ID: {}", escape_terminal_controls(&data.user.user_id)),
    ];

    lines.push(format!(
        "Profile: {}",
        escape_terminal_controls(&data.profile)
    ));

    if let Some(full_name) = &data.user.full_name {
        lines.push(format!(
            "Full name: {}",
            escape_terminal_controls(full_name)
        ));
    }
    if let Some(is_admin) = data.user.is_admin {
        lines.push(format!(
            "Administrator: {}",
            if is_admin { "yes" } else { "no" }
        ));
    }
    if !data.user.emails.is_empty() {
        let emails = data
            .user
            .emails
            .iter()
            .map(|email| {
                let address = email.address.as_deref().unwrap_or("unknown address");
                let verification = match email.verified {
                    Some(true) => "verified",
                    Some(false) => "unverified",
                    None => "verification unknown",
                };
                format!("{} ({verification})", escape_terminal_controls(address))
            })
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("Emails: {emails}"));
    }
    lines.push(format!(
        "Token expires: {}",
        escape_terminal_controls(&data.token_expires)
    ));
    lines.push("Credentials stored securely.".to_owned());
    lines.join("\n")
}
