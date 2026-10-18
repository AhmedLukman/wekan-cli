use super::support::{attachment_fixtures::fixture_command, *};
use serde_json::{Value, json};
use wekan_cli::{
    command_result::TrelloImportResult,
    output::{render_error, render_success},
};

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
fn mongo(script: &str, input: Value) -> Value {
    fixture_command("mongodb", "mongosh", script, &input)
}
fn stored(board: &str) -> Value {
    mongo(
        "const w=db.getSiblingDB('wekan');print(JSON.stringify({board:w.boards.findOne({_id:p.board}),cards:w.cards.find({boardId:p.board}).toArray(),attachments:w.attachments.find({'meta.boardId':p.board}).toArray(),checklists:w.checklists.find({boardId:p.board}).toArray(),items:w.checklistItems.find({boardId:p.board}).toArray()}));",
        json!({"board":board}),
    )
}
fn document(name: &str, member: &str) -> Value {
    json!({"id":name,"name":name,"closed":false,"prefs":{"background":"blue","permissionLevel":"private"},
        "labels":[{"id":"label","name":"Urgent","color":"red"}],
        "lists":[{"id":"l","name":"Todo","pos":1,"closed":false}],
        "cards":[{"id":"c","name":"Ship","desc":"Details","idList":"l","pos":1,"closed":false,"dateLastActivity":"2026-10-17T00:00:00Z","labels":[],"idLabels":["label"],"idMembers":[member],"attachments":[]}],
        "checklists":[{"id":"check","idBoard":name,"idCard":"c","name":"Ready","pos":1,"checkItems":[{"name":"Verify","pos":1,"state":"complete"}]}],
        "actions":[],"members":[{"id":member,"username":member,"fullName":"Mapped member"}]})
}
fn archive(path: &Path, boards: &[Value], attachment: bool) {
    let mut writer = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    for (index, board) in boards.iter().enumerate() {
        writer
            .start_file(format!("board-{index}.json"), options)
            .unwrap();
        writer
            .write_all(&serde_json::to_vec(board).unwrap())
            .unwrap();
    }
    if attachment {
        writer
            .start_file(
                format!("{}/payload.bin", boards[0]["name"].as_str().unwrap()),
                options,
            )
            .unwrap();
        writer.write_all(b"Trello attachment\0\xff\r\n").unwrap();
    }
    writer.finish().unwrap();
}
fn imported(result: CommandSuccess) -> TrelloImportResult {
    let CommandSuccess::TrelloImported(result) = result else {
        panic!("Trello result")
    };

    result
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v11.06 stack and WEKAN_TRELLO_IMPORT_E2E_URL"]
async fn complete_trello_import_lifecycle_matches_wekan_v11_06() {
    let server = env::var("WEKAN_TRELLO_IMPORT_E2E_URL")
        .expect("set WEKAN_TRELLO_IMPORT_E2E_URL to a fresh isolated stack");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let directory = TestDirectory::new("trello-import");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Trello-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let username = format!("trello_{nonce}");
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
    assert_eq!(
        mongo(
            "print(JSON.stringify(db.getSiblingDB('wekan').users.findOne({_id:p.id}).isAdmin===true));",
            json!({"id":user.user_id}),
        ),
        true,
        "the first registered account must exercise site-administrator imports"
    );
    let member = format!("mapped_{nonce}");
    let mapping = serde_json::to_string(&json!({member.clone():user.user_id})).unwrap();
    let file = directory.path().join("board.json");
    let mut created = Vec::new();
    for auth in ["bearer", "x-auth-token", "meteor-cookie", "wekan-cookie"] {
        let name = format!("Trello {auth} {nonce}");
        fs::write(
            &file,
            serde_json::to_vec(&document(&name, &member)).unwrap(),
        )
        .unwrap();
        let result = imported(
            run(
                &app,
                &server,
                &[
                    "board",
                    "import-trello",
                    file.to_str().unwrap(),
                    "--format",
                    "json",
                    "--members-mapping",
                    &mapping,
                    "--auth",
                    auth,
                ],
            )
            .await
            .unwrap(),
        );
        assert_eq!(result.board_ids.len(), 1);
        assert!(result.errors.is_none());
        let saved = stored(&result.board_ids[0]);
        assert_eq!(saved["board"]["title"], name);
        assert!(
            saved["board"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["userId"] == user.user_id && m["isAdmin"] == true)
        );
        assert_eq!(saved["cards"][0]["members"], json!([user.user_id]));
        assert_eq!(saved["cards"][0]["title"], "Ship");
        assert_eq!(saved["checklists"][0]["title"], "Ready");
        assert_eq!(saved["items"][0]["isFinished"], true);
        let output = render_success(
            OutputFormat::Json,
            &CommandSuccess::TrelloImported(result.clone()),
        );
        assert!(
            serde_json::from_str::<Value>(&output).unwrap()["data"]
                .get("errors")
                .is_none()
        );
        created.extend(result.board_ids);
    }
    let zip = directory.path().join("boards.zip");
    let mut with_attachment = document(&format!("ZIP attachment {nonce}"), &username);
    with_attachment["cards"][0]["attachments"] = json!([{"id":"attachment","name":"payload.bin","fileName":"payload.bin","type":"application/octet-stream"}]);
    archive(
        &zip,
        &[
            with_attachment,
            document(&format!("ZIP second {nonce}"), &username),
        ],
        true,
    );
    let result = imported(
        run(
            &app,
            &server,
            &[
                "board",
                "import-trello",
                zip.to_str().unwrap(),
                "--format",
                "zip",
            ],
        )
        .await
        .unwrap(),
    );
    assert_eq!(result.board_ids.len(), 2);
    assert_eq!(result.errors, Some(vec![]));
    let saved = stored(&result.board_ids[0]);
    assert_eq!(saved["cards"][0]["title"], "Ship");
    assert_eq!(
        saved["attachments"].as_array().unwrap().len(),
        1,
        "ZIP attachment metadata: {saved}"
    );
    let attachment = &saved["attachments"][0];
    let bytes = fixture_command(
        "wekan",
        "node",
        "const fs=require('fs');console.log(JSON.stringify(Array.from(fs.readFileSync(p.path))));",
        &json!({"path":attachment["versions"]["original"]["path"]}),
    );
    assert_eq!(bytes, json!(b"Trello attachment\0\xff\r\n".to_vec()));
    created.extend(result.board_ids);

    // A failing later step leaves a board behind even though its ID is absent.
    let partial_name = format!("Partial Trello {nonce}");
    let mut bad = document(&partial_name, &username);
    bad["lists"] = Value::Null;
    for mixed in [true, false] {
        let mut boards = if mixed {
            vec![document(&format!("Mixed success {nonce}"), &username)]
        } else {
            vec![]
        };
        boards.push(bad.clone());
        boards.push(json!({"name":42,"cards":[]}));
        boards.push(json!({"name":true,"cards":[]}));
        archive(&zip, &boards, false);
        let error = run(
            &app,
            &server,
            &[
                "board",
                "import-trello",
                zip.to_str().unwrap(),
                "--format",
                "zip",
            ],
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::BatchIncomplete);
        let result = error.details().trello_import_result.as_ref().unwrap();
        assert_eq!(result.board_ids.len(), usize::from(mixed));
        let errors = result.errors.as_ref().unwrap();
        assert_eq!(errors.len(), 3);
        assert_eq!(serde_json::to_value(&errors[1].board).unwrap(), 42);
        assert_eq!(serde_json::to_value(&errors[2].board).unwrap(), true);
        assert!(
            serde_json::to_value(error.details())
                .unwrap()
                .get("outcome_unknown")
                .is_none()
        );
        assert!(render_error(OutputFormat::Human, &error).contains("Failed boards: 3"));
        created.extend(result.board_ids.clone());
    }
    fs::write(&file, serde_json::to_vec(&bad).unwrap()).unwrap();
    let error = run(
        &app,
        &server,
        &[
            "board",
            "import-trello",
            file.to_str().unwrap(),
            "--format",
            "json",
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(error.details().http_status, Some(400));
    assert!(
        serde_json::to_value(error.details())
            .unwrap()
            .get("outcome_unknown")
            .is_none()
    );
    let partials = mongo(
        "print(JSON.stringify(db.getSiblingDB('wekan').boards.find({title:{$regex:p.prefix}}).toArray()));",
        json!({"prefix":format!("^Partial Trello {nonce}")}),
    );
    assert_eq!(partials.as_array().unwrap().len(), 3);
    for board in partials.as_array().unwrap() {
        let saved = stored(board["_id"].as_str().unwrap());
        assert!(saved["cards"].as_array().unwrap().is_empty());
        created.push(board["_id"].as_str().unwrap().to_owned());
    }
    for body in [Value::Object(Default::default()), json!([])] {
        // Composite failed names reach strict decoding instead of being dropped.
        archive(&zip, &[json!({"name":body,"cards":[]})], false);
        let error = run(
            &app,
            &server,
            &[
                "board",
                "import-trello",
                zip.to_str().unwrap(),
                "--format",
                "zip",
            ],
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::ProtocolError);
    }
    archive(&zip, &[], false);
    let error = run(
        &app,
        &server,
        &[
            "board",
            "import-trello",
            zip.to_str().unwrap(),
            "--format",
            "zip",
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(error.details().http_status, Some(400));
    assert_eq!(
        error.details().server_error.as_deref(),
        Some("import-trello-zip-no-boards")
    );
    fs::write(&zip, b"not a ZIP").unwrap();
    let error = run(
        &app,
        &server,
        &[
            "board",
            "import-trello",
            zip.to_str().unwrap(),
            "--format",
            "zip",
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(error.details().http_status, Some(400));
    {
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
        writer
            .start_file("/absolute.bin", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"unsafe path").unwrap();
        writer.finish().unwrap();
    }
    let error = run(
        &app,
        &server,
        &[
            "board",
            "import-trello",
            zip.to_str().unwrap(),
            "--format",
            "zip",
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(error.details().http_status, Some(400));
    assert_eq!(
        error.details().server_error.as_deref(),
        Some("import-trello-zip-unsafe-path")
    );
    let saved_token = {
        let mut credential = app.credential_store().credential.lock().unwrap();
        let record = credential.as_mut().unwrap();
        std::mem::replace(&mut record.3, "invalid-trello-login-token".into())
    };
    fs::write(&file, b"{}").unwrap();
    let error = run(
        &app,
        &server,
        &[
            "board",
            "import-trello",
            file.to_str().unwrap(),
            "--format",
            "json",
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::AuthenticationRejected);
    assert_eq!(error.details().http_status, Some(401));
    assert_eq!(
        error.details().server_error.as_deref(),
        Some("error-notAuthorized")
    );
    assert!(
        serde_json::to_value(error.details())
            .unwrap()
            .get("outcome_unknown")
            .is_none()
    );
    app.credential_store()
        .credential
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .3 = saved_token;

    let ordinary_directory = TestDirectory::new("trello-import-ordinary");
    let ordinary_app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Ordinary-{nonce}!"))),
        FileProfileStore::at(ordinary_directory.path().to_owned()),
    );
    let ordinary_username = format!("trello_ordinary_{nonce}");
    let CommandSuccess::Registration(ordinary_user) = run(
        &ordinary_app,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &ordinary_username,
            "--email",
            &format!("{ordinary_username}@example.test"),
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
            json!({"id":ordinary_user.user_id}),
        ),
        false,
        "ordinary imports must not rely on site-administrator privileges"
    );
    for format in ["json", "zip"] {
        let name = format!("Ordinary Trello {format} {nonce}");
        let board = document(&name, &ordinary_username);
        let input = if format == "json" {
            fs::write(&file, serde_json::to_vec(&board).unwrap()).unwrap();
            &file
        } else {
            archive(&zip, &[board], false);
            &zip
        };
        let result = imported(
            run(
                &ordinary_app,
                &server,
                &[
                    "board",
                    "import-trello",
                    input.to_str().unwrap(),
                    "--format",
                    format,
                ],
            )
            .await
            .unwrap(),
        );
        assert_eq!(result.board_ids.len(), 1);
        assert_eq!(result.errors, (format == "zip").then(Vec::new));
        let board_id = &result.board_ids[0];
        let saved = stored(board_id);
        assert_eq!(saved["board"]["title"], name);
        assert!(
            saved["board"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |member| member["userId"] == ordinary_user.user_id && member["isAdmin"] == true
                )
        );
        assert_eq!(saved["cards"][0]["title"], "Ship");
        assert_eq!(saved["cards"][0]["members"], json!([ordinary_user.user_id]));
        assert_eq!(saved["checklists"][0]["title"], "Ready");
        assert_eq!(saved["items"][0]["isFinished"], true);
        run(
            &ordinary_app,
            &server,
            &["board", "delete", board_id, "--yes"],
        )
        .await
        .unwrap();
        assert!(stored(board_id)["board"].is_null());
    }
    for board in created {
        run(&app, &server, &["board", "delete", &board, "--yes"])
            .await
            .unwrap();
    }
}
