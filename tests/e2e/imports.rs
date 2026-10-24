use super::support::attachment_fixtures::fixture_command;
use super::support::*;
use serde_json::{Value, json};
#[path = "../support/export_fixtures.rs"]
mod fixtures;

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
fn mongo(script: &str, input: &Value) -> Value {
    fixture_command("mongodb", "mongosh", script, input)
}
fn stored(board: &str) -> Value {
    mongo(
        "const w=db.getSiblingDB('wekan');const cards=w.cards.find({boardId:p.board}).toArray();const ids=cards.map(c=>c._id);print(JSON.stringify({board:w.boards.findOne({_id:p.board}),cards,lists:w.lists.find({boardId:p.board}).toArray(),swimlanes:w.swimlanes.find({boardId:p.board}).toArray(),checklists:w.checklists.find({cardId:{$in:ids}}).toArray(),items:w.checklistItems.find({cardId:{$in:ids}}).toArray(),comments:w.card_comments.find({boardId:p.board}).toArray()}));",
        &json!({"board":board}),
    )
}
fn archive(path: &Path, document: &Value, attachment: Option<(&str, &[u8])>) {
    let mut archive = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    archive.start_file("wekan.json", options).unwrap();
    archive
        .write_all(&serde_json::to_vec(document).unwrap())
        .unwrap();
    if let Some((name, bytes)) = attachment {
        archive.start_file(name, options).unwrap();
        archive.write_all(bytes).unwrap();
    }
    archive.finish().unwrap();
}
fn workbook(path: &Path) {
    let mut archive = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    for (name, text) in [
        (
            "[Content_Types].xml",
            r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Import" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>title</t></is></c><c r="B1" t="inlineStr"><is><t>stage</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>Ship</t></is></c><c r="B2" t="inlineStr"><is><t>Todo</t></is></c></row></sheetData></worksheet>"#,
        ),
    ] {
        archive
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(text.as_bytes()).unwrap();
    }
    archive.finish().unwrap();
}

