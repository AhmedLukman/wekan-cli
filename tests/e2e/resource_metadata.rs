use super::support::{attachment_fixtures::fixture_command, live::run, *};
use serde_json::{Value, json};

#[tokio::test]
#[ignore = "requires an isolated Wekan v13.28 stack and WEKAN_METADATA_E2E_URL"]
async fn native_resource_fields_and_parent_creation() {
    let server = env::var("WEKAN_METADATA_E2E_URL").unwrap();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = TestDirectory::new("resource-metadata");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Metadata-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let user = run(
        &app,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &format!("metadata_{nonce}"),
            "--password-stdin",
        ],
    )
    .await
    .unwrap();
    let board = run(
        &app,
        &server,
        &["board", "create", "--title", "Board fields"],
    )
    .await
    .unwrap();
    let board_id = board["board_id"].as_str().unwrap();
    let lane = board["default_swimlane_id"].as_str().unwrap();
    let list = run(
        &app,
        &server,
        &["list", "create", "--board", board_id, "--title", "Todo"],
    )
    .await
    .unwrap();
    let list_id = list["list_id"].as_str().unwrap();
    let create = [
        "card",
        "create",
        "--board",
        board_id,
        "--list",
        list_id,
        "--swimlane-id",
        lane,
    ];
    let parent = run(
        &app,
        &server,
        &[&create[..], &["--title", "Parent"]].concat(),
    )
    .await
    .unwrap();
    let parent_id = parent["card_id"].as_str().unwrap();
    let child = run(
        &app,
        &server,
        &[&create[..], &["--title", "Child", "--parent-id", parent_id]].concat(),
    )
    .await
    .unwrap();
    let child_id = child["card_id"].as_str().unwrap();
    let get = [
        "card", "get", child_id, "--board", board_id, "--list", list_id,
    ];
    let before = run(&app, &server, &get).await.unwrap();
    assert_eq!(before["parent_id"], parent_id);
    assert!(before["flow"].is_null());
    let updated = run(
        &app,
        &server,
        &[
            "card",
            "update",
            child_id,
            "--board",
            board_id,
            "--list",
            list_id,
            "--sort",
            "0",
            "--description",
            "",
            "--is-over-time",
            "true",
        ],
    )
    .await
    .unwrap();
    assert_eq!(updated["card_id"], child_id);
    let after = run(&app, &server, &get).await.unwrap();
    assert_eq!(after["sort"], 0);
    assert_eq!(after["description"], "");
    assert_eq!(after["is_overtime"], true);

    // UI-owned optional metadata uses the same stored shapes as the tagged model writers.
    fixture_command(
        "mongodb",
        "mongosh",
        r#"
        const w = db.getSiblingDB('wekan');
        w.boards.updateOne({_id:p.board}, {$set:{
            commitmentListId:p.list, personWipLimit:3, allowsListPolicies:true,
            cardSettingsSectionOrder:['labels'], cardSettingsHiddenOnCard:['date'],
            shopSettings:{enabled:false,title:'Test shop',intro:'',currency:'EUR',priceFieldId:'',
                skuFieldId:'',stockFieldId:'',saleListIds:[],categoryLabelIds:[],ordersBoardId:'',
                ordersListId:'',guestCheckout:false,termsUrl:'',vatPercent:0,pricesIncludeVat:true,
                referenceType:'none',decrementStock:false,confirmationEmail:false,
                paymentMethods:[{id:'cash',enabled:true,method:{type:'cash',details:'On pickup'}}],
                updatedBy:p.user,updatedAt:new Date()}
        }});
        w.lists.updateOne({_id:p.list}, {$set:{policy:'Review before moving', split:true}});
        w.cards.updateOne({_id:p.card}, {$set:{
            flow:{blocked:true,blockedReason:'Waiting',blockedBy:p.user,blockedAt:new Date(),
                  classOfService:'expedite',moscow:'must',wsjf:{businessValue:8},splitDoneListId:p.list},
            estimatedHours:2.5,billable:false, shownFields:['descriptionText'],hiddenFields:['stickers']
        }});
        w.users.updateOne({_id:p.user}, {$set:{
            'profile.cardSettingsSplit':45,'profile.cardDetailsWidth':65,
            'profile.drawingColors':['#112233'],'profile.keepAddCardFormOpen':true
        }});
        print('{}');
    "#,
        &json!({"board":board_id,"list":list_id,"card":child_id,"user":user["user_id"]}),
    );
    let card = run(&app, &server, &get).await.unwrap();
    assert_eq!(card["flow"]["wsjf"]["business_value"], 8);
    assert_eq!(card["flow"]["split_done_list_id"], list_id);
    assert_eq!(card["estimated_hours"], 2.5);
    assert_eq!(card["billable"], false);
    assert_eq!(card["shown_fields"], json!(["descriptionText"]));
    assert_eq!(card["hidden_fields"], json!(["stickers"]));
    let board = run(&app, &server, &["board", "get", board_id])
        .await
        .unwrap();
    assert_eq!(
        board["shop_settings"]["payment_methods"][0]["method"]["details"],
        "On pickup"
    );
    assert_eq!(board["shop_settings"]["updated_by"], user["user_id"]);
    assert!(board["shop_settings"]["updated_at"].is_string());
    let copied = run(&app, &server, &["board", "copy", board_id])
        .await
        .unwrap();
    let copied_id = copied["board_id"].as_str().unwrap();
    let copied = run(&app, &server, &["board", "get", copied_id])
        .await
        .unwrap();
    assert_eq!(copied["shop_settings"]["title"], "Test shop");
    assert!(copied["shop_settings"]["updated_by"].is_null());
    assert!(copied["shop_settings"]["updated_at"].is_null());
    let destination = directory.path().join("copied-board.json");
    run(
        &app,
        &server,
        &[
            "board",
            "export",
            copied_id,
            "--format",
            "json",
            "--destination",
            destination.to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    let exported: Value = serde_json::from_slice(&std::fs::read(destination).unwrap()).unwrap();
    assert_eq!(exported["shopSettings"]["title"], "Test shop");
    assert!(exported["shopSettings"].get("updatedBy").is_none());
    assert!(exported["shopSettings"].get("updatedAt").is_none());
    assert_eq!(board["person_wip_limit"], 3);
    assert_eq!(board["commitment_list_id"], list_id);
    let list = run(
        &app,
        &server,
        &["list", "get", list_id, "--board", board_id],
    )
    .await
    .unwrap();
    assert_eq!(list["policy"], "Review before moving");
    assert_eq!(list["split"], true);
    let me = run(&app, &server, &["user", "current"]).await.unwrap();
    assert_eq!(me["profile"]["drawing_colors"], json!(["#112233"]));
    for (option, id) in [("--list", list_id), ("--swimlane", lane)] {
        let batch = run(
            &app,
            &server,
            &["card", "list", "--board", board_id, option, id],
        )
        .await
        .unwrap();
        let listed = batch["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["card_id"] == child_id)
            .unwrap();
        for key in [
            "board_id",
            "modified_at",
            "date_last_activity",
            "label_ids",
            "members",
            "custom_fields",
        ] {
            assert_eq!(listed[key], card[key], "{option} {key}");
        }
    }
    let settings = run(
        &app,
        &server,
        &[
            "board",
            "card-settings",
            "update",
            "--board",
            board_id,
            "--allows-blocked-on-card",
            "true",
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        settings["settings"]["allows_blocked_on_card"],
        Value::Bool(true)
    );
    let settings = run(
        &app,
        &server,
        &[
            "admin",
            "settings",
            "update",
            "--login-background-color",
            "#314159",
        ],
    )
    .await
    .unwrap();
    assert_eq!(settings["settings"]["login_background_color"], "#314159");
    run(
        &app,
        &server,
        &[
            "card", "delete", child_id, "--board", board_id, "--list", list_id, "--yes",
        ],
    )
    .await
    .unwrap();
    let missing = run(&app, &server, &get).await.unwrap_err();
    assert_eq!(missing.details().http_status, Some(404));
}
