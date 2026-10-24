//! Native tray delivery must remain readable by every user-record command.
use super::super::support::{
    attachment_fixtures::fixture_command,
    live::{LiveApp, run},
    *,
};
use serde_json::{Value, json};

pub(super) async fn verify(admin: &LiveApp, server: &str, admin_token: &str, nonce: u128) {
    let board = run(
        admin,
        server,
        &["board", "create", "--title", "Tray timestamps"],
    )
    .await
    .unwrap();
    let board_id = board["board_id"].as_str().unwrap();
    let lane = board["default_swimlane_id"].as_str().unwrap();
    let list = run(
        admin,
        server,
        &["list", "create", "--board", board_id, "--title", "Inbox"],
    )
    .await
    .unwrap();
    let list_id = list["list_id"].as_str().unwrap();
    let canonical = ServerUrl::parse(server).unwrap().as_str().to_owned();
    let (_directory, member, member_id) = register_live_board_role(
        server,
        &canonical,
        admin_token,
        board_id,
        nonce + 717,
        "normal",
    )
    .await;
    // These settings and watcher records are the UI's persisted inputs, not
    // notification fixtures. Card creation below invokes the actual producer:
    // v12.57 server/notifications/trayQueue.js and server/lib/trayDelivery.js.
    fixture_command(
        "mongodb",
        "mongosh",
        r#"
        const w=db.getSiblingDB('wekan');
        w.boards.updateOne({_id:p.board},{$set:{watchers:[{userId:p.user,level:'watching'}]}});
        w.users.updateOne({_id:p.user},{$set:{'profile.notifyOverrideTray':true,'profile.notifications':[]}});
        print('{}');
    "#,
        &json!({"board":board_id,"user":member_id}),
    );
    assert_eq!(
        run(&member, server, &["user", "current"]).await.unwrap()["is_admin"],
        false
    );

    for (schedule, settings) in [
        ("immediate", json!({"schedule":"immediate"})),
        (
            "interval",
            json!({"schedule":"interval","intervalMinutes":60}),
        ),
    ] {
        fixture_command(
            "mongodb",
            "mongosh",
            r#"
            db.getSiblingDB('wekan').users.updateOne({_id:p.user},
                {$set:{'profile.notificationDelivery':{tray:p.settings}}});
            print('{}');
        "#,
            &json!({"user":member_id,"settings":settings}),
        );
        let made = run(
            admin,
            server,
            &[
                "card",
                "create",
                "--board",
                board_id,
                "--list",
                list_id,
                "--swimlane-id",
                lane,
                "--title",
                schedule,
            ],
        )
        .await
        .unwrap();
        let card_id = made["card_id"].as_str().unwrap();
        let notification = wait_for_notification(board_id, card_id, &member_id).await;
        let expected = notification.get("showAt").cloned().unwrap_or(Value::Null);
        if schedule == "immediate" {
            assert!(notification.get("showAt").is_none());
        } else {
            assert!(
                expected.is_number(),
                "scheduled native delivery must store milliseconds"
            );
        }

        for (reader, args) in [
            (&member, vec!["user", "current"]),
            (admin, vec!["user", "get", &member_id]),
            (admin, vec!["user", "update", &member_id]),
            (admin, vec!["user", "enable-login", &member_id]),
        ] {
            let result = run(reader, server, &args).await.unwrap();
            let user = if args[1] == "enable-login" {
                &result["user"]
            } else {
                &result
            };
            let shown = user["profile"]["notifications"]
                .as_array()
                .unwrap()
                .iter()
                .find(|entry| entry["activity"] == notification["activity"])
                .expect("the persisted notification must survive command projection");
            assert_eq!(shown["show_at"], expected, "{schedule}: {args:?}");
            assert!(shown.get("showAt").is_none());
        }
        assert_eq!(
            run(&member, server, &["auth", "status"]).await.unwrap()["authenticated"],
            true
        );
    }
    run(admin, server, &["board", "delete", board_id, "--yes"])
        .await
        .unwrap();
    run(admin, server, &["user", "delete", &member_id, "--yes"])
        .await
        .unwrap();
    wait_for_live_user_absent(admin, server, &member_id).await;
    run(
        &member,
        server,
        &["auth", "logout", "--local-only", "--yes"],
    )
    .await
    .unwrap();
}

async fn wait_for_notification(board: &str, card: &str, user: &str) -> Value {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    loop {
        let saved = fixture_command(
            "mongodb",
            "mongosh",
            r#"
            const w=db.getSiblingDB('wekan');
            const activity=w.activities.findOne({boardId:p.board,cardId:p.card,activityType:'createCard'});
            const user=w.users.findOne({_id:p.user});
            const entry=activity && (user.profile.notifications||[]).find(n=>n.activity===activity._id);
            print(JSON.stringify(entry||null));
        "#,
            &json!({"board":board,"card":card,"user":user}),
        );
        if saved.is_object() {
            return saved;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "native tray notification did not persist within 20 seconds"
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}
