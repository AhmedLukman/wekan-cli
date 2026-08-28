use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

use secrecy::ExposeSecret;
use serde_json::json;
use wekan_cli::client::{BoardType, BoardWatchLevel, ClientError};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

use super::{
    SecretString, client, client_from_url, create_board_request, create_user_request,
    login_request, logout_request, logout_token, register_request, status_token, user_card_query,
    user_token,
};

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
