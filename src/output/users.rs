use super::formatting::{
    escape_terminal_controls, push_optional, push_optional_bool, two_column_table,
};
use crate::command_result::{
    UserBoardsSuccess, UserCardsSuccess, UserCreateSuccess, UserDeleteSuccess, UserDetail,
    UserListSuccess, UserLoginChangeSuccess, UserOwnershipSuccess,
};

pub(super) fn render_token_created(data: &crate::command_result::UserTokenCreateSuccess) -> String {
    use secrecy::ExposeSecret;
    format!(
        "User ID: {}\nAuth token: {}\nCreation requested: {}\nMutation confirmed: {}",
        escape_terminal_controls(&data.user_id),
        escape_terminal_controls(data.auth_token().expose_secret()),
        data.creation_requested,
        data.mutation_confirmed
    )
}

pub(super) fn render_token_deleted(data: &crate::command_result::UserTokenDeleteSuccess) -> String {
    format!(
        "User ID: {}\nScope: {}\nMessage: {}\nDeletion requested: true\nMutation confirmed: false",
        escape_terminal_controls(&data.user_id),
        match data.scope {
            crate::command_result::UserTokenDeleteScope::Single => "single",
            crate::command_result::UserTokenDeleteScope::All => "all",
        },
        escape_terminal_controls(&data.message)
    )
}

pub(super) fn render_user_detail(data: &UserDetail) -> String {
    let mut lines = vec![format!(
        "User ID: {}",
        escape_terminal_controls(&data.user_id)
    )];
    push_optional(&mut lines, "Username", data.username.as_deref());
    push_optional(&mut lines, "Full name", data.full_name.as_deref());
    push_optional_bool(&mut lines, "Administrator", data.is_admin);
    push_optional_bool(&mut lines, "Login disabled", data.login_disabled);
    push_optional(
        &mut lines,
        "Authentication method",
        data.authentication_method.as_deref(),
    );
    push_optional(&mut lines, "Created", data.created_at.as_deref());
    push_optional(&mut lines, "Modified", data.modified_at.as_deref());
    push_optional(
        &mut lines,
        "Last connection",
        data.last_connection_date.as_deref(),
    );
    if !data.emails.is_empty() {
        let emails = data
            .emails
            .iter()
            .map(|email| {
                let address =
                    escape_terminal_controls(email.address.as_deref().unwrap_or("unknown address"));
                let verified = match email.verified {
                    Some(true) => "verified",
                    Some(false) => "unverified",
                    None => "verification unknown",
                };
                format!("{address} ({verified})")
            })
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("Emails: {emails}"));
    }
    if !data.organizations.is_empty() {
        lines.push(format!("Organizations: {}", data.organizations.len()));
    }
    if !data.teams.is_empty() {
        lines.push(format!("Teams: {}", data.teams.len()));
    }
    if !data.boards.is_empty() {
        lines.push(format!("Board memberships: {}", data.boards.len()));
    }
    let value = serde_json::to_value(data).expect("user details serialize");
    for name in [
        "profile",
        "created_through_api",
        "heartbeat",
        "session_data",
        "import_usernames",
    ] {
        super::formatting::render_named_value_complete(&mut lines, 0, name, &value[name]);
    }
    lines.join("\n")
}

pub(super) fn render_user_list(data: &UserListSuccess) -> String {
    if data.users.is_empty() {
        return "No users found.".to_owned();
    }
    let rows = data
        .users
        .iter()
        .map(|user| {
            (
                escape_terminal_controls(&user.user_id),
                escape_terminal_controls(user.username.as_deref().unwrap_or("<none>")),
            )
        })
        .collect::<Vec<_>>();
    two_column_table("ID", "USERNAME", &rows)
}

pub(super) fn render_user_cards(data: &UserCardsSuccess) -> String {
    if data.cards.is_empty() {
        return "No matching cards found.".to_owned();
    }
    let mut lines = vec!["ID  TITLE  BOARD  LIST  DUE".to_owned()];
    lines.extend(data.cards.iter().map(|card| {
        format!(
            "{}  {}  {}  {}  {}",
            escape_terminal_controls(&card.card_id),
            escape_terminal_controls(card.title.as_deref().unwrap_or("")),
            escape_terminal_controls(card.board_id.as_deref().unwrap_or("")),
            escape_terminal_controls(card.list_id.as_deref().unwrap_or("")),
            escape_terminal_controls(card.due_at.as_deref().unwrap_or("")),
        )
    }));
    lines.join("\n")
}

pub(super) fn render_user_created(data: &UserCreateSuccess) -> String {
    let mut lines = vec![format!(
        "Requested user creation for {} ({}) — unconfirmed",
        escape_terminal_controls(&data.username),
        escape_terminal_controls(&data.email)
    )];
    if let Some(user_id) = &data.user_id {
        lines.push(format!("User ID: {}", escape_terminal_controls(user_id)));
    } else {
        lines.push("User ID: unavailable".to_owned());
    }
    if data.warning.is_some() {
        lines.push(
            "Warning [user_id_unavailable_in_wekan_v11_06]: Wekan acknowledged the request without confirming creation or returning an ID."
                .to_owned(),
        );
    }
    lines.join("\n")
}

pub(super) fn render_user_boards(data: &UserBoardsSuccess) -> String {
    if data.boards.is_empty() {
        return format!(
            "No active boards found for user {}.",
            escape_terminal_controls(&data.user_id)
        );
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

pub(super) fn render_user_ownership(data: &UserOwnershipSuccess) -> String {
    let mut lines = vec![format!(
        "Transferred {} board(s) from user {} to user {}.",
        data.boards.len(),
        escape_terminal_controls(&data.from_user_id),
        escape_terminal_controls(&data.to_user_id)
    )];
    if !data.boards.is_empty() {
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
        lines.push(two_column_table("ID", "TITLE", &rows));
    }
    lines.join("\n")
}

pub(super) fn render_user_login_change(data: &UserLoginChangeSuccess) -> String {
    let action = match data.action {
        crate::command_result::UserLoginAction::Disabled => "Requested disabling login for",
        crate::command_result::UserLoginAction::Enabled => "Requested enabling login for",
    };
    format!(
        "{action} user {}.\n{}",
        escape_terminal_controls(&data.user.user_id),
        render_user_detail(&data.user)
    )
}

pub(super) fn render_user_deleted(data: &UserDeleteSuccess) -> String {
    let mut lines = vec![format!(
        "Deleted user {}.",
        escape_terminal_controls(&data.user_id)
    )];
    if data.deleted_current_user {
        lines.push(if data.local_credential_removed {
            "Removed the matching local credential.".to_owned()
        } else if data.credential_stored {
            "The local credential changed concurrently and was preserved.".to_owned()
        } else {
            "No matching local credential remained.".to_owned()
        });
    }
    lines.join("\n")
}
