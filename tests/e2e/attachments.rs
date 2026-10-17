use super::support::attachment_fixtures::{assert_stored_file_deleted, fixture_command, seed};
use super::support::*;
use serde_json::json;
use wekan_cli::command_result::AttachmentScope;

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
                .starts_with("text/html")
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

#[tokio::test]
#[ignore = "destructive: requires an isolated Wekan v11.06 stack and WEKAN_ATTACHMENT_E2E_URL"]
async fn complete_attachment_flow_matches_wekan_v11_06() {
    let server = env::var("WEKAN_ATTACHMENT_E2E_URL")
        .expect("set WEKAN_ATTACHMENT_E2E_URL to an isolated Wekan v11.06 stack");
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
    for (id, stored) in &stored_files {
        let CommandSuccess::AttachmentDeleted(result) =
            run(&app, &server, OutputFormat::Json, &["delete", id, "--yes"])
                .await
                .unwrap()
        else {
            panic!("delete")
        };
        assert!(!result.mutation_confirmed);
        assert_eq!(
            run(&app, &server, OutputFormat::Json, &["get", id])
                .await
                .unwrap_err()
                .code(),
            ErrorCode::NotFound
        );
        assert_stored_file_deleted(stored);
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
