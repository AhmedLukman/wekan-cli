use super::client;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use secrecy::SecretString;
use serde_json::{Value, json};
use wekan_cli::client::{ClientError, WekanClient, attachments::*};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path},
};

fn metadata() -> Value {
    json!({"attachmentId":"a","fileName":"data.bin","fileSize":3,"fileType":"application/octet-stream","storageBackend":"fs","boardId":"b","swimlaneId":"s","listId":"l","cardId":"c","createdAt":"2030-01-01T00:00:00.000Z","isImage":false})
}
fn response(operation: usize) -> Value {
    match operation {
        0 | 1 => {
            let mut value = json!({"success":true,"attachmentId":"a","fileName":"data.bin","fileSize":3,"storageBackend":"fs","message":"Uploaded"});
            if operation == 1 {
                value["backgroundImageURL"] = json!("/background");
            }
            value
        }
        2 | 3 => {
            let mut value = json!({"success":true,"attachmentId":"a","fileName":"data.bin","fileSize":3,"fileType":"application/octet-stream","storageBackend":"fs","base64Data":"AP8K"});
            if operation == 2 {
                value["backgroundImageURL"] = json!("/background");
            }
            value
        }
        4 => {
            let mut value = metadata();
            value["success"] = json!(true);
            value["versions"] = json!([{"versionName":"original","storage":"fs","size":3,"type":"application/octet-stream"}, {"versionName":"thumbnail"}]);
            value
        }
        5..=8 => json!({"success":true,"attachments":[metadata()],"count":1}),
        9 => {
            json!({"success":true,"sourceAttachmentId":"a","newAttachmentId":"new","fileName":"data.bin","fileSize":3,"message":"Copied"})
        }
        10 => {
            json!({"success":true,"attachmentId":"a","fileName":"data.bin","fileSize":3,"sourceBoardId":"source","targetBoardId":"b","message":"Moved"})
        }
        11 => json!({"success":true,"attachmentId":"a","fileName":"data.bin","message":"Deleted"}),
        _ => unreachable!(),
    }
}
fn route(operation: usize) -> (&'static str, &'static str) {
    match operation {
        0 => ("POST", "/api/attachment/upload"),
        1 => ("POST", "/api/attachment/upload-background"),
        2 => ("GET", "/api/attachment/download-background/b"),
        3 => ("GET", "/api/attachment/download/a"),
        4 => ("GET", "/api/attachment/info/a"),
        5 => ("GET", "/api/attachment/list/b"),
        6 => ("GET", "/api/attachment/list/b/s"),
        7 => ("GET", "/api/attachment/list/b/s/l"),
        8 => ("GET", "/api/attachment/list/b/s/l/c"),
        9 => ("POST", "/api/attachment/copy"),
        10 => ("POST", "/api/attachment/move"),
        11 => ("DELETE", "/api/attachment/delete/a"),
        _ => unreachable!(),
    }
}
fn transfer() -> TransferAttachmentRequest {
    TransferAttachmentRequest {
        attachment_id: "a".into(),
        target_board_id: "b".into(),
        target_swimlane_id: "s".into(),
        target_list_id: "l".into(),
        target_card_id: "c".into(),
    }
}
async fn invoke(client: &WekanClient, operation: usize) -> Result<(), ClientError> {
    let token = SecretString::from("attachment-token".to_owned());
    match operation {
        0 => {
            client
                .upload_attachment(
                    &UploadAttachmentRequest {
                        board_id: "b".into(),
                        swimlane_id: "s".into(),
                        list_id: "l".into(),
                        card_id: "c".into(),
                        file_name: "data.bin".into(),
                        file_data: vec![0, 255, 10],
                        file_type: None,
                    },
                    &token,
                )
                .await?;
        }
        1 => {
            client
                .upload_background(
                    &UploadBackgroundRequest {
                        board_id: "b".into(),
                        file_name: "data.bin".into(),
                        file_data: vec![0, 255, 10],
                        file_type: None,
                    },
                    &token,
                )
                .await?;
        }
        2 => {
            let r = client.download_background("b", &token).await?;
            decode_file(&r.base64_data)?;
        }
        3 => {
            let r = client.download_attachment("a", &token).await?;
            decode_file(&r.base64_data)?;
        }
        4 => {
            client.attachment_info("a", &token).await?;
        }
        5..=8 => {
            client
                .attachments(
                    "b",
                    (operation >= 6).then_some("s"),
                    (operation >= 7).then_some("l"),
                    (operation >= 8).then_some("c"),
                    &token,
                )
                .await?;
        }
        9 => {
            client.copy_attachment(&transfer(), &token).await?;
        }
        10 => {
            client.move_attachment(&transfer(), &token).await?;
        }
        11 => {
            client.delete_attachment("a", &token).await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

#[tokio::test]
async fn all_twelve_operations_use_exact_routes_and_request_bodies() {
    for operation in 0..12 {
        let server = MockServer::start().await;
        let (verb, url) = route(operation);
        Mock::given(method(verb))
            .and(path(url))
            .and(header("Authorization", "Bearer attachment-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response(operation)))
            .expect(1)
            .mount(&server)
            .await;
        invoke(&client(&server), operation).await.unwrap();
        let requests = server.received_requests().await.unwrap();
        if matches!(operation, 0 | 1 | 9 | 10) {
            assert_eq!(
                requests[0].headers.get("content-type").unwrap(),
                "application/json"
            );
            let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
            let expected = match operation {
                0 => {
                    json!({"boardId":"b","swimlaneId":"s","listId":"l","cardId":"c","fileName":"data.bin","fileData":"AP8K"})
                }
                1 => json!({"boardId":"b","fileName":"data.bin","fileData":"AP8K"}),
                _ => {
                    json!({"attachmentId":"a","targetBoardId":"b","targetSwimlaneId":"s","targetListId":"l","targetCardId":"c"})
                }
            };
            assert_eq!(body, expected);
        }
    }
    let request = UploadBackgroundRequest {
        board_id: "b".into(),
        file_name: "  image.png  ".into(),
        file_data: vec![0, 255, 10],
        file_type: Some("image/png".into()),
    };
    assert_eq!(
        serde_json::to_value(request).unwrap()["fileType"],
        "image/png"
    );
}

#[tokio::test]
async fn responses_reject_unknown_missing_invalid_and_unsuccessful_properties() {
    for operation in 0..12 {
        let mut unknown = response(operation);
        unknown["future"] = json!({"secret":"do not disclose"});
        let mut missing = response(operation);
        missing.as_object_mut().unwrap().remove("success");
        let mut invalid = response(operation);
        invalid["fileSize"] = json!("3");
        if matches!(operation, 5..=8) {
            invalid = response(operation);
            invalid["attachments"][0]["fileSize"] = json!("3");
        }
        let mut failed = response(operation);
        failed["success"] = json!(false);
        for body in [unknown, missing, invalid, failed] {
            let server = MockServer::start().await;
            let (verb, url) = route(operation);
            Mock::given(method(verb))
                .and(path(url))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .expect(1)
                .mount(&server)
                .await;
            assert!(
                matches!(
                    invoke(&client(&server), operation).await,
                    Err(ClientError::Protocol { .. })
                ),
                "operation {operation}"
            );
        }
    }
    for pointer in [
        "/versions/0/future",
        "/versions/0/size",
        "/versions/0/versionName",
    ] {
        let mut body = response(4);
        let key = pointer.rsplit('/').next().unwrap();
        body["versions"][0][key] = json!({"unknown":true});
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
        assert!(matches!(
            invoke(&client(&server), 4).await,
            Err(ClientError::Protocol { .. })
        ));
    }
    let mut body = response(5);
    body["attachments"][0]["future"] = json!(true);
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    assert!(matches!(
        invoke(&client(&server), 5).await,
        Err(ClientError::Protocol { .. })
    ));
    let mut omitted = response(4);
    for key in [
        "boardId",
        "swimlaneId",
        "listId",
        "cardId",
        "createdAt",
        "isImage",
    ] {
        omitted.as_object_mut().unwrap().remove(key);
    }
    assert!(serde_json::from_value::<AttachmentInfo>(omitted.clone()).is_ok());
    omitted["createdAt"] = Value::Null;
    assert!(serde_json::from_value::<AttachmentInfo>(omitted).is_err());
    let mut invalid_date = response(4);
    invalid_date["createdAt"] = json!("not-a-date");
    assert!(serde_json::from_value::<AttachmentInfo>(invalid_date).is_err());
    let mut extended_date = response(4);
    extended_date["createdAt"] = json!("+010000-01-01T00:00:00.000Z");
    assert!(serde_json::from_value::<AttachmentInfo>(extended_date).is_ok());
}

#[tokio::test]
async fn download_budget_does_not_expand_metadata_or_error_budgets() {
    let data = [0, 255, 13, 10].repeat(300_000);
    let mut body = response(3);
    body["fileSize"] = json!(data.len());
    body["base64Data"] = json!(STANDARD.encode(&data));
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    let result = client(&server)
        .download_attachment("a", &SecretString::from("attachment-token".to_owned()))
        .await
        .unwrap();
    assert_eq!(decode_file(&result.base64_data).unwrap(), data);
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .mount(&server)
        .await;
    assert!(matches!(
        invoke(&client(&server), 3).await,
        Err(ClientError::ResponseTooLarge {
            limit_bytes: 1_048_576,
            ..
        })
    ));
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
        .mount(&server)
        .await;
    assert!(matches!(
        invoke(&client(&server), 4).await,
        Err(ClientError::ResponseTooLarge {
            limit_bytes: 1_048_576,
            ..
        })
    ));
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = std::thread::spawn(move || {
        use std::io::{Read, Write};
        let (mut socket, _) = listener.accept().unwrap();
        let mut request = [0; 8192];
        assert!(socket.read(&mut request).unwrap() > 0);
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100663297\r\nConnection: close\r\n\r\n")
            .unwrap();
    });
    assert!(matches!(
        invoke(&super::client_from_url(&url), 3).await,
        Err(ClientError::ResponseTooLarge {
            limit_bytes: 100_663_296,
            ..
        })
    ));
    task.join().unwrap();
}

#[tokio::test]
async fn malformed_base64_and_server_errors_remain_explicit() {
    for operation in [2, 3] {
        let mut body = response(operation);
        body["base64Data"] = json!("%%%");
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
        assert!(matches!(
            invoke(&client(&server), operation).await,
            Err(ClientError::Protocol { .. })
        ));
    }
    for operation in [0, 1, 9, 10] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(408)
                    .set_body_json(json!({"success":false,"error":"Request timeout"})),
            )
            .expect(1)
            .mount(&server)
            .await;
        assert!(
            matches!(invoke(&client(&server), operation).await, Err(ClientError::Server { status, .. }) if status.as_u16() == 408)
        );
    }
}
