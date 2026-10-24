use super::client;
use secrecy::SecretString;
use serde_json::{Value, json};
use wekan_cli::client::{
    ClientError, WekanClient, cards::SetCardCustomFieldRequest, custom_fields::*,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

fn document() -> Value {
    json!({"_id":"field-1","boardIds":["board-1"],"name":"  Field  ","type":"dropdown",
        "settings":{"currencyCode":"USD","dropdownItems":[{"_id":"item-1","name":"Ready"}],"stringtemplateFormat":"{$a}","stringtemplateSeparator":", "},
        "showOnCard":true,"automaticallyOnCard":false,"alwaysOnCard":false,"showLabelOnMiniCard":true,"showSumAtTopOfList":false,
        "createdAt":"2030-01-01T00:00:00Z","modifiedAt":"2030-01-02T00:00:00Z"})
}

#[test]
fn v12_field_metadata_preserves_optional_values_and_rejects_wrong_types() {
    let mut value = document();
    value["cardKinds"] = json!(["card"]);
    value["readOnly"] = json!(true);
    value["adminOnly"] = json!(false);
    value["sort"] = json!(0.5);
    let decoded: CustomFieldDocument = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(decoded.card_kinds, Some(vec!["card".to_owned()]));
    assert_eq!(decoded.read_only, Some(true));
    assert_eq!(decoded.admin_only, Some(false));
    assert_eq!(
        decoded.sort,
        Some(serde_json::Number::from_f64(0.5).unwrap())
    );
    let omitted: CustomFieldDocument = serde_json::from_value(document()).unwrap();
    assert!(
        omitted.card_kinds.is_none()
            && omitted.read_only.is_none()
            && omitted.admin_only.is_none()
            && omitted.sort.is_none()
    );
    for field in ["cardKinds", "readOnly", "adminOnly", "sort"] {
        let mut invalid = value.clone();
        invalid[field] = json!({"unexpected":true});
        assert!(serde_json::from_value::<CustomFieldDocument>(invalid).is_err());
    }
}
fn create() -> CreateCustomFieldRequest {
    CreateCustomFieldRequest {
        name: "  Field  ".to_owned(),
        field_type: CustomFieldType::Text,
        settings: CustomFieldSettings::default(),
        show_on_card: None,
        automatically_on_card: None,
        show_label_on_mini_card: None,
        show_sum_at_top_of_list: None,
    }
}

#[tokio::test]
async fn multiselect_definitions_use_exact_native_casing_for_reads_and_writes() {
    let server = MockServer::start().await;
    let token = SecretString::from("custom-field-token");
    let mut document = document();
    document["type"] = json!("dropdownMultiSelect");
    Mock::given(method("GET"))
        .and(path(endpoint(0).1))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"_id":"field-1","name":"Multi","type":"dropdownMultiSelect"}
        ])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(endpoint(1).1))
        .respond_with(ResponseTemplate::new(200).set_body_json(document))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(endpoint(2).1))
        .and(body_json(
            json!({"name":"  Field  ","type":"dropdownMultiSelect","settings":{}}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"field-1"})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path(endpoint(3).1))
        .and(body_json(json!({"type":"dropdownMultiSelect"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"field-1"})))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server);
    assert_eq!(
        client.board_custom_fields("board-1", &token).await.unwrap()[0].field_type,
        CustomFieldType::DropdownMultiSelect
    );
    assert_eq!(
        client
            .custom_field("board-1", "field-1", &token)
            .await
            .unwrap()
            .unwrap()
            .field_type,
        CustomFieldType::DropdownMultiSelect
    );
    let mut request = create();
    request.field_type = CustomFieldType::DropdownMultiSelect;
    client
        .create_custom_field("board-1", &request, &token)
        .await
        .unwrap();
    client
        .update_custom_field(
            "board-1",
            "field-1",
            &UpdateCustomFieldRequest {
                field_type: Some(CustomFieldType::DropdownMultiSelect),
                ..Default::default()
            },
            &token,
        )
        .await
        .unwrap();
}
fn endpoint(operation: usize) -> (&'static str, &'static str) {
    match operation {
        0 => ("GET", "/api/boards/board-1/custom-fields"),
        1 => ("GET", "/api/boards/board-1/custom-fields/field-1"),
        2 => ("POST", "/api/boards/board-1/custom-fields"),
        3 => ("PUT", "/api/boards/board-1/custom-fields/field-1"),
        4 => ("DELETE", "/api/boards/board-1/custom-fields/field-1"),
        5 => (
            "POST",
            "/api/boards/board-1/custom-fields/field-1/dropdown-items",
        ),
        6 => (
            "PUT",
            "/api/boards/board-1/custom-fields/field-1/dropdown-items/item-1",
        ),
        7 => (
            "DELETE",
            "/api/boards/board-1/custom-fields/field-1/dropdown-items/item-1",
        ),
        8 => (
            "POST",
            "/api/boards/board-1/lists/list-1/cards/card-1/customFields/field-1",
        ),
        9 => (
            "GET",
            "/api/boards/board-1/cardsByCustomField/field-1/Ready",
        ),
        _ => unreachable!(),
    }
}
async fn invoke(client: &WekanClient, operation: usize) -> Result<(), ClientError> {
    let token = SecretString::from("custom-field-token".to_owned());
    match operation {
        0 => {
            client.board_custom_fields("board-1", &token).await?;
        }
        1 => {
            client.custom_field("board-1", "field-1", &token).await?;
        }
        2 => {
            client
                .create_custom_field("board-1", &create(), &token)
                .await?;
        }
        3 => {
            client
                .update_custom_field(
                    "board-1",
                    "field-1",
                    &UpdateCustomFieldRequest::default(),
                    &token,
                )
                .await?;
        }
        4 => {
            client
                .delete_custom_field("board-1", "field-1", &token)
                .await?;
        }
        5 => {
            client
                .create_custom_field_items(
                    "board-1",
                    "field-1",
                    &CreateCustomFieldItemsRequest::default(),
                    &token,
                )
                .await?;
        }
        6 => {
            client
                .update_custom_field_item(
                    "board-1",
                    "field-1",
                    "item-1",
                    &UpdateCustomFieldItemRequest::default(),
                    &token,
                )
                .await?;
        }
        7 => {
            client
                .delete_custom_field_item("board-1", "field-1", "item-1", &token)
                .await?;
        }
        8 => {
            client
                .set_card_custom_field(
                    "board-1",
                    "list-1",
                    "card-1",
                    "field-1",
                    &SetCardCustomFieldRequest::default(),
                    &token,
                )
                .await?;
        }
        9 => {
            client
                .cards_by_custom_field("board-1", "field-1", "Ready", &token)
                .await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
fn response(operation: usize) -> Value {
    match operation {
        0 => json!([{"_id":"field-1","name":"Field","type":"text"}]),
        1 => document(),
        6 => json!({"_id":"item-1"}),
        8 => json!({"_id":"card-1","customFields":[{"_id":"field-1","value":["a",null]}]}),
        9 => json!([super::responses::complete_card_response()]),
        _ => json!({"_id":"field-1"}),
    }
}

#[tokio::test]
async fn all_ten_routes_preserve_default_bodies_and_authentication() {
    for operation in 0..10 {
        let server = MockServer::start().await;
        let (verb, route) = endpoint(operation);
        let mut mock = Mock::given(method(verb))
            .and(path(route))
            .and(header("Authorization", "Bearer custom-field-token"))
            .and(header("Accept", "application/json"));
        if matches!(operation, 2 | 3 | 5 | 6 | 8) {
            mock = mock
                .and(header("Content-Type", "application/json"))
                .and(body_json(if operation == 2 {
                    json!({"name":"  Field  ","type":"text","settings":{}})
                } else {
                    json!({})
                }));
        }
        mock.respond_with(ResponseTemplate::new(200).set_body_json(response(operation)))
            .expect(1)
            .mount(&server)
            .await;
        invoke(&client(&server), operation).await.unwrap();
    }
}

#[tokio::test]
async fn every_response_object_rejects_unknown_fields_and_bad_acknowledgements() {
    for operation in 0..10 {
        let mut unknown = response(operation);
        if matches!(operation, 0 | 9) {
            unknown[0]["future"] = json!("secret-value");
        } else {
            unknown["future"] = json!("secret-value");
        }
        let mut invalid = vec![unknown, json!(null), json!({})];
        if (2..8).contains(&operation) {
            invalid.extend([json!({"_id":""}), json!({"_id":42})]);
            if operation != 2 {
                invalid.push(json!({"_id":"wrong-id"}));
            }
        }
        if operation == 1 {
            for value in [json!(null), json!([]), json!(["not", "an", "object"])] {
                let mut body = document();
                body["settings"] = value;
                invalid.push(body);
            }
            let mut body = document();
            body["settings"]["dropdownItems"] = json!([["item-1", "Ready"]]);
            invalid.push(body);
            for pointer in ["/settings", "/settings/dropdownItems/0"] {
                let mut body = document();
                body.pointer_mut(pointer).unwrap()["future"] = json!("secret-value");
                invalid.push(body);
            }
            for (field, value) in [
                ("_id", json!("wrong-id")),
                ("boardIds", json!(["wrong-board"])),
                ("modifiedAt", json!("2030-01-01")),
                ("type", json!("future-type")),
                ("type", json!("dropdownmultiselect")),
                ("showOnCard", json!("true")),
            ] {
                let mut body = document();
                body[field] = value;
                invalid.push(body);
            }
            for field in document()
                .as_object()
                .unwrap()
                .keys()
                .filter(|field| field.as_str() != "createdAt")
            {
                let mut body = document();
                body.as_object_mut().unwrap().remove(field);
                invalid.push(body);
            }
            for (field, value) in [
                ("jiraTimeField", json!(false)),
                ("jiraEstimateFieldId", json!(42)),
                ("jiraEstimateUnit", json!([])),
                ("currencyCode", json!(42)),
                ("dropdownItems", json!("items")),
                ("stringtemplateFormat", json!(42)),
                ("stringtemplateSeparator", json!(false)),
            ] {
                let mut body = document();
                body["settings"][field] = value;
                invalid.push(body);
            }
        }
        if operation == 8 {
            invalid.extend([json!({"_id":"wrong-card","customFields":[]}),json!({"_id":"card-1","customFields":[{"_id":"field-1","future":true}]}),json!({"_id":"card-1","customFields":[{"_id":"field-1","value":{"arbitrary":true}}]}),json!({"_id":"card-1","customFields":[{"value":[1,true]}]})]);
        }
        if operation == 9 {
            let mut body = response(operation);
            body[0]["customFields"][0]["future"] = json!(true);
            invalid.push(body);
        }
        for body in invalid {
            let server = MockServer::start().await;
            let (verb, route) = endpoint(operation);
            Mock::given(method(verb))
                .and(path(route))
                .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
                .expect(1)
                .mount(&server)
                .await;
            let error = invoke(&client(&server), operation).await.unwrap_err();
            assert!(
                matches!(
                    error,
                    ClientError::Protocol {
                        success_status_received: true,
                        ..
                    }
                ),
                "operation {operation}: {body}: {error:?}"
            );
            assert!(!format!("{error:?}").contains("secret-value"));
        }
    }
}

#[tokio::test]
async fn nullable_settings_preserve_null_and_omission_in_requests_and_responses() {
    let server = MockServer::start().await;
    let token = SecretString::from("token".to_owned());
    for settings in [
        json!({}),
        json!({"currencyCode":null}),
        json!({"dropdownItems":null}),
        json!({"stringtemplateFormat":null}),
        json!({"stringtemplateSeparator":null}),
        json!({"jiraTimeField":null,"jiraEstimateFieldId":null,"jiraEstimateUnit":null}),
        json!({"jiraTimeField":"original","jiraEstimateFieldId":"customfield_10001","jiraEstimateUnit":"points"}),
        json!({"currencyCode":null,"dropdownItems":null,"stringtemplateFormat":null,"stringtemplateSeparator":null}),
    ] {
        server.reset().await;
        let typed: CustomFieldSettings = serde_json::from_value(settings.clone()).unwrap();
        assert_eq!(serde_json::to_value(&typed).unwrap(), settings);
        let request = UpdateCustomFieldRequest {
            settings: Some(typed.clone()),
            ..Default::default()
        };
        Mock::given(method("PUT"))
            .and(path(endpoint(3).1))
            .and(body_json(json!({"settings":settings})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"field-1"})))
            .expect(1)
            .mount(&server)
            .await;
        client(&server)
            .update_custom_field("board-1", "field-1", &request, &token)
            .await
            .unwrap();
        let mut response = document();
        response["settings"] = settings.clone();
        Mock::given(method("GET"))
            .and(path(endpoint(1).1))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        let document = client(&server)
            .custom_field("board-1", "field-1", &token)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(document.settings, typed);
    }
}

#[tokio::test]
async fn exact_card_and_dropdown_inputs_are_not_coerced_or_retried() {
    for value in [
        json!(null),
        json!(""),
        json!(2.5),
        json!(false),
        json!([1,true,null,{"x":1}]),
        json!({"x":1}),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(endpoint(8).1))
            .and(body_json(json!({"value":value})))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"_id":"card-1","customFields":[]})),
            )
            .expect(1)
            .mount(&server)
            .await;
        client(&server)
            .set_card_custom_field(
                "board-1",
                "list-1",
                "card-1",
                "field-1",
                &SetCardCustomFieldRequest {
                    value: Some(value.clone()),
                },
                &SecretString::from("token".to_owned()),
            )
            .await
            .unwrap();
        server.reset().await;
        Mock::given(method("POST"))
            .and(path(endpoint(5).1))
            .and(body_json(json!({"items":value})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"field-1"})))
            .expect(1)
            .mount(&server)
            .await;
        client(&server)
            .create_custom_field_items(
                "board-1",
                "field-1",
                &CreateCustomFieldItemsRequest { items: Some(value) },
                &SecretString::from("token".to_owned()),
            )
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn definition_fields_serialize_with_exact_wire_names_and_values() {
    let request=UpdateCustomFieldRequest {name:Some("  Name  ".to_owned()),field_type:Some(CustomFieldType::Currency),settings:Some(serde_json::from_value(json!({"currencyCode":"USD","dropdownItems":[{"_id":"i","name":"N"}],"stringtemplateFormat":"x","stringtemplateSeparator":" "})).unwrap()),show_on_card:Some(false),automatically_on_card:Some(true),always_on_card:Some(true),show_label_on_mini_card:Some(false),show_sum_at_top_of_list:Some(true)};
    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        json!({"name":"  Name  ","type":"currency","settings":{"currencyCode":"USD","dropdownItems":[{"_id":"i","name":"N"}],"stringtemplateFormat":"x","stringtemplateSeparator":" "},"showOnCard":false,"automaticallyOnCard":true,"alwaysOnCard":true,"showLabelOnMiniCard":false,"showSumAtTopOfList":true})
    );
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path(endpoint(3).1))
        .and(body_json(serde_json::to_value(&request).unwrap()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"field-1"})))
        .expect(1)
        .mount(&server)
        .await;
    client(&server)
        .update_custom_field(
            "board-1",
            "field-1",
            &request,
            &SecretString::from("token".to_owned()),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn definition_omissions_empty_lookup_and_http_errors_are_explicit() {
    let mut minimal = document();
    minimal.as_object_mut().unwrap().remove("createdAt");
    minimal["settings"] = json!({});
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(endpoint(1).1))
        .respond_with(ResponseTemplate::new(200).set_body_json(minimal))
        .mount(&server)
        .await;
    let field = client(&server)
        .custom_field(
            "board-1",
            "field-1",
            &SecretString::from("token".to_owned()),
        )
        .await
        .unwrap()
        .unwrap();
    assert!(field.created_at.is_none());
    assert!(field.settings.dropdown_items.is_none());
    server.reset().await;
    Mock::given(method("GET"))
        .and(path(endpoint(1).1))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    assert!(
        client(&server)
            .custom_field(
                "board-1",
                "field-1",
                &SecretString::from("token".to_owned())
            )
            .await
            .unwrap()
            .is_none()
    );
    for operation in 0..10 {
        for status in [401, 403, 404, 500] {
            server.reset().await;
            let (verb, route) = endpoint(operation);
            Mock::given(method(verb))
                .and(path(route))
                .respond_with(ResponseTemplate::new(status).set_body_string("<html>Error</html>"))
                .expect(1)
                .mount(&server)
                .await;
            assert!(
                matches!(invoke(&client(&server),operation).await.unwrap_err(),ClientError::Server {status:actual,..} if actual.as_u16()==status)
            );
        }
    }
}

