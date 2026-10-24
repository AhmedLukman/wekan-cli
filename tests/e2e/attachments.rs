use super::support::attachment_fixtures::{assert_stored_file_retained, fixture_command, seed};
use super::support::*;
use serde_json::json;
use wekan_cli::command_result::AttachmentScope;

const PNG_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Y9ZlZkAAAAASUVORK5CYII=";

async fn wait_for_attachment_validation_log(marker: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let logs = Command::new("docker")
            .args(["compose", "logs", "--no-color", "wekan"])
            .output()
            .expect("the deferred attachment validator log must be readable");
        assert!(logs.status.success());
        if String::from_utf8_lossy(&logs.stdout).contains(marker)
            || String::from_utf8_lossy(&logs.stderr).contains(marker)
        {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the native attachment validator did not report its outcome: {marker}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

async fn run(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    format: OutputFormat,
    args: &[&str],
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec![
        "wekan",
        "--server",
        server,
        "--output",
        match format {
            OutputFormat::Human => "human",
            OutputFormat::Json => "json",
            OutputFormat::Raw => "raw",
        },
        "attachment",
    ];
    argv.extend(args);
    let cli = Cli::try_parse_from(argv).unwrap();
    cli.command.validate_output(format)?;
    app.execute(cli.command).await
}

async fn verify_legacy_attachment_permissions(
    owner: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    board: &str,
    nonce: u128,
) {
    let http = reqwest::Client::new();
    let endpoint = format!(
        "{}/api/boards/{board}/attachments",
        server.trim_end_matches('/')
    );
    for token in [None, Some("invalid-attachment-token")] {
        let request = http.get(&endpoint);
        let request = if let Some(token) = token {
            request.bearer_auth(token)
        } else {
            request
        };
        let response = request.send().await.unwrap();
        assert_eq!(response.status().as_u16(), 401);
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("application/json")
        );
    }
    let missing = execute_live_board(
        owner,
        server,
        &["attachment", "list", "--board", "nonexistent"],
    )
    .await
    .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::NotFound);
    assert_eq!(missing.details().http_status, Some(404));
    let directory = TestDirectory::new("legacy-attachment-member");
    let member = App::with_profile_store(
        ServerSelection::new(Some(server.to_owned()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Legacy-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let username = format!("legacy_attachment_{nonce}");
    let CommandSuccess::Registration(user) = member
        .execute(
            Cli::try_parse_from([
                "wekan",
                "auth",
                "register",
                "--username",
                &username,
                "--password-stdin",
            ])
            .unwrap()
            .command,
        )
        .await
        .unwrap()
    else {
        panic!("member registration")
    };
    for role in [None, Some("normal"), Some("worker")] {
        if let Some(role) = role {
            execute_live_board(
                owner,
                server,
                &[
                    "member",
                    if role == "normal" { "add" } else { "update" },
                    &user.user_id,
                    "--board",
                    board,
                    "--role",
                    role,
                ],
            )
            .await
            .unwrap();
            if role == "normal" {
                wait_for_live_board_membership(&member, server, board).await;
            }
        }
        let result =
            execute_live_board(&member, server, &["attachment", "list", "--board", board]).await;
        if role == Some("normal") {
            let CommandSuccess::BoardAttachments(data) = result.unwrap() else {
                panic!("member listing")
            };
            assert_eq!(data.count, 2);
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.code(), ErrorCode::PermissionDenied);
            assert_eq!(error.details().http_status, Some(403));
        }
    }
    fixture_command(
        "mongodb",
        "mongosh",
        "db.getSiblingDB('wekan').users.deleteOne({_id:p.user_id}); print('{}');",
        &json!({"user_id":user.user_id}),
    );
}

async fn verify_raw_background_partial_failure(
    app: &super::support::live::LiveApp,
    server: &str,
    board: &str,
    directory: &Path,
    nonce: u128,
) {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let encoded = PNG_BASE64;
    let bytes = STANDARD.decode(encoded).unwrap();
    let source = directory.join("background.png");
    fs::write(&source, &bytes).unwrap();
    let user_id = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .2
        .clone();
    let before = fixture_command(
        "mongodb",
        "mongosh",
        "print(JSON.stringify(db.getSiblingDB('wekan').boards.findOne({_id:p.board},{backgroundImageId:1,backgroundImageURL:1})));",
        &json!({"board":board}),
    );
    let mut saved_uploads = Vec::new();
    for explicit_type in [false, true] {
        // Stay within v12.57's 30-character filename cap so its unrelated
        // rename/deferred-validator path does not alter this MIME fixture.
        let name = format!("bg-{nonce}-{explicit_type}.png");
        let mut args = vec![
            "upload-background",
            source.to_str().unwrap(),
            "--board",
            board,
            "--file-name",
            &name,
            "--transport",
            "raw",
        ];
        if explicit_type {
            args.extend(["--file-type", "image/png"]);
        }
        // W12-006: the native route saves the file before the background helper fails.
        let error = run(app, server, OutputFormat::Json, &args)
            .await
            .unwrap_err();
        assert_eq!(
            (error.code(), error.details().http_status),
            (ErrorCode::ServerError, Some(401)),
            "raw background explicit MIME={explicit_type}: {error:?}"
        );
        assert!(
            error
                .details()
                .server_error
                .as_deref()
                .is_some_and(|message| message.contains("isBoardAdmin is not a function")),
            "raw background explicit MIME={explicit_type}: {error:?}"
        );
        assert_eq!(error.details().mutation_attempted, Some(true));
        assert_eq!(error.details().retry_safe, Some(false));
        assert!(!error.message().contains("logout"));
        assert!(!error.message().contains("log in again"));
        // The raw branch's outer catch misclassifies the helper error as 401.
        // The exact credential used for the failed upload remains valid.
        let CommandSuccess::UserCurrent(current) = execute_live_user(app, server, &["current"])
            .await
            .unwrap_or_else(|error| {
                panic!("same credential after explicit MIME={explicit_type}: {error:?}")
            })
        else {
            panic!("current user")
        };
        assert_eq!(current.user_id, user_id);
        let saved = fixture_command(
            "mongodb",
            "mongosh",
            "const w=db.getSiblingDB('wekan');print(JSON.stringify({attachments:w.attachments.find({'meta.boardId':p.board,name:p.name}).toArray(),board:w.boards.findOne({_id:p.board},{backgroundImageId:1,backgroundImageURL:1})}));",
            &json!({"board":board,"name":name}),
        );
        assert_eq!(saved["board"], before);
        let attachments = saved["attachments"].as_array().unwrap();
        assert_eq!(attachments.len(), 1);
        let attachment = &attachments[0];
        assert_eq!(attachment["type"], "image/png");
        assert_eq!(attachment["isImage"], true);
        assert_eq!(attachment["size"], bytes.len());
        saved_uploads.push(attachment.clone());
    }
    for attachment in saved_uploads {
        // addFile awaits persistence, but onAfterUpload defers content validation
        // (attachments.server.js). Its final success log proves its file reads
        // have finished; deleting the bytes earlier races the native scanner.
        let path = attachment["versions"]["original"]["path"].as_str().unwrap();
        let marker = format!("Validation of uploaded file successful: file {path}");
        wait_for_attachment_validation_log(&marker).await;
        let persisted = fixture_command(
            "mongodb",
            "mongosh",
            "print(JSON.stringify(db.getSiblingDB('wekan').attachments.findOne({_id:p.id,'meta.boardId':p.board})));",
            &json!({"id":attachment["_id"],"board":board}),
        );
        assert_eq!(persisted["_id"], attachment["_id"]);
        assert_eq!(persisted["type"], "image/png");
        assert_eq!(persisted["isImage"], true);
        let attachment = persisted;
        // Check actual backing bytes, then remove only this failed upload's files.
        let removed = fixture_command(
            "wekan",
            "node",
            "const fs=require('fs'),path=require('path');const original=p.attachment.versions.original.path;if(!fs.readFileSync(original).equals(Buffer.from(p.encoded,'base64')))throw Error('background bytes changed');for(const v of Object.values(p.attachment.versions)){if(!v.path)continue;const file=path.resolve(v.path);if(!file.startsWith('/data/files/attachments/'))throw Error('unexpected attachment storage path');fs.rmSync(file,{force:true});}console.log('true');",
            &json!({"attachment":attachment,"encoded":encoded}),
        );
        assert_eq!(removed, true);
        let removed = fixture_command(
            "mongodb",
            "mongosh",
            "const w=db.getSiblingDB('wekan'),selector={_id:p.id,'meta.boardId':p.board,name:p.name};const removed=w.attachments.deleteOne(selector).deletedCount;print(JSON.stringify({removed,remaining:w.attachments.countDocuments(selector)}));",
            &json!({"id":attachment["_id"],"board":board,"name":attachment["name"]}),
        );
        assert_eq!(removed, json!({"removed":1,"remaining":0}));
    }
}

async fn verify_raw_upload_rename_validation_failure(
    app: &super::support::live::LiveApp,
    server: &str,
    scope: (&str, &str, &str, &str),
    directory: &Path,
    nonce: u128,
) {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let (board, swimlane, list, card) = scope;
    let source = directory.join("renamed-upload.png");
    fs::write(&source, STANDARD.decode(PNG_BASE64).unwrap()).unwrap();
    let name = format!("raw-card-{nonce}-renamed-upload.png");
    assert!(name.len() > 30);
    let CommandSuccess::AttachmentUploaded(upload) = run(
        app,
        server,
        OutputFormat::Json,
        &[
            "upload",
            source.to_str().unwrap(),
            "--board",
            board,
            "--swimlane-id",
            swimlane,
            "--list",
            list,
            "--card",
            card,
            "--file-name",
            &name,
            "--file-type",
            "image/png",
            "--transport",
            "raw",
        ],
    )
    .await
    .unwrap() else {
        panic!("raw upload")
    };
    assert_eq!(upload.file_name, name);
    assert!(upload.request_accepted);
    assert!(!upload.upload_confirmed);
    assert_eq!(upload.server_message, "Attachment uploaded successfully");
    let result = CommandSuccess::AttachmentUploaded(upload.clone());
    let output: serde_json::Value = serde_json::from_str(&wekan_cli::output::render_success(
        OutputFormat::Json,
        &result,
    ))
    .unwrap();
    assert_eq!(output["data"]["request_accepted"], true);
    assert_eq!(output["data"]["upload_confirmed"], false);
    assert!(output["data"].get("message").is_none());
    assert!(
        wekan_cli::output::render_success(OutputFormat::Human, &result)
            .contains("final availability is unconfirmed")
    );
    // W12-007: upload returns its new ID before the deferred validator rejects
    // the stale pre-rename path and removes the persisted attachment document.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let count = fixture_command(
            "mongodb",
            "mongosh",
            "print(JSON.stringify(db.getSiblingDB('wekan').attachments.countDocuments({_id:p.id,'meta.boardId':p.board})));",
            &json!({"id":upload.attachment_id,"board":board}),
        );
        if count == 0 {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the native validator did not remove renamed upload {}",
            upload.attachment_id
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let missing = run(
        app,
        server,
        OutputFormat::Json,
        &["get", &upload.attachment_id],
    )
    .await
    .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::NotFound);
    assert_eq!(missing.details().http_status, Some(404));
    wait_for_attachment_validation_log(&format!("-import-{name}'")).await;
    // Removal also misses the renamed physical file. Verify its bytes before
    // removing this upload's orphan by its returned immutable attachment ID.
    let orphan = fixture_command(
        "wekan",
        "node",
        "const fs=require('fs'),path=require('path'),root='/data/files/attachments';const prefix=p.id+'-original-';const files=fs.readdirSync(root).filter(name=>name.startsWith(prefix));const matching=files.filter(name=>fs.readFileSync(path.join(root,name)).equals(Buffer.from(p.encoded,'base64'))).length;for(const name of files)fs.rmSync(path.join(root,name));console.log(JSON.stringify({files:files.length,matching,remaining:fs.readdirSync(root).filter(name=>name.startsWith(prefix)).length}));",
        &json!({"id":upload.attachment_id,"encoded":PNG_BASE64}),
    );
    assert_eq!(orphan, json!({"files":1,"matching":1,"remaining":0}));
}

#[tokio::test]
#[ignore = "destructive: requires an isolated Wekan v12.57 stack and WEKAN_ATTACHMENT_E2E_URL"]
async fn complete_attachment_flow_matches_wekan_v12_57() {
    let server = env::var("WEKAN_ATTACHMENT_E2E_URL")
        .expect("set WEKAN_ATTACHMENT_E2E_URL to an isolated Wekan v12.57 stack");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let directory = TestDirectory::new("attachment-flow");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Attachments-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let username = format!("attachments_{nonce}");
    let CommandSuccess::Registration(user) = app
        .execute(
            Cli::try_parse_from([
                "wekan",
                "--server",
                &server,
                "auth",
                "register",
                "--username",
                &username,
                "--email",
                &format!("{username}@example.test"),
                "--password-stdin",
            ])
            .unwrap()
            .command,
        )
        .await
        .unwrap()
    else {
        panic!("registration")
    };
    let CommandSuccess::BoardCreated(board) =
        execute_live_board(&app, &server, &["create", "--title", "Attachments"])
            .await
            .unwrap()
    else {
        panic!("board")
    };
    let CommandSuccess::BoardAttachments(empty) = execute_live_board(
        &app,
        &server,
        &["attachment", "list", "--board", &board.board_id],
    )
    .await
    .unwrap() else {
        panic!("legacy empty listing")
    };
    assert_eq!(empty.count, 0);
    assert!(empty.attachments.is_empty());
    let CommandSuccess::ListCreated(list) = execute_live_list(
        &app,
        &server,
        &[
            "create",
            "--board",
            &board.board_id,
            "--title",
            "Todo",
            "--swimlane-id",
            &board.default_swimlane_id,
        ],
    )
    .await
    .unwrap() else {
        panic!("list")
    };
    let CommandSuccess::CardCreated(card) = execute_live_card(
        &app,
        &server,
        &[
            "create",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--swimlane-id",
            &board.default_swimlane_id,
            "--title",
            "Attachment target",
        ],
    )
    .await
    .unwrap() else {
        panic!("card")
    };
    let bytes = [0, 255, 13, 10].repeat(300_000);
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let fs_id = format!("attachmentfs{nonce}");
    let grid_id = format!("attachmentgrid{nonce}");
    let mut stored_files = Vec::new();
    for (id, storage, background) in [(&fs_id, "fs", false), (&grid_id, "gridfs", true)] {
        let stored = seed(
            &json!({"id":id,"storage":storage,"background":background,"path":format!("/data/files/attachments/{id}-original-fixture.bin"),"data":STANDARD.encode(&bytes),"user_id":user.user_id,"board_id":board.board_id,"swimlane_id":board.default_swimlane_id,"list_id":list.list_id,"card_id":card.card_id}),
        );
        stored_files.push((id.clone(), stored));
        let CommandSuccess::AttachmentShown(info) =
            run(&app, &server, OutputFormat::Json, &["get", id])
                .await
                .unwrap()
        else {
            panic!("metadata")
        };
        assert_eq!(info.storage_backend, storage);
        assert_eq!(info.file_size, bytes.len() as u64);
        assert_eq!(info.board_id.as_deref(), Some(board.board_id.as_str()));
        let thumbnail = info
            .versions
            .iter()
            .find(|v| v.version_name == "thumbnail")
            .unwrap();
        assert!(
            thumbnail.storage.is_none() && thumbnail.size.is_none() && thumbnail.r#type.is_none()
        );
        let saved = directory.path().join(format!("{storage}.bin"));
        let CommandSuccess::AttachmentDownloaded(result) = run(
            &app,
            &server,
            OutputFormat::Json,
            &["download", id, "--destination", saved.to_str().unwrap()],
        )
        .await
        .unwrap() else {
            panic!("download")
        };
        assert!(result.file_saved);
        assert_eq!(result.bytes_written, bytes.len() as u64);
        assert_eq!(fs::read(&saved).unwrap(), bytes);
        assert_eq!(
            run(
                &app,
                &server,
                OutputFormat::Json,
                &["download", id, "--destination", saved.to_str().unwrap()]
            )
            .await
            .unwrap_err()
            .code(),
            ErrorCode::FileIoError
        );
        fs::write(&saved, b"old").unwrap();
        run(
            &app,
            &server,
            OutputFormat::Json,
            &[
                "download",
                id,
                "--destination",
                saved.to_str().unwrap(),
                "--overwrite",
            ],
        )
        .await
        .unwrap();
        assert_eq!(fs::read(saved).unwrap(), bytes);
        let raw = run(
            &app,
            &server,
            OutputFormat::Raw,
            &["download", id, "--destination", "-"],
        )
        .await
        .unwrap();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        write_success(OutputFormat::Raw, raw, &mut stdout, &mut stderr)
            .await
            .unwrap();
        assert_eq!(stdout, bytes);
        assert!(stderr.is_empty());
    }
    let legacy_result = execute_live_board(
        &app,
        &server,
        &["attachment", "list", "--board", &board.board_id],
    )
    .await
    .unwrap();
    let CommandSuccess::BoardAttachments(links) = &legacy_result else {
        panic!("legacy attachment links")
    };
    assert_eq!(links.count, 2);
    assert_eq!(links.board_id, board.board_id);
    for (id, background) in [(&fs_id, false), (&grid_id, true)] {
        let link = links
            .attachments
            .iter()
            .find(|link| link.attachment_id == *id)
            .unwrap();
        assert_eq!(link.attachment_name, "fixture.bin");
        assert_eq!(link.attachment_type, "application/octet-stream");
        assert_eq!(link.board_id, board.board_id);
        assert!(link.url.is_some());
        assert!(link.url_download.is_some());
        if background {
            assert!(link.swimlane_id.is_none() && link.list_id.is_none() && link.card_id.is_none());
        } else {
            assert_eq!(
                link.swimlane_id.as_deref(),
                Some(board.default_swimlane_id.as_str())
            );
            assert_eq!(link.list_id.as_deref(), Some(list.list_id.as_str()));
            assert_eq!(link.card_id.as_deref(), Some(card.card_id.as_str()));
        }
    }
    for format in [OutputFormat::Human, OutputFormat::Json] {
        let rendered = wekan_cli::output::render_success(format, &legacy_result);
        assert!(rendered.contains("fixture.bin"));
        assert!(rendered.contains("url_download"));
    }
    verify_legacy_attachment_permissions(&app, &server, &board.board_id, nonce).await;
    verify_raw_background_partial_failure(&app, &server, &board.board_id, directory.path(), nonce)
        .await;
    verify_raw_upload_rename_validation_failure(
        &app,
        &server,
        (
            &board.board_id,
            &board.default_swimlane_id,
            &list.list_id,
            &card.card_id,
        ),
        directory.path(),
        nonce,
    )
    .await;
    for flags in [
        vec![],
        vec!["--swimlane-id", "nonexistent"],
        vec!["--swimlane-id", "nonexistent", "--list", "nonexistent"],
        vec![
            "--swimlane-id",
            "nonexistent",
            "--list",
            "nonexistent",
            "--card",
            "nonexistent",
        ],
    ] {
        let mut args = vec!["list", "--board", &board.board_id];
        args.extend(flags);
        let CommandSuccess::AttachmentList(collection) =
            run(&app, &server, OutputFormat::Json, &args).await.unwrap()
        else {
            panic!("list")
        };
        assert_eq!(collection.count, 2);
        assert_eq!(collection.effective_scope, AttachmentScope::Board);
        assert!(
            collection
                .attachments
                .iter()
                .any(|a| a.attachment_id == fs_id)
        );
        assert!(
            collection
                .attachments
                .iter()
                .any(|a| a.attachment_id == grid_id)
        );
    }
    let background = run(
        &app,
        &server,
        OutputFormat::Raw,
        &[
            "download-background",
            "--board",
            &board.board_id,
            "--destination",
            "-",
        ],
    )
    .await
    .unwrap();
    let mut stdout = Vec::new();
    write_success(OutputFormat::Raw, background, &mut stdout, &mut Vec::new())
        .await
        .unwrap();
    assert_eq!(stdout, bytes);
    let missing = run(&app, &server, OutputFormat::Json, &["get", "nonexistent"])
        .await
        .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::NotFound);
    assert_eq!(missing.details().http_status, Some(404));
    let source = directory.path().join("upload.bin");
    fs::write(&source, [0, 255, 10]).unwrap();
    let upload = vec![
        "upload",
        source.to_str().unwrap(),
        "--board",
        &board.board_id,
        "--swimlane-id",
        &board.default_swimlane_id,
        "--list",
        &list.list_id,
        "--card",
        &card.card_id,
    ];
    let background_upload = vec![
        "upload-background",
        source.to_str().unwrap(),
        "--board",
        &board.board_id,
    ];
    let copy = vec![
        "copy",
        &fs_id,
        "--target-board",
        &board.board_id,
        "--target-swimlane-id",
        &board.default_swimlane_id,
        "--target-list",
        &list.list_id,
        "--target-card",
        &card.card_id,
    ];
    let mut moving = copy.clone();
    moving[0] = "move";
    for args in [upload, background_upload, copy, moving] {
        let error = run(&app, &server, OutputFormat::Json, &args)
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::ServerError);
        assert_eq!(error.details().http_status, Some(408));
        let CommandSuccess::AttachmentList(collection) = run(
            &app,
            &server,
            OutputFormat::Json,
            &["list", "--board", &board.board_id],
        )
        .await
        .unwrap() else {
            panic!("readback")
        };
        assert_eq!(collection.count, 2);
    }
    for (index, (id, stored)) in stored_files.iter().enumerate() {
        let mut args = vec!["delete", id, "--yes"];
        if index == 1 {
            args.extend(["--board", &board.board_id]);
        }
        let deleted = run(&app, &server, OutputFormat::Json, &args).await.unwrap();
        if index == 0 {
            let CommandSuccess::AttachmentDeleted(result) = deleted else {
                panic!("legacy deletion")
            };
            assert_eq!(&result.attachment_id, id);
        } else {
            let CommandSuccess::AttachmentSoftDeleted(result) = deleted else {
                panic!("scoped deletion")
            };
            assert!(result.deleted);
        }
        assert_stored_file_retained(stored);
        // Soft deletion retains metadata and bytes; restoration recovers the original file.
        let CommandSuccess::AttachmentShown(info) =
            run(&app, &server, OutputFormat::Json, &["get", id])
                .await
                .unwrap()
        else {
            panic!("retained metadata")
        };
        assert_eq!(info.file_size, bytes.len() as u64);
        let CommandSuccess::DeletedAttachments(deleted) = run(
            &app,
            &server,
            OutputFormat::Json,
            &["list-deleted", "--board", &board.board_id],
        )
        .await
        .unwrap() else {
            panic!("deleted collection")
        };
        assert_eq!(deleted.count, 1);
        assert_eq!(deleted.attachments[0].attachment_id, *id);
        let CommandSuccess::AttachmentRestored(restored) = run(
            &app,
            &server,
            OutputFormat::Json,
            &["restore", id, "--board", &board.board_id],
        )
        .await
        .unwrap() else {
            panic!("restoration")
        };
        assert!(restored.restored);
        let raw = run(
            &app,
            &server,
            OutputFormat::Raw,
            &["download", id, "--destination", "-"],
        )
        .await
        .unwrap();
        let mut recovered = Vec::new();
        write_success(OutputFormat::Raw, raw, &mut recovered, &mut Vec::new())
            .await
            .unwrap();
        assert_eq!(recovered, bytes);
        let CommandSuccess::DeletedAttachments(deleted) = run(
            &app,
            &server,
            OutputFormat::Json,
            &["list-deleted", "--board", &board.board_id],
        )
        .await
        .unwrap() else {
            panic!("restored collection")
        };
        assert!(deleted.attachments.is_empty());
    }
    execute_live_board(&app, &server, &["delete", &board.board_id, "--yes"])
        .await
        .unwrap();
    // Account cleanup is fixture ownership, independent of whether this test's
    // account happened to be the isolated server's first global administrator.
    fixture_command(
        "mongodb",
        "mongosh",
        "const w=db.getSiblingDB('wekan'); w.users.deleteOne({_id:p.user_id}); print('{}');",
        &json!({"user_id":user.user_id}),
    );
}
