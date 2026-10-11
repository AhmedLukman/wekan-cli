mod boards;
mod cards;
mod create;
mod current;
mod delete;
mod disable_login;
mod enable_login;
mod get;
mod list;
mod take_ownership;
mod update;

#[cfg(test)]
mod tests;

use clap::{Args, Subcommand};

use crate::{
    client::{UserRecord, WekanClientFactory},
    command_result::{
        CommandSuccess, UserBoardMembership, UserDetail, UserEmail, UserOrganization,
        UserProfileDetail, UserProfileNotificationDetail, UserProfileStarredPageDetail,
        UserSessionData, UserTeam, UserWorkspaceNodeDetail,
    },
    credentials::{CredentialStore, SecretInputProvider},
    error::AppError,
    input::ConfirmationProvider,
};

use super::client_error::map_client_error;

#[derive(Debug, Args)]
pub struct UserArgs {
    #[command(subcommand)]
    pub command: UserCommand,
}

#[derive(Debug, Subcommand)]
pub enum UserCommand {
    /// Show the currently authenticated Wekan user.
    Current(current::CurrentArgs),

    /// List cards related to the currently authenticated user.
    Cards(cards::CardsArgs),

    /// List Wekan users (site admin only).
    List(list::ListArgs),

    /// Show a Wekan user by ID or username (site admin only).
    Get(get::GetArgs),

    /// Submit Wekan's no-action update and return the unchanged user (site admin only).
    Update(update::UpdateArgs),

    /// Create a Wekan user (site admin only).
    Create(create::CreateArgs),

    /// List active boards visible for a user ID.
    Boards(boards::BoardsArgs),

    /// Transfer boards administered by a user to the current user.
    TakeOwnership(take_ownership::TakeOwnershipArgs),

    /// Disable a user's login and clear their sessions.
    DisableLogin(disable_login::DisableLoginArgs),

    /// Re-enable a user's login.
    EnableLogin(enable_login::EnableLoginArgs),

    /// Delete a Wekan user.
    Delete(delete::DeleteArgs),
}

pub(crate) async fn dispatch(
    command: UserCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    secret_input: &dyn SecretInputProvider,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        UserCommand::Current(args) => {
            current::execute(args, client_factory, credential_store).await
        }
        UserCommand::Cards(args) => cards::execute(args, client_factory, credential_store).await,
        UserCommand::List(args) => list::execute(args, client_factory, credential_store).await,
        UserCommand::Get(args) => get::execute(args, client_factory, credential_store).await,
        UserCommand::Update(args) => update::execute(args, client_factory, credential_store).await,
        UserCommand::Create(args) => {
            create::execute(args, client_factory, credential_store, secret_input).await
        }
        UserCommand::Boards(args) => boards::execute(args, client_factory, credential_store).await,
        UserCommand::TakeOwnership(args) => {
            take_ownership::execute(args, client_factory, credential_store, confirmation).await
        }
        UserCommand::DisableLogin(args) => {
            disable_login::execute(args, client_factory, credential_store, confirmation).await
        }
        UserCommand::EnableLogin(args) => {
            enable_login::execute(args, client_factory, credential_store).await
        }
        UserCommand::Delete(args) => {
            delete::execute(args, client_factory, credential_store, confirmation).await
        }
    }
}

fn user_detail(user: UserRecord) -> UserDetail {
    UserDetail {
        user_id: user.user_id,
        username: user.username,
        full_name: user.profile.fullname.clone(),
        profile: Box::new(user_profile(user.profile)),
        created_through_api: user.created_through_api,
        heartbeat: user.heartbeat,
        session_data: user.session_data.map(|data| UserSessionData {
            total_hits: data.total_hits,
        }),
        import_usernames: user.import_usernames,
        emails: user
            .emails
            .into_iter()
            .map(|email| UserEmail {
                address: email.address,
                verified: email.verified,
            })
            .collect(),
        is_admin: user.is_admin,
        login_disabled: user.login_disabled,
        authentication_method: user.authentication_method,
        created_at: user.created_at,
        modified_at: user.modified_at,
        last_connection_date: user.last_connection_date,
        organizations: user
            .orgs
            .into_iter()
            .map(|organization| UserOrganization {
                organization_id: organization.org_id,
                display_name: organization.org_display_name,
                is_admin: organization.is_admin,
            })
            .collect(),
        teams: user
            .teams
            .into_iter()
            .map(|team| UserTeam {
                team_id: team.team_id,
                display_name: team.team_display_name,
            })
            .collect(),
        boards: user
            .boards
            .into_iter()
            .map(|board| UserBoardMembership {
                board_id: board.board_id,
                is_active: board.is_active,
                is_admin: board.is_admin,
                is_no_comments: board.is_no_comments,
                is_comment_only: board.is_comment_only,
                is_worker: board.is_worker,
                is_normal_assigned_only: board.is_normal_assigned_only,
                is_comment_assigned_only: board.is_comment_assigned_only,
                is_read_only: board.is_read_only,
                is_read_assigned_only: board.is_read_assigned_only,
            })
            .collect(),
    }
}

