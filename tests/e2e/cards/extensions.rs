use crate::support::*;
use wekan_cli::command_result::{BoardCreateSuccess, CardDetail, ListCreateSuccess};

pub(super) async fn verify_complete_create_and_update(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    board: &BoardCreateSuccess,
    list: &ListCreateSuccess,
    source_card_id: &str,
) {
    let CommandSuccess::CardCreated(linked) = execute_live_card(
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
            "--linked-id",
            source_card_id,
        ],
    )
    .await
    .expect("linked creation without a title must succeed") else {
        panic!("expected linked-card creation");
    };
    let linked_detail = get(app, server, &board.board_id, &list.list_id, &linked.card_id).await;
    assert_eq!(linked_detail.card_type, "cardType-linkedCard");
    assert_eq!(linked_detail.linked_id.as_deref(), Some(source_card_id));

    let missing_link = execute_live_card(
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
            "--linked-id",
            "missing-card",
        ],
    )
    .await
    .expect_err("an absent linked source must be rejected by Wekan");
    assert_eq!(missing_link.code(), ErrorCode::NotFound, "{missing_link:?}");

    let CommandSuccess::CardCreated(created) = execute_live_card(
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
            "Extended fields",
        ],
    )
    .await
    .unwrap() else {
        panic!("expected card creation");
    };
    let card_id = &created.card_id;
    update(app, server, &board.board_id, &list.list_id, card_id, &[
        "--vote", r#"{"question":"Ship?","positive":["ignored"],"negative":["ignored"],"end":"2030-01-02T03:04:05Z","public":true,"allowNonBoardMembers":true}"#,
        "--poker", r#"{"question":true,"one":["ignored"],"two":["ignored"],"three":["ignored"],"five":["ignored"],"eight":["ignored"],"thirteen":["ignored"],"twenty":["ignored"],"forty":["ignored"],"oneHundred":["ignored"],"unsure":["ignored"],"end":"2030-01-02T03:04:05Z","allowNonBoardMembers":true,"estimation":3}"#,
        "--custom-fields", r#"[{"_id":"text","value":"hello"},{"_id":"number","value":2.5},{"_id":"bool","value":false},{"_id":"multi","value":["a",null,"b"]},{"_id":"empty","value":null},{"_id":null}]"#,
        "--stickers", r#"[{"icon":"star","name":"Milestone","highlight":"round","position":2}]"#,
        "--locations", r#"[{"_id":"office","name":"Office","address":"Nairobi","latitude":-1.28,"longitude":36.82}]"#,
    ]).await;
    let detail = get(app, server, &board.board_id, &list.list_id, card_id).await;
    let vote = detail.vote.as_ref().expect("vote settings must persist");
    assert_eq!(vote.question, "Ship?");
    assert!(vote.public && vote.allow_non_board_members);
    assert!(vote.positive.is_empty() && vote.negative.is_empty());
    let poker = detail.poker.as_ref().expect("poker settings must persist");
    assert_eq!(poker.question, Some(true));
    assert!(
        [
            &poker.one,
            &poker.two,
            &poker.three,
            &poker.five,
            &poker.eight,
            &poker.thirteen,
            &poker.twenty,
            &poker.forty,
            &poker.one_hundred,
            &poker.unsure
        ]
        .iter()
        .all(|values| values.is_empty())
    );
    assert_eq!(poker.allow_non_board_members, Some(true));
    assert_eq!(poker.estimation, Some(serde_json::Number::from(3)));
    assert_eq!(detail.custom_fields.len(), 6);
    assert_eq!(
        serde_json::to_value(&detail.custom_fields[3].value).unwrap(),
        serde_json::json!(["a", null, "b"])
    );
    assert!(detail.custom_fields[4].value.is_none());
    assert!(detail.custom_fields[5].custom_field_id.is_none());
    assert_eq!(vote.end.as_deref(), Some("2030-01-02T03:04:05.000Z"));
    assert_eq!(poker.end.as_deref(), Some("2030-01-02T03:04:05.000Z"));
    assert_eq!(detail.stickers[0].icon.as_deref(), Some("star"));
    assert_eq!(detail.locations[0].location_id, "office");
    assert_eq!(detail.locations[0].name.as_deref(), Some("Office"));

    update(
        app,
        server,
        &board.board_id,
        &list.list_id,
        card_id,
        &[
            "--vote",
            r#"{"question":"Again?","end":null}"#,
            "--poker",
            r#"{"question":null,"end":null,"allowNonBoardMembers":null,"estimation":null}"#,
            "--locations",
            r#"[{"latitude":null}]"#,
        ],
    )
    .await;
    let defaults = get(app, server, &board.board_id, &list.list_id, card_id).await;
    let vote = defaults.vote.unwrap();
    assert!(!vote.public && !vote.allow_non_board_members);
    assert!(vote.end.is_none());
    let poker = defaults.poker.unwrap();
    assert!(poker.question.is_none() && poker.end.is_none());
    assert!(poker.allow_non_board_members.is_none() && poker.estimation.is_none());
    assert!(!defaults.locations[0].location_id.is_empty());
    assert_eq!(defaults.locations[0].name.as_deref(), Some(""));
    assert_eq!(
        defaults.locations[0].latitude,
        Some(serde_json::Number::from(0))
    );

    update(
        app,
        server,
        &board.board_id,
        &list.list_id,
        card_id,
        &[
            "--custom-fields",
            "[]",
            "--stickers",
            "[]",
            "--locations",
            "[]",
        ],
    )
    .await;
    let cleared = get(app, server, &board.board_id, &list.list_id, card_id).await;
    assert!(cleared.custom_fields.is_empty());
    assert!(cleared.stickers.is_empty());
    assert!(cleared.locations.is_empty());

    update(
        app,
        server,
        &board.board_id,
        &list.list_id,
        card_id,
        &["--archive", "true"],
    )
    .await;
    let archived = execute_live_card(
        app,
        server,
        &[
            "get",
            card_id,
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
        ],
    )
    .await
    .expect_err("the scoped get hides archived cards");
    assert_eq!(archived.code(), ErrorCode::NotFound);
    update(
        app,
        server,
        &board.board_id,
        &list.list_id,
        card_id,
        &["--archive", "false"],
    )
    .await;
    assert!(
        !get(app, server, &board.board_id, &list.list_id, card_id)
            .await
            .archived
    );

    let CommandSuccess::SwimlaneCreated(swimlane) = execute_live_swimlane(
        app,
        server,
        &["create", "--board", &board.board_id, "--title", "Alternate"],
    )
    .await
    .unwrap() else {
        panic!("expected swimlane creation");
    };
    let CommandSuccess::ListCreated(destination_list) = execute_live_list(
        app,
        server,
        &["create", "--board", &board.board_id, "--title", "Doing"],
    )
    .await
    .unwrap() else {
        panic!("expected list creation");
    };
    let rejected = execute_live_card(
        app,
        server,
        &[
            "update",
            card_id,
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--title",
            "Saved before invalid movement",
            "--swimlane-id",
            "missing-lane",
        ],
    )
    .await
    .expect_err("Wekan must reject an invalid same-board swimlane after the earlier title write");
    assert!(
        serde_json::to_value(rejected.details())
            .unwrap()
            .get("outcome_unknown")
            .is_none()
    );
    assert_eq!(
        get(app, server, &board.board_id, &list.list_id, card_id)
            .await
            .title
            .as_deref(),
        Some("Saved before invalid movement")
    );

    let incomplete = execute_live_card(
        app,
        server,
        &[
            "update",
            card_id,
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--new-list-id",
            &destination_list.list_id,
        ],
    )
    .await
    .expect_err("Wekan ignores an incomplete cross-board move and reports no recognized update");
    assert_eq!(incomplete.code(), ErrorCode::NotFound);
    assert_eq!(
        get(app, server, &board.board_id, &list.list_id, card_id)
            .await
            .list_id
            .as_deref(),
        Some(list.list_id.as_str())
    );
    update(
        app,
        server,
        &board.board_id,
        &list.list_id,
        card_id,
        &[
            "--swimlane-id",
            &swimlane.swimlane_id,
            "--list-id",
            &destination_list.list_id,
            "--due-complete",
            "true",
            "--stickers",
            r#"[{"icon":"ignored-after-move"}]"#,
            "--locations",
            r#"[{"name":"Ignored after move"}]"#,
        ],
    )
    .await;
    let moved = get(
        app,
        server,
        &board.board_id,
        &destination_list.list_id,
        card_id,
    )
    .await;
    assert_eq!(moved.swimlane_id, swimlane.swimlane_id);
    assert_eq!(moved.due_complete, cleared.due_complete);
    assert!(moved.stickers.is_empty() && moved.locations.is_empty());
    assert_eq!(
        moved.list_id.as_deref(),
        Some(destination_list.list_id.as_str())
    );

    let CommandSuccess::BoardCreated(destination_board) = execute_live_board(
        app,
        server,
        &["create", "--title", "Cross-board destination"],
    )
    .await
    .unwrap() else {
        panic!("expected board creation");
    };
    let CommandSuccess::ListCreated(cross_list) = execute_live_list(
        app,
        server,
        &[
            "create",
            "--board",
            &destination_board.board_id,
            "--title",
            "Incoming",
        ],
    )
    .await
    .unwrap() else {
        panic!("expected list creation");
    };
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let canonical_server = ServerUrl::parse(server).unwrap();
    let (_member_directory, member_app, _) = register_live_board_role(
        server,
        canonical_server.as_str(),
        &token,
        &board.board_id,
        nonce,
        "normal",
    )
    .await;
    let denied = execute_live_card(
        &member_app,
        server,
        &[
            "update",
            card_id,
            "--board",
            &board.board_id,
            "--list",
            &destination_list.list_id,
            "--new-board-id",
            &destination_board.board_id,
            "--new-swimlane-id",
            &destination_board.default_swimlane_id,
            "--new-list-id",
            &cross_list.list_id,
        ],
    )
    .await
    .expect_err("moving to a private board requires destination write access");
    assert_eq!(denied.code(), ErrorCode::PermissionDenied);
    assert!(
        serde_json::to_value(denied.details())
            .unwrap()
            .get("outcome_unknown")
            .is_none()
    );
    assert_eq!(
        get(
            app,
            server,
            &board.board_id,
            &destination_list.list_id,
            card_id
        )
        .await
        .board_id
        .as_deref(),
        Some(board.board_id.as_str())
    );

    let missing_destination = execute_live_card(
        app,
        server,
        &[
            "update",
            card_id,
            "--board",
            &board.board_id,
            "--list",
            &destination_list.list_id,
            "--new-board-id",
            &destination_board.board_id,
            "--new-swimlane-id",
            &destination_board.default_swimlane_id,
            "--new-list-id",
            "missing-list",
        ],
    )
    .await
    .expect_err("Wekan must validate a cross-board destination list");
    assert_eq!(missing_destination.code(), ErrorCode::NotFound);
    assert!(
        serde_json::to_value(missing_destination.details())
            .unwrap()
            .get("outcome_unknown")
            .is_none()
    );
    update(
        app,
        server,
        &board.board_id,
        &destination_list.list_id,
        card_id,
        &[
            "--new-board-id",
            &destination_board.board_id,
            "--new-swimlane-id",
            &destination_board.default_swimlane_id,
            "--new-list-id",
            &cross_list.list_id,
            "--swimlane-id",
            "ignored-same-board-lane",
            "--list-id",
            "ignored-same-board-list",
        ],
    )
    .await;
    let moved = get(
        app,
        server,
        &destination_board.board_id,
        &cross_list.list_id,
        card_id,
    )
    .await;
    assert_eq!(
        moved.board_id.as_deref(),
        Some(destination_board.board_id.as_str())
    );
    assert_eq!(moved.swimlane_id, destination_board.default_swimlane_id);

    let denied_link = execute_live_card(
        &member_app,
        server,
        &[
            "create",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--swimlane-id",
            &board.default_swimlane_id,
            "--linked-id",
            card_id,
        ],
    )
    .await
    .expect_err("linking requires read access to the source card's private board");
    assert_eq!(denied_link.code(), ErrorCode::PermissionDenied);

    let CommandSuccess::CardCreated(cross_link) = execute_live_card(
        app,
        server,
        &[
            "create",
            "--board",
            &destination_board.board_id,
            "--list",
            &cross_list.list_id,
            "--swimlane-id",
            &destination_board.default_swimlane_id,
            "--linked-id",
            source_card_id,
        ],
    )
    .await
    .unwrap() else {
        panic!("expected cross-board linked creation");
    };
    assert_eq!(
        get(
            app,
            server,
            &destination_board.board_id,
            &cross_list.list_id,
            &cross_link.card_id
        )
        .await
        .linked_id
        .as_deref(),
        Some(source_card_id)
    );

    for id in [card_id, &cross_link.card_id] {
        execute_live_card(
            app,
            server,
            &[
                "delete",
                id,
                "--board",
                &destination_board.board_id,
                "--list",
                &cross_list.list_id,
                "--yes",
            ],
        )
        .await
        .unwrap();
    }
    execute_live_board(
        app,
        server,
        &["delete", &destination_board.board_id, "--yes"],
    )
    .await
    .unwrap();
    execute_live_card(
        app,
        server,
        &[
            "delete",
            &linked.card_id,
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--yes",
        ],
    )
    .await
    .unwrap();
}

async fn get(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    board: &str,
    list: &str,
    card: &str,
) -> CardDetail {
    let CommandSuccess::CardShown(detail) = execute_live_card(
        app,
        server,
        &["get", card, "--board", board, "--list", list],
    )
    .await
    .expect("extended card fields must pass strict response decoding") else {
        panic!("expected card detail");
    };
    *detail
}

async fn update(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    board: &str,
    list: &str,
    card: &str,
    fields: &[&str],
) {
    let mut args = vec!["update", card, "--board", board, "--list", list];
    args.extend_from_slice(fields);
    let CommandSuccess::CardUpdated(result) = execute_live_card(app, server, &args)
        .await
        .expect("the extended update must be accepted by Wekan")
    else {
        panic!("expected card update");
    };
    assert_eq!(result.card_id, card);
    assert!(!result.submitted_fields.is_empty());
}
