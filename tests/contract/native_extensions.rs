use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use wekan_cli::client::{
    admin::login_providers::{OauthProviderSettings, PasswordlessSettings},
    admin::problems::{ProblemAcknowledgment, ProblemPage, ProblemsOverview},
    attachments::recovery::{
        DeletedAttachment, RestoreAttachmentResponse, SoftDeleteAttachmentResponse,
    },
    boards::presentation::{BoardCardFieldOrder, BoardViewSettings},
    cards::bot_runs::BotRunReport,
    integrations::inbound_email::{BounceReceipt, InboundEmailReceipt},
};

fn strict<T: DeserializeOwned>(value: Value, required: &[&str], nested: &[&str]) {
    serde_json::from_value::<T>(value.clone())
        .unwrap_or_else(|e| panic!("valid native fixture: {e}"));
    for pointer in std::iter::once("").chain(nested.iter().copied()) {
        let mut unknown = value.clone();
        unknown
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unmappedField".into(), json!(true));
        assert!(
            serde_json::from_value::<T>(unknown).is_err(),
            "unknown at {pointer}"
        );
    }
    for field in required {
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(*field);
        assert!(
            serde_json::from_value::<T>(missing).is_err(),
            "missing {field}"
        );
        let mut invalid = value.clone();
        invalid[*field] = json!({"invalidType":true});
        assert!(
            serde_json::from_value::<T>(invalid).is_err(),
            "type of {field}"
        );
    }
}

#[test]
fn native_user_channels_and_webhook_delivery_reject_unmapped_fields() {
    use wekan_cli::client::{integrations::WebhookNotificationDelivery, users::channels::*};
    strict::<Passkey>(
        json!({"id":"key","publicKey":"public","counter":0,"transports":["usb"],"name":"Key","createdAt":"2026-10-01T00:00:00Z"}),
        &[
            "id",
            "publicKey",
            "counter",
            "transports",
            "name",
            "createdAt",
        ],
        &[],
    );
    strict::<SqrlIdentity>(
        json!({"idk":"public","suk":"public","vuk":"public","disabled":false,"addedAt":"2026-10-01T00:00:00Z"}),
        &["idk", "disabled", "addedAt"],
        &[],
    );
    strict::<PushSubscription>(
        json!({"endpoint":"https://fcm.googleapis.com/fcm/send/private","keys":{"p256dh":"public","auth":"private"},"createdAt":"2026-10-01T00:00:00Z"}),
        &["endpoint", "keys", "createdAt"],
        &["/keys"],
    );
    strict::<NtfySettings>(
        json!({"enabled":false,"url":"https://ntfy.sh/private","token":"private"}),
        &["enabled", "url", "token"],
        &[],
    );
    strict::<PendingNotification>(
        json!({"_id":"receipt","version":1,"userId":"u","activityId":"a"}),
        &["_id", "version", "userId", "activityId"],
        &[],
    );
    strict::<WebhookNotificationDelivery>(
        json!({"webhook":{"text":false,"dailyTime":"09:30","intervalMinutes":60}}),
        &[],
        &["/webhook"],
    );
    for bad in [
        json!({"webhook":{"text":"false"}}),
        json!({"webhook":{"intervalMinutes":"60"}}),
        json!({"email":{}}),
    ] {
        assert!(serde_json::from_value::<WebhookNotificationDelivery>(bad).is_err());
    }
    let subscription: PushSubscription = serde_json::from_value(json!({"endpoint":"secret-endpoint","keys":{"p256dh":"public-key","auth":"secret-auth"},"createdAt":"2026-10-01T00:00:00Z"})).unwrap();
    let output = format!(
        "{subscription:?}{}",
        serde_json::to_string(&subscription).unwrap()
    );
    assert!(!output.contains("secret-endpoint"));
    assert!(!output.contains("secret-auth"));
    assert!(serde_json::from_value::<Passkey>(json!({"id":"key","publicKey":"public","counter":0,"transports":[],"name":"Key","createdAt":"invalid-date"})).is_err());
    assert!(
        serde_json::from_value::<SqrlIdentity>(
            json!({"idk":"public","disabled":false,"addedAt":"invalid-date"})
        )
        .is_err()
    );
    assert!(serde_json::from_value::<PushSubscription>(json!({"endpoint":"private","keys":{"p256dh":"public","auth":"private"},"createdAt":"invalid-date"})).is_err());
}

