use std::ffi::OsString;

use clap::ValueEnum;
use serde::Serialize;

use crate::command_result::{
    AuthStatusSuccess, AuthSuccess, BoardCountSuccess, BoardCreateSuccess, BoardDeleteSuccess,
    BoardDetail, BoardListScope, BoardListSuccess, BoardRenameSuccess, CommandSuccess, LogoutScope,
    LogoutSuccess, ProfileItem, ProfileListSuccess, ProfileRemoveSuccess, UserBoardsSuccess,
    UserCardsSuccess, UserCreateSuccess, UserDeleteSuccess, UserDetail, UserListSuccess,
    UserLoginChangeSuccess, UserOwnershipSuccess,
};
use crate::error::AppError;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
}

impl OutputFormat {
    pub fn detect_from_args(args: &[OsString]) -> Self {
        let mut iter = args.iter().filter_map(|arg| arg.to_str());
        while let Some(arg) = iter.next() {
            if arg == "--output" && iter.next().is_some_and(|value| value == "json") {
                return Self::Json;
            }
            if arg == "--output=json" {
                return Self::Json;
            }
        }
        Self::Human
    }
}

#[derive(Serialize)]
struct SuccessEnvelope<'a, T> {
    ok: bool,
    data: &'a T,
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    ok: bool,
    error: ErrorBody<'a>,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'static str,
    message: &'a str,
    details: &'a crate::error::ErrorDetails,
}

