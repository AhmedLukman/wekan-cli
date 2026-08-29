use std::ffi::OsString;

use super::{OutputFormat, render_error, render_success};
use crate::{
    command_result::{
        AuthStatusEmail, AuthStatusSuccess, AuthStatusUser, AuthSuccess, BoardCountSuccess,
        BoardCreateSuccess, BoardDeleteSuccess, BoardListScope, BoardListSuccess,
        BoardRenameSuccess, BoardSummary, CancellationSuccess, CardCollectionSuccess,
        CardCreateSuccess, CardDeleteMode, CardDeleteSuccess, CardSubmittedField, CardSummary,
        CardUpdateSuccess, CommandSuccess, DestructiveOperation, ListCollectionSuccess,
        ListCreateSuccess, ListDeleteMode, ListDeleteSuccess, ListDetail, ListSummary,
        ListUpdateSuccess, ListUpdatedField, ListWipLimitDetail, LogoutScope, LogoutSuccess,
        ProfileItem, ProfileListSuccess, ProfileRemoveSuccess, SwimlaneCollectionSuccess,
        SwimlaneCreateSuccess, SwimlaneDeleteMode, SwimlaneDeleteSuccess, SwimlaneDetail,
        SwimlaneSummary, SwimlaneUpdateSuccess, SwimlaneUpdatedField, UserBoardSummary,
        UserBoardsSuccess, UserCard, UserCardsSuccess, UserCreateSuccess, UserCreateWarning,
        UserDeleteSuccess, UserDetail, UserEmail, UserListSuccess, UserOwnershipSuccess,
        UserSummary,
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

#[test]
fn list_outputs_are_stable_comprehensive_and_terminal_safe() {
    let collection = CommandSuccess::ListCollection(ListCollectionSuccess {
        board_id: "board-1".to_owned(),
        lists: vec![ListSummary {
            list_id: "list-1".to_owned(),
            title: "Todo\nnext".to_owned(),
            modified_at: Some("2026-08-28T00:00:00Z".to_owned()),
            cards_modified_at: None,
        }],
    });
    let collection_json: serde_json::Value =
        serde_json::from_str(&render_success(OutputFormat::Json, &collection)).unwrap();
    assert_eq!(collection_json["data"]["board_id"], "board-1");
    assert_eq!(
        collection_json["data"]["lists"][0]["cards_modified_at"],
        serde_json::Value::Null
    );
    assert!(render_success(OutputFormat::Human, &collection).contains(r"Todo\nnext"));

    let shown = CommandSuccess::ListShown(ListDetail {
        list_id: "list-1".to_owned(),
        title: "Todo\u{1b}]52;c;x\u{7}".to_owned(),
        starred: Some(true),
        archived: false,
        archived_at: None,
        deleted_at: None,
        deleted_by: None,
        delete_batch_id: None,
        board_id: "board-1".to_owned(),
        swimlane_id: Some("swimlane-1".to_owned()),
        created_at: "2026-08-28T00:00:00Z".to_owned(),
        sort: Some(1.into()),
        updated_at: None,
        modified_at: "2026-08-28T00:00:00Z".to_owned(),
        position_updated_at: None,
        wip_limit: Some(ListWipLimitDetail {
            value: 2.into(),
            enabled: true,
            soft: false,
        }),
        color: Some("silver".to_owned()),
        list_type: "list".to_owned(),
        width: Some(220.into()),
    });
    let shown_json: serde_json::Value =
        serde_json::from_str(&render_success(OutputFormat::Json, &shown)).unwrap();
    assert_eq!(shown_json["data"]["wip_limit"]["value"], 2);
    assert_eq!(shown_json["data"]["list_type"], "list");
    let shown_human = render_success(OutputFormat::Human, &shown);
    assert!(!shown_human.contains('\u{1b}'));
    assert!(!shown_human.contains('\u{7}'));

    let created = CommandSuccess::ListCreated(ListCreateSuccess {
        board_id: "board-1".to_owned(),
        list_id: "list-1".to_owned(),
    });
    let updated = CommandSuccess::ListUpdated(ListUpdateSuccess {
        board_id: "board-1".to_owned(),
        list_id: "list-1".to_owned(),
        updated_fields: vec![ListUpdatedField::Title, ListUpdatedField::WipLimit],
    });
    let deleted = CommandSuccess::ListDeleted(ListDeleteSuccess {
        board_id: "board-1".to_owned(),
        list_id: "list-1".to_owned(),
        deleted: true,
        delete_mode: ListDeleteMode::Soft,
    });
    let updated_json: serde_json::Value =
        serde_json::from_str(&render_success(OutputFormat::Json, &updated)).unwrap();
    assert_eq!(
        updated_json["data"]["updated_fields"],
        serde_json::json!(["title", "wip_limit"])
    );
    let deleted_json: serde_json::Value =
        serde_json::from_str(&render_success(OutputFormat::Json, &deleted)).unwrap();
    assert_eq!(deleted_json["data"]["delete_mode"], "soft");
    assert!(render_success(OutputFormat::Human, &created).contains("Created list"));
    assert!(render_success(OutputFormat::Human, &updated).contains("title, wip_limit"));
    assert!(render_success(OutputFormat::Human, &deleted).contains("Soft-deleted"));
}

#[test]
fn card_outputs_are_stable_comprehensive_and_terminal_safe() {
    let collection = CommandSuccess::CardCollection(CardCollectionSuccess {
        board_id: "board-1".to_owned(),
        list_id: "list-1".to_owned(),
        cards: vec![CardSummary {
            card_id: "card-1".to_owned(),
            title: Some("Todo\nnext".to_owned()),
            description: Some("Details".to_owned()),
            swimlane_id: Some("swimlane-1".to_owned()),
            received_at: None,
            start_at: None,
            due_at: Some("2030-01-02T03:04:05Z".to_owned()),
            end_at: None,
            assignees: vec!["user-1".to_owned()],
            sort: Some(2.into()),
        }],
    });
    let collection_json: serde_json::Value =
        serde_json::from_str(&render_success(OutputFormat::Json, &collection)).unwrap();
    assert_eq!(collection_json["data"]["list_id"], "list-1");
    assert_eq!(
        collection_json["data"]["cards"][0]["received_at"],
        serde_json::Value::Null
    );
    let collection_human = render_success(OutputFormat::Human, &collection);
    assert!(collection_human.contains(r"Todo\nnext"));
    assert!(collection_human.contains("Details"));
    assert!(collection_human.contains("user-1"));

    let card = crate::command_result::CardDetail {
        card_id: "card-1".to_owned(),
        title: Some("Todo\u{001b}]52;c;x\u{0007}".to_owned()),
        archived: false,
        archived_at: None,
        deleted_at: None,
        deleted_by: None,
        delete_batch_id: None,
        parent_id: None,
        list_id: None,
        swimlane_id: "swimlane-1".to_owned(),
        board_id: None,
        cover_id: None,
        color: None,
        created_at: "2030-01-02T03:04:05Z".to_owned(),
        modified_at: "2030-01-02T03:04:05Z".to_owned(),
        custom_fields: vec![],
        date_last_activity: "2030-01-02T03:04:05Z".to_owned(),
        description: None,
        requested_by: None,
        assigned_by: None,
        label_ids: vec![],
        members: vec![],
        assignees: vec![],
        requesters: vec![],
        assigners: vec![],
        received_at: None,
        start_at: None,
        due_at: None,
        end_at: None,
        due_complete: None,
        stickers: vec![],
        location_name: None,
        location_address: None,
        location_latitude: None,
        location_longitude: None,
        locations: vec![],
        spent_time: None,
        is_overtime: None,
        user_id: "user-1".to_owned(),
        sort: None,
        subtask_sort: None,
        card_type: "cardType-card".to_owned(),
        linked_id: None,
        card_dependencies: vec![],
        vote: None,
        poker: None,
        target_id_gantt: vec![],
        link_type_gantt: vec![],
        link_id_gantt: vec![],
        card_number: None,
        show_activities: false,
        show_list_on_minicard: None,
        show_checklist_at_minicard: None,
        hide_finished_checklist_if_items_are_hidden: None,
    };
    let shown = CommandSuccess::CardShown(Box::new(card));
    let shown_json: serde_json::Value =
        serde_json::from_str(&render_success(OutputFormat::Json, &shown)).unwrap();
    assert_eq!(shown_json["data"]["card_type"], "cardType-card");
    assert_eq!(shown_json["data"]["members"], serde_json::json!([]));
    assert_eq!(shown_json["data"]["parent_id"], serde_json::Value::Null);
    let shown_human = render_success(OutputFormat::Human, &shown);
    assert!(!shown_human.contains('\u{1b}'));
    assert!(!shown_human.contains('\u{7}'));
    assert!(shown_human.contains("parent_id: null"));
    assert!(shown_human.contains("members: [0]"));

    let created = CommandSuccess::CardCreated(CardCreateSuccess {
        board_id: "board-1".to_owned(),
        list_id: "list-1".to_owned(),
        card_id: "card-1".to_owned(),
    });
    let updated = CommandSuccess::CardUpdated(CardUpdateSuccess {
        board_id: "board-1".to_owned(),
        list_id: "list-1".to_owned(),
        card_id: "card-1".to_owned(),
        submitted_fields: vec![
            CardSubmittedField::Title,
            CardSubmittedField::Sort,
            CardSubmittedField::DueAt,
            CardSubmittedField::IsOverTime,
        ],
    });
    let deleted = CommandSuccess::CardDeleted(CardDeleteSuccess {
        board_id: "board-1".to_owned(),
        list_id: "list-1".to_owned(),
        card_id: "card-1".to_owned(),
        deleted: true,
        delete_mode: CardDeleteMode::Hard,
    });
    assert!(render_success(OutputFormat::Human, &created).contains("Created card"));
    let updated_human = render_success(OutputFormat::Human, &updated);
    assert!(updated_human.contains("Updated card"));
    assert!(updated_human.contains("submitted fields: title, sort, due_at, is_over_time"));
    let updated_json: serde_json::Value =
        serde_json::from_str(&render_success(OutputFormat::Json, &updated)).unwrap();
    assert_eq!(
        updated_json["data"]["submitted_fields"],
        serde_json::json!(["title", "sort", "due_at", "is_over_time"])
    );
    assert!(updated_json["data"].get("updated_fields").is_none());
    assert!(render_success(OutputFormat::Human, &deleted).contains("Permanently deleted"));
    let deleted_json: serde_json::Value =
        serde_json::from_str(&render_success(OutputFormat::Json, &deleted)).unwrap();
    assert_eq!(deleted_json["data"]["delete_mode"], "hard");
}

#[test]
fn swimlane_outputs_are_stable_comprehensive_and_terminal_safe() {
    let collection = CommandSuccess::SwimlaneCollection(SwimlaneCollectionSuccess {
        board_id: "board-1".to_owned(),
        swimlanes: vec![SwimlaneSummary {
            swimlane_id: "swimlane-1".to_owned(),
            title: "Delivery\nnext".to_owned(),
        }],
    });
    let collection_json: serde_json::Value =
        serde_json::from_str(&render_success(OutputFormat::Json, &collection)).unwrap();
    assert_eq!(collection_json["data"]["board_id"], "board-1");
    assert_eq!(
        collection_json["data"]["swimlanes"][0]["swimlane_id"],
        "swimlane-1"
    );
    assert!(render_success(OutputFormat::Human, &collection).contains(r"Delivery\nnext"));

    let shown = CommandSuccess::SwimlaneShown(SwimlaneDetail {
        swimlane_id: "swimlane-1".to_owned(),
        title: "Delivery\u{1b}]52;c;x\u{7}".to_owned(),
        archived: false,
        archived_at: None,
        board_id: "board-1".to_owned(),
        created_at: "2026-08-28T00:00:00Z".to_owned(),
        sort: Some(2.into()),
        color: Some("silver".to_owned()),
        updated_at: Some("2026-08-28T00:30:00Z".to_owned()),
        modified_at: "2026-08-28T01:00:00Z".to_owned(),
        swimlane_type: "swimlane".to_owned(),
        height: Some((-1).into()),
    });
    let shown_json: serde_json::Value =
        serde_json::from_str(&render_success(OutputFormat::Json, &shown)).unwrap();
    assert_eq!(shown_json["data"]["swimlane_type"], "swimlane");
    assert_eq!(shown_json["data"]["height"], -1);
    let shown_human = render_success(OutputFormat::Human, &shown);
    assert!(!shown_human.contains('\u{1b}'));
    assert!(!shown_human.contains('\u{7}'));

    let created = CommandSuccess::SwimlaneCreated(SwimlaneCreateSuccess {
        board_id: "board-1".to_owned(),
        swimlane_id: "swimlane-1".to_owned(),
    });
    let updated = CommandSuccess::SwimlaneUpdated(SwimlaneUpdateSuccess {
        board_id: "board-1".to_owned(),
        swimlane_id: "swimlane-1".to_owned(),
        updated_fields: vec![SwimlaneUpdatedField::Title],
    });
    let deleted = CommandSuccess::SwimlaneDeleted(SwimlaneDeleteSuccess {
        board_id: "board-1".to_owned(),
        swimlane_id: "swimlane-1".to_owned(),
        deleted: true,
        delete_mode: SwimlaneDeleteMode::Hard,
    });
    let updated_json: serde_json::Value =
        serde_json::from_str(&render_success(OutputFormat::Json, &updated)).unwrap();
    assert_eq!(
        updated_json["data"]["updated_fields"],
        serde_json::json!(["title"])
    );
    let deleted_json: serde_json::Value =
        serde_json::from_str(&render_success(OutputFormat::Json, &deleted)).unwrap();
    assert_eq!(deleted_json["data"]["delete_mode"], "hard");
    assert!(render_success(OutputFormat::Human, &created).contains("Created swimlane"));
    assert!(render_success(OutputFormat::Human, &updated).contains("title"));
    assert!(render_success(OutputFormat::Human, &deleted).contains("Permanently deleted"));
}
