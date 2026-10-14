use super::*;
use serde_json::{Value, json};
use wekan_cli::client::{
    ClientError,
    boards::CopyBoardRequest,
    cards::{CardCopyPosition, CopyCardRequest},
    lists::TransferListRequest,
    swimlanes::TransferSwimlaneRequest,
};
use wiremock::{
    Mock, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

async fn transfer(
    server: &MockServer,
    kind: &str,
    position: Option<Value>,
) -> Result<String, ClientError> {
    let client = client(server);
    let token = user_token();
    match kind {
        "board" => {
            client
                .copy_board("b/1", &CopyBoardRequest::default(), &token)
                .await
        }
        "card" => {
            client
                .copy_card(
                    "b/1",
                    "l/1",
                    "c/1",
                    &CopyCardRequest {
                        target_board_id: None,
                        target_list_id: None,
                        target_swimlane_id: "s2".into(),
                        position: position
                            .map(|p| serde_json::from_value::<CardCopyPosition>(p).unwrap()),
                    },
                    &token,
                )
                .await
        }
        "list-copy" => {
            client
                .copy_list(
                    "b/1",
                    "l/1",
                    &TransferListRequest {
                        position,
                        ..Default::default()
                    },
                    &token,
                )
                .await
        }
        "list-move" => {
            client
                .move_list(
                    "b/1",
                    "l/1",
                    &TransferListRequest {
                        position,
                        ..Default::default()
                    },
                    &token,
                )
                .await
        }
        "swimlane-copy" => {
            client
                .copy_swimlane(
                    "b/1",
                    "s/1",
                    &TransferSwimlaneRequest {
                        position,
                        ..Default::default()
                    },
                    &token,
                )
                .await
        }
        "swimlane-move" => {
            client
                .move_swimlane(
                    "b/1",
                    "s/1",
                    &TransferSwimlaneRequest {
                        position,
                        ..Default::default()
                    },
                    &token,
                )
                .await
        }
        _ => unreachable!(),
    }
}

fn endpoint(kind: &str) -> &'static str {
    match kind {
        "board" => "/api/boards/b%2F1/copy",
        "card" => "/api/boards/b%2F1/lists/l%2F1/cards/c%2F1/copy",
        "list-copy" => "/api/boards/b%2F1/lists/l%2F1/copy",
        "list-move" => "/api/boards/b%2F1/lists/l%2F1/move",
        "swimlane-copy" => "/api/boards/b%2F1/swimlanes/s%2F1/copy",
        "swimlane-move" => "/api/boards/b%2F1/swimlanes/s%2F1/move",
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn all_six_transfers_encode_ids_authenticate_and_preserve_omitted_defaults() {
    for kind in [
        "board",
        "card",
        "list-copy",
        "list-move",
        "swimlane-copy",
        "swimlane-move",
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(endpoint(kind)))
            .and(header("authorization", "Bearer user-token"))
            .and(header("content-type", "application/json"))
            .and(body_json(if kind == "card" {
                json!({"toSwimlaneId":"s2"})
            } else {
                json!({})
            }))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(if kind == "board" {
                    json!("new")
                } else {
                    json!({"_id":"new"})
                }),
            )
            .expect(1)
            .mount(&server)
            .await;
        assert_eq!(transfer(&server, kind, None).await.unwrap(), "new");
    }
}

#[tokio::test]
async fn transfer_positions_preserve_exact_json_types_and_null() {
    for kind in [
        "card",
        "list-copy",
        "list-move",
        "swimlane-copy",
        "swimlane-move",
    ] {
        let values = if kind == "card" {
            vec![json!(-2.5), json!("Infinity"), Value::Null]
        } else {
            vec![
                json!(-2.5),
                json!("Infinity"),
                Value::Null,
                json!(false),
                json!(["2"]),
                json!({"x":1}),
            ]
        };
        for value in values {
            let server = MockServer::start().await;
            let mut body = json!({"position":value});
            if kind == "card" {
                body["toSwimlaneId"] = json!("s2");
            }
            Mock::given(method("POST"))
                .and(path(endpoint(kind)))
                .and(body_json(body))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"new"})))
                .expect(1)
                .mount(&server)
                .await;
            transfer(&server, kind, Some(value)).await.unwrap();
        }
    }
}

