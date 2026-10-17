use super::client;
use secrecy::SecretString;
use serde_json::{Value, json};
use wekan_cli::client::{
    ClientError, CreateIntegrationRequest, IntegrationActivitiesRequest, UpdateIntegrationRequest,
    WekanClient,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

fn document() -> Value {
    json!({"_id":"i", "enabled":true, "type":"outgoing-webhooks", "activities":["all"], "url":"https://8.8.8.8/hook", "boardId":"b", "userId":"u", "createdAt":"2026-10-14T00:00:00.000Z", "modifiedAt":"2026-10-14T00:00:00.000Z"})
}

fn operation(index: usize) -> (&'static str, &'static str, Value) {
    match index {
        0 => ("GET", "/api/boards/b/integrations", json!([document()])),
        1 => ("GET", "/api/boards/b/integrations/i", document()),
        2 => ("POST", "/api/boards/b/integrations", json!({"_id":"i"})),
        3 => ("PUT", "/api/boards/b/integrations/i", json!({"_id":"i"})),
        4 => ("DELETE", "/api/boards/b/integrations/i", json!({"_id":"i"})),
        5 => (
            "POST",
            "/api/boards/b/integrations/i/activities",
            json!({"_id":"i", "activities":["all","a"]}),
        ),
        6 => (
            "DELETE",
            "/api/boards/b/integrations/i/activities",
            json!({"_id":"i", "activities":["all"]}),
        ),
        _ => unreachable!(),
    }
}

async fn invoke(client: &WekanClient, index: usize) -> Result<(), ClientError> {
    let token = SecretString::from("integration-token".to_owned());
    let activities = IntegrationActivitiesRequest {
        activities: vec!["a".into(), "a".into()],
    };
    match index {
        0 => {
            client.board_integrations("b", &token).await?;
        }
        1 => {
            client.integration("b", "i", &token).await?;
        }
        2 => {
            client
                .create_integration(
                    "b",
                    &CreateIntegrationRequest {
                        url: "https://8.8.8.8/hook".into(),
                    },
                    &token,
                )
                .await?;
        }
        3 => {
            client
                .update_integration("b", "i", &UpdateIntegrationRequest::default(), &token)
                .await?;
        }
        4 => {
            client.delete_integration("b", "i", &token).await?;
        }
        5 => {
            client
                .add_integration_activities("b", "i", &activities, &token)
                .await?;
        }
        6 => {
            client
                .remove_integration_activities("b", "i", &activities, &token)
                .await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

#[tokio::test]
async fn all_seven_operations_use_exact_http_contracts() {
    for index in 0..7 {
        let server = MockServer::start().await;
        let (verb, url, response) = operation(index);
        let mut mock = Mock::given(method(verb))
            .and(path(url))
            .and(header("authorization", "Bearer integration-token"))
            .and(header("accept", "application/json"));
        let body = match index {
            2 => Some(json!({"url":"https://8.8.8.8/hook"})),
            3 => Some(json!({})),
            5 | 6 => Some(json!({"activities":["a","a"]})),
            _ => None,
        };
        if let Some(body) = body {
            mock = mock
                .and(header("content-type", "application/json"))
                .and(body_json(body));
        }
        mock.respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        invoke(&client(&server), index).await.unwrap();
        server.verify().await;
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn response_objects_reject_unknown_fields_including_collection_entries() {
    for index in 0..7 {
        let server = MockServer::start().await;
        let (verb, url, mut response) = operation(index);
        let object = if index == 0 {
            &mut response[0]
        } else {
            &mut response
        };
        object["unmapped"] = json!({"token":"never-print"});
        Mock::given(method(verb))
            .and(path(url))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;
        let error = invoke(&client(&server), index).await.unwrap_err();
        assert!(matches!(error, ClientError::Protocol { .. }));
        assert!(!format!("{error:?}").contains("never-print"));
    }
}

#[tokio::test]
async fn documents_reject_missing_required_fields_and_wrong_known_types() {
    for field in [
        "_id",
        "enabled",
        "type",
        "activities",
        "url",
        "boardId",
        "userId",
        "createdAt",
        "modifiedAt",
    ] {
        let server = MockServer::start().await;
        for invalid in [Value::Null, json!({"invalid":true})] {
            server.reset().await;
            let mut response = document();
            response[field] = invalid;
            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(200).set_body_json(response))
                .mount(&server)
                .await;
            assert!(invoke(&client(&server), 1).await.is_err(), "{field}");
        }
        server.reset().await;
        let mut response = document();
        response.as_object_mut().unwrap().remove(field);
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;
        assert!(invoke(&client(&server), 1).await.is_err(), "{field}");
    }
    for (field, value) in [
        ("title", json!(false)),
        ("activities", json!([{"unmapped":true}])),
        ("createdAt", json!("not-a-date")),
        ("modifiedAt", json!("2026-02-30T00:00:00Z")),
        ("boardId", json!("other")),
        ("_id", json!("other")),
        ("userId", json!(" ")),
        ("token", json!("secret")),
    ] {
        let server = MockServer::start().await;
        let mut response = document();
        response[field] = value;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;
        assert!(invoke(&client(&server), 1).await.is_err(), "{field}");
    }
}

#[tokio::test]
async fn optional_title_and_dates_are_preserved() {
    let server = MockServer::start().await;
    let mut response = document();
    response["title"] = json!(" Hook ");
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;
    let document = client(&server)
        .integration("b", "i", &SecretString::from("token".to_owned()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(document.title.as_deref(), Some(" Hook "));
    assert_eq!(document.created_at, "2026-10-14T00:00:00.000Z");
    server.reset().await;
    let mut response = self::document();
    response["title"] = Value::Null;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;
    assert_eq!(
        client(&server)
            .integration("b", "i", &SecretString::from("token".to_owned()))
            .await
            .unwrap()
            .unwrap()
            .title,
        None
    );
}

#[tokio::test]
async fn acknowledgements_and_activity_projections_validate_ids_and_required_fields() {
    for index in 2..7 {
        for invalid in [
            json!({}),
            json!({"_id":null}),
            json!({"_id":""}),
            json!({"_id":"other"}),
            json!({"_id":"i", "activities":null}),
        ] {
            if index == 2 && invalid == json!({"_id":"other"}) {
                continue;
            }
            let server = MockServer::start().await;
            let (verb, _, _) = operation(index);
            Mock::given(method(verb))
                .respond_with(ResponseTemplate::new(200).set_body_json(invalid))
                .mount(&server)
                .await;
            assert!(
                invoke(&client(&server), index).await.is_err(),
                "operation {index}"
            );
        }
    }
}

#[tokio::test]
async fn only_documented_missing_resource_responses_allow_empty_bodies() {
    for index in 0..7 {
        let server = MockServer::start().await;
        let (verb, _, _) = operation(index);
        Mock::given(method(verb))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        assert_eq!(
            invoke(&client(&server), index).await.is_ok(),
            matches!(index, 1 | 5 | 6)
        );
        server.reset().await;
        Mock::given(method(verb))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    json!({"statusCode":403, "error":"Forbidden", "reason":"denied"}),
                ),
            )
            .mount(&server)
            .await;
        assert!(matches!(
            invoke(&client(&server), index).await,
            Err(ClientError::EmbeddedServer {
                wekan_status_code: 403,
                ..
            })
        ));
    }
}

#[tokio::test]
async fn endpoint_ids_are_encoded_and_invalid_ids_send_no_requests() {
    let server = MockServer::start().await;
    let token = SecretString::from("token".to_owned());
    Mock::given(method("DELETE"))
        .and(path("/api/boards/b%2F%20%3F%23/integrations/i%2F%3F%23"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"i/?#"})))
        .expect(1)
        .mount(&server)
        .await;
    client(&server)
        .delete_integration("b/ ?#", "i/?#", &token)
        .await
        .unwrap();
    server.verify().await;
    server.reset().await;
    for id in ["", " ", ".", ".."] {
        assert!(client(&server).integration("b", id, &token).await.is_err());
        assert!(
            client(&server)
                .board_integrations(id, &token)
                .await
                .is_err()
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}
