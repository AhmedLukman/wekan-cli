use super::{client, client_from_url};
use secrecy::SecretString;
use serde_json::{Value, json};
use wekan_cli::client::{ClientError, WekanClient, boards::members::BoardMemberPermissions};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

fn operation(index: usize) -> (&'static str, &'static str, Value) {
    match index {
        0 => (
            "POST",
            "/api/boards/b/members/u/add",
            json!([{"_id":"b", "title":"Board"}]),
        ),
        1 => (
            "POST",
            "/api/boards/b/members/u/remove",
            json!([{"_id":"b", "title":"Board"}]),
        ),
        2 => ("POST", "/api/boards/b/members/u", json!(1)),
        3 => (
            "POST",
            "/api/boards/b/lists/l/cards/c/members/u",
            json!({"_id":"c", "members":["other", "u"], "assignees":["other"]}),
        ),
        4 => (
            "DELETE",
            "/api/boards/b/lists/l/cards/c/members/u",
            json!({"_id":"c", "members":["other"], "assignees":["other"]}),
        ),
        5 => (
            "POST",
            "/api/boards/b/lists/l/cards/c/assignees/u",
            json!({"_id":"c", "members":["other"], "assignees":["other", "u"]}),
        ),
        6 => (
            "DELETE",
            "/api/boards/b/lists/l/cards/c/assignees/u",
            json!({"_id":"c", "members":["other"], "assignees":["other"]}),
        ),
        _ => unreachable!(),
    }
}

