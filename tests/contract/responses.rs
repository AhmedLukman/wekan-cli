use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

use secrecy::ExposeSecret;
use serde_json::json;
use wekan_cli::client::{
    BoardType, BoardWatchLevel, ClientError, CreateCardRequest, CreateCommentRequest,
    CreateListRequest, CreateSwimlaneRequest, UpdateCardRequest, UpdateListRequest,
    UpdateSwimlaneRequest,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

use super::{
    SecretString, client, client_from_url, create_board_request, create_user_request,
    login_request, logout_request, logout_token, register_request, status_token, user_card_query,
    user_token,
};

fn complete_card_response() -> serde_json::Value {
    json!({
        "_id": "card-1",
        "title": "Todo",
        "archived": false,
        "archivedAt": "2030-01-01T00:00:00Z",
        "deletedAt": "2030-01-01T01:00:00Z",
        "deletedBy": "user-2",
        "deleteBatchId": "batch-1",
        "parentId": "parent-1",
        "listId": "list-1",
        "swimlaneId": "swimlane-1",
        "boardId": "board-1",
        "coverId": "cover-1",
        "color": "#12aBcF",
        "createdAt": "2030-01-02T03:04:05Z",
        "modifiedAt": "2030-01-02T03:05:05Z",
        "customFields": [
            {"_id": "field-1", "value": "text"},
            {"_id": "field-2", "value": 2.5},
            {"_id": "field-3", "value": true},
            {"_id": "field-4", "value": ["one", "two"]},
            {"_id": "field-5", "value": null}
        ],
        "dateLastActivity": "2030-01-02T03:06:05Z",
        "description": "Details",
        "requestedBy": "Requester",
        "assignedBy": "Dispatcher",
        "labelIds": ["label-1"],
        "members": ["user-1"],
        "assignees": ["user-2"],
        "requesters": ["user-3"],
        "assigners": ["user-4"],
        "receivedAt": "2030-01-03T00:00:00Z",
        "startAt": "2030-01-04T00:00:00Z",
        "dueAt": "2030-01-05T00:00:00Z",
        "endAt": "2030-01-06T00:00:00Z",
        "dueComplete": true,
        "stickers": [{"icon": "star", "name": "Star", "highlight": "round", "position": 1}],
        "locationName": "Office",
        "locationAddress": "Main Street",
        "locationLatitude": 1.5,
        "locationLongitude": 2.5,
        "locations": [{"_id": "location-1", "name": "Office", "address": "Main Street", "latitude": 1.5, "longitude": 2.5}],
        "spentTime": 3.5,
        "isOvertime": false,
        "userId": "user-1",
        "sort": 2.5,
        "subtaskSort": -1,
        "type": "cardType-card",
        "linkedId": "linked-1",
        "cardDependencies": [{"cardId": "card-2", "type": "blocks", "color": "red", "icon": "link"}],
        "vote": {"question": "Ship?", "positive": ["user-1"], "negative": [], "end": "2030-01-07T00:00:00Z", "public": true, "allowNonBoardMembers": false},
        "poker": {"question": true, "one": ["user-1"], "two": [], "three": [], "five": [], "eight": [], "thirteen": [], "twenty": [], "forty": [], "oneHundred": [], "unsure": [], "end": "2030-01-08T00:00:00Z", "allowNonBoardMembers": false, "estimation": 5},
        "targetId_gantt": ["card-2"],
        "linkType_gantt": [1],
        "linkId_gantt": ["link-1"],
        "cardNumber": 42,
        "showActivities": true,
        "showListOnMinicard": true,
        "showChecklistAtMinicard": false,
        "hideFinishedChecklistIfItemsAreHidden": true
    })
}

#[tokio::test]
async fn comment_responses_decode_complete_and_compact_v11_06_shapes() {
    let server = MockServer::start().await;
    let base = "/api/boards/board-1/cards/card-1/comments";
    Mock::given(method("GET"))
        .and(path(base))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "comment-1",
            "comment": "Hello",
            "authorId": "user-1"
        }])))
        .expect(1)
        .mount(&server)
        .await;
    for (comment_id, parent) in [
        ("absent-parent", None),
        ("empty-parent", Some(json!(""))),
        ("null-parent", Some(serde_json::Value::Null)),
        ("reply", Some(json!("comment-0"))),
    ] {
        let mut response = json!({
            "_id": comment_id,
            "boardId": "board-1",
            "cardId": "card-1",
            "text": "Hello",
            "createdAt": "2030-01-02T03:04:05.000Z",
            "modifiedAt": "2030-01-02T03:05:05.000Z",
            "userId": "user-1"
        });
        if let Some(parent) = parent {
            response["parentId"] = parent;
        }
        Mock::given(method("GET"))
            .and(path(format!("{base}/{comment_id}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
    }

    let client = client(&server);
    let comments = client
        .card_comments("board-1", "card-1", &user_token())
        .await
        .unwrap();
    assert_eq!(comments[0].comment_id, "comment-1");
    assert_eq!(comments[0].text, "Hello");
    assert_eq!(comments[0].author_id, "user-1");
    for comment_id in ["absent-parent", "empty-parent", "null-parent"] {
        let comment = client
            .comment("board-1", "card-1", comment_id, &user_token())
            .await
            .unwrap();
        assert_eq!(comment.parent_id, None);
    }
    let reply = client
        .comment("board-1", "card-1", "reply", &user_token())
        .await
        .unwrap();
    assert_eq!(reply.parent_id.as_deref(), Some("comment-0"));
}

#[tokio::test]
async fn comment_responses_reject_unmapped_missing_and_invalid_fields() {
    let server = MockServer::start().await;
    let base = "/api/boards/board-1/cards/card-1/comments";
    Mock::given(method("GET"))
        .and(path(format!("{base}/unknown")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "unknown",
            "boardId": "board-1",
            "cardId": "card-1",
            "text": "Hello",
            "createdAt": "2030-01-02T03:04:05Z",
            "modifiedAt": "2030-01-02T03:05:05Z",
            "userId": "user-1",
            "futureField": true
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{base}/missing-author")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "missing-author",
            "boardId": "board-1",
            "cardId": "card-1",
            "text": "Hello",
            "createdAt": "2030-01-02T03:04:05Z",
            "modifiedAt": "2030-01-02T03:05:05Z"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{base}/invalid-date")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "invalid-date",
            "boardId": "board-1",
            "cardId": "card-1",
            "text": "Hello",
            "createdAt": "yesterday",
            "modifiedAt": "2030-01-02T03:05:05Z",
            "userId": "user-1"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/cards/unknown-list/comments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "comment-1",
            "comment": "Hello",
            "authorId": "user-1",
            "futureField": true
        }])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/cards/embedded/comments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "forbidden",
            "reason": "board access denied",
            "statusCode": 403
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{base}/malformed")))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("{base}/oversized")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server);

    for comment_id in ["unknown", "missing-author", "invalid-date"] {
        assert!(matches!(
            client
                .comment("board-1", "card-1", comment_id, &user_token())
                .await,
            Err(ClientError::Protocol {
                success_status_received: true,
                ..
            })
        ));
    }
    assert!(matches!(
        client
            .card_comments("board-1", "unknown-list", &user_token())
            .await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
    assert!(matches!(
        client
            .card_comments("board-1", "embedded", &user_token())
            .await,
        Err(ClientError::EmbeddedServer {
            http_status,
            wekan_status_code: 403,
            ..
        }) if http_status == reqwest::StatusCode::OK
    ));
    assert!(matches!(
        client
            .comment("board-1", "card-1", "malformed", &user_token())
            .await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
    assert!(matches!(
        client
            .comment("board-1", "card-1", "oversized", &user_token())
            .await,
        Err(ClientError::ResponseTooLarge { .. })
    ));
}

#[tokio::test]
async fn comment_missing_and_mutation_id_response_shapes_are_strict() {
    let server = MockServer::start().await;
    let base = "/api/boards/board-1/cards/card-1/comments";
    Mock::given(method("GET"))
        .and(path(format!("{base}/missing")))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(base))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "",
            "unexpected": true
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(format!("{base}/comment-1")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server);

    assert!(matches!(
        client
            .comment("board-1", "card-1", "missing", &user_token())
            .await,
        Err(ClientError::EmbeddedServer {
            wekan_status_code: 404,
            ..
        })
    ));
    assert!(matches!(
        client
            .create_comment(
                "board-1",
                "card-1",
                &CreateCommentRequest {
                    text: "Hello".to_owned()
                },
                &user_token()
            )
            .await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
    assert!(matches!(
        client
            .delete_comment("board-1", "card-1", "comment-1", &user_token())
            .await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
}

#[tokio::test]
async fn comment_routes_preserve_explicit_http_error_statuses() {
    let server = MockServer::start().await;
    let base = "/api/boards/board-1/cards/card-1/comments";
    for status in [401, 403, 404, 500] {
        let card_id = format!("list-error-{status}");
        Mock::given(method("GET"))
            .and(path(format!(
                "/api/boards/board-1/cards/{card_id}/comments"
            )))
            .respond_with(ResponseTemplate::new(status))
            .expect(1)
            .mount(&server)
            .await;
    }
    for status in [401, 403, 404, 500] {
        let card_id = format!("create-error-{status}");
        Mock::given(method("POST"))
            .and(path(format!(
                "/api/boards/board-1/cards/{card_id}/comments"
            )))
            .respond_with(ResponseTemplate::new(status))
            .expect(1)
            .mount(&server)
            .await;
    }
    for status in [401, 403, 404, 500] {
        let comment_id = format!("get-error-{status}");
        Mock::given(method("GET"))
            .and(path(format!("{base}/{comment_id}")))
            .respond_with(ResponseTemplate::new(status))
            .expect(1)
            .mount(&server)
            .await;
    }
    for status in [401, 403, 404, 500] {
        let comment_id = format!("delete-error-{status}");
        Mock::given(method("DELETE"))
            .and(path(format!("{base}/{comment_id}")))
            .respond_with(ResponseTemplate::new(status))
            .expect(1)
            .mount(&server)
            .await;
    }
    let client = client(&server);
    let token = user_token();
    let request = CreateCommentRequest {
        text: "Hello".to_owned(),
    };

    for status in [401, 403, 404, 500] {
        let card_id = format!("list-error-{status}");
        let error = client
            .card_comments("board-1", &card_id, &token)
            .await
            .unwrap_err();
        assert!(
            matches!(error, ClientError::Server { status: http_status, .. } if http_status.as_u16() == status)
        );
    }
    for status in [401, 403, 404, 500] {
        let card_id = format!("create-error-{status}");
        let error = client
            .create_comment("board-1", &card_id, &request, &token)
            .await
            .unwrap_err();
        assert!(
            matches!(error, ClientError::Server { status: http_status, .. } if http_status.as_u16() == status)
        );
    }
    for status in [401, 403, 404, 500] {
        let comment_id = format!("get-error-{status}");
        let error = client
            .comment("board-1", "card-1", &comment_id, &token)
            .await
            .unwrap_err();
        assert!(
            matches!(error, ClientError::Server { status: http_status, .. } if http_status.as_u16() == status)
        );
    }
    for status in [401, 403, 404, 500] {
        let comment_id = format!("delete-error-{status}");
        let error = client
            .delete_comment("board-1", "card-1", &comment_id, &token)
            .await
            .unwrap_err();
        assert!(
            matches!(error, ClientError::Server { status: http_status, .. } if http_status.as_u16() == status)
        );
    }
}

#[tokio::test]
async fn user_resource_responses_decode_the_mapped_shapes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/users"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            { "_id": "user-1", "username": "alice" },
            { "_id": "email-only" }
        ])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/user/cards"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "card-1",
            "title": "Task",
            "boardId": "board-1",
            "swimlaneId": "swimlane-1",
            "listId": "list-1",
            "dueAt": "2026-08-27T12:00:00.000Z",
            "members": ["user-1"],
            "assignees": ["user-2"]
        }])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/users/user-1/boards"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            { "_id": "board-1", "title": "Board" }
        ])))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    let users = client.users(&user_token()).await.unwrap();
    assert_eq!(users[0].username.as_deref(), Some("alice"));
    assert_eq!(users[1].user_id, "email-only");
    assert_eq!(users[1].username, None);
    let cards = client
        .user_cards(&user_card_query(), &user_token())
        .await
        .unwrap();
    assert_eq!(cards[0].card_id, "card-1");
    assert_eq!(cards[0].members, ["user-1"]);
    let boards = client.user_boards("user-1", &user_token()).await.unwrap();
    assert_eq!(boards[0].title, "Board");
}

