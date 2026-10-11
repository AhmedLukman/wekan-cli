use secrecy::SecretString;
use serde_json::{Value, json};
use wekan_cli::client::{
    ChecklistDocument, ChecklistItemDocument, ChecklistItemFinishedInput, ChecklistItemSortInput,
    ChecklistItemsInput, ClientError, CreateChecklistItemRequest, CreateChecklistRequest,
    ResponseDecodeKind, UpdateChecklistItemRequest, WekanClient,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

use super::{client, client_from_url};

fn checklist() -> Value {
    json!({"_id":"checklist-1", "cardId":"card-1", "boardId":"board-1", "title":"Release",
        "createdAt":"2030-01-02T03:04:05Z", "modifiedAt":"2030-01-02T03:04:05Z", "sort":0,
        "items":[{"_id":"item-1", "title":"Verify", "isFinished":false}]})
}

fn item() -> Value {
    json!({"_id":"item-1", "title":"Verify", "sort":-1.5, "isFinished":false,
        "checklistId":"checklist-1", "cardId":"card-1", "boardId":"board-1",
        "createdAt":"2030-01-02T03:04:05Z", "modifiedAt":"2030-01-02T03:04:05Z"})
}

async fn invoke(client: &WekanClient, operation: usize) -> Result<(), ClientError> {
    let token = SecretString::from("checklist-token".to_owned());
    match operation {
        0 => {
            client.card_checklists("board-1", "card-1", &token).await?;
        }
        1 => {
            client
                .create_checklist(
                    "board-1",
                    "card-1",
                    &CreateChecklistRequest::default(),
                    &token,
                )
                .await?;
        }
        2 => {
            client
                .checklist("board-1", "card-1", "checklist-1", &token)
                .await?;
        }
        3 => {
            client
                .delete_checklist("board-1", "card-1", "checklist-1", &token)
                .await?;
        }
        4 => {
            client
                .create_checklist_item(
                    "board-1",
                    "card-1",
                    "checklist-1",
                    &CreateChecklistItemRequest {
                        title: "Verify".to_owned(),
                        sort: None,
                    },
                    &token,
                )
                .await?;
        }
        5 => {
            client
                .checklist_item("board-1", "card-1", "checklist-1", "item-1", &token)
                .await?;
        }
        6 => {
            client
                .update_checklist_item(
                    "board-1",
                    "card-1",
                    "checklist-1",
                    "item-1",
                    &UpdateChecklistItemRequest::default(),
                    &token,
                )
                .await?;
        }
        7 => {
            client
                .delete_checklist_item("board-1", "card-1", "checklist-1", "item-1", &token)
                .await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn endpoint(operation: usize) -> (&'static str, &'static str) {
    match operation {
        0 => ("GET", "/api/boards/board-1/cards/card-1/checklists"),
        1 => ("POST", "/api/boards/board-1/cards/card-1/checklists"),
        2 => (
            "GET",
            "/api/boards/board-1/cards/card-1/checklists/checklist-1",
        ),
        3 => (
            "DELETE",
            "/api/boards/board-1/cards/card-1/checklists/checklist-1",
        ),
        4 => (
            "POST",
            "/api/boards/board-1/cards/card-1/checklists/checklist-1/items",
        ),
        5 => (
            "GET",
            "/api/boards/board-1/cards/card-1/checklists/checklist-1/items/item-1",
        ),
        6 => (
            "PUT",
            "/api/boards/board-1/cards/card-1/checklists/checklist-1/items/item-1",
        ),
        7 => (
            "DELETE",
            "/api/boards/board-1/cards/card-1/checklists/checklist-1/items/item-1",
        ),
        _ => unreachable!(),
    }
}

fn success(operation: usize) -> Value {
    match operation {
        0 => json!([{"_id":"checklist-1", "title":"Release"}]),
        2 => checklist(),
        5 => item(),
        1 | 3 => json!({"_id":"checklist-1"}),
        _ => json!({"_id":"item-1"}),
    }
}

#[tokio::test]
async fn all_checklist_operations_use_exact_methods_authentication_and_bodies() {
    for operation in 0..8 {
        let server = MockServer::start().await;
        let (verb, url) = endpoint(operation);
        let mut mock = Mock::given(method(verb))
            .and(path(url))
            .and(header("authorization", "Bearer checklist-token"))
            .and(header("accept", "application/json"));
        if operation == 1 || operation == 6 {
            mock = mock.and(body_json(json!({})));
        }
        if operation == 4 {
            mock = mock.and(body_json(json!({"title":"Verify"})));
        }
        mock.respond_with(ResponseTemplate::new(200).set_body_json(success(operation)))
            .expect(1)
            .mount(&server)
            .await;
        invoke(&client(&server), operation).await.unwrap();
        server.verify().await;
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn checklist_endpoints_preserve_server_subpaths_and_encode_each_identifier() {
    let server = MockServer::start().await;
    let client = client_from_url(&format!("{}/tenant", server.uri()));
    let token = SecretString::from("encoded-token".to_owned());
    Mock::given(method("GET"))
        .and(path(
            "/tenant/api/boards/board%2F1/cards/card%201/checklists",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(
            "/tenant/api/boards/board%2F1/cards/card%201/checklists/list%2F1/items/item%201",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"item 1"})))
        .expect(1)
        .mount(&server)
        .await;
    client
        .card_checklists("board/1", "card 1", &token)
        .await
        .unwrap();
    client
        .delete_checklist_item("board/1", "card 1", "list/1", "item 1", &token)
        .await
        .unwrap();
}

#[tokio::test]
async fn checklist_requests_preserve_every_corrected_json_union_form() {
    let server = MockServer::start().await;
    let client = client(&server);
    let token = SecretString::from("checklist-token".to_owned());
    for items in [
        json!(null),
        json!(""),
        json!("  One  "),
        json!([]),
        json!([" One ", "Two"]),
    ] {
        Mock::given(method("POST"))
            .and(path(endpoint(1).1))
            .and(body_json(json!({"title":"  Release  ","items":items})))
            .respond_with(ResponseTemplate::new(200).set_body_json(success(1)))
            .expect(1)
            .mount(&server)
            .await;
        client
            .create_checklist(
                "board-1",
                "card-1",
                &CreateChecklistRequest {
                    title: Some("  Release  ".to_owned()),
                    items: Some(serde_json::from_value(items).unwrap()),
                },
                &token,
            )
            .await
            .unwrap();
    }
    for sort in [
        json!(-1.5),
        json!("2.5"),
        json!(""),
        json!(null),
        json!("NaN"),
        json!("Infinity"),
        json!("0x10"),
    ] {
        Mock::given(method("POST"))
            .and(path(endpoint(4).1))
            .and(body_json(json!({"title":"  Verify  ","sort":sort})))
            .respond_with(ResponseTemplate::new(200).set_body_json(success(4)))
            .expect(1)
            .mount(&server)
            .await;
        client
            .create_checklist_item(
                "board-1",
                "card-1",
                "checklist-1",
                &CreateChecklistItemRequest {
                    title: "  Verify  ".to_owned(),
                    sort: Some(serde_json::from_value(sort).unwrap()),
                },
                &token,
            )
            .await
            .unwrap();
    }
    for finished in [
        json!(true),
        json!(false),
        json!("TRUE"),
        json!("false"),
        json!("anything"),
        json!(""),
    ] {
        Mock::given(method("PUT"))
            .and(path(endpoint(6).1))
            .and(body_json(
                json!({"title":"  Renamed  ","isFinished":finished}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(success(6)))
            .expect(1)
            .mount(&server)
            .await;
        client
            .update_checklist_item(
                "board-1",
                "card-1",
                "checklist-1",
                "item-1",
                &UpdateChecklistItemRequest {
                    title: Some("  Renamed  ".to_owned()),
                    is_finished: Some(serde_json::from_value(finished).unwrap()),
                },
                &token,
            )
            .await
            .unwrap();
    }
}

#[test]
fn checklist_input_unions_reject_shapes_outside_the_corrected_contract() {
    for value in [json!(true), json!(1), json!([null]), json!({})] {
        assert!(serde_json::from_value::<ChecklistItemsInput>(value).is_err());
    }
    for value in [json!(true), json!([]), json!({})] {
        assert!(serde_json::from_value::<ChecklistItemSortInput>(value).is_err());
    }
    for value in [json!(null), json!(1), json!([]), json!({})] {
        assert!(serde_json::from_value::<ChecklistItemFinishedInput>(value).is_err());
    }
}

#[tokio::test]
async fn all_checklist_responses_reject_unknown_fields_at_the_decoding_boundary() {
    for operation in 0..8 {
        let server = MockServer::start().await;
        let mut response = success(operation);
        if operation == 0 {
            response[0]["future"] = json!("secret");
        } else {
            response["future"] = json!("secret");
        }
        if operation == 2 || operation == 5 {
            response["userId"] = json!("creator-1");
        }
        let (verb, url) = endpoint(operation);
        Mock::given(method(verb))
            .and(path(url))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;
        let error = invoke(&client(&server), operation).await.unwrap_err();
        let ClientError::Protocol {
            diagnostic: Some(diagnostic),
            success_status_received: true,
            message,
        } = error
        else {
            panic!("expected strict protocol error: {error:?}");
        };
        assert_eq!(diagnostic.kind, ResponseDecodeKind::UnknownField);
        assert!(!message.contains("secret"));
    }
    let server = MockServer::start().await;
    let mut response = checklist();
    response["items"][0]["future"] = json!(true);
    Mock::given(method("GET"))
        .and(path(endpoint(2).1))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;
    let ClientError::Protocol {
        diagnostic: Some(diagnostic),
        ..
    } = invoke(&client(&server), 2).await.unwrap_err()
    else {
        panic!("expected nested diagnostic");
    };
    assert_eq!(diagnostic.path, "/items/0/future");
}

#[tokio::test]
async fn checklist_reads_and_mutations_reject_missing_invalid_and_mismatched_values() {
    for (operation, response) in [
        (0, json!([{"title":"Release"}])),
        (0, json!([{"_id":"", "title":"Release"}])),
        (2, {
            let mut v = checklist();
            v.as_object_mut().unwrap().remove("items");
            v
        }),
        (2, {
            let mut v = checklist();
            v["items"][0]["isFinished"] = json!("false");
            v
        }),
        (2, {
            let mut v = checklist();
            v["boardId"] = json!("");
            v
        }),
        (2, {
            let mut v = checklist();
            v["boardId"] = json!(1);
            v
        }),
        (2, {
            let mut v = checklist();
            v["cardId"] = json!("other");
            v
        }),
        (2, {
            let mut v = checklist();
            v["_id"] = json!("other");
            v
        }),
        (2, {
            let mut v = checklist();
            v["modifiedAt"] = json!("invalid");
            v
        }),
        (5, {
            let mut v = item();
            v["checklistId"] = json!("other");
            v
        }),
        (5, {
            let mut v = item();
            v["cardId"] = json!("other");
            v
        }),
        (5, {
            let mut v = item();
            v["boardId"] = json!("");
            v
        }),
        (5, {
            let mut v = item();
            v["boardId"] = json!(1);
            v
        }),
        (5, {
            let mut v = item();
            v["_id"] = json!("other");
            v
        }),
        (5, {
            let mut v = item();
            v["createdAt"] = json!("invalid");
            v
        }),
        (5, {
            let mut v = item();
            v["sort"] = json!("1");
            v
        }),
        (1, json!({"_id":" "})),
        (4, json!({"_id":1})),
        (3, json!({"_id":"other"})),
        (6, json!({"_id":"other"})),
        (7, json!({"_id":"other"})),
    ] {
        let server = MockServer::start().await;
        let (verb, url) = endpoint(operation);
        Mock::given(method(verb))
            .and(path(url))
            .respond_with(ResponseTemplate::new(200).set_body_json(response.clone()))
            .mount(&server)
            .await;
        assert!(
            matches!(
                invoke(&client(&server), operation).await,
                Err(ClientError::Protocol { .. })
            ),
            "operation {operation}: {response}"
        );
    }
}

#[tokio::test]
async fn checklist_reads_preserve_denormalized_source_board_ids() {
    let server = MockServer::start().await;
    for (operation, mut response) in [(2, checklist()), (5, item())] {
        response["boardId"] = json!("source-board");
        Mock::given(method("GET"))
            .and(path(endpoint(operation).1))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client(&server);
    let token = SecretString::from("checklist-token".to_owned());
    let checklist = client
        .checklist("board-1", "card-1", "checklist-1", &token)
        .await
        .unwrap();
    assert_eq!(checklist.board_id.as_deref(), Some("source-board"));
    let item = client
        .checklist_item("board-1", "card-1", "checklist-1", "item-1", &token)
        .await
        .unwrap();
    assert_eq!(item.board_id.as_deref(), Some("source-board"));
}

#[test]
fn complete_checklist_documents_map_known_optional_fields_without_discarding_data() {
    let mut full = checklist();
    full["userId"] = json!("creator-1");
    full["finishedAt"] = json!("+010000-01-01T00:00:00.000Z");
    full["hideCheckedChecklistItems"] = json!(false);
    full["hideAllChecklistItems"] = json!(true);
    full["showChecklistAtMinicard"] = json!(false);
    let document: ChecklistDocument = serde_json::from_value(full).unwrap();
    let output = serde_json::to_value(document).unwrap();
    assert_eq!(output["user_id"], "creator-1");
    assert_eq!(output["hide_checked_checklist_items"], false);
    assert_eq!(output["hide_all_checklist_items"], true);
    assert_eq!(output["show_checklist_at_minicard"], false);
    assert_eq!(output["items"][0]["item_id"], "item-1");
    let mut full_item = item();
    full_item["userId"] = json!("creator-2");
    let document: ChecklistItemDocument = serde_json::from_value(full_item).unwrap();
    assert_eq!(
        serde_json::to_value(document).unwrap()["user_id"],
        "creator-2"
    );
    for explicit_null in [false, true] {
        let mut checklist = checklist();
        let mut item = item();
        for field in [
            "boardId",
            "userId",
            "finishedAt",
            "hideCheckedChecklistItems",
            "hideAllChecklistItems",
            "showChecklistAtMinicard",
        ] {
            if explicit_null {
                checklist[field] = Value::Null;
            } else {
                checklist.as_object_mut().unwrap().remove(field);
            }
        }
        for field in ["boardId", "userId", "createdAt"] {
            if explicit_null {
                item[field] = Value::Null;
            } else {
                item.as_object_mut().unwrap().remove(field);
            }
        }
        let checklist = serde_json::from_value::<ChecklistDocument>(checklist).unwrap();
        assert_eq!(checklist.board_id, None);
        assert_eq!(checklist.user_id, None);
        let item = serde_json::from_value::<ChecklistItemDocument>(item).unwrap();
        assert_eq!(item.created_at, None);
        assert_eq!(item.user_id, None);
    }
}

#[tokio::test]
async fn checklist_creator_ids_accept_optional_strings_and_reject_invalid_values() {
    for operation in [2, 5] {
        for creator in [None, Some(Value::Null), Some(json!("creator-1"))] {
            let server = MockServer::start().await;
            let mut response = success(operation);
            if let Some(creator) = creator {
                response["userId"] = creator;
            }
            let (verb, url) = endpoint(operation);
            Mock::given(method(verb))
                .and(path(url))
                .respond_with(ResponseTemplate::new(200).set_body_json(response))
                .mount(&server)
                .await;
            invoke(&client(&server), operation).await.unwrap();
        }
        for creator in [
            json!(""),
            json!("  "),
            json!(false),
            json!(1),
            json!([]),
            json!({}),
        ] {
            let server = MockServer::start().await;
            let mut response = success(operation);
            response["userId"] = creator;
            let (verb, url) = endpoint(operation);
            Mock::given(method(verb))
                .and(path(url))
                .respond_with(ResponseTemplate::new(200).set_body_json(response))
                .mount(&server)
                .await;
            assert!(matches!(
                invoke(&client(&server), operation).await,
                Err(ClientError::Protocol { .. })
            ));
        }
    }
}

#[tokio::test]
async fn missing_checklist_and_empty_item_errors_keep_the_real_http_status() {
    for (operation, status) in [(2, 500), (5, 404)] {
        let server = MockServer::start().await;
        let (verb, url) = endpoint(operation);
        Mock::given(method(verb))
            .and(path(url))
            .respond_with(ResponseTemplate::new(status))
            .mount(&server)
            .await;
        assert!(
            matches!(invoke(&client(&server),operation).await, Err(ClientError::Server { status: actual, .. }) if actual.as_u16()==status)
        );
    }
}
