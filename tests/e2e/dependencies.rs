use super::support::*;
use serde_json::{Value, json};

type LiveApp = App<CapturingStore, FixedPassword, FileProfileStore>;

async fn run(app: &LiveApp, server: &str, args: &[&str]) -> Result<Value, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output", "json"];
    argv.extend_from_slice(args);
    let result = app
        .execute(Cli::try_parse_from(argv).unwrap().command)
        .await?;
    // Render only after the production strict DTO boundary has decoded the response.
    let rendered = wekan_cli::output::render_success(OutputFormat::Json, &result);
    Ok(serde_json::from_str::<Value>(&rendered).unwrap()["data"].clone())
}

struct Target<'a> {
    app: &'a LiveApp,
    server: &'a str,
    board: &'a str,
    list: &'a str,
    card: &'a str,
}
impl Target<'_> {
    async fn dependency(&self, args: &[&str]) -> Result<Value, AppError> {
        let mut argv = vec!["dependency"];
        argv.extend_from_slice(args);
        argv.extend(["--board", self.board, "--card", self.card]);
        run(self.app, self.server, &argv).await
    }
    async fn card(&self, args: &[&str]) -> Value {
        let mut argv = vec!["card"];
        argv.extend_from_slice(args);
        argv.extend(["--board", self.board, "--list", self.list]);
        run(self.app, self.server, &argv).await.unwrap()
    }
    async fn edges(&self) -> Vec<Value> {
        self.dependency(&["list"]).await.unwrap()["dependencies"]
            .as_array()
            .unwrap()
            .clone()
    }
    async fn assert_single_dependency(
        &self,
        target: &str,
        dependency_type: &str,
        color: &str,
        icon: &str,
    ) {
        // Read saved state independently of POST's metadata acknowledgment.
        assert_eq!(
            self.edges().await,
            vec![
                json!({"target_card_id":target, "dependency_type":dependency_type, "color":color, "icon":icon})
            ],
            "dependency listing must contain exactly the expected saved edge"
        );
        assert_eq!(
            self.card(&["get", self.card]).await["card_dependencies"],
            json!([{"card_id":target, "dependency_type":dependency_type, "color":color, "icon":icon}]),
            "raw card metadata must match the saved POST postcondition"
        );
    }
    async fn board_edges(&self) -> Vec<Value> {
        run(
            self.app,
            self.server,
            &["dependency", "list", "--board", self.board],
        )
        .await
        .unwrap()["dependencies"]
            .as_array()
            .unwrap()
            .clone()
    }
}

async fn new_board(app: &LiveApp, server: &str, title: &str) -> (String, String, String) {
    let board = run(app, server, &["board", "create", "--title", title])
        .await
        .unwrap();
    let id = board["board_id"].as_str().unwrap().to_owned();
    let swimlane = board["default_swimlane_id"].as_str().unwrap().to_owned();
    let list = run(
        app,
        server,
        &[
            "list",
            "create",
            "--board",
            &id,
            "--title",
            "Todo",
            "--swimlane-id",
            &swimlane,
        ],
    )
    .await
    .unwrap();
    (id, list["list_id"].as_str().unwrap().to_owned(), swimlane)
}
async fn new_card(
    app: &LiveApp,
    server: &str,
    board: &str,
    list: &str,
    swimlane: &str,
    title: &str,
) -> String {
    run(
        app,
        server,
        &[
            "card",
            "create",
            "--board",
            board,
            "--list",
            list,
            "--swimlane-id",
            swimlane,
            "--title",
            title,
        ],
    )
    .await
    .unwrap()["card_id"]
        .as_str()
        .unwrap()
        .to_owned()
}
fn assert_server_failure(error: &AppError, symbol: &str) {
    assert_eq!(error.code(), ErrorCode::ServerError);
    assert_eq!(error.details().http_status, Some(500));
    assert_eq!(error.details().server_error.as_deref(), Some(symbol));
}

