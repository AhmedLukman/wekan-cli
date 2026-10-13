use super::support::*;
use serde_json::{Value, json};
use wekan_cli::command_result::{CustomFieldDetail, CustomFieldSubmittedField};

struct Target<'a> {
    app: &'a App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &'a str,
    board: &'a str,
    list: &'a str,
    card: &'a str,
}
impl Target<'_> {
    async fn run(&self, args: &[&str]) -> Result<CommandSuccess, AppError> {
        let mut argv = vec!["wekan", "--server", self.server, "--output", "json"];
        argv.extend_from_slice(args);
        argv.extend(["--board", self.board]);
        self.app
            .execute(Cli::try_parse_from(argv).unwrap().command)
            .await
    }
    async fn field(&self, args: &[&str]) -> Result<CommandSuccess, AppError> {
        let mut argv = vec!["custom-field"];
        argv.extend_from_slice(args);
        self.run(&argv).await
    }
    async fn item(&self, field: &str, args: &[&str]) -> Result<CommandSuccess, AppError> {
        let mut argv = vec!["custom-field-item"];
        argv.extend_from_slice(args);
        argv.extend(["--custom-field", field]);
        self.run(&argv).await
    }
    async fn card(&self, args: &[&str]) -> Result<CommandSuccess, AppError> {
        let mut argv = vec!["card"];
        argv.extend_from_slice(args);
        argv.extend(["--list", self.list]);
        self.run(&argv).await
    }
    async fn detail(&self, field: &str) -> CustomFieldDetail {
        let CommandSuccess::CustomFieldShown(detail) = self.field(&["get", field]).await.unwrap()
        else {
            panic!("expected field detail")
        };
        detail
    }
    async fn card_detail(&self) -> wekan_cli::command_result::CardDetail {
        let CommandSuccess::CardShown(detail) = self.card(&["get", self.card]).await.unwrap()
        else {
            panic!("expected card detail")
        };
        *detail
    }
    async fn find(&self, field: &str, value: &str) -> Vec<wekan_cli::command_result::CardDetail> {
        let CommandSuccess::CardCustomFieldQuery(result) = self
            .run(&[
                "card",
                "find-by-custom-field",
                "--custom-field",
                field,
                "--value",
                value,
            ])
            .await
            .unwrap()
        else {
            panic!("expected card query")
        };
        assert_eq!(result.value, value);
        result.cards
    }
}

