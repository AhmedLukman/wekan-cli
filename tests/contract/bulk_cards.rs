use super::*;
use serde_json::json;
use wekan_cli::client::{
    ClientError,
    boards::labels::AddBoardLabelRequest,
    cards::bulk::{BulkCreateCardsRequest, BulkLabelsError, BulkLabelsRequest},
};
use wiremock::{
    Mock, ResponseTemplate,
    matchers::{body_json, method, path},
};

#[tokio::test]
async fn native_bulk_requests_preserve_input_shapes_and_order() {
    let server = MockServer::start().await;
    let cards = json!([{"title":null,"description":"","parentId":"parent","members":["u",null,7],"assignees":null}, {"swimlaneId":"override"}]);
    Mock::given(method("POST"))
        .and(path("/api/boards/b/lists/l/cards/bulk"))
        .and(body_json(json!({"swimlaneId":"s","cards":cards})))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                json!([{"index":0,"_id":"c"},{"index":1,"error":"authorId not found"}]),
            ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let result = client(&server)
        .bulk_create_cards(
            "b",
            "l",
            &BulkCreateCardsRequest {
                swimlane_id: Some("s".into()),
                cards: serde_json::from_value(cards).unwrap(),
            },
            &user_token(),
        )
        .await
        .unwrap();
    assert_eq!(result[1].error.as_deref(), Some("authorId not found"));
    Mock::given(method("DELETE"))
        .and(path("/api/boards/b/cards/bulk"))
        .and(body_json(json!({"cardIds":["c","c","missing"]})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"deleted":["c"],"notFound":["c","missing"]})),
        )
        .expect(1)
        .mount(&server)
        .await;
    client(&server)
        .bulk_delete_cards(
            "b",
            &["c".into(), "c".into(), "missing".into()],
            &user_token(),
        )
        .await
        .unwrap();
    Mock::given(method("POST")).and(path("/api/boards/b/cards/labels"))
        .and(body_json(json!({"cardIds":["c","c"],"addLabelIds":["a"],"removeLabelIds":["a","z"]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"updated":[{"_id":"c","labelIds":["a"]},{"_id":"c","labelIds":["a"]}],"notFound":[]}))).expect(1).mount(&server).await;
    client(&server)
        .bulk_card_labels(
            "b",
            &BulkLabelsRequest {
                card_ids: vec!["c".into(), "c".into()],
                add_label_ids: vec!["a".into()],
                remove_label_ids: vec!["a".into(), "z".into()],
            },
            &user_token(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn bulk_responses_reject_unknown_fields_invalid_types_and_bad_correlation() {
    let server = MockServer::start().await;
    let c = client(&server);
    let request = BulkCreateCardsRequest {
        swimlane_id: Some("s".into()),
        cards: serde_json::from_value(json!([{}])).unwrap(),
    };
    for response in [
        json!([{"index":0,"_id":"c","future":1}]),
        json!([{"index":0,"_id":null}]),
        json!([{"index":0}]),
        json!([{"index":0,"_id":"c","error":"bad"}]),
        json!([{"index":1,"_id":"c"}]),
        json!([]),
        json!([{"index":"0","_id":"c"}]),
    ] {
        server.reset().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;
        assert!(matches!(
            c.bulk_create_cards("b", "l", &request, &user_token()).await,
            Err(ClientError::Protocol { .. })
        ));
    }
    for response in [
        json!({"deleted":["c"],"notFound":[],"future":1}),
        json!({"deleted":["other"],"notFound":[]}),
        json!({"deleted":["c"]}),
        json!({"deleted":[1],"notFound":[]}),
    ] {
        server.reset().await;
        Mock::given(method("DELETE"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;
        assert!(matches!(
            c.bulk_delete_cards("b", &["c".into()], &user_token()).await,
            Err(ClientError::Protocol { .. })
        ));
    }
    for response in [
        json!({"updated":[{"_id":"c","labelIds":[],"future":1}],"notFound":[]}),
        json!({"updated":[],"notFound":["c"],"future":1}),
        json!({"updated":[{"_id":"c","labelIds":[7]}],"notFound":[]}),
        json!({"updated":[{"_id":"c","labelIds":[{}]}],"notFound":[]}),
        json!({"updated":[{"_id":"c","labelIds":null}],"notFound":[]}),
        json!({"updated":[{"_id":"c"}],"notFound":[]}),
        json!({"updated":[],"notFound":[]}),
    ] {
        server.reset().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;
        assert!(matches!(
            c.bulk_card_labels(
                "b",
                &BulkLabelsRequest {
                    card_ids: vec!["c".into()],
                    add_label_ids: vec!["a".into()],
                    remove_label_ids: vec![]
                },
                &user_token()
            )
            .await,
            Err(BulkLabelsError::Client(ClientError::Protocol { .. }))
        ));
    }
}

#[tokio::test]
async fn bulk_label_responses_preserve_nullable_elements_and_order() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/boards/b/cards/labels"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "updated":[{"_id":"c","labelIds":[null,"a"]}],"notFound":[]
        })))
        .expect(1)
        .mount(&server)
        .await;
    let response = client(&server)
        .bulk_card_labels(
            "b",
            &BulkLabelsRequest {
                card_ids: vec!["c".into()],
                add_label_ids: vec!["a".into()],
                remove_label_ids: vec![],
            },
            &user_token(),
        )
        .await
        .unwrap();
    assert_eq!(response.updated[0].label_ids, [None, Some("a".into())]);
}

#[tokio::test]
async fn invalid_label_error_is_strict_and_endpoint_specific() {
    let server = MockServer::start().await;
    let c = client(&server);
    let request = BulkLabelsRequest {
        card_ids: vec!["c".into()],
        add_label_ids: vec!["bad".into()],
        remove_label_ids: vec![],
    };
    for (extra, valid) in [(false, true), (true, false)] {
        server.reset().await;
        let mut response = json!({"error":"invalid labels","invalidLabelIds":["bad"]});
        if extra {
            response["future"] = json!(1);
        }
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(400).set_body_json(response))
            .mount(&server)
            .await;
        let error = c
            .bulk_card_labels("b", &request, &user_token())
            .await
            .unwrap_err();
        if valid {
            assert!(
                matches!(error,BulkLabelsError::InvalidLabels {invalid_label_ids,..} if invalid_label_ids==["bad"])
            );
        } else {
            assert!(matches!(
                error,
                BulkLabelsError::Client(ClientError::Protocol { .. })
            ));
        }
        assert!(matches!(
            c.bulk_create_cards(
                "b",
                "l",
                &BulkCreateCardsRequest {
                    swimlane_id: None,
                    cards: serde_json::from_value(json!([{}])).unwrap()
                },
                &user_token()
            )
            .await,
            Err(ClientError::Protocol { .. })
        ));
    }
}

#[tokio::test]
async fn board_label_add_preserves_text_and_defaults_and_requires_string_acknowledgment() {
    let server = MockServer::start().await;
    for (name, color) in [(None, None), (Some("  Bug\n".into()), Some("red".into()))] {
        server.reset().await;
        let request = AddBoardLabelRequest { name, color };
        Mock::given(method("PUT"))
            .and(path("/api/boards/b/labels"))
            .and(body_json(json!({"label":&request})))
            .respond_with(ResponseTemplate::new(200).set_body_json("label-id"))
            .expect(1)
            .mount(&server)
            .await;
        assert_eq!(
            client(&server)
                .add_board_label("b", &request, &user_token())
                .await
                .unwrap(),
            "label-id"
        );
    }
}
