use super::client;
use secrecy::SecretString;
use serde_json::{Value, json};
use wekan_cli::client::{
    ClientError, CreateRuleRequest, RuleAction, RuleTrigger, UpdateRuleRequest, WekanClient,
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

fn document() -> Value {
    json!({"_id":"r","title":"Rule","trigger":{"activityType":"moveCard","oldListName":"Doing","listName":"*","userId":"*","cardTitle":"*","swimlaneName":"*"},"action":{"actionType":"removeMember","username":"*"}})
}
fn operation(i: usize) -> (&'static str, &'static str, Value) {
    match i {
        0 => ("GET", "/api/boards/b/rules", json!([document()])),
        1 => ("GET", "/api/boards/b/rules/r", document()),
        2 => (
            "POST",
            "/api/boards/b/rules",
            json!({"_id":"r","triggerId":"t","actionId":"a"}),
        ),
        3 => ("PUT", "/api/boards/b/rules/r", json!({"_id":"r"})),
        4 => ("DELETE", "/api/boards/b/rules/r", json!({"_id":"r"})),
        _ => unreachable!(),
    }
}
async fn invoke(c: &WekanClient, i: usize) -> Result<(), ClientError> {
    let token = SecretString::from("rule-token".to_owned());
    match i {
        0 => {
            c.board_rules("b", &token).await?;
        }
        1 => {
            c.rule("b", "r", &token).await?;
        }
        2 => {
            c.create_rule("b",&serde_json::from_value::<CreateRuleRequest>(json!({"trigger":{"activityType":"moveCard","oldListName":null},"action":{"actionType":"removeMember","username":"*"}})).unwrap(),&token).await?;
        }
        3 => {
            c.update_rule("b", "r", &UpdateRuleRequest::default(), &token)
                .await?;
        }
        4 => {
            c.delete_rule("b", "r", &token).await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}
#[tokio::test]
async fn five_native_operations_have_exact_contracts_and_no_extra_requests() {
    for i in 0..5 {
        let s = MockServer::start().await;
        let (verb, url, response) = operation(i);
        let mut m = Mock::given(method(verb))
            .and(path(url))
            .and(header("accept", "application/json"))
            .and(header("authorization", "Bearer rule-token"));
        if i == 2 {
            m=m.and(header("content-type","application/json")).and(body_json(json!({"trigger":{"activityType":"moveCard","oldListName":null},"action":{"actionType":"removeMember","username":"*"}})));
        }
        if i == 3 {
            m = m.and(body_json(json!({})));
        }
        m.respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&s)
            .await;
        invoke(&client(&s), i).await.unwrap();
        s.verify().await;
        assert_eq!(s.received_requests().await.unwrap().len(), 1);
    }
}
#[tokio::test]
async fn large_rule_payloads_remain_readable_in_details_and_collections() {
    let server = MockServer::start().await;
    let description = "x".repeat(2 * 1024 * 1024);
    let document = json!({"_id":"r", "title":"Large rule", "trigger":null,
        "action":{"actionType":"archive", "desc":description}});
    for (path_value, response) in [
        ("/api/boards/b/rules/r", document.clone()),
        ("/api/boards/b/rules", json!([document])),
    ] {
        Mock::given(method("GET"))
            .and(path(path_value))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
    }
    let c = client(&server);
    let token = SecretString::from("t".to_owned());
    let detail = c.rule("b", "r", &token).await.unwrap();
    let collection = c.board_rules("b", &token).await.unwrap();
    assert_eq!(collection, vec![detail.clone()]);
    assert_eq!(detail.action.unwrap().desc, Some(Some(description)));
    server.verify().await;
}

#[tokio::test]
async fn rule_read_limits_stay_bounded_and_do_not_expand_errors_or_acknowledgements() {
    for i in 0..2 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = std::thread::spawn(move || {
            use std::io::{Read, Write};
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 8192];
            assert!(socket.read(&mut request).unwrap() > 0);
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 67108865\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
        });
        let result = invoke(&super::client_from_url(&url), i).await;
        task.join().unwrap();
        assert!(matches!(
            result,
            Err(ClientError::ResponseTooLarge {
                limit_bytes: 67_108_864,
                ..
            })
        ));
    }
    let server = MockServer::start().await;
    for (i, status) in [(0, 500), (1, 500), (2, 200), (3, 200), (4, 200)] {
        server.reset().await;
        Mock::given(method(operation(i).0))
            .respond_with(ResponseTemplate::new(status).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
            .mount(&server)
            .await;
        assert!(matches!(
            invoke(&client(&server), i).await,
            Err(ClientError::ResponseTooLarge {
                limit_bytes: 1_048_576,
                ..
            })
        ));
    }
}

#[test]
fn request_timestamps_preserve_valid_text_null_and_omission() {
    let mut triggers = vec![
        json!({"activityType":"button"}),
        json!({"activityType":"button", "lastRunAt":null}),
    ];
    for date in [
        "2030-01-01T00:00:00Z",
        "2030-01-01T03:00:00+03:00",
        "+010000-01-01T00:00:00.000Z",
        "-000001-01-01T00:00:00.000Z",
        "+275760-09-13T00:00:00.000Z",
        "-271821-04-20T00:00:00.000Z",
    ] {
        triggers.push(json!({"activityType":"button", "lastRunAt":date}));
    }
    for trigger in triggers {
        let value = json!({"trigger":trigger, "action":{"actionType":"archive"}});
        let create: CreateRuleRequest = serde_json::from_value(value.clone()).unwrap();
        let update: UpdateRuleRequest = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(create).unwrap(), value);
        assert_eq!(serde_json::to_value(update).unwrap(), value);
    }
}