pub fn render_success(format: OutputFormat, success: &CommandSuccess) -> String {
    match (format, success) {
        (OutputFormat::Human, CommandSuccess::Cancelled(_)) => {
            "Cancelled; no changes made.".to_owned()
        }
        (OutputFormat::Json, CommandSuccess::Cancelled(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("cancellation success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::Registration(data)) => {
            render_auth_success("Registered user", data)
        }
        (OutputFormat::Json, CommandSuccess::Registration(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("registration success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::Login(data)) => {
            render_auth_success("Logged in user", data)
        }
        (OutputFormat::Json, CommandSuccess::Login(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("login success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::Logout(data)) => render_logout(data),
        (OutputFormat::Json, CommandSuccess::Logout(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("logout success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::AuthStatus(data)) => render_auth_status(data),
        (OutputFormat::Json, CommandSuccess::AuthStatus(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("authentication status success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::UserCurrent(data))
        | (OutputFormat::Human, CommandSuccess::UserShown(data)) => render_user_detail(data),
        (OutputFormat::Json, CommandSuccess::UserCurrent(data))
        | (OutputFormat::Json, CommandSuccess::UserShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserList(data)) => render_user_list(data),
        (OutputFormat::Json, CommandSuccess::UserList(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserCards(data)) => render_user_cards(data),
        (OutputFormat::Json, CommandSuccess::UserCards(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserCreated(data)) => render_user_created(data),
        (OutputFormat::Json, CommandSuccess::UserCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserBoards(data)) => render_user_boards(data),
        (OutputFormat::Json, CommandSuccess::UserBoards(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserOwnershipTaken(data)) => {
            render_user_ownership(data)
        }
        (OutputFormat::Json, CommandSuccess::UserOwnershipTaken(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserLoginChanged(data)) => {
            render_user_login_change(data)
        }
        (OutputFormat::Json, CommandSuccess::UserLoginChanged(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserDeleted(data)) => render_user_deleted(data),
        (OutputFormat::Json, CommandSuccess::UserDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardList(data)) => render_board_list(data),
        (OutputFormat::Json, CommandSuccess::BoardList(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardCount(data)) => render_board_count(data),
        (OutputFormat::Json, CommandSuccess::BoardCount(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardShown(data)) => render_board_detail(data),
        (OutputFormat::Json, CommandSuccess::BoardShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardCreated(data)) => render_board_created(data),
        (OutputFormat::Json, CommandSuccess::BoardCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardRenamed(data)) => render_board_renamed(data),
        (OutputFormat::Json, CommandSuccess::BoardRenamed(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardDeleted(data)) => render_board_deleted(data),
        (OutputFormat::Json, CommandSuccess::BoardDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ProfileAdded(data)) => {
            render_profile_item("Added profile", data)
        }
        (OutputFormat::Json, CommandSuccess::ProfileAdded(data))
        | (OutputFormat::Json, CommandSuccess::ProfileShown(data))
        | (OutputFormat::Json, CommandSuccess::ProfileUsed(data))
        | (OutputFormat::Json, CommandSuccess::ProfileUpdated(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("profile success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::ProfileList(data)) => render_profile_list(data),
        (OutputFormat::Json, CommandSuccess::ProfileList(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("profile list success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::ProfileShown(data)) => {
            render_profile_item("Profile", data)
        }
        (OutputFormat::Human, CommandSuccess::ProfileUsed(data)) => {
            render_profile_item("Using profile", data)
        }
        (OutputFormat::Human, CommandSuccess::ProfileUpdated(data)) => {
            render_profile_item("Updated profile", data)
        }
        (OutputFormat::Human, CommandSuccess::ProfileRemoved(data)) => render_profile_removed(data),
        (OutputFormat::Json, CommandSuccess::ProfileRemoved(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("profile removal success is always serializable")
        }
    }
}

fn render_json_success<T: Serialize>(data: &T) -> String {
    serde_json::to_string(&SuccessEnvelope { ok: true, data })
        .expect("command success is always serializable")
}

fn render_user_detail(data: &UserDetail) -> String {
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
    lines.join("\n")
}

fn push_optional(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value {
        lines.push(format!("{label}: {}", escape_terminal_controls(value)));
    }
}

fn push_optional_bool(lines: &mut Vec<String>, label: &str, value: Option<bool>) {
    if let Some(value) = value {
        lines.push(format!("{label}: {}", if value { "yes" } else { "no" }));
    }
}

fn render_user_list(data: &UserListSuccess) -> String {
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

fn render_user_cards(data: &UserCardsSuccess) -> String {
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

fn render_user_created(data: &UserCreateSuccess) -> String {
    let mut lines = vec![format!(
        "Created user {} ({})",
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
            "Warning [user_id_unavailable_in_wekan_v11_06]: Wekan created the user but did not return its ID."
                .to_owned(),
        );
    }
    lines.join("\n")
}

fn render_user_boards(data: &UserBoardsSuccess) -> String {
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

fn render_user_ownership(data: &UserOwnershipSuccess) -> String {
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

fn render_user_login_change(data: &UserLoginChangeSuccess) -> String {
    let action = match data.action {
        crate::command_result::UserLoginAction::Disabled => "Set loginDisabled=true for",
        crate::command_result::UserLoginAction::Enabled => "Cleared loginDisabled for",
    };
    format!(
        "{action} user {}.",
        escape_terminal_controls(&data.user.user_id)
    )
}

fn render_user_deleted(data: &UserDeleteSuccess) -> String {
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

fn render_board_list(data: &BoardListSuccess) -> String {
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

fn render_board_count(data: &BoardCountSuccess) -> String {
    format!(
        "Private boards: {}\nPublic boards: {}",
        data.private, data.public
    )
}

fn render_board_created(data: &BoardCreateSuccess) -> String {
    format!(
        "Created board {}.\nDefault swimlane ID: {}",
        escape_terminal_controls(&data.board_id),
        escape_terminal_controls(&data.default_swimlane_id)
    )
}

fn render_board_renamed(data: &BoardRenameSuccess) -> String {
    format!(
        "Renamed board {} to {}.",
        escape_terminal_controls(&data.board_id),
        escape_terminal_controls(&data.title)
    )
}

fn render_board_deleted(data: &BoardDeleteSuccess) -> String {
    format!(
        "Deleted board {}.",
        escape_terminal_controls(&data.board_id)
    )
}

fn render_board_detail(data: &BoardDetail) -> String {
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

fn board_field_group(name: &str) -> usize {
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

fn render_named_value(
    lines: &mut Vec<String>,
    indent: usize,
    name: &str,
    value: &serde_json::Value,
) {
    let padding = " ".repeat(indent);
    match value {
        serde_json::Value::Array(values) => {
            lines.push(format!("{padding}{name}: [{}]", values.len()));
            for (index, value) in values.iter().enumerate() {
                render_named_value(lines, indent + 2, &format!("[{index}]"), value);
            }
        }
        serde_json::Value::Object(values) => {
            lines.push(format!("{padding}{name}:"));
            for (name, value) in values {
                if !value.is_null() {
                    render_named_value(lines, indent + 2, name, value);
                }
            }
        }
        serde_json::Value::String(value) => lines.push(format!(
            "{padding}{name}: {}",
            escape_terminal_controls(value)
        )),
        other => lines.push(format!("{padding}{name}: {other}")),
    }
}

fn two_column_table(left_header: &str, right_header: &str, rows: &[(String, String)]) -> String {
    let left_width = rows
        .iter()
        .map(|(left, _)| left.len())
        .max()
        .unwrap_or(left_header.len())
        .max(left_header.len());
    let mut lines = vec![format!("{left_header:<left_width$}  {right_header}")];
    lines.extend(
        rows.iter()
            .map(|(left, right)| format!("{left:<left_width$}  {right}")),
    );
    lines.join("\n")
}

fn render_auth_success(action: &str, data: &AuthSuccess) -> String {
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

fn render_profile_item(action: &str, data: &ProfileItem) -> String {
    format!(
        "{action}: {}\nServer: {}\nActive: {}",
        escape_terminal_controls(&data.name),
        escape_terminal_controls(&data.server),
        if data.active { "yes" } else { "no" }
    )
}

fn render_profile_list(data: &ProfileListSuccess) -> String {
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

fn render_profile_removed(data: &ProfileRemoveSuccess) -> String {
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

fn render_logout(data: &LogoutSuccess) -> String {
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

fn render_local_credential_state(data: &LogoutSuccess) -> &'static str {
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

fn render_auth_status(data: &AuthStatusSuccess) -> String {
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

fn escape_terminal_controls(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control() {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
    }
    escaped
}

pub fn render_error(format: OutputFormat, error: &AppError) -> String {
    match format {
        OutputFormat::Human => format!(
            "Error [{}]: {}",
            error.code().as_str(),
            error.message().trim()
        ),
        OutputFormat::Json => serde_json::to_string(&ErrorEnvelope {
            ok: false,
            error: ErrorBody {
                code: error.code().as_str(),
                message: error.message(),
                details: error.details(),
            },
        })
        .expect("application errors are always serializable"),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{OutputFormat, render_error, render_success};
    use crate::{
        command_result::{
            AuthStatusEmail, AuthStatusSuccess, AuthStatusUser, AuthSuccess, BoardCountSuccess,
            BoardCreateSuccess, BoardDeleteSuccess, BoardListScope, BoardListSuccess,
            BoardRenameSuccess, BoardSummary, CancellationSuccess, CommandSuccess,
            DestructiveOperation, LogoutScope, LogoutSuccess, ProfileItem, ProfileListSuccess,
            ProfileRemoveSuccess, UserBoardSummary, UserBoardsSuccess, UserCard, UserCardsSuccess,
            UserCreateSuccess, UserCreateWarning, UserDeleteSuccess, UserDetail, UserEmail,
            UserListSuccess, UserOwnershipSuccess, UserSummary,
        },
        error::AppError,
    };

    fn success() -> CommandSuccess {
        CommandSuccess::Registration(AuthSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            user_id: "user-1".to_owned(),
            token_expires: "2030-01-02T03:04:05Z".to_owned(),
            credential_stored: true,
            profile_created: true,
            profile_active: true,
        })
    }

    #[test]
    fn renders_the_stable_json_success_envelope() {
        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &success())).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["data"]["user_id"], "user-1");
        assert_eq!(value["data"]["credential_stored"], true);
        assert!(value["data"].get("token").is_none());
    }

    #[test]
    fn cancellation_has_stable_human_and_json_output() {
        let cancellation = CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::ProfileRemove,
        ));

        assert_eq!(
            render_success(OutputFormat::Human, &cancellation),
            "Cancelled; no changes made."
        );
        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &cancellation)).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["data"]["cancelled"], true);
        assert_eq!(value["data"]["operation"], "profile_remove");
    }

    #[test]
    fn human_success_reports_metadata_without_a_token() {
        let rendered = render_success(OutputFormat::Human, &success());
        assert!(rendered.contains("user-1"));
        assert!(rendered.contains("https://wekan.example/"));
        assert!(rendered.contains("2030-01-02T03:04:05Z"));
        assert!(rendered.contains("Credentials stored securely."));
        assert!(!rendered.contains("token\":\""));
    }

    #[test]
    fn human_success_escapes_terminal_control_sequences() {
        let success = CommandSuccess::Registration(AuthSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            user_id: "user\u{1b}]52;c;clipboard\u{7}\nnext-line".to_owned(),
            token_expires: "2030-01-02T03:04:05Z".to_owned(),
            credential_stored: true,
            profile_created: false,
            profile_active: true,
        });

        let rendered = render_success(OutputFormat::Human, &success);

        assert!(!rendered.contains('\u{1b}'));
        assert!(!rendered.contains('\u{7}'));
        assert!(rendered.contains(r"user\u{1b}]52;c;clipboard\u{7}\nnext-line"));
    }

    #[test]
    fn login_success_uses_the_same_secret_free_json_shape() {
        let success = CommandSuccess::Login(AuthSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            user_id: "user-1".to_owned(),
            token_expires: "2030-01-02T03:04:05Z".to_owned(),
            credential_stored: true,
            profile_created: false,
            profile_active: true,
        });

        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &success)).unwrap();
        assert_eq!(value["data"]["user_id"], "user-1");
        assert!(value["data"].get("token").is_none());
        assert!(render_success(OutputFormat::Human, &success).starts_with("Logged in user"));
    }

    #[test]
    fn logout_success_reports_scope_and_local_credential_state() {
        let success = CommandSuccess::Logout(LogoutSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            logout_scope: LogoutScope::AllTokens,
            remote_logout_completed: true,
            credential_stored: false,
            local_credential_removed: true,
        });

        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &success)).unwrap();
        assert_eq!(value["data"]["logout_scope"], "all_tokens");
        assert_eq!(value["data"]["remote_logout_completed"], true);
        assert_eq!(value["data"]["credential_stored"], false);
        assert_eq!(value["data"]["local_credential_removed"], true);
        assert!(value["data"].get("token").is_none());
        assert!(render_success(OutputFormat::Human, &success).contains("All login tokens revoked"));
    }

    #[test]
    fn local_only_output_warns_that_remote_tokens_were_not_revoked() {
        let success = CommandSuccess::Logout(LogoutSuccess {
            server: "https://wekan.example/\u{1b}]52;c;clipboard\u{7}".to_owned(),
            profile: "default".to_owned(),
            logout_scope: LogoutScope::LocalOnly,
            remote_logout_completed: false,
            credential_stored: false,
            local_credential_removed: true,
        });

        let rendered = render_success(OutputFormat::Human, &success);
        assert!(rendered.contains("No Wekan login tokens were revoked"));
        assert!(!rendered.contains('\u{1b}'));
        assert!(!rendered.contains('\u{7}'));
    }

    #[test]
    fn local_only_output_distinguishes_an_already_absent_credential() {
        let success = CommandSuccess::Logout(LogoutSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            logout_scope: LogoutScope::LocalOnly,
            remote_logout_completed: false,
            credential_stored: false,
            local_credential_removed: false,
        });

        let rendered = render_success(OutputFormat::Human, &success);
        assert!(rendered.contains("No stored credential was present"));
        assert!(rendered.contains("No Wekan login tokens were revoked"));
    }

    #[test]
    fn remote_output_reports_that_a_newer_credential_was_preserved() {
        let success = CommandSuccess::Logout(LogoutSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            logout_scope: LogoutScope::CurrentToken,
            remote_logout_completed: true,
            credential_stored: true,
            local_credential_removed: false,
        });

        assert!(
            render_success(OutputFormat::Human, &success)
                .contains("A newer stored credential was preserved")
        );
    }

    #[test]
    fn all_tokens_output_requires_verification_of_a_changed_credential() {
        let success = CommandSuccess::Logout(LogoutSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            logout_scope: LogoutScope::AllTokens,
            remote_logout_completed: true,
            credential_stored: true,
            local_credential_removed: false,
        });

        assert!(render_success(OutputFormat::Human, &success).contains("verify it before use"));
    }

    #[test]
    fn status_success_uses_the_stable_nested_profile_shape() {
        let success = CommandSuccess::AuthStatus(AuthStatusSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            authenticated: true,
            token_expires: "2030-01-02T03:04:05Z".to_owned(),
            credential_stored: true,
            user: AuthStatusUser {
                user_id: "user-1".to_owned(),
                username: Some("alice".to_owned()),
                full_name: None,
                is_admin: Some(false),
                emails: vec![AuthStatusEmail {
                    address: Some("alice@example.com".to_owned()),
                    verified: None,
                }],
            },
        });

        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &success)).unwrap();
        assert_eq!(value["data"]["authenticated"], true);
        assert_eq!(value["data"]["user"]["user_id"], "user-1");
        assert_eq!(value["data"]["user"]["full_name"], serde_json::Value::Null);
        assert_eq!(
            value["data"]["user"]["emails"][0]["verified"],
            serde_json::Value::Null
        );
        assert!(value["data"].get("token").is_none());
    }

    #[test]
    fn human_status_omits_absent_profile_lines_and_escapes_controls() {
        let success = CommandSuccess::AuthStatus(AuthStatusSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            authenticated: true,
            token_expires: "2030-01-02T03:04:05Z".to_owned(),
            credential_stored: true,
            user: AuthStatusUser {
                user_id: "user-1".to_owned(),
                username: Some("alice\u{1b}]52;c;clipboard\u{7}".to_owned()),
                full_name: None,
                is_admin: None,
                emails: Vec::new(),
            },
        });

        let rendered = render_success(OutputFormat::Human, &success);
        assert!(!rendered.contains('\u{1b}'));
        assert!(!rendered.contains("Full name:"));
        assert!(!rendered.contains("Administrator:"));
        assert!(!rendered.contains("Emails:"));
        assert!(rendered.contains("Credentials stored securely."));
    }

    #[test]
    fn profile_list_output_is_stable_and_human_readable() {
        let success = CommandSuccess::ProfileList(ProfileListSuccess {
            active_profile: Some("local".to_owned()),
            profiles: vec![
                ProfileItem {
                    name: "local".to_owned(),
                    server: "http://localhost:3000/".to_owned(),
                    active: true,
                },
                ProfileItem {
                    name: "work".to_owned(),
                    server: "https://wekan.example/".to_owned(),
                    active: false,
                },
            ],
        });
        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &success)).unwrap();
        assert_eq!(value["data"]["active_profile"], "local");
        assert_eq!(value["data"]["profiles"][0]["active"], true);
        let human = render_success(OutputFormat::Human, &success);
        assert_eq!(
            human
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .collect::<Vec<_>>(),
            ["NAME", "SERVER", "ACTIVE"]
        );
        assert!(human.contains("http://localhost:3000/"));
    }

    #[test]
    fn empty_profile_list_and_removal_report_the_result() {
        let empty = CommandSuccess::ProfileList(ProfileListSuccess {
            active_profile: None,
            profiles: Vec::new(),
        });
        assert_eq!(
            render_success(OutputFormat::Human, &empty),
            "No profiles configured."
        );

        let removed = CommandSuccess::ProfileRemoved(ProfileRemoveSuccess {
            name: "local".to_owned(),
            server: "http://localhost:3000/".to_owned(),
            removed: true,
            active_profile: None,
        });
        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &removed)).unwrap();
        assert_eq!(value["data"]["removed"], true);
        assert_eq!(value["data"]["active_profile"], serde_json::Value::Null);
        assert!(render_success(OutputFormat::Human, &removed).contains("No profile is active"));
    }

    #[test]
    fn auth_output_identifies_profile_and_initialization_state() {
        let named = CommandSuccess::Login(AuthSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "work".to_owned(),
            user_id: "user-1".to_owned(),
            token_expires: "2030-01-02T03:04:05Z".to_owned(),
            credential_stored: true,
            profile_created: true,
            profile_active: false,
        });
        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &named)).unwrap();
        assert_eq!(value["data"]["profile"], "work");
        assert_eq!(value["data"]["profile_created"], true);
        assert_eq!(value["data"]["profile_active"], false);
        assert!(render_success(OutputFormat::Human, &named).contains("Profile: work"));
    }

    #[test]
    fn renders_the_stable_json_error_envelope() {
        let value: serde_json::Value = serde_json::from_str(&render_error(
            OutputFormat::Json,
            &AppError::configuration("missing server"),
        ))
        .unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "configuration_error");
        assert_eq!(value["error"]["details"], serde_json::json!({}));
    }

    #[test]
    fn user_detail_output_is_typed_secret_free_and_terminal_safe() {
        let success = CommandSuccess::UserCurrent(UserDetail {
            user_id: "user-1".to_owned(),
            username: Some("alice\u{1b}]52;c;clipboard\u{7}".to_owned()),
            full_name: Some("Alice".to_owned()),
            emails: vec![UserEmail {
                address: Some("alice@example.com".to_owned()),
                verified: Some(true),
            }],
            is_admin: Some(false),
            login_disabled: Some(false),
            authentication_method: Some("password".to_owned()),
            created_at: None,
            modified_at: None,
            last_connection_date: None,
            organizations: Vec::new(),
            teams: Vec::new(),
            boards: Vec::new(),
        });

        let json: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &success)).unwrap();
        assert_eq!(json["data"]["user_id"], "user-1");
        for forbidden in [
            "services",
            "sessionData",
            "password",
            "token",
            "preferences",
        ] {
            assert!(json["data"].get(forbidden).is_none());
        }
        let human = render_success(OutputFormat::Human, &success);
        assert!(!human.contains('\u{1b}'));
        assert!(!human.contains('\u{7}'));
        assert!(human.contains(r"alice\u{1b}]52;c;clipboard\u{7}"));
    }

    #[test]
    fn user_lists_and_cards_have_stable_empty_and_escaped_output() {
        let users = CommandSuccess::UserList(UserListSuccess { users: Vec::new() });
        assert_eq!(
            render_success(OutputFormat::Human, &users),
            "No users found."
        );
        let user_json: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &users)).unwrap();
        assert_eq!(user_json["data"]["users"], serde_json::json!([]));

        let cards = CommandSuccess::UserCards(UserCardsSuccess {
            cards: vec![UserCard {
                card_id: "card-1".to_owned(),
                title: Some("Task\nnext".to_owned()),
                board_id: Some("board-1".to_owned()),
                swimlane_id: None,
                list_id: Some("list-1".to_owned()),
                due_at: None,
                start_at: None,
                end_at: None,
                members: Vec::new(),
                assignees: Vec::new(),
            }],
        });
        let human = render_success(OutputFormat::Human, &cards);
        assert!(human.starts_with("ID  TITLE  BOARD  LIST  DUE"));
        assert!(human.contains(r"Task\nnext"));
        assert_eq!(human.lines().count(), 2);
    }

    #[test]
    fn user_creation_and_mutation_results_are_stable() {
        let created = CommandSuccess::UserCreated(UserCreateSuccess {
            created: true,
            username: "bob".to_owned(),
            email: "bob@example.com".to_owned(),
            user_id: None,
            warning: Some(UserCreateWarning::UserIdUnavailableInWekanV1106),
        });
        let json: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &created)).unwrap();
        assert_eq!(json["data"]["created"], true);
        assert_eq!(json["data"]["user_id"], serde_json::Value::Null);
        assert_eq!(
            json["data"]["warning"],
            "user_id_unavailable_in_wekan_v11_06"
        );
        assert!(
            render_success(OutputFormat::Human, &created)
                .contains("Warning [user_id_unavailable_in_wekan_v11_06]")
        );

        let ownership = CommandSuccess::UserOwnershipTaken(UserOwnershipSuccess {
            from_user_id: "owner-1".to_owned(),
            to_user_id: "admin-1".to_owned(),
            boards: vec![UserBoardSummary {
                board_id: "board-1".to_owned(),
                title: "Board".to_owned(),
            }],
        });
        let json: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &ownership)).unwrap();
        assert_eq!(json["data"]["boards"][0]["board_id"], "board-1");
        assert!(render_success(OutputFormat::Human, &ownership).contains("Transferred 1 board"));

        let boards = CommandSuccess::UserBoards(UserBoardsSuccess {
            user_id: "user-1".to_owned(),
            boards: vec![UserBoardSummary {
                board_id: "board-1".to_owned(),
                title: "Board\u{1b}".to_owned(),
            }],
        });
        assert!(!render_success(OutputFormat::Human, &boards).contains('\u{1b}'));

        let deleted = CommandSuccess::UserDeleted(UserDeleteSuccess {
            user_id: "user-1".to_owned(),
            deleted: true,
            deleted_current_user: true,
            credential_stored: false,
            local_credential_removed: true,
        });
        let json: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &deleted)).unwrap();
        assert_eq!(json["data"]["deleted"], true);
        assert_eq!(json["data"]["deleted_current_user"], true);
        assert_eq!(json["data"]["local_credential_removed"], true);
        let rendered = render_success(OutputFormat::Human, &deleted);
        assert!(rendered.contains("Deleted user user-1"));
        assert!(rendered.contains("local credential"));

        let list = CommandSuccess::UserList(UserListSuccess {
            users: vec![UserSummary {
                user_id: "user-1".to_owned(),
                username: Some("alice".to_owned()),
            }],
        });
        assert!(render_success(OutputFormat::Human, &list).starts_with("ID      USERNAME"));

        let email_only = CommandSuccess::UserList(UserListSuccess {
            users: vec![UserSummary {
                user_id: "email-only".to_owned(),
                username: None,
            }],
        });
        assert!(render_success(OutputFormat::Human, &email_only).contains("<none>"));
    }

    #[test]
    fn detects_json_output_before_clap_parsing() {
        assert_eq!(
            OutputFormat::detect_from_args(&[
                OsString::from("wekan"),
                OsString::from("auth"),
                OsString::from("--output=json"),
                OsString::from("register"),
            ]),
            OutputFormat::Json
        );
    }

    #[test]
    fn board_outputs_are_stable_comprehensive_and_terminal_safe() {
        let list = CommandSuccess::BoardList(BoardListSuccess {
            scope: BoardListScope::Public,
            boards: vec![BoardSummary {
                board_id: "board-1".to_owned(),
                title: "Board\nnext".to_owned(),
            }],
        });
        let list_json: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &list)).unwrap();
        assert_eq!(list_json["data"]["scope"], "public");
        assert_eq!(list_json["data"]["boards"][0]["board_id"], "board-1");
        assert!(render_success(OutputFormat::Human, &list).contains(r"Board\nnext"));

        let board = serde_json::from_value(serde_json::json!({
            "board_id": "board-1",
            "title": "Board\u{1b}]52;c;clipboard\u{7}",
            "permission": "private",
            "allows_comments": true,
            "members": [{"user_id": "user-1", "is_admin": true, "is_active": true}],
            "watchers": [{"user_id": "watcher-1", "level": "tracking"}],
            "unknownSecret": "must-not-render"
        }))
        .unwrap();
        let shown = CommandSuccess::BoardShown(Box::new(board));
        let shown_json: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &shown)).unwrap();
        assert_eq!(shown_json["data"]["board_id"], "board-1");
        assert_eq!(shown_json["data"]["allows_comments"], true);
        assert_eq!(shown_json["data"]["watchers"][0]["level"], "tracking");
        assert!(shown_json["data"].get("unknownSecret").is_none());
        let shown_human = render_success(OutputFormat::Human, &shown);
        assert!(!shown_human.contains('\u{1b}'));
        assert!(!shown_human.contains('\u{7}'));
        assert!(!shown_human.contains("must-not-render"));
        assert!(shown_human.contains("Settings:"));
        assert!(shown_human.contains("watchers: [1]"));

        let count = CommandSuccess::BoardCount(BoardCountSuccess {
            private: 4,
            public: 2,
        });
        assert!(render_success(OutputFormat::Human, &count).contains("Private boards: 4"));

        let created = CommandSuccess::BoardCreated(BoardCreateSuccess {
            board_id: "board-1".to_owned(),
            default_swimlane_id: "swimlane-1".to_owned(),
        });
        let renamed = CommandSuccess::BoardRenamed(BoardRenameSuccess {
            board_id: "board-1".to_owned(),
            title: "Renamed".to_owned(),
        });
        let deleted = CommandSuccess::BoardDeleted(BoardDeleteSuccess {
            board_id: "board-1".to_owned(),
            deleted: true,
        });
        assert!(render_success(OutputFormat::Human, &created).contains("swimlane-1"));
        assert!(render_success(OutputFormat::Human, &renamed).contains("Renamed"));
        assert!(render_success(OutputFormat::Human, &deleted).contains("Deleted board"));
    }
}
