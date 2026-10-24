use crate::support::{
    attachment_fixtures::fixture_command,
    live::{LiveApp, run},
    *,
};
use serde_json::json;

pub(super) async fn verify(
    app: &LiveApp,
    server: &str,
    board: &str,
    list: &str,
    swimlane: &str,
    card: &str,
    nonce: u128,
) {
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
    let (_reader_dir, reader, _) =
        register_live_board_role(server, &canonical, &token, board, nonce + 880, "normal").await;
    let board_get = ["board", "get", board];
    let card_get = ["card", "get", card];
    let list_get = ["list", "get", list, "--board", board];
    let before = run(app, server, &board_get).await.unwrap();
    for field in [
        "announcement",
        "wip_limit_groups",
        "column_sets",
        "notification_delivery",
    ] {
        assert!(
            before[field].is_null(),
            "{field} must be absent before its native producer runs"
        );
    }
    let before = run(app, server, &card_get).await.unwrap();
    for field in ["drawings", "custom_field_links"] {
        assert!(before[field].is_null(), "{field}");
    }
    let before = run(app, server, &list_get).await.unwrap();
    assert!(before["role_exceptions"].is_null());
    assert!(before["sync_source"].is_null());
    let other_list = run(
        app,
        server,
        &[
            "list",
            "create",
            "--board",
            board,
            "--title",
            "WIP metadata partner",
        ],
    )
    .await
    .unwrap();
    let other_list = other_list["list_id"].as_str().unwrap();
    let target = run(
        app,
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
            "Linked fields metadata target",
        ],
    )
    .await
    .unwrap();
    let target = target["card_id"].as_str().unwrap();
    let user_id = run(app, server, &["user", "current"]).await.unwrap()["user_id"]
        .as_str()
        .unwrap()
        .to_owned();
    tokio::time::timeout(std::time::Duration::from_secs(45), async {
        let mut session = ddp::Session::login(server, &token).await;
        // These features save collection updates from their UI model helpers:
        // models/boards.js setAnnouncement/addWipLimitGroup,
        // client/components/boards/columnSetsView.js and cards/cardDrawing.js.
        // Use their exact stored forms, then exercise the server-side producers.
        session.call("/boards/update", json!([{"_id":board},{"$set":{
            "announcement":{"enabled":true,"body":"Metadata announcement","updatedAt":{"$date":1893553445000_i64}},
            "columnSets":[{"_id":"metadataSet","name":"Metadata set","columns":[{"_id":"metadataColumn","title":"Ready","color":"green"}]}],
            "wipLimitGroups":[{"_id":"metaWip","name":"Metadata WIP","listIds":[list,other_list],"limit":4,"enabled":true}]
        }}])).await;
        session.call("updateWipLimitGroup", json!([board,"metaWip",{"limit":5,"name":"Updated metadata WIP"}])).await;
        session.call("/cards/update", json!([{"_id":card},{"$set":{
            "drawings":[{"_id":"metadataDrawing","width":800,"height":500,"strokes":[],"updatedAt":{"$date":1893553445000_i64}}],
            "columnSetValues":{"metadataSet":"metadataColumn"}
        }}])).await;
        session.call("cards.whiteboardAddStroke", json!([card,"metadataDrawing",{
            "_id":"metadataStroke","color":"#000000","width":2,"points":[[10,20],[30,40]]
        }])).await;
        session.call("linkCardCustomFields", json!([card,target,"both"])).await;
        session.call("setNotificationDelivery", json!(["board",board,"email",{
            "layout":"clear","parts":["board","card"],"grouping":"board","schedule":"daily",
            "dailyTime":"09:30","intervalMinutes":60,"quietStart":"22:00","quietEnd":"07:00","timezone":"Africa/Nairobi"
        }])).await;
        session.call("setNotificationDelivery", json!(["board",board,"webhook",{
            "text":false,"fields":["standard"],"grouping":"none","schedule":"immediate"
        }])).await;
        session.call("lists.setRoleExceptions", json!([list,{
            "normal":{"write":true,"comment":true},"read-only":{"write":false,"comment":false}
        }])).await;
        // Configure a real source without starting an external sync or storing credentials.
        session.call("setListSyncSource", json!([list,{
            "type":"github","projectKey":"wekan/wekan","enabled":false,
            "createCards":false,"archiveCards":false,"fields":["title","description"]
        }])).await;
        session.close().await;
    }).await.expect("native populated metadata producers timed out");
    let shown = run(app, server, &board_get).await.unwrap();
    assert_eq!(shown["announcement"]["enabled"], true);
    assert_eq!(shown["announcement"]["body"], "Metadata announcement");
    assert!(shown["announcement"]["updated_at"].is_string());
    assert_eq!(shown["column_sets"][0]["columns"][0]["title"], "Ready");
    assert_eq!(shown["wip_limit_groups"][0]["limit"], 5);
    assert_eq!(
        shown["notification_delivery"]["email"]["quiet_start"],
        "22:00"
    );
    assert_eq!(shown["notification_delivery"]["webhook"]["text"], false);
    let shown = run(app, server, &card_get).await.unwrap();
    assert_eq!(shown["column_set_values"]["metadataSet"], "metadataColumn");
    assert_eq!(shown["drawings"][0]["strokes"][0]["id"], "metadataStroke");
    assert_eq!(shown["drawings"][0]["strokes"][0]["user_id"], user_id);
    assert_eq!(
        shown["drawings"][0]["strokes"][0]["points"],
        json!([[10, 20], [30, 40]])
    );
    assert!(shown["drawings"][0]["png_attachment_id"].is_null());
    assert_eq!(shown["custom_field_links"][0]["card_id"], target);
    assert_eq!(shown["custom_field_links"][0]["mode"], "both");
    assert_eq!(shown["custom_field_links"][0]["user_id"], user_id);
    assert!(shown["custom_field_links"][0]["created_at"].is_string());
    let shown = run(app, server, &list_get).await.unwrap();
    assert_eq!(shown["role_exceptions"]["normal"]["write"], true);
    assert_eq!(shown["role_exceptions"]["read-only"]["comment"], false);
    assert_eq!(shown["sync_source"]["type"], "github");
    assert_eq!(shown["sync_source"]["project_key"], "wekan/wekan");
    assert_eq!(shown["sync_source"]["enabled"], false);
    assert_eq!(
        shown["sync_source"]["fields"],
        json!(["title", "description"])
    );
    assert!(shown["sync_source"]["last_synced_at"].is_null());
    // The ordinary member must traverse the same populated strict DTOs.
    for args in [
        board_get.as_slice(),
        card_get.as_slice(),
        list_get.as_slice(),
    ] {
        run(&reader, server, args).await.unwrap();
    }

    // Inject only deliberately invalid extra properties, then let the production
    // read reach its normal strict decoder. Do not strip fields in the live fixture.
    for (collection, id, field, args) in [
        (
            "boards",
            board,
            "announcement.unmappedAuditField",
            board_get.as_slice(),
        ),
        (
            "boards",
            board,
            "columnSets.0.unmappedAuditField",
            board_get.as_slice(),
        ),
        (
            "boards",
            board,
            "columnSets.0.columns.0.unmappedAuditField",
            board_get.as_slice(),
        ),
        (
            "boards",
            board,
            "wipLimitGroups.0.unmappedAuditField",
            board_get.as_slice(),
        ),
        (
            "boards",
            board,
            "notificationDelivery.email.unmappedAuditField",
            board_get.as_slice(),
        ),
        (
            "boards",
            board,
            "notificationDelivery.webhook.unmappedAuditField",
            board_get.as_slice(),
        ),
        (
            "cards",
            card,
            "drawings.0.unmappedAuditField",
            card_get.as_slice(),
        ),
        (
            "cards",
            card,
            "drawings.0.strokes.0.unmappedAuditField",
            card_get.as_slice(),
        ),
        (
            "cards",
            card,
            "customFieldLinks.0.unmappedAuditField",
            card_get.as_slice(),
        ),
        (
            "lists",
            list,
            "roleExceptions.normal.unmappedAuditField",
            list_get.as_slice(),
        ),
        (
            "lists",
            list,
            "syncSource.unmappedAuditField",
            list_get.as_slice(),
        ),
    ] {
        let input = json!({"collection":collection,"id":id,"field":field});
        fixture_command(
            "mongodb",
            "mongosh",
            "db.getSiblingDB('wekan')[p.collection].updateOne({_id:p.id},{$set:{[p.field]:'unmapped-value-sentinel'}});print('{}');",
            &input,
        );
        let result = run(app, server, args).await;
        fixture_command(
            "mongodb",
            "mongosh",
            "db.getSiblingDB('wekan')[p.collection].updateOne({_id:p.id},{$unset:{[p.field]:''}});print('{}');",
            &input,
        );
        let error = result.unwrap_err();
        assert_eq!(error.code(), ErrorCode::ProtocolError, "{field}: {error:?}");
        assert!(!format!("{error:?}").contains("unmapped-value-sentinel"));
    }
    // Native disconnect/unlink operations also exercise their omission/empty forms.
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut session = ddp::Session::login(server, &token).await;
        session.call("setListSyncSource", json!([list, null])).await;
        session
            .call("lists.setRoleExceptions", json!([list, {}]))
            .await;
        session
            .call("unlinkCardCustomFields", json!([card, target]))
            .await;
        session
            .call(
                "setNotificationDelivery",
                json!(["board", board, "email", null]),
            )
            .await;
        session
            .call(
                "setNotificationDelivery",
                json!(["board", board, "webhook", null]),
            )
            .await;
        session.close().await;
    })
    .await
    .expect("native metadata cleanup timed out");
    let shown = run(app, server, &list_get).await.unwrap();
    assert!(shown["sync_source"].is_null());
    assert!(shown["role_exceptions"].is_null());
    assert_eq!(
        run(app, server, &card_get).await.unwrap()["custom_field_links"],
        json!([])
    );
}