#[tokio::test]
async fn compact_user_responses_reject_unmapped_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/users"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "user-1",
            "username": "alice",
            "services": {"secret": true}
        }])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/user/cards"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "card-1",
            "description": "unmapped"
        }])))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    assert!(matches!(
        client.users(&user_token()).await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
    assert!(matches!(
        client.user_cards(&user_card_query(), &user_token()).await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
}

#[tokio::test]
async fn list_responses_decode_the_complete_v11_06_shapes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "list-1",
            "title": "Todo",
            "modifiedAt": "2026-08-28T01:00:00Z",
            "cardsModifiedAt": null
        }])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/list-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "list-1",
            "title": "Todo",
            "starred": true,
            "archived": false,
            "archivedAt": "2026-08-26T00:00:00Z",
            "deletedAt": "2026-08-27T00:00:00Z",
            "deletedBy": "user-1",
            "deleteBatchId": "batch-1",
            "boardId": "board-1",
            "swimlaneId": "swimlane-1",
            "createdAt": "2026-08-25T00:00:00Z",
            "sort": 1.5,
            "updatedAt": "2026-08-28T00:00:00Z",
            "modifiedAt": "2026-08-28T01:00:00Z",
            "_updatedAt": "2026-08-28T00:30:00Z",
            "wipLimit": {"value": 3, "enabled": true, "soft": false},
            "color": "#12aBcF",
            "type": "template-list",
            "width": 320
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    let summaries = client.board_lists("board-1", &user_token()).await.unwrap();
    assert_eq!(summaries[0].list_id, "list-1");
    assert_eq!(summaries[0].cards_modified_at, None);

    let list = client
        .list("board-1", "list-1", &user_token())
        .await
        .unwrap();
    assert_eq!(list.list_type, "template-list");
    assert_eq!(
        list.position_updated_at.as_deref(),
        Some("2026-08-28T00:30:00Z")
    );
    assert_eq!(list.wip_limit.unwrap().value, 3.into());
}

