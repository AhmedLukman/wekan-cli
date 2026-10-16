use serde_json::json;
use wekan_cli::command_result::{ChecklistDetail, ChecklistItemDetail};

use super::support::*;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct DdpChecklist {
    checklist_id: String,
    item_id: String,
    user_id: String,
}

fn create_ui_checklist(card: &str, token: &str, nonce: u128) -> DdpChecklist {
    let mut child = Command::new("docker")
        .args([
            "compose",
            "exec",
            "-T",
            "wekan",
            "node",
            "-e",
            include_str!("checklist_creators.cjs"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Docker Compose must be available for DDP checklist fixtures");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            serde_json::to_string(&json!({
                "card_id":card, "token":token,
                "checklist_id":format!("uichecklist{nonce}"),
                "item_id":format!("uiitem{nonce}")
            }))
            .unwrap()
            .as_bytes(),
        )
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "DDP checklist fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("DDP fixture must return scoped IDs")
}

fn mongo_json(script: &str) -> serde_json::Value {
    let result = Command::new("docker")
        .args([
            "compose", "exec", "-T", "mongodb", "mongosh", "--quiet", "--eval", script,
        ])
        .output()
        .expect("Docker Compose must be available for checklist fixtures");
    assert!(
        result.status.success(),
        "checklist fixture failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).expect("checklist fixture must return JSON")
}

fn item_activities(item_id: &str) -> serde_json::Value {
    let id = json!(item_id);
    mongo_json(&format!(
        "print(JSON.stringify(db.getSiblingDB('wekan').activities.find({{checklistItemId:{id}}}).toArray().map(a => a.activityType).sort()));"
    ))
}

struct Target<'a> {
    app: &'a App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &'a str,
    board: &'a str,
    card: &'a str,
}

