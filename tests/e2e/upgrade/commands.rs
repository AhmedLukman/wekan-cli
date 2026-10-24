use crate::support::{
    live::{LiveApp, run},
    *,
};
use serde_json::json;

pub(super) async fn verify(app: &LiveApp, server: &str, board: &str, swimlane: &str, nonce: u128) {
    let token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let canonical = ServerUrl::parse(server).unwrap().as_str().to_owned();
    let (_member_dir, member, member_id) =
        register_live_board_role(server, &canonical, &token, board, nonce + 800, "normal").await;
    assert_eq!(
        run(&member, server, &["user", "current"]).await.unwrap()["is_admin"],
        false
    );
    let list = run(
        &member,
        server,
        &[
            "list",
            "create",
            "--board",
            board,
            "--title",
            "Native route permissions",
        ],
    )
    .await
    .unwrap();
    let list = list["list_id"].as_str().unwrap();
    let mut cards = Vec::new();
    for title in ["Archived with list", "Archived independently"] {
        let card = run(
            &member,
            server,
            &[
                "card",
                "create",
                "--board",
                board,
                "--list",
                list,
                "--swimlane-id",
                swimlane,
                "--title",
                title,
            ],
        )
        .await
        .unwrap();
        cards.push(card["card_id"].as_str().unwrap().to_owned());
    }
    let card = cards[0].as_str();
    run(
        &member,
        server,
        &[
            "card", "archive", &cards[1], "--board", board, "--list", list,
        ],
    )
    .await
    .unwrap();
    let checklist = run(
        &member,
        server,
        &[
            "checklist",
            "create",
            "--board",
            board,
            "--card",
            card,
            "--title",
            "Before ordinary update",
        ],
    )
    .await
    .unwrap();
    let checklist = checklist["checklist_id"].as_str().unwrap();
    run(
        &member,
        server,
        &[
            "checklist",
            "update",
            checklist,
            "--board",
            board,
            "--card",
            card,
            "--title",
            "Ordinary checklist update",
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        run(
            &member,
            server,
            &[
                "checklist",
                "get",
                checklist,
                "--board",
                board,
                "--card",
                card
            ]
        )
        .await
        .unwrap()["title"],
        "Ordinary checklist update"
    );
    let item = run(
        &member,
        server,
        &[
            "checklist",
            "item",
            "create",
            "--board",
            board,
            "--card",
            card,
            "--checklist",
            checklist,
            "--title",
            "Deadline",
            "--due-at",
            "2030-06-15T12:00:00Z",
        ],
    )
    .await
    .unwrap();
    let item = item["item_id"].as_str().unwrap();
    let item_get = [
        "checklist",
        "item",
        "get",
        item,
        "--board",
        board,
        "--card",
        card,
        "--checklist",
        checklist,
    ];
    assert_eq!(
        run(&member, server, &item_get).await.unwrap()["due_at"],
        "2030-06-15T12:00:00.000Z"
    );
    run(
        &member,
        server,
        &[
            "checklist",
            "item",
            "update",
            item,
            "--board",
            board,
            "--card",
            card,
            "--checklist",
            checklist,
            "--due-at",
            "2031-07-16T12:13:14+03:00",
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        run(&member, server, &item_get).await.unwrap()["due_at"],
        "2031-07-16T09:13:14.000Z"
    );
    run(
        &member,
        server,
        &[
            "checklist",
            "item",
            "update",
            item,
            "--board",
            board,
            "--card",
            card,
            "--checklist",
            checklist,
            "--clear-due-at",
        ],
    )
    .await
    .unwrap();
    assert!(run(&member, server, &item_get).await.unwrap()["due_at"].is_null());
    // The collection is a separate native projection of the stored deadline.
    let shown = run(
        &member,
        server,
        &[
            "checklist",
            "get",
            checklist,
            "--board",
            board,
            "--card",
            card,
        ],
    )
    .await
    .unwrap();
    assert!(shown["items"][0]["due_at"].is_null());
    let comment = run(
        &member,
        server,
        &[
            "comment",
            "create",
            "--board",
            board,
            "--card",
            card,
            "--text",
            "Before ordinary update",
        ],
    )
    .await
    .unwrap();
    let comment = comment["comment_id"].as_str().unwrap();
    run(
        &member,
        server,
        &[
            "comment",
            "update",
            comment,
            "--board",
            board,
            "--card",
            card,
            "--text",
            " Ordinary comment update ",
        ],
    )
    .await
    .unwrap();
    let comment_get = ["comment", "get", comment, "--board", board, "--card", card];
    assert_eq!(
        run(&member, server, &comment_get).await.unwrap()["text"],
        "Ordinary comment update"
    );

    let run_id = format!("ordinaryRun{nonce}");
    crate::support::attachment_fixtures::fixture_command(
        "mongodb",
        "mongosh",
        "db.getSiblingDB('wekan').bot_runs.insertOne({_id:p.run,boardId:p.board,cardId:p.card,status:'queued',createdAt:new Date(),updatedAt:new Date(),events:[]});print('{}');",
        &json!({"run":run_id,"board":board,"card":card}),
    );
    let report_dir = TestDirectory::new("ordinary-bot-report");
    let report = report_dir.path().join("report.json");
    fs::write(
        &report,
        b"{\"status\":\"running\",\"text\":\"Ordinary runner\"}",
    )
    .unwrap();
    let bot_args = [
        "card",
        "bot-run",
        "report",
        &run_id,
        "--board",
        board,
        "--card",
        card,
        "--file",
        report.to_str().unwrap(),
    ];
    run(&member, server, &bot_args).await.unwrap();
    let stored_run = || {
        crate::support::attachment_fixtures::fixture_command(
            "mongodb",
            "mongosh",
            "print(JSON.stringify(db.getSiblingDB('wekan').bot_runs.findOne({_id:p.run})));",
            &json!({"run":run_id}),
        )
    };
    assert_eq!(stored_run()["status"], "running");
    assert_eq!(stored_run()["reportedBy"], member_id);

    run(
        &member,
        server,
        &["list", "archive", list, "--board", board],
    )
    .await
    .unwrap();
    assert_eq!(
        run(&member, server, &["list", "get", list, "--board", board])
            .await
            .unwrap_err()
            .code(),
        ErrorCode::NotFound
    );
    for card in &cards {
        assert_eq!(
            run(&member, server, &["card", "get", card]).await.unwrap()["archived"],
            true
        );
    }
    run(
        &member,
        server,
        &["list", "restore", list, "--board", board],
    )
    .await
    .unwrap();
    assert_eq!(
        run(&member, server, &["list", "get", list, "--board", board])
            .await
            .unwrap()["archived"],
        false
    );
    assert_eq!(
        run(&member, server, &["card", "get", card]).await.unwrap()["archived"],
        false
    );
    assert_eq!(
        run(&member, server, &["card", "get", &cards[1]])
            .await
            .unwrap()["archived"],
        true
    );

    // Each denied write gets its own actor: native security controls can disable it.
    let denied: Vec<Vec<&str>> = vec![
        vec![
            "checklist",
            "update",
            checklist,
            "--board",
            board,
            "--card",
            card,
            "--title",
            "Denied",
        ],
        vec![
            "checklist",
            "item",
            "update",
            item,
            "--board",
            board,
            "--card",
            card,
            "--checklist",
            checklist,
            "--due-at",
            "2032-01-01T00:00:00Z",
        ],
        vec![
            "comment", "update", comment, "--board", board, "--card", card, "--text", "Denied",
        ],
        vec!["list", "archive", list, "--board", board],
        bot_args.to_vec(),
    ];
    for (index, args) in denied.iter().enumerate() {
        let (_dir, reader, _) = register_live_board_role(
            server,
            &canonical,
            &token,
            board,
            nonce + 810 + index as u128,
            "readonly",
        )
        .await;
        for family in ["view-settings", "card-field-order"] {
            run(&reader, server, &["board", family, "get", "--board", board])
                .await
                .unwrap();
        }
        let error = run(&reader, server, args).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::PermissionDenied, "{args:?}");
        assert_eq!(error.details().http_status, Some(403), "{args:?}");
    }
    assert_eq!(
        run(
            app,
            server,
            &[
                "checklist",
                "get",
                checklist,
                "--board",
                board,
                "--card",
                card
            ]
        )
        .await
        .unwrap()["title"],
        "Ordinary checklist update"
    );
    assert!(run(app, server, &item_get).await.unwrap()["due_at"].is_null());
    assert_eq!(
        run(app, server, &comment_get).await.unwrap()["text"],
        "Ordinary comment update"
    );
    assert_eq!(
        run(app, server, &["card", "get", card]).await.unwrap()["archived"],
        false
    );
    assert_eq!(stored_run()["status"], "running");
    assert_eq!(stored_run()["events"].as_array().unwrap().len(), 1);
    for (index, args) in [
        vec![
            "board",
            "view-settings",
            "update",
            "--board",
            board,
            "--default-private-view",
            "board-view-lists",
        ],
        vec![
            "board",
            "card-field-order",
            "update",
            "--board",
            board,
            "--field",
            "description",
        ],
    ]
    .iter()
    .enumerate()
    {
        let (_dir, member, _) = register_live_board_role(
            server,
            &canonical,
            &token,
            board,
            nonce + 830 + index as u128,
            "normal",
        )
        .await;
        let before = run(app, server, &["board", args[1], "get", "--board", board])
            .await
            .unwrap();
        let error = run(&member, server, args).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::PermissionDenied);
        assert_eq!(error.details().http_status, Some(403));
        let after = run(app, server, &["board", args[1], "get", "--board", board])
            .await
            .unwrap();
        assert_eq!(after, before);
    }
}