#[tokio::test]
async fn dropdown_responses_preserve_javascript_non_whitespace_ids() {
    let server = MockServer::start().await;
    let token = SecretString::from("token".to_owned());
    for item in ["\u{85}option\u{85}", "\u{85}"] {
        server.reset().await;
        let mut response = document();
        response["settings"]["dropdownItems"] = json!([{"_id": item, "name": "Option"}]);
        Mock::given(method("GET"))
            .and(path(endpoint(1).1))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": item})))
            .expect(1)
            .mount(&server)
            .await;
        let client = client(&server);
        let field = client
            .custom_field("board-1", "field-1", &token)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            field.settings.dropdown_items.unwrap().unwrap()[0].item_id,
            item
        );
        let result = client
            .update_custom_field_item(
                "board-1",
                "field-1",
                item,
                &UpdateCustomFieldItemRequest {
                    name: Some("Changed".to_owned()),
                },
                &token,
            )
            .await
            .unwrap();
        assert_eq!(result.id, item);
        let requests = server.received_requests().await.unwrap();
        assert!(
            requests[1]
                .url
                .path()
                .ends_with(&item.replace('\u{85}', "%C2%85"))
        );
    }
}

#[tokio::test]
async fn unrepresentable_lookup_values_are_rejected_without_http() {
    let server = MockServer::start().await;
    for value in ["", ".", ".."] {
        let error = client(&server)
            .cards_by_custom_field(
                "board-1",
                "field-1",
                value,
                &SecretString::from("token".to_owned()),
            )
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ClientError::Protocol {
                success_status_received: false,
                ..
            }
        ));
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn dot_only_dropdown_ids_are_rejected_without_http() {
    let server = MockServer::start().await;
    let client = client(&server);
    let token = SecretString::from("token".to_owned());
    for item in [".", ".."] {
        for result in [
            client.board_custom_fields(item, &token).await.map(|_| ()),
            client
                .custom_field("board-1", item, &token)
                .await
                .map(|_| ()),
            client
                .set_card_custom_field(
                    "board-1",
                    "list-1",
                    "card-1",
                    item,
                    &SetCardCustomFieldRequest::default(),
                    &token,
                )
                .await
                .map(|_| ()),
            client
                .cards_by_custom_field("board-1", item, "Option", &token)
                .await
                .map(|_| ()),
        ] {
            assert!(
                matches!(
                    result,
                    Err(ClientError::Protocol {
                        success_status_received: false,
                        ..
                    })
                ),
                "{result:?}"
            );
        }
        for result in [
            client
                .update_custom_field_item(
                    "board-1",
                    "field-1",
                    item,
                    &UpdateCustomFieldItemRequest::default(),
                    &token,
                )
                .await,
            client
                .delete_custom_field_item("board-1", "field-1", item, &token)
                .await,
        ] {
            assert!(
                matches!(
                    result,
                    Err(ClientError::Protocol {
                        success_status_received: false,
                        ..
                    })
                ),
                "{result:?}"
            );
        }
    }
    for item in [".", "..", " . ", "\u{feff}..\u{feff}"] {
        let settings: CustomFieldSettings =
            serde_json::from_value(json!({"dropdownItems":[{"_id":item,"name":"Option"}]}))
                .unwrap();
        let mut request = create();
        request.settings = settings.clone();
        for result in [
            client
                .create_custom_field("board-1", &request, &token)
                .await,
            client
                .update_custom_field(
                    "board-1",
                    "field-1",
                    &UpdateCustomFieldRequest {
                        name: Some("Must not save".to_owned()),
                        settings: Some(settings),
                        ..Default::default()
                    },
                    &token,
                )
                .await,
        ] {
            assert!(
                matches!(
                    result,
                    Err(ClientError::Protocol {
                        success_status_received: false,
                        ..
                    })
                ),
                "{result:?}"
            );
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn identifiers_and_lookup_values_are_encoded_as_individual_segments() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;
    client(&server)
        .cards_by_custom_field(
            "board/one",
            "field?one",
            " Ready/#?% ",
            &SecretString::from("token".to_owned()),
        )
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    assert!(
        requests[0]
            .url
            .path()
            .contains("board%2Fone/cardsByCustomField/field%3Fone/%20Ready%2F%23%3F%25%20")
    );
    assert!(requests[0].url.query().is_none());
}
