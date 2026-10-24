use serde_json::{Value, json};
use wekan_cli::client::{ClientError, ResponseDecodeKind, UserAction, UserProfile};
use wiremock::{Mock, MockServer, ResponseTemplate};

const DIALOGS: [&str; 3] = [
    "moveAndCopyDialog",
    "moveChecklistDialog",
    "copyChecklistDialog",
];

#[tokio::test]
async fn missing_user_responses_preserve_native_server_and_protocol_failures() {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/api/users/missing"))
        .respond_with(
            ResponseTemplate::new(500).set_body_json(json!({"error": "Internal server error"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(wiremock::matchers::method("PUT"))
        .and(wiremock::matchers::path("/api/users/missing"))
        .and(wiremock::matchers::body_json(json!({})))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let client = super::client(&server);
    let token = super::user_token();
    let get_error = client.user("missing", &token).await.unwrap_err();
    assert!(matches!(
        get_error,
        ClientError::Server {
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            server_error: Some(ref error),
            ..
        } if error == "Internal server error"
    ));
    let update_error = client
        .update_user_without_action("missing", &token)
        .await
        .unwrap_err();
    assert!(matches!(
        update_error,
        ClientError::Protocol {
            success_status_received: true,
            ..
        }
    ));
    server.verify().await;
}

fn options() -> Value {
    json!({"boardId":"destination", "swimlaneId":"lane", "listId":"list", "cardId":null})
}

#[test]
fn profile_preferences_preserve_dynamic_ids_and_verified_numeric_values() {
    for field in DIALOGS {
        for card in [
            None,
            Some(Value::Null),
            Some(json!("card-1")),
            Some(json!("")),
        ] {
            let mut options = options();
            if let Some(card) = card {
                options["cardId"] = card;
            } else {
                options.as_object_mut().unwrap().remove("cardId");
            }
            let profile: UserProfile = serde_json::from_value(json!({
                field: {"source-a": options, "source-b": options},
                "listConstraints": {"source-a":{"list-a":550,"list-b":320.5},"source-b":{"list-c":-1,"list-d":0}}
            })).unwrap();
            let dialogs = match field {
                "moveAndCopyDialog" => profile.move_and_copy_dialog,
                "moveChecklistDialog" => profile.move_checklist_dialog,
                _ => profile.copy_checklist_dialog,
            }
            .unwrap();
            assert_eq!(dialogs.len(), 2);
            assert_eq!(dialogs["source-a"].board_id, "destination");
            assert_eq!(
                dialogs["source-a"].card_id.as_deref(),
                options["cardId"].as_str()
            );
            let constraints = profile.list_constraints.unwrap();
            assert_eq!(constraints["source-a"]["list-b"].as_f64(), Some(320.5));
            assert_eq!(constraints["source-b"]["list-c"].as_i64(), Some(-1));
        }
    }
    for profile in [
        json!({}),
        json!({"moveAndCopyDialog":null,"listConstraints":null}),
        json!({"moveAndCopyDialog":{},"moveChecklistDialog":{},"copyChecklistDialog":{},"listConstraints":{}}),
    ] {
        serde_json::from_value::<UserProfile>(profile).unwrap();
    }
}

async fn reject_at_every_user_record_boundary(
    profile: Value,
    kind: ResponseDecodeKind,
    path: &str,
) {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::any())
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"_id":"user-1","profile":profile})),
        )
        .expect(4)
        .mount(&server)
        .await;
    let client = super::client(&server);
    let token = super::user_token();
    let results = [
        client.current_user(&token).await.map(|_| ()),
        client.user("user-1", &token).await.map(|_| ()),
        client
            .update_user_without_action("user-1", &token)
            .await
            .map(|_| ()),
        client
            .user_action("user-1", UserAction::EnableLogin, &token)
            .await
            .map(|_| ()),
    ];
    for result in results {
        let error = result.unwrap_err();
        assert!(!format!("{error:?}").contains("unmapped-secret"));
        let ClientError::Protocol {
            diagnostic: Some(diagnostic),
            success_status_received: true,
            ..
        } = error
        else {
            panic!("expected a strict response decoding error: {error:?}");
        };
        if kind == ResponseDecodeKind::InvalidType {
            // Arbitrary-precision JSON numbers can be reported by Serde as
            // invalid values rather than invalid types. Both reject the value.
            assert!(matches!(
                diagnostic.kind,
                ResponseDecodeKind::InvalidType | ResponseDecodeKind::InvalidValue
            ));
        } else {
            assert_eq!(diagnostic.kind, kind);
        }
        assert_eq!(diagnostic.path, path);
    }
}

#[tokio::test]
async fn notification_visibility_time_is_optional_numeric_at_every_user_record_boundary() {
    for show_at in [
        None,
        Some(json!(1_792_645_200_000u64)),
        Some(json!(0)),
        Some(json!(-0.5)),
    ] {
        let mut notification = json!({"activity":"activity-1", "read":null});
        if let Some(value) = &show_at {
            notification["showAt"] = value.clone();
        }
        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::any())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_id":"user-1", "profile":{"notifications":[notification]}
            })))
            .expect(4)
            .mount(&server)
            .await;
        let client = super::client(&server);
        let token = super::user_token();
        let users = [
            client.current_user(&token).await.unwrap(),
            client.user("user-1", &token).await.unwrap(),
            client
                .update_user_without_action("user-1", &token)
                .await
                .unwrap(),
            match client
                .user_action("user-1", UserAction::EnableLogin, &token)
                .await
                .unwrap()
            {
                wekan_cli::client::UserActionResult::LoginChanged(user) => *user,
                _ => panic!("expected the updated user"),
            },
        ];
        for user in users {
            assert_eq!(
                user.profile.notifications.unwrap()[0]
                    .show_at
                    .as_ref()
                    .map(|value| json!(value)),
                show_at
            );
        }
    }
}