async fn board(app: &LiveApp, server: &str) -> (String, String, String, String) {
    let CommandSuccess::BoardCreated(board) = run(
        app,
        server,
        &["board", "create", "--title", "Import destination"],
    )
    .await
    .unwrap() else {
        panic!("board")
    };
    let CommandSuccess::ListCreated(list) = run(
        app,
        server,
        &[
            "list",
            "create",
            "--board",
            &board.board_id,
            "--swimlane-id",
            &board.default_swimlane_id,
            "--title",
            "Existing",
        ],
    )
    .await
    .unwrap() else {
        panic!("list")
    };
    let CommandSuccess::CardCreated(card) = run(
        app,
        server,
        &[
            "card",
            "create",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--swimlane-id",
            &board.default_swimlane_id,
            "--title",
            "Existing card",
        ],
    )
    .await
    .unwrap() else {
        panic!("card")
    };
    (
        board.board_id,
        board.default_swimlane_id,
        list.list_id,
        card.card_id,
    )
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v12.57 stack and WEKAN_IMPORT_E2E_URL"]
async fn complete_import_lifecycle_matches_wekan_v12_57() {
    let server = env::var("WEKAN_IMPORT_E2E_URL")
        .expect("set WEKAN_IMPORT_E2E_URL to a fresh isolated stack");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let directory = TestDirectory::new("imports");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Imports-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let username = format!("imports_{nonce}");
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
    let (source_board, _, _, _) = board(&app, &server).await;
    let native = directory.path().join("native.json");
    run(
        &app,
        &server,
        &[
            "board",
            "export",
            &source_board,
            "--format",
            "json",
            "--destination",
            native.to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    let mut imported_boards = vec![source_board.clone()];
    let foreign_user = format!("foreign_import_{nonce}");
    let mut member_document: Value = serde_json::from_slice(&fs::read(&native).unwrap()).unwrap();
    member_document["users"] = json!([{
        "_id": foreign_user,
        "username": foreign_user,
        "profile": {"fullname": "Imported person", "initials": "IP"}
    }]);
    member_document["members"] = json!([{
        "userId": foreign_user,
        "isActive": true,
        "isAdmin": false
    }]);
    for card in member_document["cards"].as_array_mut().unwrap() {
        card["userId"] = json!(foreign_user);
        card["members"] = json!([foreign_user]);
    }
    let member_file = directory.path().join("member-mode.json");
    fs::write(&member_file, serde_json::to_vec(&member_document).unwrap()).unwrap();
    let attachment_auth = directory.path().join("attachment-auth.json");
    fs::write(
        &attachment_auth,
        "{\"token\":\"fixture-attachment-token\"}\r\n",
    )
    .unwrap();
    let error = run(
        &app,
        &server,
        &[
            "board",
            "import",
            member_file.to_str().unwrap(),
            "--members-mode",
            "me",
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::InvalidInput);
    let CommandSuccess::BoardImported(member_import) = run(
        &app,
        &server,
        &[
            "board",
            "import",
            member_file.to_str().unwrap(),
            "--source",
            "wekan",
            "--members-mode",
            "me",
            "--attachment-auth-file",
            attachment_auth.to_str().unwrap(),
        ],
    )
    .await
    .unwrap() else {
        panic!("member-mode import")
    };
    let saved = stored(&member_import.board_id);
    assert!(!saved["cards"].as_array().unwrap().is_empty());
    for card in saved["cards"].as_array().unwrap() {
        assert_eq!(card["members"], json!([user.user_id]));
    }
    assert_eq!(saved["board"]["members"].as_array().unwrap().len(), 1);
    assert_eq!(saved["board"]["members"][0]["userId"], user.user_id);
    assert_eq!(
        mongo(
            "print(JSON.stringify(db.getSiblingDB('wekan').users.countDocuments({_id:p.id})));",
            &json!({"id":foreign_user}),
        ),
        0,
        "member-mode me must not create a placeholder account"
    );
    imported_boards.push(member_import.board_id);
    let mapping = serde_json::to_string(&json!({"upstream":user.user_id})).unwrap();
    for source in [
        "native",
        "wekan",
        "trello",
        "csv",
        "jira",
        "kanboard",
        "excel",
        "deck",
        "openproject",
        "github",
        "gitlab",
        "gitea",
        "forgejo",
        "asana",
        "zenkit",
    ] {
        let file = directory.path().join(format!("{source}.input"));
        match source {
            "native"|"wekan" => {fs::copy(&native,&file).unwrap();},
            "csv" => fs::write(&file,"title,description,stage,members\nShip,Details,Todo,upstream unmapped\n").unwrap(),
            "excel" => workbook(&file),
            "trello" => fs::write(&file,serde_json::to_vec(&json!({"id":format!("trello{nonce}"),"name":"Trello import","prefs":{"background":"blue","permissionLevel":"private"},"labels":[],"lists":[{"id":"l","name":"Todo","pos":1,"closed":false}],"cards":[{"id":"c","name":"Ship","desc":"Details","idList":"l","pos":1,"closed":false,"labels":[],"idMembers":[]}],"checklists":[],"actions":[],"members":[]})).unwrap()).unwrap(),
            _ => fs::write(&file,serde_json::to_vec(&fixtures::external(source)).unwrap()).unwrap(),
        }
        let mut args = vec![
            "board",
            "import",
            file.to_str().unwrap(),
            "--members-mapping",
            &mapping,
        ];
        if source != "native" {
            args.extend(["--source", source]);
        }
        let result = run(&app, &server, &args)
            .await
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        let CommandSuccess::BoardImported(result) = result else {
            panic!("import result")
        };
        let saved = stored(&result.board_id);
        assert_eq!(saved["board"]["_id"], result.board_id, "{source}");
        assert!(
            saved["board"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["userId"] == user.user_id && m["isAdmin"] == true),
            "{source}"
        );
        assert!(!saved["cards"].as_array().unwrap().is_empty(), "{source}");
        if source == "kanboard" {
            assert_eq!(saved["board"]["title"], "Minimal");
        }
        if source == "csv" {
            assert_eq!(saved["cards"][0]["members"], json!([user.user_id]));
        }
        imported_boards.push(result.board_id);
    }
    // Repeating a valid import creates another board with a distinct ID.
    let CommandSuccess::BoardImported(duplicate) = run(
        &app,
        &server,
        &["board", "import", native.to_str().unwrap()],
    )
    .await
    .unwrap() else {
        panic!("duplicate")
    };
    assert!(!imported_boards.contains(&duplicate.board_id));
    imported_boards.push(duplicate.board_id);

    let mut doc = json!({"_format":"wekan-board-1.0.0","swimlanes":[{"_id":"s","title":"Imported lane"}],"lists":[{"_id":"l","title":"Imported list"}],"cards":[{"_id":"c","title":"Imported card","description":"Details","listId":"l","swimlaneId":"s"}],"checklists":[{"_id":"ch","cardId":"c","title":"Verify"}],"checklistItems":[{"_id":"i","cardId":"c","checklistId":"ch","title":"Done","isFinished":false}],"comments":[{"_id":"co","cardId":"c","text":"Imported comment"}],"attachments":[]});
    let zip = directory.path().join("populated.zip");
    archive(&zip, &doc, None);
    for scope in [
        "whole",
        "swimlane",
        "list",
        "card",
        "checklist",
        "combined",
        "fields",
    ] {
        let (b, s, l, c) = board(&app, &server).await;
        imported_boards.push(b.clone());
        let mut args = vec!["board", "import-zip", zip.to_str().unwrap(), "--board", &b];
        match scope {
            "swimlane" => args.extend(["--swimlane", &s]),
            "list" => args.extend(["--list", &l]),
            "card" => args.extend(["--card", &c]),
            "checklist" => args.extend(["--checklist", "ignored"]),
            "combined" => args.extend([
                "--card",
                &c,
                "--list",
                &l,
                "--swimlane",
                &s,
                "--checklist",
                "ignored",
            ]),
            "fields" => args.extend(["--field", "description", "--auth-token-query"]),
            _ => {}
        }
        let CommandSuccess::ZipImported(result) = run(&app, &server, &args).await.unwrap() else {
            panic!("ZIP")
        };
        assert_eq!(result.counts.cards, 1, "{scope}");
        assert_eq!(
            result.counts.swimlanes,
            if matches!(scope, "list" | "card" | "combined") {
                0
            } else {
                1
            },
            "{scope}"
        );
        assert_eq!(
            result.counts.lists,
            if matches!(scope, "card" | "combined") {
                0
            } else {
                1
            },
            "{scope}"
        );
        assert_eq!(
            result.counts.checklists,
            if scope == "fields" { 0 } else { 1 },
            "{scope}"
        );
        assert_eq!(
            result.counts.comments,
            if scope == "fields" { 0 } else { 1 },
            "{scope}"
        );
        assert_eq!(result.counts.attachments, 0);
        let saved = stored(&b);
        assert_eq!(saved["cards"].as_array().unwrap().len(), 2, "{scope}");
        assert_eq!(
            saved["lists"].as_array().unwrap().len() as u64,
            1 + result.counts.lists,
            "{scope}"
        );
        assert_eq!(
            saved["swimlanes"].as_array().unwrap().len() as u64,
            1 + result.counts.swimlanes,
            "{scope}"
        );
        assert_eq!(
            saved["checklists"].as_array().unwrap().len() as u64,
            result.counts.checklists,
            "{scope}"
        );
        assert_eq!(
            saved["items"].as_array().unwrap().len() as u64,
            result.counts.checklists,
            "{scope}"
        );
        assert_eq!(
            saved["comments"].as_array().unwrap().len() as u64,
            result.counts.comments,
            "{scope}"
        );
        let imported = saved["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["title"] == "Imported card")
            .unwrap();
        assert_eq!(imported["description"], "Details");
        if matches!(scope, "card" | "combined") {
            assert_eq!(imported["listId"], l);
            assert_eq!(imported["swimlaneId"], s);
        }
    }
    let (b, s, l, _) = board(&app, &server).await;
    imported_boards.push(b.clone());
    let ics = directory.path().join("events.ics");
    fs::write(&ics,"BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nSUMMARY:Launch\r\nDESCRIPTION:First\\nSecond\r\nDTSTART:20261220T120000Z\r\nDTEND:20261220T130000Z\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nSUMMARY:All day\r\nDTSTART;VALUE=DATE:20261221\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n").unwrap();
    let CommandSuccess::IcsImported(result) = run(
        &app,
        &server,
        &[
            "card",
            "import-ics",
            ics.to_str().unwrap(),
            "--board",
            &b,
            "--list",
            &l,
            "--swimlane-id",
            &s,
        ],
    )
    .await
    .unwrap() else {
        panic!("ICS")
    };
    assert_eq!(result.created, 2);
    assert_eq!(result.card_ids.len(), 2);
    let events = mongo(
        "print(JSON.stringify(db.getSiblingDB('wekan').cards.find({_id:{$in:p.ids}}).sort({sort:1}).toArray()));",
        &json!({"ids":result.card_ids}),
    );
    assert_eq!(events[0]["title"], "Launch");
    assert_eq!(events[0]["description"], "First\nSecond");
    assert_eq!(events[0]["startAt"], "2026-12-20T12:00:00.000Z");
    assert_eq!(events[0]["dueAt"], "2026-12-20T13:00:00.000Z");
    assert_eq!(events[1]["dueAt"], events[1]["startAt"]);
    for (target, list, status) in [(b.as_str(), "missing", 500), ("missing", l.as_str(), 401)] {
        let error = run(
            &app,
            &server,
            &[
                "card",
                "import-ics",
                ics.to_str().unwrap(),
                "--board",
                target,
                "--list",
                list,
                "--swimlane-id",
                &s,
            ],
        )
        .await
        .unwrap_err();
        assert_eq!(error.details().http_status, Some(status));
        if status == 401 {
            assert_eq!(error.code(), ErrorCode::PermissionDenied);
            assert_eq!(
                error.details().server_error.as_deref(),
                Some("not-authorized")
            );
            assert!(!error.message().contains("credential"));
        }
    }
    let error = run(
        &app,
        &server,
        &[
            "board",
            "import-zip",
            zip.to_str().unwrap(),
            "--board",
            "missing",
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(error.details().http_status, Some(404));
    let invalid = directory.path().join("invalid.json");
    fs::write(&invalid, "{}").unwrap();
    for source in [None, Some("wekan")] {
        let mut args = vec!["board", "import", invalid.to_str().unwrap()];
        if let Some(source) = source {
            args.extend(["--source", source]);
        }
        let error = run(&app, &server, &args).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::ServerError);
        assert_eq!(error.details().http_status, Some(500));
        assert_eq!(
            error.details().server_error.as_deref(),
            Some("Internal server error")
        );
        assert!(
            serde_json::to_value(error.details())
                .unwrap()
                .get("outcome_unknown")
                .is_none()
        );
    }

    // W11-032 is fixed: native ZIP imports finish, retain bytes, and clean their temporary archive.
    let filename = format!("importbytes{nonce}.bin");
    let bytes = [0_u8, 255, 13, 10, 42];
    doc["attachments"] = json!([{"_id":"a","cardId":"c","name":filename,"type":"application/octet-stream","size":bytes.len()}]);
    archive(
        &zip,
        &doc,
        Some((&format!("attachments/a-{filename}"), &bytes)),
    );
    let before = fixture_command(
        "wekan",
        "node",
        "console.log(JSON.stringify(require('fs').readdirSync('/tmp').filter(n=>n.startsWith('wekan-import-')&&n.endsWith('.zip'))));",
        &json!({}),
    );
    let CommandSuccess::ZipImported(imported) = run(
        &app,
        &server,
        &["board", "import-zip", zip.to_str().unwrap(), "--board", &b],
    )
    .await
    .unwrap() else {
        panic!("attachment import")
    };
    assert_eq!(imported.counts.attachments, 1);
    let attachment = mongo(
        "const w=db.getSiblingDB('wekan');const rows=w.attachments.find({'meta.boardId':p.board,name:p.name}).toArray();if(rows.length!==1)throw Error('expected one imported attachment');print(JSON.stringify({_id:rows[0]._id}));",
        &json!({"board":b,"name":filename}),
    );
    let destination = directory.path().join("imported.bin");
    run(
        &app,
        &server,
        &[
            "attachment",
            "download",
            attachment["_id"].as_str().unwrap(),
            "--destination",
            destination.to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(fs::read(destination).unwrap(), bytes);
    let cleanup = fixture_command(
        "wekan",
        "node",
        r#"const fs=require('fs');const additions=()=>fs.readdirSync('/tmp').filter(n=>n.startsWith('wekan-import-')&&n.endsWith('.zip')&&!p.before.includes(n));const deadline=Date.now()+5000;while(additions().length&&Date.now()<deadline)await new Promise(r=>setTimeout(r,100));console.log(JSON.stringify({remaining:additions().length}));"#,
        &json!({"before":before}),
    );
    assert_eq!(cleanup["remaining"], 0);

    let outsider_dir = TestDirectory::new("import-outsider");
    let outsider = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Other-{nonce}!"))),
        FileProfileStore::at(outsider_dir.path().to_owned()),
    );
    let CommandSuccess::Registration(ordinary_user) = run(
        &outsider,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &format!("outsider{nonce}"),
            "--email",
            &format!("outsider{nonce}@example.test"),
            "--password-stdin",
        ],
    )
    .await
    .unwrap() else {
        panic!("ordinary-user registration")
    };
    assert_eq!(
        mongo(
            "print(JSON.stringify(db.getSiblingDB('wekan').users.findOne({_id:p.id}).isAdmin===true));",
            &json!({"id":ordinary_user.user_id}),
        ),
        false
    );
    archive(&zip, &doc, None);
    for (args, status) in [
        (
            vec!["board", "import-zip", zip.to_str().unwrap(), "--board", &b],
            403,
        ),
        (
            vec![
                "card",
                "import-ics",
                ics.to_str().unwrap(),
                "--board",
                &b,
                "--list",
                &l,
                "--swimlane-id",
                &s,
            ],
            401,
        ),
    ] {
        let error = run(&outsider, &server, &args).await.unwrap_err();
        assert_eq!(error.details().http_status, Some(status));
        assert_eq!(error.code(), ErrorCode::PermissionDenied);
        assert!(!error.message().contains("credential"));
    }

    // Ordinary accounts can use both board-import endpoints and import into
    // boards they own. Check persisted resources independently of responses.
    for source in [None, Some("wekan")] {
        let mut args = vec!["board", "import", native.to_str().unwrap()];
        if let Some(source) = source {
            args.extend(["--source", source]);
        }
        let CommandSuccess::BoardImported(result) = run(&outsider, &server, &args).await.unwrap()
        else {
            panic!("ordinary board import")
        };
        let saved = stored(&result.board_id);
        assert!(!saved["cards"].as_array().unwrap().is_empty());
        assert!(
            saved["board"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |member| member["userId"] == ordinary_user.user_id && member["isAdmin"] == true
                )
        );
        run(
            &outsider,
            &server,
            &["board", "delete", &result.board_id, "--yes"],
        )
        .await
        .unwrap();
    }
    let (ordinary_board, ordinary_lane, ordinary_list, _) = board(&outsider, &server).await;
    doc["attachments"] = json!([]);
    archive(&zip, &doc, None);
    let CommandSuccess::ZipImported(result) = run(
        &outsider,
        &server,
        &[
            "board",
            "import-zip",
            zip.to_str().unwrap(),
            "--board",
            &ordinary_board,
        ],
    )
    .await
    .unwrap() else {
        panic!("ordinary ZIP import")
    };
    assert_eq!(result.counts.cards, 1);
    assert!(
        stored(&ordinary_board)["cards"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| card["title"] == "Imported card")
    );
    let CommandSuccess::IcsImported(result) = run(
        &outsider,
        &server,
        &[
            "card",
            "import-ics",
            ics.to_str().unwrap(),
            "--board",
            &ordinary_board,
            "--list",
            &ordinary_list,
            "--swimlane-id",
            &ordinary_lane,
        ],
    )
    .await
    .unwrap() else {
        panic!("ordinary ICS import")
    };
    assert_eq!(result.created, 2);
    assert_eq!(
        mongo(
            "print(JSON.stringify(db.getSiblingDB('wekan').cards.countDocuments({_id:{$in:p.ids},boardId:p.board,listId:p.list,swimlaneId:p.lane})));",
            &json!({"ids":result.card_ids,"board":ordinary_board,"list":ordinary_list,"lane":ordinary_lane}),
        ),
        2
    );
    run(
        &outsider,
        &server,
        &["board", "delete", &ordinary_board, "--yes"],
    )
    .await
    .unwrap();
    for b in imported_boards {
        run(&app, &server, &["board", "delete", &b, "--yes"])
            .await
            .unwrap();
    }
}
