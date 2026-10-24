use super::support::attachment_fixtures::{fixture_command, seed};
use super::support::*;
use serde_json::{Value, json};

type LiveApp = App<CapturingStore, FixedPassword, FileProfileStore>;

async fn run(app: &LiveApp, server: &str, args: &[&str]) -> Result<Value, AppError> {
    let cli = Cli::try_parse_from(
        ["wekan", "--server", server, "--output", "json"]
            .into_iter()
            .chain(args.iter().copied()),
    )
    .unwrap();
    let result = app.execute(cli.command).await?;
    Ok(
        serde_json::from_str::<Value>(&wekan_cli::output::render_success(
            OutputFormat::Json,
            &result,
        ))
        .unwrap()["data"]
            .clone(),
    )
}

async fn board(app: &LiveApp, server: &str, name: &str) -> (String, String, String) {
    let board = run(app, server, &["board", "create", "--title", name])
        .await
        .unwrap();
    let b = board["board_id"].as_str().unwrap().to_owned();
    let s = board["default_swimlane_id"].as_str().unwrap().to_owned();
    let list = run(
        app,
        server,
        &[
            "list",
            "create",
            "--board",
            &b,
            "--title",
            "Todo",
            "--swimlane-id",
            &s,
        ],
    )
    .await
    .unwrap();
    (b, list["list_id"].as_str().unwrap().to_owned(), s)
}

async fn create(app: &LiveApp, server: &str, b: &str, l: &str, s: &str, cards: Value) -> Value {
    run(
        app,
        server,
        &[
            "card",
            "bulk",
            "create",
            "--board",
            b,
            "--list",
            l,
            "--swimlane-id",
            s,
            "--cards",
            &cards.to_string(),
        ],
    )
    .await
    .unwrap()
}

