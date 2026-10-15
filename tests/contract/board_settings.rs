use super::client;
use secrecy::SecretString;
use serde_json::{Value, json};
use wekan_cli::client::{
    ClientError, WekanClient,
    boards::card_settings::{BoardCardSettings, UpdateBoardCardSettingsRequest},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

// Exact v11.06 corrected-contract keys, independent of DTO deserialization.
const FIELDS: &[(&str, &str, &str)] = &[
    (
        "allowsCardCounterList",
        "allows_card_counter_list",
        "--allows-card-counter-list",
    ),
    ("cardAging", "card_aging", "--card-aging"),
    (
        "allowsBoardMemberList",
        "allows_board_member_list",
        "--allows-board-member-list",
    ),
    (
        "allowsShowLists",
        "allows_show_lists",
        "--allows-show-lists",
    ),
    (
        "allowsAttachments",
        "allows_attachments",
        "--allows-attachments",
    ),
    (
        "allowsChecklists",
        "allows_checklists",
        "--allows-checklists",
    ),
    ("allowsComments", "allows_comments", "--allows-comments"),
    (
        "allowsDescriptionTitle",
        "allows_description_title",
        "--allows-description-title",
    ),
    (
        "allowsDescriptionText",
        "allows_description_text",
        "--allows-description-text",
    ),
    (
        "allowsActivities",
        "allows_activities",
        "--allows-activities",
    ),
    ("allowsLabels", "allows_labels", "--allows-labels"),
    ("allowsCreator", "allows_creator", "--allows-creator"),
    ("allowsAssignee", "allows_assignee", "--allows-assignee"),
    ("allowsMembers", "allows_members", "--allows-members"),
    (
        "allowsRequestedBy",
        "allows_requested_by",
        "--allows-requested-by",
    ),
    (
        "allowsCardSortingByNumber",
        "allows_card_sorting_by_number",
        "--allows-card-sorting-by-number",
    ),
    (
        "allowsCardNumber",
        "allows_card_number",
        "--allows-card-number",
    ),
    (
        "allowsAssignedBy",
        "allows_assigned_by",
        "--allows-assigned-by",
    ),
    (
        "allowsReceivedDate",
        "allows_received_date",
        "--allows-received-date",
    ),
    (
        "allowsStartDate",
        "allows_start_date",
        "--allows-start-date",
    ),
    ("allowsEndDate", "allows_end_date", "--allows-end-date"),
    ("allowsDueDate", "allows_due_date", "--allows-due-date"),
    ("allowsSubtasks", "allows_subtasks", "--allows-subtasks"),
    (
        "allowsCreatorOnMinicard",
        "allows_creator_on_minicard",
        "--allows-creator-on-minicard",
    ),
    (
        "allowsAttachmentsOnMinicard",
        "allows_attachments_on_minicard",
        "--allows-attachments-on-minicard",
    ),
    (
        "allowsChecklistsOnMinicard",
        "allows_checklists_on_minicard",
        "--allows-checklists-on-minicard",
    ),
    (
        "allowsChecklistAtMinicard",
        "allows_checklist_at_minicard",
        "--allows-checklist-at-minicard",
    ),
    (
        "allowsCoverAttachmentOnMinicard",
        "allows_cover_attachment_on_minicard",
        "--allows-cover-attachment-on-minicard",
    ),
    (
        "allowsBadgeAttachmentOnMinicard",
        "allows_badge_attachment_on_minicard",
        "--allows-badge-attachment-on-minicard",
    ),
    (
        "allowsCardSortingByNumberOnMinicard",
        "allows_card_sorting_by_number_on_minicard",
        "--allows-card-sorting-by-number-on-minicard",
    ),
    (
        "allowsCardNumberOnMinicard",
        "allows_card_number_on_minicard",
        "--allows-card-number-on-minicard",
    ),
    (
        "allowsDescriptionTitleOnMinicard",
        "allows_description_title_on_minicard",
        "--allows-description-title-on-minicard",
    ),
    (
        "allowsDescriptionTextOnMinicard",
        "allows_description_text_on_minicard",
        "--allows-description-text-on-minicard",
    ),
    (
        "allowsLabelsOnMinicard",
        "allows_labels_on_minicard",
        "--allows-labels-on-minicard",
    ),
    (
        "allowsAssigneeOnMinicard",
        "allows_assignee_on_minicard",
        "--allows-assignee-on-minicard",
    ),
    (
        "allowsMembersOnMinicard",
        "allows_members_on_minicard",
        "--allows-members-on-minicard",
    ),
    (
        "allowsRequestedByOnMinicard",
        "allows_requested_by_on_minicard",
        "--allows-requested-by-on-minicard",
    ),
    (
        "allowsAssignedByOnMinicard",
        "allows_assigned_by_on_minicard",
        "--allows-assigned-by-on-minicard",
    ),
    (
        "allowsReceivedDateOnMinicard",
        "allows_received_date_on_minicard",
        "--allows-received-date-on-minicard",
    ),
    (
        "allowsStartDateOnMinicard",
        "allows_start_date_on_minicard",
        "--allows-start-date-on-minicard",
    ),
    (
        "allowsEndDateOnMinicard",
        "allows_end_date_on_minicard",
        "--allows-end-date-on-minicard",
    ),
    (
        "allowsDueDateOnMinicard",
        "allows_due_date_on_minicard",
        "--allows-due-date-on-minicard",
    ),
    (
        "allowsSubtasksOnMinicard",
        "allows_subtasks_on_minicard",
        "--allows-subtasks-on-minicard",
    ),
    (
        "allowsShowListsOnMinicard",
        "allows_show_lists_on_minicard",
        "--allows-show-lists-on-minicard",
    ),
    ("cardAgingDays1", "card_aging_days1", "--card-aging-days1"),
    ("cardAgingDays2", "card_aging_days2", "--card-aging-days2"),
    ("cardAgingDays3", "card_aging_days3", "--card-aging-days3"),
];

fn settings_document() -> Value {
    FIELDS
        .iter()
        .enumerate()
        .map(|(i, (wire, _, _))| {
            (
                (*wire).to_owned(),
                if i < 44 {
                    json!(i % 2 == 0)
                } else {
                    json!(i as f64 - 44.5)
                },
            )
        })
        .collect::<serde_json::Map<_, _>>()
        .into()
}

async fn invoke(client: &WekanClient, index: usize) -> Result<(), ClientError> {
    let token = SecretString::from("settings-token".to_owned());
    match index {
        0 => {
            client.board_card_settings("b", &token).await?;
        }
        1 => {
            client
                .update_board_card_settings(
                    "b",
                    &UpdateBoardCardSettingsRequest {
                        allows_attachments: Some(false),
                        card_aging_days1: Some(0),
                        ..Default::default()
                    },
                    &token,
                )
                .await?;
        }
        2 => {
            client.board_domains("b", &token).await?;
        }
        3 => {
            client
                .add_board_domain("b", " EXAMPLE.test ", &token)
                .await?;
        }
        4 => {
            client
                .remove_board_domain("b", " EXAMPLE.test ", &token)
                .await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn operation(index: usize) -> (&'static str, &'static str) {
    match index {
        0 => ("GET", "/api/boards/b/cardSettings"),
        1 => ("PUT", "/api/boards/b/cardSettings"),
        2 => ("GET", "/api/boards/b/domains"),
        3 => ("POST", "/api/boards/b/domains"),
        4 => ("DELETE", "/api/boards/b/domains/%20EXAMPLE.test%20"),
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn all_five_operations_use_native_http_contracts() {
    for index in 0..5 {
        let server = MockServer::start().await;
        let (verb, url) = operation(index);
        let response = if index < 2 {
            settings_document()
        } else {
            json!([{"domain":"example.test","isActive":false}])
        };
        let mut mock = Mock::given(method(verb))
            .and(path(url))
            .and(header("authorization", "Bearer settings-token"))
            .and(header("accept", "application/json"));
        if index == 1 {
            mock = mock.and(body_json(
                json!({"allowsAttachments":false,"cardAgingDays1":0}),
            ));
        }
        if index == 3 {
            mock = mock.and(body_json(json!({"domain":" EXAMPLE.test "})));
        }
        mock.respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        invoke(&client(&server), index).await.unwrap();
        let requests = server.received_requests().await.unwrap();
        if matches!(index, 0 | 2 | 4) {
            assert!(requests[0].body.is_empty());
        }
    }
}

#[test]
fn every_settings_response_field_maps_without_losing_stored_numbers() {
    let document = settings_document();
    let settings: BoardCardSettings = serde_json::from_value(document.clone()).unwrap();
    let output = serde_json::to_value(settings).unwrap();
    assert_eq!(output.as_object().unwrap().len(), 47);
    for (wire, snake, _) in FIELDS {
        assert_eq!(output[*snake], document[*wire], "{wire}");
    }
    let sparse: BoardCardSettings = serde_json::from_value(json!({})).unwrap();
    assert!(
        serde_json::to_value(sparse)
            .unwrap()
            .as_object()
            .unwrap()
            .values()
            .all(Value::is_null)
    );
}

#[tokio::test]
async fn settings_reads_and_writes_reject_unknown_null_and_incorrect_fields() {
    let mut invalid = vec![
        json!({"unmapped":true}),
        json!({"allowsAttachments":{"nested":true}}),
    ];
    for (index, (wire, _, _)) in FIELDS.iter().enumerate() {
        invalid.push(json!({*wire: null}));
        invalid.push(json!({*wire: if index < 44 { json!(1) } else { json!("7") }}));
    }
    for index in 0..2 {
        let server = MockServer::start().await;
        for response in &invalid {
            server.reset().await;
            let (verb, url) = operation(index);
            Mock::given(method(verb))
                .and(path(url))
                .respond_with(ResponseTemplate::new(200).set_body_json(response))
                .mount(&server)
                .await;
            assert!(
                matches!(
                    invoke(&client(&server), index).await,
                    Err(ClientError::Protocol {
                        success_status_received: true,
                        ..
                    })
                ),
                "{response}"
            );
        }
    }
}

#[tokio::test]
async fn every_domain_operation_requires_strict_nested_domain_records() {
    for index in 2..5 {
        let server = MockServer::start().await;
        for response in [
            json!([{"domain":"x.test","isActive":true,"unmapped":{}}]),
            json!([{"domain":"x.test"}]),
            json!([{"isActive":true}]),
            json!([{"domain":null,"isActive":true}]),
            json!([{"domain":"x.test","isActive":"true"}]),
            json!([null]),
            json!({"domains":[]}),
        ] {
            server.reset().await;
            let (verb, url) = operation(index);
            Mock::given(method(verb))
                .and(path(url))
                .respond_with(ResponseTemplate::new(200).set_body_json(&response))
                .mount(&server)
                .await;
            assert!(
                matches!(
                    invoke(&client(&server), index).await,
                    Err(ClientError::Protocol {
                        success_status_received: true,
                        ..
                    })
                ),
                "{response}"
            );
        }
    }
}

#[tokio::test]
async fn every_operation_preserves_embedded_and_http_errors() {
    for index in 0..5 {
        let server = MockServer::start().await;
        let (verb, url) = operation(index);
        for (status, body) in [
            (
                200,
                json!({"error":"not-authorized","reason":"You must be logged in to perform this action.","message":"denied [not-authorized]","errorType":"Meteor.Error","isClientSafe":true}),
            ),
            (
                200,
                json!({"error":"Forbidden","reason":"Forbidden","statusCode":403}),
            ),
            (404, json!({"error":"Board not found"})),
        ] {
            server.reset().await;
            Mock::given(method(verb))
                .and(path(url))
                .respond_with(ResponseTemplate::new(status).set_body_json(body))
                .mount(&server)
                .await;
            let error = invoke(&client(&server), index).await.unwrap_err();
            match status {
                200 => assert!(matches!(
                    error,
                    ClientError::EmbeddedProtocol { .. } | ClientError::EmbeddedServer { .. }
                )),
                _ => assert!(matches!(error, ClientError::Server { .. })),
            }
        }
    }
}

#[tokio::test]
async fn removal_encodes_one_domain_segment_and_rejects_dot_only_targets() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/api/boards/b/domains/x%2Fy%3Fz%23p%25.test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .expect(1)
        .mount(&server)
        .await;
    client(&server)
        .remove_board_domain(
            "b",
            "x/y?z#p%.test",
            &SecretString::from("token".to_owned()),
        )
        .await
        .unwrap();
    for domain in ["", ".", ".."] {
        assert!(matches!(
            client(&server)
                .remove_board_domain("b", domain, &SecretString::from("token".to_owned()))
                .await,
            Err(ClientError::Protocol {
                success_status_received: false,
                ..
            })
        ));
    }
}
