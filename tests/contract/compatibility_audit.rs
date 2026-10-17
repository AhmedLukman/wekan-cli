use serde_json::{Value, json};
use wekan_cli::client::{CardDocument, ListDocument};

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
    assert_eq!(nulls.as_object().unwrap().len(), 35);
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
            json!({"_id":"rule-1","title":"Rule","trigger":{"description":value},"action":{}}),
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
        "edit_board_card_settings",
        "import_ics",
        "create_board",
        "update_board_title",
        "add_board_label",
        "copy_board",
        "update_board_member_permissions",
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
        for media in ["text/plain", "application/octet-stream"] {
            assert!(
                operation(id)["consumes"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(media)),
                "{id}"
            );
        }
    }
    let definitions = &spec["definitions"];
    let types = |schema: &Value, name: &str| {
        schema["type"] == name
            || schema["x-oneOf"]
                .as_array()
                .is_some_and(|a| a.iter().any(|v| v["type"] == name))
    };
    for (definition, field, kind) in [
        ("ChecklistItemCreateRequest", "sort", "object"),
        ("BoardTitleResponse", "title", "number"),
        ("CardUpdateRequest", "archive", "array"),
        ("ChecklistItemUpdateRequest", "isFinished", "number"),
        ("CustomFieldUpdateRequest", "name", "boolean"),
        ("CardLocationRequest", "latitude", "array"),
        ("CardCopyRequest", "position", "object"),
        ("RestSettingsUpdate", "productName", "number"),
    ] {
        assert!(
            types(&definitions[definition]["properties"][field], kind),
            "{definition}.{field}"
        );
    }
    for (wire, _, _, _) in super::admin_fixtures::GLOBAL_FIELDS {
        assert_eq!(
            definitions["RestSettings"]["properties"][*wire]["x-nullable"] == true,
            *wire != "defaultAuthenticationMethod",
            "{wire}"
        );
    }
    assert_eq!(definitions["RestSettings"]["additionalProperties"], false);
    assert_eq!(
        definitions["RestSettingsUpdate"]["additionalProperties"],
        true
    );
    assert_eq!(
        definitions["Integration"]["properties"]["title"]["x-nullable"],
        true
    );
    assert!(
        definitions["RuleResponse"]["properties"]["trigger"]["properties"]
            .get("description")
            .is_some()
    );
    assert!(
        definitions["UpdateUserActionRequest"]
            .get("required")
            .is_none()
    );
    assert_eq!(
        operation("put_user")["responses"]["200"]["x-wekan-empty-body"],
        true
    );
    for id in ["update_admin_org_features", "update_admin_team_features"] {
        let body = operation(id)["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["in"] == "body")
            .unwrap();
        assert_eq!(body["required"], false);
    }
    assert_eq!(
        definitions["NativeMatchError"]["additionalProperties"],
        false
    );
    assert_eq!(
        definitions["SanitizedMatchError"]["additionalProperties"],
        false
    );
    assert!(
        definitions["NativeMatchError"]["properties"]
            .get("sanitizedError")
            .is_some()
    );
}
