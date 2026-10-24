use super::support::*;
use serde_json::{Value, json};

type LiveApp = App<CapturingStore, FixedPassword, FileProfileStore>;

async fn run(app: &LiveApp, server: &str, args: &[&str]) -> Result<Value, AppError> {
    let cli = Cli::try_parse_from(
        ["wekan", "--server", server]
            .into_iter()
            .chain(args.iter().copied()),
    )
    .unwrap();
    let success = app.execute(cli.command).await?;
    let human = wekan_cli::output::render_success(OutputFormat::Human, &success);
    assert!(!human.is_empty());
    if matches!(
        &success,
        CommandSuccess::BoardCopied(_) | CommandSuccess::CardCopied(_)
    ) {
        assert!(human.starts_with("Board copied:") || human.starts_with("Card copied:"));
    }
    Ok(
        serde_json::from_str::<Value>(&wekan_cli::output::render_success(
            OutputFormat::Json,
            &success,
        ))
        .unwrap()["data"]
            .clone(),
    )
}

async fn board(app: &LiveApp, server: &str, title: &str) -> (String, String, String) {
    let b = run(app, server, &["board", "create", "--title", title])
        .await
        .unwrap();
    let id = b["board_id"].as_str().unwrap().to_owned();
    let lane = b["default_swimlane_id"].as_str().unwrap().to_owned();
    let l = run(
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
            &lane,
        ],
    )
    .await
    .unwrap();
    (id, l["list_id"].as_str().unwrap().to_owned(), lane)
}

