use super::support::*;
use serde_json::{Value, json};

type LiveApp = App<CapturingStore, FixedPassword, FileProfileStore>;

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

async fn run(app: &LiveApp, server: &str, args: &[&str]) -> Result<Value, AppError> {
    let result = execute_live_board(app, server, args).await?;
    Ok(
        serde_json::from_str::<Value>(&wekan_cli::output::render_success(
            OutputFormat::Json,
            &result,
        ))
        .unwrap()["data"]
            .clone(),
    )
}

fn mongo(script: &str) -> Value {
    let result = Command::new("docker")
        .args([
            "compose", "exec", "-T", "mongodb", "mongosh", "--quiet", "--eval", script,
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "fixture operation failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).expect("fixture operation must return JSON")
}

fn stored_board(board: &str) -> Value {
    mongo(&format!(
        "print(JSON.stringify(db.getSiblingDB('wekan').boards.findOne({{_id:{}}})));",
        json!(board)
    ))
}

fn seed_board(board: &str, update: Value) {
    mongo(&format!(
        "const r=db.getSiblingDB('wekan').boards.updateOne({{_id:{}}},{});if(r.matchedCount!==1)throw new Error('fixture board missing');print(JSON.stringify({{ok:true}}));",
        json!(board),
        update
    ));
}

async fn wait_for_domains(app: &LiveApp, server: &str, board: &str, expected: &Value) {
    for _ in 0..50 {
        let output = run(app, server, &["domain", "list", "--board", board])
            .await
            .unwrap();
        if output["domains"] == *expected {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("fixture domain changes did not reach the REST projection");
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v12.57 stack and WEKAN_BOARD_SETTINGS_E2E_URL"]
async fn complete_board_settings_and_domain_lifecycle_matches_wekan_v12_57() {
    let server = env::var("WEKAN_BOARD_SETTINGS_E2E_URL")
        .expect("set WEKAN_BOARD_SETTINGS_E2E_URL to a fresh isolated v12.57 stack");
    let canonical = ServerUrl::parse(&server).unwrap().as_str().to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let username = format!("wekan_cli_settings_{nonce}");
    let directory = TestDirectory::new("board-settings-lifecycle");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Settings-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let registration = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "auth",
        "register",
        "--username",
        &username,
        "--email",
        &format!("{username}@example.test"),
        "--password-stdin",
    ])
    .unwrap();
    let CommandSuccess::Registration(registration) =
        app.execute(registration.command).await.unwrap()
    else {
        panic!("expected registration");
    };
    let token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let board = run(
        &app,
        &server,
        &["create", "--title", "Settings and domains"],
    )
    .await
    .unwrap()["board_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_normal_dir, normal, _) =
        register_live_board_role(&server, &canonical, &token, &board, nonce, "normal").await;
    let (_reader_dir, reader, _) =
        register_live_board_role(&server, &canonical, &token, &board, nonce, "readonly").await;
    let (_owner_dir, owner, _) =
        register_live_board_role(&server, &canonical, &token, &board, nonce, "admin").await;
    let (_outsider_dir, outsider, outsider_id) =
        register_live_board_role(&server, &canonical, &token, &board, nonce + 1, "normal").await;
    run(
        &app,
        &server,
        &["member", "remove", &outsider_id, "--board", &board],
    )
    .await
    .unwrap();
    assert!(!registration.user_id.is_empty());

    let baseline = run(&app, &server, &["card-settings", "get", "--board", &board])
        .await
        .unwrap();
    assert_eq!(baseline["settings"].as_object().unwrap().len(), 114);
    let saved = stored_board(&board);
    for (wire, snake, _) in FIELDS {
        assert_eq!(baseline["settings"][*snake], saved[*wire], "{wire}");
    }

    // Toggle every flag both ways; verify the native response and independent persisted state.
    for pass in 0..2 {
        let mut args = vec![
            "card-settings".to_owned(),
            "update".into(),
            "--board".into(),
            board.clone(),
        ];
        let mut expected = serde_json::Map::new();
        for (i, (wire, snake, flag)) in FIELDS.iter().enumerate() {
            let value = if !(44..47).contains(&i) {
                json!((i + pass) % 2 == 0)
            } else if i == 44 {
                json!(0)
            } else if i == 45 {
                json!(28)
            } else {
                json!(9_007_199_254_740_991_u64)
            };
            args.extend([(*flag).to_owned(), value.to_string()]);
            expected.insert((*wire).to_owned(), json!({"snake":snake,"value":value}));
        }
        let argv: Vec<_> = args.iter().map(String::as_str).collect();
        let output = run(&owner, &server, &argv).await.unwrap();
        assert_eq!(output["submitted_fields"].as_array().unwrap().len(), 114);
        let saved = stored_board(&board);
        for (wire, entry) in expected {
            assert_eq!(saved[&wire], entry["value"], "{wire}");
            assert_eq!(
                output["settings"][entry["snake"].as_str().unwrap()],
                entry["value"],
                "{wire}"
            );
        }
    }

    // An ordinary setting remains editable after every optional native flag
    // has been populated. Verify the persisted values independently of PUT.
    let before = run(&app, &server, &["card-settings", "get", "--board", &board])
        .await
        .unwrap()["settings"]
        .clone();
    let changed = run(
        &owner,
        &server,
        &[
            "card-settings",
            "update",
            "--board",
            &board,
            "--allows-attachments",
            "false",
        ],
    )
    .await
    .unwrap();
    let saved = stored_board(&board);
    for (wire, snake, _) in FIELDS {
        let expected = if *snake == "allows_attachments" {
            json!(false)
        } else {
            before[*snake].clone()
        };
        assert_eq!(changed["settings"][*snake], expected, "{wire}");
        assert_eq!(saved[*wire], expected, "{wire} must remain persisted");
    }
    run(
        &reader,
        &server,
        &["card-settings", "get", "--board", &board],
    )
    .await
    .unwrap();
    let denied = run(
        &reader,
        &server,
        &[
            "card-settings",
            "update",
            "--board",
            &board,
            "--card-aging",
            "true",
        ],
    )
    .await
    .unwrap_err();
    assert_eq!(denied.code(), ErrorCode::PermissionDenied);
    assert_eq!(denied.details().http_status, Some(403));
    let denied = run(
        &outsider,
        &server,
        &["card-settings", "get", "--board", &board],
    )
    .await
    .unwrap_err();
    assert_eq!(denied.code(), ErrorCode::PermissionDenied);

    for args in [
        vec!["card-settings", "get", "--board", "missing-board-settings"],
        vec![
            "card-settings",
            "update",
            "--board",
            "missing-board-settings",
            "--card-aging",
            "true",
        ],
        vec!["domain", "list", "--board", "missing-board-settings"],
        vec![
            "domain",
            "add",
            "example.test",
            "--board",
            "missing-board-settings",
        ],
        vec![
            "domain",
            "remove",
            "example.test",
            "--board",
            "missing-board-settings",
        ],
    ] {
        let error = run(&app, &server, &args).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::NotFound, "{args:?}: {error:?}");
        assert_eq!(error.details().http_status, Some(404));
    }

    assert_eq!(
        run(&app, &server, &["domain", "list", "--board", &board])
            .await
            .unwrap()["domains"],
        json!([])
    );
    let (_domain_reader_dir, domain_reader, _) =
        register_live_board_role(&server, &canonical, &token, &board, nonce + 2, "readonly").await;
    for user in [&normal, &domain_reader, &owner] {
        let denied = run(user, &server, &["domain", "list", "--board", &board])
            .await
            .unwrap_err();
        assert_eq!(denied.code(), ErrorCode::PermissionDenied);
        assert_eq!(denied.details().http_status, Some(403));
        assert_eq!(denied.details().wekan_status_code, None);
    }

    // v12.57 REST cannot seed sharing: setBoardDomains receives no DDP user identity.
    // Seed only this isolated test's board, then read through the production strict decoder.
    let domains = json!([{"domain":"example.test","isActive":true},{"domain":"inactive.test","isActive":false}]);
    seed_board(&board, json!({"$set":{"domains":domains}}));
    let expected = json!([{"domain":"example.test","is_active":true},{"domain":"inactive.test","is_active":false}]);
    wait_for_domains(&app, &server, &board, &expected).await;
    for user in [&app, &owner] {
        for (action, domain) in [
            ("add", " New.Example.test "),
            ("remove", " EXAMPLE.test "),
            ("remove", "absent.test"),
            ("add", "example.test"),
        ] {
            let error = run(
                user,
                &server,
                &["domain", action, domain, "--board", &board],
            )
            .await
            .unwrap_err();
            assert_eq!(error.code(), ErrorCode::ServerError, "{error:?}");
            assert_eq!(error.exit_code() as u8, 5);
            assert_eq!(error.details().http_status, Some(500));
            assert_eq!(
                error.details().server_error.as_deref(),
                Some("Internal server error")
            );
            assert_eq!(stored_board(&board)["domains"], domains);
            assert_eq!(
                run(&app, &server, &["domain", "list", "--board", &board])
                    .await
                    .unwrap()["domains"],
                expected
            );
        }
    }
    for domain in ["", "invalid", "a@b.test", "has space.test"] {
        let error = run(&app, &server, &["domain", "add", domain, "--board", &board])
            .await
            .unwrap_err();
        assert_eq!(error.details().http_status, Some(400));
        assert_eq!(stored_board(&board)["domains"], domains);
    }
    for action in ["add", "remove"] {
        let denied = run(
            &normal,
            &server,
            &["domain", action, "example.test", "--board", &board],
        )
        .await
        .unwrap_err();
        assert_eq!(denied.code(), ErrorCode::PermissionDenied);
        assert_eq!(denied.details().http_status, Some(403));
    }

    // Model Number fields allow persisted fractions and negatives, unlike typed REST writes.
    seed_board(
        &board,
        json!({"$set":{"cardAgingDays1":-0.5,"cardAgingDays2":1.25}}),
    );
    for _ in 0..50 {
        let settings = run(&app, &server, &["card-settings", "get", "--board", &board])
            .await
            .unwrap();
        if settings["settings"]["card_aging_days1"] == json!(-0.5)
            && settings["settings"]["card_aging_days2"] == json!(1.25)
        {
            run(&app, &server, &["delete", &board, "--yes"])
                .await
                .unwrap();
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!(
        "stored fractional and negative thresholds must be returned by strict response decoding"
    );
}
