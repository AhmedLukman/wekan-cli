use super::{FakeCredentialStore, board_command, factory};
use crate::{
    cli::Cli,
    commands::boards,
    error::ErrorCode,
    input::FakeConfirmationProvider,
    output::{OutputFormat, render_success},
};
use clap::Parser;
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

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
    (
        "allowsSwimlaneDragging",
        "allows_swimlane_dragging",
        "--allows-swimlane-dragging",
    ),
    (
        "allowsListDragging",
        "allows_list_dragging",
        "--allows-list-dragging",
    ),
    (
        "allowsCardDragging",
        "allows_card_dragging",
        "--allows-card-dragging",
    ),
    (
        "allowsChecklistDragging",
        "allows_checklist_dragging",
        "--allows-checklist-dragging",
    ),
    (
        "allowsChecklistItemDragging",
        "allows_checklist_item_dragging",
        "--allows-checklist-item-dragging",
    ),
    (
        "allowsSubtaskDragging",
        "allows_subtask_dragging",
        "--allows-subtask-dragging",
    ),
    (
        "allowsMinicardCollapse",
        "allows_minicard_collapse",
        "--allows-minicard-collapse",
    ),
    (
        "labelsAboveTitleOnMinicard",
        "labels_above_title_on_minicard",
        "--labels-above-title-on-minicard",
    ),
    (
        "allowsChecklistDueDate",
        "allows_checklist_due_date",
        "--allows-checklist-due-date",
    ),
    (
        "allowsChecklistTitle",
        "allows_checklist_title",
        "--allows-checklist-title",
    ),
    (
        "askForMoveReason",
        "ask_for_move_reason",
        "--ask-for-move-reason",
    ),
    (
        "allowsCommentsOnMinicard",
        "allows_comments_on_minicard",
        "--allows-comments-on-minicard",
    ),
    (
        "allowsSpentTime",
        "allows_spent_time",
        "--allows-spent-time",
    ),
    (
        "allowsSpentTimeOnMinicard",
        "allows_spent_time_on_minicard",
        "--allows-spent-time-on-minicard",
    ),
    (
        "allowsSwimlaneNameOnMinicard",
        "allows_swimlane_name_on_minicard",
        "--allows-swimlane-name-on-minicard",
    ),
    ("showLabelText", "show_label_text", "--show-label-text"),
    ("allowsStickers", "allows_stickers", "--allows-stickers"),
    (
        "allowsStickersOnMinicard",
        "allows_stickers_on_minicard",
        "--allows-stickers-on-minicard",
    ),
    ("allowsLocation", "allows_location", "--allows-location"),
    (
        "allowsDependencies",
        "allows_dependencies",
        "--allows-dependencies",
    ),
    (
        "allowsDependenciesOnMinicard",
        "allows_dependencies_on_minicard",
        "--allows-dependencies-on-minicard",
    ),
    ("allowsFlowtime", "allows_flowtime", "--allows-flowtime"),
    ("allowsPomodoro", "allows_pomodoro", "--allows-pomodoro"),
    ("allowsVote", "allows_vote", "--allows-vote"),
    (
        "allowsVoteOnMinicard",
        "allows_vote_on_minicard",
        "--allows-vote-on-minicard",
    ),
    ("allowsPoker", "allows_poker", "--allows-poker"),
    (
        "allowsPokerOnMinicard",
        "allows_poker_on_minicard",
        "--allows-poker-on-minicard",
    ),
    (
        "allowsTextNotes",
        "allows_text_notes",
        "--allows-text-notes",
    ),
    (
        "allowsCommentCountOnMinicard",
        "allows_comment_count_on_minicard",
        "--allows-comment-count-on-minicard",
    ),
    (
        "allowsIssueTypeOnMinicard",
        "allows_issue_type_on_minicard",
        "--allows-issue-type-on-minicard",
    ),
    (
        "allowsCardColorOnCard",
        "allows_card_color_on_card",
        "--allows-card-color-on-card",
    ),
    (
        "allowsCardColorOnMinicard",
        "allows_card_color_on_minicard",
        "--allows-card-color-on-minicard",
    ),
    (
        "allowsParentCardOnCard",
        "allows_parent_card_on_card",
        "--allows-parent-card-on-card",
    ),
    (
        "allowsParentCardOnMinicard",
        "allows_parent_card_on_minicard",
        "--allows-parent-card-on-minicard",
    ),
    (
        "allowsLinkedCardOnCard",
        "allows_linked_card_on_card",
        "--allows-linked-card-on-card",
    ),
    (
        "allowsLinkedCardOnMinicard",
        "allows_linked_card_on_minicard",
        "--allows-linked-card-on-minicard",
    ),
    (
        "allowsCardButtonsOnCard",
        "allows_card_buttons_on_card",
        "--allows-card-buttons-on-card",
    ),
    (
        "allowsDescriptionBadgeOnMinicard",
        "allows_description_badge_on_minicard",
        "--allows-description-badge-on-minicard",
    ),
    (
        "allowsUnreadCommentsOnMinicard",
        "allows_unread_comments_on_minicard",
        "--allows-unread-comments-on-minicard",
    ),
    (
        "allowsCardCollapse",
        "allows_card_collapse",
        "--allows-card-collapse",
    ),
    (
        "allowsChecklistTitleOnMinicard",
        "allows_checklist_title_on_minicard",
        "--allows-checklist-title-on-minicard",
    ),
    (
        "allowsLabelTextOnCard",
        "allows_label_text_on_card",
        "--allows-label-text-on-card",
    ),
    (
        "allowsCardButtonsOnMinicard",
        "allows_card_buttons_on_minicard",
        "--allows-card-buttons-on-minicard",
    ),
    (
        "allowsDescriptionBadgeOnCard",
        "allows_description_badge_on_card",
        "--allows-description-badge-on-card",
    ),
    (
        "allowsUnreadCommentsOnCard",
        "allows_unread_comments_on_card",
        "--allows-unread-comments-on-card",
    ),
    (
        "allowsDueFromTitleOnCard",
        "allows_due_from_title_on_card",
        "--allows-due-from-title-on-card",
    ),
    (
        "allowsDueFromTitleOnMinicard",
        "allows_due_from_title_on_minicard",
        "--allows-due-from-title-on-minicard",
    ),
    (
        "allowsSimilarCardsOnCard",
        "allows_similar_cards_on_card",
        "--allows-similar-cards-on-card",
    ),
    (
        "allowsSimilarCardsOnMinicard",
        "allows_similar_cards_on_minicard",
        "--allows-similar-cards-on-minicard",
    ),
    (
        "allowsAtRiskOnCard",
        "allows_at_risk_on_card",
        "--allows-at-risk-on-card",
    ),
    (
        "allowsAtRiskOnMinicard",
        "allows_at_risk_on_minicard",
        "--allows-at-risk-on-minicard",
    ),
    (
        "allowsSuggestedMembersOnCard",
        "allows_suggested_members_on_card",
        "--allows-suggested-members-on-card",
    ),
    (
        "allowsForecastOnCard",
        "allows_forecast_on_card",
        "--allows-forecast-on-card",
    ),
    (
        "allowsForecastOnMinicard",
        "allows_forecast_on_minicard",
        "--allows-forecast-on-minicard",
    ),
    (
        "allowsChecklistFromDescriptionOnCard",
        "allows_checklist_from_description_on_card",
        "--allows-checklist-from-description-on-card",
    ),
    (
        "allowsActionItemsOnCard",
        "allows_action_items_on_card",
        "--allows-action-items-on-card",
    ),
    (
        "allowsWhiteboardOnCard",
        "allows_whiteboard_on_card",
        "--allows-whiteboard-on-card",
    ),
    (
        "labelsAboveTitleOnCard",
        "labels_above_title_on_card",
        "--labels-above-title-on-card",
    ),
    (
        "allowsChecklistDueDateOnMinicard",
        "allows_checklist_due_date_on_minicard",
        "--allows-checklist-due-date-on-minicard",
    ),
    (
        "allowsLocationOnMinicard",
        "allows_location_on_minicard",
        "--allows-location-on-minicard",
    ),
    (
        "allowsSwimlaneNameOnCard",
        "allows_swimlane_name_on_card",
        "--allows-swimlane-name-on-card",
    ),
    (
        "allowsFlowtimeOnMinicard",
        "allows_flowtime_on_minicard",
        "--allows-flowtime-on-minicard",
    ),
    (
        "allowsPomodoroOnMinicard",
        "allows_pomodoro_on_minicard",
        "--allows-pomodoro-on-minicard",
    ),
    (
        "allowsAttachmentListOnMinicard",
        "allows_attachment_list_on_minicard",
        "--allows-attachment-list-on-minicard",
    ),
    (
        "allowsTextNotesOnMinicard",
        "allows_text_notes_on_minicard",
        "--allows-text-notes-on-minicard",
    ),
    (
        "allowsCommentCountOnCard",
        "allows_comment_count_on_card",
        "--allows-comment-count-on-card",
    ),
    (
        "allowsActivitiesOnMinicard",
        "allows_activities_on_minicard",
        "--allows-activities-on-minicard",
    ),
];