#[tokio::test]
async fn notification_visibility_time_rejects_null_invalid_types_and_unknown_fields() {
    for invalid in [
        Value::Null,
        json!("1792645200000"),
        json!(false),
        json!([]),
        json!({}),
    ] {
        reject_at_every_user_record_boundary(
            json!({"notifications":[{"activity":"activity-1", "showAt":invalid}]}),
            ResponseDecodeKind::InvalidType,
            "/profile/notifications/0/showAt",
        )
        .await;
    }
    reject_at_every_user_record_boundary(
        json!({"notifications":[{"activity":"activity-1", "showAt":1792645200000u64, "future":"unmapped-secret"}]}),
        ResponseDecodeKind::UnknownField,
        "/profile/notifications/0/future",
    ).await;
    reject_at_every_user_record_boundary(
        json!({"notifications":[{"showAt":1792645200000u64}]}),
        ResponseDecodeKind::MissingField,
        "/profile/notifications/0/activity",
    )
    .await;
}

#[tokio::test]
async fn dialog_preferences_reject_unknown_fields_missing_ids_and_invalid_types() {
    for field in DIALOGS {
        let mut unknown = options();
        unknown["future"] = json!({"unmapped-secret":true});
        reject_at_every_user_record_boundary(
            json!({field:{"source":unknown}}),
            ResponseDecodeKind::UnknownField,
            &format!("/profile/{field}/source/future"),
        )
        .await;
        for name in ["boardId", "swimlaneId", "listId"] {
            let mut missing = options();
            missing.as_object_mut().unwrap().remove(name);
            reject_at_every_user_record_boundary(
                json!({field:{"source":missing}}),
                ResponseDecodeKind::MissingField,
                &format!("/profile/{field}/source/{name}"),
            )
            .await;
            for invalid in [Value::Null, json!(false), json!(4), json!([]), json!({})] {
                let mut options = options();
                options[name] = invalid;
                reject_at_every_user_record_boundary(
                    json!({field:{"source":options}}),
                    ResponseDecodeKind::InvalidType,
                    &format!("/profile/{field}/source/{name}"),
                )
                .await;
            }
        }
        for invalid in [json!(false), json!(4), json!([]), json!({})] {
            let mut options = options();
            options["cardId"] = invalid;
            reject_at_every_user_record_boundary(
                json!({field:{"source":options}}),
                ResponseDecodeKind::InvalidType,
                &format!("/profile/{field}/source/cardId"),
            )
            .await;
        }
        for invalid in [
            Value::Null,
            json!(false),
            json!(4),
            json!("unmapped-secret"),
            json!([]),
        ] {
            reject_at_every_user_record_boundary(
                json!({field:{"source":invalid}}),
                ResponseDecodeKind::InvalidType,
                &format!("/profile/{field}/source"),
            )
            .await;
        }
    }
}

#[tokio::test]
async fn list_constraints_reject_non_numeric_values_without_hiding_dynamic_keys() {
    for invalid in [
        Value::Null,
        json!(false),
        json!("unmapped-secret"),
        json!([]),
        json!({"future":550}),
    ] {
        let path = if invalid.is_object() {
            "/profile/listConstraints/source/list/future"
        } else {
            "/profile/listConstraints/source/list"
        };
        reject_at_every_user_record_boundary(
            json!({"listConstraints":{"source":{"list":invalid}}}),
            ResponseDecodeKind::InvalidType,
            path,
        )
        .await;
    }
    for invalid in [Value::Null, json!(550), json!([])] {
        reject_at_every_user_record_boundary(
            json!({"listConstraints":{"source":invalid}}),
            ResponseDecodeKind::InvalidType,
            "/profile/listConstraints/source",
        )
        .await;
    }
}

#[test]
#[ignore = "requires WEKAN_CORRECTED_SPEC pointing to a freshly generated corrected contract"]
fn corrected_contract_maps_typed_profile_preferences() {
    let path = std::env::var_os("WEKAN_CORRECTED_SPEC").expect("set WEKAN_CORRECTED_SPEC");
    let spec: serde_yaml_ng::Value =
        serde_yaml_ng::from_reader(std::fs::File::open(path).unwrap()).unwrap();
    let definitions = &spec["definitions"];
    let notification = &definitions["VerifiedUsersUserProfileNotification"];
    assert_eq!(notification["additionalProperties"], false);
    assert_eq!(notification["properties"]["showAt"]["type"], "number");
    assert!(notification["properties"]["showAt"]["x-nullable"].is_null());
    assert_eq!(
        notification["required"].as_sequence().unwrap(),
        &vec![serde_yaml_ng::Value::from("activity")]
    );
    let options = &definitions["VerifiedUsersUserDialogOptions"];
    assert_eq!(options["additionalProperties"], false);
    let required: Vec<_> = options["required"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(required, ["boardId", "swimlaneId", "listId"]);
    for field in ["boardId", "swimlaneId", "listId", "cardId"] {
        assert_eq!(options["properties"][field]["type"], "string");
    }
    assert_eq!(options["properties"]["cardId"]["x-nullable"], true);
    for field in DIALOGS {
        assert_eq!(
            definitions["VerifiedUsersUserProfile"]["properties"][field]["additionalProperties"]["$ref"],
            "#/definitions/VerifiedUsersUserDialogOptions"
        );
    }
    assert_eq!(
        definitions["VerifiedUsersUserProfile"]["properties"]["listConstraints"]["additionalProperties"]
            ["additionalProperties"]["type"],
        "number"
    );
}
