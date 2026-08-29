use std::ffi::OsString;

use clap::ValueEnum;
use serde::Serialize;

use crate::command_result::{
    AuthStatusSuccess, AuthSuccess, BoardCountSuccess, BoardCreateSuccess, BoardDeleteSuccess,
    BoardDetail, BoardListScope, BoardListSuccess, BoardRenameSuccess, CardCollectionSuccess,
    CardCreateSuccess, CardDeleteSuccess, CardDetail, CardUpdateSuccess, CommandSuccess,
    ListCollectionSuccess, ListCreateSuccess, ListDeleteSuccess, ListDetail, ListUpdateSuccess,
    LogoutScope, LogoutSuccess, ProfileItem, ProfileListSuccess, ProfileRemoveSuccess,
    SwimlaneCollectionSuccess, SwimlaneCreateSuccess, SwimlaneDeleteSuccess, SwimlaneDetail,
    SwimlaneUpdateSuccess, UserBoardsSuccess, UserCardsSuccess, UserCreateSuccess,
    UserDeleteSuccess, UserDetail, UserListSuccess, UserLoginChangeSuccess, UserOwnershipSuccess,
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
        (OutputFormat::Human, CommandSuccess::ListCollection(data)) => render_list_collection(data),
        (OutputFormat::Json, CommandSuccess::ListCollection(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ListShown(data)) => render_list_detail(data),
        (OutputFormat::Json, CommandSuccess::ListShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ListCreated(data)) => render_list_created(data),
        (OutputFormat::Json, CommandSuccess::ListCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ListUpdated(data)) => render_list_updated(data),
        (OutputFormat::Json, CommandSuccess::ListUpdated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ListDeleted(data)) => render_list_deleted(data),
        (OutputFormat::Json, CommandSuccess::ListDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardCollection(data)) => render_card_collection(data),
        (OutputFormat::Json, CommandSuccess::CardCollection(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardShown(data)) => render_card_detail(data),
        (OutputFormat::Json, CommandSuccess::CardShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardCreated(data)) => render_card_created(data),
        (OutputFormat::Json, CommandSuccess::CardCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardUpdated(data)) => render_card_updated(data),
        (OutputFormat::Json, CommandSuccess::CardUpdated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardDeleted(data)) => render_card_deleted(data),
        (OutputFormat::Json, CommandSuccess::CardDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::SwimlaneCollection(data)) => {
            render_swimlane_collection(data)
        }
        (OutputFormat::Json, CommandSuccess::SwimlaneCollection(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::SwimlaneShown(data)) => render_swimlane_detail(data),
        (OutputFormat::Json, CommandSuccess::SwimlaneShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::SwimlaneCreated(data)) => {
            render_swimlane_created(data)
        }
        (OutputFormat::Json, CommandSuccess::SwimlaneCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::SwimlaneUpdated(data)) => {
            render_swimlane_updated(data)
        }
        (OutputFormat::Json, CommandSuccess::SwimlaneUpdated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::SwimlaneDeleted(data)) => {
            render_swimlane_deleted(data)
        }
        (OutputFormat::Json, CommandSuccess::SwimlaneDeleted(data)) => render_json_success(data),
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

fn render_list_collection(data: &ListCollectionSuccess) -> String {
    if data.lists.is_empty() {
        return format!(
            "No lists found on board {}.",
            escape_terminal_controls(&data.board_id)
        );
    }
    let mut lines = vec!["ID  TITLE  MODIFIED  CARDS MODIFIED".to_owned()];
    lines.extend(data.lists.iter().map(|list| {
        format!(
            "{}  {}  {}  {}",
            escape_terminal_controls(&list.list_id),
            escape_terminal_controls(&list.title),
            escape_terminal_controls(list.modified_at.as_deref().unwrap_or("")),
            escape_terminal_controls(list.cards_modified_at.as_deref().unwrap_or("")),
        )
    }));
    lines.join("\n")
}

fn render_list_detail(data: &ListDetail) -> String {
    let value = serde_json::to_value(data).expect("list documents are always serializable");
    let serde_json::Value::Object(mut fields) = value else {
        unreachable!("list documents serialize as objects")
    };
    fields.remove("list_id");
    fields.remove("title");

    let mut lines = vec![
        format!("List ID: {}", escape_terminal_controls(&data.list_id)),
        format!("Title: {}", escape_terminal_controls(&data.title)),
    ];
    for (name, value) in fields {
        if !value.is_null() {
            render_named_value(&mut lines, 0, &name, &value);
        }
    }
    lines.join("\n")
}

fn render_list_created(data: &ListCreateSuccess) -> String {
    format!(
        "Created list {} on board {}.",
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id)
    )
}

fn render_list_updated(data: &ListUpdateSuccess) -> String {
    let fields = data
        .updated_fields
        .iter()
        .map(|field| {
            serde_json::to_value(field)
                .expect("updated list fields are always serializable")
                .as_str()
                .expect("updated list fields serialize as strings")
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Updated list {} on board {}: {}.",
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id),
        escape_terminal_controls(&fields)
    )
}

fn render_list_deleted(data: &ListDeleteSuccess) -> String {
    format!(
        "Soft-deleted list {} on board {} and its live cards.",
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id)
    )
}

fn render_card_collection(data: &CardCollectionSuccess) -> String {
    if data.cards.is_empty() {
        return format!(
            "No cards found in list {} on board {}.",
            escape_terminal_controls(&data.list_id),
            escape_terminal_controls(&data.board_id)
        );
    }
    let mut lines = vec![
        "ID  TITLE  DESCRIPTION  SWIMLANE  RECEIVED  START  DUE  END  ASSIGNEES  SORT".to_owned(),
    ];
    lines.extend(data.cards.iter().map(|card| {
        format!(
            "{}  {}  {}  {}  {}  {}  {}  {}  {}  {}",
            escape_terminal_controls(&card.card_id),
            escape_terminal_controls(card.title.as_deref().unwrap_or("")),
            escape_terminal_controls(card.description.as_deref().unwrap_or("")),
            escape_terminal_controls(card.swimlane_id.as_deref().unwrap_or("")),
            escape_terminal_controls(card.received_at.as_deref().unwrap_or("")),
            escape_terminal_controls(card.start_at.as_deref().unwrap_or("")),
            escape_terminal_controls(card.due_at.as_deref().unwrap_or("")),
            escape_terminal_controls(card.end_at.as_deref().unwrap_or("")),
            escape_terminal_controls(&card.assignees.join(",")),
            card.sort
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )
    }));
    lines.join("\n")
}

fn render_card_detail(data: &CardDetail) -> String {
    let value = serde_json::to_value(data).expect("card documents are always serializable");
    let serde_json::Value::Object(mut fields) = value else {
        unreachable!("card documents serialize as objects")
    };
    fields.remove("card_id");

    let mut lines = vec![format!(
        "Card ID: {}",
        escape_terminal_controls(&data.card_id)
    )];
    for (name, value) in fields {
        render_named_value_complete(&mut lines, 0, &name, &value);
    }
    lines.join("\n")
}

fn render_card_created(data: &CardCreateSuccess) -> String {
    format!(
        "Created card {} in list {} on board {}.",
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id)
    )
}

fn render_card_updated(data: &CardUpdateSuccess) -> String {
    let fields = data
        .submitted_fields
        .iter()
        .map(|field| {
            serde_json::to_value(field)
                .expect("submitted card fields are always serializable")
                .as_str()
                .expect("submitted card fields serialize as strings")
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Updated card {} in list {} on board {}; submitted fields: {}.",
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id),
        escape_terminal_controls(&fields)
    )
}

fn render_card_deleted(data: &CardDeleteSuccess) -> String {
    format!(
        "Permanently deleted card {} from list {} on board {}.",
        escape_terminal_controls(&data.card_id),
        escape_terminal_controls(&data.list_id),
        escape_terminal_controls(&data.board_id)
    )
}

fn render_swimlane_collection(data: &SwimlaneCollectionSuccess) -> String {
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

fn render_swimlane_detail(data: &SwimlaneDetail) -> String {
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

fn render_swimlane_created(data: &SwimlaneCreateSuccess) -> String {
    format!(
        "Created swimlane {} on board {}.",
        escape_terminal_controls(&data.swimlane_id),
        escape_terminal_controls(&data.board_id)
    )
}

fn render_swimlane_updated(data: &SwimlaneUpdateSuccess) -> String {
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

fn render_swimlane_deleted(data: &SwimlaneDeleteSuccess) -> String {
    format!(
        "Permanently deleted swimlane {} on board {}.",
        escape_terminal_controls(&data.swimlane_id),
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

fn render_named_value_complete(
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
                render_named_value_complete(lines, indent + 2, &format!("[{index}]"), value);
            }
        }
        serde_json::Value::Object(values) => {
            lines.push(format!("{padding}{name}:"));
            for (name, value) in values {
                render_named_value_complete(lines, indent + 2, name, value);
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
mod tests;