async fn run(
    server: &MockServer,
    args: &[&str],
) -> Result<crate::command_result::CommandSuccess, crate::error::AppError> {
    boards::dispatch(
        board_command(args),
        &factory(server),
        &FakeCredentialStore::authenticated(server),
        &crate::credentials::SystemSecretInputProvider,
        &FakeConfirmationProvider::declining(),
    )
    .await
}

#[tokio::test]
async fn every_setting_flag_sends_its_exact_field_and_reports_submission() {
    let server = MockServer::start().await;
    for (index, (wire, snake, flag)) in FIELDS.iter().enumerate() {
        for value in if !(44..47).contains(&index) {
            vec!["true", "false"]
        } else {
            vec!["0", "9007199254740991"]
        } {
            server.reset().await;
            let value_json: Value = serde_json::from_str(value).unwrap();
            Mock::given(method("PUT"))
                .and(path("/api/boards/b/cardSettings"))
                .and(header("authorization", "Bearer board-token"))
                .and(body_json(json!({*wire:value_json})))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({*wire:value_json})))
                .expect(1)
                .mount(&server)
                .await;
            let result = run(
                &server,
                &["card-settings", "update", "--board", "b", flag, value],
            )
            .await
            .unwrap();
            let output: Value =
                serde_json::from_str(&render_success(OutputFormat::Json, &result)).unwrap();
            assert_eq!(output["data"]["settings"][*snake], value_json);
            assert_eq!(output["data"]["submitted_fields"], json!([snake]));
        }
    }
}

