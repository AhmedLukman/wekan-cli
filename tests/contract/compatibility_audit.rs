use serde_json::{Value, json};
use wekan_cli::client::{CardDocument, ListDocument};

#[test]
fn v13_28_projection_and_nested_metadata_reject_unmapped_or_mistyped_fields() {
    use wekan_cli::client::{
        cards::CardListCustomField,
        metadata::{CardFlow, ShopPaymentMethod},
    };
    for value in [Value::Null, json!("value"), json!([null, "option"])] {
        let field: CardListCustomField =
            serde_json::from_value(json!({"_id":"field","value":value})).unwrap();
        assert_eq!(serde_json::to_value(field).unwrap()["value"], value);
    }
    for bad in [
        json!({"_id":"field"}),
        json!({"_id":"field","value":{}}),
        json!({"_id":"field","value":null,"extra":true}),
    ] {
        assert!(serde_json::from_value::<CardListCustomField>(bad).is_err());
    }
    for bad in [
        json!({"extra":true}),
        json!({"wsjf":{"extra":1}}),
        json!({"wsjf":{"businessValue":"8"}}),
    ] {
        assert!(serde_json::from_value::<CardFlow>(bad).is_err());
    }
    let valid = json!({"id":"cash","enabled":true,"method":{"type":"cash","details":"On pickup"}});
    assert!(serde_json::from_value::<ShopPaymentMethod>(valid.clone()).is_ok());
    for method in [
        json!({"type":"future"}),
        json!({"type":"cash","details":"test","extra":true}),
        json!({"type":"cash"}),
        json!({"type":"cash","details":false}),
    ] {
        let mut payment = valid.clone();
        payment["method"] = method;
        assert!(serde_json::from_value::<ShopPaymentMethod>(payment).is_err());
    }
}

#[test]
fn watchers_preserve_absence_and_empty_arrays_without_weakening_strict_decoding() {
    let list = json!({
        "_id":"list-1", "title":"Todo", "archived":false, "boardId":"board-1",
        "createdAt":"2026-10-17T00:00:00Z", "modifiedAt":"2026-10-17T00:00:00Z", "type":"list"
    });
    let card = super::responses::complete_card_response();
    for value in [None, Some(json!([])), Some(json!(["user-1"]))] {
        let mut card = card.clone();
        let mut list = list.clone();
        if let Some(value) = &value {
            card["watchers"] = value.clone();
            list["watchers"] = value.clone();
        }
        let expected: Option<Vec<String>> = value.map(|v| serde_json::from_value(v).unwrap());
        assert_eq!(
            serde_json::from_value::<CardDocument>(card)
                .unwrap()
                .watchers,
            expected
        );
        assert_eq!(
            serde_json::from_value::<ListDocument>(list)
                .unwrap()
                .watchers,
            expected
        );
    }
    for invalid in [
        Value::Null,
        json!(true),
        json!([null]),
        json!([4]),
        json!([{"future":true}]),
    ] {
        let mut card = card.clone();
        let mut list = list.clone();
        card["watchers"] = invalid.clone();
        list["watchers"] = invalid;
        assert!(serde_json::from_value::<CardDocument>(card).is_err());
        assert!(serde_json::from_value::<ListDocument>(list).is_err());
    }
    let mut card = card;
    card["watchers"] = json!([]);
    card["future"] = json!(true);
    assert!(serde_json::from_value::<CardDocument>(card).is_err());
    let mut list = list;
    list["watchers"] = json!([]);
    list["wipLimit"] = json!({"value":1,"enabled":true,"soft":false,"future":true});
    assert!(serde_json::from_value::<ListDocument>(list).is_err());
}

