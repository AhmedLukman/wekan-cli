use crate::support::*;
use wekan_cli::command_result::{BoardCreateSuccess, CardDetail, ListCreateSuccess};

type LiveApp = App<CapturingStore, FixedPassword, FileProfileStore>;

pub(super) async fn verify_archive_restore(
    admin: &LiveApp,
    normal: &LiveApp,
    denied_roles: &[(&str, &LiveApp)],
    server: &str,
    board: &BoardCreateSuccess,
    list: &ListCreateSuccess,
    parent: &str,
) {
    let child = create_child(admin, server, board, list, parent, "Archive child").await;
    let grandchild = create_child(admin, server, board, list, &child, "Archive grandchild").await;
    let ids = [parent, child.as_str(), grandchild.as_str()];
    let before = get(admin, server, &board.board_id, &list.list_id, parent).await;
    let token = admin
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let http = reqwest::Client::new();

    let other_board =
        execute_live_board(admin, server, &["create", "--title", "Archive scope probe"])
            .await
            .unwrap();
    let CommandSuccess::BoardCreated(other_board) = other_board else {
        panic!("expected board creation")
    };

    for (command, action) in [("archive", "archive"), ("restore", "unarchive")] {
        for (role, app) in denied_roles {
            let error = change(app, server, command, &board.board_id, &list.list_id, parent)
                .await
                .unwrap_err();
            assert_eq!(
                error.code(),
                ErrorCode::PermissionDenied,
                "{role} {command}"
            );
            assert_eq!(error.details().http_status, Some(403));
            assert_ne!(error.details().outcome_unknown, Some(true));
        }
        let response = http
            .post(format!(
                "{server}/api/boards/{}/lists/{}/cards/{parent}/{action}",
                board.board_id, list.list_id
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
        for (bad_board, bad_list, bad_card, status) in [
            ("missing-board", list.list_id.as_str(), parent, 404),
            (board.board_id.as_str(), "wrong-list", parent, 500),
            (
                board.board_id.as_str(),
                list.list_id.as_str(),
                "missing-card",
                500,
            ),
            (
                other_board.board_id.as_str(),
                list.list_id.as_str(),
                parent,
                500,
            ),
        ] {
            let error = change(admin, server, command, bad_board, bad_list, bad_card)
                .await
                .unwrap_err();
            assert_eq!(
                error.details().http_status,
                Some(status),
                "{command}: {bad_board}/{bad_list}/{bad_card}: {error:?}"
            );
            assert_eq!(
                error.code(),
                if status == 404 {
                    ErrorCode::NotFound
                } else {
                    ErrorCode::ServerError
                }
            );
            assert_eq!(
                error.details().outcome_unknown,
                (status == 500).then_some(true)
            );
        }
    }
    // Restore rejects an active card before any state change.
    assert_wrong_state(
        admin,
        server,
        "restore",
        &board.board_id,
        &list.list_id,
        parent,
    )
    .await;
    assert_eq!(
        get(admin, server, &board.board_id, &list.list_id, parent).await,
        before
    );

    let CommandSuccess::CardArchived(archived) = change(
        normal,
        server,
        "archive",
        &board.board_id,
        &list.list_id,
        parent,
    )
    .await
    .unwrap() else {
        panic!("expected archive result")
    };
    assert_eq!(
        (
            &archived.board_id,
            &archived.list_id,
            archived.card_id.as_str()
        ),
        (&board.board_id, &list.list_id, parent)
    );
    assert!(archived.archived);
    OffsetDateTime::parse(
        &archived.archived_at,
        &time::format_description::well_known::Rfc3339,
    )
    .unwrap();
    assert_wrong_state(
        admin,
        server,
        "archive",
        &board.board_id,
        &list.list_id,
        parent,
    )
    .await;
    let CommandSuccess::CardCollection(collection) = execute_live_card(
        admin,
        server,
        &["list", "--board", &board.board_id, "--list", &list.list_id],
    )
    .await
    .unwrap() else {
        panic!("expected collection")
    };
    assert!(
        !collection
            .cards
            .iter()
            .any(|card| ids.contains(&card.card_id.as_str()))
    );
    let mut stored_timestamps = Vec::new();
    for id in ids {
        let error = execute_live_card(
            admin,
            server,
            &[
                "get",
                id,
                "--board",
                &board.board_id,
                "--list",
                &list.list_id,
            ],
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::NotFound);
        let document = live_api_json(
            &http,
            server,
            &token,
            reqwest::Method::GET,
            &format!("api/cards/{id}"),
            None,
        )
        .await;
        assert_eq!(document["archived"], true);
        stored_timestamps.push(
            document["archivedAt"]
                .as_str()
                .expect("archived document timestamp")
                .to_owned(),
        );
    }
    // Correct scope is required even when the card is archived.
    let wrong_scope = change(
        admin,
        server,
        "restore",
        &board.board_id,
        "wrong-list",
        parent,
    )
    .await
    .unwrap_err();
    assert_eq!(wrong_scope.details().http_status, Some(500));
    let CommandSuccess::CardRestored(restored) = change(
        normal,
        server,
        "restore",
        &board.board_id,
        &list.list_id,
        parent,
    )
    .await
    .unwrap() else {
        panic!("expected restore result")
    };
    assert_eq!(
        (
            &restored.board_id,
            &restored.list_id,
            restored.card_id.as_str()
        ),
        (&board.board_id, &list.list_id, parent)
    );
    assert!(!restored.archived);
    for (id, timestamp) in ids.into_iter().zip(stored_timestamps) {
        let detail = get(admin, server, &board.board_id, &list.list_id, id).await;
        assert!(!detail.archived);
        assert_eq!(detail.archived_at.as_deref(), Some(timestamp.as_str()));
    }
    let CommandSuccess::CardCollection(collection) = execute_live_card(
        admin,
        server,
        &["list", "--board", &board.board_id, "--list", &list.list_id],
    )
    .await
    .unwrap() else {
        panic!("expected collection")
    };
    assert!(
        ids.iter()
            .all(|id| collection.cards.iter().any(|card| card.card_id == *id))
    );
    assert_wrong_state(
        normal,
        server,
        "restore",
        &board.board_id,
        &list.list_id,
        parent,
    )
    .await;
    // Board administrators also use the same typed command path.
    change(
        admin,
        server,
        "archive",
        &board.board_id,
        &list.list_id,
        parent,
    )
    .await
    .unwrap();
    change(
        admin,
        server,
        "restore",
        &board.board_id,
        &list.list_id,
        parent,
    )
    .await
    .unwrap();
    execute_live_card(
        admin,
        server,
        &[
            "delete",
            &child,
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--yes",
        ],
    )
    .await
    .unwrap();
    execute_live_board(admin, server, &["delete", &other_board.board_id, "--yes"])
        .await
        .unwrap();
}

async fn create_child(
    app: &LiveApp,
    server: &str,
    board: &BoardCreateSuccess,
    list: &ListCreateSuccess,
    parent: &str,
    title: &str,
) -> String {
    let CommandSuccess::CardCreated(card) = execute_live_card(
        app,
        server,
        &[
            "create",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--swimlane-id",
            &board.default_swimlane_id,
            "--title",
            title,
        ],
    )
    .await
    .unwrap() else {
        panic!("expected child creation")
    };
    execute_live_card(
        app,
        server,
        &[
            "update",
            &card.card_id,
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--parent-id",
            parent,
        ],
    )
    .await
    .unwrap();
    card.card_id
}

async fn change(
    app: &LiveApp,
    server: &str,
    command: &str,
    board: &str,
    list: &str,
    card: &str,
) -> Result<CommandSuccess, AppError> {
    execute_live_card(
        app,
        server,
        &[command, card, "--board", board, "--list", list],
    )
    .await
}

async fn assert_wrong_state(
    app: &LiveApp,
    server: &str,
    command: &str,
    board: &str,
    list: &str,
    card: &str,
) {
    let error = change(app, server, command, board, list, card)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ServerError);
    assert_eq!(error.details().http_status, Some(500));
    assert_eq!(error.details().outcome_unknown, Some(true));
}

async fn get(app: &LiveApp, server: &str, board: &str, list: &str, card: &str) -> CardDetail {
    let CommandSuccess::CardShown(detail) = execute_live_card(
        app,
        server,
        &["get", card, "--board", board, "--list", list],
    )
    .await
    .unwrap() else {
        panic!("expected card detail")
    };
    *detail
}