#[tokio::test]
async fn strict_responses_reject_extra_fields_missing_fields_invalid_types_and_ids() {
    for i in 0..5 {
        let (_, _, base) = operation(i);
        let mut bad = Vec::new();
        let mut extra = base.clone();
        let object = if i == 0 { &mut extra[0] } else { &mut extra };
        object["unmapped"] = json!({"secret":"never print"});
        bad.push(extra);
        for field in if i < 2 {
            vec!["_id", "title", "trigger", "action"]
        } else if i == 2 {
            vec!["_id", "triggerId", "actionId"]
        } else {
            vec!["_id"]
        } {
            let mut missing = base.clone();
            (if i == 0 {
                &mut missing[0]
            } else {
                &mut missing
            })
            .as_object_mut()
            .unwrap()
            .remove(field);
            bad.push(missing);
        }
        let mut empty = base.clone();
        (if i == 0 { &mut empty[0] } else { &mut empty })["_id"] = json!("");
        bad.push(empty);
        if i != 0 && i != 2 {
            let mut mismatch = base.clone();
            mismatch["_id"] = json!("different");
            bad.push(mismatch);
        }
        if i < 2 {
            for (field, value) in [
                (
                    "trigger",
                    json!({"activityType":"moveCard","unmapped":true}),
                ),
                (
                    "action",
                    json!({"actionType":"setColor","selectedColor":{"unmapped":1}}),
                ),
                ("trigger", json!([])),
                ("action", json!([])),
                ("title", json!(null)),
                ("trigger", json!({"activityType":12})),
                ("trigger", json!({"lastRunAt":"not-a-date"})),
                ("action", json!({"days":true})),
            ] {
                let mut d = base.clone();
                (if i == 0 { &mut d[0] } else { &mut d })[field] = value;
                bad.push(d);
            }
        }
        bad.push(if i == 0 { json!([[]]) } else { json!([]) });
        for response in bad {
            let s = MockServer::start().await;
            let (verb, url, _) = operation(i);
            Mock::given(method(verb))
                .and(path(url))
                .respond_with(ResponseTemplate::new(200).set_body_json(response))
                .mount(&s)
                .await;
            assert!(matches!(
                invoke(&client(&s), i).await,
                Err(ClientError::Protocol { .. })
            ));
        }
    }
}
#[tokio::test]
async fn nullable_references_sparse_documents_and_runtime_metadata_are_mapped() {
    let s = MockServer::start().await;
    for (trigger, action) in [
        (Value::Null, Value::Null),
        (json!({}), json!({})),
        (
            json!({"activityType":"scheduledTrigger","days":"2","lastRunKey":"2026-10-15 01:02","lastRunAt":"2026-10-15T01:02:00.000Z","desc":null}),
            json!({"actionType":"setDateRelative","days":-2,"unit":"hours"}),
        ),
    ] {
        s.reset().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"_id":"r","title":"","trigger":trigger,"action":action})),
            )
            .mount(&s)
            .await;
        let d = client(&s)
            .rule("b", "r", &SecretString::from("t".to_owned()))
            .await
            .unwrap();
        assert_eq!(
            d.trigger
                .map(|v| serde_json::to_value(v).unwrap())
                .unwrap_or(Value::Null),
            trigger
        );
        assert_eq!(
            d.action
                .map(|v| serde_json::to_value(v).unwrap())
                .unwrap_or(Value::Null),
            action
        );
    }
}
#[test]
fn typed_requests_preserve_omission_nulls_and_fields_from_different_kinds() {
    let value = json!({"title":"","trigger":{"activityType":"moveCard","oldListName":null,"buttonLabel":"old","lastRunAt":null,"boardId":"ignored","_id":"ignored"},"action":{"actionType":"archive","username":"old","days":"2"}});
    let request: UpdateRuleRequest = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), value);
    for value in [
        json!({"trigger":[]}),
        json!({"action":null}),
        json!({"title":null}),
        json!({"trigger":{"extra":1}}),
        json!({"action":{"days":{}}}),
    ] {
        assert!(serde_json::from_value::<UpdateRuleRequest>(value).is_err());
    }
    // Every payload is sparse, including persisted fields left by previous kinds.
    assert!(
        serde_json::from_value::<RuleTrigger>(
            json!({"activityType":"button","oldListName":"old","days":1})
        )
        .is_ok()
    );
    assert!(
        serde_json::from_value::<RuleAction>(json!({"actionType":"archive","username":"old"}))
            .is_ok()
    );
}
#[tokio::test]
async fn scopes_are_encoded_as_individual_segments_and_missing_rules_keep_http_500() {
    let s = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/boards/board%2Fid/rules/rule%2Fid"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({"error":"not-found","reason":"Rule not found","message":"Rule not found [not-found]","errorType":"Meteor.Error","isClientSafe":true}))).expect(1).mount(&s).await;
    assert!(
        matches!(client(&s).rule("board/id","rule/id",&SecretString::from("t".to_owned())).await,
        Err(ClientError::Server {status,server_error:Some(code),..}) if status.as_u16()==500 && code=="not-found")
    );
    s.verify().await;
}