#[test]
fn all_optional_global_settings_accept_null_and_updates_preserve_explicit_null() {
    use wekan_cli::client::admin::settings::{GlobalSettingsDocument, UpdateGlobalSettings};
    let mut nulls = json!({"defaultAuthenticationMethod":"password"});
    for (wire, _, _, _) in super::admin_fixtures::GLOBAL_FIELDS {
        if *wire != "defaultAuthenticationMethod" {
            nulls[*wire] = Value::Null;
        }
    }
    assert_eq!(nulls.as_object().unwrap().len(), 36);
    serde_json::from_value::<GlobalSettingsDocument>(nulls.clone()).unwrap();
    let update: UpdateGlobalSettings = serde_json::from_value(nulls.clone()).unwrap();
    assert_eq!(serde_json::to_value(update).unwrap(), nulls);
    let update: UpdateGlobalSettings = serde_json::from_value(json!({"productName":null})).unwrap();
    assert_eq!(
        serde_json::to_value(update).unwrap(),
        json!({"productName":null})
    );
    assert!(
        serde_json::from_value::<GlobalSettingsDocument>(
            json!({"defaultAuthenticationMethod":null})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<UpdateGlobalSettings>(json!({"defaultAuthenticationMethod":null}))
            .is_err()
    );
    assert!(serde_json::from_value::<GlobalSettingsDocument>(json!({"productName":12})).is_err());
    assert!(
        serde_json::from_value::<GlobalSettingsDocument>(json!({"productName":null,"future":true}))
            .is_err()
    );
}

#[test]
fn built_in_trigger_description_round_trips_and_custom_properties_still_reject() {
    use wekan_cli::client::rules::{CreateRuleRequest, RuleDocument};
    for value in [json!("Renamed trigger"), Value::Null] {
        let input = json!({"trigger":{"description":value},"action":{}});
        let request: CreateRuleRequest = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(request).unwrap(), input);
        let document: RuleDocument = serde_json::from_value(
            json!({"_id":"rule-1","title":"Rule","enabled":true,"trigger":{"description":value},"action":{}}),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(document.trigger.unwrap()).unwrap()["description"],
            value
        );
    }
    for payload in [
        json!({"description":[]}),
        json!({"description":"Known","future":true}),
    ] {
        assert!(
            serde_json::from_value::<RuleDocument>(
                json!({"_id":"rule-1","title":"Rule","trigger":payload,"action":{}})
            )
            .is_err()
        );
    }
}