#[tokio::test]
async fn list_document_accepts_omitted_optional_fields_and_unrestricted_type() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/list-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "list-1",
            "title": "Todo",
            "archived": false,
            "boardId": "board-1",
            "createdAt": "2026-08-25T00:00:00Z",
            "modifiedAt": "2026-08-28T01:00:00Z",
            "type": "custom-list-type"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let list = client(&server)
        .list("board-1", "list-1", &user_token())
        .await
        .unwrap();
    assert_eq!(list.starred, None);
    assert_eq!(list.swimlane_id, None);
    assert_eq!(list.wip_limit, None);
    assert_eq!(list.width, None);
    assert_eq!(list.list_type, "custom-list-type");

    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/blank-color"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "blank-color",
            "title": "No color",
            "archived": false,
            "boardId": "board-1",
            "createdAt": "2026-08-25T00:00:00Z",
            "modifiedAt": "2026-08-28T01:00:00Z",
            "type": "list",
            "color": ""
        })))
        .expect(1)
        .mount(&server)
        .await;

    let blank_color = client(&server)
        .list("board-1", "blank-color", &user_token())
        .await
        .unwrap();
    assert_eq!(blank_color.color.as_deref(), Some(""));
}

#[tokio::test]
async fn list_responses_reject_unmapped_top_level_and_nested_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "list-1",
            "title": "Todo",
            "modifiedAt": null,
            "cardsModifiedAt": null,
            "future": true
        }])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/list-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "list-1",
            "title": "Todo",
            "archived": false,
            "boardId": "board-1",
            "createdAt": "2026-08-25T00:00:00Z",
            "modifiedAt": "2026-08-28T01:00:00Z",
            "type": "list",
            "wipLimit": {"value": 3, "enabled": true, "soft": false, "future": true}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    assert!(matches!(
        client.board_lists("board-1", &user_token()).await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
    assert!(matches!(
        client.list("board-1", "list-1", &user_token()).await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
}

#[tokio::test]
async fn list_documents_reject_invalid_known_values() {
    for (suffix, field) in [
        ("date", json!({"modifiedAt": "not-a-date"})),
        ("color", json!({"color": "belize"})),
        ("width", json!({"width": 99})),
    ] {
        let server = MockServer::start().await;
        let mut response = json!({
            "_id": suffix,
            "title": "Todo",
            "archived": false,
            "boardId": "board-1",
            "createdAt": "2026-08-25T00:00:00Z",
            "modifiedAt": "2026-08-28T01:00:00Z",
            "type": "list"
        });
        response
            .as_object_mut()
            .unwrap()
            .extend(field.as_object().unwrap().clone());
        Mock::given(method("GET"))
            .and(path(format!("/api/boards/board-1/lists/{suffix}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            client(&server).list("board-1", suffix, &user_token()).await,
            Err(ClientError::Protocol {
                success_status_received: true,
                ..
            })
        ));
    }
}

#[tokio::test]
async fn list_missing_and_mutation_response_shapes_are_strict() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/missing"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/boards/board-1/lists"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "list-1",
            "future": true
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/boards/board-1/lists/list-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": 1})))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    assert!(matches!(
        client.list("board-1", "missing", &user_token()).await,
        Err(ClientError::EmbeddedServer {
            http_status: reqwest::StatusCode::OK,
            wekan_status_code: 404,
            ..
        })
    ));
    assert!(matches!(
        client
            .create_list(
                "board-1",
                &CreateListRequest {
                    title: "Todo".to_owned(),
                    swimlane_id: None,
                },
                &user_token()
            )
            .await,
        Err(ClientError::Protocol { .. })
    ));
    assert!(matches!(
        client
            .update_list(
                "board-1",
                "list-1",
                &UpdateListRequest {
                    title: Some("Doing".to_owned()),
                    color: None,
                    starred: None,
                    wip_limit: None,
                },
                &user_token()
            )
            .await,
        Err(ClientError::Protocol { .. })
    ));
}

#[tokio::test]
async fn list_responses_preserve_embedded_errors_and_reject_malformed_or_oversized_bodies() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/embedded/lists"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "forbidden",
            "reason": "board access denied",
            "statusCode": 403
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/malformed"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/oversized"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    assert!(matches!(
        client.board_lists("embedded", &user_token()).await,
        Err(ClientError::EmbeddedServer {
            http_status,
            wekan_status_code: 403,
            ..
        }) if http_status == reqwest::StatusCode::OK
    ));
    assert!(matches!(
        client.list("board-1", "malformed", &user_token()).await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
    assert!(matches!(
        client.list("board-1", "oversized", &user_token()).await,
        Err(ClientError::ResponseTooLarge { .. })
    ));
}

#[tokio::test]
async fn card_responses_decode_complete_documents_and_normalize_omissions() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/list-1/cards"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "card-1",
            "title": "Todo",
            "description": "Details",
            "swimlaneId": "swimlane-1",
            "receivedAt": "2030-01-03T00:00:00Z",
            "assignees": null,
            "sort": 2.5
        }])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/list-1/cards/card-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(complete_card_response()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/list-1/cards/minimal"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "minimal",
            "archived": false,
            "parentId": "",
            "coverId": "",
            "linkedId": "",
            "swimlaneId": "swimlane-1",
            "createdAt": "2030-01-02T03:04:05Z",
            "modifiedAt": "2030-01-02T03:04:05Z",
            "dateLastActivity": "2030-01-02T03:04:05Z",
            "userId": "user-1",
            "type": "cardType-card",
            "showActivities": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    let summaries = client
        .board_list_cards("board-1", "list-1", &user_token())
        .await
        .unwrap();
    assert_eq!(summaries[0].card_id, "card-1");
    assert!(summaries[0].assignees.is_empty());
    assert_eq!(summaries[0].due_at, None);

    let card = client
        .card("board-1", "list-1", "card-1", &user_token())
        .await
        .unwrap();
    assert_eq!(card.card_type, "cardType-card");
    assert_eq!(card.custom_fields.len(), 5);
    assert_eq!(card.stickers.len(), 1);
    assert_eq!(card.locations[0].location_id, "location-1");
    assert_eq!(card.card_dependencies[0].card_id, "card-2");
    assert_eq!(
        card.poker.as_ref().unwrap().one_hundred,
        Vec::<String>::new()
    );

    let minimal = client
        .card("board-1", "list-1", "minimal", &user_token())
        .await
        .unwrap();
    assert_eq!(minimal.parent_id, None);
    assert_eq!(minimal.cover_id, None);
    assert_eq!(minimal.linked_id, None);
    assert!(minimal.label_ids.is_empty());
    assert!(minimal.members.is_empty());
    assert!(minimal.custom_fields.is_empty());
    assert_eq!(minimal.title, None);
}

