use clap::Args;

use crate::{
    client::{
        WekanClientFactory,
        boards::{
            BoardDocument, BoardDomain, BoardLabel, BoardMember, BoardOrganization, BoardTeam,
            BoardWatcher,
        },
    },
    command_result::{
        BoardDetail, BoardDetailDomain, BoardDetailLabel, BoardDetailMember,
        BoardDetailOrganization, BoardDetailTeam, BoardDetailWatcher, CommandSuccess,
    },
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::non_empty;

#[derive(Debug, Args)]
pub struct GetArgs {
    /// Wekan board ID.
    #[arg(value_parser = non_empty)]
    pub board_id: String,
}

pub(super) async fn execute(
    args: GetArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let board = context
        .client()
        .board(&args.board_id, context.record().token())
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "board lookup",
                "the requested Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::BoardShown(Box::new(board_detail(board))))
}

fn board_detail(board: BoardDocument) -> BoardDetail {
    BoardDetail {
        scrum: board.scrum,
        scrum_import_losses: board.scrum_import_losses,
        announcement: board.announcement,
        wip_limit_groups: board.wip_limit_groups,
        email_to_card: board.email_to_card,
        notification_delivery: board.notification_delivery,
        column_sets: board.column_sets,
        scrum_revision: board.scrum_revision,
        allows_instance_editing: board.allows_instance_editing,
        import_run_id: board.import_run_id,
        sync_effects_enabled: board.sync_effects_enabled,
        map_image_attachment_id: board.map_image_attachment_id,
        auto_archive_inactive_days: board.auto_archive_inactive_days,
        rules_visible_to_members: board.rules_visible_to_members,
        card_field_order: board.card_field_order,
        minicard_field_order: board.minicard_field_order,
        notify_override_tray: board.notify_override_tray,
        notify_override_email: board.notify_override_email,
        due_reminder_days: board.due_reminder_days,
        due_reminder_webhook: board.due_reminder_webhook,
        labels_above_title_on_card: board.labels_above_title_on_card,
        allows_checklist_due_date_on_minicard: board.allows_checklist_due_date_on_minicard,
        allows_location_on_minicard: board.allows_location_on_minicard,
        allows_swimlane_name_on_card: board.allows_swimlane_name_on_card,
        allows_flowtime_on_minicard: board.allows_flowtime_on_minicard,
        allows_pomodoro_on_minicard: board.allows_pomodoro_on_minicard,
        allows_attachment_list_on_minicard: board.allows_attachment_list_on_minicard,
        allows_text_notes_on_minicard: board.allows_text_notes_on_minicard,
        allows_comment_count_on_card: board.allows_comment_count_on_card,
        allows_activities_on_minicard: board.allows_activities_on_minicard,
        allows_card_buttons_on_minicard: board.allows_card_buttons_on_minicard,
        allows_description_badge_on_card: board.allows_description_badge_on_card,
        allows_unread_comments_on_card: board.allows_unread_comments_on_card,
        allows_due_from_title_on_card: board.allows_due_from_title_on_card,
        allows_due_from_title_on_minicard: board.allows_due_from_title_on_minicard,
        allows_similar_cards_on_card: board.allows_similar_cards_on_card,
        allows_similar_cards_on_minicard: board.allows_similar_cards_on_minicard,
        allows_at_risk_on_card: board.allows_at_risk_on_card,
        allows_at_risk_on_minicard: board.allows_at_risk_on_minicard,
        allows_suggested_members_on_card: board.allows_suggested_members_on_card,
        allows_forecast_on_card: board.allows_forecast_on_card,
        allows_forecast_on_minicard: board.allows_forecast_on_minicard,
        allows_checklist_from_description_on_card: board.allows_checklist_from_description_on_card,
        allows_action_items_on_card: board.allows_action_items_on_card,
        allows_whiteboard_on_card: board.allows_whiteboard_on_card,
        shared_list_headers: board.shared_list_headers,
        board_view_settings: board.board_view_settings,
        board_view_order: board.board_view_order,
        date_format_override: board.date_format_override,
        date_format: board.date_format,
        ask_for_move_reason: board.ask_for_move_reason,
        subtasks_default_swimlane_id: board.subtasks_default_swimlane_id,
        allows_swimlane_dragging: board.allows_swimlane_dragging,
        allows_list_dragging: board.allows_list_dragging,
        allows_card_dragging: board.allows_card_dragging,
        allows_checklist_dragging: board.allows_checklist_dragging,
        allows_checklist_item_dragging: board.allows_checklist_item_dragging,
        allows_subtask_dragging: board.allows_subtask_dragging,
        allows_minicard_collapse: board.allows_minicard_collapse,
        labels_above_title_on_minicard: board.labels_above_title_on_minicard,
        allows_checklist_due_date: board.allows_checklist_due_date,
        allows_checklist_title: board.allows_checklist_title,
        allows_comments_on_minicard: board.allows_comments_on_minicard,
        show_label_text: board.show_label_text,
        allows_swimlane_name_on_minicard: board.allows_swimlane_name_on_minicard,
        allows_spent_time: board.allows_spent_time,
        allows_spent_time_on_minicard: board.allows_spent_time_on_minicard,
        allows_stickers: board.allows_stickers,
        allows_stickers_on_minicard: board.allows_stickers_on_minicard,
        allows_location: board.allows_location,
        allows_dependencies: board.allows_dependencies,
        allows_dependencies_on_minicard: board.allows_dependencies_on_minicard,
        allows_flowtime: board.allows_flowtime,
        allows_pomodoro: board.allows_pomodoro,
        allows_vote: board.allows_vote,
        allows_vote_on_minicard: board.allows_vote_on_minicard,
        allows_poker: board.allows_poker,
        allows_poker_on_minicard: board.allows_poker_on_minicard,
        allows_text_notes: board.allows_text_notes,
        allows_comment_count_on_minicard: board.allows_comment_count_on_minicard,
        allows_card_collapse: board.allows_card_collapse,
        allows_checklist_title_on_minicard: board.allows_checklist_title_on_minicard,
        allows_label_text_on_card: board.allows_label_text_on_card,
        allows_card_color_on_card: board.allows_card_color_on_card,
        allows_card_color_on_minicard: board.allows_card_color_on_minicard,
        allows_parent_card_on_card: board.allows_parent_card_on_card,
        allows_parent_card_on_minicard: board.allows_parent_card_on_minicard,
        allows_linked_card_on_card: board.allows_linked_card_on_card,
        allows_linked_card_on_minicard: board.allows_linked_card_on_minicard,
        allows_card_buttons_on_card: board.allows_card_buttons_on_card,
        allows_description_badge_on_minicard: board.allows_description_badge_on_minicard,
        allows_unread_comments_on_minicard: board.allows_unread_comments_on_minicard,
        allows_issue_type_on_minicard: board.allows_issue_type_on_minicard,
        sticky_list_headers: board.sticky_list_headers,
        list_width_resize_locked: board.list_width_resize_locked,
        swimlane_height_resize_locked: board.swimlane_height_resize_locked,
        default_public_board_view: board.default_public_board_view,
        default_private_board_view: board.default_private_board_view,
        same_width_for_all_lists: board.same_width_for_all_lists,
        same_width_for_all_lists_value: board.same_width_for_all_lists_value,
        key: board.key,
        board_id: board.board_id,
        title: board.title,
        slug: board.slug,
        archived: board.archived,
        archived_at: board.archived_at,
        created_at: board.created_at,
        modified_at: board.modified_at,
        stars: board.stars,
        labels: board.labels.into_iter().map(board_detail_label).collect(),
        members: board.members.into_iter().map(board_detail_member).collect(),
        watchers: board
            .watchers
            .into_iter()
            .map(board_detail_watcher)
            .collect(),
        permission: board
            .permission
            .map(|permission| permission.as_str().to_owned()),
        orgs: board
            .orgs
            .into_iter()
            .map(board_detail_organization)
            .collect(),
        teams: board.teams.into_iter().map(board_detail_team).collect(),
        domains: board.domains.into_iter().map(board_detail_domain).collect(),
        import_usernames: board.import_usernames,
        color: board.color.map(|color| color.as_str().to_owned()),
        custom_theme_colors: board.custom_theme_colors,
        background_image_url: board.background_image_url,
        background_image_id: board.background_image_id,
        allows_card_counter_list: board.allows_card_counter_list,
        card_aging: board.card_aging,
        show_dependencies: board.show_dependencies,
        card_aging_days1: board.card_aging_days1,
        card_aging_days2: board.card_aging_days2,
        card_aging_days3: board.card_aging_days3,
        allows_board_member_list: board.allows_board_member_list,
        description: board.description,
        subtasks_default_board_id: board.subtasks_default_board_id,
        migration_version: board.migration_version,
        subtasks_default_list_id: board.subtasks_default_list_id,
        date_settings_default_board_id: board.date_settings_default_board_id,
        date_settings_default_list_id: board.date_settings_default_list_id,
        allows_subtasks: board.allows_subtasks,
        allows_subtasks_on_minicard: board.allows_subtasks_on_minicard,
        allows_attachments: board.allows_attachments,
        allows_attachments_on_minicard: board.allows_attachments_on_minicard,
        allows_checklists: board.allows_checklists,
        allows_checklists_on_minicard: board.allows_checklists_on_minicard,
        allows_custom_fields: board.allows_custom_fields,
        allows_custom_fields_on_minicard: board.allows_custom_fields_on_minicard,
        allows_checklist_count_badge_on_minicard: board.allows_checklist_count_badge_on_minicard,
        allows_comments: board.allows_comments,
        allows_description_title: board.allows_description_title,
        allows_description_title_on_minicard: board.allows_description_title_on_minicard,
        allows_description_text: board.allows_description_text,
        allows_description_text_on_minicard: board.allows_description_text_on_minicard,
        allows_cover_attachment_on_minicard: board.allows_cover_attachment_on_minicard,
        allows_cover_attachment_on_card: board.allows_cover_attachment_on_card,
        allows_badge_attachment_on_minicard: board.allows_badge_attachment_on_minicard,
        allows_attachment_count_on_card: board.allows_attachment_count_on_card,
        allows_checklist_count_badge_on_card: board.allows_checklist_count_badge_on_card,
        allows_card_sorting_by_number_on_minicard: board.allows_card_sorting_by_number_on_minicard,
        allows_card_number: board.allows_card_number,
        allows_card_number_on_minicard: board.allows_card_number_on_minicard,
        allows_activities: board.allows_activities,
        allows_labels: board.allows_labels,
        allows_labels_on_minicard: board.allows_labels_on_minicard,
        allows_creator: board.allows_creator,
        allows_creator_on_minicard: board.allows_creator_on_minicard,
        allows_assignee: board.allows_assignee,
        allows_assignee_on_minicard: board.allows_assignee_on_minicard,
        allows_members: board.allows_members,
        allows_members_on_minicard: board.allows_members_on_minicard,
        allows_requested_by: board.allows_requested_by,
        allows_requested_by_on_minicard: board.allows_requested_by_on_minicard,
        allows_card_sorting_by_number: board.allows_card_sorting_by_number,
        allows_show_lists: board.allows_show_lists,
        allows_assigned_by: board.allows_assigned_by,
        allows_assigned_by_on_minicard: board.allows_assigned_by_on_minicard,
        allows_show_lists_on_minicard: board.allows_show_lists_on_minicard,
        allows_checklist_at_minicard: board.allows_checklist_at_minicard,
        allows_received_date: board.allows_received_date,
        restrict_comment_editing: board.restrict_comment_editing,
        allows_personal_list_width: board.allows_personal_list_width,
        auto_width: board.auto_width,
        allows_received_date_on_minicard: board.allows_received_date_on_minicard,
        allows_start_date: board.allows_start_date,
        allows_start_date_on_minicard: board.allows_start_date_on_minicard,
        allows_end_date: board.allows_end_date,
        allows_end_date_on_minicard: board.allows_end_date_on_minicard,
        allows_due_date: board.allows_due_date,
        allows_due_date_on_minicard: board.allows_due_date_on_minicard,
        allows_due_complete: board.allows_due_complete,
        allows_due_complete_on_minicard: board.allows_due_complete_on_minicard,
        present_parent_task: board
            .present_parent_task
            .map(|presentation| presentation.as_str().to_owned()),
        received_at: board.received_at,
        start_at: board.start_at,
        due_at: board.due_at,
        end_at: board.end_at,
        spent_time: board.spent_time,
        is_overtime: board.is_overtime,
        board_type: board
            .board_type
            .map(|board_type| board_type.as_str().to_owned()),
        sort: board.sort,
        show_activities: board.show_activities,
    }
}

pub(super) fn board_detail_member(member: BoardMember) -> BoardDetailMember {
    BoardDetailMember {
        granted_by: member.granted_by,
        custom_role_id: member.custom_role_id,
        user_id: member.user_id,
        is_admin: member.is_admin,
        is_active: member.is_active,
        is_no_comments: member.is_no_comments,
        is_comment_only: member.is_comment_only,
        is_worker: member.is_worker,
        is_normal_assigned_only: member.is_normal_assigned_only,
        is_comment_assigned_only: member.is_comment_assigned_only,
        is_read_only: member.is_read_only,
        is_read_assigned_only: member.is_read_assigned_only,
    }
}

fn board_detail_watcher(watcher: BoardWatcher) -> BoardDetailWatcher {
    BoardDetailWatcher {
        user_id: watcher.user_id,
        level: watcher.level.as_str().to_owned(),
    }
}

fn board_detail_label(label: BoardLabel) -> BoardDetailLabel {
    BoardDetailLabel {
        due_at: label.due_at,
        shared_label_id: label.shared_label_id,
        label_id: label.label_id,
        name: label.name,
        color: label.color,
    }
}

fn board_detail_organization(organization: BoardOrganization) -> BoardDetailOrganization {
    BoardDetailOrganization {
        org_id: organization.org_id,
        org_display_name: organization.org_display_name,
        is_active: organization.is_active,
    }
}

fn board_detail_team(team: BoardTeam) -> BoardDetailTeam {
    BoardDetailTeam {
        team_id: team.team_id,
        team_display_name: team.team_display_name,
        is_active: team.is_active,
    }
}

fn board_detail_domain(domain: BoardDomain) -> BoardDetailDomain {
    BoardDetailDomain {
        domain: domain.domain,
        is_active: domain.is_active,
    }
}
