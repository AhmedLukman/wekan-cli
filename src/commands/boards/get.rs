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

fn board_detail_member(member: BoardMember) -> BoardDetailMember {
    BoardDetailMember {
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