async fn invoke(client: &WekanClient, index: usize, user: &str) -> Result<(), ClientError> {
    let token = SecretString::from("membership-token".to_owned());
    let permissions = BoardMemberPermissions {
        role: Some("worker".into()),
        is_admin: Some(false),
        ..Default::default()
    };
    match index {
        0 => {
            client
                .add_board_member("b", user, &permissions, &token)
                .await?;
        }
        1 => {
            client.remove_board_member("b", user, &token).await?;
        }
        2 => {
            client
                .update_board_member("b", user, &permissions, &token)
                .await?;
        }
        3 => {
            client.add_card_member("b", "l", "c", user, &token).await?;
        }
        4 => {
            client
                .remove_card_member("b", "l", "c", user, &token)
                .await?;
        }
        5 => {
            client
                .add_card_assignee("b", "l", "c", user, &token)
                .await?;
        }
        6 => {
            client
                .remove_card_assignee("b", "l", "c", user, &token)
                .await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

#[tokio::test]
async fn membership_requests_match_all_seven_operations_without_extra_http() {
    for index in 0..7 {
        let server = MockServer::start().await;
        let (verb, endpoint, response) = operation(index);
        let mut mock = Mock::given(method(verb))
            .and(path(endpoint))
            .and(header("authorization", "Bearer membership-token"));
        match index {
            0 => {
                mock = mock.and(body_json(
                    json!({"action":"add", "role":"worker", "isAdmin":false}),
                ))
            }
            1 => mock = mock.and(body_json(json!({"action":"remove"}))),
            2 => mock = mock.and(body_json(json!({"role":"worker", "isAdmin":false}))),
            _ => {}
        }
        mock.respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        invoke(&client(&server), index, "u").await.unwrap();
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        if index >= 3 {
            assert!(requests[0].body.is_empty());
        }
    }
    assert_eq!(
        serde_json::to_value(BoardMemberPermissions::default()).unwrap(),
        json!({})
    );
    let permissions = BoardMemberPermissions {
        role: None,
        is_admin: Some(false),
        is_no_comments: Some(false),
        is_comment_only: Some(false),
        is_worker: Some(false),
        is_normal_assigned_only: Some(false),
        is_comment_assigned_only: Some(false),
        is_read_only: Some(false),
        is_read_assigned_only: Some(false),
    };
    assert_eq!(
        serde_json::to_value(permissions).unwrap(),
        json!({"isAdmin":false,"isNoComments":false,"isCommentOnly":false,"isWorker":false,"isNormalAssignedOnly":false,"isCommentAssignedOnly":false,"isReadOnly":false,"isReadAssignedOnly":false})
    );
}

#[tokio::test]
async fn membership_responses_reject_unknown_missing_and_invalid_fields() {
    for index in 0..7 {
        let (verb, endpoint, valid) = operation(index);
        let mut invalid = vec![json!({}), json!(null), json!("bad")];
        if index < 2 {
            invalid.extend([
                json!([{"_id":"b","title":"Board","secret":"hidden"}]),
                json!([{"_id":"b"}]),
                json!([{"_id":"b","title":7}]),
                json!([{"_id":"","title":"Board"}]),
            ]);
        } else if index == 2 {
            invalid.extend([json!(-1), json!(1.5), json!([])]);
        } else {
            for key in ["_id", "members", "assignees"] {
                let mut missing = valid.clone();
                missing.as_object_mut().unwrap().remove(key);
                invalid.push(missing);
                let mut bad = valid.clone();
                bad[key] = json!(17);
                invalid.push(bad);
            }
            for (field, value) in [
                ("unknown", json!({"secret":"hidden"})),
                ("_id", json!("wrong")),
                ("members", json!([{"userId":"u","extra":true}])),
                ("assignees", json!([null])),
                ("members", json!([""])),
            ] {
                let mut bad = valid.clone();
                bad[field] = value;
                invalid.push(bad);
            }
        }
        for response in invalid {
            let server = MockServer::start().await;
            Mock::given(method(verb))
                .and(path(endpoint))
                .respond_with(ResponseTemplate::new(200).set_body_json(response))
                .expect(1)
                .mount(&server)
                .await;
            let error = invoke(&client(&server), index, "u").await.unwrap_err();
            assert!(
                matches!(
                    error,
                    ClientError::Protocol {
                        success_status_received: true,
                        ..
                    }
                ),
                "{error:?}"
            );
            assert!(!error.to_string().contains("hidden"));
        }
    }
}

#[tokio::test]
async fn membership_errors_empty_bodies_and_server_failures_are_preserved_without_retry() {
    for index in 0..7 {
        let (verb, endpoint, _) = operation(index);
        for template in [
            ResponseTemplate::new(200),
            ResponseTemplate::new(200).set_body_json(
                json!({"error":"not-authorized","reason":"Denied","statusCode":403}),
            ),
            ResponseTemplate::new(500).set_body_string("<!DOCTYPE html><p>Failure</p>"),
        ] {
            let server = MockServer::start().await;
            Mock::given(method(verb))
                .and(path(endpoint))
                .respond_with(template)
                .expect(1)
                .mount(&server)
                .await;
            assert!(invoke(&client(&server), index, "u").await.is_err());
            assert_eq!(server.received_requests().await.unwrap().len(), 1);
        }
    }
}

#[tokio::test]
async fn membership_paths_escape_user_ids_and_preserve_subpath_servers() {
    for index in 0..7 {
        let server = MockServer::start().await;
        let (_, _, response) = operation(index);
        Mock::given(header("authorization", "Bearer membership-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        let client = client_from_url(&format!("{}/wekan/", server.uri()));
        invoke(&client, index, "u/a?#%雪").await.unwrap();
        let requests = server.received_requests().await.unwrap();
        let endpoint = requests[0].url.path();
        assert!(endpoint.starts_with("/wekan/api/boards/b/"));
        assert!(endpoint.contains("u%2Fa%3F%23%25%E9%9B%AA"), "{endpoint}");
        assert!(requests[0].url.query().is_none());
        assert!(requests[0].url.fragment().is_none());
    }
}

#[tokio::test]
async fn membership_clients_reject_unrepresentable_identifiers_before_http() {
    let server = MockServer::start().await;
    for index in 0..7 {
        for id in ["", " ", ".", ".."] {
            assert!(invoke(&client(&server), index, id).await.is_err());
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}
