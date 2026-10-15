use super::admin_fixtures as fixtures;
use super::{client, status_token};
use serde_json::{Value, json};
use wekan_cli::client::{
    ClientError,
    admin::{
        PreparedSettingsUpdate, attachment_settings::UpdateAttachmentSettings,
        settings::UpdateGlobalSettings,
    },
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

#[tokio::test]
async fn admin_settings_use_exact_native_routes_and_typed_bodies() {
    for (route, body, response) in [
        (
            "/api/settings",
            json!({"disableRegistration":false,"productName":""}),
            fixtures::global(),
        ),
        (
            "/api/admin/attachment-settings",
            json!({"storageConfig":{"s3":{"enabled":false,"secretAccessKey":"new-secret"},"gcs":{"credentials":{"private_key":"key"}}},"limitSettings":{"apiUploadBlocked":false},"uploadSettings":{"allowedMimeTypes":[]}}),
            fixtures::attachment(),
        ),
    ] {
        let server = MockServer::start().await;
        for verb in ["GET", "PUT"] {
            let mut mock = Mock::given(method(verb))
                .and(path(route))
                .and(header("authorization", "Bearer status-token"))
                .and(header("accept", "application/json"));
            if verb == "PUT" {
                mock = mock
                    .and(header("content-type", "application/json"))
                    .and(body_json(&body));
            }
            mock.respond_with(ResponseTemplate::new(200).set_body_json(&response))
                .expect(1)
                .mount(&server)
                .await;
        }
        let c = client(&server);
        if route == "/api/settings" {
            c.global_settings(&status_token()).await.unwrap();
            c.update_global_settings(
                PreparedSettingsUpdate::new(
                    &serde_json::from_value::<UpdateGlobalSettings>(body).unwrap(),
                )
                .unwrap(),
                &status_token(),
            )
            .await
            .unwrap();
        } else {
            c.attachment_settings(&status_token()).await.unwrap();
            let request = serde_json::from_value::<UpdateAttachmentSettings>(body).unwrap();
            assert!(!format!("{request:?}").contains("new-secret"));
            assert!(!format!("{request:?}").contains("private_key"));
            c.update_attachment_settings(
                PreparedSettingsUpdate::new(&request).unwrap(),
                &status_token(),
            )
            .await
            .unwrap();
        }
        assert_eq!(server.received_requests().await.unwrap().len(), 2);
    }
}

#[tokio::test]
async fn every_response_object_rejects_unknown_fields_and_known_fields_reject_wrong_types() {
    let mut cases = Vec::new();
    let global = fixtures::global();
    for (wire, _, _, boolean) in fixtures::GLOBAL_FIELDS {
        let mut bad = global.clone();
        bad[*wire] = if *boolean {
            json!("false")
        } else {
            json!(false)
        };
        cases.push(("/api/settings", bad));
        let mut bad = global.clone();
        bad[*wire] = Value::Null;
        cases.push(("/api/settings", bad));
    }
    let mut bad = global;
    bad["future"] = json!("secret-value");
    cases.push(("/api/settings", bad));
    let attachment = fixtures::attachment();
    for location in [
        "",
        "/storageConfig",
        "/storageConfig/filesystem",
        "/storageConfig/gridfs",
        "/storageConfig/s3",
        "/storageConfig/azure",
        "/storageConfig/gcs",
        "/uploadSettings",
        "/limitSettings",
        "/migrationSettings",
    ] {
        let mut bad = attachment.clone();
        bad.pointer_mut(location)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("future".into(), json!("secret-value"));
        cases.push(("/api/admin/attachment-settings", bad));
    }
    for (wire, _, _, kind) in fixtures::ATTACHMENT_FIELDS {
        let mut bad = attachment.clone();
        *bad.pointer_mut(wire).unwrap() = if *kind == "String" {
            json!(true)
        } else {
            json!("secret-value")
        };
        cases.push(("/api/admin/attachment-settings", bad));
    }
    for key in ["_id", "defaultStorage", "createdAt", "updatedAt"] {
        let mut bad = attachment.clone();
        bad.as_object_mut().unwrap().remove(key);
        cases.push(("/api/admin/attachment-settings", bad));
    }
    for key in ["secretAccessKey", "secretAccessKeySet"] {
        let mut bad = attachment.clone();
        bad["storageConfig"]["s3"]
            .as_object_mut()
            .unwrap()
            .remove(key);
        cases.push(("/api/admin/attachment-settings", bad));
    }
    for (pointer, value) in [
        ("/createdAt", json!("invalid-date")),
        ("/storageConfig/s3/secretAccessKey", json!("secret-value")),
        ("/storageConfig/azure/accountKey", json!("secret-value")),
        (
            "/storageConfig/gcs/credentials",
            json!({"private_key":"secret-value"}),
        ),
        ("/storageConfig/s3/secretAccessKeySet", json!("false")),
        ("/storageConfig", json!([])),
    ] {
        let mut bad = attachment.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        cases.push(("/api/admin/attachment-settings", bad));
    }
    for (route, response) in cases {
        let server = MockServer::start().await;
        Mock::given(path(route))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        let error = if route == "/api/settings" {
            client(&server)
                .global_settings(&status_token())
                .await
                .unwrap_err()
        } else {
            client(&server)
                .attachment_settings(&status_token())
                .await
                .unwrap_err()
        };
        assert!(matches!(error, ClientError::Protocol { .. }), "{error:?}");
        assert!(!format!("{error:?}").contains("secret-value"));
    }
}

#[tokio::test]
async fn empty_global_and_sparse_nullable_attachment_documents_remain_decodable() {
    for response in [
        json!({}),
        json!({"_id":"settings","defaultAuthenticationMethod":"password"}),
    ] {
        let server = MockServer::start().await;
        Mock::given(path("/api/settings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;
        client(&server)
            .global_settings(&status_token())
            .await
            .unwrap();
    }
    let server = MockServer::start().await;
    Mock::given(path("/api/admin/attachment-settings")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"s","defaultStorage":"fs","createdAt":"2026-10-15T00:00:00Z","updatedAt":"2026-10-15T00:00:00Z","storageConfig":null,"createdBy":null}))).mount(&server).await;
    client(&server)
        .attachment_settings(&status_token())
        .await
        .unwrap();
}

#[tokio::test]
async fn embedded_authorization_and_application_errors_keep_native_metadata() {
    for route in ["/api/settings", "/api/admin/attachment-settings"] {
        for code in [401, 403] {
            let server = MockServer::start().await;
            Mock::given(path(route)).respond_with(ResponseTemplate::new(200).set_body_json(json!({"error":"Forbidden","reason":"Forbidden","statusCode":code,"errorType":"Meteor.Error","isClientSafe":true,"message":"Forbidden"}))).mount(&server).await;
            let error = if route == "/api/settings" {
                client(&server)
                    .global_settings(&status_token())
                    .await
                    .unwrap_err()
            } else {
                client(&server)
                    .attachment_settings(&status_token())
                    .await
                    .unwrap_err()
            };
            assert!(
                matches!(error,ClientError::EmbeddedServer{wekan_status_code,..} if wekan_status_code==code)
            );
        }
    }
}

#[tokio::test]
async fn large_stored_settings_remain_readable_within_the_write_budget() {
    let server = MockServer::start().await;
    let product_name = "x".repeat(2 * 1024 * 1024);
    Mock::given(path("/api/settings"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"_id":"s","productName":product_name})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let document = client(&server)
        .global_settings(&status_token())
        .await
        .unwrap();
    assert_eq!(document.product_name.unwrap().len(), 2 * 1024 * 1024);
}
