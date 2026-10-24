use secrecy::SecretString;
use serde_json::{Value, json};
use wekan_cli::client::cards::bot_runs::BotRunUpdate;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

#[test]
fn bot_reports_preserve_omission_null_and_present_values() {
    for input in [
        json!({}),
        json!({"status":"awaiting_input","options":[]}),
        json!({"text":null,"tool":null,"question":null,"tokens":null,"costUsd":null}),
        json!({"text":"Progress","tool":"build","question":"Continue?","tokens":0,"costUsd":0.25}),
        json!({"options":[{"value":"yes"},{"value":"no","label":null},{"value":"later","label":"Later"}]}),
    ] {
        let update: BotRunUpdate = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(update).unwrap(), input);
    }
}

#[test]
fn bot_reports_reject_null_nonnullable_fields_and_unmapped_properties() {
    for input in [
        json!({"status":null}),
        json!({"options":null}),
        json!({"status":"unknown"}),
        json!({"text":true}),
        json!({"tool":[]}),
        json!({"question":{}}),
        json!({"tokens":"17"}),
        json!({"costUsd":false}),
        json!({"options":[{"value":null}]}),
        json!({"options":[{"value":"yes","label":false}]}),
        json!({"options":[{"value":"yes","unmapped":true}]}),
        json!({"unmapped":true}),
    ] {
        assert!(
            serde_json::from_value::<BotRunUpdate>(input.clone()).is_err(),
            "accepted invalid report: {input}"
        );
    }
}

#[tokio::test]
async fn bot_report_requests_forward_explicit_nulls_without_emulating_server_coercions() {
    let server = MockServer::start().await;
    let client = super::client_from_url(&format!("{}/wekan/", server.uri()));
    let input = json!({
        "text":null,"tool":null,"question":null,"tokens":null,"costUsd":null,
        "options":[{"value":"yes","label":null},{"value":"no"}]
    });
    Mock::given(method("POST"))
        .and(path("/wekan/api/boards/board/cards/card/bot-runs/run"))
        .and(header("authorization", "Bearer bot-token"))
        .and(header("content-type", "application/json"))
        .and(body_json(input.clone()))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"_id":"run","status":"awaiting_input"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let update: BotRunUpdate = serde_json::from_value(input.clone()).unwrap();
    client
        .report_bot_run(
            "board",
            "card",
            "run",
            &update,
            &SecretString::from("bot-token"),
        )
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&requests[0].body).unwrap(),
        input
    );
}