#[tokio::test]
async fn card_responses_reject_unmapped_top_level_and_nested_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/list-1/cards"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "card-1",
            "future": true
        }])))
        .expect(1)
        .mount(&server)
        .await;

    let nested_cases = [
        ("custom", vec!["customFields", "0"]),
        ("sticker", vec!["stickers", "0"]),
        ("location", vec!["locations", "0"]),
        ("dependency", vec!["cardDependencies", "0"]),
        ("vote", vec!["vote"]),
        ("poker", vec!["poker"]),
    ];
    for (suffix, path_parts) in nested_cases {
        let mut response = complete_card_response();
        let mut value = &mut response;
        for part in path_parts {
            value = if let Ok(index) = part.parse::<usize>() {
                &mut value.as_array_mut().unwrap()[index]
            } else {
                value.get_mut(part).unwrap()
            };
        }
        value
            .as_object_mut()
            .unwrap()
            .insert("future".to_owned(), json!(true));
        Mock::given(method("GET"))
            .and(path(format!(
                "/api/boards/board-1/lists/list-1/cards/{suffix}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
    }
    let mut top_level = complete_card_response();
    top_level["future"] = json!(true);
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/list-1/cards/top-level"))
        .respond_with(ResponseTemplate::new(200).set_body_json(top_level))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    assert!(matches!(
        client
            .board_list_cards("board-1", "list-1", &user_token())
            .await,
        Err(ClientError::Protocol { .. })
    ));
    for suffix in [
        "custom",
        "sticker",
        "location",
        "dependency",
        "vote",
        "poker",
        "top-level",
    ] {
        assert!(matches!(
            client
                .card("board-1", "list-1", suffix, &user_token())
                .await,
            Err(ClientError::Protocol {
                success_status_received: true,
                ..
            })
        ));
    }
}

#[tokio::test]
async fn card_responses_reject_missing_required_vote_fields() {
    let server = MockServer::start().await;
    for (suffix, field) in [
        ("vote-question", "question"),
        ("vote-public", "public"),
        ("vote-allow-non-board-members", "allowNonBoardMembers"),
    ] {
        let mut response = complete_card_response();
        response["vote"].as_object_mut().unwrap().remove(field);
        Mock::given(method("GET"))
            .and(path(format!(
                "/api/boards/board-1/lists/list-1/cards/{suffix}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
    }

    let client = client(&server);
    for suffix in [
        "vote-question",
        "vote-public",
        "vote-allow-non-board-members",
    ] {
        assert!(matches!(
            client
                .card("board-1", "list-1", suffix, &user_token())
                .await,
            Err(ClientError::Protocol {
                success_status_received: true,
                ..
            })
        ));
    }
}

#[tokio::test]
async fn card_documents_reject_invalid_known_values() {
    for (suffix, field) in [
        ("id", json!({"_id": ""})),
        ("swimlane", json!({"swimlaneId": ""})),
        ("user", json!({"userId": ""})),
        ("date", json!({"modifiedAt": "not-a-date"})),
        ("color", json!({"color": "belize"})),
        ("type", json!({"type": "future-card"})),
        ("highlight", json!({"stickers": [{"highlight": "square"}]})),
        (
            "custom-value",
            json!({"customFields": [{"_id": "field-1", "value": {"raw": true}}]}),
        ),
        (
            "dependency-type",
            json!({"cardDependencies": [{"cardId": "card-2", "type": "future"}]}),
        ),
        ("member", json!({"members": [""]})),
    ] {
        let server = MockServer::start().await;
        let mut response = complete_card_response();
        response
            .as_object_mut()
            .unwrap()
            .extend(field.as_object().unwrap().clone());
        Mock::given(method("GET"))
            .and(path(format!(
                "/api/boards/board-1/lists/list-1/cards/{suffix}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            client(&server)
                .card("board-1", "list-1", suffix, &user_token())
                .await,
            Err(ClientError::Protocol {
                success_status_received: true,
                ..
            })
        ));
    }
}

#[tokio::test]
async fn card_missing_and_mutation_id_response_shapes_are_strict() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/lists/list-1/cards/missing"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/boards/board-1/lists/list-1/cards"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "card-1",
            "future": true
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/boards/board-1/lists/list-1/cards/card-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": 1})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api/boards/board-1/lists/list-1/cards/card-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": ""})))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    assert!(matches!(
        client
            .card("board-1", "list-1", "missing", &user_token())
            .await,
        Err(ClientError::EmbeddedServer {
            http_status: reqwest::StatusCode::OK,
            wekan_status_code: 404,
            ..
        })
    ));
    assert!(matches!(
        client
            .create_card(
                "board-1",
                "list-1",
                &CreateCardRequest {
                    title: "Todo".to_owned(),
                    swimlane_id: "swimlane-1".to_owned(),
                    description: None,
                    members: None,
                    assignees: None,
                    received_at: None,
                    start_at: None,
                    due_at: None,
                    end_at: None,
                },
                &user_token()
            )
            .await,
        Err(ClientError::Protocol { .. })
    ));
    let empty_update = UpdateCardRequest {
        title: None,
        sort: None,
        parent_id: None,
        description: None,
        color: None,
        label_ids: None,
        requested_by: None,
        assigned_by: None,
        received_at: None,
        start_at: None,
        due_at: None,
        end_at: None,
        spent_time: None,
        is_over_time: None,
        members: None,
        assignees: None,
        due_complete: Some(false),
    };
    assert!(matches!(
        client
            .update_card("board-1", "list-1", "card-1", &empty_update, &user_token())
            .await,
        Err(ClientError::Protocol { .. })
    ));
    assert!(matches!(
        client
            .delete_card("board-1", "list-1", "card-1", &user_token())
            .await,
        Err(ClientError::Protocol { .. })
    ));
}

#[tokio::test]
async fn swimlane_responses_decode_complete_and_optional_v11_06_shapes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/swimlanes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "swimlane-1",
            "title": "Delivery"
        }])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/swimlanes/swimlane-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "swimlane-1",
            "title": "Delivery",
            "archived": true,
            "archivedAt": "2026-08-26T00:00:00Z",
            "boardId": "board-1",
            "createdAt": "2026-08-25T00:00:00Z",
            "sort": 1.5,
            "color": "#12aBcF",
            "updatedAt": "2026-08-28T00:00:00Z",
            "modifiedAt": "2026-08-28T01:00:00Z",
            "type": "template-swimlane",
            "height": 320
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/swimlanes/minimal"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "minimal",
            "title": "Minimal",
            "archived": false,
            "boardId": "board-1",
            "createdAt": "2026-08-25T00:00:00Z",
            "color": "",
            "modifiedAt": "2026-08-28T01:00:00Z",
            "type": "custom-swimlane-type"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    let summaries = client
        .board_swimlanes("board-1", &user_token())
        .await
        .unwrap();
    assert_eq!(summaries[0].swimlane_id, "swimlane-1");

    let swimlane = client
        .swimlane("board-1", "swimlane-1", &user_token())
        .await
        .unwrap();
    assert_eq!(swimlane.swimlane_type, "template-swimlane");
    assert_eq!(swimlane.height, Some(320.into()));
    assert_eq!(
        swimlane.sort,
        Some(serde_json::Number::from_f64(1.5).unwrap())
    );

    let minimal = client
        .swimlane("board-1", "minimal", &user_token())
        .await
        .unwrap();
    assert_eq!(minimal.archived_at, None);
    assert_eq!(minimal.sort, None);
    assert_eq!(minimal.color.as_deref(), Some(""));
    assert_eq!(minimal.updated_at, None);
    assert_eq!(minimal.height, None);
    assert_eq!(minimal.swimlane_type, "custom-swimlane-type");
}