#[tokio::test]
async fn settings_updates_preserve_optional_native_flags_and_omissions() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(body_json(
            json!({"allowsAttachments":false,"cardAging":true,"cardAgingDays1":0}),
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                json!({"allowsAttachments":false,"cardAging":true,"cardAgingDays1":0,"allowsDueFromTitleOnCard":false}),
            ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let result = run(
        &server,
        &[
            "card-settings",
            "update",
            "--board",
            "b",
            "--card-aging-days1",
            "0",
            "--allows-attachments",
            "false",
            "--card-aging",
            "true",
        ],
    )
    .await
    .unwrap();
    let output: Value = serde_json::from_str(&render_success(OutputFormat::Json, &result)).unwrap();
    assert_eq!(
        output["data"]["submitted_fields"],
        json!(["card_aging", "allows_attachments", "card_aging_days1"])
    );
    assert_eq!(output["data"]["settings"].as_object().unwrap().len(), 114);
    assert!(output["data"]["settings"]["allows_members"].is_null());
    assert_eq!(
        output["data"]["settings"]["allows_due_from_title_on_card"],
        false
    );
    let human = render_success(OutputFormat::Human, &result);
    assert!(human.contains("allows_attachments: false"));
    assert!(human.contains("allows_members: null"));
    assert!(human.contains("card_aging_days1: 0"));
    assert!(human.contains("allows_due_from_title_on_card: false"));
}

#[test]
fn settings_and_domain_parser_enforce_the_selected_interface() {
    let parse = |args: &[&str]| {
        let mut argv = vec!["wekan", "board"];
        argv.extend_from_slice(args);
        Cli::try_parse_from(argv)
    };
    for args in [
        vec!["card-settings", "get"],
        vec!["card-settings", "update", "--board", "b"],
        vec!["domain", "list"],
        vec!["domain", "add", "x.test"],
        vec!["domain", "remove", "x.test"],
        vec![
            "card-settings",
            "update",
            "--board",
            "b",
            "--allows-attachments",
        ],
        vec![
            "card-settings",
            "update",
            "--board",
            "b",
            "--allows-attachments",
            "1",
        ],
        vec![
            "card-settings",
            "update",
            "--board",
            "b",
            "--allows-attachments",
            "null",
        ],
        vec![
            "card-settings",
            "update",
            "--board",
            "b",
            "--allows-attachments",
            "TRUE",
        ],
    ] {
        assert!(parse(&args).is_err(), "{args:?}");
    }
    for (_, _, flag) in &FIELDS[44..47] {
        for value in [
            "-1",
            "1.5",
            "NaN",
            "1e3",
            "9007199254740992",
            "18446744073709551616",
        ] {
            assert!(parse(&["card-settings", "update", "--board", "b", flag, value]).is_err());
        }
    }
    for domain in ["", ".", ".."] {
        assert!(parse(&["domain", "remove", domain, "--board", "b"]).is_err());
    }
    for board in ["", ".", "..", "  "] {
        assert!(parse(&["card-settings", "get", "--board", board]).is_err());
    }
    assert!(
        parse(&[
            "card-settings",
            "update",
            "--board",
            "b",
            "--card-aging-days1",
            "28",
            "--card-aging-days2",
            "0"
        ])
        .is_ok()
    );
    assert!(parse(&["domain", "remove", "x.test", "--board", "b", "--yes"]).is_err());
}

#[tokio::test]
async fn domain_output_preserves_records_and_escapes_controls_without_claiming_effects() {
    let server = MockServer::start().await;
    let domains = json!([{"domain":"evil\n\u{1b}.test","isActive":false},{"domain":"other.test","isActive":true}]);
    for (args, verb) in [
        (vec!["domain", "list", "--board", "b"], "GET"),
        (
            vec!["domain", "add", " Example.test ", "--board", "b"],
            "POST",
        ),
        (
            vec!["domain", "remove", " Example.test ", "--board", "b"],
            "DELETE",
        ),
    ] {
        server.reset().await;
        Mock::given(method(verb))
            .respond_with(ResponseTemplate::new(200).set_body_json(&domains))
            .expect(1)
            .mount(&server)
            .await;
        let result = run(&server, &args).await.unwrap();
        let output: Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &result)).unwrap();
        assert_eq!(
            output["data"]["domains"],
            json!([{"domain":"evil\n\u{1b}.test","is_active":false},{"domain":"other.test","is_active":true}])
        );
        if verb != "GET" {
            assert_eq!(output["data"]["domain"], " Example.test ");
            assert_eq!(
                output["data"]["action"],
                if verb == "POST" { "add" } else { "remove" }
            );
        }
        let human = render_success(OutputFormat::Human, &result);
        assert!(!human.contains('\u{1b}'));
        assert!(human.contains("evil\\n\\u{1b}.test"));
        assert!(human.contains("is_active: false"));
        assert!(!human.contains("Added") && !human.contains("Removed"));
    }
}