impl Target<'_> {
    async fn run(&self, family: &str, args: &[&str]) -> Result<CommandSuccess, AppError> {
        let mut argv = vec!["wekan", "--server", self.server, "--output", "json", family];
        argv.extend_from_slice(args);
        argv.extend(["--board", self.board, "--card", self.card]);
        self.app
            .execute(Cli::try_parse_from(argv).unwrap().command)
            .await
    }

    async fn checklist(&self, args: &[&str]) -> Result<CommandSuccess, AppError> {
        self.run("checklist", args).await
    }

    async fn item(&self, checklist: &str, args: &[&str]) -> Result<CommandSuccess, AppError> {
        let mut argv = vec!["item"];
        argv.extend_from_slice(args);
        argv.extend(["--checklist", checklist]);
        self.run("checklist", &argv).await
    }

    async fn detail(&self, id: &str) -> ChecklistDetail {
        for _ in 0..40 {
            match self.checklist(&["get", id]).await {
                Ok(CommandSuccess::ChecklistShown(detail)) => return detail,
                Err(error)
                    if error.code() == ErrorCode::ServerError
                        && error.details().http_status == Some(500) => {}
                result => panic!("checklist must decode through its typed boundary: {result:?}"),
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
        panic!("created checklist was not visible within 10 seconds");
    }

    async fn item_detail(
        &self,
        checklist: &str,
        id: &str,
        title: &str,
        finished: bool,
    ) -> ChecklistItemDetail {
        for _ in 0..40 {
            match self.item(checklist, &["get", id]).await {
                Ok(CommandSuccess::ChecklistItemShown(detail))
                    if detail.title == title && detail.is_finished == finished =>
                {
                    return detail;
                }
                Ok(CommandSuccess::ChecklistItemShown(_)) => {}
                Err(error) if error.code() == ErrorCode::NotFound => {}
                result => panic!("item must decode through its typed boundary: {result:?}"),
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
        panic!("item `{id}` state was not visible within 10 seconds");
    }

    async fn collection(&self) -> Vec<wekan_cli::command_result::ChecklistSummary> {
        let CommandSuccess::ChecklistCollection(result) = self.checklist(&["list"]).await.unwrap()
        else {
            panic!("expected collection");
        };
        assert_eq!(result.board_id, self.board);
        assert_eq!(result.card_id, self.card);
        result.checklists
    }
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v11.06 stack and WEKAN_CHECKLIST_E2E_URL"]
async fn complete_checklist_lifecycle_matches_wekan_v11_06() {
    let server = env::var("WEKAN_CHECKLIST_E2E_URL")
        .expect("set WEKAN_CHECKLIST_E2E_URL to an isolated Wekan v11.06 stack");
    let canonical = ServerUrl::parse(&server).unwrap().as_str().to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let directory = TestDirectory::new("checklist-lifecycle");
    let username = format!("checklist_admin_{nonce}");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".to_owned()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Checklist-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let registration = Cli::try_parse_from([
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
    .unwrap();
    app.execute(registration.command)
        .await
        .expect("the checklist test account must register");
    let token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let CommandSuccess::BoardCreated(board) =
        execute_live_board(&app, &server, &["create", "--title", "Checklist lifecycle"])
            .await
            .unwrap()
    else {
        panic!("expected board");
    };
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
        panic!("expected list");
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
            "Checklist target",
        ],
    )
    .await
    .unwrap() else {
        panic!("expected card");
    };
    let target = Target {
        app: &app,
        server: &server,
        board: &board.board_id,
        card: &card.card_id,
    };
    assert!(target.collection().await.is_empty());

    let CommandSuccess::ChecklistCreated(default) = target.checklist(&["create"]).await.unwrap()
    else {
        panic!("expected default checklist");
    };
    let detail = target.detail(&default.checklist_id).await;
    assert_eq!(detail.title, "Checklist");
    assert_eq!(detail.sort, json!(0).as_number().unwrap().clone());
    assert_eq!(detail.board_id.as_deref(), Some(board.board_id.as_str()));
    assert_eq!(
        detail.user_id, None,
        "REST creation has no DDP creator identity"
    );
    assert!(detail.items.is_empty());
    assert_eq!(detail.hide_checked_checklist_items, None);
    assert_eq!(detail.hide_all_checklist_items, None);
    assert_eq!(detail.show_checklist_at_minicard, None);

    // UI collection inserts attach creator IDs that REST must map on reads.
    let ui = create_ui_checklist(&card.card_id, &token, nonce);
    let creator = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .2
        .clone();
    assert_eq!(ui.user_id, creator);
    let mut ui_checklist = target.detail(&ui.checklist_id).await;
    for _ in 0..40 {
        if ui_checklist.items.len() == 1 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        ui_checklist = target.detail(&ui.checklist_id).await;
    }
    assert_eq!(ui_checklist.user_id.as_deref(), Some(creator.as_str()));
    assert_eq!(ui_checklist.items.len(), 1);
    assert_eq!(ui_checklist.items[0].item_id, ui.item_id);
    let ui_item = target
        .item_detail(&ui.checklist_id, &ui.item_id, "UI item", false)
        .await;
    assert_eq!(ui_item.user_id.as_deref(), Some(creator.as_str()));
    for result in [
        CommandSuccess::ChecklistShown(ui_checklist),
        CommandSuccess::ChecklistItemShown(ui_item),
    ] {
        let json_output = wekan_cli::output::render_success(OutputFormat::Json, &result);
        let envelope: serde_json::Value = serde_json::from_str(&json_output).unwrap();
        assert_eq!(envelope["data"]["user_id"], creator);
        let human_output = wekan_cli::output::render_success(OutputFormat::Human, &result);
        assert!(human_output.contains(&format!("user_id: {creator}")));
    }
    // Remove only this fixture, so existing collection/order checks stay isolated.
    target
        .item(&ui.checklist_id, &["delete", &ui.item_id, "--yes"])
        .await
        .unwrap();
    target
        .checklist(&["delete", &ui.checklist_id, "--yes"])
        .await
        .unwrap();

    // These fields are exposed by GET but have no REST update operation. Seed only
    // this test's checklist, using the exact optional fields from models/checklists.js.
    let id = json!(default.checklist_id);
    let seeded = mongo_json(&format!(
        "const r=db.getSiblingDB('wekan').checklists.updateOne({{_id:{id}}},{{$set:{{hideCheckedChecklistItems:false,hideAllChecklistItems:true,showChecklistAtMinicard:false,finishedAt:new Date('2030-01-01T00:00:00Z')}}}}); print(JSON.stringify(r.matchedCount));"
    ));
    assert_eq!(seeded, 1);
    let mut detail = target.detail(&default.checklist_id).await;
    for _ in 0..40 {
        if detail.hide_all_checklist_items == Some(true) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        detail = target.detail(&default.checklist_id).await;
    }
    assert_eq!(detail.hide_checked_checklist_items, Some(false));
    assert_eq!(detail.hide_all_checklist_items, Some(true));
    assert_eq!(detail.show_checklist_at_minicard, Some(false));
    assert_eq!(
        detail.finished_at.as_deref(),
        Some("2030-01-01T00:00:00.000Z")
    );

    let CommandSuccess::ChecklistCreated(blank) =
        target.checklist(&["create", "--title", ""]).await.unwrap()
    else {
        panic!("expected blank-title checklist");
    };
    assert_eq!(target.detail(&blank.checklist_id).await.title, "Checklist");

    for (wire, count) in [
        ("null", 0),
        (r#""""#, 0),
        ("[]", 0),
        (r#""Only""#, 1),
        (r#"["First","Second"]"#, 2),
    ] {
        let CommandSuccess::ChecklistCreated(created) = target
            .checklist(&["create", "--items", wire])
            .await
            .unwrap()
        else {
            panic!("expected checklist");
        };
        let mut detail = target.detail(&created.checklist_id).await;
        for _ in 0..40 {
            if detail.items.len() == count {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            detail = target.detail(&created.checklist_id).await;
        }
        assert_eq!(detail.items.len(), count, "{wire}");
        for item in detail.items {
            let item = target
                .item_detail(&created.checklist_id, &item.item_id, &item.title, false)
                .await;
            assert_eq!(
                item.user_id, None,
                "REST item creation omits creator identity"
            );
        }
    }
    let CommandSuccess::ChecklistCreated(created) = target
        .checklist(&[
            "create",
            "--title",
            "  Release  ",
            "--item",
            "  Verify  ",
            "--item",
            "Ship",
        ])
        .await
        .unwrap()
    else {
        panic!("expected checklist");
    };
    let checklist = &created.checklist_id;
    let mut detail = target.detail(checklist).await;
    for _ in 0..40 {
        if detail.items.len() == 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        detail = target.detail(checklist).await;
    }
    assert_eq!(detail.title, "Release");
    assert_eq!(detail.items.len(), 2);
    let original = detail
        .items
        .iter()
        .find(|item| item.title == "Verify")
        .unwrap()
        .item_id
        .clone();
    let initial = target
        .item_detail(checklist, &original, "Verify", false)
        .await;
    assert_eq!(initial.sort, json!(0).as_number().unwrap().clone());
    assert!(initial.created_at.is_some());
    let activities_before = item_activities(&original);

    let mut max_sort = 1.0;
    for (flags, expected) in [
        (vec![], 2.0),
        (vec!["--sort", "-1.5"], -1.5),
        (vec!["--sort-json", r#""4.5""#], 4.5),
        (vec!["--sort-json", "null"], 0.0),
        (vec!["--sort-json", r#""""#], 0.0),
        (vec!["--sort-json", r#""Infinity""#], 5.5),
        (vec!["--sort-json", r#""NaN""#], 6.5),
        (vec!["--sort-json", r#""0x10""#], 16.0),
        (vec!["--sort-json", "1e309"], 17.0),
    ] {
        let mut args = vec!["create", "--title", "  Appended  "];
        args.extend(flags);
        let CommandSuccess::ChecklistItemCreated(item) =
            target.item(checklist, &args).await.unwrap()
        else {
            panic!("expected item");
        };
        let detail = target
            .item_detail(checklist, &item.item_id, "Appended", false)
            .await;
        assert_eq!(detail.sort.as_f64(), Some(expected));
        max_sort = f64::max(max_sort, expected);
    }
    assert_eq!(max_sort, 17.0);
    for (value, expected) in [
        ("true", true),
        (r#""TRUE""#, true),
        (r#"" true ""#, false),
        (r#""other""#, false),
        ("false", false),
    ] {
        let CommandSuccess::ChecklistItemUpdated(updated) = target
            .item(
                checklist,
                &[
                    "update",
                    &original,
                    "--is-finished-json",
                    value,
                    "--title",
                    "  Renamed  ",
                ],
            )
            .await
            .unwrap()
        else {
            panic!("expected update");
        };
        assert_eq!(
            serde_json::to_value(updated.submitted_fields).unwrap(),
            json!(["title", "is_finished"])
        );
        target
            .item_detail(checklist, &original, "Renamed", expected)
            .await;
    }
    let CommandSuccess::ChecklistItemUpdated(empty) = target
        .item(checklist, &["update", &original])
        .await
        .unwrap()
    else {
        panic!("expected empty update");
    };
    assert!(empty.submitted_fields.is_empty());
    target
        .item(checklist, &["update", &original, "--is-finished", "true"])
        .await
        .unwrap();
    target
        .item_detail(checklist, &original, "Renamed", true)
        .await;
    assert_eq!(target.detail(checklist).await.finished_at, None);
    assert_eq!(
        item_activities(&original),
        activities_before,
        "direct REST item updates bypass activity hooks"
    );

    // Completion is written before title validation, so an invalid title can fail
    // after saving the requested completion state.
    let error = target
        .item(
            checklist,
            &["update", &original, "--is-finished", "false", "--title", ""],
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ServerError);
    assert_eq!(error.details().http_status, Some(500));
    assert_eq!(error.details().outcome_unknown, Some(true));
    target
        .item_detail(checklist, &original, "Renamed", false)
        .await;
    target
        .item(checklist, &["update", &original, "--is-finished", "true"])
        .await
        .unwrap();
    target
        .item_detail(checklist, &original, "Renamed", true)
        .await;

    // Checklist insertion precedes item insertion, so a later invalid item can leave partial state.
    let partial_title = format!("Partial checklist {nonce}");
    let error = target
        .checklist(&[
            "create",
            "--title",
            &partial_title,
            "--items",
            r#"["Before failure",""]"#,
        ])
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ServerError);
    assert_eq!(error.details().http_status, Some(500));
    assert_eq!(error.details().outcome_unknown, Some(true));
    let mut partial = None;
    for _ in 0..40 {
        partial = target
            .collection()
            .await
            .into_iter()
            .find(|value| value.title == partial_title);
        if partial.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    let partial = partial.expect("failed creation retains its inserted checklist");
    let detail = target.detail(&partial.checklist_id).await;
    assert_eq!(detail.items.len(), 1);
    assert_eq!(detail.items[0].title, "Before failure");

    let missing = target
        .checklist(&["get", "missing-checklist"])
        .await
        .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::ServerError);
    assert_eq!(missing.details().http_status, Some(500));
    for args in [
        vec!["get", "missing-item"],
        vec!["update", "missing-item"],
        vec!["delete", "missing-item", "--yes"],
    ] {
        let error = target.item(checklist, &args).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::NotFound);
        assert_eq!(error.details().http_status, Some(404));
    }
    let wrong = Target {
        app: &app,
        server: &server,
        board: &board.board_id,
        card: "missing-card",
    };
    for args in [
        vec!["list"],
        vec!["get", checklist],
        vec!["create"],
        vec!["delete", checklist, "--yes"],
    ] {
        assert_eq!(
            wrong.checklist(&args).await.unwrap_err().code(),
            ErrorCode::NotFound
        );
    }
    for args in [
        vec!["get", &original],
        vec!["create", "--title", "Wrong card"],
        vec!["update", &original, "--is-finished", "false"],
        vec!["delete", &original, "--yes"],
    ] {
        assert_eq!(
            wrong.item(checklist, &args).await.unwrap_err().code(),
            ErrorCode::NotFound
        );
    }
    for args in [
        vec!["get", &original],
        vec!["create", "--title", "Wrong checklist"],
        vec!["update", &original],
        vec!["delete", &original, "--yes"],
    ] {
        assert_eq!(
            target
                .item("missing-checklist", &args)
                .await
                .unwrap_err()
                .code(),
            ErrorCode::NotFound
        );
    }

    for role in ["normal", "readonly", "commentonly", "worker", "nocomments"] {
        let (_directory, role_app, _) =
            register_live_board_role(&server, &canonical, &token, &board.board_id, nonce, role)
                .await;
        let role_target = Target {
            app: &role_app,
            server: &server,
            board: &board.board_id,
            card: &card.card_id,
        };
        if role == "normal" || role == "readonly" {
            role_target.checklist(&["list"]).await.unwrap();
            role_target.detail(checklist).await;
            role_target
                .item_detail(checklist, &original, "Renamed", true)
                .await;
            let creator = if role == "readonly" {
                assert_eq!(
                    role_target.checklist(&["create"]).await.unwrap_err().code(),
                    ErrorCode::PermissionDenied
                );
                &target
            } else {
                &role_target
            };
            let CommandSuccess::ChecklistCreated(role_checklist) =
                creator.checklist(&["create"]).await.unwrap()
            else {
                panic!("expected role checklist");
            };
            role_target.detail(&role_checklist.checklist_id).await;
            let CommandSuccess::ChecklistItemCreated(role_item) = role_target
                .item(
                    &role_checklist.checklist_id,
                    &["create", "--title", "Role item"],
                )
                .await
                .unwrap()
            else {
                panic!("expected role item");
            };
            role_target
                .item_detail(
                    &role_checklist.checklist_id,
                    &role_item.item_id,
                    "Role item",
                    false,
                )
                .await;
            role_target
                .item(
                    &role_checklist.checklist_id,
                    &["update", &role_item.item_id, "--is-finished", "true"],
                )
                .await
                .unwrap();
            role_target
                .item(
                    &role_checklist.checklist_id,
                    &["delete", &role_item.item_id, "--yes"],
                )
                .await
                .unwrap();
            role_target
                .checklist(&["delete", &role_checklist.checklist_id, "--yes"])
                .await
                .unwrap();
        } else {
            for args in [
                vec!["list"],
                vec!["get", checklist],
                vec!["create"],
                vec!["delete", checklist, "--yes"],
            ] {
                let error = role_target.checklist(&args).await.unwrap_err();
                assert_eq!(error.code(), ErrorCode::PermissionDenied, "{role}");
                assert_eq!(error.details().http_status, Some(403));
            }
            for args in [
                vec!["get", &original],
                vec!["create", "--title", "Denied"],
                vec!["update", &original],
                vec!["delete", &original, "--yes"],
            ] {
                assert_eq!(
                    role_target.item(checklist, &args).await.unwrap_err().code(),
                    ErrorCode::PermissionDenied,
                    "{role}"
                );
            }
        }
    }

    target
        .item(checklist, &["delete", &original, "--yes"])
        .await
        .unwrap();
    for _ in 0..40 {
        match target.item(checklist, &["get", &original]).await {
            Err(error) if error.code() == ErrorCode::NotFound => break,
            Ok(_) => tokio::time::sleep(std::time::Duration::from_millis(250)).await,
            result => panic!("unexpected item deletion response: {result:?}"),
        }
    }
    assert_eq!(
        target
            .item(checklist, &["get", &original])
            .await
            .unwrap_err()
            .code(),
        ErrorCode::NotFound
    );
    assert_eq!(
        item_activities(&original),
        activities_before,
        "direct REST item deletion bypasses activity cleanup hooks"
    );
    // v11.06 checklist deletion does not cascade to its item documents.
    let survivor = target
        .detail(checklist)
        .await
        .items
        .into_iter()
        .find(|item| item.item_id != original)
        .unwrap();
    target
        .checklist(&["delete", checklist, "--yes"])
        .await
        .unwrap();
    for _ in 0..40 {
        if !target
            .collection()
            .await
            .iter()
            .any(|value| value.checklist_id == *checklist)
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    assert!(
        !target
            .collection()
            .await
            .iter()
            .any(|value| value.checklist_id == *checklist)
    );
    target
        .item_detail(checklist, &survivor.item_id, &survivor.title, false)
        .await;
    target
        .item(checklist, &["delete", &survivor.item_id, "--yes"])
        .await
        .unwrap();

    // REST card moves bypass the hook that synchronizes denormalized board IDs.
    // Reads use the destination scope but must preserve the source metadata.
    let CommandSuccess::ChecklistItemCreated(moving_item) = target
        .item(&default.checklist_id, &["create", "--title", "Moving item"])
        .await
        .unwrap()
    else {
        panic!("expected item to move with its card");
    };
    let before_move = target
        .item_detail(
            &default.checklist_id,
            &moving_item.item_id,
            "Moving item",
            false,
        )
        .await;
    assert_eq!(
        before_move.board_id.as_deref(),
        Some(board.board_id.as_str())
    );
    assert_eq!(
        target
            .detail(&default.checklist_id)
            .await
            .board_id
            .as_deref(),
        Some(board.board_id.as_str())
    );
    let CommandSuccess::BoardCreated(destination) = execute_live_board(
        &app,
        &server,
        &["create", "--title", "Checklist destination"],
    )
    .await
    .unwrap() else {
        panic!("expected destination board");
    };
    let CommandSuccess::ListCreated(destination_list) = execute_live_list(
        &app,
        &server,
        &[
            "create",
            "--board",
            &destination.board_id,
            "--title",
            "Destination list",
            "--swimlane-id",
            &destination.default_swimlane_id,
        ],
    )
    .await
    .unwrap() else {
        panic!("expected destination list");
    };
    let CommandSuccess::CardUpdated(_) = execute_live_card(
        &app,
        &server,
        &[
            "update",
            &card.card_id,
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--new-board-id",
            &destination.board_id,
            "--new-swimlane-id",
            &destination.default_swimlane_id,
            "--new-list-id",
            &destination_list.list_id,
        ],
    )
    .await
    .unwrap() else {
        panic!("expected card board move");
    };
    let moved = Target {
        app: &app,
        server: &server,
        board: &destination.board_id,
        card: &card.card_id,
    };
    let moved_checklist = moved.detail(&default.checklist_id).await;
    let moved_item = moved
        .item_detail(
            &default.checklist_id,
            &moving_item.item_id,
            "Moving item",
            false,
        )
        .await;
    assert_eq!(moved_checklist.card_id, card.card_id);
    assert_eq!(moved_item.card_id, card.card_id);
    assert_eq!(
        moved_checklist.board_id.as_deref(),
        Some(board.board_id.as_str())
    );
    assert_eq!(
        moved_item.board_id.as_deref(),
        Some(board.board_id.as_str())
    );
    assert!(
        moved
            .collection()
            .await
            .iter()
            .any(|value| value.checklist_id == default.checklist_id)
    );
    for result in [
        CommandSuccess::ChecklistShown(moved_checklist),
        CommandSuccess::ChecklistItemShown(moved_item),
    ] {
        let json_output = wekan_cli::output::render_success(OutputFormat::Json, &result);
        let envelope: serde_json::Value = serde_json::from_str(&json_output).unwrap();
        assert_eq!(envelope["data"]["board_id"], board.board_id);
        let human_output = wekan_cli::output::render_success(OutputFormat::Human, &result);
        assert!(human_output.contains(&format!("board_id: {}", board.board_id)));
    }
    // The server still rejects reads through the card's old board scope.
    assert_eq!(
        target
            .checklist(&["get", &default.checklist_id])
            .await
            .unwrap_err()
            .code(),
        ErrorCode::NotFound
    );
    assert_eq!(
        target
            .item(&default.checklist_id, &["get", &moving_item.item_id])
            .await
            .unwrap_err()
            .code(),
        ErrorCode::NotFound
    );
    execute_live_board(&app, &server, &["delete", &destination.board_id, "--yes"])
        .await
        .unwrap();
    execute_live_board(&app, &server, &["delete", &board.board_id, "--yes"])
        .await
        .unwrap();
}