#[tokio::test]
async fn swimlane_responses_reject_unmapped_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/swimlanes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "_id": "swimlane-1",
            "title": "Delivery",
            "future": true
        }])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/swimlanes/swimlane-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "swimlane-1",
            "title": "Delivery",
            "archived": false,
            "boardId": "board-1",
            "createdAt": "2026-08-25T00:00:00Z",
            "modifiedAt": "2026-08-28T01:00:00Z",
            "type": "swimlane",
            "future": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    assert!(matches!(
        client.board_swimlanes("board-1", &user_token()).await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
    assert!(matches!(
        client
            .swimlane("board-1", "swimlane-1", &user_token())
            .await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
}

#[tokio::test]
async fn swimlane_documents_reject_invalid_known_values() {
    for (suffix, field) in [
        ("date", json!({"modifiedAt": "not-a-date"})),
        ("color", json!({"color": "belize"})),
        ("height-low", json!({"height": 49})),
        ("height-gap", json!({"height": 0})),
        ("height-high", json!({"height": 2001})),
        ("id", json!({"_id": ""})),
        ("title", json!({"title": ""})),
        ("board", json!({"boardId": ""})),
        ("type", json!({"type": ""})),
    ] {
        let server = MockServer::start().await;
        let mut response = json!({
            "_id": suffix,
            "title": "Delivery",
            "archived": false,
            "boardId": "board-1",
            "createdAt": "2026-08-25T00:00:00Z",
            "modifiedAt": "2026-08-28T01:00:00Z",
            "type": "swimlane"
        });
        response
            .as_object_mut()
            .unwrap()
            .extend(field.as_object().unwrap().clone());
        Mock::given(method("GET"))
            .and(path(format!("/api/boards/board-1/swimlanes/{suffix}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            client(&server)
                .swimlane("board-1", suffix, &user_token())
                .await,
            Err(ClientError::Protocol {
                success_status_received: true,
                ..
            })
        ));
    }

    for valid_height in [-1, 50, 2000] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!(
                "/api/boards/board-1/swimlanes/height-{valid_height}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_id": format!("height-{valid_height}"),
                "title": "Delivery",
                "archived": false,
                "boardId": "board-1",
                "createdAt": "2026-08-25T00:00:00Z",
                "modifiedAt": "2026-08-28T01:00:00Z",
                "type": "swimlane",
                "height": valid_height
            })))
            .expect(1)
            .mount(&server)
            .await;
        client(&server)
            .swimlane("board-1", &format!("height-{valid_height}"), &user_token())
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn swimlane_missing_and_mutation_response_shapes_are_strict() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/swimlanes/missing"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/boards/board-1/swimlanes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "swimlane-1",
            "future": true
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/boards/board-1/swimlanes/swimlane-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": 1})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api/boards/board-1/swimlanes/swimlane-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id": ""})))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    assert!(matches!(
        client.swimlane("board-1", "missing", &user_token()).await,
        Err(ClientError::EmbeddedServer {
            http_status: reqwest::StatusCode::OK,
            wekan_status_code: 404,
            ..
        })
    ));
    assert!(matches!(
        client
            .create_swimlane(
                "board-1",
                &CreateSwimlaneRequest {
                    title: "Delivery".to_owned(),
                    sort: None,
                },
                &user_token()
            )
            .await,
        Err(ClientError::Protocol { .. })
    ));
    assert!(matches!(
        client
            .update_swimlane(
                "board-1",
                "swimlane-1",
                &UpdateSwimlaneRequest {
                    title: "Operations".to_owned(),
                },
                &user_token()
            )
            .await,
        Err(ClientError::Protocol { .. })
    ));
    assert!(matches!(
        client
            .delete_swimlane("board-1", "swimlane-1", &user_token())
            .await,
        Err(ClientError::Protocol { .. })
    ));
}

#[tokio::test]
async fn swimlane_responses_preserve_errors_and_reject_malformed_or_oversized_bodies() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/embedded/swimlanes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "forbidden",
            "reason": "board access denied",
            "statusCode": 403
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/swimlanes/malformed"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1/swimlanes/oversized"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    assert!(matches!(
        client.board_swimlanes("embedded", &user_token()).await,
        Err(ClientError::EmbeddedServer {
            http_status,
            wekan_status_code: 403,
            ..
        }) if http_status == reqwest::StatusCode::OK
    ));
    assert!(matches!(
        client.swimlane("board-1", "malformed", &user_token()).await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
    assert!(matches!(
        client.swimlane("board-1", "oversized", &user_token()).await,
        Err(ClientError::ResponseTooLarge { .. })
    ));
}

#[tokio::test]
async fn error_envelopes_reject_unmapped_fields_in_production_paths() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/users"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "invalid-request",
            "unexpected": true
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/users/missing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": 404,
            "error": "user-not-found",
            "unexpected": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    assert!(matches!(
        client.users(&user_token()).await,
        Err(ClientError::Protocol {
            success_status_received: false,
            ..
        })
    ));
    assert!(matches!(
        client.user("missing", &user_token()).await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
}

#[tokio::test]
async fn board_document_decodes_the_complete_v11_06_shape() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "board-1",
            "title": "Delivery",
            "slug": "delivery",
            "archived": false,
            "archivedAt": "2026-08-28T01:00:00.000Z",
            "createdAt": "2026-08-27T01:00:00.000Z",
            "modifiedAt": "2026-08-28T02:00:00.000Z",
            "stars": 3,
            "labels": [{"_id": "label-1", "name": "Urgent", "color": "red"}],
            "members": [{
                "userId": "user-1",
                "isAdmin": true,
                "isActive": true,
                "isNoComments": false,
                "isCommentOnly": false,
                "isWorker": false,
                "isNormalAssignedOnly": false,
                "isCommentAssignedOnly": false,
                "isReadOnly": false,
                "isReadAssignedOnly": false
            }],
            "watchers": [{"userId": "watcher-1", "level": "tracking"}],
            "permission": "private",
            "orgs": [{"orgId": "org-1", "orgDisplayName": "Org", "isActive": true}],
            "teams": [{"teamId": "team-1", "teamDisplayName": "Team", "isActive": true}],
            "domains": [{"domain": "example.test", "isActive": true}],
            "importUsernames": ["legacy-user"],
            "color": "belize",
            "customThemeColors": ["#112233", "#445566"],
            "backgroundImageURL": "https://images.example/board.png",
            "backgroundImageId": "attachment-1",
            "allowsCardCounterList": false,
            "cardAging": true,
            "showDependencies": true,
            "cardAgingDays1": 7,
            "cardAgingDays2": 14,
            "cardAgingDays3": 28,
            "allowsBoardMemberList": true,
            "description": "Board description",
            "subtasksDefaultBoardId": "board-2",
            "migrationVersion": 1,
            "subtasksDefaultListId": "list-2",
            "dateSettingsDefaultBoardId": "board-3",
            "dateSettingsDefaultListId": "list-3",
            "allowsSubtasks": true,
            "allowsSubtasksOnMinicard": true,
            "allowsAttachments": true,
            "allowsAttachmentsOnMinicard": true,
            "allowsChecklists": true,
            "allowsChecklistsOnMinicard": true,
            "allowsCustomFields": true,
            "allowsCustomFieldsOnMinicard": false,
            "allowsChecklistCountBadgeOnMinicard": false,
            "allowsComments": true,
            "allowsDescriptionTitle": true,
            "allowsDescriptionTitleOnMinicard": true,
            "allowsDescriptionText": true,
            "allowsDescriptionTextOnMinicard": false,
            "allowsCoverAttachmentOnMinicard": true,
            "allowsCoverAttachmentOnCard": false,
            "allowsBadgeAttachmentOnMinicard": false,
            "allowsAttachmentCountOnCard": false,
            "allowsChecklistCountBadgeOnCard": false,
            "allowsCardSortingByNumberOnMinicard": false,
            "allowsCardNumber": false,
            "allowsCardNumberOnMinicard": false,
            "allowsActivities": true,
            "allowsLabels": true,
            "allowsLabelsOnMinicard": true,
            "allowsCreator": true,
            "allowsCreatorOnMinicard": false,
            "allowsAssignee": true,
            "allowsAssigneeOnMinicard": true,
            "allowsMembers": true,
            "allowsMembersOnMinicard": true,
            "allowsRequestedBy": true,
            "allowsRequestedByOnMinicard": true,
            "allowsCardSortingByNumber": true,
            "allowsShowLists": true,
            "allowsAssignedBy": true,
            "allowsAssignedByOnMinicard": true,
            "allowsShowListsOnMinicard": false,
            "allowsChecklistAtMinicard": false,
            "allowsReceivedDate": true,
            "restrictCommentEditing": false,
            "allowsPersonalListWidth": false,
            "autoWidth": false,
            "allowsReceivedDateOnMinicard": true,
            "allowsStartDate": true,
            "allowsStartDateOnMinicard": true,
            "allowsEndDate": true,
            "allowsEndDateOnMinicard": true,
            "allowsDueDate": true,
            "allowsDueDateOnMinicard": true,
            "allowsDueComplete": false,
            "allowsDueCompleteOnMinicard": false,
            "presentParentTask": "no-parent",
            "receivedAt": "2026-08-27T02:00:00.000Z",
            "startAt": "2026-08-27T03:00:00.000Z",
            "dueAt": "2026-08-29T03:00:00.000Z",
            "endAt": "2026-08-30T03:00:00.000Z",
            "spentTime": 90,
            "isOvertime": false,
            "type": "board",
            "sort": 1,
            "showActivities": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let board = client(&server)
        .board("board-1", &user_token())
        .await
        .unwrap();
    assert_eq!(board.board_id, "board-1");
    assert_eq!(
        board.background_image_url.as_deref(),
        Some("https://images.example/board.png")
    );
    assert_eq!(board.board_type, Some(BoardType::Board));
    assert_eq!(board.watchers[0].user_id, "watcher-1");
    assert_eq!(board.watchers[0].level, BoardWatchLevel::Tracking);

    let output = serde_json::to_value(board).unwrap();
    assert_eq!(output["board_id"], "board-1");
    assert_eq!(
        output["background_image_url"],
        "https://images.example/board.png"
    );
    assert_eq!(output["board_type"], "board");
    assert_eq!(output["watchers"][0]["user_id"], "watcher-1");
    assert_eq!(output["watchers"][0]["level"], "tracking");
    assert!(output.get("_id").is_none());
    assert!(output.get("backgroundImageURL").is_none());
}

#[tokio::test]
async fn board_document_accepts_verified_null_default_board_and_list_ids() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "board-1",
            "title": "Fresh board",
            "members": [],
            "subtasksDefaultBoardId": null,
            "subtasksDefaultListId": null,
            "dateSettingsDefaultBoardId": null,
            "dateSettingsDefaultListId": null
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_from_url(&format!("{}/", server.uri()));
    let board = client
        .board("board-1", &SecretString::from("token".to_owned()))
        .await
        .unwrap();

    assert_eq!(board.subtasks_default_board_id, None);
    assert_eq!(board.subtasks_default_list_id, None);
    assert_eq!(board.date_settings_default_board_id, None);
    assert_eq!(board.date_settings_default_list_id, None);
}