#[test]
fn native_presentation_and_recovery_contracts_are_strict() {
    strict::<BoardViewSettings>(
        json!({"boardViewSettings":{"board":{"showOnPublic":true,"showOnPrivate":false}},"defaultPublicBoardView":"board","defaultPrivateBoardView":"board","boardViewOrder":["board"],"keys":["board"]}),
        &[
            "boardViewSettings",
            "defaultPublicBoardView",
            "defaultPrivateBoardView",
            "boardViewOrder",
            "keys",
        ],
        &["/boardViewSettings/board"],
    );
    strict::<BoardCardFieldOrder>(
        json!({"cardFieldOrder":["labels"],"keys":["labels"]}),
        &["cardFieldOrder", "keys"],
        &[],
    );
    strict::<DeletedAttachment>(
        json!({"attachmentId":"a","attachmentName":"file.bin","attachmentType":"application/octet-stream","boardId":"b","deletedAt":"2026-10-20T08:00:00Z"}),
        &[
            "attachmentId",
            "attachmentName",
            "attachmentType",
            "boardId",
            "deletedAt",
        ],
        &[],
    );
    strict::<SoftDeleteAttachmentResponse>(
        json!({"deleted":true,"batchId":"batch"}),
        &["deleted"],
        &[],
    );
    strict::<RestoreAttachmentResponse>(json!({"restored":true}), &["restored"], &[]);
}

#[test]
fn native_login_provider_contracts_reject_nested_unknowns() {
    let source = json!({"source":"default","value":null});
    let entry =
        json!({"enabled":source,"id":source,"secret":{"source":"default","hasValue":false}});
    let providers: serde_json::Map<_, _> =
        wekan_cli::client::admin::login_providers::OAUTH_PROVIDER_KEYS
            .iter()
            .map(|key| ((*key).to_owned(), entry.clone()))
            .collect();
    strict::<OauthProviderSettings>(
        json!({"providers":providers,"loginStyle":source,"mergeExistingUsers":source,"allowedEmailDomains":source,"passwordless":source}),
        &[
            "providers",
            "loginStyle",
            "mergeExistingUsers",
            "allowedEmailDomains",
            "passwordless",
        ],
        &[
            "/providers",
            "/providers/google",
            "/providers/google/enabled",
            "/providers/google/id",
            "/providers/google/secret",
            "/loginStyle",
            "/mergeExistingUsers",
            "/allowedEmailDomains",
            "/passwordless",
        ],
    );
    strict::<PasswordlessSettings>(
        json!({"passwordless":{"source":"database","value":"true"}}),
        &["passwordless"],
        &["/passwordless"],
    );
    // An unset default is undefined in authConfigSources and omitted by JSON.
    serde_json::from_value::<PasswordlessSettings>(json!({"passwordless":{"source":"default"}}))
        .unwrap();
}

#[test]
fn native_problem_contracts_reject_top_level_and_nested_unknown_fields() {
    strict::<ProblemsOverview>(
        json!({
            "anyInProgress":true,"inProgress":[{"kind":"migration","active":true,"message":"Running"}],
            "anyProblems":true,"problems":[{"id":"problem","severity":"high","title":"Problem","detail":"Details","count":2}],
            "newProblems":[{"stream":"tests","count":2}],"streams":["tests"]
        }),
        &[
            "anyInProgress",
            "inProgress",
            "anyProblems",
            "problems",
            "newProblems",
            "streams",
        ],
        &["/inProgress/0", "/problems/0", "/newProblems/0"],
    );
    strict::<ProblemPage>(
        json!({
            "stream":"tests","total":1,"limit":50,"skip":0,
            "rows":[{"_id":"event","stream":"tests","at":"2026-10-01T00:00:00Z",
                "location":{"country":"KE","city":"Nairobi","latitude":-1.5,"longitude":36},
                "actors":{"actor":{"kind":"user","value":"alice","count":1,"at":"2026-10-01T00:00:00Z"}}}]
        }),
        &["stream", "total", "limit", "skip", "rows"],
        &["/rows/0", "/rows/0/location", "/rows/0/actors/actor"],
    );
    strict::<ProblemAcknowledgment>(json!({"acknowledged":"tests"}), &["acknowledged"], &[]);
}