#[test]
fn every_modeled_payload_field_preserves_types_nullability_and_omission() {
    for field in [
        "activityType",
        "userId",
        "username",
        "listName",
        "oldListName",
        "swimlaneName",
        "cardTitle",
        "checklistName",
        "checklistItemName",
        "labelId",
        "attachmentName",
        "desc",
        "scheduleKind",
        "scheduleType",
        "atTime",
        "onDate",
        "dueCondition",
        "lastRunKey",
        "lastRunAt",
        "buttonType",
        "buttonLabel",
    ] {
        for value in [json!("value"), Value::Null] {
            let payload = json!({field:value});
            let d: RuleTrigger = serde_json::from_value(payload.clone()).unwrap();
            assert_eq!(serde_json::to_value(d).unwrap(), payload);
        }
        for value in [json!(1), json!(true), json!([]), json!({"extra":1})] {
            assert!(
                serde_json::from_value::<RuleTrigger>(json!({field:value})).is_err(),
                "{field}"
            );
        }
    }
    for field in ["weekday", "dayOfMonth", "days"] {
        for value in [json!(2), json!(-0.5), json!("2"), Value::Null] {
            let payload = json!({field:value});
            let d: RuleTrigger = serde_json::from_value(payload.clone()).unwrap();
            assert_eq!(serde_json::to_value(d).unwrap(), payload);
        }
        for value in [json!(false), json!([]), json!({"extra":1})] {
            assert!(serde_json::from_value::<RuleTrigger>(json!({field:value})).is_err());
        }
    }
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<RuleTrigger>(json!({})).unwrap()).unwrap(),
        json!({})
    );
    for field in [
        "actionType",
        "listName",
        "swimlaneName",
        "cardName",
        "fromListName",
        "checklistName",
        "checkItemName",
        "checklistItems",
        "dateField",
        "unit",
        "emailTo",
        "emailSubject",
        "emailMsg",
        "selectedColor",
        "labelId",
        "username",
        "sortField",
        "desc",
    ] {
        for value in [json!("value"), Value::Null] {
            let payload = json!({field:value});
            let d: RuleAction = serde_json::from_value(payload.clone()).unwrap();
            assert_eq!(serde_json::to_value(d).unwrap(), payload);
        }
        for value in [json!(1), json!(true), json!([]), json!({"extra":1})] {
            assert!(
                serde_json::from_value::<RuleAction>(json!({field:value})).is_err(),
                "{field}"
            );
        }
    }
    for field in ["days"] {
        for value in [json!(2), json!(-0.5), json!("2"), Value::Null] {
            let payload = json!({field:value});
            let d: RuleAction = serde_json::from_value(payload.clone()).unwrap();
            assert_eq!(serde_json::to_value(d).unwrap(), payload);
        }
        for value in [json!(false), json!([]), json!({"extra":1})] {
            assert!(serde_json::from_value::<RuleAction>(json!({field:value})).is_err());
        }
    }
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<RuleAction>(json!({})).unwrap()).unwrap(),
        json!({})
    );
}
