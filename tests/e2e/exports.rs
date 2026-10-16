use super::support::attachment_fixtures::{fixture_command, seed};
use super::support::*;
use serde_json::{Value, json};
use std::io::{Cursor, Read};

fn inspect_xlsx(bytes: &[u8]) {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).expect("XLSX must be a complete ZIP archive");
    assert!(archive.by_name("xl/workbook.xml").is_ok());
    assert!(archive.by_name("xl/worksheets/sheet1.xml").is_ok());
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let mut contents = Vec::new();
        entry
            .read_to_end(&mut contents)
            .expect("every XLSX entry must decompress and pass its CRC");
    }
}
fn inspect_pdf(bytes: &[u8]) {
    assert!(bytes.starts_with(b"%PDF-"));
    assert!(bytes.windows(5).any(|w| w == b"%%EOF"));
    assert!(bytes.windows(7).any(|w| w == b"/Type /"));
}

type LiveApp = App<CapturingStore, FixedPassword, FileProfileStore>;
async fn run(app: &LiveApp, server: &str, args: &[&str]) -> Result<CommandSuccess, AppError> {
    let cli = Cli::try_parse_from(
        ["wekan", "--server", server]
            .into_iter()
            .chain(args.iter().copied()),
    )
    .unwrap();
    app.execute(cli.command).await
}
async fn raw(app: &LiveApp, server: &str, args: &[&str]) -> Vec<u8> {
    let cli = Cli::try_parse_from(
        ["wekan", "--server", server, "--output", "raw"]
            .into_iter()
            .chain(args.iter().copied())
            .chain(["--destination", "-"]),
    )
    .unwrap();
    cli.command.validate_output(OutputFormat::Raw).unwrap();
    let success = app
        .execute(cli.command)
        .await
        .unwrap_or_else(|e| panic!("{args:?}: {e:?}"));
    let mut out = Vec::new();
    let mut err = Vec::new();
    write_success(OutputFormat::Raw, success, &mut out, &mut err)
        .await
        .unwrap();
    assert!(err.is_empty());
    out
}
fn mongo(script: &str, input: &Value) -> Value {
    fixture_command("mongodb", "mongosh", script, input)
}

