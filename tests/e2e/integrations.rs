use super::support::*;
use serde_json::{Value, json};

type LiveApp = App<CapturingStore, FixedPassword, FileProfileStore>;

async fn run(app: &LiveApp, server: &str, args: &[&str]) -> Result<Value, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output", "json"];
    argv.extend_from_slice(args);
    let result = app
        .execute(Cli::try_parse_from(argv).unwrap().command)
        .await?;
    let rendered = wekan_cli::output::render_success(OutputFormat::Json, &result);
    Ok(serde_json::from_str::<Value>(&rendered).unwrap()["data"].clone())
}

struct Target<'a> {
    app: &'a LiveApp,
    server: &'a str,
    board: &'a str,
    integration: &'a str,
}
impl Target<'_> {
    async fn command(&self, args: &[&str]) -> Result<Value, AppError> {
        let mut argv = vec!["integration"];
        argv.extend_from_slice(args);
        argv.extend(["--board", self.board]);
        run(self.app, self.server, &argv).await
    }
    async fn detail(&self) -> Value {
        self.command(&["get", self.integration]).await.unwrap()
    }
}

fn assert_stored_token(integration: &str, expected: Option<&str>) {
    // Integration reads deliberately exclude token. Inspect only this fixture;
    // never print the stored value, including on assertion failure.
    let condition = expected.map_or_else(
        || "d.token === undefined".to_owned(),
        |token| format!("d.token === {}", json!(token)),
    );
    let script = format!(
        "const d=db.getSiblingDB('wekan').integrations.findOne({{_id:{}}});if(!d||!({condition}))throw new Error('integration token did not match expected state');",
        json!(integration)
    );
    let result = Command::new("docker")
        .args([
            "compose", "exec", "-T", "mongodb", "mongosh", "--quiet", "--eval", &script,
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[tokio::test]
#[ignore = "destructive: requires an isolated Wekan v11.06 stack and WEKAN_INTEGRATION_E2E_URL"]
async fn complete_integration_lifecycle_matches_wekan_v11_06() {
    let server = env::var("WEKAN_INTEGRATION_E2E_URL")
        .expect("set WEKAN_INTEGRATION_E2E_URL to an isolated Wekan v11.06 stack");
    let canonical = ServerUrl::parse(&server).unwrap().as_str().to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let username = format!("wekan_cli_integration_{nonce}");
    let password = format!("Integration-{nonce}!");
    let directory = TestDirectory::new("integration-lifecycle");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password.clone())),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let registration = run(
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
    .unwrap();
    let user_id = registration["user_id"].as_str().unwrap();
    let token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let board = run(
        &app,
        &server,
        &["board", "create", "--title", "Integration lifecycle"],
    )
    .await
    .unwrap()["board_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let foreign = run(
        &app,
        &server,
        &["board", "create", "--title", "Other integration board"],
    )
    .await
    .unwrap()["board_id"]
        .as_str()
        .unwrap()
        .to_owned();
    // Register roles before integrations exist so no fixture generates deliveries.
    let (_reader_dir, reader, _) =
        register_live_board_role(&server, &canonical, &token, &board, nonce, "readonly").await;
    let (_normal_dir, normal, _) =
        register_live_board_role(&server, &canonical, &token, &board, nonce, "normal").await;
    let (_outsider_dir, outsider, outsider_id) =
        register_live_board_role(&server, &canonical, &token, &board, nonce + 1, "normal").await;
    run(
        &app,
        &server,
        &["board", "member", "remove", &outsider_id, "--board", &board],
    )
    .await
    .unwrap();
    let base = Target {
        app: &app,
        server: &server,
        board: &board,
        integration: "",
    };
    assert_eq!(
        base.command(&["list"]).await.unwrap(),
        json!({"board_id":board, "integrations":[]})
    );
    for invalid in [
        "",
        "not-a-url",
        "ftp://8.8.8.8",
        "http://127.0.0.1/hook",
        "http://192.168.1.1/hook",
    ] {
        let error = base
            .command(&["create", "--url", invalid])
            .await
            .unwrap_err();
        assert_eq!(error.details().http_status, Some(400), "{error:?}");
        assert_eq!(
            error.details().server_error.as_deref(),
            Some("invalid-webhook-url")
        );
    }
    assert!(
        base.command(&["list"]).await.unwrap()["integrations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    // Literal public IP avoids a DNS dependency. URL validation never sends HTTP.
    let created = base
        .command(&["create", "--url", "https://8.8.8.8/wekan-cli-test"])
        .await
        .unwrap();
    let integration = created["integration_id"].as_str().unwrap();
    let target = Target {
        integration,
        ..base
    };
    let defaults = target.detail().await;
    assert_eq!(defaults["enabled"], true);
    assert_eq!(defaults["integration_type"], "outgoing-webhooks");
    assert_eq!(defaults["activities"], json!(["all"]));
    assert_eq!(defaults["title"], Value::Null);
    assert_eq!(defaults["user_id"], user_id);
    assert_eq!(defaults["board_id"], board);
    assert!(defaults.get("token").is_none());
    assert_stored_token(integration, None);
    target
        .command(&["update", integration, "--enabled", "false"])
        .await
        .unwrap();
    assert_eq!(target.detail().await["enabled"], false);
    assert_eq!(
        target.command(&["list"]).await.unwrap()["integrations"],
        json!([target.detail().await])
    );
    let updated = target
        .command(&[
            "update",
            integration,
            "--title",
            "Hook title",
            "--url",
            "https://8.8.4.4/wekan-cli-test",
            "--token-stdin",
            "--activity",
            "createCard",
            "--activity",
            "createCard",
            "--activity",
            "future-event",
        ])
        .await
        .unwrap();
    assert_eq!(
        updated["submitted_fields"],
        json!(["title", "url", "token", "activities"])
    );
    assert!(updated.get("mutation_confirmed").is_none());
    let detail = target.detail().await;
    assert_eq!(detail["title"], "Hook title");
    assert_eq!(detail["url"], "https://8.8.4.4/wekan-cli-test");
    assert_eq!(
        detail["activities"],
        json!(["createCard", "createCard", "future-event"])
    );
    assert!(detail.get("token").is_none());
    assert_stored_token(integration, Some(&password));
    let added = target
        .command(&[
            "activity",
            "add",
            integration,
            "--activity",
            "future-event",
            "--activity",
            "addComment",
            "--activity",
            "addComment",
        ])
        .await
        .unwrap();
    assert_eq!(
        added["requested_activities"],
        json!(["future-event", "addComment", "addComment"])
    );
    assert_eq!(
        added["activities"],
        json!(["createCard", "createCard", "future-event", "addComment"])
    );
    assert_eq!(added["activities"], target.detail().await["activities"]);
    let removed = target
        .command(&[
            "activity",
            "remove",
            integration,
            "--activity",
            "createCard",
            "--activity",
            "absent",
        ])
        .await
        .unwrap();
    assert_eq!(removed["activities"], json!(["future-event", "addComment"]));
    assert_eq!(removed["activities"], target.detail().await["activities"]);
    target
        .command(&[
            "update",
            integration,
            "--clear-activities",
            "--clear-token",
            "--title",
            "",
        ])
        .await
        .unwrap();
    let cleared = target.detail().await;
    assert_eq!(cleared["activities"], json!([]));
    assert_eq!(cleared["title"], Value::Null);
    assert_stored_token(integration, None);
    target
        .command(&["update", integration, "--activity", "all"])
        .await
        .unwrap();
    assert_eq!(
        target
            .command(&["activity", "add", integration, "--activity", "createCard"])
            .await
            .unwrap()["activities"],
        json!(["all", "createCard"])
    );
    let unchanged = target.detail().await;
    let empty = target.command(&["update", integration]).await.unwrap();
    assert_eq!(empty["submitted_fields"], json!([]));
    assert_eq!(target.detail().await, unchanged);
    target
        .command(&["update", integration, "--enabled", "true"])
        .await
        .unwrap();
    assert_eq!(target.detail().await["enabled"], true);
    target
        .command(&["update", integration, "--enabled", "false"])
        .await
        .unwrap();

    // Field writes precede URL validation: rejection does not roll back the title.
    let before_url = target.detail().await["url"].clone();
    let error = target
        .command(&[
            "update",
            integration,
            "--title",
            "Saved before rejection",
            "--url",
            "http://127.0.0.1/hook",
        ])
        .await
        .unwrap_err();
    assert_eq!(error.details().http_status, Some(400));
    assert!(
        serde_json::to_value(error.details())
            .unwrap()
            .get("outcome_unknown")
            .is_none()
    );
    assert_eq!(target.detail().await["title"], "Saved before rejection");
    assert_eq!(target.detail().await["url"], before_url);

    for id in ["missing-integration", integration] {
        let scope = Target {
            board: &foreign,
            integration: id,
            ..target
        };
        let error = scope.command(&["get", id]).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::NotFound);
        assert_eq!(error.details().http_status, Some(200));
        for args in [
            vec!["update", id, "--title", "No match"],
            vec!["delete", id, "--yes"],
        ] {
            assert!(
                scope
                    .command(&args)
                    .await
                    .unwrap()
                    .get("mutation_confirmed")
                    .is_none()
            );
        }
        for action in ["add", "remove"] {
            let error = scope
                .command(&["activity", action, id, "--activity", "all"])
                .await
                .unwrap_err();
            assert_eq!(error.code(), ErrorCode::NotFound);
            assert_eq!(error.details().http_status, Some(200));
        }
    }
    assert_eq!(target.detail().await["title"], "Saved before rejection");
    for app in [&reader, &normal] {
        let scope = Target { app, ..target };
        assert_eq!(scope.detail().await["integration_id"], integration);
        assert_eq!(
            scope.command(&["list"]).await.unwrap()["integrations"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        for args in [
            vec!["create", "--url", "https://8.8.8.8/hook"],
            vec!["update", integration, "--title", "denied"],
            vec!["delete", integration, "--yes"],
            vec!["activity", "add", integration, "--activity", "all"],
            vec!["activity", "remove", integration, "--activity", "all"],
        ] {
            assert_eq!(
                scope.command(&args).await.unwrap_err().code(),
                ErrorCode::PermissionDenied
            );
        }
    }
    let scope = Target {
        app: &outsider,
        ..target
    };
    for args in [
        vec!["list"],
        vec!["get", integration],
        vec!["create", "--url", "https://8.8.8.8/hook"],
    ] {
        assert_eq!(
            scope.command(&args).await.unwrap_err().code(),
            ErrorCode::PermissionDenied
        );
    }
    let deleted = target
        .command(&["delete", integration, "--yes"])
        .await
        .unwrap();
    assert!(deleted.get("deletion_requested").is_none());
    assert!(deleted.get("mutation_confirmed").is_none());
    assert!(
        target.command(&["list"]).await.unwrap()["integrations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        target
            .command(&["get", integration])
            .await
            .unwrap_err()
            .code(),
        ErrorCode::NotFound
    );
    assert!(
        target
            .command(&["delete", integration, "--yes"])
            .await
            .unwrap()
            .get("mutation_confirmed")
            .is_none()
    );
    for board in [&board, &foreign] {
        run(&app, &server, &["board", "delete", board, "--yes"])
            .await
            .unwrap();
    }
}