fn non_empty(value: &str) -> Result<String, String> {
    if value.is_empty() {
        Err("value must not be empty".to_owned())
    } else {
        Ok(value.to_owned())
    }
}

fn user_workspace_node(node: crate::client::UserWorkspaceNode) -> UserWorkspaceNodeDetail {
    UserWorkspaceNodeDetail {
        id: node.id,
        name: node.name,
        icon: node.icon,
        children: node
            .children
            .map(|nodes| nodes.into_iter().map(user_workspace_node).collect()),
    }
}

fn user_profile(profile: crate::client::UserProfile) -> UserProfileDetail {
    UserProfileDetail {
        full_name: profile.fullname,
        avatar_url: profile.avatar_url,
        email_buffer: profile.email_buffer,
        show_desktop_drag_handles: profile.show_desktop_drag_handles,
        left_menu_collapsed: profile.left_menu_collapsed,
        collapsed_workspaces: profile.collapsed_workspaces,
        left_menu_width: profile.left_menu_width,
        submit_on_enter: profile.submit_on_enter,
        open_many_cards_at_once: profile.open_many_cards_at_once,
        all_boards_theme_tiles: profile.all_boards_theme_tiles,
        global_theme_color: profile.global_theme_color,
        global_theme_custom_colors: profile.global_theme_custom_colors,
        ui_font: profile.ui_font,
        ui_font_size: profile.ui_font_size,
        ui_text_color: profile.ui_text_color,
        ui_text_bg_color: profile.ui_text_bg_color,
        dismissed_announcement_version: profile.dismissed_announcement_version,
        card_maximized: profile.card_maximized,
        card_collapsed: profile.card_collapsed,
        show_activities: profile.show_activities,
        custom_fields_grid: profile.custom_fields_grid,
        trello_api_saved: profile.trello_api_saved,
        hidden_minicard_label_text: profile.hidden_minicard_label_text,
        initials: profile.initials,
        board_workspaces_tree: profile
            .board_workspaces_tree
            .map(|nodes| nodes.into_iter().map(user_workspace_node).collect()),
        board_workspace_assignments: profile.board_workspace_assignments,
        invited_boards: profile.invited_boards,
        language: profile.language,
        map_provider: profile.map_provider,
        move_and_copy_dialog: profile.move_and_copy_dialog,
        move_checklist_dialog: profile.move_checklist_dialog,
        copy_checklist_dialog: profile.copy_checklist_dialog,
        notifications: profile.notifications.map(|items| {
            items
                .into_iter()
                .map(|item| UserProfileNotificationDetail {
                    activity: item.activity,
                    read: item.read,
                })
                .collect()
        }),
        rescue_card_description: profile.rescue_card_description,
        show_cards_count_at: profile.show_cards_count_at,
        start_day_of_week: profile.start_day_of_week,
        starred_boards: profile.starred_boards,
        starred_pages: profile.starred_pages.map(|items| {
            items
                .into_iter()
                .map(|item| UserProfileStarredPageDetail {
                    url: item.url,
                    title: item.title,
                })
                .collect()
        }),
        default_board_id: profile.default_board_id,
        icode: profile.icode,
        board_view: profile.board_view,
        list_sort_by: profile.list_sort_by,
        all_boards_sort_by: profile.all_boards_sort_by,
        templates_board_id: profile.templates_board_id,
        card_templates_swimlane_id: profile.card_templates_swimlane_id,
        list_templates_swimlane_id: profile.list_templates_swimlane_id,
        board_templates_swimlane_id: profile.board_templates_swimlane_id,
        sidebar_width: profile.sidebar_width,
        list_widths: profile.list_widths,
        list_constraints: profile.list_constraints,
        auto_width_boards: profile.auto_width_boards,
        board_sort_index: profile.board_sort_index,
        fixed_list_width_boards: profile.fixed_list_width_boards,
        fixed_list_widths: profile.fixed_list_widths,
        swimlane_heights: profile.swimlane_heights,
        collapsed_lists: profile.collapsed_lists,
        collapsed_swimlanes: profile.collapsed_swimlanes,
        collapsed_card_sections: profile.collapsed_card_sections,
        keyboard_shortcuts: profile.keyboard_shortcuts,
        vertical_scrollbars: profile.vertical_scrollbars,
        show_week_of_year: profile.show_week_of_year,
        date_format: profile.date_format,
        mobile_mode: profile.mobile_mode,
        card_zoom: profile.card_zoom,
    }
}