#[tokio::test]
async fn board_document_defaults_omitted_optional_collections() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/board-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "board-1",
            "title": "Board",
            "members": [{
                "userId": "user-1",
                "isAdmin": true,
                "isActive": true
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let board = client(&server)
        .board("board-1", &user_token())
        .await
        .unwrap();
    assert_eq!(board.members.len(), 1);
    assert_eq!(board.members[0].user_id, "user-1");
    assert!(board.watchers.is_empty());
    assert!(board.labels.is_empty());
    assert!(board.orgs.is_empty());
    assert!(board.teams.is_empty());
    assert!(board.domains.is_empty());
    assert!(board.import_usernames.is_empty());
    assert!(board.custom_theme_colors.is_empty());
}

#[tokio::test]
async fn board_document_rejects_null_collections_outside_the_corrected_contract() {
    for field in [
        "labels",
        "watchers",
        "orgs",
        "teams",
        "domains",
        "importUsernames",
        "customThemeColors",
    ] {
        let server = MockServer::start().await;
        let mut response = json!({
            "_id": "board-1",
            "title": "Board",
            "members": [{
                "userId": "user-1",
                "isAdmin": true,
                "isActive": true
            }]
        });
        response
            .as_object_mut()
            .unwrap()
            .insert(field.to_owned(), serde_json::Value::Null);
        Mock::given(method("GET"))
            .and(path("/api/boards/board-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;

        assert!(matches!(
            client(&server).board("board-1", &user_token()).await,
            Err(ClientError::Protocol {
                success_status_received: true,
                ..
            })
        ));
    }
}

#[tokio::test]
async fn board_document_rejects_invalid_watcher_entries() {
    for watchers in [
        json!([{"userId": "watcher-1", "level": "following"}]),
        json!([{"userId": "watcher-1"}]),
        json!([{
            "userId": "watcher-1",
            "level": "watching",
            "futureWatcherField": true
        }]),
        json!(["watcher-1"]),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/boards/board-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_id": "board-1",
                "title": "Board",
                "members": [],
                "watchers": watchers
            })))
            .expect(1)
            .mount(&server)
            .await;

        assert!(matches!(
            client(&server).board("board-1", &user_token()).await,
            Err(ClientError::Protocol {
                success_status_received: true,
                ..
            })
        ));
    }
}

#[tokio::test]
async fn board_document_rejects_values_outside_the_corrected_scalar_contract() {
    for (field, value) in [
        ("permission", json!("shared")),
        ("color", json!("future-theme")),
        ("presentParentTask", json!("beside-parent")),
        ("type", json!("future-board")),
        ("archivedAt", json!("not-a-date")),
        ("createdAt", json!("not-a-date")),
        ("modifiedAt", json!("not-a-date")),
        ("receivedAt", json!("not-a-date")),
        ("startAt", json!("not-a-date")),
        ("dueAt", json!("not-a-date")),
        ("endAt", json!("not-a-date")),
    ] {
        let server = MockServer::start().await;
        let mut response = json!({
            "_id": "board-1",
            "title": "Board",
            "members": [{
                "userId": "user-1",
                "isAdmin": true,
                "isActive": true
            }]
        });
        response
            .as_object_mut()
            .unwrap()
            .insert(field.to_owned(), value);
        Mock::given(method("GET"))
            .and(path("/api/boards/board-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;

        assert!(matches!(
            client(&server).board("board-1", &user_token()).await,
            Err(ClientError::Protocol {
                success_status_received: true,
                ..
            })
        ));
    }
}

#[tokio::test]
async fn board_document_rejects_missing_required_v11_06_fields() {
    for response in [
        json!({"_id": "board-1", "title": "Board"}),
        json!({"_id": "board-1", "title": "Board", "members": null}),
        json!({
            "_id": "board-1",
            "title": "Board",
            "members": [{"userId": "user-1", "isAdmin": true}]
        }),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/boards/board-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;

        assert!(matches!(
            client(&server).board("board-1", &user_token()).await,
            Err(ClientError::Protocol {
                success_status_received: true,
                ..
            })
        ));
    }
}

#[tokio::test]
async fn board_document_rejects_unmapped_top_level_and_nested_fields() {
    for response in [
        json!({
            "_id": "board-1",
            "title": "Board",
            "members": [],
            "futureSetting": true
        }),
        json!({
            "_id": "board-1",
            "title": "Board",
            "members": [{
                "userId": "user-1",
                "isAdmin": true,
                "isActive": true,
                "futureRole": "observer"
            }]
        }),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/boards/board-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;

        assert!(matches!(
            client(&server).board("board-1", &user_token()).await,
            Err(ClientError::Protocol {
                success_status_received: true,
                ..
            })
        ));
    }
}

#[tokio::test]
async fn board_projection_and_mutation_responses_reject_unmapped_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "_id": "board-1",
                "title": "Board",
                "futureBoardField": true
            }
        ])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards_count"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "private": 2,
            "public": 1,
            "futureCount": 3
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/boards"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "board-1",
            "defaultSwimlaneId": "swimlane-1",
            "futureCreationField": "value"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/boards/board-1/title"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "board-1",
            "title": "Renamed",
            "futureRenameField": false
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api/boards/board-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "board-1",
            "futureDeleteField": ["value"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    assert!(matches!(
        client.public_boards(&user_token()).await,
        Err(ClientError::Protocol { .. })
    ));
    assert!(matches!(
        client.board_counts(&user_token()).await,
        Err(ClientError::Protocol { .. })
    ));
    assert!(matches!(
        client
            .create_board(&create_board_request(), &user_token())
            .await,
        Err(ClientError::Protocol { .. })
    ));
    assert!(matches!(
        client
            .rename_board("board-1", "Renamed", &user_token())
            .await,
        Err(ClientError::Protocol { .. })
    ));
    assert!(matches!(
        client.delete_board("board-1", &user_token()).await,
        Err(ClientError::Protocol { .. })
    ));
}