#[test]
fn native_bot_and_mail_receipts_are_complete_typed_objects() {
    strict::<BotRunReport>(
        json!({"_id":"run","status":"awaiting_input"}),
        &["_id", "status"],
        &[],
    );
    strict::<InboundEmailReceipt>(json!({"_id":"card"}), &["_id"], &[]);
    serde_json::from_value::<InboundEmailReceipt>(json!({"_id":null})).unwrap();
    strict::<BounceReceipt>(json!({"ok":true,"recorded":1}), &["ok", "recorded"], &[]);
}

#[tokio::test]
async fn inbound_email_transport_preserves_provider_body_without_session_credentials() {
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_bytes, header, method, path},
    };
    let server = MockServer::start().await;
    let client = super::client_from_url(&format!("{}/wekan/", server.uri()));
    let body = b"{ \"signature\": \"original-provider-signature\" }\r\n";
    Mock::given(method("POST"))
        .and(path("/wekan/api/inbound-email"))
        .and(header("content-type", "application/json"))
        .and(body_bytes(body.as_slice()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"_id":"card"})))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        client
            .submit_inbound_email(
                body.to_vec(),
                "application/json",
                reqwest::header::HeaderMap::new(),
            )
            .await
            .unwrap()
            .resource_id
            .as_deref(),
        Some("card")
    );
    let requests = server.received_requests().await.unwrap();
    assert!(!requests[0].headers.contains_key("authorization"));
}

#[tokio::test]
async fn raw_uploads_preserve_file_bytes_and_transport_specific_mime_defaults() {
    use secrecy::SecretString;
    use wekan_cli::client::attachments::{UploadAttachmentRequest, UploadBackgroundRequest};
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_bytes, header, method, path, query_param},
    };

    for background in [false, true] {
        for mime in [None, Some("application/json")] {
            let server = MockServer::start().await;
            let client = super::client_from_url(&format!("{}/wekan/", server.uri()));
            let operation = if background {
                "upload-background"
            } else {
                "upload"
            };
            let bytes = b"{\"content\":\"original bytes\"}\n";
            let mut mock = Mock::given(method("POST"))
                .and(path(format!("/wekan/api/attachment/{operation}")))
                .and(header("content-type", "application/octet-stream"))
                .and(header("authorization", "Bearer raw-upload-token"))
                .and(query_param("boardId", "b"))
                .and(query_param("fileName", "data.json"))
                .and(body_bytes(bytes.as_slice()));
            let expected_mime = mime.or(background.then_some("image/png"));
            if let Some(expected) = expected_mime {
                mock = mock.and(query_param("fileType", expected));
            }
            let mut response = json!({
                "success":true,"attachmentId":"a","fileName":"data.json",
                "fileSize":bytes.len(),"storageBackend":"fs","message":"uploaded"
            });
            if background {
                response["backgroundImageURL"] = json!("/background/a");
            } else {
                mock = mock
                    .and(query_param("swimlaneId", "s"))
                    .and(query_param("listId", "l"))
                    .and(query_param("cardId", "c"));
            }
            mock.respond_with(ResponseTemplate::new(200).set_body_json(response))
                .expect(1)
                .mount(&server)
                .await;
            let token = SecretString::from("raw-upload-token");
            if background {
                client
                    .upload_background_raw(
                        UploadBackgroundRequest {
                            board_id: "b".into(),
                            file_name: "data.json".into(),
                            file_data: bytes.to_vec(),
                            file_type: mime.map(str::to_owned),
                        },
                        &token,
                    )
                    .await
                    .unwrap();
            } else {
                client
                    .upload_attachment_raw(
                        UploadAttachmentRequest {
                            board_id: "b".into(),
                            swimlane_id: "s".into(),
                            list_id: "l".into(),
                            card_id: "c".into(),
                            file_name: "data.json".into(),
                            file_data: bytes.to_vec(),
                            file_type: mime.map(str::to_owned),
                        },
                        &token,
                    )
                    .await
                    .unwrap();
            }
            let requests = server.received_requests().await.unwrap();
            let mime_values: Vec<_> = requests[0]
                .url
                .query_pairs()
                .filter(|(key, _)| key == "fileType")
                .map(|(_, value)| value.into_owned())
                .collect();
            assert_eq!(
                mime_values,
                expected_mime
                    .into_iter()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            );
        }
    }
}
