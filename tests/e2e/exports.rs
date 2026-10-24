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
fn workbook_text(bytes: &[u8]) -> String {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        if entry.name().unwrap().ends_with(".xml") {
            let mut part = String::new();
            entry.read_to_string(&mut part).unwrap();
            text.push_str(&part);
        }
    }
    text
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

async fn verify_custom_field_and_movement_exports(app: &LiveApp, server: &str, token: &str) {
    let CommandSuccess::BoardCreated(board) = execute_live_board(
        app,
        server,
        &["create", "--title", "Export metadata regressions"],
    )
    .await
    .unwrap() else {
        panic!("metadata board")
    };
    let b = &board.board_id;
    let CommandSuccess::CustomFieldCreated(field) = run(
        app,
        server,
        &[
            "custom-field",
            "create",
            "--board",
            b,
            "--name",
            "Score",
            "--type",
            "number",
        ],
    )
    .await
    .unwrap() else {
        panic!("custom-field")
    };
    for phase in ["defaults", "configured", "legacy"] {
        if phase == "configured" {
            // Exercise the same collection method as the custom-field settings UI.
            tokio::time::timeout(std::time::Duration::from_secs(30), async {
                let mut session = ddp::Session::login(server, token).await;
                session
                    .call(
                        "/customFields/update",
                        json!([{"_id":field.custom_field_id},{"$set":{
                            "readOnly":true,"adminOnly":true,"cardKinds":["story","task"],"sort":2.5
                        }}]),
                    )
                    .await;
                session.close().await;
            })
            .await
            .expect("custom-field settings fixture timed out");
        } else if phase == "legacy" {
            // A pre-feature record has no metadata; the native exporter does not add defaults.
            mongo(
                "db.getSiblingDB('wekan').customFields.updateOne({_id:p.id},{$unset:{readOnly:'',adminOnly:'',cardKinds:'',sort:''}});print('{}');",
                &json!({"id":field.custom_field_id}),
            );
        }
        let mut stored = mongo(
            "print(JSON.stringify(db.getSiblingDB('wekan').customFields.findOne({_id:p.id})));",
            &json!({"id":field.custom_field_id}),
        );
        match phase {
            "defaults" => {
                assert_eq!(stored["readOnly"], false);
                assert_eq!(stored["adminOnly"], false);
                assert!(stored.get("cardKinds").is_none());
                assert!(stored.get("sort").is_none());
            }
            "configured" => {
                assert_eq!(stored["readOnly"], true);
                assert_eq!(stored["adminOnly"], true);
                assert_eq!(stored["cardKinds"], json!(["story", "task"]));
                assert_eq!(stored["sort"], 2.5);
            }
            _ => {
                for property in ["readOnly", "adminOnly", "cardKinds", "sort"] {
                    assert!(stored.get(property).is_none());
                }
            }
        }
        let bytes = raw(app, server, &["board", "export", b, "--format", "json"]).await;
        let exported: Value = serde_json::from_slice(&bytes).unwrap();
        // Native JSON excludes boardIds but otherwise preserves the actual persisted definition.
        stored.as_object_mut().unwrap().remove("boardIds");
        assert_eq!(exported["customFields"], json!([stored]), "{phase}");
    }

    let mut lists = Vec::new();
    for title in ["Todo", "Review"] {
        let CommandSuccess::ListCreated(list) =
            execute_live_list(app, server, &["create", "--board", b, "--title", title])
                .await
                .unwrap()
        else {
            panic!("movement list")
        };
        lists.push(list.list_id);
    }
    let CommandSuccess::CardCreated(card) = execute_live_card(
        app,
        server,
        &[
            "create",
            "--board",
            b,
            "--list",
            &lists[0],
            "--swimlane-id",
            &board.default_swimlane_id,
            "--title",
            "Move for export",
        ],
    )
    .await
    .unwrap() else {
        panic!("movement card")
    };
    for (from, to, reason) in [
        (&lists[0], &lists[1], Some("  Ready for review  ")),
        (&lists[1], &lists[0], None),
    ] {
        let mut args = vec![
            "card",
            "update",
            &card.card_id,
            "--board",
            b,
            "--list",
            from,
            "--list-id",
            to,
        ];
        if let Some(reason) = reason {
            args.extend(["--move-reason", reason]);
        }
        run(app, server, &args).await.unwrap();
        let stored = mongo(
            "const w=db.getSiblingDB('wekan');print(JSON.stringify({card:w.cards.findOne({_id:p.card},{listId:1,lastMoveReason:1}),activity:w.activities.findOne({cardId:p.card,activityType:'moveCard',listId:p.list})}));",
            &json!({"card":card.card_id,"list":to}),
        );
        let expected = reason.unwrap_or("").trim();
        assert_eq!(stored["card"]["listId"], to.as_str());
        if expected.is_empty() {
            assert!(stored["card"].get("lastMoveReason").is_none());
        } else {
            assert_eq!(stored["card"]["lastMoveReason"], expected);
        }
        assert_eq!(stored["activity"]["moveReason"], expected);
        let bytes = raw(app, server, &["board", "export", b, "--format", "json"]).await;
        let exported: Value = serde_json::from_slice(&bytes).unwrap();
        let activities = exported["activities"].as_array().unwrap();
        let moved = activities
            .iter()
            .find(|activity| activity["_id"] == stored["activity"]["_id"])
            .expect("export must include the independently stored movement activity");
        let mut expected = stored["activity"].clone();
        expected.as_object_mut().unwrap().remove("boardId");
        assert_eq!(moved, &expected);
        assert!(activities.iter().any(|activity| {
            activity["activityType"] == "createCard" && activity.get("moveReason").is_none()
        }));
    }
    execute_live_board(app, server, &["delete", b, "--yes"])
        .await
        .unwrap();
    wait_for_live_board_absent(&reqwest::Client::new(), server, token, b).await;
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

async fn external_export(app: &LiveApp, server: &str, board: &str, format: &str) -> Value {
    let bytes = raw(app, server, &["board", "export", board, "--format", format]).await;
    serde_json::from_slice(&bytes).unwrap()
}

async fn gitlab_issue(app: &LiveApp, server: &str, board: &str) -> Value {
    let issues = external_export(app, server, board, "gitlab").await;
    let issues = issues
        .as_array()
        .expect("GitLab export must be an issue array");
    assert_eq!(issues.len(), 1);
    issues[0].clone()
}

async fn verify_external_comment_authors(
    app: &LiveApp,
    server: &str,
    board: &str,
    text: &str,
    author: Option<&str>,
) {
    for (format, pointer, text_pointer, identity_pointer) in [
        ("kanboard", "/tasks/0/comments/0", "/comment", "/username"),
        (
            "deck",
            "/stacks/0/cards/0/comments/0",
            "/message",
            "/actorId",
        ),
        (
            "openproject",
            "/_embedded/elements/0/_embedded/activities/0",
            "/comment/raw",
            "/_links/user/title",
        ),
        (
            "jira",
            "/issues/0/fields/comment/comments/0",
            "/body",
            "/author/name",
        ),
        ("asana", "/data/0/stories/0", "/text", "/created_by/name"),
        ("zenkit", "/items/0/comments/0", "/text", "/author"),
    ] {
        let document = external_export(app, server, board, format).await;
        let comment = document.pointer(pointer).expect("native retained comment");
        assert_eq!(comment.pointer(text_pointer).unwrap(), text, "{format}");
        match author {
            Some(name) => assert_eq!(comment.pointer(identity_pointer).unwrap(), name, "{format}"),
            None => {
                assert!(comment.pointer(identity_pointer).is_none(), "{format}");
                if matches!(format, "openproject" | "jira" | "asana") {
                    let (parent, _) = identity_pointer.rsplit_once('/').unwrap();
                    assert_eq!(comment.pointer(parent).unwrap(), &json!({}), "{format}");
                }
            }
        }
    }
}

async fn verify_external_planning_and_retained_comments(
    app: &LiveApp,
    server: &str,
    token: &str,
    nonce: u128,
) {
    let CommandSuccess::BoardCreated(board) = execute_live_board(
        app,
        server,
        &["create", "--title", "External export regressions"],
    )
    .await
    .unwrap() else {
        panic!("GitLab export board")
    };
    let b = &board.board_id;
    let CommandSuccess::ListCreated(list) =
        execute_live_list(app, server, &["create", "--board", b, "--title", "Todo"])
            .await
            .unwrap()
    else {
        panic!("GitLab export list")
    };
    let CommandSuccess::CardCreated(card) = execute_live_card(
        app,
        server,
        &[
            "create",
            "--board",
            b,
            "--list",
            &list.list_id,
            "--swimlane-id",
            &board.default_swimlane_id,
            "--title",
            "GitLab planning and comment history",
        ],
    )
    .await
    .unwrap() else {
        panic!("GitLab export card")
    };
    let canonical = ServerUrl::parse(server).unwrap().as_str().to_owned();
    let (_author_directory, author, author_id) =
        register_live_board_role(server, &canonical, token, b, nonce + 777, "normal").await;
    let CommandSuccess::UserCurrent(author_record) =
        execute_live_user(&author, server, &["current"])
            .await
            .unwrap()
    else {
        panic!("GitLab comment author")
    };
    let author_name = author_record
        .username
        .expect("registered author has a username");
    let text = "Comment retained after its author deletes their account";
    let CommandSuccess::CommentCreated(comment) = run(
        &author,
        server,
        &[
            "comment",
            "create",
            "--board",
            b,
            "--card",
            &card.card_id,
            "--text",
            text,
        ],
    )
    .await
    .unwrap() else {
        panic!("GitLab export comment")
    };
    let before = gitlab_issue(app, server, b).await;
    assert_eq!(before["notes"][0]["body"], text);
    assert_eq!(before["notes"][0]["author"]["username"], author_name);
    assert!(before.get("iteration").is_none());
    assert!(before.get("milestone").is_none());
    verify_external_comment_authors(app, server, b, text, Some(&author_name)).await;

    // The native account-deletion hook keeps comments and removes the user;
    // the GitLab formatter consequently emits an empty note-author object.
    let CommandSuccess::UserDeleted(deleted) =
        execute_live_user(app, server, &["delete", &author_id, "--yes"])
            .await
            .unwrap()
    else {
        panic!("GitLab comment-author deletion")
    };
    assert!(deleted.deleted);
    assert!(!deleted.deleted_current_user);
    let stored = mongo(
        "const w=db.getSiblingDB('wekan');print(JSON.stringify({user:w.users.findOne({_id:p.user}),comment:w.card_comments.findOne({_id:p.comment})}));",
        &json!({"user":author_id,"comment":comment.comment_id}),
    );
    assert!(stored["user"].is_null());
    assert_eq!(stored["comment"]["userId"], author_id);
    assert_eq!(stored["comment"]["text"], text);
    let retained = gitlab_issue(app, server, b).await;
    assert_eq!(retained["notes"][0]["body"], text);
    assert_eq!(retained["notes"][0]["author"], json!({}));
    assert_eq!(
        retained["notes"][0]["created_at"],
        before["notes"][0]["created_at"]
    );
    verify_external_comment_authors(app, server, b, text, None).await;
    run(
        &author,
        server,
        &["auth", "logout", "--local-only", "--yes"],
    )
    .await
    .unwrap();

    // Create and edit planning records through the same methods as the Scrum UI.
    let (sprint, release) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut session = ddp::Session::login(server, token).await;
        let sprint = session
            .call(
                "scrum.saveSprint",
                json!([b, null, {"name":"Undated sprint"}]),
            )
            .await;
        session
            .call(
                "scrum.updateCard",
                json!([b,card.card_id,{"sprintId":sprint["_id"]}]),
            )
            .await;
        session.close().await;
        let openproject = external_export(app, server, b, "openproject").await;
        let planning = &openproject["_embedded"]["sprints"][0];
        for field in ["description", "startDate", "finishDate"] {
            assert!(planning.get(field).is_none(), "{field}");
        }
        let mut session = ddp::Session::login(server, token).await;
        let sprint = session
            .call(
                "scrum.saveSprint",
                json!([b,sprint["_id"],{"goal":"Sprint goal"},sprint["revision"]]),
            )
            .await;
        session.close().await;
        let issue = gitlab_issue(app, server, b).await;
        assert_eq!(issue["iteration"]["title"], "Undated sprint");
        assert_eq!(issue["iteration"]["description"], "Sprint goal");
        assert_eq!(issue["iteration"]["state"], 1);
        assert!(issue["iteration"].get("start_date").is_none());
        assert!(issue["iteration"].get("due_date").is_none());

        let mut session = ddp::Session::login(server, token).await;
        let release = session
            .call(
                "scrum.saveRelease",
                json!([b,null,{"name":"Undated release"}]),
            )
            .await;
        session
            .call(
                "scrum.updateCard",
                json!([b,card.card_id,{"releaseId":release["_id"]}]),
            )
            .await;
        session.close().await;
        let openproject = external_export(app, server, b, "openproject").await;
        let planning = &openproject["_embedded"]["versions"][0];
        for field in ["description", "startDate", "endDate"] {
            assert!(planning.get(field).is_none(), "{field}");
        }
        assert_eq!(
            openproject["_embedded"]["sprints"][0]["description"]["raw"],
            "Sprint goal"
        );
        assert_eq!(
            openproject["_embedded"]["elements"][0]["_links"]["version"],
            json!({"href":"/api/v3/versions/1","title":"Undated release"})
        );
        let mut session = ddp::Session::login(server, token).await;
        let release = session
            .call(
                "scrum.saveRelease",
                json!([b,release["_id"],{"notes":"Release notes"},release["revision"]]),
            )
            .await;
        session.close().await;
        (sprint, release)
    })
    .await
    .expect("GitLab planning fixtures timed out");
    let undated = gitlab_issue(app, server, b).await;
    assert_eq!(undated["milestone"]["id"], 1);
    assert_eq!(undated["milestone"]["iid"], 1);
    assert_eq!(undated["milestone"]["title"], "Undated release");
    assert_eq!(undated["milestone"]["description"], "Release notes");
    assert_eq!(undated["milestone"]["state"], "active");
    for field in ["iteration", "milestone"] {
        assert!(undated[field].get("start_date").is_none());
        assert!(undated[field].get("due_date").is_none());
    }
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut session = ddp::Session::login(server, token).await;
        for (method, record) in [
            ("scrum.saveSprint", &sprint),
            ("scrum.saveRelease", &release),
        ] {
            session
                .call(
                    method,
                    json!([b,record["_id"],{
                    "plannedStart":"2030-01-02T00:00:00Z","plannedEnd":"2030-01-09T00:00:00Z"
                },record["revision"]]),
                )
                .await;
        }
        session.close().await;
    })
    .await
    .expect("dated GitLab planning fixtures timed out");
    let dated = gitlab_issue(app, server, b).await;
    for field in ["iteration", "milestone"] {
        assert_eq!(dated[field]["start_date"], "2030-01-02");
        assert_eq!(dated[field]["due_date"], "2030-01-09");
    }
    assert_eq!(dated["notes"][0]["author"], json!({}));
    let openproject = external_export(app, server, b, "openproject").await;
    for (collection, end, description) in [
        ("versions", "endDate", "Release notes"),
        ("sprints", "finishDate", "Sprint goal"),
    ] {
        let planning = &openproject["_embedded"][collection][0];
        assert_eq!(planning["startDate"], "2030-01-02");
        assert_eq!(planning[end], "2030-01-09");
        assert_eq!(planning["description"]["raw"], description);
    }
    execute_live_board(app, server, &["delete", b, "--yes"])
        .await
        .unwrap();
    wait_for_live_board_absent(&reqwest::Client::new(), server, token, b).await;
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v12.57 stack and WEKAN_EXPORT_E2E_URL"]
async fn complete_export_lifecycle_matches_wekan_v12_57() {
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
    for (case, (resource, id, scope, format)) in [
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
    ]
    .into_iter()
    .enumerate()
    {
        let (_probe_dir, probe, _) = register_live_board_role(
            &server,
            &canonical,
            &token,
            b,
            nonce + 100 + case as u128,
            "normal",
        )
        .await;
        let mut denied = vec![resource, "export", id, "--destination", "-"];
        denied.extend(scope.clone());
        if let Some(format) = format {
            denied.extend(["--format", format]);
        }
        let mut accepted = denied.clone();
        accepted.push("--auth-token-query");
        let cli = Cli::try_parse_from(["wekan", "--server", &server].into_iter().chain(accepted))
            .unwrap();
        probe.execute(cli.command).await.unwrap();
        // W12-002: attachment Bearer denial can disable a legitimate member.
        let error = run(&probe, &server, &denied).await.unwrap_err();
        assert_eq!(error.details().http_status, Some(403));
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
    let bytes = raw(
        &app,
        &server,
        &[
            "board",
            "export",
            b,
            "--format",
            "zip",
            "--auth-token-query",
        ],
    )
    .await;
    let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let document: Value = serde_json::from_reader(archive.by_name("wekan.json").unwrap()).unwrap();
    assert_eq!(document["_id"], *b);
    let attachment_name = "attachments/fixture.bin";
    let mut attachment_bytes = Vec::new();
    archive
        .by_name(attachment_name)
        .unwrap()
        .read_to_end(&mut attachment_bytes)
        .unwrap();
    assert_eq!(attachment_bytes, [0, 255, 10]);
    for index in 0..archive.len() {
        let mut bytes = Vec::new();
        archive
            .by_index(index)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
    }
    // Native ZIP exports omit attachment IDs from filenames, while both ZIP
    // import modes resolve entries by ID. Preserve this verified server failure.
    let roundtrip_dir = TestDirectory::new("native-zip-roundtrip");
    let roundtrip_file = roundtrip_dir.path().join("board.zip");
    std::fs::write(&roundtrip_file, &bytes).unwrap();
    // W12-005: the tracked importer starts outside the DDP identity context.
    for members_mode in [None, Some("me")] {
        let mut args = vec![
            "board",
            "import-zip",
            roundtrip_file.to_str().unwrap(),
            "--new-board",
        ];
        if let Some(mode) = members_mode {
            args.extend(["--members-mode", mode]);
        }
        let error = run(&app, &server, &args).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::ServerError);
        assert_eq!(error.details().http_status, Some(500));
        assert_eq!(
            error.details().server_error.as_deref(),
            Some("import-failed")
        );
    }
    let CommandSuccess::BoardCreated(scoped_board) = execute_live_board(
        &app,
        &server,
        &["create", "--title", "ZIP roundtrip target"],
    )
    .await
    .unwrap() else {
        panic!("ZIP target");
    };
    let CommandSuccess::ZipImported(scoped) = run(
        &app,
        &server,
        &[
            "board",
            "import-zip",
            roundtrip_file.to_str().unwrap(),
            "--board",
            &scoped_board.board_id,
        ],
    )
    .await
    .unwrap() else {
        panic!("scoped ZIP import");
    };
    assert_eq!(scoped.counts.cards, 1);
    assert_eq!(scoped.counts.attachments, 0);
    for id in [&scoped_board.board_id] {
        execute_live_board(&app, &server, &["delete", id, "--yes"])
            .await
            .unwrap();
    }
    // W11-063: GridFS downloads and ZIP streams retain bytes, but JSON reads a filesystem path.
    let grid_id = format!("export-grid-{nonce}");
    let grid_bytes = [0_u8, 255, 13, 10, 42];
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let _grid = seed(
        &json!({"id":grid_id,"storage":"gridfs","background":false,"data":STANDARD.encode(grid_bytes),"user_id":user.user_id,"board_id":b,"swimlane_id":s,"list_id":l,"card_id":c}),
    );
    let grid_download = roundtrip_dir.path().join("grid.bin");
    run(
        &app,
        &server,
        &[
            "attachment",
            "download",
            &grid_id,
            "--destination",
            grid_download.to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(&grid_download).unwrap(), grid_bytes);
    let grid_json = raw(&app, &server, &["board", "export", b, "--format", "json"]).await;
    let grid_json: Value = serde_json::from_slice(&grid_json).unwrap();
    let row = grid_json["attachments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["_id"] == grid_id)
        .unwrap();
    assert_eq!(row["file"], "");
    let grid_single = raw(
        &app,
        &server,
        &[
            "attachment",
            "export",
            &grid_id,
            "--board",
            b,
            "--auth-token-query",
        ],
    )
    .await;
    assert_eq!(
        serde_json::from_slice::<Value>(&grid_single).unwrap()["file"],
        Value::Null
    );
    let grid_zip = raw(&app, &server, &["board", "export", b, "--format", "zip"]).await;
    let mut grid_zip = zip::ZipArchive::new(Cursor::new(grid_zip)).unwrap();
    let mut found_grid = false;
    let mut attachment_names = std::collections::BTreeSet::new();
    for index in 0..grid_zip.len() {
        let mut entry = grid_zip.by_index(index).unwrap();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).unwrap();
        if entry.name().unwrap().starts_with("attachments/") {
            assert!(attachment_names.insert(entry.name().unwrap().into_owned()));
            if data == grid_bytes {
                found_grid = true;
            }
        }
    }
    assert_eq!(attachment_names.len(), 2);
    assert!(found_grid, "ZIP must independently retain the GridFS bytes");
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
    // W11-028: v12.57 starts the XLSX response, then destroys it after the null-list exception.
    assert_eq!(broken_card.code(), ErrorCode::TransportError);
    assert_eq!(broken_card.details().http_status, None);
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
    let private_locale = raw(
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
            "--auth-token-query",
            "--field",
            "board-info",
            "--field",
            "dates",
            "--date-format",
            "DD-MM-YYYY",
            "--timezone",
            "Pacific/Kiritimati",
        ],
    )
    .await;
    // W11-027 is fixed by native Excel date cells: both branches retain the same UTC serial.
    assert!(workbook_text(&private_locale).contains("<v>46376.5</v>"));
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
    // Public and private workbooks now use actual Excel date cells rather than differently formatted text.
    let public_locale = raw(
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
            "--no-auth",
            "--field",
            "board-info",
            "--field",
            "dates",
            "--date-format",
            "DD-MM-YYYY",
            "--timezone",
            "Pacific/Kiritimati",
        ],
    )
    .await;
    let public_text = workbook_text(&public_locale);
    assert!(public_text.contains("<v>46376.5</v>"));
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
    verify_custom_field_and_movement_exports(&app, &server, &token).await;
    verify_external_planning_and_retained_comments(&app, &server, &token, nonce).await;

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