async fn import_and_export(
    app: &LiveApp,
    server: &str,
    token: &str,
    source: &str,
    document: Value,
) -> (String, Value) {
    // Fixture only: exercise the same importer as Wekan's UI, without seeding
    // an activity whose shape simply repeats the decoder's expectations.
    let board_id = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut session = ddp::Session::login(server, token).await;
        let result = session
            .call("importBoard", json!([document, {}, source, null]))
            .await;
        session.close().await;
        result
            .as_str()
            .expect("native import must return a board ID")
            .to_owned()
    })
    .await
    .expect("native board import fixture timed out");
    // importBoard awaits its writes. Independently confirm the persisted
    // activity, then read it through the production export and strict decoder.
    let stored_activity = mongo(
        "print(JSON.stringify(db.getSiblingDB('wekan').activities.findOne({boardId:p.board_id,activityType:'importBoard'})));",
        &json!({"board_id":board_id}),
    );
    assert!(stored_activity["source"].is_object());
    let bytes = raw(
        app,
        server,
        &["board", "export", &board_id, "--format", "json"],
    )
    .await;
    let exported: Value = serde_json::from_slice(&bytes).unwrap();
    let exported_activity = exported["activities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|activity| activity["_id"] == stored_activity["_id"])
        .expect("native export must include the persisted import activity");
    assert_eq!(exported_activity["source"], stored_activity["source"]);
    execute_live_board(app, server, &["delete", &board_id, "--yes"])
        .await
        .unwrap();
    wait_for_live_board_absent(&reqwest::Client::new(), server, token, &board_id).await;
    (board_id, exported_activity["source"].clone())
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v11.06 stack and WEKAN_EXPORT_E2E_URL"]
async fn complete_export_lifecycle_matches_wekan_v11_06() {
    let server = env::var("WEKAN_EXPORT_E2E_URL")
        .expect("set WEKAN_EXPORT_E2E_URL to a fresh isolated stack");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let directory = TestDirectory::new("exports");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Exports-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let username = format!("exports_{nonce}");
    let CommandSuccess::Registration(user) = run(
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
    .unwrap() else {
        panic!("registration")
    };
    let CommandSuccess::BoardCreated(board) =
        execute_live_board(&app, &server, &["create", "--title", "Export lifecycle"])
            .await
            .unwrap()
    else {
        panic!("board")
    };
    let b = &board.board_id;
    let s = &board.default_swimlane_id;
    let CommandSuccess::ListCreated(list) = execute_live_list(
        &app,
        &server,
        &[
            "create",
            "--board",
            b,
            "--swimlane-id",
            s,
            "--title",
            "Todo",
        ],
    )
    .await
    .unwrap() else {
        panic!("list")
    };
    let l = &list.list_id;
    let CommandSuccess::CardCreated(card) = execute_live_card(
        &app,
        &server,
        &[
            "create",
            "--board",
            b,
            "--list",
            l,
            "--swimlane-id",
            s,
            "--title",
            "Ship",
            "--description",
            "Export details",
            "--due-at",
            "2026-12-20T12:00:00Z",
        ],
    )
    .await
    .unwrap() else {
        panic!("card")
    };
    let c = &card.card_id;
    execute_live_card(
        &app,
        &server,
        &[
            "update",
            c,
            "--board",
            b,
            "--list",
            l,
            "--title",
            "Ship updated",
        ],
    )
    .await
    .unwrap();
    run(
        &app,
        &server,
        &[
            "comment",
            "create",
            "--board",
            b,
            "--card",
            c,
            "--text",
            "Export comment",
        ],
    )
    .await
    .unwrap();
    let checklist = run(
        &app,
        &server,
        &[
            "checklist",
            "create",
            "--board",
            b,
            "--card",
            c,
            "--title",
            "Release",
        ],
    )
    .await
    .unwrap();
    let CommandSuccess::ChecklistCreated(checklist) = checklist else {
        panic!("checklist")
    };
    run(&app,&server,&["rule","create","--board",b,"--rule",r#"{"title":"Export rule","trigger":{"activityType":"createCard","listName":"Todo"},"action":{"actionType":"addLabel","labelId":"fixture-label"}}"#]).await.unwrap();
    let attachment_id = format!("export-attachment-{nonce}");
    let _file = seed(
        &json!({"id":attachment_id,"board_id":b,"swimlane_id":s,"list_id":l,"card_id":c,"user_id":user.user_id,"storage":"fs","path":format!("/data/files/attachments/{attachment_id}.bin"),"data":"AP8K"}),
    );
    let token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let canonical = ServerUrl::parse(&server).unwrap().as_str().to_owned();
    let ui_checklist_id = format!("export-ui-checklist-{nonce}");
    // Use the UI's native collection insert so the server supplies the creator.
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut session = ddp::Session::login(&server, &token).await;
        session
            .call(
                "/checklists/insert",
                json!([{
                    "_id":ui_checklist_id, "cardId":c, "title":"UI export checklist", "sort":1
                }]),
            )
            .await;
        session.close().await;
    })
    .await
    .expect("UI checklist export fixture timed out");
    let stored_checklists = mongo(
        "const w=db.getSiblingDB('wekan');print(JSON.stringify({ui:w.checklists.findOne({_id:p.ui_id}),rest:w.checklists.findOne({_id:p.rest_id})}));",
        &json!({"ui_id":ui_checklist_id,"rest_id":checklist.checklist_id}),
    );
    assert_eq!(stored_checklists["ui"]["userId"], user.user_id);
    assert!(stored_checklists["rest"].is_object());
    assert!(stored_checklists["rest"].get("userId").is_none());
    let (_normal_dir, normal, _normal_id) =
        register_live_board_role(&server, &canonical, &token, b, nonce, "normal").await;
    // Native authentication modes remain explicit: no retry/fallback to query auth.
    raw(
        &normal,
        &server,
        &["board", "export", b, "--format", "json"],
    )
    .await;
    for (resource, id, scope, format) in [
        ("board", b.as_str(), vec![], Some("csv")),
        ("board", b.as_str(), vec![], Some("tsv")),
        ("board", b.as_str(), vec![], Some("xlsx")),
        (
            "card",
            c.as_str(),
            vec!["--board", b, "--list", l],
            Some("xlsx"),
        ),
        (
            "attachment",
            attachment_id.as_str(),
            vec!["--board", b],
            None,
        ),
    ] {
        let mut denied = vec![resource, "export", id, "--destination", "-"];
        denied.extend(scope.clone());
        if let Some(format) = format {
            denied.extend(["--format", format]);
        }
        let error = run(&normal, &server, &denied).await.unwrap_err();
        assert_eq!(error.details().http_status, Some(403));
        let mut accepted = denied;
        accepted.push("--auth-token-query");
        let cli = Cli::try_parse_from(["wekan", "--server", &server].into_iter().chain(accepted))
            .unwrap();
        normal.execute(cli.command).await.unwrap();
    }
    inspect_pdf(&raw(&normal, &server, &["board", "export", b, "--format", "pdf"]).await);
    inspect_pdf(
        &raw(
            &normal,
            &server,
            &[
                "card", "export", c, "--board", b, "--list", l, "--format", "pdf",
            ],
        )
        .await,
    );

    // Every JSON format passes through production strict decoding.
    for format in [
        "json",
        "kanboard",
        "deck",
        "openproject",
        "github",
        "gitlab",
        "gitea",
        "forgejo",
        "trello",
        "jira",
        "asana",
        "zenkit",
    ] {
        let out = raw(
            &app,
            &server,
            &[
                "board",
                "export",
                b,
                "--format",
                format,
                "--auth-token-query",
            ],
        )
        .await;
        let value: Value = serde_json::from_slice(&out).unwrap();
        assert!(value.is_object() || value.is_array());
        if format == "json" {
            assert_eq!(value["cards"].as_array().unwrap().len(), 1);
            assert_eq!(value["attachments"][0]["file"], "AP8K");
            for stored in ["ui", "rest"] {
                let exported = value["checklists"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|item| item["_id"] == stored_checklists[stored]["_id"])
                    .expect("export must include the independently stored checklist");
                assert_eq!(exported, &stored_checklists[stored]);
            }
            assert!(!value["activities"].as_array().unwrap().is_empty());
            assert!(!value["triggers"].as_array().unwrap().is_empty());
            assert!(
                value["activities"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|activity| activity.get("source").is_none())
            );
        }
    }
    for labels in ["[null]", "[]"] {
        execute_live_card(
            &app,
            &server,
            &["update", c, "--board", b, "--list", l, "--labels", labels],
        )
        .await
        .unwrap();
        let expected: Value = serde_json::from_str(labels).unwrap();
        let stored = mongo(
            "print(JSON.stringify(db.getSiblingDB('wekan').cards.findOne({_id:p.card_id},{labelIds:1,_id:0})));",
            &json!({"card_id":c}),
        );
        assert_eq!(stored["labelIds"], expected);
        let exported = raw(&app, &server, &["board", "export", b, "--format", "trello"]).await;
        let exported: Value = serde_json::from_slice(&exported).unwrap();
        assert_eq!(exported["cards"][0]["id"], c.as_str());
        assert_eq!(exported["cards"][0]["idLabels"], expected);
    }
    for format in ["csv", "tsv", "xlsx", "pdf"] {
        let out = raw(
            &app,
            &server,
            &[
                "board",
                "export",
                b,
                "--format",
                format,
                "--auth-token-query",
            ],
        )
        .await;
        match format {
            "pdf" => inspect_pdf(&out),
            "xlsx" => inspect_xlsx(&out),
            _ => {
                let text = String::from_utf8(out).unwrap();
                assert!(text.contains("Ship updated"));
                assert!(text.contains(if format == "csv" { ',' } else { '\t' }));
            }
        }
    }
    for format in ["xlsx", "pdf"] {
        let out = raw(
            &app,
            &server,
            &[
                "card",
                "export",
                c,
                "--board",
                b,
                "--list",
                l,
                "--format",
                format,
                "--auth-token-query",
                "--field",
                "board-info",
                "--field",
                "description",
                "--language",
                "fi",
                "--date-format",
                "DD-MM-YYYY",
                "--timezone",
                "Africa/Nairobi",
            ],
        )
        .await;
        if format == "pdf" {
            inspect_pdf(&out);
        } else {
            inspect_xlsx(&out);
        }
    }
    let attachment = raw(
        &app,
        &server,
        &[
            "attachment",
            "export",
            &attachment_id,
            "--board",
            b,
            "--auth-token-query",
        ],
    )
    .await;
    assert_eq!(
        serde_json::from_slice::<Value>(&attachment).unwrap()["file"],
        "AP8K"
    );
    assert_eq!(
        raw(
            &app,
            &server,
            &[
                "attachment",
                "export",
                "missing",
                "--board",
                b,
                "--auth-token-query"
            ]
        )
        .await,
        b"{}"
    );
    let selected = raw(
        &app,
        &server,
        &[
            "board",
            "export",
            b,
            "--format",
            "json",
            "--auth-token-query",
            "--attachments",
            "false",
            "--field",
            "description",
            "--list",
            l,
        ],
    )
    .await;
    let selected: Value = serde_json::from_slice(&selected).unwrap();
    assert_eq!(selected["attachments"], json!([]));
    assert_eq!(selected["activities"], json!([]));
    let no_files = raw(
        &app,
        &server,
        &[
            "board",
            "export",
            b,
            "--format",
            "json",
            "--auth-token-query",
            "--attachments",
            "false",
        ],
    )
    .await;
    let no_files: Value = serde_json::from_slice(&no_files).unwrap();
    assert!(
        no_files["attachments"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a.get("file").is_none())
    );
    for selector in ["--swimlane", "--list", "--card", "--checklist"] {
        let id = match selector {
            "--swimlane" => s,
            "--list" => l,
            "--checklist" => &checklist.checklist_id,
            _ => c,
        };
        let out = raw(
            &app,
            &server,
            &[
                "board",
                "export",
                b,
                "--format",
                "json",
                "--auth-token-query",
                selector,
                id,
            ],
        )
        .await;
        assert_eq!(
            serde_json::from_slice::<Value>(&out).unwrap()["cards"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
    let scoped = raw(
        &app,
        &server,
        &[
            "board",
            "export",
            b,
            "--format",
            "json",
            "--auth-token-query",
            "--list",
            "missing",
            "--card",
            c,
        ],
    )
    .await;
    assert_eq!(
        serde_json::from_slice::<Value>(&scoped).unwrap()["cards"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let private_csv = raw(
        &app,
        &server,
        &[
            "board",
            "export",
            b,
            "--format",
            "csv",
            "--auth-token-query",
            "--field",
            "description",
            "--delimiter",
            ";",
        ],
    )
    .await;
    let private_csv = String::from_utf8(private_csv).unwrap();
    assert!(private_csv.contains(';'));
    assert!(!private_csv.lines().next().unwrap().contains("labels"));
    let selected_issue = raw(
        &app,
        &server,
        &[
            "board",
            "export",
            b,
            "--format",
            "github",
            "--auth-token-query",
            "--field",
            "labels",
        ],
    )
    .await;
    let selected_issue: Value = serde_json::from_slice(&selected_issue).unwrap();
    assert_eq!(selected_issue[0]["body"], "");
    assert!(selected_issue[0].get("due_date").is_none());
    let board_locale = raw(
        &app,
        &server,
        &[
            "board",
            "export",
            b,
            "--format",
            "xlsx",
            "--auth-token-query",
            "--field",
            "card-details",
            "--field",
            "board-info",
            "--field",
            "dates",
            "--language",
            "fi",
            "--date-format",
            "DD-MM-YYYY",
            "--timezone",
            "Africa/Nairobi",
        ],
    )
    .await;
    inspect_xlsx(&board_locale);
    let zip = run(
        &app,
        &server,
        &[
            "board",
            "export",
            b,
            "--format",
            "zip",
            "--destination",
            "-",
            "--auth-token-query",
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(zip.details().http_status, Some(500));
    let missing = run(
        &app,
        &server,
        &[
            "board",
            "export",
            "missing",
            "--format",
            "xlsx",
            "--destination",
            "-",
            "--auth-token-query",
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::ProtocolError);
    assert_eq!(missing.details().http_status, Some(200));
    let broken_card = run(
        &app,
        &server,
        &[
            "card",
            "export",
            c,
            "--board",
            b,
            "--list",
            l,
            "--format",
            "xlsx",
            "--destination",
            "-",
            "--auth-token-query",
            "--field",
            "description",
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(broken_card.details().http_status, Some(500));
    let denied = run(
        &app,
        &server,
        &[
            "board",
            "export",
            b,
            "--format",
            "json",
            "--destination",
            "-",
            "--no-auth",
        ],
    )
    .await
    .unwrap_err();
    assert!(matches!(denied.details().http_status, Some(401 | 403)));

    // Confirm that unknown schemaless records fail at the actual production boundary.
    mongo(
        "db.getSiblingDB('wekan').activities.insertOne({_id:'export-unknown',boardId:p.board_id,activityType:'createCard',unmapped:true,createdAt:new Date()});print('{}');",
        &json!({"board_id":b}),
    );
    let unknown = run(
        &app,
        &server,
        &[
            "board",
            "export",
            b,
            "--format",
            "json",
            "--destination",
            "-",
            "--auth-token-query",
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(unknown.code(), ErrorCode::ProtocolError);
    assert!(
        unknown
            .details()
            .response_path
            .as_deref()
            .unwrap_or("")
            .contains("activities")
    );
    mongo(
        "db.getSiblingDB('wekan').activities.deleteOne({_id:'export-unknown'});print('{}');",
        &json!({}),
    );
    // Public exports do not need a credential; public CSV's native fields mask is ignored.
    mongo(
        "db.getSiblingDB('wekan').boards.updateOne({_id:p.board_id},{$set:{permission:'public'}});print('{}');",
        &json!({"board_id":b}),
    );
    for format in ["json", "csv", "pdf", "xlsx", "kanboard"] {
        raw(
            &app,
            &server,
            &["board", "export", b, "--format", format, "--no-auth"],
        )
        .await;
    }
    let all = raw(
        &app,
        &server,
        &["board", "export", b, "--format", "csv", "--no-auth"],
    )
    .await;
    let filtered = raw(
        &app,
        &server,
        &[
            "board",
            "export",
            b,
            "--format",
            "csv",
            "--no-auth",
            "--field",
            "description",
        ],
    )
    .await;
    assert_eq!(all, filtered);

    // Exercise the UI's native watch/unwatch writes, including a subtask's
    // full-card projection. Untouched cards omit watchers; unwatching keeps [].
    let subtask_id = format!("export-watched-subtask-{nonce}");
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut session = ddp::Session::login(&server, &token).await;
        session
            .call(
                "/cards/insert",
                json!([{
                    "_id":subtask_id, "boardId":b, "listId":l, "swimlaneId":s,
                    "parentId":c, "title":"Watched subtask", "userId":user.user_id,
                }]),
            )
            .await;
        session.close().await;
    })
    .await
    .expect("subtask export fixture timed out");
    for watching in [None, Some(true), Some(false)] {
        if let Some(watching) = watching {
            tokio::time::timeout(std::time::Duration::from_secs(30), async {
                let mut session = ddp::Session::login(&server, &token).await;
                for id in [c.as_str(), subtask_id.as_str()] {
                    let modifier = if watching {
                        json!({"$addToSet":{"watchers":user.user_id}})
                    } else {
                        json!({"$pull":{"watchers":user.user_id}})
                    };
                    assert_eq!(
                        session
                            .call("/cards/update", json!([{"_id":id}, modifier]))
                            .await,
                        json!(1)
                    );
                }
                session.close().await;
            })
            .await
            .expect("card watch export fixture timed out");
        }
        let stored = mongo(
            "print(JSON.stringify(db.getSiblingDB('wekan').cards.find({_id:{$in:p.ids}},{_id:1,watchers:1}).toArray()));",
            &json!({"ids":[c,subtask_id]}),
        );
        let expected = watching.map(|watching| {
            if watching {
                json!([user.user_id])
            } else {
                json!([])
            }
        });
        let bytes = raw(&app, &server, &["board", "export", b, "--format", "json"]).await;
        let exported: Value = serde_json::from_slice(&bytes).unwrap();
        for (id, collection) in [(c.as_str(), "cards"), (subtask_id.as_str(), "subtaskItems")] {
            let stored = stored
                .as_array()
                .unwrap()
                .iter()
                .find(|card| card["_id"] == id)
                .expect("watch fixture card must be persisted");
            assert_eq!(stored.get("watchers"), expected.as_ref());
            let exported = exported[collection]
                .as_array()
                .unwrap()
                .iter()
                .find(|card| card["_id"] == id)
                .expect("export must include the card's native projection");
            assert_eq!(exported.get("watchers"), stored.get("watchers"));
        }
    }

    let CommandSuccess::BoardCreated(empty) =
        execute_live_board(&app, &server, &["create", "--title", "Empty export"])
            .await
            .unwrap()
    else {
        panic!("empty")
    };
    for format in [
        "json",
        "github",
        "gitlab",
        "gitea",
        "forgejo",
        "deck",
        "openproject",
        "trello",
        "jira",
        "asana",
        "zenkit",
        "kanboard",
    ] {
        let out = raw(
            &app,
            &server,
            &[
                "board",
                "export",
                &empty.board_id,
                "--format",
                format,
                "--auth-token-query",
            ],
        )
        .await;
        let value: Value = serde_json::from_slice(&out).unwrap();
        if matches!(format, "github" | "gitlab" | "gitea" | "forgejo") {
            assert_eq!(value, json!([]));
        }
    }
    let (imported_id, source) = import_and_export(
        &app,
        &server,
        &token,
        "csv",
        json!([
            ["title", "description", "stage"],
            ["Imported card", "Export import metadata", "Todo"]
        ]),
    )
    .await;
    assert_eq!(source, json!({"id":imported_id,"system":"CSV/TSV"}));

    let trello_id = format!("trello-export-{nonce}");
    for url in [None, Some("https://trello.com/b/export-regression")] {
        let mut trello = json!({
            "id":trello_id,"name":"Trello import metadata",
            "prefs":{"background":"blue","permissionLevel":"private"},
            "labels":[],"lists":[],"cards":[],"checklists":[],"actions":[],"members":[]
        });
        let mut expected_source = json!({"id":trello_id,"system":"Trello"});
        if let Some(url) = url {
            trello["url"] = json!(url);
            expected_source["url"] = json!(url);
        }
        let (_, source) = import_and_export(&app, &server, &token, "trello", trello).await;
        assert_eq!(source, expected_source);
    }

    let native = raw(
        &app,
        &server,
        &["board", "export", &empty.board_id, "--format", "json"],
    )
    .await;
    let native: Value = serde_json::from_slice(&native).unwrap();
    assert!(native.get("id").is_none());
    let (_, source) = import_and_export(&app, &server, &token, "wekan", native).await;
    assert_eq!(source, json!({"system":"Wekan"}));

    execute_live_board(&app, &server, &["delete", &empty.board_id, "--yes"])
        .await
        .unwrap();
    execute_live_board(&app, &server, &["delete", b, "--yes"])
        .await
        .unwrap();
}
