//! Complete pinned import/export catalogs, using actual server-generated artifacts.
use super::support::{
    attachment_fixtures::fixture_command,
    live::{LiveApp, run},
    *,
};
use clap::CommandFactory;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
};

fn choices(path: &[&str], argument: &str) -> Vec<String> {
    let mut root = Cli::command();
    root.build();
    let mut command = &root;
    for part in path {
        command = command.find_subcommand(part).unwrap();
    }
    command
        .get_arguments()
        .find(|a| a.get_id() == argument)
        .unwrap()
        .get_possible_values()
        .into_iter()
        .map(|v| v.get_name().to_owned())
        .collect()
}

async fn artifact(app: &LiveApp, server: &str, args: &[&str]) -> Vec<u8> {
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
    let mut bytes = Vec::new();
    let mut errors = Vec::new();
    write_success(OutputFormat::Raw, success, &mut bytes, &mut errors)
        .await
        .unwrap();
    assert!(errors.is_empty());
    assert!(!bytes.is_empty(), "{args:?}");
    bytes
}

fn archive(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut files = BTreeMap::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).unwrap();
        if entry.is_dir() {
            continue;
        }
        let mut data = Vec::new();
        entry
            .read_to_end(&mut data)
            .expect("each entry must decompress and pass CRC");
        files.insert(entry.name().unwrap().into_owned(), data);
    }
    assert!(!files.is_empty());
    files
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v12.57 stack and WEKAN_CATALOG_E2E_URL"]
async fn every_native_import_export_and_chart_format_matches_wekan_v12_57() {
    let server =
        env::var("WEKAN_CATALOG_E2E_URL").expect("set WEKAN_CATALOG_E2E_URL to an isolated stack");
    // Mass export selects the caller's memberships even when boards are public.
    // Its native authToken alternative does not make authentication optional.
    let anonymous = reqwest::Client::new()
        .get(format!(
            "{}/api/export-all-boards/wekan",
            server.trim_end_matches('/')
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(anonymous.status(), 401);
    assert_eq!(
        anonymous.json::<Value>().await.unwrap(),
        json!({"error":"Unauthorized"})
    );
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let directory = TestDirectory::new("native-catalog");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Catalog-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let user = run(
        &app,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &format!("catalog_{nonce}"),
            "--password-stdin",
        ],
    )
    .await
    .unwrap();
    let made = run(
        &app,
        &server,
        &["board", "create", "--title", "Catalog board"],
    )
    .await
    .unwrap();
    let board = made["board_id"].as_str().unwrap();
    let lane = made["default_swimlane_id"].as_str().unwrap();
    let made_list = run(
        &app,
        &server,
        &["list", "create", "--board", board, "--title", "Todo"],
    )
    .await
    .unwrap();
    let list = made_list["list_id"].as_str().unwrap();
    let made_card = run(
        &app,
        &server,
        &[
            "card",
            "create",
            "--board",
            board,
            "--list",
            list,
            "--swimlane-id",
            lane,
            "--title",
            "Catalog card",
        ],
    )
    .await
    .unwrap();
    let card = made_card["card_id"].as_str().unwrap();
    for auth in [None, Some("--auth-token-query")] {
        let mut args = vec!["board", "export-all", "--board", board, "--format", "wekan"];
        if let Some(auth) = auth {
            args.push(auth);
        }
        let files = archive(&artifact(&app, &server, &args).await);
        assert_eq!(files.len(), 1);
        let document: Value = serde_json::from_slice(files.values().next().unwrap()).unwrap();
        assert_eq!(document["_id"], board);
        assert!(
            document["cards"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value["_id"] == card)
        );
    }
    run(
        &app,
        &server,
        &[
            "card",
            "update",
            card,
            "--board",
            board,
            "--list",
            list,
            "--due-at",
            "2026-11-01T12:00:00Z",
        ],
    )
    .await
    .unwrap();

    let formats = choices(&["board", "export"], "format");
    assert_eq!(formats.len(), 48);
    let mut exported = BTreeMap::new();
    for format in formats {
        let bytes = artifact(
            &app,
            &server,
            &["board", "export", board, "--format", &format],
        )
        .await;
        if bytes.starts_with(b"PK") {
            archive(&bytes);
        }
        if format == "pdf" {
            assert!(bytes.starts_with(b"%PDF-"));
            assert!(bytes.windows(5).any(|w| w == b"%%EOF"));
        }
        if format == "ics" {
            let text = std::str::from_utf8(&bytes).unwrap();
            assert!(text.contains("BEGIN:VEVENT"));
            assert!(text.contains("Catalog card"));
        }
        let file = directory.path().join(format!("export-{format}.artifact"));
        fs::write(&file, bytes).unwrap();
        exported.insert(format, file);
    }
    let mass_formats = choices(&["board", "export-all"], "format");
    assert_eq!(mass_formats.len(), 45);
    for (index, format) in mass_formats.into_iter().enumerate() {
        if matches!(format.as_str(), "csv" | "scsv" | "tsv") {
            // W12-003: the archive names the failed board in skipped.txt, and
            // the erroneous non-plain-object security event disables its actor.
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
            let (_actor_dir, actor, actor_id) = register_live_board_role(
                &server,
                &canonical,
                &token,
                board,
                nonce + 100 + index as u128,
                "normal",
            )
            .await;
            let bytes = artifact(
                &actor,
                &server,
                &["board", "export-all", "--board", board, "--format", &format],
            )
            .await;
            let files = archive(&bytes);
            assert_eq!(files.len(), 1);
            let skipped = std::str::from_utf8(files.get("skipped.txt").unwrap()).unwrap();
            assert!(skipped.contains("Import/export non-plain object at /lists/0"));
            let blocked = fixture_command(
                "mongodb",
                "mongosh",
                "print(JSON.stringify(db.getSiblingDB('wekan').users.findOne({_id:p.id}).loginDisabled===true));",
                &json!({"id":actor_id}),
            );
            assert_eq!(blocked, true);
            continue;
        }
        let bytes = artifact(
            &app,
            &server,
            &["board", "export-all", "--board", board, "--format", &format],
        )
        .await;
        let files = archive(&bytes);
        if format == "excel" {
            assert!(files.contains_key("xl/workbook.xml"));
        } else {
            assert_eq!(
                files.len(),
                1,
                "one selected board in {format}: {:?}",
                files.keys()
            );
        }
    }

    let sources = choices(&["board", "import"], "source");
    assert_eq!(sources.len(), 49);
    for source in sources {
        let format = match source.as_str() {
            "wekan" => "json",
            "excel" => "xlsx",
            other => other,
        };
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/requests/imports/v12.57")
            .join(format!("{source}.json"));
        let file = exported.get(format).unwrap_or(&fixture);
        let mut args = vec![
            "board",
            "import",
            file.to_str().unwrap(),
            "--source",
            &source,
        ];
        if source == "vikunja" {
            args.extend(["--input-format", "zip"]);
        }
        let result = run(&app, &server, &args)
            .await
            .unwrap_or_else(|e| panic!("{source}: {e:?}"));
        let imported = result["board_id"].as_str().unwrap();
        let detail = run(&app, &server, &["board", "get", imported])
            .await
            .unwrap();
        assert_eq!(detail["board_id"], imported);
        let saved = fixture_command(
            "mongodb",
            "mongosh",
            "const w=db.getSiblingDB('wekan');print(JSON.stringify({cards:w.cards.find({boardId:p.board},{title:1}).toArray(),lists:w.lists.countDocuments({boardId:p.board})}));",
            &json!({"board":imported}),
        );
        assert!(saved["lists"].as_u64().unwrap() > 0, "{source}: {saved}");
        assert!(
            saved["cards"].as_array().unwrap().iter().any(|c| {
                let title = c["title"].as_str().unwrap();
                if source == "jira" {
                    title.starts_with('[') && title.ends_with("] Catalog card")
                } else {
                    title == "Catalog card"
                }
            }),
            "{source}: {saved}"
        );
        run(&app, &server, &["board", "delete", imported, "--yes"])
            .await
            .unwrap();
    }

    // Source-normalized native Scrum metadata, including snapshots and daily history.
    let mut transfer: Value = serde_json::from_str(include_str!(
        "../fixtures/responses/exports/v12.57/scrum-transfer.json"
    ))
    .unwrap();
    let mut serialized = serde_json::to_string(&transfer).unwrap();
    for (from, to) in [
        ("card1", card),
        ("card2", card),
        ("list1", list),
        ("lane1", lane),
        ("user1", user["user_id"].as_str().unwrap()),
    ] {
        serialized = serialized.replace(&format!("\"{from}\""), &format!("\"{to}\""));
    }
    transfer = serde_json::from_str(&serialized).unwrap();
    transfer["settings"]["enabled"] = json!(true);
    // Avoid duplicate rows when the fixture's second snapshot card maps to the one live card.
    for sprint in transfer["sprints"].as_array_mut().unwrap() {
        for key in ["startSnapshot", "closeSnapshot"] {
            if let Some(snapshot) = sprint.get_mut(key) {
                snapshot["cards"].as_array_mut().unwrap().truncate(1);
                snapshot["missingEstimates"] = json!(0);
            }
        }
    }
    for observation in transfer["dailyObservations"].as_array_mut().unwrap() {
        observation["snapshot"]["cards"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        observation["snapshot"]["missingEstimates"] = json!(0);
    }
    fixture_command(
        "mongodb",
        "mongosh",
        r#"
      const w=db.getSiblingDB('wekan'),t=p.transfer;
      function dates(value){if(Array.isArray(value))return value.map(dates);if(value&&typeof value==='object')return Object.fromEntries(Object.entries(value).map(([k,v])=>[k,dates(v)]));return typeof value==='string'&&/^\d{4}-\d\d-\d\dT/.test(value)?new Date(value):value;}
      w.boards.updateOne({_id:p.board},{$set:{scrum:t.settings}});
      for(const [collection,rows] of [['cards',t.cards],['lists',t.lists],['swimlanes',t.swimlanes]])for(const row of rows)w[collection].updateOne({_id:row._id},{$set:{scrum:row.scrum}});
      for(const [collection,rows] of [['scrumSprints',t.sprints],['scrumReleases',t.releases],['scrumEvents',t.events]])for(const row of rows)w[collection].insertOne({...dates(row),boardId:p.board,revision:1});
      for(const row of t.dailyObservations)w.scrumDailySnapshots.insertOne({...dates(row),boardId:p.board});
      print('{}');"#,
        &json!({"board":board,"transfer":transfer}),
    );
    let rich = artifact(
        &app,
        &server,
        &["board", "export", board, "--format", "json"],
    )
    .await;
    let rich: Value = serde_json::from_slice(&rich).unwrap();
    assert_eq!(rich["scrumTransfer"]["format"], "wekan-scrum-2");
    assert_eq!(
        rich["scrumTransfer"]["sprints"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        rich["scrumTransfer"]["dailyObservations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let charts = choices(&["board", "chart", "export"], "chart");
    assert_eq!(charts.len(), 21);
    for chart in charts {
        for format in ["pdf", "xlsx"] {
            let bytes = artifact(
                &app,
                &server,
                &[
                    "board", "chart", "export", &chart, "--board", board, "--format", format,
                    "--sprint", "sprint1",
                ],
            )
            .await;
            if format == "xlsx" {
                assert!(archive(&bytes).contains_key("xl/workbook.xml"));
            } else {
                assert!(bytes.starts_with(b"%PDF-"));
                assert!(bytes.windows(5).any(|w| w == b"%%EOF"));
            }
        }
    }
    run(&app, &server, &["board", "delete", board, "--yes"])
        .await
        .unwrap();
}