#[tokio::test]
async fn transfer_success_bodies_reject_unknown_missing_and_invalid_ids() {
    for kind in [
        "board",
        "card",
        "list-copy",
        "list-move",
        "swimlane-copy",
        "swimlane-move",
    ] {
        for body in [
            json!({"_id":"new","future":true}),
            json!({}),
            json!({"_id":17}),
            json!({"_id":""}),
            Value::Null,
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .expect(1)
                .mount(&server)
                .await;
            assert!(matches!(
                transfer(&server, kind, None).await,
                Err(ClientError::Protocol { .. })
            ));
        }
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            transfer(&server, kind, None).await,
            Err(ClientError::Protocol { .. })
        ));
    }
}

#[tokio::test]
async fn card_counts_handle_strict_objects_and_embedded_errors() {
    for scoped in [false, true] {
        let field = if scoped {
            "list_cards_count"
        } else {
            "board_cards_count"
        };
        for body in [
            json!({field:1,"future":true}),
            json!({field:"1"}),
            json!({field:-1}),
            json!({field:1.5}),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .expect(1)
                .mount(&server)
                .await;
            let result = if scoped {
                client(&server)
                    .list_card_count("b", "l", &user_token())
                    .await
            } else {
                client(&server).board_card_count("b", &user_token()).await
            };
            assert!(matches!(result, Err(ClientError::Protocol { .. })));
        }
        for body in [
            json!({"statusCode":403,"error":"forbidden","reason":"denied"}),
            json!({"error":"not-found","reason":"missing"}),
            json!({}),
            Value::Null,
            json!(-1),
            json!({"board_cards_count":-1,"list_cards_count":-1}),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .expect(1)
                .mount(&server)
                .await;
            let result = if scoped {
                client(&server)
                    .list_card_count("b", "l", &user_token())
                    .await
            } else {
                client(&server).board_card_count("b", &user_token()).await
            };
            assert!(result.is_err());
        }
        for count in [0, 123] {
            let server = MockServer::start().await;
            let field = if scoped {
                "list_cards_count"
            } else {
                "board_cards_count"
            };
            let route = if scoped {
                "/api/boards/b%2F1/lists/l%2F1/cards_count"
            } else {
                "/api/boards/b%2F1/cards_count"
            };
            Mock::given(method("GET"))
                .and(path(route))
                .and(header("authorization", "Bearer user-token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({field:count})))
                .expect(1)
                .mount(&server)
                .await;
            let result = if scoped {
                client(&server)
                    .list_card_count("b/1", "l/1", &user_token())
                    .await
            } else {
                client(&server).board_card_count("b/1", &user_token()).await
            };
            assert_eq!(result.unwrap(), count);
        }
    }
}

#[tokio::test]
async fn remaining_reads_map_swimlane_projection_and_decode_complete_cards_strictly() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/boards/b%2F1/swimlanes/s%2F1/cards"))
        .and(header("authorization", "Bearer user-token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                json!([{"_id":"c","listId":"l","assignees":[null,"u"],"title":null}]),
            ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let cards = client(&server)
        .swimlane_cards("b/1", "s/1", &user_token())
        .await
        .unwrap();
    assert_eq!(cards[0].list_id.as_deref(), Some("l"));
    assert_eq!(
        cards[0].assignees.as_ref().unwrap(),
        &vec![None, Some("u".into())]
    );
    let mut card = super::responses::complete_card_response();
    card["archived"] = json!(true);
    Mock::given(method("GET"))
        .and(path("/api/cards/c%2F1"))
        .and(header("authorization", "Bearer user-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(card.clone()))
        .expect(1)
        .mount(&server)
        .await;
    assert!(
        client(&server)
            .card_by_id("c/1", &user_token())
            .await
            .unwrap()
            .archived
    );
    for body in [json!({"future":true}), {
        let mut c = card.clone();
        c["vote"]["future"] = json!(true);
        c
    }] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            client(&server).card_by_id("c", &user_token()).await,
            Err(ClientError::Protocol { .. })
        ));
    }
    for body in [
        json!([{"_id":"c","future":true}]),
        json!([{"_id":17}]),
        json!([{}]),
        json!([{"_id":"c","assignees":[{}]}]),
        Value::Null,
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            client(&server)
                .swimlane_cards("b", "s", &user_token())
                .await,
            Err(ClientError::Protocol { .. })
        ));
    }
}