fn seed_raw_dependency_type(card: &str, dependency_type: &str) {
    // v11.06 models/cards.js:535-571 defines an optional plain string, without
    // an enum constraint. Seed retained/imported data on this test's card only.
    let script = format!(
        "const r=db.getSiblingDB('wekan').cards.updateOne({{_id:{}}},{{$set:{{'cardDependencies.0.type':{}}}}});if(r.matchedCount!==1)throw new Error('fixture card not found');",
        json!(card),
        json!(dependency_type)
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
#[ignore = "destructive: requires an isolated Wekan v11.06 stack and WEKAN_DEPENDENCY_E2E_URL"]
async fn complete_dependency_lifecycle_matches_wekan_v11_06() {
    let server = env::var("WEKAN_DEPENDENCY_E2E_URL")
        .expect("set WEKAN_DEPENDENCY_E2E_URL to an isolated Wekan v11.06 stack");
    let canonical = ServerUrl::parse(&server).unwrap().as_str().to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let username = format!("wekan_cli_dependency_{nonce}");
    let directory = TestDirectory::new("dependency-lifecycle");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Dependency-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    run(
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
    let token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let http = reqwest::Client::new();
    let (board, list, swimlane) = new_board(&app, &server, "Dependencies").await;
    let source = new_card(&app, &server, &board, &list, &swimlane, "Source").await;
    let target = new_card(&app, &server, &board, &list, &swimlane, "Target").await;
    let scope = Target {
        app: &app,
        server: &server,
        board: &board,
        list: &list,
        card: &source,
    };
    assert!(scope.edges().await.is_empty());
    assert!(scope.board_edges().await.is_empty());

    let defaults = scope.dependency(&["create", &target]).await.unwrap();
    assert_eq!(
        defaults,
        json!({"board_id":board, "card_id":source, "target_card_id":target, "dependency_type":"related-to", "color":"#eb144c", "icon":"link"})
    );
    scope
        .assert_single_dependency(&target, "related-to", "#eb144c", "link")
        .await;
    let target_scope = Target {
        card: &target,
        ..scope
    };
    assert!(
        target_scope.edges().await.is_empty(),
        "a source edge is not mirrored onto the target"
    );
    let board_lines = scope.board_edges().await;
    assert_eq!(board_lines.len(), 1);
    assert_eq!(board_lines[0]["source_title"], "Source");
    assert_eq!(board_lines[0]["target_title"], "Target");
    assert_eq!(
        board_lines[0]["source_card_number"],
        scope.card(&["get", &source]).await["card_number"]
    );
    assert_eq!(
        board_lines[0]["target_card_number"],
        scope.card(&["get", &target]).await["card_number"]
    );

    for relation in [
        "related-to",
        "blocks",
        "is-blocked-by",
        "fixes",
        "is-fixed-by",
    ] {
        let created = scope
            .dependency(&[
                "create", &target, "--type", relation, "--color", "#2196f3", "--icon", "lock",
            ])
            .await
            .unwrap();
        assert_eq!(created["dependency_type"], relation);
        scope
            .assert_single_dependency(&target, relation, "#2196f3", "lock")
            .await;
        let updated = scope
            .dependency(&["update", &target, "--type", relation])
            .await
            .unwrap();
        assert_eq!(updated["submitted_fields"], json!(["type"]));
        assert_eq!(updated["mutation_confirmed"], false);
        assert_eq!(scope.edges().await[0]["dependency_type"], relation);
    }
    for args in [
        vec!["create", &target],
        vec![
            "create",
            &target,
            "--type",
            "",
            "--color",
            "",
            "--null-icon",
        ],
        vec![
            "create",
            &target,
            "--null-type",
            "--null-color",
            "--icon",
            "",
        ],
    ] {
        let merged = scope.dependency(&args).await.unwrap();
        assert_eq!(merged["dependency_type"], "is-fixed-by");
        assert_eq!(merged["color"], "#2196f3");
        assert_eq!(merged["icon"], "lock");
        scope
            .assert_single_dependency(&target, "is-fixed-by", "#2196f3", "lock")
            .await;
    }
    scope
        .dependency(&["update", &target, "--color", "orange"])
        .await
        .unwrap();
    let edge = &scope.edges().await[0];
    assert_eq!(edge["color"], "orange");
    assert_eq!(edge["dependency_type"], "is-fixed-by");
    assert_eq!(edge["icon"], "lock");

    for null in [false, true] {
        let args = if null {
            vec![
                "update",
                &target,
                "--null-type",
                "--null-color",
                "--null-icon",
            ]
        } else {
            vec!["update", &target, "--type", "", "--color", "", "--icon", ""]
        };
        scope.dependency(&args).await.unwrap();
        let raw_card = scope.card(&["get", &source]).await;
        let raw = &raw_card["card_dependencies"][0];
        // Collection2's SimpleSchema cleaning removes empty strings; explicit
        // nulls persist. Stable raw card output renders both as null.
        assert_eq!(raw["dependency_type"], Value::Null);
        assert_eq!(raw["color"], Value::Null);
        assert_eq!(raw["icon"], Value::Null);
        let wire = live_api_json(
            &http,
            &canonical,
            &token,
            reqwest::Method::GET,
            &format!("api/boards/{board}/lists/{list}/cards/{source}"),
            None,
        )
        .await;
        for field in ["type", "color", "icon"] {
            assert_eq!(
                wire["cardDependencies"][0].get(field),
                if null { Some(&Value::Null) } else { None }
            );
        }
        assert_eq!(
            scope.edges().await,
            vec![
                json!({"target_card_id":target, "dependency_type":"related-to", "color":"#eb144c", "icon":"link"})
            ]
        );
        let line = &scope.board_edges().await[0];
        assert_eq!(line["dependency_type"], "related-to");
        assert_eq!(line["color"], "#eb144c");
        assert_eq!(line["icon"], "link");
    }
    for raw_type in ["", "retained-type"] {
        seed_raw_dependency_type(&source, raw_type);
        assert_eq!(
            scope.card(&["get", &source]).await["card_dependencies"][0]["dependency_type"],
            raw_type
        );
        assert_eq!(scope.edges().await[0]["dependency_type"], "related-to");
    }
    assert_server_failure(
        &scope.dependency(&["update", &target]).await.unwrap_err(),
        "bad-request",
    );
    for invalid_target in [&source, "missing-target"] {
        assert_server_failure(
            &scope
                .dependency(&["create", invalid_target])
                .await
                .unwrap_err(),
            "bad-request",
        );
    }
    let (foreign_board, foreign_list, foreign_swimlane) =
        new_board(&app, &server, "Foreign dependency target").await;
    let foreign_card = new_card(
        &app,
        &server,
        &foreign_board,
        &foreign_list,
        &foreign_swimlane,
        "Foreign",
    )
    .await;
    assert_server_failure(
        &scope
            .dependency(&["create", &foreign_card])
            .await
            .unwrap_err(),
        "bad-request",
    );
    let missing = Target {
        card: "missing-source",
        ..scope
    };
    for args in [
        vec!["list"],
        vec!["create", &target],
        vec!["update", &target, "--color", "red"],
        vec!["delete", &target, "--yes"],
    ] {
        assert_server_failure(&missing.dependency(&args).await.unwrap_err(), "not-found");
    }
    let update = scope
        .dependency(&["update", "missing-edge", "--color", "red"])
        .await
        .unwrap();
    assert_eq!(update["mutation_confirmed"], false);
    let deleted = scope
        .dependency(&["delete", "missing-edge", "--yes"])
        .await
        .unwrap();
    assert_eq!(deleted["deletion_requested"], true);
    assert_eq!(deleted["mutation_confirmed"], false);
    assert_eq!(scope.edges().await.len(), 1);

    // Raw requests record the pinned invalid-type behavior and alias support.
    let path = format!("{canonical}api/boards/{board}/cards/{source}/dependencies");
    for verb in [reqwest::Method::POST, reqwest::Method::PUT] {
        let url = if verb == reqwest::Method::PUT {
            format!("{path}/{target}")
        } else {
            path.clone()
        };
        let response = http
            .request(verb, url)
            .bearer_auth(&token)
            .json(&json!({"cardId":target, "type":"future"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 500);
        assert_eq!(
            response.json::<Value>().await.unwrap()["error"],
            "bad-request"
        );
    }
    let response = http
        .post(&path)
        .bearer_auth(&token)
        .json(&json!({"dependsOn":target, "type":"blocks"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.json::<Value>().await.unwrap()["cardId"], target);
    scope
        .assert_single_dependency(&target, "blocks", "#eb144c", "link")
        .await;
    let anonymous = http.get(&path).send().await.unwrap();
    assert_eq!(anonymous.status(), 401);

    let (_reader_directory, reader_app, _) =
        register_live_board_role(&server, &canonical, &token, &board, nonce, "readonly").await;
    let reader = Target {
        app: &reader_app,
        ..scope
    };
    assert_eq!(reader.edges().await.len(), 1);
    assert_eq!(reader.board_edges().await.len(), 1);
    for args in [
        vec!["create", &target],
        vec!["update", &target, "--color", "red"],
        vec!["delete", &target, "--yes"],
    ] {
        let error = reader.dependency(&args).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::PermissionDenied);
        assert_eq!(error.details().http_status, Some(403));
    }

    scope.card(&["archive", &source]).await;
    assert_eq!(scope.edges().await.len(), 1);
    assert!(
        scope.board_edges().await.is_empty(),
        "board listing excludes archived sources"
    );
    scope
        .dependency(&["update", &target, "--color", "purple"])
        .await
        .unwrap();
    assert_eq!(scope.edges().await[0]["color"], "purple");
    scope.card(&["restore", &source]).await;
    assert_eq!(scope.board_edges().await.len(), 1);

    scope.card(&["delete", &target, "--yes"]).await;
    wait_for_live_card_absent(&http, &canonical, &token, &board, &list, &target).await;
    assert_eq!(
        scope.edges().await.len(),
        1,
        "target deletion leaves incoming dependency"
    );
    let dangling = &scope.board_edges().await[0];
    assert_eq!(dangling["target_title"], Value::Null);
    assert_eq!(dangling["target_card_number"], Value::Null);
    scope
        .dependency(&["delete", &target, "--yes"])
        .await
        .unwrap();
    assert!(scope.edges().await.is_empty());
    assert!(scope.board_edges().await.is_empty());
    for id in [&board, &foreign_board] {
        run(&app, &server, &["board", "delete", id, "--yes"])
            .await
            .unwrap();
    }
}