fn mongo(script: &str) -> Value {
    let output = Command::new("docker")
        .args([
            "compose", "exec", "-T", "mongodb", "mongosh", "--quiet", "--eval", script,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn assert_deletion_acknowledged(
    result: &CommandSuccess,
    board: &str,
    field: &str,
    item: Option<&str>,
) {
    let rendered = wekan_cli::output::render_success(OutputFormat::Json, result);
    let mut expected = json!({
        "board_id": board,
        "custom_field_id": field,
        "deletion_requested": true,
        "mutation_confirmed": false,
        "delete_mode": "hard",
    });
    if let Some(item) = item {
        expected["item_id"] = json!(item);
    }
    assert_eq!(
        serde_json::from_str::<Value>(&rendered).unwrap(),
        json!({"ok":true,"data":expected})
    );
}

fn assert_missing_definition_metadata(error: &AppError) {
    assert_eq!(error.code(), ErrorCode::NotFound);
    assert_eq!(error.details().http_status, Some(200));
    assert!(error.details().wekan_status_code.is_none());
    assert!(error.details().server_error.is_none());
    assert!(error.details().server_reason.is_none());
}

#[tokio::test]
#[ignore = "destructive: requires an isolated Wekan v11.06 stack and WEKAN_CUSTOM_FIELD_E2E_URL"]
async fn complete_custom_field_lifecycle_matches_wekan_v11_06() {
    let server = env::var("WEKAN_CUSTOM_FIELD_E2E_URL")
        .expect("set WEKAN_CUSTOM_FIELD_E2E_URL to an isolated Wekan v11.06 stack");
    let canonical = ServerUrl::parse(&server).unwrap().as_str().to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let directory = TestDirectory::new("custom-field-lifecycle");
    let username = format!("customfields_{nonce}");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".to_owned()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("CustomFields-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    app.execute(
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
    let CommandSuccess::BoardCreated(board) =
        execute_live_board(&app, &server, &["create", "--title", "Custom fields"])
            .await
            .unwrap()
    else {
        panic!("expected board")
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
        panic!("expected list")
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
            "Field target",
        ],
    )
    .await
    .unwrap() else {
        panic!("expected card")
    };
    let target = Target {
        app: &app,
        server: &server,
        board: &board.board_id,
        list: &list.list_id,
        card: &card.card_id,
    };
    let CommandSuccess::CustomFieldCollection(empty) = target.field(&["list"]).await.unwrap()
    else {
        panic!("expected collection")
    };
    assert!(empty.custom_fields.is_empty());
    let mut fields = Vec::new();
    for field_type in [
        "text",
        "number",
        "date",
        "dropdown",
        "checkbox",
        "currency",
        "stringtemplate",
    ] {
        let settings = match field_type {
            "currency" => r#"{"currencyCode":"USD"}"#,
            "dropdown" => r#"{"dropdownItems":[{"_id":"initial","name":"Initial"}]}"#,
            "stringtemplate" => r#"{"stringtemplateFormat":"{$a}","stringtemplateSeparator":", "}"#,
            _ => "{}",
        };
        let CommandSuccess::CustomFieldCreated(created) = target
            .field(&[
                "create",
                "--name",
                field_type,
                "--type",
                field_type,
                "--settings",
                settings,
                "--show-on-card",
                "true",
                "--automatically-on-card",
                "false",
                "--show-label-on-mini-card",
                "true",
                "--show-sum-at-top-of-list",
                "false",
            ])
            .await
            .unwrap()
        else {
            panic!("expected creation")
        };
        let detail = target.detail(&created.custom_field_id).await;
        assert_eq!(detail.field_type, field_type);
        assert_eq!(detail.name, field_type);
        assert_eq!(detail.board_ids, vec![board.board_id.clone()]);
        assert!(detail.show_on_card);
        assert!(detail.show_label_on_mini_card);
        assert!(!detail.automatically_on_card);
        assert!(!detail.always_on_card);
        assert!(!detail.show_sum_at_top_of_list);
        assert!(detail.created_at.is_some());
        fields.push(created.custom_field_id);
    }
    let text = &fields[0];
    let dropdown = &fields[3];
    let CommandSuccess::CustomFieldCollection(collection) = target.field(&["list"]).await.unwrap()
    else {
        panic!("expected collection")
    };
    assert_eq!(collection.custom_fields.len(), 7);
    for (index, summary) in collection.custom_fields.iter().enumerate() {
        assert_eq!(summary.custom_field_id, fields[index]);
    }

    let http = reqwest::Client::new();
    // Pinned Wekan accepts these supplied IDs, but the URL library drops dot
    // segments. Raw creation records the server behavior; typed reads stay strict.
    for (supplied, stored) in [
        (".", "."),
        ("..", ".."),
        (" . ", "."),
        ("\u{feff}..\u{feff}", ".."),
    ] {
        let created: wekan_cli::client::CustomFieldIdResponse = http
            .post(format!(
                "{server}/api/boards/{}/custom-fields",
                target.board
            ))
            .bearer_auth(&token)
            .json(&json!({
                "name": "Dot ID fixture", "type": "dropdown",
                "settings": {"dropdownItems": [{"_id": supplied, "name": "Option"}]},
            }))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        let items = target
            .detail(&created.id)
            .await
            .settings
            .dropdown_items
            .unwrap();
        assert_eq!(items[0].item_id, stored);
        for verb in ["update", "delete"] {
            assert!(
                Cli::try_parse_from([
                    "wekan",
                    "custom-field-item",
                    verb,
                    stored,
                    "--board",
                    target.board,
                    "--custom-field",
                    &created.id,
                ])
                .is_err()
            );
        }
        target
            .field(&["delete", &created.id, "--yes"])
            .await
            .unwrap();
    }
    // U+0085 remains part of Wekan IDs. Keep the wrapped option before its
    // plain counterpart so deleting it must leave the plain option intact.
    let addressable_ids = [
        "...",
        "%2E",
        "./option",
        "a/../b",
        "\u{85}option\u{85}",
        "option",
        "\u{85}",
        "\u{85}.\u{85}",
    ];
    let settings = json!({"dropdownItems": addressable_ids.iter().map(|id| {
        json!({"_id":id,"name":"Option"})
    }).collect::<Vec<_>>()})
    .to_string();
    let CommandSuccess::CustomFieldCreated(addressable_field) = target
        .field(&[
            "create",
            "--name",
            "Encoded ID fixture",
            "--type",
            "dropdown",
            "--settings",
            &settings,
        ])
        .await
        .unwrap()
    else {
        panic!("expected field creation")
    };
    for id in addressable_ids {
        target
            .item(
                &addressable_field.custom_field_id,
                &["update", id, "--name", "Changed"],
            )
            .await
            .unwrap();
    }
    let items = target
        .detail(&addressable_field.custom_field_id)
        .await
        .settings
        .dropdown_items
        .unwrap();
    assert_eq!(
        items
            .iter()
            .map(|item| item.item_id.as_str())
            .collect::<Vec<_>>(),
        addressable_ids
    );
    assert!(items.iter().all(|item| item.name == "Changed"));
    for (index, id) in addressable_ids.iter().enumerate() {
        target
            .item(&addressable_field.custom_field_id, &["delete", id, "--yes"])
            .await
            .unwrap();
        let remaining = target
            .detail(&addressable_field.custom_field_id)
            .await
            .settings
            .dropdown_items
            .unwrap();
        assert_eq!(
            remaining
                .iter()
                .map(|item| item.item_id.as_str())
                .collect::<Vec<_>>(),
            addressable_ids[index + 1..],
            "deleting {id:?} must remove only the selected option"
        );
    }
    target
        .field(&["delete", &addressable_field.custom_field_id, "--yes"])
        .await
        .unwrap();

    let nullable_settings = json!({
        "currencyCode": null,
        "dropdownItems": null,
        "stringtemplateFormat": null,
        "stringtemplateSeparator": null,
    });
    let nullable_input = nullable_settings.to_string();
    let CommandSuccess::CustomFieldCreated(nullable_field) = target
        .field(&[
            "create",
            "--name",
            "Nullable settings",
            "--type",
            "text",
            "--settings",
            &nullable_input,
        ])
        .await
        .unwrap()
    else {
        panic!("expected nullable field creation")
    };
    let nullable_id = &nullable_field.custom_field_id;
    for (index, settings) in [nullable_settings.clone(), json!({}), nullable_settings]
        .into_iter()
        .enumerate()
    {
        // The first read verifies creation; subsequent writes verify replacement
        // with omitted properties and explicit nulls without silently dropping nulls.
        if index > 0 {
            target
                .field(&["update", nullable_id, "--settings", &settings.to_string()])
                .await
                .unwrap();
        }
        let raw: Value = http
            .get(format!(
                "{server}/api/boards/{}/custom-fields/{nullable_id}",
                target.board
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(raw["settings"], settings);
        let detail = target.detail(nullable_id).await;
        assert!(detail.settings.currency_code.is_none());
        assert!(detail.settings.dropdown_items.is_none());
        assert!(detail.settings.stringtemplate_format.is_none());
        assert!(detail.settings.stringtemplate_separator.is_none());
        let rendered: Value = serde_json::from_str(&wekan_cli::output::render_success(
            OutputFormat::Json,
            &CommandSuccess::CustomFieldShown(detail),
        ))
        .unwrap();
        assert_eq!(
            rendered["data"]["settings"],
            json!({
                "currency_code": null,
                "dropdown_items": null,
                "stringtemplate_format": null,
                "stringtemplate_separator": null,
            })
        );
    }
    target
        .field(&["delete", nullable_id, "--yes"])
        .await
        .unwrap();

    let CommandSuccess::CustomFieldUpdated(updated) =
        target.field(&["update", text]).await.unwrap()
    else {
        panic!("expected update")
    };
    assert!(updated.submitted_fields.is_empty());
    for args in [
        vec!["create"],
        vec!["create", "--items", "null"],
        vec!["create", "--items", "{}"],
        vec!["create", "--items", r#""ignored""#],
    ] {
        target.item(dropdown, &args).await.unwrap();
    }
    assert_eq!(
        target
            .detail(dropdown)
            .await
            .settings
            .dropdown_items
            .unwrap()
            .len(),
        1
    );
    let CommandSuccess::CustomFieldItemsCreated(ack) = target
        .item(
            dropdown,
            &["create", "--items", r#"["Ready",42,null,{},"Done"]"#],
        )
        .await
        .unwrap()
    else {
        panic!("expected creation")
    };
    assert_eq!(&ack.custom_field_id, dropdown);
    target
        .item(dropdown, &["create", "--item", "Extra", "--item", "Last"])
        .await
        .unwrap();
    let items = target
        .detail(dropdown)
        .await
        .settings
        .dropdown_items
        .unwrap();
    assert_eq!(
        items
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Initial", "Ready", "Done", "Extra", "Last"]
    );
    for item in &items[1..] {
        assert_eq!(item.item_id.len(), 6);
    }
    target
        .item(dropdown, &["update", &items[1].item_id])
        .await
        .unwrap();
    target
        .item(
            dropdown,
            &["update", &items[1].item_id, "--name", "Renamed"],
        )
        .await
        .unwrap();
    assert_eq!(
        target
            .detail(dropdown)
            .await
            .settings
            .dropdown_items
            .unwrap()[1]
            .name,
        "Renamed"
    );
    for _ in 0..2 {
        let result = target
            .item(dropdown, &["delete", &items[1].item_id, "--yes"])
            .await
            .unwrap();
        assert_deletion_acknowledged(&result, target.board, dropdown, Some(&items[1].item_id));
    }
    assert_eq!(
        target
            .detail(dropdown)
            .await
            .settings
            .dropdown_items
            .unwrap()
            .len(),
        4
    );
    target
        .field(&[
            "update",
            dropdown,
            "--settings",
            "{}",
            "--always-on-card",
            "true",
            "--automatically-on-card",
            "true",
        ])
        .await
        .unwrap();
    assert!(
        target
            .detail(dropdown)
            .await
            .settings
            .dropdown_items
            .is_none()
    );
    assert!(
        target.card_detail().await.custom_fields.is_empty(),
        "REST direct updates bypass alwaysOnCard population hooks"
    );
    let seed = json!([{"_id":text,"value":"Initial"},{"_id":dropdown,"value":"Other"}]).to_string();
    target
        .card(&["update", target.card, "--custom-fields", &seed])
        .await
        .unwrap();

    for value in [
        json!(" Ready/#?% "),
        json!(2.5),
        json!(true),
        json!(["a", null, "b"]),
        json!(null),
    ] {
        let encoded = value.to_string();
        let expected = if let Value::String(text) = &value {
            json!(text.trim())
        } else {
            value.clone()
        };
        let CommandSuccess::CardCustomFieldSet(result) = target
            .card(&[
                "set-custom-field",
                target.card,
                "--custom-field",
                text,
                "--value-json",
                &encoded,
            ])
            .await
            .unwrap()
        else {
            panic!("expected assignment")
        };
        assert_eq!(result.custom_fields.len(), 2);
        assert_eq!(
            serde_json::to_value(&result.custom_fields[0].value).unwrap(),
            expected
        );
        assert_eq!(
            serde_json::to_value(&result.custom_fields[1].value).unwrap(),
            json!("Other")
        );
        assert_eq!(
            serde_json::to_value(&target.card_detail().await.custom_fields[0].value).unwrap(),
            expected
        );
    }
    target
        .card(&["set-custom-field", target.card, "--custom-field", text])
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&target.card_detail().await.custom_fields[0].value).unwrap(),
        json!("")
    );
    target
        .card(&[
            "set-custom-field",
            target.card,
            "--custom-field",
            text,
            "--value",
            " Ready/#?% ",
        ])
        .await
        .unwrap();
    assert!(target.find(text, " Ready/#?% ").await.is_empty());
    assert_eq!(target.find(text, "Ready/#?%").await.len(), 1);
    for value in ["...", "%2E", "./value", "a/../b"] {
        target
            .card(&[
                "set-custom-field",
                target.card,
                "--custom-field",
                text,
                "--value",
                value,
            ])
            .await
            .unwrap();
        assert_eq!(target.find(text, value).await[0].card_id, target.card);
    }
    for value in [".", ".."] {
        target
            .card(&[
                "set-custom-field",
                target.card,
                "--custom-field",
                text,
                "--value",
                value,
            ])
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(&target.card_detail().await.custom_fields[0].value).unwrap(),
            json!(value)
        );
        assert!(
            Cli::try_parse_from([
                "wekan",
                "card",
                "find-by-custom-field",
                "--board",
                target.board,
                "--custom-field",
                text,
                "--value",
                value,
            ])
            .is_err()
        );
    }
    target
        .card(&[
            "set-custom-field",
            target.card,
            "--custom-field",
            text,
            "--value-json",
            "2.5",
        ])
        .await
        .unwrap();
    assert!(target.find(text, "2.5").await.is_empty());
    target
        .card(&[
            "set-custom-field",
            target.card,
            "--custom-field",
            text,
            "--value-json",
            r#"["a","b"]"#,
        ])
        .await
        .unwrap();
    assert_eq!(target.find(text, "b").await.len(), 1);
    let before = target.card_detail().await.custom_fields;
    target
        .card(&[
            "set-custom-field",
            target.card,
            "--custom-field",
            "unattached",
            "--value",
            "Ignored",
        ])
        .await
        .unwrap();
    assert_eq!(target.card_detail().await.custom_fields, before);
    for (input, stored) in [
        (r#"{"x":true}"#, json!("")),
        (
            r#"[1,true,null,{"x":1},"word"]"#,
            json!(["1", "true", null, "word"]),
        ),
    ] {
        let result = target
            .card(&[
                "set-custom-field",
                target.card,
                "--custom-field",
                text,
                "--value-json",
                input,
            ])
            .await;
        if let CommandSuccess::CardCustomFieldSet(result) = result.unwrap() {
            assert_eq!(
                serde_json::to_value(&result.custom_fields[0].value).unwrap(),
                stored
            );
        } else {
            panic!("expected cleaned typed assignment response");
        }
        assert_eq!(
            serde_json::to_value(&target.card_detail().await.custom_fields[0].value).unwrap(),
            stored
        );
    }
    target
        .card(&[
            "set-custom-field",
            target.card,
            "--custom-field",
            text,
            "--value",
            "Archived",
        ])
        .await
        .unwrap();
    target.card(&["archive", target.card]).await.unwrap();
    assert!(target.find(text, "Archived").await.is_empty());
    let error = target
        .card(&[
            "set-custom-field",
            target.card,
            "--custom-field",
            text,
            "--value",
            "No",
        ])
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ServerError);
    assert_eq!(error.details().http_status, Some(500));
    target.card(&["restore", target.card]).await.unwrap();

    let error = target
        .card(&[
            "set-custom-field",
            target.card,
            "--custom-field",
            text,
            "--value-json",
            "[[1]]",
        ])
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ServerError);
    assert_eq!(error.details().outcome_unknown, Some(true));
    // A later invalid settings update can fail after the earlier name update landed.
    let error = target
        .field(&[
            "update",
            text,
            "--name",
            "Partial saved",
            "--settings",
            r#"{"dropdownItems":[{"_id":"x","name":""}]}"#,
        ])
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ServerError);
    assert_eq!(error.details().outcome_unknown, Some(true));
    assert_eq!(target.detail(text).await.name, "Partial saved");
    assert_missing_definition_metadata(&target.field(&["get", "missing"]).await.unwrap_err());
    target
        .field(&["update", "missing", "--name", "Ignored"])
        .await
        .unwrap();
    let result = target.field(&["delete", "missing", "--yes"]).await.unwrap();
    assert_deletion_acknowledged(&result, target.board, "missing", None);
    target
        .item("missing", &["create", "--item", "Ignored"])
        .await
        .unwrap();
    target
        .item("missing", &["update", "missing", "--name", "Ignored"])
        .await
        .unwrap();
    let result = target
        .item("missing", &["delete", "missing", "--yes"])
        .await
        .unwrap();
    assert_deletion_acknowledged(&result, target.board, "missing", Some("missing"));
    let wrong = Target {
        board: "missing-board",
        ..target
    };
    assert_eq!(
        wrong.field(&["list"]).await.unwrap_err().code(),
        ErrorCode::NotFound
    );
    let wrong = Target {
        list: "missing-list",
        ..target
    };
    assert_eq!(
        wrong
            .card(&[
                "set-custom-field",
                target.card,
                "--custom-field",
                text,
                "--value",
                "No"
            ])
            .await
            .unwrap_err()
            .code(),
        ErrorCode::ServerError
    );
    let wrong = Target {
        card: "missing-card",
        ..target
    };
    assert_eq!(
        wrong
            .card(&["set-custom-field", wrong.card, "--custom-field", text])
            .await
            .unwrap_err()
            .code(),
        ErrorCode::ServerError
    );

    let creator = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .2
        .clone();
    // Promote only this suite's disposable user so the admin-bypass case also
    // works when rerunning on an isolated stack with previously registered users.
    assert_eq!(
        mongo(&format!(
            "const r=db.getSiblingDB('wekan').users.updateOne({{_id:{}}},{{$set:{{isAdmin:true}}}});print(JSON.stringify(r.matchedCount));",
            json!(creator)
        )),
        1
    );
    // Check every route's role gates through typed commands, including site-admin bypass.
    for role in ["normal", "readonly", "commentonly", "worker", "nocomments"] {
        let (_directory, role_app, _) =
            register_live_board_role(&server, &canonical, &token, &board.board_id, nonce, role)
                .await;
        let role_target = Target {
            app: &role_app,
            ..target
        };
        if role == "normal" {
            let CommandSuccess::BoardCreated(unjoined) =
                execute_live_board(&role_app, &server, &["create", "--title", "Admin bypass"])
                    .await
                    .unwrap()
            else {
                panic!("expected board")
            };
            let unjoined_target = Target {
                board: &unjoined.board_id,
                ..target
            };
            unjoined_target.field(&["list"]).await.unwrap();
            let CommandSuccess::CustomFieldCreated(field) = unjoined_target
                .field(&["create", "--name", "Admin bypass field", "--type", "text"])
                .await
                .unwrap()
            else {
                panic!("expected field")
            };
            unjoined_target.detail(&field.custom_field_id).await;
            unjoined_target
                .field(&["delete", &field.custom_field_id, "--yes"])
                .await
                .unwrap();
            execute_live_board(&app, &server, &["delete", &unjoined.board_id, "--yes"])
                .await
                .unwrap();
        }
        let read_allowed = matches!(role, "normal" | "readonly");
        for args in [
            vec!["custom-field", "list"],
            vec!["custom-field", "get", text],
            vec![
                "card",
                "find-by-custom-field",
                "--custom-field",
                text,
                "--value",
                "Archived",
            ],
        ] {
            let result = role_target.run(&args).await;
            if read_allowed {
                result.unwrap();
            } else {
                assert_eq!(
                    result.unwrap_err().code(),
                    ErrorCode::PermissionDenied,
                    "{role}"
                );
            }
        }
        for args in [
            vec![
                "custom-field",
                "create",
                "--name",
                "Role field",
                "--type",
                "text",
            ],
            vec!["custom-field", "update", "missing"],
            vec!["custom-field", "delete", "missing", "--yes"],
            vec!["custom-field-item", "create", "--custom-field", dropdown],
            vec![
                "custom-field-item",
                "update",
                "missing",
                "--custom-field",
                dropdown,
            ],
            vec![
                "custom-field-item",
                "delete",
                "missing",
                "--custom-field",
                dropdown,
                "--yes",
            ],
            vec![
                "card",
                "set-custom-field",
                target.card,
                "--list",
                target.list,
                "--custom-field",
                "missing",
            ],
        ] {
            let result = role_target.run(&args).await;
            if role == "normal" {
                result.unwrap();
            } else {
                assert_eq!(
                    result.unwrap_err().code(),
                    ErrorCode::PermissionDenied,
                    "{role}"
                );
            }
        }
    }

    let CommandSuccess::BoardCreated(second) =
        execute_live_board(&app, &server, &["create", "--title", "Shared field board"])
            .await
            .unwrap()
    else {
        panic!("expected board")
    };
    let second_target = Target {
        board: &second.board_id,
        ..target
    };
    assert_missing_definition_metadata(&second_target.field(&["get", text]).await.unwrap_err());
    second_target
        .field(&["update", text, "--name", "Ignored wrong board"])
        .await
        .unwrap();
    let result = second_target
        .field(&["delete", text, "--yes"])
        .await
        .unwrap();
    assert_deletion_acknowledged(&result, second_target.board, text, None);
    assert_eq!(target.detail(text).await.name, "Partial saved");
    target
        .item(
            dropdown,
            &["create", "--item", "Kept after wrong-board delete"],
        )
        .await
        .unwrap();
    let kept_item = target
        .detail(dropdown)
        .await
        .settings
        .dropdown_items
        .unwrap()[0]
        .clone();
    let result = second_target
        .item(dropdown, &["delete", &kept_item.item_id, "--yes"])
        .await
        .unwrap();
    assert_deletion_acknowledged(
        &result,
        second_target.board,
        dropdown,
        Some(&kept_item.item_id),
    );
    assert_eq!(
        target
            .detail(dropdown)
            .await
            .settings
            .dropdown_items
            .unwrap(),
        vec![kept_item]
    );
    // Shared board membership is a legitimate definition shape, without a REST attach operation.
    assert_eq!(
        mongo(&format!(
            "const r=db.getSiblingDB('wekan').customFields.updateOne({{_id:{}}},{{$push:{{boardIds:{}}}}});print(JSON.stringify(r.matchedCount));",
            json!(text),
            json!(second.board_id)
        )),
        1
    );
    assert_eq!(second_target.detail(text).await.board_ids.len(), 2);
    let result = second_target
        .field(&["delete", text, "--yes"])
        .await
        .unwrap();
    assert_deletion_acknowledged(&result, second_target.board, text, None);
    assert!(
        wekan_cli::output::render_success(OutputFormat::Human, &result).contains("acknowledged")
    );
    assert_missing_definition_metadata(&target.field(&["get", text]).await.unwrap_err());
    assert!(
        !target
            .card_detail()
            .await
            .custom_fields
            .iter()
            .any(|field| field.custom_field_id.as_ref() == Some(text))
    );
    let CommandSuccess::CustomFieldUpdated(result) = target
        .field(&[
            "update",
            dropdown,
            "--show-on-card",
            "false",
            "--always-on-card",
            "false",
        ])
        .await
        .unwrap()
    else {
        panic!("expected update")
    };
    assert_eq!(
        result.submitted_fields,
        vec![
            CustomFieldSubmittedField::ShowOnCard,
            CustomFieldSubmittedField::AlwaysOnCard
        ]
    );
    for field in &fields[1..] {
        target.field(&["delete", field, "--yes"]).await.unwrap();
    }
    execute_live_board(&app, &server, &["delete", &board.board_id, "--yes"])
        .await
        .unwrap();
    execute_live_board(&app, &server, &["delete", &second.board_id, "--yes"])
        .await
        .unwrap();
}
