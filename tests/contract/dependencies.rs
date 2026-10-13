use super::{client, client_from_url};
use secrecy::SecretString;
use serde_json::{Value, json};
use wekan_cli::client::{
    ClientError, WekanClient,
    dependencies::{CreateDependencyRequest, DependencyMetadata},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

fn entry() -> Value {
    json!({"cardId":"t", "type":"blocks", "color":"red", "icon":"lock"})
}
fn line() -> Value {
    json!({"from":"c", "to":"t", "type":"blocks", "color":"red", "icon":"lock", "fromTitle":null, "toTitle":"Target", "fromCardNumber":9007199254740993_u64, "toCardNumber":2.5})
}
fn created() -> Value {
    let mut entry = entry();
    entry["_id"] = json!("c");
    entry
}

async fn invoke(client: &WekanClient, operation: usize) -> Result<(), ClientError> {
    let token = SecretString::from("dependency-token".to_owned());
    match operation {
        0 => {
            client.card_dependencies("b", "c", &token).await?;
        }
        1 => {
            client.board_dependencies("b", &token).await?;
        }
        2 => {
            client
                .create_dependency(
                    "b",
                    "c",
                    &CreateDependencyRequest {
                        target_card_id: "t".into(),
                        metadata: DependencyMetadata::default(),
                    },
                    &token,
                )
                .await?;
        }
        3 => {
            client
                .update_dependency("b", "c", "t", &DependencyMetadata::default(), &token)
                .await?;
        }
        4 => {
            client.delete_dependency("b", "c", "t", &token).await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn operation(operation: usize) -> (&'static str, &'static str, Value) {
    match operation {
        0 => (
            "GET",
            "/api/boards/b/cards/c/dependencies",
            json!([entry()]),
        ),
        1 => ("GET", "/api/boards/b/dependencies", json!([line()])),
        2 => ("POST", "/api/boards/b/cards/c/dependencies", created()),
        3 => (
            "PUT",
            "/api/boards/b/cards/c/dependencies/t",
            json!({"_id":"c"}),
        ),
        4 => (
            "DELETE",
            "/api/boards/b/cards/c/dependencies/t",
            json!({"_id":"c"}),
        ),
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn all_five_dependency_operations_use_exact_http_contracts() {
    for index in 0..5 {
        let server = MockServer::start().await;
        let (verb, url, response) = operation(index);
        let mut mock = Mock::given(method(verb))
            .and(path(url))
            .and(header("authorization", "Bearer dependency-token"))
            .and(header("accept", "application/json"));
        if index == 2 {
            mock = mock
                .and(header("content-type", "application/json"))
                .and(body_json(json!({"cardId":"t"})));
        }
        if index == 3 {
            mock = mock.and(body_json(json!({})));
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
async fn dependency_metadata_preserves_omission_null_and_empty_strings() {
    for metadata in [
        DependencyMetadata::default(),
        DependencyMetadata {
            dependency_type: Some(None),
            color: Some(None),
            icon: Some(None),
        },
        DependencyMetadata {
            dependency_type: Some(Some("".into())),
            color: Some(Some("".into())),
            icon: Some(Some("  link  ".into())),
        },
    ] {
        let server = MockServer::start().await;
        let expected = serde_json::to_value(&metadata).unwrap();
        Mock::given(method("PUT"))
            .and(body_json(expected))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"c"})))
            .expect(1)
            .mount(&server)
            .await;
        client(&server)
            .update_dependency(
                "b",
                "c",
                "t",
                &metadata,
                &SecretString::from("token".to_owned()),
            )
            .await
            .unwrap();
        server.verify().await;
    }
}

#[tokio::test]
async fn dependency_responses_reject_unknown_missing_and_invalid_fields() {
    for index in 0..5 {
        let (_, _, original) = operation(index);
        let object = if index < 2 {
            original[0].clone()
        } else {
            original.clone()
        };
        let required: &[&str] = match index {
            0 => &["cardId", "type", "color", "icon"],
            1 => &["from", "to", "type", "color", "icon"],
            2 => &["_id", "cardId", "type", "color", "icon"],
            _ => &["_id"],
        };
        let mut cases = Vec::new();
        let mut unknown = object.clone();
        unknown["unmapped"] = json!({"nested":true});
        cases.push(unknown);
        for field in required {
            let mut missing = object.clone();
            missing.as_object_mut().unwrap().remove(*field);
            cases.push(missing);
            for invalid in [json!(null), json!(3), json!({"unexpected":true})] {
                let mut value = object.clone();
                value[*field] = invalid;
                cases.push(value);
            }
        }
        if index < 3 {
            for relation in ["future", ""] {
                let mut value = object.clone();
                value["type"] = json!(relation);
                cases.push(value);
            }
        }
        if index == 1 {
            for field in ["fromTitle", "toTitle", "fromCardNumber", "toCardNumber"] {
                let mut value = object.clone();
                value[field] = json!(true);
                cases.push(value);
            }
        }
        for invalid in cases {
            let server = MockServer::start().await;
            let (verb, url, _) = operation(index);
            let response = if index < 2 { json!([invalid]) } else { invalid };
            Mock::given(method(verb))
                .and(path(url))
                .respond_with(ResponseTemplate::new(200).set_body_json(response))
                .expect(1)
                .mount(&server)
                .await;
            assert!(matches!(
                invoke(&client(&server), index).await.unwrap_err(),
                ClientError::Protocol {
                    success_status_received: true,
                    ..
                }
            ));
        }
    }
}

#[tokio::test]
async fn dependency_acknowledgments_reject_mismatched_ids() {
    for index in 2..5 {
        for field in if index == 2 {
            vec!["_id", "cardId"]
        } else {
            vec!["_id"]
        } {
            let server = MockServer::start().await;
            let (verb, url, mut response) = operation(index);
            response[field] = json!("different");
            Mock::given(method(verb))
                .and(path(url))
                .respond_with(ResponseTemplate::new(200).set_body_json(response))
                .mount(&server)
                .await;
            assert!(matches!(
                invoke(&client(&server), index).await.unwrap_err(),
                ClientError::Protocol {
                    success_status_received: true,
                    ..
                }
            ));
        }
    }
}

#[tokio::test]
async fn board_dependency_nullable_details_preserve_numbers_and_omission() {
    let server = MockServer::start().await;
    let mut omitted = line();
    for field in ["fromTitle", "fromCardNumber", "toTitle", "toCardNumber"] {
        omitted.as_object_mut().unwrap().remove(field);
    }
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([line(), omitted])))
        .mount(&server)
        .await;
    let values = client(&server)
        .board_dependencies("b", &SecretString::from("token".to_owned()))
        .await
        .unwrap();
    assert_eq!(
        values[0].source_card_number.as_ref().unwrap().to_string(),
        "9007199254740993"
    );
    assert_eq!(
        values[0].target_card_number.as_ref().unwrap().to_string(),
        "2.5"
    );
    assert_eq!(values[1].source_title, None);
    assert_eq!(values[1].target_card_number, None);
}

#[tokio::test]
async fn dependency_paths_escape_identifiers_and_preserve_base_paths() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path(
            "/base/api/boards/b%2F%23%3F/cards/c%2F%23%3F/dependencies/t%2F%23%3F",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"c/#?"})))
        .expect(1)
        .mount(&server)
        .await;
    client_from_url(&format!("{}/base/", server.uri()))
        .update_dependency(
            "b/#?",
            "c/#?",
            "t/#?",
            &DependencyMetadata::default(),
            &SecretString::from("token".to_owned()),
        )
        .await
        .unwrap();
    server.verify().await;
    for id in ["", ".", ".."] {
        assert!(matches!(
            client(&server)
                .card_dependencies(id, "c", &SecretString::from("token".to_owned()))
                .await
                .unwrap_err(),
            ClientError::Protocol {
                success_status_received: false,
                ..
            }
        ));
    }
}