#[tokio::test]
async fn board_responses_reject_malformed_and_oversized_bodies_and_preserve_embedded_errors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/malformed"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards_count"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/boards"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "forbidden",
            "reason": "site policy",
            "statusCode": 403
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client(&server);
    assert!(matches!(
        client.board("malformed", &user_token()).await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
    assert!(matches!(
        client.board_counts(&user_token()).await,
        Err(ClientError::ResponseTooLarge { .. })
    ));
    assert!(matches!(
        client.public_boards(&user_token()).await,
        Err(ClientError::EmbeddedServer {
            http_status,
            wekan_status_code: 403,
            ..
        }) if http_status == reqwest::StatusCode::OK
    ));
}

#[tokio::test]
async fn create_user_accepts_the_known_v11_06_empty_object_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/users"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "_id": {} })))
        .expect(1)
        .mount(&server)
        .await;

    let result = client(&server)
        .create_user(&create_user_request(), &user_token())
        .await
        .unwrap();
    assert_eq!(result, wekan_cli::client::CreateUserResult);
}

#[tokio::test]
async fn create_user_rejects_responses_outside_the_exact_v11_06_shape() {
    for response in [
        json!({ "_id": { "unexpected": true } }),
        json!({ "_id": "user-1" }),
        json!({ "_id": {}, "unexpected": true }),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;

        let error = client(&server)
            .create_user(&create_user_request(), &user_token())
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ClientError::Protocol {
                success_status_received: true,
                ..
            }
        ));
    }
}

#[tokio::test]
async fn all_user_endpoints_preserve_embedded_errors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/users/missing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "statusCode": 404,
            "error": "not-found",
            "reason": "User not found"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .user("missing", &user_token())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::EmbeddedServer {
            wekan_status_code: 404,
            ..
        }
    ));
}

#[tokio::test]
async fn user_cards_preserves_the_verified_http_401_authentication_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/user/cards"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": "Unauthorized"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .user_cards(&user_card_query(), &user_token())
        .await
        .unwrap_err();
    let ClientError::Server {
        status,
        server_error,
        server_reason,
        ..
    } = error
    else {
        panic!("expected an HTTP server error")
    };
    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(server_error.as_deref(), Some("Unauthorized"));
    assert_eq!(server_reason, None);
}

#[tokio::test]
async fn embedded_errors_without_status_code_preserve_protocol_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/users/missing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "isClientSafe": true,
            "error": "user-not-found",
            "reason": "No such user",
            "message": "No such user [user-not-found]",
            "errorType": "Meteor.Error"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .user("missing", &user_token())
        .await
        .unwrap_err();
    let ClientError::EmbeddedProtocol {
        http_status,
        server_error,
        server_reason,
        server_message,
        server_error_type,
        server_is_client_safe,
    } = error
    else {
        panic!("expected an embedded protocol error")
    };
    assert_eq!(http_status, reqwest::StatusCode::OK);
    assert_eq!(server_error.as_deref(), Some("user-not-found"));
    assert_eq!(server_reason.as_deref(), Some("No such user"));
    assert_eq!(
        server_message.as_deref(),
        Some("No such user [user-not-found]")
    );
    assert_eq!(server_error_type.as_deref(), Some("Meteor.Error"));
    assert_eq!(server_is_client_safe, Some(true));
}

#[tokio::test]
async fn user_responses_enforce_the_shared_size_limit() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/users"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .expect(1)
        .mount(&server)
        .await;

    assert!(matches!(
        client(&server).users(&user_token()).await,
        Err(ClientError::ResponseTooLarge { .. })
    ));
}

#[tokio::test]
async fn logout_success_rejects_unmapped_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/logout"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "message": "You've been logged out!",
            "ignored": "not exposed"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .logout(&logout_request(false), &logout_token())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Protocol {
            success_status_received: true,
            ..
        }
    ));
}

#[tokio::test]
async fn logout_rejections_preserve_structured_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/logout"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": "unauthorized",
            "reason": "invalid bearer token"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .logout(&logout_request(false), &logout_token())
        .await
        .unwrap_err();
    let ClientError::Server {
        status,
        server_error,
        server_reason,
        ..
    } = error
    else {
        panic!("expected a structured server error")
    };
    assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(server_error.as_deref(), Some("unauthorized"));
    assert_eq!(server_reason.as_deref(), Some("invalid bearer token"));
}

#[tokio::test]
async fn malformed_logout_success_records_the_success_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/logout"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "wrong": true })))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .logout(&logout_request(false), &logout_token())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Protocol {
            success_status_received: true,
            ..
        }
    ));
}

#[tokio::test]
async fn logout_responses_enforce_the_size_limit() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/logout"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .logout(&logout_request(false), &logout_token())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::ResponseTooLarge {
            status: reqwest::StatusCode::OK,
            ..
        }
    ));
}

#[tokio::test]
async fn current_user_success_fields_are_mapped_and_decoded() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "user-1",
            "username": "alice",
            "emails": [{"address": "alice@example.com", "verified": true}],
            "profile": {"fullname": "Alice Example", "language": "en"},
            "isAdmin": true,
            "boards": [{"boardId": "ignored"}]
        })))
        .mount(&server)
        .await;

    let user = client(&server).current_user(&status_token()).await.unwrap();

    assert_eq!(user.user_id(), "user-1");
    assert_eq!(user.username(), Some("alice"));
    assert_eq!(user.full_name(), Some("Alice Example"));
    assert_eq!(user.is_admin(), Some(true));
    assert_eq!(user.emails()[0].address(), Some("alice@example.com"));
    assert_eq!(user.emails()[0].verified(), Some(true));
}

#[tokio::test]
async fn current_user_accepts_absent_optional_profile_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "user-1"
        })))
        .mount(&server)
        .await;

    let user = client(&server).current_user(&status_token()).await.unwrap();

    assert_eq!(user.username(), None);
    assert_eq!(user.full_name(), None);
    assert_eq!(user.is_admin(), None);
    assert!(user.emails().is_empty());
}

#[tokio::test]
async fn current_user_accepts_null_optional_collections_and_profile() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_id": "user-1",
            "emails": null,
            "profile": null,
            "orgs": null,
            "teams": null,
            "boards": null
        })))
        .expect(1)
        .mount(&server)
        .await;

    let user = client(&server).current_user(&status_token()).await.unwrap();
    assert!(user.emails.is_empty());
    assert!(user.orgs.is_empty());
    assert!(user.teams.is_empty());
    assert!(user.boards.is_empty());
    assert_eq!(user.profile, Default::default());
}

#[tokio::test]
async fn current_user_preserves_embedded_authentication_errors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "Unauthorized",
            "reason": "Unauthorized",
            "statusCode": 401
        })))
        .mount(&server)
        .await;

    let error = client(&server)
        .current_user(&status_token())
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        ClientError::EmbeddedServer {
            http_status: reqwest::StatusCode::OK,
            wekan_status_code: 401,
            server_error: Some(ref value),
            server_reason: Some(_),
        } if value == "Unauthorized"
    ));
}

