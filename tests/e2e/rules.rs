use super::support::*;
use serde_json::{Value, json};
type LiveApp = App<CapturingStore, FixedPassword, FileProfileStore>;

async fn run(app: &LiveApp, server: &str, args: &[&str]) -> Result<Value, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output", "json"];
    argv.extend_from_slice(args);
    let result = app
        .execute(Cli::try_parse_from(argv).unwrap().command)
        .await?;
    Ok(
        serde_json::from_str::<Value>(&wekan_cli::output::render_success(
            OutputFormat::Json,
            &result,
        ))
        .unwrap()["data"]
            .clone(),
    )
}
fn mongo(script: &str) {
    let script = format!("const d=db.getSiblingDB('wekan');{script}");
    let result = Command::new("docker")
        .args([
            "compose", "exec", "-T", "mongodb", "mongosh", "--quiet", "--eval", &script,
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "Mongo fixture failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}
async fn create(app: &LiveApp, server: &str, board: &str, definition: Value) -> Value {
    run(
        app,
        server,
        &[
            "rule",
            "create",
            "--board",
            board,
            "--rule",
            &definition.to_string(),
        ],
    )
    .await
    .unwrap()
}
async fn get(app: &LiveApp, server: &str, board: &str, rule: &str) -> Value {
    run(app, server, &["rule", "get", rule, "--board", board])
        .await
        .unwrap()
}
async fn delete(app: &LiveApp, server: &str, board: &str, rule: &str) {
    assert_eq!(
        run(
            app,
            server,
            &["rule", "delete", rule, "--board", board, "--yes"]
        )
        .await
        .unwrap()["deleted"],
        true
    );
}
async fn wait_members(
    app: &LiveApp,
    server: &str,
    board: &str,
    card: &str,
    user: &str,
    present: bool,
) {
    for _ in 0..80 {
        let d = run(app, server, &["card", "get", card]).await.unwrap();
        assert_eq!(d["board_id"], board);
        if d["members"].as_array().unwrap().iter().any(|v| v == user) == present {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("rule member action did not reach expected state");
}
#[tokio::test]
#[ignore = "destructive: requires an isolated Wekan v12.57 stack and WEKAN_RULE_E2E_URL"]
async fn complete_rule_lifecycle_matches_wekan_v12_57() {
    let server = env::var("WEKAN_RULE_E2E_URL")
        .expect("set WEKAN_RULE_E2E_URL to an isolated Wekan v12.57 stack");
    let canonical = ServerUrl::parse(&server).unwrap().as_str().to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let username = format!("wekan_cli_rules_{nonce}");
    let dir = TestDirectory::new("rule-lifecycle");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Rules-{nonce}!"))),
        FileProfileStore::at(dir.path().to_owned()),
    );
    let owner = run(
        &app,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &username,
            "--email",
            &format!("{username}@example.test"),
            "--password-stdin",
        ],
    )
    .await
    .unwrap()["user_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let board_data = run(
        &app,
        &server,
        &["board", "create", "--title", "Rule lifecycle"],
    )
    .await
    .unwrap();
    let board = board_data["board_id"].as_str().unwrap();
    let swimlane = board_data["default_swimlane_id"].as_str().unwrap();
    let foreign_data = run(
        &app,
        &server,
        &["board", "create", "--title", "Foreign rules"],
    )
    .await
    .unwrap();
    let foreign = foreign_data["board_id"].as_str().unwrap();
    let (_rd, reader, _) =
        register_live_board_role(&server, &canonical, &token, board, nonce, "readonly").await;
    let (_nd, normal, _) =
        register_live_board_role(&server, &canonical, &token, board, nonce, "normal").await;
    let (_od, _outsider, outsider_id) =
        register_live_board_role(&server, &canonical, &token, board, nonce + 1, "normal").await;
    run(
        &app,
        &server,
        &["board", "member", "remove", &outsider_id, "--board", board],
    )
    .await
    .unwrap();
    assert_eq!(
        run(&app, &server, &["rule", "list", "--board", board])
            .await
            .unwrap(),
        json!({"board_id":board,"rules":[]})
    );

    let created=create(&app,&server,board,json!({"title":"","trigger":{"activityType":"moveCard","oldListName":"Doing","_id":"ignored","boardId":"ignored","createdAt":"ignored","updatedAt":"ignored"},"action":{"actionType":"removeMember","username":"*","boardId":foreign,"modifiedAt":"ignored"}})).await;
    let rule = created["rule_id"].as_str().unwrap();
    let detail = get(&app, &server, board, rule).await;
    assert_eq!(detail["title"], "API rule");
    assert_eq!(detail["enabled"], true);
    assert!(detail["extra_triggers"].is_null());
    assert!(detail["extra_actions"].is_null());
    for enabled in [false, true] {
        let definition = json!({"enabled":enabled}).to_string();
        let updated = run(
            &app,
            &server,
            &[
                "rule",
                "update",
                rule,
                "--board",
                board,
                "--rule",
                &definition,
            ],
        )
        .await
        .unwrap();
        assert_eq!(updated["submitted_fields"], json!(["enabled"]));
        assert_eq!(get(&app, &server, board, rule).await["enabled"], enabled);
    }
    let invalid = run(
        &app,
        &server,
        &[
            "rule",
            "update",
            rule,
            "--board",
            board,
            "--rule",
            r#"{"enabled":null}"#,
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(invalid.code(), ErrorCode::InvalidInput);
    assert_eq!(get(&app, &server, board, rule).await["enabled"], true);
    // Native null is a different request from omission. The typed command
    // rejects it; the raw escape hatch preserves the pinned server's coercion.
    let response = reqwest::Client::new()
        .put(format!("{canonical}api/boards/{board}/rules/{rule}"))
        .bearer_auth(&token)
        .json(&json!({"enabled":null}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(get(&app, &server, board, rule).await["enabled"], false);
    run(
        &app,
        &server,
        &[
            "rule",
            "update",
            rule,
            "--board",
            board,
            "--rule",
            r#"{"enabled":true}"#,
        ],
    )
    .await
    .unwrap();
    assert_eq!(get(&app, &server, board, rule).await["enabled"], true);

    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        // These are the same producers used by the native Rules UI for
        // multiple triggers and actions (server/rulesButton.js).
        let mut session = ddp::Session::login(&server, &token).await;
        let trigger = session
            .call("rules.addPart", json!([rule,"trigger",{"activityType":"textContainsTrigger","textContains":"Extra match"}]))
            .await;
        let action = session
            .call("rules.addPart", json!([rule,"action",{"actionType":"addComment","commentText":"Extra action"}]))
            .await;
        let shown = get(&app, &server, board, rule).await;
        assert_eq!(shown["extra_triggers"], json!([{"activity_type":"textContainsTrigger","text_contains":"Extra match"}]));
        assert_eq!(shown["extra_actions"], json!([{"action_type":"addComment","comment_text":"Extra action"}]));
        let listed = run(&app, &server, &["rule", "list", "--board", board]).await.unwrap();
        assert_eq!(listed["rules"], json!([shown]));
        session.call("rules.removePart", json!([rule,"trigger",trigger["partId"]])).await;
        session.call("rules.removePart", json!([rule,"action",action["partId"]])).await;
        session.close().await;
    })
    .await
    .expect("native extra rule parts timed out");
    let detail = get(&app, &server, board, rule).await;
    assert!(detail["extra_triggers"].is_null());
    assert!(detail["extra_actions"].is_null());
    for field in ["list_name", "user_id", "swimlane_name", "card_title"] {
        assert_eq!(detail["trigger"][field], "*");
    }
    for field in ["board_id", "id", "created_at", "updated_at"] {
        assert!(detail["trigger"].get(field).is_none());
    }
    assert!(detail["action"].get("board_id").is_none());
    mongo(&format!(
        "const r=d.rules.findOne({{_id:{}}});const a=d.actions.findOne({{_id:r.actionId}});if(a.boardId!=={})throw Error('REST action board was not rebound');",
        json!(rule),
        json!(board)
    ));
    let updated = run(
        &app,
        &server,
        &[
            "rule",
            "update",
            rule,
            "--board",
            board,
            "--rule",
            r#"{"title":"Renamed","trigger":{"listName":"Next"},"action":{"desc":null}}"#,
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        updated["submitted_fields"],
        json!(["title", "trigger", "action"])
    );
    assert!(updated.get("mutation_confirmed").is_none());
    let detail = get(&app, &server, board, rule).await;
    assert_eq!(detail["trigger"]["activity_type"], "moveCard");
    assert_eq!(detail["trigger"]["old_list_name"], "Doing");
    assert_eq!(detail["action"]["desc"], Value::Null);
    run(
        &app,
        &server,
        &[
            "rule",
            "update",
            rule,
            "--board",
            board,
            "--rule",
            r#"{"trigger":{"activityType":"createCard","listName":null}}"#,
        ],
    )
    .await
    .unwrap();
    let detail = get(&app, &server, board, rule).await;
    assert_eq!(detail["trigger"]["list_name"], "*");
    assert_eq!(detail["trigger"]["old_list_name"], "Doing");
    run(
        &app,
        &server,
        &["rule", "update", rule, "--board", board, "--rule", "{}"],
    )
    .await
    .unwrap();
    assert_eq!(get(&app, &server, board, rule).await, detail);

    let rules_before = run(&app, &server, &["rule", "list", "--board", board])
        .await
        .unwrap();
    for args in [
        vec![
            "rule",
            "create",
            "--board",
            board,
            "--rule",
            r#"{"trigger":{"activityType":"button","lastRunAt":"not-a-date"},"action":{"actionType":"archive"}}"#,
        ],
        vec![
            "rule",
            "update",
            rule,
            "--board",
            board,
            "--rule",
            r#"{"title":"Must not persist","trigger":{"lastRunAt":"not-a-date"}}"#,
        ],
    ] {
        assert_eq!(
            run(&app, &server, &args).await.unwrap_err().code(),
            ErrorCode::InvalidInput
        );
    }
    assert_eq!(get(&app, &server, board, rule).await, detail);
    assert_eq!(
        run(&app, &server, &["rule", "list", "--board", board])
            .await
            .unwrap(),
        rules_before
    );

    let definition_file = dir.path().join("large-rule.json");
    let description = "x".repeat(2 * 1024 * 1024);
    std::fs::write(
        &definition_file,
        json!({
            "trigger":{"activityType":"button", "lastRunAt":"2030-01-01T03:00:00+03:00"},
            "action":{"actionType":"archive", "desc":description}
        })
        .to_string(),
    )
    .unwrap();
    let large = run(
        &app,
        &server,
        &[
            "rule",
            "create",
            "--board",
            board,
            "--rule-file",
            definition_file.to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    let large_id = large["rule_id"].as_str().unwrap();
    let large_detail = get(&app, &server, board, large_id).await;
    assert_eq!(large_detail["action"]["desc"], description);
    assert_eq!(
        large_detail["trigger"]["last_run_at"],
        "2030-01-01T03:00:00+03:00"
    );
    let listed = run(&app, &server, &["rule", "list", "--board", board])
        .await
        .unwrap();
    assert_eq!(
        listed["rules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["rule_id"] == large_id)
            .unwrap(),
        &large_detail
    );

    let description = "y".repeat(3 * 1024 * 1024);
    std::fs::write(
        &definition_file,
        json!({
            "trigger":{"lastRunAt":null}, "action":{"desc":description}
        })
        .to_string(),
    )
    .unwrap();
    run(
        &app,
        &server,
        &[
            "rule",
            "update",
            large_id,
            "--board",
            board,
            "--rule-file",
            definition_file.to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    let large_detail = get(&app, &server, board, large_id).await;
    assert_eq!(large_detail["action"]["desc"], description);
    assert!(large_detail["trigger"]["last_run_at"].is_null());
    let listed = run(&app, &server, &["rule", "list", "--board", board])
        .await
        .unwrap();
    assert_eq!(
        listed["rules"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["rule_id"] == large_id)
            .unwrap(),
        &large_detail
    );
    delete(&app, &server, board, large_id).await;

    for target in ["missing-rule", rule] {
        let scope = if target == rule { foreign } else { board };
        for args in [
            vec!["rule", "get", target, "--board", scope],
            vec!["rule", "update", target, "--board", scope, "--rule", "{}"],
            vec!["rule", "delete", target, "--board", scope, "--yes"],
        ] {
            let e = run(&app, &server, &args).await.unwrap_err();
            assert_eq!(e.code(), ErrorCode::ServerError);
            assert_eq!(e.details().http_status, Some(500));
            assert_eq!(
                e.details().server_error.as_deref(),
                Some("Internal server error")
            );
        }
    }
    assert_eq!(
        run(&reader, &server, &["rule", "get", rule, "--board", board])
            .await
            .unwrap()["rule_id"],
        rule
    );
    for (case, args) in [
        vec![
            "rule",
            "create",
            "--board",
            board,
            "--rule",
            r#"{"trigger":{"activityType":"button"},"action":{"actionType":"archive"}}"#,
        ],
        vec!["rule", "update", rule, "--board", board, "--rule", "{}"],
        vec!["rule", "delete", rule, "--board", board, "--yes"],
    ]
    .into_iter()
    .enumerate()
    {
        let (_probe_dir, probe, _) = register_live_board_role(
            &server,
            &canonical,
            &token,
            board,
            nonce + 10 + case as u128,
            "readonly",
        )
        .await;
        assert_eq!(
            run(&probe, &server, &args).await.unwrap_err().code(),
            ErrorCode::PermissionDenied
        );
    }
    for (case, args) in [
        vec!["rule", "list", "--board", board],
        vec!["rule", "get", rule, "--board", board],
        vec!["rule", "update", rule, "--board", board, "--rule", "{}"],
    ]
    .into_iter()
    .enumerate()
    {
        let (_probe_dir, probe, probe_id) = register_live_board_role(
            &server,
            &canonical,
            &token,
            board,
            nonce + 20 + case as u128,
            "normal",
        )
        .await;
        run(
            &app,
            &server,
            &["board", "member", "remove", &probe_id, "--board", board],
        )
        .await
        .unwrap();
        assert_eq!(
            run(&probe, &server, &args).await.unwrap_err().code(),
            ErrorCode::PermissionDenied
        );
    }
    assert!(
        !run(&reader, &server, &["rule", "list", "--board", board])
            .await
            .unwrap()["rules"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        run(
            &app,
            &server,
            &["rule", "get", rule, "--board", "missing-board"]
        )
        .await
        .unwrap_err()
        .details()
        .http_status,
        Some(404)
    );
    assert_eq!(
        reqwest::Client::new()
            .get(format!("{canonical}api/boards/{board}/rules"))
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        401
    );
    run(&normal, &server, &["rule", "list", "--board", board])
        .await
        .unwrap();
    run(&normal, &server, &["rule", "get", rule, "--board", board])
        .await
        .unwrap();
    for (index, args) in [
        vec![
            "rule",
            "create",
            "--board",
            board,
            "--rule",
            r#"{"trigger":{"activityType":"button"},"action":{"actionType":"archive"}}"#,
        ],
        vec!["rule", "update", rule, "--board", board, "--rule", "{}"],
        vec!["rule", "delete", rule, "--board", board, "--yes"],
    ]
    .into_iter()
    .enumerate()
    {
        let (_case_dir, case, _) = register_live_board_role(
            &server,
            &canonical,
            &token,
            board,
            nonce + 30 + index as u128,
            "normal",
        )
        .await;
        let error = run(&case, &server, &args).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::PermissionDenied);
        assert_eq!(error.details().http_status, Some(403));
    }

    // All built-in fields are explicit; no opaque payload maps bypass decoding.
    let full_trigger = json!({"activityType":"button","userId":"*","username":"*","listName":"List","oldListName":"Old","swimlaneName":"Default","cardTitle":"Filter","checklistName":"Checklist","checklistItemName":"Item","labelId":"label","attachmentName":"file","desc":"Description","scheduleKind":"calendar","scheduleType":"weekly","atTime":"25:00","weekday":2,"dayOfMonth":"3","onDate":"2035-01-02","dueCondition":"soon","days":"2","lastRunKey":"old","lastRunAt":"2026-10-15T00:00:00.000Z","buttonType":"board","buttonLabel":"Run","advancedFilter":"title:Filter","textContains":"Filter","cron":"0 9 * * 1-5","direction":"right"});
    let full_action = json!({"actionType":"setDateRelative","listName":"List","swimlaneName":"Default","cardName":"Card","fromListName":"Old","checklistName":"Checklist","checkItemName":"Item","checklistItems":"one,two","dateField":"dueAt","unit":"hours","emailTo":"nobody@example.test","emailSubject":"Subject","emailMsg":"Body","selectedColor":"green","labelId":"label","username":"*","sortField":"name","desc":"Description","days":-2,"includeCardDetails":true,"includeChecklistsAndComments":false,"includeAttachments":true,"integrationId":"integration","commentText":"A comment","listId":"list","swimlaneId":"lane"});
    let full = create(
        &app,
        &server,
        board,
        json!({"trigger":full_trigger,"action":full_action}),
    )
    .await;
    let full_id = full["rule_id"].as_str().unwrap();
    let mapped = get(&app, &server, board, full_id).await;
    assert_eq!(mapped["trigger"]["last_run_at"], "2026-10-15T00:00:00.000Z");
    assert_eq!(mapped["trigger"]["day_of_month"], "3");
    assert_eq!(mapped["action"]["days"], -2);
    assert_eq!(mapped["action"]["checklist_items"], "one,two");
    for (input, actual) in [
        (&full_trigger, &mapped["trigger"]),
        (&full_action, &mapped["action"]),
    ] {
        for (key, value) in input.as_object().unwrap() {
            let key: String = key
                .chars()
                .flat_map(|c| {
                    if c.is_ascii_uppercase() {
                        vec!['_', c.to_ascii_lowercase()]
                    } else {
                        vec![c]
                    }
                })
                .collect();
            assert_eq!(
                &actual[&key], value,
                "unmapped or changed payload field {key}"
            );
        }
    }

    let mut null_trigger = full_trigger.clone();
    let mut null_action = full_action.clone();
    for (key, value) in null_trigger.as_object_mut().unwrap() {
        if key != "activityType" {
            *value = Value::Null;
        }
    }
    for (key, value) in null_action.as_object_mut().unwrap() {
        if key != "actionType" {
            *value = Value::Null;
        }
    }
    let nulls = create(
        &app,
        &server,
        board,
        json!({"trigger":null_trigger,"action":null_action}),
    )
    .await;
    let null_id = nulls["rule_id"].as_str().unwrap();
    let null_detail = get(&app, &server, board, null_id).await;
    for (key, value) in null_detail["trigger"].as_object().unwrap() {
        if key != "activity_type" {
            assert!(value.is_null(), "{key}");
        }
    }
    for (key, value) in null_detail["action"].as_object().unwrap() {
        if key != "action_type" {
            assert!(value.is_null(), "{key}");
        }
    }
    assert_eq!(
        null_detail["trigger"].as_object().unwrap().len(),
        null_trigger.as_object().unwrap().len()
    );
    assert_eq!(
        null_detail["action"].as_object().unwrap().len(),
        null_action.as_object().unwrap().len()
    );
    delete(&app, &server, board, null_id).await;
    for activity in [
        "createCard",
        "moveCard",
        "archivedCard",
        "restoredCard",
        "joinMember",
        "unjoinMember",
        "addChecklist",
        "removeChecklist",
        "completeChecklist",
        "uncompleteChecklist",
        "addedChecklistItem",
        "removedChecklistItem",
        "checkedItem",
        "uncheckedItem",
        "addAttachment",
        "deleteAttachment",
        "addedLabel",
        "removedLabel",
        "scheduledTrigger",
        "button",
    ] {
        let c=create(&app,&server,board,json!({"trigger":{"activityType":activity,"atTime":"25:00"},"action":{"actionType":"archive"}})).await;
        let id = c["rule_id"].as_str().unwrap();
        assert_eq!(
            get(&app, &server, board, id).await["trigger"]["activity_type"],
            activity
        );
        delete(&app, &server, board, id).await;
    }
    for action in [
        "moveCardToTop",
        "moveCardToBottom",
        "sendEmail",
        "setDate",
        "updateDate",
        "removeDate",
        "setDateRelative",
        "archive",
        "unarchive",
        "setColor",
        "addLabel",
        "removeLabel",
        "addMember",
        "removeMember",
        "checkAll",
        "uncheckAll",
        "checkItem",
        "uncheckItem",
        "addChecklist",
        "removeChecklist",
        "addChecklistWithItems",
        "addSwimlane",
        "createCard",
        "linkCard",
        "markCardComplete",
        "markCardIncomplete",
        "sortList",
        "moveAllCardsInList",
    ] {
        let c = create(
            &app,
            &server,
            board,
            json!({"trigger":{"activityType":"button"},"action":{"actionType":action}}),
        )
        .await;
        let id = c["rule_id"].as_str().unwrap();
        assert_eq!(
            get(&app, &server, board, id).await["action"]["action_type"],
            action
        );
        delete(&app, &server, board, id).await;
    }

    // W11-054 is fixed: REST keeps the denormalized metadata used by UI buttons.
    mongo(&format!(
        "const r=d.rules.findOne({{_id:{}}});if(r.buttonType!=='board'||r.buttonLabel!=='Run')throw Error('REST button metadata did not match its trigger');",
        json!(full_id)
    ));
    let client = reqwest::Client::new();
    let url = format!("{canonical}api/boards/{board}/rules/{rule}");
    // Native sequential writes can precede a Mongo rejection of a later field.
    let response=client.put(&url).bearer_auth(&token).json(&json!({"title":"Before action failure","trigger":{"desc":"Before failure"},"action":{"$invalid":1}})).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 500);
    let after = get(&app, &server, board, rule).await;
    assert_eq!(after["title"], "Before action failure");
    assert_eq!(after["trigger"]["desc"], "Before failure");

    let loose = client
        .post(format!("{canonical}api/boards/{board}/rules"))
        .bearer_auth(&token)
        .json(&json!({"trigger":{"activityType":1},"action":{"actionType":true}}))
        .send()
        .await
        .unwrap();
    assert_eq!(loose.status().as_u16(), 200);
    let loose: Value = loose.json().await.unwrap();
    let loose_id = loose["_id"].as_str().unwrap();
    assert_eq!(
        run(&app, &server, &["rule", "get", loose_id, "--board", board])
            .await
            .unwrap_err()
            .code(),
        ErrorCode::ProtocolError
    );
    delete(&app, &server, board, loose_id).await;

    // Deliberately seed unmapped payloads; the production decoder must fail.
    for collection in ["triggers", "actions"] {
        let id = if collection == "triggers" {
            &full["trigger_id"]
        } else {
            &full["action_id"]
        };
        mongo(&format!(
            "d.{collection}.updateOne({{_id:{id}}},{{$set:{{unmapped:{{nested:true}}}}}});"
        ));
        let error = run(&app, &server, &["rule", "get", full_id, "--board", board])
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::ProtocolError);
        let error = run(&app, &server, &["rule", "list", "--board", board])
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::ProtocolError);
        mongo(&format!(
            "d.{collection}.updateOne({{_id:{id}}},{{$unset:{{unmapped:''}}}});"
        ));
    }
    mongo(&format!(
        "d.triggers.deleteOne({{_id:{}}});d.actions.deleteOne({{_id:{}}});",
        full["trigger_id"], full["action_id"]
    ));
    let dangling = get(&app, &server, board, full_id).await;
    assert_eq!(dangling["trigger"], Value::Null);
    assert_eq!(dangling["action"], Value::Null);
    run(
        &app,
        &server,
        &[
            "rule",
            "update",
            full_id,
            "--board",
            board,
            "--rule",
            r#"{"trigger":{"desc":"no match"},"action":{"desc":"no match"}}"#,
        ],
    )
    .await
    .unwrap();
    let repaired = get(&app, &server, board, full_id).await;
    assert_eq!(repaired["trigger"], json!({"desc":"no match"}));
    assert_eq!(repaired["action"], json!({"desc":"no match"}));
    delete(&app, &server, board, full_id).await;
    delete(&app, &server, board, rule).await;
    mongo(&format!(
        "if(d.rules.findOne({{_id:{}}})||d.triggers.findOne({{_id:{}}})||d.actions.findOne({{_id:{}}}))throw Error('rule deletion left references');",
        created["rule_id"], created["trigger_id"], created["action_id"]
    ));

    // Prove actual rule execution through existing production card commands.
    let todo = run(
        &app,
        &server,
        &["list", "create", "--board", board, "--title", "Todo"],
    )
    .await
    .unwrap()["list_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let doing = run(
        &app,
        &server,
        &["list", "create", "--board", board, "--title", "Doing"],
    )
    .await
    .unwrap()["list_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let card = run(
        &app,
        &server,
        &[
            "card",
            "create",
            "--board",
            board,
            "--list",
            &todo,
            "--title",
            "Rule card",
            "--swimlane-id",
            swimlane,
        ],
    )
    .await
    .unwrap()["card_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let add=create(&app,&server,board,json!({"trigger":{"activityType":"moveCard","listName":"Doing"},"action":{"actionType":"addMember","username":username}})).await;
    let remove=create(&app,&server,board,json!({"trigger":{"activityType":"moveCard","oldListName":"Doing"},"action":{"actionType":"removeMember","username":username}})).await;
    run(
        &app,
        &server,
        &[
            "card",
            "update",
            &card,
            "--board",
            board,
            "--list",
            &todo,
            "--list-id",
            &doing,
        ],
    )
    .await
    .unwrap();
    wait_members(&app, &server, board, &card, &owner, true).await;
    run(
        &app,
        &server,
        &[
            "card",
            "update",
            &card,
            "--board",
            board,
            "--list",
            &doing,
            "--list-id",
            &todo,
        ],
    )
    .await
    .unwrap();
    wait_members(&app, &server, board, &card, &owner, false).await;
    for id in [&add["rule_id"], &remove["rule_id"]] {
        delete(&app, &server, board, id.as_str().unwrap()).await;
    }

    let scheduled=create(&app,&server,board,json!({"trigger":{"activityType":"scheduledTrigger","scheduleKind":"calendar","scheduleType":"daily","listName":"Todo"},"action":{"actionType":"setColor","selectedColor":"purple"}})).await;
    let scheduled_id = scheduled["rule_id"].as_str().unwrap();
    let mut fired = false;
    for _ in 0..90 {
        let detail = get(&app, &server, board, scheduled_id).await;
        if detail["trigger"].get("last_run_at").is_some() {
            assert!(detail["trigger"]["last_run_key"].as_str().is_some());
            assert_eq!(
                run(&app, &server, &["card", "get", &card]).await.unwrap()["color"],
                "purple"
            );
            fired = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    // W11-053 is fixed: the native startup starts the registered minute scanner.
    assert!(
        fired,
        "the native scheduled rule did not run within 90 seconds"
    );
    let detail = get(&app, &server, board, scheduled_id).await;
    assert!(detail["trigger"]["last_run_at"].is_string());
    assert!(detail["trigger"]["last_run_key"].is_string());
    assert_eq!(
        run(&app, &server, &["card", "get", &card]).await.unwrap()["color"],
        "purple"
    );
    delete(&app, &server, board, scheduled_id).await;
    assert_eq!(
        run(&app, &server, &["rule", "list", "--board", board])
            .await
            .unwrap()["rules"],
        json!([])
    );
    for board in [board, foreign] {
        run(&app, &server, &["board", "delete", board, "--yes"])
            .await
            .unwrap();
    }
}
