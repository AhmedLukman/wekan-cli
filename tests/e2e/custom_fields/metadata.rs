//! Native v12.57 producers for multi-select fields and Jira integration metadata.
use crate::support::{
    attachment_fixtures::fixture_command,
    live::{LiveApp, run},
    *,
};
use serde_json::{Value, json};

async fn native_export(app: &LiveApp, server: &str, board: &str) -> Result<Value, AppError> {
    let cli = Cli::try_parse_from([
        "wekan",
        "--server",
        server,
        "--output",
        "raw",
        "board",
        "export",
        board,
        "--format",
        "json",
        "--destination",
        "-",
    ])
    .unwrap();
    cli.command.validate_output(OutputFormat::Raw).unwrap();
    let success = Box::pin(app.execute(cli.command)).await?;
    let mut bytes = Vec::new();
    let mut errors = Vec::new();
    write_success(OutputFormat::Raw, success, &mut bytes, &mut errors)
        .await
        .unwrap();
    assert!(errors.is_empty());
    Ok(serde_json::from_slice(&bytes).unwrap())
}

fn exported_field<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["customFields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|field| field["_id"] == id)
        .expect("the native export must retain the custom-field definition")
}

pub(super) async fn verify(server: &str) {
    let server = server.to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let directory = TestDirectory::new("native-custom-field-metadata");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("CustomFieldsMetadata-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    run(
        &app,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &format!("fieldmetadata_{nonce}"),
            "--password-stdin",
        ],
    )
    .await
    .unwrap();
    let board = run(
        &app,
        &server,
        &["board", "create", "--title", "Native multi-select fields"],
    )
    .await
    .unwrap();
    let board = board["board_id"].as_str().unwrap();
    let multi = run(
        &app,
        &server,
        &[
            "custom-field",
            "create",
            "--board",
            board,
            "--name",
            "Choices",
            "--type",
            "dropdownMultiSelect",
            "--settings",
            r#"{"dropdownItems":[{"_id":"a","name":"A"},{"_id":"b","name":"B"}]}"#,
        ],
    )
    .await
    .unwrap();
    let multi = multi["custom_field_id"].as_str().unwrap();
    let shown = run(
        &app,
        &server,
        &["custom-field", "get", multi, "--board", board],
    )
    .await
    .unwrap();
    assert_eq!(shown["field_type"], "dropdownMultiSelect");
    assert_eq!(shown["settings"]["dropdown_items"][0]["item_id"], "a");
    for field in [
        "jira_time_field",
        "jira_estimate_field_id",
        "jira_estimate_unit",
    ] {
        assert_eq!(shown["settings"][field], Value::Null);
    }
    let listed = run(&app, &server, &["custom-field", "list", "--board", board])
        .await
        .unwrap();
    assert_eq!(
        listed["custom_fields"][0]["field_type"],
        "dropdownMultiSelect"
    );
    let nullable = run(
        &app,
        &server,
        &[
            "custom-field",
            "create",
            "--board",
            board,
            "--name",
            "Explicit null metadata",
            "--type",
            "number",
            "--settings",
            r#"{"jiraTimeField":null,"jiraEstimateFieldId":null,"jiraEstimateUnit":null}"#,
        ],
    )
    .await
    .unwrap();
    let nullable = nullable["custom_field_id"].as_str().unwrap();
    let exported = native_export(&app, &server, board).await.unwrap();
    let exported_multi = exported_field(&exported, multi);
    assert_eq!(exported_multi["type"], "dropdownMultiSelect");
    assert_eq!(
        exported_multi["settings"]["dropdownItems"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for field in ["jiraTimeField", "jiraEstimateFieldId", "jiraEstimateUnit"] {
        assert!(exported_multi["settings"].get(field).is_none(), "{field}");
        assert_eq!(
            exported_field(&exported, nullable)["settings"].get(field),
            Some(&Value::Null),
            "{field}"
        );
    }

    // This is the actual Jira creator, not a seeded response or a DTO-only fixture.
    let input = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/requests/custom-fields-jira-v12.57.json");
    let imported = run(
        &app,
        &server,
        &[
            "board",
            "import",
            input.to_str().unwrap(),
            "--source",
            "jira",
        ],
    )
    .await
    .unwrap();
    let imported = imported["board_id"].as_str().unwrap();
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
    let (_reader_directory, reader, _) =
        register_live_board_role(&server, &canonical, &token, imported, nonce + 1, "normal").await;
    assert_eq!(
        run(&reader, &server, &["user", "current"]).await.unwrap()["is_admin"],
        false
    );
    let listed = run(
        &reader,
        &server,
        &["custom-field", "list", "--board", imported],
    )
    .await
    .unwrap();
    let fields = listed["custom_fields"].as_array().unwrap();
    assert_eq!(
        fields.len(),
        3,
        "Jira must create its numeric estimate and two time fields"
    );
    let mut estimate_id = None;
    for summary in fields {
        assert_eq!(summary["field_type"], "number");
        let id = summary["custom_field_id"].as_str().unwrap();
        let shown = run(
            &reader,
            &server,
            &["custom-field", "get", id, "--board", imported],
        )
        .await
        .unwrap();
        let settings = &shown["settings"];
        match summary["name"].as_str().unwrap() {
            "Jira estimate (points)" => {
                assert_eq!(settings["jira_estimate_field_id"], "customfield_10001");
                assert_eq!(settings["jira_estimate_unit"], "points");
                assert_eq!(settings["jira_time_field"], Value::Null);
                estimate_id = Some(id);
            }
            "Jira original estimate (hours)" => assert_eq!(settings["jira_time_field"], "original"),
            "Jira remaining estimate (hours)" => {
                assert_eq!(settings["jira_time_field"], "remaining")
            }
            name => panic!("unexpected Jira field {name}"),
        }
    }
    let estimate_id = estimate_id.unwrap();
    let exported = native_export(&reader, &server, imported).await.unwrap();
    let estimate_settings = exported_field(&exported, estimate_id)["settings"].clone();
    assert_eq!(
        estimate_settings,
        json!({"jiraEstimateFieldId":"customfield_10001","jiraEstimateUnit":"points"})
    );
    for field in exported["customFields"].as_array().unwrap() {
        if field["_id"] != estimate_id {
            assert!(matches!(
                field["settings"]["jiraTimeField"].as_str(),
                Some("original" | "remaining")
            ));
            assert!(field["settings"].get("jiraEstimateFieldId").is_none());
            assert!(field["settings"].get("jiraEstimateUnit").is_none());
        }
    }

    // Invalid extra fields and types must reach each production strict boundary.
    for invalid in [
        json!({"jiraEstimateFieldId":"customfield_10001","jiraEstimateUnit":"points","unmapped":"secret-value-sentinel"}),
        json!({"jiraEstimateFieldId":"customfield_10001","jiraEstimateUnit":42}),
    ] {
        fixture_command(
            "mongodb",
            "mongosh",
            "db.getSiblingDB('wekan').customFields.updateOne({_id:p.id},{$set:{settings:p.settings}});print('{}');",
            &json!({"id":estimate_id,"settings":invalid}),
        );
        let read = run(
            &reader,
            &server,
            &["custom-field", "get", estimate_id, "--board", imported],
        )
        .await;
        let export = native_export(&reader, &server, imported).await;
        fixture_command(
            "mongodb",
            "mongosh",
            "db.getSiblingDB('wekan').customFields.updateOne({_id:p.id},{$set:{settings:p.settings}});print('{}');",
            &json!({"id":estimate_id,"settings":estimate_settings}),
        );
        for error in [read.unwrap_err(), export.unwrap_err()] {
            assert_eq!(error.code(), ErrorCode::ProtocolError);
            assert!(!format!("{error:?}").contains("secret-value-sentinel"));
        }
    }
    run(
        &reader,
        &server,
        &["custom-field", "get", estimate_id, "--board", imported],
    )
    .await
    .unwrap();
    native_export(&reader, &server, imported).await.unwrap();
    for board in [board, imported] {
        run(&app, &server, &["board", "delete", board, "--yes"])
            .await
            .unwrap();
    }
}
