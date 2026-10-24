use super::support::{
    live::{LiveApp, run},
    *,
};
use serde_json::Value;

#[path = "upgrade/admin.rs"]
mod admin;
#[path = "upgrade/commands.rs"]
mod commands;
#[path = "upgrade/metadata.rs"]
mod metadata;

async fn native_board_json(app: &LiveApp, server: &str, board: &str) -> Value {
    let cli = Cli::try_parse_from([
        "wekan",
        "--server",
        server,
        "--output",
        "raw",
        "board",
        "export",
        board,
        "--format",
        "json",
        "--destination",
        "-",
    ])
    .unwrap();
    cli.command.validate_output(OutputFormat::Raw).unwrap();
    let success = app.execute(cli.command).await.unwrap();
    let mut bytes = Vec::new();
    let mut errors = Vec::new();
    write_success(OutputFormat::Raw, success, &mut bytes, &mut errors)
        .await
        .unwrap();
    assert!(errors.is_empty());
    serde_json::from_slice(&bytes).unwrap()
}

async fn nested_metadata_lifecycle(
    app: &LiveApp,
    server: &str,
    board: &str,
    card: &str,
    checklist: &str,
    nonce: u128,
) {
    use serde_json::json;
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
    let (_member_directory, member_app, member_id) =
        register_live_board_role(server, &canonical, &token, board, nonce + 713, "normal").await;
    let ordinary_label = run(
        app,
        server,
        &[
            "board",
            "label",
            "add",
            "--board",
            board,
            "--name",
            "Ordinary metadata label",
            "--color",
            "green",
        ],
    )
    .await
    .unwrap();
    let ordinary_label_id = ordinary_label["label_id"].as_str().unwrap();
    let detail = run(
        app,
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
    let item = detail["items"][0]["item_id"].as_str().unwrap();
    let item_args = [
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
    let before = run(&member_app, server, &item_args).await.unwrap();
    assert!(before["assignee_id"].is_null());
    assert!(before["linked_card_id"].is_null());
    let before = run(&member_app, server, &["board", "get", board])
        .await
        .unwrap();
    let ordinary_label = before["labels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|label| label["label_id"] == ordinary_label_id)
        .expect("the ordinary label must be readable before adding metadata");
    assert!(ordinary_label["due_at"].is_null());
    assert!(ordinary_label["shared_label_id"].is_null());
    let member = before["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["user_id"] == member_id)
        .unwrap();
    assert!(member["granted_by"].is_null());
    assert!(member["custom_role_id"].is_null());
    let before = native_board_json(&member_app, server, board).await;
    let exported_item = before["checklistItems"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["_id"] == item)
        .unwrap();
    assert!(exported_item.get("assigneeId").is_none());
    assert!(exported_item.get("linkedCardId").is_none());
    let exported_label = before["labels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|label| label["_id"] == ordinary_label_id)
        .expect("native export must include the ordinary label");
    assert!(exported_label.get("dueAt").is_none());
    assert!(exported_label.get("sharedLabelId").is_none());

    // Exercise the same native producers as the Shared Labels, custom-role,
    // label deadline and checklist assignment/subtask UI at the pinned version.
    let (shared, label, role, linked) =
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            let mut session = ddp::Session::login(server, &token).await;
            let shared = session
                .call(
                    "sharedLabels.save",
                    json!([null,{"name":"Shared milestone","color":"red","scope":"instance"}]),
                )
                .await;
            let label = session
                .call("boards.useSharedLabel", json!([board, shared]))
                .await;
            let board_document = run(app, server, &["board", "get", board]).await.unwrap();
            let index = board_document["labels"]
                .as_array()
                .unwrap()
                .iter()
                .position(|entry| entry["label_id"] == label)
                .unwrap();
            let due = time::OffsetDateTime::parse(
                "2030-01-02T03:04:05Z",
                &time::format_description::well_known::Rfc3339,
            )
            .unwrap()
            .unix_timestamp()
                * 1000;
            session
                .call(
                    "/boards/update",
                    json!([{"_id":board},{"$set":{format!("labels.{index}.dueAt"):{"$date":due}}}]),
                )
                .await;
            let linked = session
                .call(
                    "addSubtaskCard",
                    json!([card, "Linked metadata subtask", false]),
                )
                .await;
            session
                .call(
                    "/checklistItems/update",
                    json!([{"_id":item},{"$set":{"assigneeId":member_id,"linkedCardId":linked}}]),
                )
                .await;
            let role = session
                .call(
                    "customBoardRoles.save",
                    json!([null,{"name":"Metadata reader","capabilities":{"seesAllCards":true}}]),
                )
                .await;
            session
                .call(
                    "boards.setMemberCustomRole",
                    json!([board, member_id, role]),
                )
                .await;
            session.close().await;
            (shared, label, role, linked)
        })
        .await
        .expect("native nested metadata fixture timed out");
    // Group membership writes this exact string-array shape (grantOf/updateFor
    // in models/lib/groupBoardMembership.js); fixture that independent producer.
    let origins = json!([format!("team:metadata-{nonce}")]);
    super::support::attachment_fixtures::fixture_command(
        "mongodb",
        "mongosh",
        "const w=db.getSiblingDB('wekan'),b=w.boards.findOne({_id:p.board}),i=b.members.findIndex(m=>m.userId===p.member);if(i<0)throw Error('member missing');w.boards.updateOne({_id:p.board},{$set:{['members.'+i+'.grantedBy']:p.origins}});print('{}');",
        &json!({"board":board,"member":member_id,"origins":origins}),
    );
    for reader in [app, &member_app] {
        let shown = run(reader, server, &item_args).await.unwrap();
        assert_eq!(shown["assignee_id"], member_id);
        assert_eq!(shown["linked_card_id"], linked);
        let shown = run(reader, server, &["board", "get", board]).await.unwrap();
        let shown_label = shown["labels"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["label_id"] == label)
            .unwrap();
        assert_eq!(shown_label["shared_label_id"], shared);
        assert_eq!(shown_label["due_at"], "2030-01-02T03:04:05.000Z");
        let shown_member = shown["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["user_id"] == member_id)
            .unwrap();
        assert_eq!(shown_member["granted_by"], origins);
        assert_eq!(shown_member["custom_role_id"], role);
        let labels = run(
            reader,
            server,
            &["board", "label", "list", "--board", board],
        )
        .await
        .unwrap();
        assert!(
            labels["labels"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["shared_label_id"] == shared
                    && entry["due_at"] == "2030-01-02T03:04:05.000Z")
        );
        let exported = native_board_json(reader, server, board).await;
        let exported_item = exported["checklistItems"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["_id"] == item)
            .unwrap();
        assert_eq!(exported_item["assigneeId"], member_id);
        assert_eq!(exported_item["linkedCardId"], linked);
        let exported_label = exported["labels"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["_id"] == label)
            .unwrap();
        assert_eq!(exported_label["dueAt"], "2030-01-02T03:04:05.000Z");
        assert_eq!(exported_label["sharedLabelId"], shared);
        let exported_member = exported["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["userId"] == member_id)
            .unwrap();
        assert_eq!(exported_member["grantedBy"], origins);
        assert_eq!(exported_member["customRoleId"], role);
    }
    let current = run(&member_app, server, &["user", "current"])
        .await
        .unwrap();
    let membership = current["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["board_id"] == board)
        .unwrap();
    assert_eq!(membership["granted_by"], origins);
    assert_eq!(membership["custom_role_id"], role);
    let admin_view = run(app, server, &["user", "get", &member_id])
        .await
        .unwrap();
    let membership = admin_view["boards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["board_id"] == board)
        .unwrap();
    assert_eq!(membership["granted_by"], origins);
    assert_eq!(membership["custom_role_id"], role);
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut session = ddp::Session::login(server, &token).await;
        session.call("sharedLabels.remove", json!([shared])).await;
        session.call("customBoardRoles.remove", json!([role])).await;
        session.close().await;
    })
    .await
    .expect("native nested metadata cleanup timed out");
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v12.57 stack and WEKAN_UPGRADE_E2E_URL"]
async fn native_v12_57_extensions_complete_lifecycle() {
    let server = env::var("WEKAN_UPGRADE_E2E_URL")
        .expect("set WEKAN_UPGRADE_E2E_URL to a fresh isolated v12.57 stack");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let directory = TestDirectory::new("v12-57-extensions");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Upgrade-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    run(
        &app,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &format!("upgrade_{nonce}"),
            "--password-stdin",
        ],
    )
    .await
    .unwrap();
    let current = run(&app, &server, &["user", "current"]).await.unwrap();
    assert_eq!(current["profile"]["calendar_system"], "gregorian");
    let user = run(
        &app,
        &server,
        &[
            "user",
            "create",
            "--username",
            &format!("upgrade_created_{nonce}"),
            "--password-stdin",
        ],
    )
    .await
    .unwrap();
    assert!(user["user_id"].as_str().is_some_and(|id| !id.is_empty()));

    let made = run(
        &app,
        &server,
        &["board", "create", "--title", "Upgrade extensions"],
    )
    .await
    .unwrap();
    let board = made["board_id"].as_str().unwrap();
    let swimlane = made["default_swimlane_id"].as_str().unwrap();
    let detail = run(&app, &server, &["board", "get", board]).await.unwrap();
    assert!(detail["key"].is_string());
    let list = run(
        &app,
        &server,
        &["list", "create", "--board", board, "--title", "Todo"],
    )
    .await
    .unwrap();
    let list = list["list_id"].as_str().unwrap();
    run(&app, &server, &["list", "get", list, "--board", board])
        .await
        .unwrap();
    let card = run(
        &app,
        &server,
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
            "New APIs",
        ],
    )
    .await
    .unwrap();
    let card = card["card_id"].as_str().unwrap();
    let checklist = run(
        &app,
        &server,
        &[
            "checklist",
            "create",
            "--board",
            board,
            "--card",
            card,
            "--title",
            "Before",
            "--item",
            "First",
        ],
    )
    .await
    .unwrap();
    let checklist = checklist["checklist_id"].as_str().unwrap();
    run(
        &app,
        &server,
        &[
            "checklist",
            "update",
            checklist,
            "--board",
            board,
            "--card",
            card,
            "--title",
            "After",
            "--due-at",
            "2030-01-02T03:04:05Z",
        ],
    )
    .await
    .unwrap();
    let detail = run(
        &app,
        &server,
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
    assert_eq!(detail["title"], "After");
    assert_eq!(detail["due_at"], "2030-01-02T03:04:05.000Z");
    run(
        &app,
        &server,
        &[
            "checklist",
            "update",
            checklist,
            "--board",
            board,
            "--card",
            card,
            "--clear-due-at",
        ],
    )
    .await
    .unwrap();
    assert!(
        run(
            &app,
            &server,
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
        .unwrap()["due_at"]
            .is_null()
    );
    let embedded = run(&app, &server, &["card", "get", card]).await.unwrap();
    assert_eq!(embedded["checklists"][0]["title"], "After");
    nested_metadata_lifecycle(&app, &server, board, card, checklist, nonce).await;
    let comment = run(
        &app,
        &server,
        &[
            "comment", "create", "--board", board, "--card", card, "--text", "Before",
        ],
    )
    .await
    .unwrap();
    let comment = comment["comment_id"].as_str().unwrap();
    run(
        &app,
        &server,
        &[
            "comment", "update", comment, "--board", board, "--card", card, "--text", " After ",
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        run(
            &app,
            &server,
            &["comment", "get", comment, "--board", board, "--card", card]
        )
        .await
        .unwrap()["text"],
        "After"
    );

    run(
        &app,
        &server,
        &["board", "card-settings", "get", "--board", board],
    )
    .await
    .unwrap();
    let settings = run(
        &app,
        &server,
        &[
            "board",
            "card-settings",
            "update",
            "--board",
            board,
            "--allows-swimlane-dragging",
            "false",
            "--allows-vote",
            "false",
        ],
    )
    .await
    .unwrap();
    assert_eq!(settings["settings"]["allows_swimlane_dragging"], false);
    assert_eq!(settings["settings"]["allows_vote"], false);
    let views = run(
        &app,
        &server,
        &["board", "view-settings", "get", "--board", board],
    )
    .await
    .unwrap();
    assert!(
        views["settings"]["keys"]
            .as_array()
            .is_some_and(|keys| !keys.is_empty())
    );
    run(
        &app,
        &server,
        &[
            "board",
            "view-settings",
            "update",
            "--board",
            board,
            "--default-private-view",
            "board-view-lists",
            "--view-order",
            "board-view-lists",
        ],
    )
    .await
    .unwrap();
    let views = run(
        &app,
        &server,
        &["board", "view-settings", "get", "--board", board],
    )
    .await
    .unwrap();
    assert_eq!(
        views["settings"]["default_private_board_view"],
        "board-view-lists"
    );
    run(
        &app,
        &server,
        &["board", "card-field-order", "get", "--board", board],
    )
    .await
    .unwrap();
    let order = run(
        &app,
        &server,
        &[
            "board",
            "card-field-order",
            "update",
            "--board",
            board,
            "--field",
            "description",
            "--field",
            "description",
            "--field",
            "labels",
        ],
    )
    .await
    .unwrap();
    assert_eq!(order["settings"]["card_field_order"][0], "description");
    let persisted = run(
        &app,
        &server,
        &["board", "card-field-order", "get", "--board", board],
    )
    .await
    .unwrap();
    assert_eq!(persisted["settings"], order["settings"]);
    let order = run(
        &app,
        &server,
        &[
            "board",
            "card-field-order",
            "update",
            "--board",
            board,
            "--reset",
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        order["settings"]["card_field_order"]
            .as_array()
            .unwrap()
            .len(),
        14
    );
    let persisted = run(
        &app,
        &server,
        &["board", "card-field-order", "get", "--board", board],
    )
    .await
    .unwrap();
    assert_eq!(persisted["settings"], order["settings"]);
    assert_eq!(persisted["settings"]["card_field_order"][0], "labels");

    admin::verify(&app, &server, nonce).await;
    metadata::verify(&app, &server, board, list, swimlane, card, nonce).await;

    let file = directory.path().join("attachment.txt");
    fs::write(&file, b"Wekan v12.57 attachment round trip").unwrap();
    let uploaded = run(
        &app,
        &server,
        &[
            "attachment",
            "upload",
            file.to_str().unwrap(),
            "--board",
            board,
            "--list",
            list,
            "--swimlane-id",
            swimlane,
            "--card",
            card,
            "--file-type",
            "text/plain",
            "--transport",
            "raw",
        ],
    )
    .await
    .unwrap();
    let attachment = uploaded["attachment_id"].as_str().unwrap();
    let deletion = run(
        &app,
        &server,
        &[
            "attachment",
            "delete",
            attachment,
            "--board",
            board,
            "--yes",
        ],
    )
    .await
    .unwrap();
    assert_eq!(deletion["deleted"], true);
    let deleted = run(
        &app,
        &server,
        &["attachment", "list-deleted", "--board", board],
    )
    .await
    .unwrap();
    assert!(
        deleted["attachments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["attachment_id"] == attachment)
    );
    assert_eq!(
        run(
            &app,
            &server,
            &["attachment", "restore", attachment, "--board", board]
        )
        .await
        .unwrap()["restored"],
        true
    );
    assert_eq!(
        run(
            &app,
            &server,
            &["attachment", "restore", attachment, "--board", board]
        )
        .await
        .unwrap()["restored"],
        false
    );
    let destination = directory.path().join("restored.txt");
    run(
        &app,
        &server,
        &[
            "attachment",
            "download",
            attachment,
            "--destination",
            destination.to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(fs::read(destination).unwrap(), fs::read(file).unwrap());
    run(
        &app,
        &server,
        &["attachment", "delete", attachment, "--yes"],
    )
    .await
    .unwrap();
    assert_eq!(
        run(
            &app,
            &server,
            &["attachment", "restore", attachment, "--board", board]
        )
        .await
        .unwrap()["restored"],
        true
    );

    commands::verify(&app, &server, board, swimlane, nonce).await;
    run(&app, &server, &["board", "delete", board, "--yes"])
        .await
        .unwrap();
    let webdav_password = "wekan-webdav-secret-sentinel";
    let password_file = directory.path().join("webdav-password.txt");
    fs::write(&password_file, webdav_password).unwrap();
    let settings = run(
        &app,
        &server,
        &[
            "admin",
            "attachment-settings",
            "update",
            "--webdav-enabled",
            "false",
            "--webdav-read",
            "false",
            "--webdav-write",
            "false",
            "--webdav-endpoint",
            "https://webdav.example.test",
            "--webdav-username",
            "fixture",
            "--webdav-bucket",
            "attachments",
            "--webdav-password-file",
            password_file.to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        settings["settings"]["storage_config"]["webdav"]["password_set"],
        true
    );
    assert!(!settings.to_string().contains(webdav_password));
    let current = run(&app, &server, &["user", "current"]).await.unwrap();
    let current_id = current["user_id"].as_str().unwrap();
    // Matching stored shapes from passkeys.js, sqrl.js, webPush.js, ntfy.js and
    // trayDelivery.js. Secrets must become presence flags at the decode boundary.
    let metadata = serde_json::json!({
        "passkeys":[{"id":"credential","publicKey":"cHVibGljLWtleQ","counter":0,"transports":["internal"],"name":"Fixture","createdAt":"2026-10-20T08:00:00Z"}],
        "sqrl":{"idk":"public-idk","suk":"public-suk","vuk":"public-vuk","disabled":false,"addedAt":"2026-10-20T08:00:00Z","sqrlOnly":false,"hardlock":false},
        "webPushSubscriptions":[{"endpoint":"https://fcm.googleapis.com/fcm/send/private-subscription-sentinel","keys":{"p256dh":"public-key","auth":"push-auth-secret-sentinel"},"createdAt":"2026-10-20T08:00:00Z"}],
        "ntfy":{"enabled":false,"url":"https://ntfy.example.test/private-topic-sentinel","token":"ntfy-secret-sentinel"},
        "notificationDeliveryPending":{"_id":"receipt","version":1,"userId":current_id,"activityId":"activity"}
    });
    super::support::attachment_fixtures::fixture_command(
        "mongodb",
        "mongosh",
        "p.metadata.notificationDeliveryPending._id=require('crypto').createHash('sha256').update(JSON.stringify([p.id,'activity'])).digest('hex');db.getSiblingDB('wekan').users.updateOne({_id:p.id},{$set:p.metadata});print('{}');",
        &serde_json::json!({"id":current_id,"metadata":metadata}),
    );
    for args in [
        vec!["user", "current"],
        vec!["user", "get", current_id],
        vec!["user", "update", current_id],
    ] {
        let output = run(&app, &server, &args).await.unwrap();
        assert_eq!(output["ntfy"]["token_present"], true);
        assert_eq!(
            output["web_push_subscriptions"][0]["keys"]["auth_present"],
            true
        );
        assert_eq!(output["passkeys"][0]["last_used_at"], Value::Null);
        for secret in [
            "private-subscription-sentinel",
            "push-auth-secret-sentinel",
            "private-topic-sentinel",
            "ntfy-secret-sentinel",
        ] {
            assert!(!output.to_string().contains(secret));
        }
    }
    super::support::attachment_fixtures::fixture_command(
        "mongodb",
        "mongosh",
        "db.getSiblingDB('wekan').users.updateOne({_id:p.id},{$unset:{passkeys:'',sqrl:'',webPushSubscriptions:'',ntfy:'',notificationDeliveryPending:''}});print('{}');",
        &serde_json::json!({"id":current_id}),
    );
    run(
        &app,
        &server,
        &["user", "delete", user["user_id"].as_str().unwrap(), "--yes"],
    )
    .await
    .unwrap();
}