async fn card(app: &LiveApp, server: &str, b: &str, l: &str, s: &str, title: &str) -> String {
    run(
        app,
        server,
        &[
            "card",
            "create",
            "--board",
            b,
            "--list",
            l,
            "--swimlane-id",
            s,
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

async fn count(app: &LiveApp, server: &str, b: &str, l: Option<&str>) -> u64 {
    let mut args = vec!["card", "count", "--board", b];
    if let Some(l) = l {
        args.extend(["--list", l]);
    }
    let result = run(app, server, &args).await.unwrap();
    assert_eq!(result["list_id"].as_str(), l);
    result["count"].as_u64().unwrap()
}

async fn detail(app: &LiveApp, server: &str, c: &str) -> Value {
    run(app, server, &["card", "get", c]).await.unwrap()
}

async fn list_cards(app: &LiveApp, server: &str, b: &str, l: &str) -> Vec<Value> {
    run(app, server, &["card", "list", "--board", b, "--list", l])
        .await
        .unwrap()["cards"]
        .as_array()
        .unwrap()
        .clone()
}

async fn assert_copied_cards(
    app: &LiveApp,
    server: &str,
    destination: (&str, &str, &str),
    before: &[Value],
    expected_titles: &[&str],
) {
    let (b, l, s) = destination;
    let after = list_cards(app, server, b, l).await;
    assert_eq!(after.len(), before.len() + expected_titles.len());
    for original in before {
        assert!(after.contains(original), "copy must retain existing cards");
    }
    let mut copied_titles = Vec::new();
    for copied in after.iter().filter(|card| {
        !before
            .iter()
            .any(|original| original["card_id"] == card["card_id"])
    }) {
        let card = detail(app, server, copied["card_id"].as_str().unwrap()).await;
        assert_eq!(card["board_id"], b);
        assert_eq!(card["list_id"], l);
        assert_eq!(card["swimlane_id"], s);
        assert_eq!(card["archived"], false);
        copied_titles.push(card["title"].as_str().unwrap().to_owned());
    }
    copied_titles.sort();
    let mut expected_titles = expected_titles.to_vec();
    expected_titles.sort();
    assert_eq!(copied_titles, expected_titles);
}

async fn verify_list_copy(
    admin: &LiveApp,
    writer: &LiveApp,
    server: &str,
    source: (&str, &str),
    destination: (&str, &str),
) {
    let (b, s) = source;
    let (target, target_lane) = destination;
    let mut lists = Vec::new();
    for (board_id, swimlane_id) in [source, destination] {
        let list = run(
            admin,
            server,
            &[
                "list",
                "create",
                "--board",
                board_id,
                "--swimlane-id",
                swimlane_id,
                "--title",
                "List copy filter",
            ],
        )
        .await
        .unwrap();
        lists.push(list["list_id"].as_str().unwrap().to_owned());
    }
    let l = &lists[0];
    let target_list = &lists[1];
    let other_lane = run(
        admin,
        server,
        &[
            "swimlane",
            "create",
            "--board",
            b,
            "--title",
            "Excluded lane",
        ],
    )
    .await
    .unwrap();
    let mut originals = Vec::new();
    for (title, lane) in [
        ("Eligible first", s),
        ("Eligible second", s),
        (
            "Other swimlane",
            other_lane["swimlane_id"].as_str().unwrap(),
        ),
        ("Archived", s),
    ] {
        let id = card(admin, server, b, l, lane, title).await;
        if title == "Archived" {
            run(
                admin,
                server,
                &["card", "archive", &id, "--board", b, "--list", l],
            )
            .await
            .unwrap();
        }
        originals.push(detail(admin, server, &id).await);
    }
    card(
        admin,
        server,
        target,
        target_list,
        target_lane,
        "Existing destination",
    )
    .await;
    let source_before = list_cards(admin, server, b, l).await;
    let target_before = list_cards(admin, server, target, target_list).await;
    let eligible_titles = ["Eligible first", "Eligible second", "Other swimlane"];
    let merged = run(
        writer,
        server,
        &[
            "list",
            "copy",
            l,
            "--board",
            b,
            "--target-board",
            target,
            "--target-swimlane-id",
            target_lane,
            "--position-json",
            "null",
        ],
    )
    .await
    .unwrap();
    assert_eq!(merged["returned_list_id"], target_list.as_str());
    assert_copied_cards(
        admin,
        server,
        (target, target_list, target_lane),
        &target_before,
        &eligible_titles,
    )
    .await;
    assert_eq!(list_cards(admin, server, b, l).await, source_before);

    let self_copy = run(
        writer,
        server,
        &["list", "copy", l, "--board", b, "--target-swimlane-id", s],
    )
    .await
    .unwrap();
    let self_list = self_copy["returned_list_id"].as_str().unwrap();
    assert_ne!(
        self_list,
        l.as_str(),
        "a same-board list copy creates a separate list"
    );
    assert_copied_cards(admin, server, (b, self_list, s), &[], &eligible_titles).await;
    assert_eq!(list_cards(admin, server, b, l).await, source_before);
    for original in originals {
        assert_eq!(
            detail(admin, server, original["card_id"].as_str().unwrap()).await,
            original,
            "copy must preserve each source card, including excluded cards",
        );
    }
}

async fn assert_resource_position(
    app: &LiveApp,
    server: &str,
    family: &str,
    board_id: &str,
    id: &str,
    expected_sort: f64,
    siblings: &[(String, f64)],
) {
    let collection = run(app, server, &[family, "list", "--board", board_id])
        .await
        .unwrap();
    let mut actual = Vec::new();
    for summary in collection[format!("{family}s")].as_array().unwrap() {
        let resource_id = summary[format!("{family}_id")].as_str().unwrap();
        let detail = run(
            app,
            server,
            &[family, "get", resource_id, "--board", board_id],
        )
        .await
        .unwrap();
        actual.push((resource_id.to_owned(), detail["sort"].as_f64().unwrap()));
    }
    actual.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut expected = siblings.to_vec();
    expected.push((id.to_owned(), expected_sort));
    expected.sort_by(|a, b| a.1.total_cmp(&b.1));
    assert_eq!(actual, expected, "persisted {family} order on {board_id}");
}

async fn verify_transfer_ordering(app: &LiveApp, server: &str) {
    use super::support::attachment_fixtures::fixture_command;

    let (source, source_list, source_lane) = board(app, server, "Ordering source").await;
    let (target, target_list, target_lane) = board(app, server, "Ordering destination").await;
    // A unique source list exercises insertion; copying it again exercises reuse.
    run(
        app,
        server,
        &[
            "list",
            "update",
            &source_list,
            "--board",
            &source,
            "--title",
            "Ordering copy",
        ],
    )
    .await
    .unwrap();

    for (family, source_id, first_id) in [
        ("list", &source_list, &target_list),
        ("swimlane", &source_lane, &target_lane),
    ] {
        let mut siblings = vec![(first_id.clone(), 10.0)];
        for (title, sort) in [("Ordering middle", 30.0), ("Ordering last", 70.0)] {
            let created = run(
                app,
                server,
                &[family, "create", "--board", &target, "--title", title],
            )
            .await
            .unwrap();
            siblings.push((
                created[format!("{family}_id")].as_str().unwrap().to_owned(),
                sort,
            ));
        }
        if family == "swimlane" {
            let default_id = format!("{target}-default");
            assert_eq!(
                run(
                    app,
                    server,
                    &["swimlane", "get", &default_id, "--board", &target]
                )
                .await
                .unwrap_err()
                .code(),
                ErrorCode::NotFound
            );
            // W11-049: even with active lanes, the native move's synchronous
            // getter starts an unawaited deterministic-default upsert. Observe
            // that effect before fixing the sibling sorts for ordering checks.
            run(
                app,
                server,
                &["swimlane", "move", first_id, "--board", &target],
            )
            .await
            .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                match run(
                    app,
                    server,
                    &["swimlane", "get", &default_id, "--board", &target],
                )
                .await
                {
                    Ok(detail) => {
                        assert_eq!(detail["board_id"], target);
                        assert_eq!(detail["sort"], 0);
                        assert_eq!(detail["archived"], false);
                        break;
                    }
                    Err(error) if error.code() == ErrorCode::NotFound => {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "default swimlane insertion did not complete within 10 seconds"
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                    }
                    Err(error) => panic!("default swimlane read failed: {error:?}"),
                }
            }
            siblings.push((default_id, 100.0));
        }
        // Seed independently of the transfer command under test. Non-contiguous
        // sorts distinguish the requested position from native copy/move defaults.
        fixture_command(
            "mongodb",
            "mongosh",
            r#"
            const collection=db.getSiblingDB('wekan').getCollection(p.collection);
            for (const [id,sort] of p.sorts) {
              const result=collection.updateOne({_id:id},{$set:{sort}});
              if(result.matchedCount!==1) throw new Error('ordering fixture not found');
            }
            print('{}');
            "#,
            &json!({"collection":format!("{family}s"),"sorts":siblings}),
        );

        let (flag, value, expected_sort) = if family == "list" {
            ("--position", "1.9", 20.0)
        } else {
            ("--position-json", "false", 9.0)
        };
        let mut args = vec![
            family,
            "copy",
            source_id,
            "--board",
            &source,
            "--target-board",
            &target,
            flag,
            value,
        ];
        if family == "list" {
            args.extend(["--target-swimlane-id", &target_lane]);
        }
        let copied = run(app, server, &args).await.unwrap();
        let returned_field = format!("returned_{family}_id");
        let id = copied[&returned_field].as_str().unwrap();
        assert_ne!(id, source_id);
        assert!(copied.get("mutation_confirmed").is_none());
        assert_resource_position(app, server, family, &target, id, expected_sort, &siblings).await;

        for (flag, value, expected_sort) in [
            ("--position", "2.9", 50.0),
            ("--position", "-2.5", 9.0),
            ("--position-json", "[\"2\"]", 50.0),
            (
                "--position-json",
                "{}",
                if family == "list" { 71.0 } else { 101.0 },
            ),
            ("--position-json", "false", 9.0),
        ] {
            let result = run(
                app,
                server,
                &[family, "move", id, "--board", &target, flag, value],
            )
            .await
            .unwrap();
            assert_eq!(result[&returned_field], id);
            assert!(result.get("mutation_confirmed").is_none());
            assert_resource_position(app, server, family, &target, id, expected_sort, &siblings)
                .await;
        }

        // Null skips the route's reposition helper. List.move preserves sort;
        // Swimlane.move first assigns its native sibling-count sort (four).
        let expected_sort = if family == "list" { 9.0 } else { 4.0 };
        for position in [vec!["--position-json", "null"], vec![]] {
            let mut args = vec![family, "move", id, "--board", &target];
            args.extend(position);
            run(app, server, &args).await.unwrap();
            assert_resource_position(app, server, family, &target, id, expected_sort, &siblings)
                .await;
        }
    }
    for id in [&source, &target] {
        run(app, server, &["board", "delete", id, "--yes"])
            .await
            .unwrap();
    }
}

#[tokio::test]
#[ignore = "destructive: requires an isolated Wekan v12.57 stack and WEKAN_TRANSFER_E2E_URL"]
async fn complete_copy_move_and_card_read_lifecycle_matches_wekan_v12_57() {
    let server = env::var("WEKAN_TRANSFER_E2E_URL")
        .expect("set WEKAN_TRANSFER_E2E_URL to an isolated stack");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let dir = TestDirectory::new("transfers");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Transfers-{nonce}!"))),
        FileProfileStore::at(dir.path().to_owned()),
    );
    let user = format!("wekan_cli_transfers_{nonce}");
    run(
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
    let (b, l, s) = board(&app, &server, "Transfer source").await;
    let (target, target_list, target_lane) = board(&app, &server, "Transfer destination").await;
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
    let (_normal_dir, normal, normal_id) =
        register_live_board_role(&server, &canonical, &token, &b, nonce, "normal").await;
    run(
        &app,
        &server,
        &[
            "board", "member", "add", &normal_id, "--board", &target, "--role", "normal",
        ],
    )
    .await
    .unwrap();
    wait_for_live_board_membership(&normal, &server, &target).await;
    assert_ne!(
        run(&normal, &server, &["user", "current"]).await.unwrap()["is_admin"],
        json!(true),
    );
    assert_eq!(count(&app, &server, &b, None).await, 0);
    assert_eq!(count(&app, &server, &b, Some("missing")).await, 0);
    let empty = run(
        &app,
        &server,
        &["card", "list", "--board", &b, "--swimlane", "missing"],
    )
    .await
    .unwrap();
    assert_eq!(empty["cards"], json!([]));
    let c = card(&app, &server, &b, &l, &s, "Parent").await;
    let second = card(&app, &server, &b, &l, &s, "Second").await;
    assert_eq!(count(&app, &server, &b, None).await, 2);
    assert_eq!(count(&app, &server, &b, Some(&l)).await, 2);
    let scoped = run(
        &app,
        &server,
        &["card", "get", &c, "--board", &b, "--list", &l],
    )
    .await
    .unwrap();
    assert_eq!(detail(&app, &server, &c).await, scoped);
    let lane_cards = run(
        &app,
        &server,
        &["card", "list", "--board", &b, "--swimlane", &s],
    )
    .await
    .unwrap();
    assert_eq!(lane_cards["cards"].as_array().unwrap().len(), 2);
    assert_eq!(lane_cards["cards"][0]["list_id"], l);
    run(
        &app,
        &server,
        &["card", "archive", &second, "--board", &b, "--list", &l],
    )
    .await
    .unwrap();
    assert!(
        detail(&app, &server, &second).await["archived"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(
        run(
            &app,
            &server,
            &["card", "get", &second, "--board", &b, "--list", &l]
        )
        .await
        .unwrap_err()
        .code(),
        ErrorCode::NotFound
    );
    assert_eq!(count(&app, &server, &b, None).await, 1);
    assert_eq!(
        run(&app, &server, &["card", "get", "missing"])
            .await
            .unwrap_err()
            .code(),
        ErrorCode::NotFound
    );
    run(
        &app,
        &server,
        &["card", "restore", &second, "--board", &b, "--list", &l],
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
            &b,
            "--card",
            &c,
            "--title",
            "Steps",
            "--item",
            "Do it",
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
            &b,
            "--card",
            &c,
            "--text",
            "Copy this comment",
        ],
    )
    .await
    .unwrap();
    let children = run(
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
            "--swimlane-id",
            &s,
            "--cards",
            &json!([{"title":"Child","parentId":c}]).to_string(),
        ],
    )
    .await
    .unwrap();
    let child = children["results"][0]["card_id"].as_str().unwrap();
    assert_eq!(detail(&app, &server, child).await["parent_id"], c);
    let copied = run(
        &normal,
        &server,
        &[
            "card",
            "copy",
            &c,
            "--board",
            &b,
            "--list",
            &l,
            "--target-board",
            &target,
            "--target-list",
            &target_list,
            "--target-swimlane-id",
            &target_lane,
            "--position-json",
            "null",
        ],
    )
    .await
    .unwrap();
    let copied_card = copied["card_id"].as_str().unwrap();
    assert!(copied.get("mutation_confirmed").is_none());
    let copied_detail = detail(&app, &server, copied_card).await;
    assert_eq!(copied_detail["board_id"], target);
    assert_eq!(copied_detail["list_id"], target_list);
    assert_eq!(copied_detail["swimlane_id"], target_lane);
    assert_eq!(detail(&app, &server, &c).await["board_id"], b);
    let copied_checks = run(
        &app,
        &server,
        &[
            "checklist",
            "list",
            "--board",
            &target,
            "--card",
            copied_card,
        ],
    )
    .await
    .unwrap();
    assert_eq!(copied_checks["checklists"][0]["title"], "Steps");
    assert_ne!(
        copied_checks["checklists"][0]["checklist_id"],
        checklist["checklist_id"]
    );
    let copied_check = run(
        &app,
        &server,
        &[
            "checklist",
            "get",
            copied_checks["checklists"][0]["checklist_id"]
                .as_str()
                .unwrap(),
            "--board",
            &target,
            "--card",
            copied_card,
        ],
    )
    .await
    .unwrap();
    assert_eq!(copied_check["items"][0]["title"], "Do it");
    let comments = run(
        &app,
        &server,
        &["comment", "list", "--board", &target, "--card", copied_card],
    )
    .await
    .unwrap();
    assert_eq!(comments["comments"].as_array().unwrap().len(), 1);
    let copied_children = run(
        &app,
        &server,
        &[
            "card",
            "list",
            "--board",
            &target,
            "--swimlane",
            &target_lane,
        ],
    )
    .await
    .unwrap();
    assert!(
        copied_children["cards"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["title"] == "Child")
    );
    let copied_child_id = copied_children["cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["title"] == "Child")
        .unwrap()["card_id"]
        .as_str()
        .unwrap();
    let copied_child = detail(&app, &server, copied_child_id).await;
    assert_eq!(copied_child["parent_id"], copied_card);
    assert_eq!(copied_child["board_id"], target);
    assert_eq!(copied_child["list_id"], target_list);
    let copy_again = run(
        &normal,
        &server,
        &[
            "card",
            "copy",
            &second,
            "--board",
            &b,
            "--list",
            &l,
            "--target-swimlane-id",
            &s,
            "--position",
            "-2.5",
        ],
    )
    .await
    .unwrap();
    assert!(
        detail(&app, &server, copy_again["card_id"].as_str().unwrap()).await["sort"]
            .as_f64()
            .unwrap()
            < detail(&app, &server, &second).await["sort"]
                .as_f64()
                .unwrap()
    );

    verify_list_copy(&app, &normal, &server, (&b, &s), (&target, &target_lane)).await;
    verify_transfer_ordering(&normal, &server).await;
    let copied_lane = run(
        &normal,
        &server,
        &[
            "swimlane",
            "copy",
            &s,
            "--board",
            &b,
            "--target-board",
            &target,
            "--title",
            " Copied lane ",
            "--position-json",
            "false",
        ],
    )
    .await
    .unwrap();
    let new_lane = copied_lane["returned_swimlane_id"].as_str().unwrap();
    let lane = run(
        &app,
        &server,
        &["swimlane", "get", new_lane, "--board", &target],
    )
    .await
    .unwrap();
    assert_eq!(lane["title"], "Copied lane");
    let cards = run(
        &app,
        &server,
        &["card", "list", "--board", &target, "--swimlane", new_lane],
    )
    .await
    .unwrap();
    assert!(!cards["cards"].as_array().unwrap().is_empty());

    let board_copy = run(
        &app,
        &server,
        &["board", "copy", &b, "--title", "Board duplicate"],
    )
    .await
    .unwrap();
    let copied_board = board_copy["board_id"].as_str().unwrap();
    assert!(board_copy.get("mutation_confirmed").is_none());
    assert_ne!(copied_board, b);
    let copied_board_detail = run(&app, &server, &["board", "get", copied_board])
        .await
        .unwrap();
    assert!(
        copied_board_detail["title"]
            .as_str()
            .unwrap()
            .starts_with("Board duplicate")
    );
    assert!(count(&app, &server, copied_board, None).await > 0);

    verify_empty_and_partial_moves(&normal, &server, &b, &target, &target_lane).await;
    verify_attachment_copy(&app, &server, nonce).await;
    verify_permissions(&app, &server, &b, &l, &s, &c, nonce).await;
    // Cleanup removes only boards created by this test, after all defect assertions.
    for id in [copied_board, &b, &target] {
        run(&app, &server, &["board", "delete", id, "--yes"])
            .await
            .unwrap();
    }
}

async fn verify_empty_and_partial_moves(
    app: &LiveApp,
    server: &str,
    b: &str,
    target: &str,
    target_lane: &str,
) {
    let empty = run(
        app,
        server,
        &["list", "create", "--board", b, "--title", "Empty move"],
    )
    .await
    .unwrap();
    let list_id = empty["list_id"].as_str().unwrap();
    let copied = run(app, server, &["list", "copy", list_id, "--board", b])
        .await
        .unwrap();
    assert_ne!(copied["returned_list_id"], list_id);
    let moved = run(
        app,
        server,
        &[
            "list",
            "move",
            list_id,
            "--board",
            b,
            "--target-board",
            target,
            "--position",
            "100",
        ],
    )
    .await
    .unwrap();
    assert_eq!(moved["returned_list_id"], list_id);
    assert_eq!(
        run(app, server, &["list", "get", list_id, "--board", b])
            .await
            .unwrap()["board_id"],
        b
    );
    let lists = run(app, server, &["list", "list", "--board", target])
        .await
        .unwrap();
    assert!(
        lists["lists"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["title"] == "Empty move")
    );
    let lane = run(
        app,
        server,
        &["swimlane", "create", "--board", b, "--title", "Empty lane"],
    )
    .await
    .unwrap();
    let lane_id = lane["swimlane_id"].as_str().unwrap();
    run(
        app,
        server,
        &[
            "swimlane",
            "move",
            lane_id,
            "--board",
            b,
            "--position-json",
            "[\"0\"]",
            "--title",
            "Renamed lane",
        ],
    )
    .await
    .unwrap();
    run(
        app,
        server,
        &[
            "swimlane",
            "move",
            lane_id,
            "--board",
            b,
            "--target-board",
            target,
            "--position-json",
            "{}",
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        run(
            app,
            server,
            &["swimlane", "get", lane_id, "--board", target]
        )
        .await
        .unwrap()["title"],
        "Renamed lane"
    );

    verify_list_move_filters(app, server, b, target, target_lane).await;

    let lane = run(
        app,
        server,
        &[
            "swimlane",
            "create",
            "--board",
            b,
            "--title",
            "Partial lane",
        ],
    )
    .await
    .unwrap();
    let lane_id = lane["swimlane_id"].as_str().unwrap();
    let list = run(
        app,
        server,
        &[
            "list",
            "create",
            "--board",
            b,
            "--swimlane-id",
            lane_id,
            "--title",
            "Lane partial move",
        ],
    )
    .await
    .unwrap();
    let card_id = card(
        app,
        server,
        b,
        list["list_id"].as_str().unwrap(),
        lane_id,
        "Lane partial card",
    )
    .await;
    let error = run(
        app,
        server,
        &[
            "swimlane",
            "move",
            lane_id,
            "--board",
            b,
            "--target-board",
            target,
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
    assert_eq!(detail(app, server, &card_id).await["board_id"], target);
    assert_eq!(
        run(app, server, &["swimlane", "get", lane_id, "--board", b])
            .await
            .unwrap()["board_id"],
        b
    );
}

async fn verify_list_move_filters(
    app: &LiveApp,
    server: &str,
    source: &str,
    target: &str,
    target_lane: &str,
) {
    let source_lane = run(app, server, &["swimlane", "list", "--board", source])
        .await
        .unwrap()["swimlanes"][0]["swimlane_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(source_lane, target_lane);
    let other_lane = run(
        app,
        server,
        &[
            "swimlane",
            "create",
            "--board",
            source,
            "--title",
            "List move excluded lane",
        ],
    )
    .await
    .unwrap();
    let other_lane = other_lane["swimlane_id"].as_str().unwrap();

    // Each request has its own source list and independently established
    // destination state, so one case cannot switch another to the reuse branch.
    for (title, existing_destination, matching_filter) in [
        ("New destination nonmatching filter", false, false),
        ("New destination matching filter", false, true),
        ("Same-title destination filter bypass", true, false),
    ] {
        let list = run(
            app,
            server,
            &[
                "list",
                "create",
                "--board",
                source,
                "--title",
                title,
                "--swimlane-id",
                &source_lane,
            ],
        )
        .await
        .unwrap();
        let list_id = list["list_id"].as_str().unwrap();
        let before = run(app, server, &["list", "list", "--board", target])
            .await
            .unwrap();
        assert!(
            before["lists"]
                .as_array()
                .unwrap()
                .iter()
                .all(|l| l["title"] != title)
        );
        let existing_list = if existing_destination {
            Some(
                run(
                    app,
                    server,
                    &[
                        "list",
                        "create",
                        "--board",
                        target,
                        "--title",
                        title,
                        "--swimlane-id",
                        target_lane,
                    ],
                )
                .await
                .unwrap(),
            )
        } else {
            None
        };
        let card_id = card(app, server, source, list_id, &source_lane, title).await;
        // A second source lane proves the matching case does not simply move
        // the first available card. The reuse case needs one excluded card.
        let excluded = if existing_destination {
            None
        } else {
            let id = card(
                app,
                server,
                source,
                list_id,
                other_lane,
                "Excluded source card",
            )
            .await;
            Some((id.clone(), detail(app, server, &id).await))
        };
        let requested_lane = if matching_filter {
            &source_lane
        } else {
            target_lane
        };
        let result = run(
            app,
            server,
            &[
                "list",
                "move",
                list_id,
                "--board",
                source,
                "--target-board",
                target,
                "--target-swimlane-id",
                requested_lane,
            ],
        )
        .await;
        let after = run(app, server, &["list", "list", "--board", target])
            .await
            .unwrap();
        let destination_lists: Vec<_> = after["lists"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|l| l["title"] == title)
            .collect();
        assert_eq!(destination_lists.len(), 1);
        let destination_id = destination_lists[0]["list_id"].as_str().unwrap();
        assert_ne!(destination_id, list_id);
        if let Some(existing) = existing_list {
            assert_eq!(existing["list_id"], destination_id);
        }
        {
            let error = result.unwrap_err();
            assert_eq!(error.details().http_status, Some(500));
            assert!(
                serde_json::to_value(error.details())
                    .unwrap()
                    .get("outcome_unknown")
                    .is_none()
            );
            let moved = detail(app, server, &card_id).await;
            assert_eq!(moved["board_id"], target);
            assert_eq!(moved["list_id"], destination_id);
            assert_eq!(
                list_cards(app, server, target, destination_id).await.len(),
                1
            );
        }
        if let Some((id, original)) = excluded {
            assert_eq!(detail(app, server, &id).await, original);
        }
        assert_eq!(
            run(app, server, &["list", "get", list_id, "--board", source])
                .await
                .unwrap()["board_id"],
            source
        );
    }
}

async fn verify_permissions(
    app: &LiveApp,
    server: &str,
    b: &str,
    l: &str,
    s: &str,
    c: &str,
    nonce: u128,
) {
    let token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let canonical = ServerUrl::parse(server).unwrap().as_str().to_owned();
    let (_dir, readonly, _) =
        register_live_board_role(server, &canonical, &token, b, nonce, "readonly").await;
    for args in [
        vec![
            "card",
            "copy",
            c,
            "--board",
            b,
            "--list",
            l,
            "--target-swimlane-id",
            s,
        ],
        vec!["list", "copy", l, "--board", b],
        vec!["list", "move", l, "--board", b],
        vec!["swimlane", "copy", s, "--board", b],
        vec!["swimlane", "move", s, "--board", b],
    ] {
        assert_eq!(
            run(&readonly, server, &args).await.unwrap_err().code(),
            ErrorCode::PermissionDenied
        );
    }
    assert!(count(&readonly, server, b, None).await > 0);
    assert!(run(&readonly, server, &["card", "get", c]).await.is_ok());
    let denied = run(&readonly, server, &["board", "copy", b])
        .await
        .unwrap_err();
    assert_eq!(denied.details().http_status, Some(403));
    let missing = run(app, server, &["card", "count", "--board", "missing"])
        .await
        .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::NotFound);
    assert_eq!(missing.details().http_status, Some(404));
}

async fn verify_attachment_copy(app: &LiveApp, server: &str, _nonce: u128) {
    use super::support::attachment_fixtures::fixture_command;
    let (b, l, s) = board(app, server, "Attachment source").await;
    let (target, target_list, target_lane) = board(app, server, "Attachment copies").await;
    let c = card(app, server, &b, &l, &s, "Attachment copy").await;
    let bytes = [0_u8, 255, 13, 10, 42];
    let dir = TestDirectory::new("source-attachment");
    let source = dir.path().join("original.bin");
    fs::write(&source, bytes).unwrap();
    let uploaded = run(
        app,
        server,
        &[
            "attachment",
            "upload",
            source.to_str().unwrap(),
            "--transport",
            "raw",
            "--board",
            &b,
            "--swimlane-id",
            &s,
            "--list",
            &l,
            "--card",
            &c,
        ],
    )
    .await
    .unwrap();
    let attachment_id = uploaded["attachment_id"].as_str().unwrap();
    let file = dir.path().join("download.bin");
    run(
        app,
        server,
        &[
            "attachment",
            "download",
            attachment_id,
            "--destination",
            file.to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(fs::read(file).unwrap(), bytes);
    let copied_card = run(
        app,
        server,
        &[
            "card",
            "copy",
            &c,
            "--board",
            &b,
            "--list",
            &l,
            "--target-board",
            &target,
            "--target-list",
            &target_list,
            "--target-swimlane-id",
            &target_lane,
        ],
    )
    .await
    .unwrap();
    assert!(copied_card.get("mutation_confirmed").is_none());
    assert_eq!(
        detail(app, server, copied_card["card_id"].as_str().unwrap()).await["title"],
        "Attachment copy"
    );
    run(
        app,
        server,
        &[
            "list",
            "copy",
            &l,
            "--board",
            &b,
            "--target-board",
            &target,
            "--target-swimlane-id",
            &target_lane,
        ],
    )
    .await
    .unwrap();
    let lane_copy = run(
        app,
        server,
        &[
            "swimlane",
            "copy",
            &s,
            "--board",
            &b,
            "--target-board",
            &target,
        ],
    )
    .await
    .unwrap();
    let lane_cards = run(
        app,
        server,
        &[
            "card",
            "list",
            "--board",
            &target,
            "--swimlane",
            lane_copy["returned_swimlane_id"].as_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(lane_cards["cards"].as_array().unwrap().len(), 1);
    let board_copy = run(app, server, &["board", "copy", &b]).await.unwrap();
    assert!(board_copy.get("mutation_confirmed").is_none());
    let copied_board = board_copy["board_id"].as_str().unwrap();
    assert_eq!(count(app, server, copied_board, None).await, 1);
    // W11-047 is fixed: each deep-copy operation creates an independently readable file.
    let result = fixture_command(
        "mongodb",
        "mongosh",
        r#"const w=db.getSiblingDB('wekan');print(JSON.stringify({copies:w.attachments.find({'meta.copyFrom':p.id},{_id:1,'meta.boardId':1}).toArray(),source:w.attachments.countDocuments({_id:p.id})}));"#,
        &json!({"id":attachment_id}),
    );
    assert_eq!(result["source"], 1);
    let copies = result["copies"].as_array().unwrap();
    assert_eq!(copies.len(), 4);
    assert_eq!(
        copies
            .iter()
            .filter(|a| a["meta"]["boardId"] == target)
            .count(),
        3
    );
    assert_eq!(
        copies
            .iter()
            .filter(|a| a["meta"]["boardId"] == copied_board)
            .count(),
        1
    );
    for attachment in copies {
        let id = attachment["_id"].as_str().unwrap();
        let path = dir.path().join(format!("{id}.bin"));
        run(
            app,
            server,
            &[
                "attachment",
                "download",
                id,
                "--destination",
                path.to_str().unwrap(),
            ],
        )
        .await
        .unwrap();
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    for id in [&b, &target, copied_board] {
        run(app, server, &["board", "delete", id, "--yes"])
            .await
            .unwrap();
    }
}