#[tokio::test]
async fn pinned_domain_defect_is_an_exit_five_protocol_error_with_native_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error":"not-authorized", "reason":"You must be logged in to perform this action.",
            "message":"You must be logged in to perform this action. [not-authorized]",
            "errorType":"Meteor.Error", "isClientSafe":true
        })))
        .expect(1)
        .mount(&server)
        .await;
    let error = run(&server, &["domain", "add", "example.test", "--board", "b"])
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ProtocolError);
    assert_eq!(error.exit_code() as u8, 5);
    assert_eq!(error.details().http_status, Some(200));
    assert_eq!(
        error.details().server_error.as_deref(),
        Some("not-authorized")
    );
    assert_eq!(
        error.details().server_reason.as_deref(),
        Some("You must be logged in to perform this action.")
    );
}

#[tokio::test]
async fn reads_and_mutations_preserve_native_error_classification() {
    let server = MockServer::start().await;
    for args in [
        vec!["card-settings", "get", "--board", "b"],
        vec![
            "card-settings",
            "update",
            "--board",
            "b",
            "--card-aging",
            "true",
        ],
        vec!["domain", "list", "--board", "b"],
        vec!["domain", "add", "x.test", "--board", "b"],
        vec!["domain", "remove", "x.test", "--board", "b"],
    ] {
        for (status, body, code) in [
            (404, json!({"error":"Board not found"}), ErrorCode::NotFound),
            (403, json!({"error":"denied"}), ErrorCode::PermissionDenied),
            (
                401,
                json!({"error":"Unauthorized"}),
                ErrorCode::AuthenticationRejected,
            ),
            (
                200,
                json!({"statusCode":403,"error":"Forbidden","reason":"Forbidden"}),
                ErrorCode::PermissionDenied,
            ),
        ] {
            server.reset().await;
            Mock::given(wiremock::matchers::any())
                .respond_with(ResponseTemplate::new(status).set_body_json(body))
                .expect(1)
                .mount(&server)
                .await;
            let error = run(&server, &args).await.unwrap_err();
            assert_eq!(error.code(), code);
            assert_eq!(error.details().http_status, Some(status));
        }
    }
}