async fn verify_nullable_labels(
    app: &LiveApp,
    normal: &LiveApp,
    server: &str,
    b: &str,
    l: &str,
    s: &str,
    labels: (&str, &str),
) {
    let (green, red) = labels;
    let made = create(app, server, b, l, s, json!([{"title":"Nullable labels"}])).await;
    let card = made["results"][0]["card_id"].as_str().unwrap();
    let original = json!([null, green, null]);
    run(
        app,
        server,
        &[
            "card",
            "update",
            card,
            "--board",
            b,
            "--list",
            l,
            "--labels",
            &original.to_string(),
        ],
    )
    .await
    .unwrap();
    let before = run(
        app,
        server,
        &[
            "card", "label", "list", "--board", b, "--list", l, "--card", card,
        ],
    )
    .await
    .unwrap();
    assert_eq!(before["label_ids"], original);

    // The pinned merge preserves null and deduplicates all elements in stored order.
    let merged = run(
        normal,
        server,
        &[
            "card",
            "bulk",
            "labels",
            "--board",
            b,
            "--card",
            card,
            "--remove-label",
            green,
            "--add-label",
            red,
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        merged,
        json!({"board_id":b,"updated":[{"card_id":card,"label_ids":[null,red]}],"not_found":[]})
    );
    let stored = run(
        app,
        server,
        &["card", "get", card, "--board", b, "--list", l],
    )
    .await
    .unwrap();
    assert_eq!(stored["label_ids"], json!([null, red]));

    let error = run(
        normal,
        server,
        &[
            "card",
            "bulk",
            "labels",
            "--board",
            b,
            "--card",
            card,
            "--card",
            "missing",
            "--add-label",
            green,
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::BatchIncomplete);
    let rendered: Value =
        serde_json::from_str(&wekan_cli::output::render_error(OutputFormat::Json, &error)).unwrap();
    assert_eq!(
        rendered["error"]["details"]["batch_result"],
        json!({
            "operation":"card_bulk_labels",
            "data":{"board_id":b,"updated":[{"card_id":card,"label_ids":[null,red,green]}],"not_found":["missing"]}
        })
    );
    let stored = run(
        app,
        server,
        &[
            "card", "label", "list", "--board", b, "--list", l, "--card", card,
        ],
    )
    .await
    .unwrap();
    assert_eq!(stored["label_ids"], json!([null, red, green]));
    run(
        normal,
        server,
        &[
            "card", "bulk", "delete", "--board", b, "--card", card, "--yes",
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        run(
            app,
            server,
            &["card", "get", card, "--board", b, "--list", l]
        )
        .await
        .unwrap_err()
        .code(),
        ErrorCode::NotFound
    );
}

fn process(config: &Path, args: &[&str], input: Option<&str>) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wekan"));
    command
        .env("WEKAN_CONFIG_DIR", config)
        .env_remove("WEKAN_URL")
        .env_remove("WEKAN_PROFILE")
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    if let Some(input) = input {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
    }
    child.wait_with_output().unwrap()
}

fn verify_process_results(
    server: &str,
    username: &str,
    password: &str,
    b: &str,
    l: &str,
    s: &str,
    label: &str,
) {
    let config = TestDirectory::new("bulk-process");
    let _cleanup = NativeCredentialCleanup {
        config_directory: config.path().to_owned(),
        server: server.to_owned(),
    };
    let login = process(
        config.path(),
        &[
            "--server",
            server,
            "--output",
            "json",
            "auth",
            "login",
            "--username",
            username,
            "--password-stdin",
        ],
        Some(&format!("{password}\n")),
    );
    assert!(
        login.status.success(),
        "{}",
        String::from_utf8_lossy(&login.stderr)
    );
    for source in ["--cards-stdin", "--cards-file"] {
        let file = config.path().join("cards.json");
        fs::write(&file, r#"[{"title":"Process input"}]"#).unwrap();
        let mut args = vec![
            "--output",
            "json",
            "card",
            "bulk",
            "create",
            "--board",
            b,
            "--list",
            l,
            "--swimlane-id",
            s,
            source,
        ];
        if source == "--cards-file" {
            args.push(file.to_str().unwrap());
        }
        let created = process(
            config.path(),
            &args,
            if source == "--cards-stdin" {
                Some(r#"[{"title":"Process input"}]"#)
            } else {
                None
            },
        );
        assert!(
            created.status.success(),
            "{}",
            String::from_utf8_lossy(&created.stderr)
        );
        assert!(created.stderr.is_empty());
        let data: Value = serde_json::from_slice(&created.stdout).unwrap();
        let id = data["data"]["results"][0]["card_id"].as_str().unwrap();
        let set = process(
            config.path(),
            &[
                "--output", "json", "card", "update", id, "--board", b, "--list", l, "--labels",
                "[null]",
            ],
            None,
        );
        assert!(
            set.status.success(),
            "{}",
            String::from_utf8_lossy(&set.stderr)
        );
        let complete = process(
            config.path(),
            &[
                "--output",
                "json",
                "card",
                "bulk",
                "labels",
                "--board",
                b,
                "--card",
                id,
                "--add-label",
                label,
            ],
            None,
        );
        assert!(
            complete.status.success(),
            "{}",
            String::from_utf8_lossy(&complete.stderr)
        );
        assert!(complete.stderr.is_empty());
        let complete: Value = serde_json::from_slice(&complete.stdout).unwrap();
        assert_eq!(
            complete["data"]["updated"][0]["label_ids"],
            json!([null, label])
        );
        for format in ["json", "human"] {
            let incomplete = process(
                config.path(),
                &[
                    "--output",
                    format,
                    "card",
                    "bulk",
                    "labels",
                    "--board",
                    b,
                    "--card",
                    id,
                    "--card",
                    "missing",
                    "--add-label",
                    label,
                ],
                None,
            );
            assert_eq!(incomplete.status.code(), Some(5));
            assert!(incomplete.stdout.is_empty());
            if format == "json" {
                let value: Value = serde_json::from_slice(&incomplete.stderr).unwrap();
                assert_eq!(value["error"]["code"], "batch_incomplete");
                assert_eq!(
                    value["error"]["details"]["batch_result"]["data"]["updated"][0]["card_id"],
                    id
                );
                assert_eq!(
                    value["error"]["details"]["batch_result"]["data"]["updated"][0]["label_ids"],
                    json!([null, label])
                );
            } else {
                assert!(
                    String::from_utf8_lossy(&incomplete.stderr).contains("1 updated, 1 not found")
                );
                assert!(String::from_utf8_lossy(&incomplete.stderr).contains("null"));
            }
        }
        let denied = process(
            config.path(),
            &[
                "--output", "json", "card", "bulk", "delete", "--board", b, "--card", id,
            ],
            None,
        );
        assert_eq!(denied.status.code(), Some(2));
        assert!(denied.stdout.is_empty());
        let deleted = process(
            config.path(),
            &[
                "--output", "json", "card", "bulk", "delete", "--board", b, "--card", id, "--card",
                "missing", "--yes",
            ],
            None,
        );
        assert_eq!(deleted.status.code(), Some(5));
        assert!(deleted.stdout.is_empty());
        let value: Value = serde_json::from_slice(&deleted.stderr).unwrap();
        assert_eq!(
            value["error"]["details"]["batch_result"]["data"]["deleted"],
            json!([id])
        );
    }
}

async fn verify_bulk_delete_retains_attachments(
    app: &LiveApp,
    server: &str,
    b: &str,
    l: &str,
    s: &str,
    user_id: &str,
    nonce: u128,
) {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let created = create(
        app,
        server,
        b,
        l,
        s,
        json!([{"title":"Attachment cascade"}]),
    )
    .await;
    let card = created["results"][0]["card_id"].as_str().unwrap();
    let attachment_id = format!("bulkattachment{nonce}");
    let input = json!({
        "id":attachment_id,"storage":"fs","background":false,
        "path":format!("/data/files/attachments/{attachment_id}-original-fixture.bin"),
        "data":STANDARD.encode([0, 255, 13, 10, 42]),"user_id":user_id,
        "board_id":b,"swimlane_id":s,"list_id":l,"card_id":card
    });
    // Seed the native filesystem metadata shape and backing bytes.
    // Explicit raw upload and storage migration are covered in the attachment suite.
    let _stored = seed(&input);
    let before = run(app, server, &["attachment", "list", "--board", b])
        .await
        .unwrap();
    assert_eq!(before["count"], 1);
    assert_eq!(before["attachments"][0]["attachment_id"], attachment_id);
    assert_eq!(before["attachments"][0]["card_id"], card);
    assert_eq!(before["attachments"][0]["board_id"], b);

    let deleted = run(
        app,
        server,
        &[
            "card", "bulk", "delete", "--board", b, "--card", card, "--yes",
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        deleted,
        json!({"board_id":b,"deleted":[card],"not_found":[]})
    );
    let cards = run(app, server, &["card", "list", "--board", b, "--list", l])
        .await
        .unwrap();
    assert!(
        cards["cards"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["card_id"] != card)
    );

    // W11-046: cardRemover selects top-level cardId instead of meta.cardId.
    // The surviving bug is the attachment remaining visible after its card
    // is deleted. Backing-byte retention alone is normal recoverable deletion.
    let bytes = fixture_command(
        "wekan",
        "node",
        r#"
          const fs=require('fs');
          const exists=fs.existsSync(p.path);
          console.log(JSON.stringify({exists,data:exists ? fs.readFileSync(p.path).toString('base64') : null}));
        "#,
        &input,
    );
    assert_eq!(bytes, json!({"exists":true,"data":input["data"]}));
    let after = run(app, server, &["attachment", "list", "--board", b])
        .await
        .unwrap();
    assert_eq!(
        after, before,
        "bulk deletion unexpectedly cleaned up attachment metadata"
    );

    // Native attachment deletion is recoverable and must retain its file bytes.
    run(
        app,
        server,
        &["attachment", "delete", &attachment_id, "--yes"],
    )
    .await
    .unwrap();
    let retained = fixture_command(
        "wekan",
        "node",
        "console.log(JSON.stringify(require('fs').readFileSync(p.path).toString('base64')));",
        &input,
    );
    assert_eq!(retained, input["data"]);
}

#[tokio::test]
#[ignore = "destructive: requires an isolated Wekan v12.57 stack and WEKAN_BULK_CARD_E2E_URL"]
async fn complete_bulk_card_and_label_lifecycle_matches_wekan_v12_57() {
    let server = env::var("WEKAN_BULK_CARD_E2E_URL")
        .expect("set WEKAN_BULK_CARD_E2E_URL to an isolated stack");
    let canonical = ServerUrl::parse(&server).unwrap().as_str().to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let dir = TestDirectory::new("bulk-labels");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Bulk-{nonce}!"))),
        FileProfileStore::at(dir.path().to_owned()),
    );
    let user = format!("wekan_cli_bulk_{nonce}");
    let registration = run(
        &app,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &user,
            "--email",
            &format!("{user}@example.test"),
            "--password-stdin",
        ],
    )
    .await
    .unwrap();
    let admin = registration["user_id"].as_str().unwrap();
    let token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let (b, l, s) = board(&app, &server, "Bulk and labels").await;
    let (foreign_b, foreign_l, foreign_s) = board(&app, &server, "Foreign cards").await;
    let foreign = create(
        &app,
        &server,
        &foreign_b,
        &foreign_l,
        &foreign_s,
        json!([{"title":"Foreign"}]),
    )
    .await["results"][0]["card_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_normal_dir, normal, normal_id) =
        register_live_board_role(&server, &canonical, &token, &b, nonce, "normal").await;
    let (_readonly_dir, readonly, _) =
        register_live_board_role(&server, &canonical, &token, &b, nonce + 1, "readonly").await;
    let (_comment_dir, comment, _) =
        register_live_board_role(&server, &canonical, &token, &b, nonce + 2, "commentonly").await;
    let (_worker_dir, worker, _) =
        register_live_board_role(&server, &canonical, &token, &b, nonce + 3, "worker").await;

    let red = run(
        &app,
        &server,
        &[
            "board",
            "label",
            "add",
            "--board",
            &b,
            "--name",
            "  Bulk red  ",
            "--color",
            "red",
        ],
    )
    .await
    .unwrap();
    let red_id = red["label_id"].as_str().unwrap();
    let repeated = run(
        &app,
        &server,
        &[
            "board",
            "label",
            "add",
            "--board",
            &b,
            "--name",
            "  Bulk red  ",
            "--color",
            "red",
        ],
    )
    .await
    .unwrap();
    assert_ne!(repeated["label_id"], red["label_id"]);
    let reused = run(
        &app,
        &server,
        &[
            "board", "label", "add", "--board", &b, "--name", "Bulk red", "--color", "red",
        ],
    )
    .await
    .unwrap();
    assert_eq!(reused, red);
    let green = run(
        &app,
        &server,
        &[
            "board",
            "label",
            "add",
            "--board",
            &b,
            "--name",
            "Bulk green",
        ],
    )
    .await
    .unwrap();
    let green_id = green["label_id"].as_str().unwrap();
    let unnamed = run(&app, &server, &["board", "label", "add", "--board", &b])
        .await
        .unwrap();
    let labels = run(&app, &server, &["board", "label", "list", "--board", &b])
        .await
        .unwrap();
    assert!(
        labels["labels"]
            .as_array()
            .unwrap()
            .iter()
            .any(|label| label
                == &json!({
                    "label_id": red_id,
                    "name": "Bulk red",
                    "color": "red",
                    "due_at": null,
                    "shared_label_id": null
                }))
    );
    assert!(
        labels["labels"]
            .as_array()
            .unwrap()
            .iter()
            .any(|label| label["label_id"] == unnamed["label_id"]
                && label["name"] == ""
                && label["color"] == "green")
    );
    assert_eq!(
        run(
            &normal,
            &server,
            &["board", "label", "add", "--board", &b, "--name", "Denied"]
        )
        .await
        .unwrap_err()
        .code(),
        ErrorCode::PermissionDenied
    );

    // Supplying unknown card members triggers Wekan's GuestBleed account
    // protection even though it filters the IDs and returns created cards.
    let (_assignment_probe_dir, assignment_probe, _) =
        register_live_board_role(&server, &canonical, &token, &b, nonce + 10, "normal").await;
    let made = create(
        &assignment_probe,
        &server,
        &b,
        &l,
        &s,
        json!([
            {"members":[normal_id,admin,"missing",null,9],"assignees":normal_id},
            {"title":null,"description":null,"swimlaneId":s},
            {"title":"Archive me"}
        ]),
    )
    .await;
    let ids: Vec<String> = made["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["card_id"].as_str().unwrap().to_owned())
        .collect();
    let first = run(
        &app,
        &server,
        &["card", "get", &ids[0], "--board", &b, "--list", &l],
    )
    .await
    .unwrap();
    assert_eq!(first["title"], "");
    assert_eq!(first["members"], json!([normal_id, admin]));
    assert_eq!(first["assignees"], json!([normal_id]));
    let second = run(
        &app,
        &server,
        &["card", "get", &ids[1], "--board", &b, "--list", &l],
    )
    .await
    .unwrap();
    assert_eq!(second["title"], Value::Null);
    for denied_role in [&readonly, &comment, &worker] {
        assert_eq!(
            run(
                denied_role,
                &server,
                &[
                    "card",
                    "bulk",
                    "create",
                    "--board",
                    &b,
                    "--list",
                    &l,
                    "--swimlane-id",
                    &s,
                    "--cards",
                    "[{}]"
                ]
            )
            .await
            .unwrap_err()
            .code(),
            ErrorCode::PermissionDenied
        );
    }

    // The body parentId works here, unlike the single-create route's pinned defect.
    let child = create(
        &app,
        &server,
        &b,
        &l,
        &s,
        json!([{"title":"Child","parentId":ids[0]}]),
    )
    .await;
    let child_id = child["results"][0]["card_id"].as_str().unwrap();
    assert_eq!(
        run(
            &app,
            &server,
            &["card", "get", child_id, "--board", &b, "--list", &l]
        )
        .await
        .unwrap()["parent_id"],
        ids[0]
    );
    run(
        &app,
        &server,
        &[
            "card",
            "bulk",
            "labels",
            "--board",
            &b,
            "--card",
            &ids[0],
            "--card",
            &ids[1],
            "--add-label",
            green_id,
        ],
    )
    .await
    .unwrap();
    let merged = run(
        &normal,
        &server,
        &[
            "card",
            "bulk",
            "labels",
            "--board",
            &b,
            "--card",
            &ids[0],
            "--card",
            &ids[0],
            "--remove-label",
            green_id,
            "--add-label",
            green_id,
            "--add-label",
            red_id,
            "--add-label",
            red_id,
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        merged["updated"],
        json!([{"card_id":ids[0],"label_ids":[green_id,red_id]},{"card_id":ids[0],"label_ids":[green_id,red_id]}])
    );
    let stored = run(
        &app,
        &server,
        &[
            "card", "label", "list", "--board", &b, "--list", &l, "--card", &ids[0],
        ],
    )
    .await
    .unwrap();
    assert_eq!(stored["label_ids"], json!([green_id, red_id]));
    verify_nullable_labels(&app, &normal, &server, &b, &l, &s, (green_id, red_id)).await;
    run(
        &app,
        &server,
        &[
            "card",
            "bulk",
            "labels",
            "--board",
            &b,
            "--card",
            &ids[0],
            "--remove-label",
            red_id,
            "--remove-label",
            "unknown-removed-label",
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        run(
            &app,
            &server,
            &[
                "card", "label", "list", "--board", &b, "--list", &l, "--card", &ids[0]
            ]
        )
        .await
        .unwrap()["label_ids"],
        json!([green_id])
    );
    let error = run(
        &app,
        &server,
        &[
            "card",
            "bulk",
            "labels",
            "--board",
            &b,
            "--card",
            &ids[0],
            "--add-label",
            "bad-label",
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(error.details().http_status, Some(400));
    assert_eq!(
        error.details().invalid_label_ids.as_deref(),
        Some(["bad-label".to_owned()].as_slice())
    );
    run(
        &app,
        &server,
        &["card", "archive", &ids[2], "--board", &b, "--list", &l],
    )
    .await
    .unwrap();
    let error = run(
        &app,
        &server,
        &[
            "card",
            "bulk",
            "labels",
            "--board",
            &b,
            "--card",
            &ids[0],
            "--card",
            &ids[2],
            "--card",
            &foreign,
            "--card",
            "missing",
            "--add-label",
            red_id,
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::BatchIncomplete);
    let batch = serde_json::to_value(error.details().batch_result.as_ref().unwrap()).unwrap();
    assert_eq!(
        batch["data"]["not_found"],
        json!([ids[2], foreign, "missing"])
    );
    assert_eq!(batch["data"]["updated"].as_array().unwrap().len(), 1);
    for role in [&readonly, &comment] {
        assert_eq!(
            run(
                role,
                &server,
                &[
                    "card",
                    "bulk",
                    "labels",
                    "--board",
                    &b,
                    "--card",
                    &ids[0],
                    "--add-label",
                    red_id
                ]
            )
            .await
            .unwrap_err()
            .code(),
            ErrorCode::PermissionDenied
        );
        assert_eq!(
            run(
                role,
                &server,
                &[
                    "card", "bulk", "delete", "--board", &b, "--card", &ids[0], "--yes"
                ]
            )
            .await
            .unwrap_err()
            .code(),
            ErrorCode::PermissionDenied
        );
    }

    // A later missing swimlane fails after an earlier card has already been inserted.
    let before = run(
        &app,
        &server,
        &["card", "list", "--board", &b, "--list", &l],
    )
    .await
    .unwrap()["cards"]
        .as_array()
        .unwrap()
        .len();
    let partial = json!([{"title":"Created before failure","swimlaneId":s},{"title":"Invalid missing swimlane"}]);
    let error = run(
        &app,
        &server,
        &[
            "card",
            "bulk",
            "create",
            "--board",
            &b,
            "--list",
            &l,
            "--cards",
            &partial.to_string(),
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(error.details().http_status, Some(500));
    assert!(
        serde_json::to_value(error.details())
            .unwrap()
            .get("outcome_unknown")
            .is_none()
    );
    let after = run(
        &app,
        &server,
        &["card", "list", "--board", &b, "--list", &l],
    )
    .await
    .unwrap();
    assert_eq!(after["cards"].as_array().unwrap().len(), before + 1);
    assert!(
        after["cards"]
            .as_array()
            .unwrap()
            .iter()
            .any(|card| card["title"] == "Created before failure")
    );

    // Verify comment deletion independently from the batch acknowledgment.
    let comment_doc = run(
        &app,
        &server,
        &[
            "comment",
            "create",
            "--board",
            &b,
            "--card",
            &ids[1],
            "--text",
            "Deleted with card",
        ],
    )
    .await
    .unwrap();
    let error = run(
        &normal,
        &server,
        &[
            "card", "bulk", "delete", "--board", &b, "--card", &ids[1], "--card", &ids[1],
            "--card", &ids[2], "--card", &foreign, "--card", "missing", "--yes",
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::BatchIncomplete);
    let batch = serde_json::to_value(error.details().batch_result.as_ref().unwrap()).unwrap();
    assert_eq!(batch["data"]["deleted"], json!([ids[1], ids[2]]));
    assert_eq!(
        batch["data"]["not_found"],
        json!([ids[1], foreign, "missing"])
    );
    assert!(
        run(
            &app,
            &server,
            &["card", "get", &ids[1], "--board", &b, "--list", &l]
        )
        .await
        .is_err()
    );
    assert!(
        run(
            &app,
            &server,
            &[
                "comment",
                "get",
                comment_doc["comment_id"].as_str().unwrap(),
                "--board",
                &b,
                "--card",
                &ids[1]
            ]
        )
        .await
        .is_err()
    );
    assert!(
        run(
            &app,
            &server,
            &[
                "card", "get", &foreign, "--board", &foreign_b, "--list", &foreign_l
            ]
        )
        .await
        .is_ok()
    );
    let deleted = run(
        &app,
        &server,
        &[
            "card", "bulk", "delete", "--board", &b, "--card", child_id, "--yes",
        ],
    )
    .await
    .unwrap();
    assert_eq!(deleted["deleted"], json!([child_id]));

    verify_bulk_delete_retains_attachments(&app, &server, &b, &l, &s, admin, nonce).await;

    let http = reqwest::Client::new();
    assert_eq!(
        http.post(format!("{server}/api/boards/{b}/cards/labels"))
            .json(&json!({"cardIds":[ids[0]],"addLabelIds":[red_id]}))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    verify_process_results(
        &server,
        &user,
        &format!("Bulk-{nonce}!"),
        &b,
        &l,
        &s,
        red_id,
    );
    for target in [&b, &foreign_b] {
        run(&app, &server, &["board", "delete", target, "--yes"])
            .await
            .unwrap();
    }
}
