use super::support::attachment_fixtures::fixture_command;
use super::support::{
    live::{LiveApp, run},
    *,
};
use serde_json::{Value, json};
fn mongo(script: &str, input: Value) -> Value {
    fixture_command("mongodb", "mongosh", script, &input)
}

async fn verify_bot_report_nulls(
    app: &LiveApp,
    server: &str,
    board: &str,
    card: &str,
    run_id: &str,
    report: &Path,
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
    let http = reqwest::Client::new();
    let cli_run = format!("{run_id}-null-cli");
    let native_run = format!("{run_id}-null-native");
    let args = [
        "card",
        "bot-run",
        "report",
        &cli_run,
        "--board",
        board,
        "--card",
        card,
        "--file",
        report.to_str().unwrap(),
    ];
    for input in [
        json!({}),
        json!({"text":null,"tool":null,"question":null,"tokens":null,"costUsd":null}),
        json!({"options":[{"value":"yes"},{"value":"no","label":null},{"value":"later","label":"Later"}]}),
    ] {
        mongo(
            "const w=db.getSiblingDB('wekan');for(const id of [p.cli,p.native])w.bot_runs.replaceOne({_id:id},{_id:id,boardId:p.board,cardId:p.card,status:'awaiting_input',createdAt:new Date(),updatedAt:new Date(),question:'Original question',options:[{value:'original',label:'Original label'}],tokens:17,costUsd:0.75,events:[{text:'Initial',tool:'fixture',status:'awaiting_input',at:new Date()}]},{upsert:true});print('{}');",
            json!({"cli":cli_run,"native":native_run,"board":board,"card":card}),
        );
        fs::write(report, serde_json::to_vec(&input).unwrap()).unwrap();
        run(app, server, &args).await.unwrap();
        let response = http
            .post(format!(
                "{canonical}api/boards/{board}/cards/{card}/bot-runs/{native_run}"
            ))
            .bearer_auth(&token)
            .json(&input)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200, "{input}");
        // Compare persisted fields, excluding clocks and the distinct fixture IDs.
        // This observes native null coercion without reproducing it in the CLI.
        let state = mongo(
            "const w=db.getSiblingDB('wekan');function state(id){const r=w.bot_runs.findOne({_id:id});return {status:r.status,question:r.question,options:r.options,tokens:r.tokens,costUsd:r.costUsd,events:r.events.map(({at,...event})=>event)};}print(JSON.stringify({cli:state(p.cli),native:state(p.native)}));",
            json!({"cli":cli_run,"native":native_run}),
        );
        assert_eq!(state["cli"], state["native"], "{input}");
        if input.get("tokens").is_some() {
            assert_eq!(state["cli"]["tokens"], 0);
            assert_eq!(state["cli"]["costUsd"], 0);
            assert_eq!(state["cli"]["events"].as_array().unwrap().len(), 2);
        } else if input.get("options").is_some() {
            assert_eq!(state["cli"]["options"][0]["label"], "yes");
            assert!(state["cli"]["options"][1]["label"].is_null());
            assert_eq!(state["cli"]["options"][2]["label"], "Later");
        } else {
            assert_eq!(state["cli"]["tokens"], 17);
            assert_eq!(state["cli"]["costUsd"], 0.75);
            assert_eq!(state["cli"]["question"], "Original question");
            assert_eq!(state["cli"]["events"].as_array().unwrap().len(), 1);
        }
    }
    for input in [json!({"status":null}), json!({"options":null})] {
        fs::write(report, serde_json::to_vec(&input).unwrap()).unwrap();
        let error = run(app, server, &args).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidInput);
        assert_eq!(error.details().http_status, None);
        let response = http
            .post(format!(
                "{canonical}api/boards/{board}/cards/{card}/bot-runs/{native_run}"
            ))
            .bearer_auth(&token)
            .json(&input)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 400, "{input}");
    }
    mongo(
        "db.getSiblingDB('wekan').bot_runs.deleteMany({_id:{$in:[p.cli,p.native]}});print('{}');",
        json!({"cli":cli_run,"native":native_run}),
    );
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated v12.57 stack and WEKAN_NATIVE_HOOK_E2E_URL"]
async fn native_bot_and_inbound_email_routes_match_wekan_v12_57() {
    let server = env::var("WEKAN_NATIVE_HOOK_E2E_URL")
        .expect("set WEKAN_NATIVE_HOOK_E2E_URL to a fresh isolated stack");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let directory = TestDirectory::new("native-hooks");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Native-hooks-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let user = run(
        &app,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &format!("hooks_{nonce}"),
            "--password-stdin",
        ],
    )
    .await
    .unwrap();
    let current = run(&app, &server, &["user", "current"]).await.unwrap();
    let user_id = current["user_id"].as_str().unwrap();
    let board = run(
        &app,
        &server,
        &["board", "create", "--title", "Native hooks"],
    )
    .await
    .unwrap();
    let board_id = board["board_id"].as_str().unwrap();
    let swimlane = board["default_swimlane_id"].as_str().unwrap();
    let list = run(
        &app,
        &server,
        &["list", "create", "--board", board_id, "--title", "Inbox"],
    )
    .await
    .unwrap();
    let list_id = list["list_id"].as_str().unwrap();
    let card = run(
        &app,
        &server,
        &[
            "card",
            "create",
            "--board",
            board_id,
            "--list",
            list_id,
            "--swimlane-id",
            swimlane,
            "--title",
            "Bot task",
        ],
    )
    .await
    .unwrap();
    let card_id = card["card_id"].as_str().unwrap();
    let run_id = format!("run{nonce}");
    let address_token = "IsolatedBoardMail12345";
    assert_eq!(address_token.len(), 22);
    let bounce_secret = format!("isolated-bounce-{nonce}");
    // Fixture the records normally written by the Bots and Notification Settings UI.
    mongo(
        "const w=db.getSiblingDB('wekan');w.boards.updateOne({_id:p.board},{$set:{emailToCard:{enabled:true,listId:p.list,swimlaneId:p.swimlane}}});w.settings.updateOne({},{$set:{inboundMail:{enabled:true,domain:'example.invalid'},emailBounces:{enabled:true,providers:{postmark:{enabled:true}},providerSecrets:{postmark:p.secret}}}});w.board_email_addresses.insertOne({_id:p.run,boardId:p.board,userId:p.user,token:p.address,createdAt:new Date()});w.bot_runs.insertOne({_id:p.run,boardId:p.board,cardId:p.card,status:'queued',createdAt:new Date(),updatedAt:new Date(),events:[]});print(JSON.stringify({ok:true}));",
        json!({"board":board_id,"list":list_id,"swimlane":swimlane,"card":card_id,"run":run_id,"user":user_id,"secret":bounce_secret,"address":address_token}),
    );
    let report = directory.path().join("report.json");
    verify_bot_report_nulls(&app, &server, board_id, card_id, &run_id, &report).await;
    for status in ["running", "awaiting_input", "done"] {
        fs::write(&report,serde_json::to_vec(&json!({"status":status,"text":"Native report","tool":"fixture","question":"Continue?","options":[{"value":"yes","label":"Yes"}],"tokens":12,"costUsd":0.25})).unwrap()).unwrap();
        let result = run(
            &app,
            &server,
            &[
                "card",
                "bot-run",
                "report",
                &run_id,
                "--board",
                board_id,
                "--card",
                card_id,
                "--file",
                report.to_str().unwrap(),
            ],
        )
        .await
        .unwrap();
        assert_eq!(result["run_id"], run_id);
        assert_eq!(result["status"], status);
    }
    let stored = mongo(
        "print(JSON.stringify(db.getSiblingDB('wekan').bot_runs.findOne({_id:p.run})));",
        json!({"run":run_id}),
    );
    assert_eq!(stored["status"], "done");
    assert_eq!(stored["reportedBy"], user_id);
    assert_eq!(stored["events"].as_array().unwrap().len(), 3);
    fs::write(&report, b"{\"status\":\"running\"}").unwrap();
    let error = run(
        &app,
        &server,
        &[
            "card",
            "bot-run",
            "report",
            &run_id,
            "--board",
            board_id,
            "--card",
            card_id,
            "--file",
            report.to_str().unwrap(),
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(error.details().http_status, Some(400));
    let address = format!("board+{address_token}@example.invalid");
    let email = directory.path().join("email.json");
    fs::write(&email,serde_json::to_vec(&json!({"From":"Sender <sender@example.invalid>","To":address,"Subject":"Inbound card","TextBody":"Original message bytes","MessageID":format!("<json-{nonce}@example.invalid>")})).unwrap()).unwrap();
    let receipt = run(
        &app,
        &server,
        &[
            "integration",
            "inbound-email",
            "submit",
            "--file",
            email.to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    let received_id = receipt["resource_id"].as_str().unwrap();
    let received = run(
        &app,
        &server,
        &[
            "card",
            "get",
            received_id,
            "--board",
            board_id,
            "--list",
            list_id,
        ],
    )
    .await
    .unwrap();
    assert_eq!(received["title"], "Inbound card");
    assert_eq!(received["email"]["direction"], "in");
    assert_eq!(
        received["email"]["sender"]["address"],
        "sender@example.invalid"
    );
    assert!(
        run(
            &app,
            &server,
            &[
                "integration",
                "inbound-email",
                "submit",
                "--file",
                email.to_str().unwrap()
            ]
        )
        .await
        .unwrap()["resource_id"]
            .is_null()
    );
    let raw = directory.path().join("email.eml");
    fs::write(&raw,format!("From: sender@example.invalid\r\nTo: {address}\r\nSubject: RFC822 card\r\nMessage-ID: <raw-{nonce}@example.invalid>\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nRFC822 body\r\n")).unwrap();
    let raw_receipt = run(
        &app,
        &server,
        &[
            "integration",
            "inbound-email",
            "submit",
            "--file",
            raw.to_str().unwrap(),
            "--content-type",
            "message/rfc822",
        ],
    )
    .await
    .unwrap();
    assert!(raw_receipt["resource_id"].is_string());
    let headers = directory.path().join("headers.json");
    fs::write(
        &headers,
        serde_json::to_vec(&json!({"x-wekan-bounce-secret":bounce_secret})).unwrap(),
    )
    .unwrap();
    let bounce = directory.path().join("bounce.json");
    fs::write(&bounce,serde_json::to_vec(&json!({"RecordType":"Bounce","Type":"HardBounce","Email":"bounced@example.invalid","Description":"Fixture bounce"})).unwrap()).unwrap();
    let receipt = run(
        &app,
        &server,
        &[
            "integration",
            "inbound-email",
            "bounce",
            "postmark",
            "--file",
            bounce.to_str().unwrap(),
            "--headers-file",
            headers.to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(receipt, json!({"ok":true,"recorded":1}));
    assert!(!mongo("print(JSON.stringify(db.getSiblingDB('wekan').email_bounces.findOne({email:p.email})));",json!({"email":"bounced@example.invalid"})).is_null());
    run(&app, &server, &["board", "get", board_id])
        .await
        .unwrap();
    run(&app, &server, &["board", "delete", board_id, "--yes"])
        .await
        .unwrap();
    assert!(user.is_object());
}