#[test]
#[ignore = "requires WEKAN_CORRECTED_SPEC pointing to a freshly generated corrected contract"]
fn generated_contract_preserves_verified_audit_corrections() {
    let file = std::fs::File::open(std::env::var_os("WEKAN_CORRECTED_SPEC").unwrap()).unwrap();
    let spec: Value = serde_yaml_ng::from_reader(file).unwrap();
    let operations: Vec<_> = spec["paths"]
        .as_object()
        .unwrap()
        .values()
        .flat_map(|item| item.as_object().unwrap().values())
        .filter(|operation| operation.get("operationId").is_some())
        .collect();
    let operation = |id: &str| *operations.iter().find(|o| o["operationId"] == id).unwrap();
    let body = |id: &str| {
        &operation(id)["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|parameter| parameter["in"] == "body")
            .unwrap()["schema"]
    };
    for id in [
        "login",
        "update_attachment_settings",
        "update_admin_org_features",
        "update_admin_team_features",
        "post_board_card_checklists",
        "post_board_card_checklist_items",
        "post_board_card_comments",
        "new_card_dependency",
        "edit_card_dependency",
        "delete_board_cards_bulk",
        "post_board_cards_labels",
        "put_board_customField",
        "post_board_customField_dropdown-items",
        "put_board_customField_dropdownItem",
        "post_board_integrations",
        "delete_board_int_activities",
        "post_board_int_activities",
        "post_board_lists",
        "put_board_list",
        "post_board_list_cards",
        "put_board_list_card",
        "post_board_list_card_copy",
        "post_board_list_card_customField",
        "post_board_list_copy",
        "post_board_list_move",
        "post_board_user_add",
        "post_board_user_remove",
        "edit_board_rule",
        "post_board_swimlanes",
        "put_board_swimlane",
        "post_board_swimlane_copy",
        "post_board_swimlane_move",
        "post_deletetoken",
        "update_global_settings",
        "post_users",
        "put_user",
        "update_board_card_settings",
        "import_ics",
        "post_boards",
        "put_board_title",
        "put_board_labels",
        "post_board_copy",
        "post_board_member",
    ] {
        assert!(
            operation(id)["consumes"]
                .as_array()
                .unwrap()
                .contains(&json!("application/x-www-form-urlencoded")),
            "{id}"
        );
    }
    for id in [
        "upload_attachment",
        "upload_board_background",
        "copy_attachment",
        "move_attachment",
    ] {
        for media in ["application/json", "text/plain", "application/octet-stream"] {
            assert!(
                operation(id)["consumes"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(media)),
                "{id}"
            );
        }
    }
    // The Rust byte vectors use a custom base64 serializer: schemas must
    // describe their JSON wire form rather than their in-memory array type.
    for schema in [
        &body("upload_attachment")["properties"]["fileData"],
        &body("upload_board_background")["properties"]["fileData"],
        &spec["definitions"]["VerifiedAttachmentsUploadAttachmentRequest"]["properties"]["fileData"],
        &spec["definitions"]["VerifiedAttachmentsUploadBackgroundRequest"]["properties"]["fileData"],
    ] {
        assert_eq!(schema["type"], "string");
        assert_eq!(schema["format"], "byte");
        assert!(schema.get("items").is_none());
    }
    let native_enabled = &body("edit_board_rule")["properties"]["enabled"];
    assert_eq!(native_enabled["x-nullable"], true);
    assert!(
        native_enabled["description"]
            .as_str()
            .unwrap()
            .contains("null as false")
    );
    let typed_enabled =
        &spec["definitions"]["VerifiedRulesUpdateRuleRequest"]["properties"]["enabled"];
    assert_eq!(typed_enabled["type"], "boolean");
    assert_ne!(typed_enabled["x-nullable"], true);
    let types = |schema: &Value, name: &str| {
        schema["type"] == name
            || schema["x-oneOf"]
                .as_array()
                .is_some_and(|variants| variants.iter().any(|variant| variant["type"] == name))
    };
    // Native raw inputs retain their verified coercions. Typed command inputs
    // deliberately accept their narrower, explicit Rust types.
    for (id, field, kind) in [
        ("post_board_card_checklist_items", "sort", "object"),
        ("put_board_list_card", "archive", "array"),
        ("put_board_card_checklist_item", "isFinished", "number"),
        ("put_board_customField", "name", "boolean"),
        ("post_board_list_card_copy", "position", "object"),
        ("update_global_settings", "productName", "number"),
    ] {
        assert!(types(&body(id)["properties"][field], kind), "{id}.{field}");
    }
    assert!(types(
        &body("put_board_list_card")["properties"]["locations"]["items"]["properties"]["latitude"],
        "array"
    ));
    assert!(types(
        &operation("put_board_title")["responses"]["200"]["schema"]["properties"]["title"],
        "number"
    ));
    assert!(
        body("put_board_list_card")["properties"]["spentTime"]["description"]
            .as_str()
            .unwrap()
            .contains("zero")
    );
    let definitions = &spec["definitions"];
    for id in ["exportCSV/TSV", "exportExcel", "exportExcelCard"] {
        assert!(
            operation(id)["produces"]
                .as_array()
                .unwrap()
                .contains(&json!("text/plain")),
            "{id}"
        );
        assert_eq!(
            operation(id)["responses"]["403"]["schema"]["type"],
            "string"
        );
    }
    for id in [
        "get_board_int",
        "put_board_int",
        "delete_board_int",
        "post_board_int_activities",
        "delete_board_int_activities",
    ] {
        assert_eq!(
            operation(id)["responses"]["404"]["schema"]["$ref"],
            "#/definitions/VerifiedIntegrationNotFound",
            "{id}"
        );
    }
    let missing = &definitions["VerifiedIntegrationNotFound"];
    assert_eq!(missing["required"], json!(["error"]));
    assert_eq!(
        missing["properties"]["error"]["enum"],
        json!(["Board not found", "Integration not found"])
    );
    let external = operation("exportExternal");
    let produces = external["produces"].as_array().unwrap();
    for schema in external["x-format-response-schemas"]
        .as_object()
        .unwrap()
        .values()
    {
        if let Some(content_type) = schema.get("x-content-type") {
            assert!(
                produces.contains(content_type),
                "undeclared export media type {content_type}"
            );
        }
    }
    for status in ["403", "507"] {
        assert_eq!(
            operation("import_trello")["responses"][status]["schema"]["$ref"],
            "#/definitions/TrelloHttpImportError"
        );
    }
    let settings = &definitions["VerifiedAdminSettingsGlobalSettingsDocument"];
    for (wire, _, _, _) in super::admin_fixtures::GLOBAL_FIELDS {
        assert_eq!(
            settings["properties"][*wire]["x-nullable"] == true,
            *wire != "defaultAuthenticationMethod",
            "{wire}"
        );
    }
    assert_eq!(settings["additionalProperties"], false);
    assert_eq!(body("update_global_settings")["additionalProperties"], true);
    assert_eq!(
        definitions["VerifiedIntegrationsIntegrationDocument"]["properties"]["title"]["x-nullable"],
        true
    );
    let variants = &definitions["VerifiedIntegrationsReadDocument"]["x-oneOf"];
    assert_eq!(variants.as_array().unwrap().len(), 2);
    assert_eq!(
        variants[1]["$ref"].as_str(),
        Some("#/definitions/VerifiedIntegrationsMemberDocument")
    );
    let member = &definitions["VerifiedIntegrationsMemberDocument"];
    assert_eq!(member["additionalProperties"], false);
    assert_eq!(member["required"].as_array().unwrap().len(), 4);
    assert!(member["properties"].get("url").is_none());
    for field in [
        "description",
        "advancedFilter",
        "textContains",
        "direction",
        "cron",
    ] {
        assert!(
            definitions["VerifiedRulesPayloadsRuleTrigger"]["properties"]
                .get(field)
                .is_some(),
            "{field}"
        );
    }
    assert!(body("put_user").get("required").is_none());
    assert!(
        operation("put_user")["responses"]["200"]
            .get("x-wekan-empty-body")
            .is_none()
    );
    for id in ["update_admin_org_features", "update_admin_team_features"] {
        let parameter = operation(id)["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["in"] == "body")
            .unwrap();
        assert_eq!(parameter["required"], false);
    }
    // v12.57 no longer discloses native Match.Error internals.
    for id in ["post_deletetoken", "update_attachment_settings"] {
        assert_eq!(operation(id)["x-wekan-type-check-error"]["status"], 500);
        assert_eq!(
            operation(id)["x-wekan-type-check-error"]["schema"]["properties"]["error"]["enum"],
            json!(["Internal server error"])
        );
    }
    assert!(definitions.get("NativeMatchError").is_none());
    let known_defects: std::collections::BTreeSet<_> = operations
        .iter()
        .filter_map(|operation| operation["x-wekan-defects"].as_array())
        .flatten()
        .map(|defect| defect["id"].as_str().unwrap())
        .collect();
    let ledger = include_str!("../../docs/compatibility.md");
    let open: std::collections::BTreeSet<_> = ledger
        .lines()
        .filter_map(|line| {
            let mut cols = line.split('|').skip(1).map(str::trim);
            let id = cols.next()?;
            let disposition = cols.next()?;
            (id.starts_with('W')
                && (disposition.starts_with("Open")
                    || disposition.starts_with("Partly")
                    || disposition.starts_with("New")
                    || disposition.starts_with("Survives")))
            .then_some(id)
        })
        .collect();
    assert_eq!(known_defects, open);
    for id in ["get_user", "put_user", "post_deletetoken"] {
        assert!(operation(id)["responses"].get("404").is_some());
    }
    for name in [
        "VerifiedCustom_fieldsCustomFieldType",
        "VerifiedExportsDocumentsExportCustomFieldType",
    ] {
        let values = definitions[name]["enum"].as_array().unwrap();
        assert!(values.contains(&json!("dropdownMultiSelect")));
        assert!(!values.contains(&json!("dropdownmultiselect")));
        assert_eq!(values.len(), 8);
    }
    for name in [
        "VerifiedCustom_fieldsCustomFieldSettings",
        "VerifiedExportsDocumentsCustomFieldsSettings",
    ] {
        let schema = &definitions[name];
        assert_eq!(schema["additionalProperties"], false);
        for field in ["jiraTimeField", "jiraEstimateFieldId", "jiraEstimateUnit"] {
            assert_eq!(schema["properties"][field]["type"], "string");
            assert_eq!(schema["properties"][field]["x-nullable"], true);
        }
    }
    let report = &definitions["VerifiedCardsBot_runsBotRunUpdate"]["properties"];
    for field in ["status", "options"] {
        assert!(report[field].get("x-nullable").is_none());
    }
    for field in ["text", "tool", "question", "tokens", "costUsd"] {
        assert_eq!(report[field]["x-nullable"], true);
    }
    let report_body = body("report_card_bot_run");
    for field in ["status", "options"] {
        assert!(report_body["properties"][field].get("x-nullable").is_none());
    }
    for id in ["post_board_custom-fields", "put_board_customField"] {
        let request = body(id);
        assert!(
            request["properties"]["type"]["enum"]
                .as_array()
                .unwrap()
                .contains(&json!("dropdownMultiSelect"))
        );
        for field in ["jiraTimeField", "jiraEstimateFieldId", "jiraEstimateUnit"] {
            assert_eq!(
                request["properties"]["settings"]["properties"][field]["type"],
                "string"
            );
            assert_eq!(
                request["properties"]["settings"]["properties"][field]["x-nullable"],
                true
            );
        }
    }
    assert!(
        operation("upload_attachment")
            .get("x-wekan-defects")
            .is_none()
    );
    assert!(
        operation("upload_board_background")["x-wekan-defects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|defect| defect["id"] == "W12-006"
                && defect["liveRegression"] == "tests/e2e/attachments.rs")
    );

    // Exact v12.57 route and model fields that previously failed strict decoding.
    let settings = &definitions["VerifiedBoardsCard_settingsBoardCardSettings"];
    assert_eq!(settings["properties"].as_object().unwrap().len(), 124);
    for field in [
        "allowsCardButtonsOnMinicard",
        "allowsDescriptionBadgeOnCard",
        "allowsUnreadCommentsOnCard",
        "allowsDueFromTitleOnCard",
        "allowsDueFromTitleOnMinicard",
        "allowsSimilarCardsOnCard",
        "allowsSimilarCardsOnMinicard",
        "allowsAtRiskOnCard",
        "allowsAtRiskOnMinicard",
        "allowsSuggestedMembersOnCard",
        "allowsForecastOnCard",
        "allowsForecastOnMinicard",
        "allowsChecklistFromDescriptionOnCard",
        "allowsActionItemsOnCard",
        "allowsWhiteboardOnCard",
        "labelsAboveTitleOnCard",
        "allowsChecklistDueDateOnMinicard",
        "allowsLocationOnMinicard",
        "allowsSwimlaneNameOnCard",
        "allowsFlowtimeOnMinicard",
        "allowsPomodoroOnMinicard",
        "allowsAttachmentListOnMinicard",
        "allowsTextNotesOnMinicard",
        "allowsCommentCountOnCard",
        "allowsActivitiesOnMinicard",
    ] {
        assert_eq!(settings["properties"][field]["type"], "boolean", "{field}");
        assert_ne!(settings["properties"][field]["x-nullable"], true, "{field}");
        assert_eq!(
            body("update_board_card_settings")["properties"][field]["type"],
            "boolean",
            "{field}"
        );
    }
    for (definition, field, kind) in [
        ("VerifiedBoardsBoardLabel", "dueAt", "string"),
        ("VerifiedBoardsBoardLabel", "sharedLabelId", "string"),
        ("VerifiedExportsDocumentsBoardLabel", "dueAt", "string"),
        (
            "VerifiedExportsDocumentsBoardLabel",
            "sharedLabelId",
            "string",
        ),
        ("VerifiedBoardsBoardMember", "grantedBy", "array"),
        ("VerifiedBoardsBoardMember", "customRoleId", "string"),
        ("VerifiedExportsDocumentsBoardMember", "grantedBy", "array"),
        (
            "VerifiedExportsDocumentsBoardMember",
            "customRoleId",
            "string",
        ),
        ("VerifiedUsersCurrentUserBoard", "grantedBy", "array"),
        ("VerifiedUsersCurrentUserBoard", "customRoleId", "string"),
        (
            "VerifiedChecklist_itemsChecklistItemDocument",
            "assigneeId",
            "string",
        ),
        (
            "VerifiedChecklist_itemsChecklistItemDocument",
            "linkedCardId",
            "string",
        ),
        (
            "VerifiedExportsDocumentsChecklistItems",
            "assigneeId",
            "string",
        ),
        (
            "VerifiedExportsDocumentsChecklistItems",
            "linkedCardId",
            "string",
        ),
    ] {
        let schema = &definitions[definition];
        assert_eq!(schema["additionalProperties"], false, "{definition}");
        assert_eq!(
            schema["properties"][field]["type"], kind,
            "{definition}.{field}"
        );
        assert_ne!(
            schema["properties"][field]["x-nullable"], true,
            "{definition}.{field}"
        );
        assert!(
            schema["required"]
                .as_array()
                .is_none_or(|required| { !required.contains(&json!(field)) }),
            "{definition}.{field} must remain optional"
        );
        if field == "grantedBy" {
            assert_eq!(schema["properties"][field]["items"]["type"], "string");
        }
        if field == "dueAt" {
            assert_eq!(schema["properties"][field]["format"], "date-time");
        }
    }
    for field in ["splitBy", "membersMode", "csvMapping", "attachmentAuth"] {
        assert!(
            body("import_board")["properties"].get(field).is_none(),
            "{field}"
        );
        assert!(
            body("import_board_from")["properties"].get(field).is_some(),
            "{field}"
        );
    }
    assert_eq!(
        operation("export_all_boards")["security"],
        json!([{"UserSecurity":[]}, {"ExportAuthTokenSecurity":[]}])
    );
    assert_eq!(
        spec["securityDefinitions"]["ExportAuthTokenSecurity"]["type"],
        "apiKey"
    );
    assert_eq!(
        spec["securityDefinitions"]["ExportAuthTokenSecurity"]["in"],
        "query"
    );
    assert_eq!(
        spec["securityDefinitions"]["ExportAuthTokenSecurity"]["name"],
        "authToken"
    );
}