#[tokio::test]
async fn current_user_non_success_responses_preserve_http_status() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/user"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "error": "api-disabled",
            "reason": "API disabled"
        })))
        .mount(&server)
        .await;

    let error = client(&server)
        .current_user(&status_token())
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        ClientError::Server {
            status: reqwest::StatusCode::FORBIDDEN,
            server_error: Some(ref value),
            ..
        } if value == "api-disabled"
    ));
}

#[tokio::test]
async fn malformed_current_user_success_is_a_protocol_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "username": "missing-id"
        })))
        .mount(&server)
        .await;

    let error = client(&server)
        .current_user(&status_token())
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        ClientError::Protocol {
            success_status_received: true,
            ..
        }
    ));
}

#[tokio::test]
async fn empty_current_user_ids_remain_protocol_errors_after_client_sharing() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "_id": "" })))
        .expect(1)
        .mount(&server)
        .await;

    assert!(matches!(
        client(&server).current_user(&status_token()).await,
        Err(ClientError::Protocol {
            success_status_received: true,
            ..
        })
    ));
}

#[tokio::test]
async fn current_user_responses_enforce_the_size_limit() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/user"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .mount(&server)
        .await;

    let error = client(&server)
        .current_user(&status_token())
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        ClientError::ResponseTooLarge {
            status: reqwest::StatusCode::OK,
            ..
        }
    ));
}

#[tokio::test]
async fn truncated_current_user_responses_preserve_the_http_status() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request).unwrap();
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{\"_id\":",
            )
            .unwrap();
    });
    let client = client_from_url(&format!("http://{address}"));

    let error = client.current_user(&status_token()).await.unwrap_err();
    server.join().unwrap();

    assert!(matches!(
        error,
        ClientError::ResponseBody {
            status: reqwest::StatusCode::OK,
            ..
        }
    ));
}

#[tokio::test]
async fn login_success_fields_are_decoded() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "user-1",
            "token": "login-token",
            "tokenExpires": "2030-01-02T03:04:05Z"
        })))
        .mount(&server)
        .await;

    let session = client(&server).login(&login_request()).await.unwrap();
    assert_eq!(session.user_id(), "user-1");
    assert_eq!(session.token().expose_secret(), "login-token");
}

#[tokio::test]
async fn login_rejections_preserve_structured_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": "login-failed",
            "reason": "Incorrect username, email address or password."
        })))
        .mount(&server)
        .await;

    let error = client(&server).login(&login_request()).await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::Server {
            status: reqwest::StatusCode::UNAUTHORIZED,
            server_error: Some(ref value),
            server_reason: Some(_),
            retry_after_seconds: None,
        } if value == "login-failed"
    ));
}

#[tokio::test]
async fn login_rate_limits_preserve_retry_after_seconds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "17")
                .set_body_json(json!({
                    "error": "too-many-requests",
                    "reason": "Too many failed login attempts. Try again later."
                })),
        )
        .mount(&server)
        .await;

    let error = client(&server).login(&login_request()).await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::Server {
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            retry_after_seconds: Some(17),
            ..
        }
    ));
}

#[tokio::test]
async fn malformed_login_success_records_that_a_session_was_created() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;

    let error = client(&server).login(&login_request()).await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::Protocol {
            success_status_received: true,
            ..
        }
    ));
}

#[tokio::test]
async fn login_responses_enforce_the_size_limit() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .mount(&server)
        .await;

    let error = client(&server).login(&login_request()).await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::ResponseTooLarge {
            status: reqwest::StatusCode::OK,
            ..
        }
    ));
}

#[tokio::test]
async fn oversized_login_rate_limits_preserve_status_and_retry_after() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "19")
                .set_body_bytes(vec![b'x'; 1024 * 1024 + 1]),
        )
        .mount(&server)
        .await;

    let error = client(&server).login(&login_request()).await.unwrap_err();
    assert!(matches!(
        error,
        ClientError::ResponseTooLarge {
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            retry_after_seconds: Some(19),
            ..
        }
    ));
}

#[tokio::test]
async fn registration_success_fields_are_decoded() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "user-1",
            "token": "server-token",
            "tokenExpires": "2030-01-02T03:04:05Z"
        })))
        .mount(&server)
        .await;

    let session = client(&server).register(&register_request()).await.unwrap();
    assert_eq!(session.user_id(), "user-1");
    assert_eq!(session.token().expose_secret(), "server-token");
    assert_eq!(
        session
            .token_expires()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap(),
        "2030-01-02T03:04:05Z"
    );
}

#[tokio::test]
async fn registration_disabled_accepts_an_empty_error_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(403))
        .expect(1)
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Server {
            status: reqwest::StatusCode::FORBIDDEN,
            server_error: None,
            server_reason: None,
            ..
        }
    ));
}

#[tokio::test]
async fn structured_registration_errors_are_preserved() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "username-already-exists",
            "reason": "Username already exists."
        })))
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    match error {
        ClientError::Server {
            status,
            server_error,
            server_reason,
            ..
        } => {
            assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
            assert_eq!(server_error.as_deref(), Some("username-already-exists"));
            assert_eq!(server_reason.as_deref(), Some("Username already exists."));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn numeric_meteor_error_codes_are_preserved() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": 403,
            "reason": "Username already exists."
        })))
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    match error {
        ClientError::Server {
            status,
            server_error,
            server_reason,
            ..
        } => {
            assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
            assert_eq!(server_error.as_deref(), Some("403"));
            assert_eq!(server_reason.as_deref(), Some("Username already exists."));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn non_json_registration_errors_are_classified_by_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(500).set_body_string("upstream unavailable"))
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Server {
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            server_error: None,
            server_reason: None,
            ..
        }
    ));
}

#[tokio::test]
async fn malformed_success_is_a_possible_partial_mutation() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Protocol {
            success_status_received: true,
            ..
        }
    ));
}

#[tokio::test]
async fn malformed_success_does_not_echo_a_response_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "user-1",
            "token": 123456,
            "tokenExpires": "2030-01-02T03:04:05Z"
        })))
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();

    assert!(!error.to_string().contains("123456"));
    assert!(matches!(
        error,
        ClientError::Protocol {
            success_status_received: true,
            ..
        }
    ));
}

#[tokio::test]
async fn registration_responses_enforce_the_size_limit() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::ResponseTooLarge {
            status: reqwest::StatusCode::OK,
            ..
        }
    ));
}

#[tokio::test]
async fn oversized_registration_errors_preserve_the_http_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(400).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .mount(&server)
        .await;

    let error = client(&server)
        .register(&register_request())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::ResponseTooLarge {
            status: reqwest::StatusCode::BAD_REQUEST,
            ..
        }
    ));
}

#[tokio::test]
async fn truncated_registration_errors_preserve_the_http_status() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request).unwrap();
        stream
            .write_all(
                b"HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{\"error\":",
            )
            .unwrap();
    });
    let client = client_from_url(&format!("http://{address}"));

    let error = client.register(&register_request()).await.unwrap_err();
    server.join().unwrap();

    assert!(matches!(
        error,
        ClientError::ResponseBody {
            status: reqwest::StatusCode::BAD_REQUEST,
            ..
        }
    ));
}

#[tokio::test]
async fn truncated_logout_success_preserves_the_http_status() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request).unwrap();
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{\"message\":",
            )
            .unwrap();
    });
    let client = client_from_url(&format!("http://{address}"));

    let error = client
        .logout(&logout_request(false), &logout_token())
        .await
        .unwrap_err();
    server.join().unwrap();

    assert!(matches!(
        error,
        ClientError::ResponseBody {
            status: reqwest::StatusCode::OK,
            ..
        }
    ));
}
