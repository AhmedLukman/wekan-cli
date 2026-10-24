use crate::{
    client::{WekanClientFactory, boards::card_settings::UpdateBoardCardSettingsRequest},
    command_result::{
        BoardCardSettingField as Field, BoardCardSettingsSuccess, BoardCardSettingsUpdateSuccess,
        CommandSuccess,
    },
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
        custom_fields::non_empty,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct CardSettingsArgs {
    #[command(subcommand)]
    pub command: CardSettingsCommand,
}

#[derive(Debug, Subcommand)]
pub enum CardSettingsCommand {
    /// Show every returned card display setting and aging threshold.
    Get(Target),
    /// Update selected settings; omitted fields are preserved by Wekan.
    Update(UpdateArgs),
}

#[derive(Debug, Args)]
pub struct Target {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    board_id: String,
}

#[derive(Debug, Args)]
#[command(group(clap::ArgGroup::new("settings").required(true).multiple(true).args([
    "allows_swimlane_dragging",
    "allows_list_dragging",
    "allows_card_dragging",
    "allows_checklist_dragging",
    "allows_checklist_item_dragging",
    "allows_subtask_dragging",
    "allows_minicard_collapse",
    "labels_above_title_on_minicard",
    "allows_checklist_due_date",
    "allows_checklist_title",
    "ask_for_move_reason",
    "allows_comments_on_minicard",
    "allows_spent_time",
    "allows_spent_time_on_minicard",
    "allows_swimlane_name_on_minicard",
    "show_label_text",
    "allows_stickers",
    "allows_stickers_on_minicard",
    "allows_location",
    "allows_dependencies",
    "allows_dependencies_on_minicard",
    "allows_flowtime",
    "allows_pomodoro",
    "allows_vote",
    "allows_vote_on_minicard",
    "allows_poker",
    "allows_poker_on_minicard",
    "allows_text_notes",
    "allows_comment_count_on_minicard",
    "allows_issue_type_on_minicard",
    "allows_card_color_on_card",
    "allows_card_color_on_minicard",
    "allows_parent_card_on_card",
    "allows_parent_card_on_minicard",
    "allows_linked_card_on_card",
    "allows_linked_card_on_minicard",
    "allows_card_buttons_on_card",
    "allows_description_badge_on_minicard",
    "allows_unread_comments_on_minicard",
    "allows_card_collapse",
    "allows_checklist_title_on_minicard",
    "allows_label_text_on_card",
    "allows_card_counter_list",
    "card_aging",
    "allows_board_member_list",
    "allows_show_lists",
    "allows_attachments",
    "allows_checklists",
    "allows_comments",
    "allows_description_title",
    "allows_description_text",
    "allows_activities",
    "allows_labels",
    "allows_creator",
    "allows_assignee",
    "allows_members",
    "allows_requested_by",
    "allows_card_sorting_by_number",
    "allows_card_number",
    "allows_assigned_by",
    "allows_received_date",
    "allows_start_date",
    "allows_end_date",
    "allows_due_date",
    "allows_subtasks",
    "allows_creator_on_minicard",
    "allows_attachments_on_minicard",
    "allows_checklists_on_minicard",
    "allows_checklist_at_minicard",
    "allows_cover_attachment_on_minicard",
    "allows_badge_attachment_on_minicard",
    "allows_card_sorting_by_number_on_minicard",
    "allows_card_number_on_minicard",
    "allows_description_title_on_minicard",
    "allows_description_text_on_minicard",
    "allows_labels_on_minicard",
    "allows_assignee_on_minicard",
    "allows_members_on_minicard",
    "allows_requested_by_on_minicard",
    "allows_assigned_by_on_minicard",
    "allows_received_date_on_minicard",
    "allows_start_date_on_minicard",
    "allows_end_date_on_minicard",
    "allows_due_date_on_minicard",
    "allows_subtasks_on_minicard",
    "allows_show_lists_on_minicard",
    "card_aging_days1",
    "card_aging_days2",
    "card_aging_days3",
    "allows_card_buttons_on_minicard",
    "allows_description_badge_on_card",
    "allows_unread_comments_on_card",
    "allows_due_from_title_on_card",
    "allows_due_from_title_on_minicard",
    "allows_similar_cards_on_card",
    "allows_similar_cards_on_minicard",
    "allows_at_risk_on_card",
    "allows_at_risk_on_minicard",
    "allows_suggested_members_on_card",
    "allows_forecast_on_card",
    "allows_forecast_on_minicard",
    "allows_checklist_from_description_on_card",
    "allows_action_items_on_card",
    "allows_whiteboard_on_card",
    "labels_above_title_on_card",
    "allows_checklist_due_date_on_minicard",
    "allows_location_on_minicard",
    "allows_swimlane_name_on_card",
    "allows_flowtime_on_minicard",
    "allows_pomodoro_on_minicard",
    "allows_attachment_list_on_minicard",
    "allows_text_notes_on_minicard",
    "allows_comment_count_on_card",
    "allows_activities_on_minicard",
])), after_help = "Supply at least one setting. Boolean flags require true or false; aging thresholds require nonnegative integer days. Raw API requests can expose Wekan's permissive coercions.")]
pub struct UpdateArgs {
    /// Update allows swimlane dragging.
    #[arg(long, help_heading = "Display controls")]
    allows_swimlane_dragging: Option<bool>,
    /// Update allows list dragging.
    #[arg(long, help_heading = "Display controls")]
    allows_list_dragging: Option<bool>,
    /// Update allows card dragging.
    #[arg(long, help_heading = "Display controls")]
    allows_card_dragging: Option<bool>,
    /// Update allows checklist dragging.
    #[arg(long, help_heading = "Display controls")]
    allows_checklist_dragging: Option<bool>,
    /// Update allows checklist item dragging.
    #[arg(long, help_heading = "Display controls")]
    allows_checklist_item_dragging: Option<bool>,
    /// Update allows subtask dragging.
    #[arg(long, help_heading = "Display controls")]
    allows_subtask_dragging: Option<bool>,
    /// Update allows minicard collapse.
    #[arg(long, help_heading = "Display controls")]
    allows_minicard_collapse: Option<bool>,
    /// Update labels above title on minicard.
    #[arg(long, help_heading = "Display controls")]
    labels_above_title_on_minicard: Option<bool>,
    /// Update allows checklist due date.
    #[arg(long, help_heading = "Display controls")]
    allows_checklist_due_date: Option<bool>,
    /// Update allows checklist title.
    #[arg(long, help_heading = "Display controls")]
    allows_checklist_title: Option<bool>,
    /// Update ask for move reason.
    #[arg(long, help_heading = "Display controls")]
    ask_for_move_reason: Option<bool>,
    /// Update allows comments on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_comments_on_minicard: Option<bool>,
    /// Update allows spent time.
    #[arg(long, help_heading = "Display controls")]
    allows_spent_time: Option<bool>,
    /// Update allows spent time on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_spent_time_on_minicard: Option<bool>,
    /// Update allows swimlane name on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_swimlane_name_on_minicard: Option<bool>,
    /// Update show label text.
    #[arg(long, help_heading = "Display controls")]
    show_label_text: Option<bool>,
    /// Update allows stickers.
    #[arg(long, help_heading = "Display controls")]
    allows_stickers: Option<bool>,
    /// Update allows stickers on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_stickers_on_minicard: Option<bool>,
    /// Update allows location.
    #[arg(long, help_heading = "Display controls")]
    allows_location: Option<bool>,
    /// Update allows dependencies.
    #[arg(long, help_heading = "Display controls")]
    allows_dependencies: Option<bool>,
    /// Update allows dependencies on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_dependencies_on_minicard: Option<bool>,
    /// Update allows flowtime.
    #[arg(long, help_heading = "Display controls")]
    allows_flowtime: Option<bool>,
    /// Update allows pomodoro.
    #[arg(long, help_heading = "Display controls")]
    allows_pomodoro: Option<bool>,
    /// Update allows vote.
    #[arg(long, help_heading = "Display controls")]
    allows_vote: Option<bool>,
    /// Update allows vote on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_vote_on_minicard: Option<bool>,
    /// Update allows poker.
    #[arg(long, help_heading = "Display controls")]
    allows_poker: Option<bool>,
    /// Update allows poker on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_poker_on_minicard: Option<bool>,
    /// Update allows text notes.
    #[arg(long, help_heading = "Display controls")]
    allows_text_notes: Option<bool>,
    /// Update allows comment count on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_comment_count_on_minicard: Option<bool>,
    /// Update allows issue type on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_issue_type_on_minicard: Option<bool>,
    /// Update allows card color on card.
    #[arg(long, help_heading = "Display controls")]
    allows_card_color_on_card: Option<bool>,
    /// Update allows card color on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_card_color_on_minicard: Option<bool>,
    /// Update allows parent card on card.
    #[arg(long, help_heading = "Display controls")]
    allows_parent_card_on_card: Option<bool>,
    /// Update allows parent card on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_parent_card_on_minicard: Option<bool>,
    /// Update allows linked card on card.
    #[arg(long, help_heading = "Display controls")]
    allows_linked_card_on_card: Option<bool>,
    /// Update allows linked card on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_linked_card_on_minicard: Option<bool>,
    /// Update allows card buttons on card.
    #[arg(long, help_heading = "Display controls")]
    allows_card_buttons_on_card: Option<bool>,
    /// Update allows description badge on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_description_badge_on_minicard: Option<bool>,
    /// Update allows unread comments on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_unread_comments_on_minicard: Option<bool>,
    /// Update allows card collapse.
    #[arg(long, help_heading = "Display controls")]
    allows_card_collapse: Option<bool>,
    /// Update allows checklist title on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_checklist_title_on_minicard: Option<bool>,
    /// Update allows label text on card.
    #[arg(long, help_heading = "Display controls")]
    allows_label_text_on_card: Option<bool>,

    #[command(flatten)]
    target: Target,
    /// Show card counters on lists.
    #[arg(long, help_heading = "Display controls")]
    allows_card_counter_list: Option<bool>,
    /// Enable or disable visual card aging.
    #[arg(long, help_heading = "Display controls")]
    card_aging: Option<bool>,
    /// Show the board member list.
    #[arg(long, help_heading = "Display controls")]
    allows_board_member_list: Option<bool>,
    /// Show list information.
    #[arg(long, help_heading = "Display controls")]
    allows_show_lists: Option<bool>,
    /// Show attachments.
    #[arg(long, help_heading = "Display controls")]
    allows_attachments: Option<bool>,
    /// Show checklists.
    #[arg(long, help_heading = "Display controls")]
    allows_checklists: Option<bool>,
    /// Show comments.
    #[arg(long, help_heading = "Display controls")]
    allows_comments: Option<bool>,
    /// Show description titles.
    #[arg(long, help_heading = "Display controls")]
    allows_description_title: Option<bool>,
    /// Show description text.
    #[arg(long, help_heading = "Display controls")]
    allows_description_text: Option<bool>,
    /// Show activities.
    #[arg(long, help_heading = "Display controls")]
    allows_activities: Option<bool>,
    /// Show labels.
    #[arg(long, help_heading = "Display controls")]
    allows_labels: Option<bool>,
    /// Show the card creator.
    #[arg(long, help_heading = "Display controls")]
    allows_creator: Option<bool>,
    /// Show assignees.
    #[arg(long, help_heading = "Display controls")]
    allows_assignee: Option<bool>,
    /// Show card members.
    #[arg(long, help_heading = "Display controls")]
    allows_members: Option<bool>,
    /// Show requested-by information.
    #[arg(long, help_heading = "Display controls")]
    allows_requested_by: Option<bool>,
    /// Show card sorting by number.
    #[arg(long, help_heading = "Display controls")]
    allows_card_sorting_by_number: Option<bool>,
    /// Show card numbers.
    #[arg(long, help_heading = "Display controls")]
    allows_card_number: Option<bool>,
    /// Show assigned-by information.
    #[arg(long, help_heading = "Display controls")]
    allows_assigned_by: Option<bool>,
    /// Show received dates.
    #[arg(long, help_heading = "Display controls")]
    allows_received_date: Option<bool>,
    /// Show start dates.
    #[arg(long, help_heading = "Display controls")]
    allows_start_date: Option<bool>,
    /// Show end dates.
    #[arg(long, help_heading = "Display controls")]
    allows_end_date: Option<bool>,
    /// Show due dates.
    #[arg(long, help_heading = "Display controls")]
    allows_due_date: Option<bool>,
    /// Show subtasks.
    #[arg(long, help_heading = "Display controls")]
    allows_subtasks: Option<bool>,
    /// Show the creator on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_creator_on_minicard: Option<bool>,
    /// Show attachments on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_attachments_on_minicard: Option<bool>,
    /// Show checklists on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_checklists_on_minicard: Option<bool>,
    /// Toggle the checklist-at-minicard display.
    #[arg(long, help_heading = "Display controls")]
    allows_checklist_at_minicard: Option<bool>,
    /// Show cover attachments on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_cover_attachment_on_minicard: Option<bool>,
    /// Show attachment badges on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_badge_attachment_on_minicard: Option<bool>,
    /// Show card sorting by number on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_card_sorting_by_number_on_minicard: Option<bool>,
    /// Show card numbers on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_card_number_on_minicard: Option<bool>,
    /// Show description titles on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_description_title_on_minicard: Option<bool>,
    /// Show description text on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_description_text_on_minicard: Option<bool>,
    /// Show labels on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_labels_on_minicard: Option<bool>,
    /// Show assignees on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_assignee_on_minicard: Option<bool>,
    /// Show members on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_members_on_minicard: Option<bool>,
    /// Show requested-by information on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_requested_by_on_minicard: Option<bool>,
    /// Show assigned-by information on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_assigned_by_on_minicard: Option<bool>,
    /// Show received dates on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_received_date_on_minicard: Option<bool>,
    /// Show start dates on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_start_date_on_minicard: Option<bool>,
    /// Show end dates on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_end_date_on_minicard: Option<bool>,
    /// Show due dates on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_due_date_on_minicard: Option<bool>,
    /// Show subtasks on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_subtasks_on_minicard: Option<bool>,
    /// Show list information on minicards.
    #[arg(long, help_heading = "Display controls")]
    allows_show_lists_on_minicard: Option<bool>,
    /// Idle days for aging tier 1 (0–9007199254740991).
    #[arg(long, value_parser = days, help_heading = "Card aging")]
    card_aging_days1: Option<u64>,
    /// Idle days for aging tier 2 (0–9007199254740991).
    #[arg(long, value_parser = days, help_heading = "Card aging")]
    card_aging_days2: Option<u64>,
    /// Idle days for aging tier 3 (0–9007199254740991).
    #[arg(long, value_parser = days, help_heading = "Card aging")]
    card_aging_days3: Option<u64>,
    /// Show card buttons on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_card_buttons_on_minicard: Option<bool>,
    /// Show description badge on card.
    #[arg(long, help_heading = "Display controls")]
    allows_description_badge_on_card: Option<bool>,
    /// Show unread comments on card.
    #[arg(long, help_heading = "Display controls")]
    allows_unread_comments_on_card: Option<bool>,
    /// Show due from title on card.
    #[arg(long, help_heading = "Display controls")]
    allows_due_from_title_on_card: Option<bool>,
    /// Show due from title on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_due_from_title_on_minicard: Option<bool>,
    /// Show similar cards on card.
    #[arg(long, help_heading = "Display controls")]
    allows_similar_cards_on_card: Option<bool>,
    /// Show similar cards on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_similar_cards_on_minicard: Option<bool>,
    /// Show at risk on card.
    #[arg(long, help_heading = "Display controls")]
    allows_at_risk_on_card: Option<bool>,
    /// Show at risk on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_at_risk_on_minicard: Option<bool>,
    /// Show suggested members on card.
    #[arg(long, help_heading = "Display controls")]
    allows_suggested_members_on_card: Option<bool>,
    /// Show forecast on card.
    #[arg(long, help_heading = "Display controls")]
    allows_forecast_on_card: Option<bool>,
    /// Show forecast on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_forecast_on_minicard: Option<bool>,
    /// Show checklist from description on card.
    #[arg(long, help_heading = "Display controls")]
    allows_checklist_from_description_on_card: Option<bool>,
    /// Show action items on card.
    #[arg(long, help_heading = "Display controls")]
    allows_action_items_on_card: Option<bool>,
    /// Show whiteboard on card.
    #[arg(long, help_heading = "Display controls")]
    allows_whiteboard_on_card: Option<bool>,
    /// Place labels above the title on opened cards.
    #[arg(long, help_heading = "Display controls")]
    labels_above_title_on_card: Option<bool>,
    /// Show checklist due date on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_checklist_due_date_on_minicard: Option<bool>,
    /// Show location on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_location_on_minicard: Option<bool>,
    /// Show swimlane name on card.
    #[arg(long, help_heading = "Display controls")]
    allows_swimlane_name_on_card: Option<bool>,
    /// Show flowtime on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_flowtime_on_minicard: Option<bool>,
    /// Show pomodoro on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_pomodoro_on_minicard: Option<bool>,
    /// Show attachment list on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_attachment_list_on_minicard: Option<bool>,
    /// Show text notes on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_text_notes_on_minicard: Option<bool>,
    /// Show comment count on card.
    #[arg(long, help_heading = "Display controls")]
    allows_comment_count_on_card: Option<bool>,
    /// Show activities on minicard.
    #[arg(long, help_heading = "Display controls")]
    allows_activities_on_minicard: Option<bool>,
}

fn days(value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .ok()
        .filter(|n| *n <= 9_007_199_254_740_991)
        .ok_or_else(|| "expected integer days from 0 through 9007199254740991".into())
}

pub(super) async fn dispatch(
    command: CardSettingsCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    match command {
        CardSettingsCommand::Get(target) => {
            let settings = context
                .client()
                .board_card_settings(&target.board_id, context.record().token())
                .await
                .map_err(|e| {
                    map_client_error_with_not_found(
                        e,
                        &redactor,
                        "board card-settings get",
                        "the requested Wekan board was not found",
                    )
                })?;
            Ok(CommandSuccess::BoardCardSettings(
                BoardCardSettingsSuccess {
                    board_id: target.board_id,
                    settings,
                },
            ))
        }
        CardSettingsCommand::Update(args) => {
            let mut submitted_fields = Vec::new();
            if args.allows_swimlane_dragging.is_some() {
                submitted_fields.push(Field::AllowsSwimlaneDragging);
            }
            if args.allows_list_dragging.is_some() {
                submitted_fields.push(Field::AllowsListDragging);
            }
            if args.allows_card_dragging.is_some() {
                submitted_fields.push(Field::AllowsCardDragging);
            }
            if args.allows_checklist_dragging.is_some() {
                submitted_fields.push(Field::AllowsChecklistDragging);
            }
            if args.allows_checklist_item_dragging.is_some() {
                submitted_fields.push(Field::AllowsChecklistItemDragging);
            }
            if args.allows_subtask_dragging.is_some() {
                submitted_fields.push(Field::AllowsSubtaskDragging);
            }
            if args.allows_minicard_collapse.is_some() {
                submitted_fields.push(Field::AllowsMinicardCollapse);
            }
            if args.labels_above_title_on_minicard.is_some() {
                submitted_fields.push(Field::LabelsAboveTitleOnMinicard);
            }
            if args.allows_checklist_due_date.is_some() {
                submitted_fields.push(Field::AllowsChecklistDueDate);
            }
            if args.allows_checklist_title.is_some() {
                submitted_fields.push(Field::AllowsChecklistTitle);
            }
            if args.ask_for_move_reason.is_some() {
                submitted_fields.push(Field::AskForMoveReason);
            }
            if args.allows_comments_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsCommentsOnMinicard);
            }
            if args.allows_spent_time.is_some() {
                submitted_fields.push(Field::AllowsSpentTime);
            }
            if args.allows_spent_time_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsSpentTimeOnMinicard);
            }
            if args.allows_swimlane_name_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsSwimlaneNameOnMinicard);
            }
            if args.show_label_text.is_some() {
                submitted_fields.push(Field::ShowLabelText);
            }
            if args.allows_stickers.is_some() {
                submitted_fields.push(Field::AllowsStickers);
            }
            if args.allows_stickers_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsStickersOnMinicard);
            }
            if args.allows_location.is_some() {
                submitted_fields.push(Field::AllowsLocation);
            }
            if args.allows_dependencies.is_some() {
                submitted_fields.push(Field::AllowsDependencies);
            }
            if args.allows_dependencies_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsDependenciesOnMinicard);
            }
            if args.allows_flowtime.is_some() {
                submitted_fields.push(Field::AllowsFlowtime);
            }
            if args.allows_pomodoro.is_some() {
                submitted_fields.push(Field::AllowsPomodoro);
            }
            if args.allows_vote.is_some() {
                submitted_fields.push(Field::AllowsVote);
            }
            if args.allows_vote_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsVoteOnMinicard);
            }
            if args.allows_poker.is_some() {
                submitted_fields.push(Field::AllowsPoker);
            }
            if args.allows_poker_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsPokerOnMinicard);
            }
            if args.allows_text_notes.is_some() {
                submitted_fields.push(Field::AllowsTextNotes);
            }
            if args.allows_comment_count_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsCommentCountOnMinicard);
            }
            if args.allows_issue_type_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsIssueTypeOnMinicard);
            }
            if args.allows_card_color_on_card.is_some() {
                submitted_fields.push(Field::AllowsCardColorOnCard);
            }
            if args.allows_card_color_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsCardColorOnMinicard);
            }
            if args.allows_parent_card_on_card.is_some() {
                submitted_fields.push(Field::AllowsParentCardOnCard);
            }
            if args.allows_parent_card_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsParentCardOnMinicard);
            }
            if args.allows_linked_card_on_card.is_some() {
                submitted_fields.push(Field::AllowsLinkedCardOnCard);
            }
            if args.allows_linked_card_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsLinkedCardOnMinicard);
            }
            if args.allows_card_buttons_on_card.is_some() {
                submitted_fields.push(Field::AllowsCardButtonsOnCard);
            }
            if args.allows_description_badge_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsDescriptionBadgeOnMinicard);
            }
            if args.allows_unread_comments_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsUnreadCommentsOnMinicard);
            }
            if args.allows_card_collapse.is_some() {
                submitted_fields.push(Field::AllowsCardCollapse);
            }
            if args.allows_checklist_title_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsChecklistTitleOnMinicard);
            }
            if args.allows_label_text_on_card.is_some() {
                submitted_fields.push(Field::AllowsLabelTextOnCard);
            }

            if args.allows_card_counter_list.is_some() {
                submitted_fields.push(Field::AllowsCardCounterList);
            }
            if args.card_aging.is_some() {
                submitted_fields.push(Field::CardAging);
            }
            if args.allows_board_member_list.is_some() {
                submitted_fields.push(Field::AllowsBoardMemberList);
            }
            if args.allows_show_lists.is_some() {
                submitted_fields.push(Field::AllowsShowLists);
            }
            if args.allows_attachments.is_some() {
                submitted_fields.push(Field::AllowsAttachments);
            }
            if args.allows_checklists.is_some() {
                submitted_fields.push(Field::AllowsChecklists);
            }
            if args.allows_comments.is_some() {
                submitted_fields.push(Field::AllowsComments);
            }
            if args.allows_description_title.is_some() {
                submitted_fields.push(Field::AllowsDescriptionTitle);
            }
            if args.allows_description_text.is_some() {
                submitted_fields.push(Field::AllowsDescriptionText);
            }
            if args.allows_activities.is_some() {
                submitted_fields.push(Field::AllowsActivities);
            }
            if args.allows_labels.is_some() {
                submitted_fields.push(Field::AllowsLabels);
            }
            if args.allows_creator.is_some() {
                submitted_fields.push(Field::AllowsCreator);
            }
            if args.allows_assignee.is_some() {
                submitted_fields.push(Field::AllowsAssignee);
            }
            if args.allows_members.is_some() {
                submitted_fields.push(Field::AllowsMembers);
            }
            if args.allows_requested_by.is_some() {
                submitted_fields.push(Field::AllowsRequestedBy);
            }
            if args.allows_card_sorting_by_number.is_some() {
                submitted_fields.push(Field::AllowsCardSortingByNumber);
            }
            if args.allows_card_number.is_some() {
                submitted_fields.push(Field::AllowsCardNumber);
            }
            if args.allows_assigned_by.is_some() {
                submitted_fields.push(Field::AllowsAssignedBy);
            }
            if args.allows_received_date.is_some() {
                submitted_fields.push(Field::AllowsReceivedDate);
            }
            if args.allows_start_date.is_some() {
                submitted_fields.push(Field::AllowsStartDate);
            }
            if args.allows_end_date.is_some() {
                submitted_fields.push(Field::AllowsEndDate);
            }
            if args.allows_due_date.is_some() {
                submitted_fields.push(Field::AllowsDueDate);
            }
            if args.allows_subtasks.is_some() {
                submitted_fields.push(Field::AllowsSubtasks);
            }
            if args.allows_creator_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsCreatorOnMinicard);
            }
            if args.allows_attachments_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsAttachmentsOnMinicard);
            }
            if args.allows_checklists_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsChecklistsOnMinicard);
            }
            if args.allows_checklist_at_minicard.is_some() {
                submitted_fields.push(Field::AllowsChecklistAtMinicard);
            }
            if args.allows_cover_attachment_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsCoverAttachmentOnMinicard);
            }
            if args.allows_badge_attachment_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsBadgeAttachmentOnMinicard);
            }
            if args.allows_card_sorting_by_number_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsCardSortingByNumberOnMinicard);
            }
            if args.allows_card_number_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsCardNumberOnMinicard);
            }
            if args.allows_description_title_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsDescriptionTitleOnMinicard);
            }
            if args.allows_description_text_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsDescriptionTextOnMinicard);
            }
            if args.allows_labels_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsLabelsOnMinicard);
            }
            if args.allows_assignee_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsAssigneeOnMinicard);
            }
            if args.allows_members_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsMembersOnMinicard);
            }
            if args.allows_requested_by_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsRequestedByOnMinicard);
            }
            if args.allows_assigned_by_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsAssignedByOnMinicard);
            }
            if args.allows_received_date_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsReceivedDateOnMinicard);
            }
            if args.allows_start_date_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsStartDateOnMinicard);
            }
            if args.allows_end_date_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsEndDateOnMinicard);
            }
            if args.allows_due_date_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsDueDateOnMinicard);
            }
            if args.allows_subtasks_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsSubtasksOnMinicard);
            }
            if args.allows_show_lists_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsShowListsOnMinicard);
            }
            if args.card_aging_days1.is_some() {
                submitted_fields.push(Field::CardAgingDays1);
            }
            if args.card_aging_days2.is_some() {
                submitted_fields.push(Field::CardAgingDays2);
            }
            if args.card_aging_days3.is_some() {
                submitted_fields.push(Field::CardAgingDays3);
            }
            if args.allows_card_buttons_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsCardButtonsOnMinicard);
            }
            if args.allows_description_badge_on_card.is_some() {
                submitted_fields.push(Field::AllowsDescriptionBadgeOnCard);
            }
            if args.allows_unread_comments_on_card.is_some() {
                submitted_fields.push(Field::AllowsUnreadCommentsOnCard);
            }
            if args.allows_due_from_title_on_card.is_some() {
                submitted_fields.push(Field::AllowsDueFromTitleOnCard);
            }
            if args.allows_due_from_title_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsDueFromTitleOnMinicard);
            }
            if args.allows_similar_cards_on_card.is_some() {
                submitted_fields.push(Field::AllowsSimilarCardsOnCard);
            }
            if args.allows_similar_cards_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsSimilarCardsOnMinicard);
            }
            if args.allows_at_risk_on_card.is_some() {
                submitted_fields.push(Field::AllowsAtRiskOnCard);
            }
            if args.allows_at_risk_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsAtRiskOnMinicard);
            }
            if args.allows_suggested_members_on_card.is_some() {
                submitted_fields.push(Field::AllowsSuggestedMembersOnCard);
            }
            if args.allows_forecast_on_card.is_some() {
                submitted_fields.push(Field::AllowsForecastOnCard);
            }
            if args.allows_forecast_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsForecastOnMinicard);
            }
            if args.allows_checklist_from_description_on_card.is_some() {
                submitted_fields.push(Field::AllowsChecklistFromDescriptionOnCard);
            }
            if args.allows_action_items_on_card.is_some() {
                submitted_fields.push(Field::AllowsActionItemsOnCard);
            }
            if args.allows_whiteboard_on_card.is_some() {
                submitted_fields.push(Field::AllowsWhiteboardOnCard);
            }
            if args.labels_above_title_on_card.is_some() {
                submitted_fields.push(Field::LabelsAboveTitleOnCard);
            }
            if args.allows_checklist_due_date_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsChecklistDueDateOnMinicard);
            }
            if args.allows_location_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsLocationOnMinicard);
            }
            if args.allows_swimlane_name_on_card.is_some() {
                submitted_fields.push(Field::AllowsSwimlaneNameOnCard);
            }
            if args.allows_flowtime_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsFlowtimeOnMinicard);
            }
            if args.allows_pomodoro_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsPomodoroOnMinicard);
            }
            if args.allows_attachment_list_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsAttachmentListOnMinicard);
            }
            if args.allows_text_notes_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsTextNotesOnMinicard);
            }
            if args.allows_comment_count_on_card.is_some() {
                submitted_fields.push(Field::AllowsCommentCountOnCard);
            }
            if args.allows_activities_on_minicard.is_some() {
                submitted_fields.push(Field::AllowsActivitiesOnMinicard);
            }
            let request = UpdateBoardCardSettingsRequest {
                allows_swimlane_dragging: args.allows_swimlane_dragging,
                allows_list_dragging: args.allows_list_dragging,
                allows_card_dragging: args.allows_card_dragging,
                allows_checklist_dragging: args.allows_checklist_dragging,
                allows_checklist_item_dragging: args.allows_checklist_item_dragging,
                allows_subtask_dragging: args.allows_subtask_dragging,
                allows_minicard_collapse: args.allows_minicard_collapse,
                labels_above_title_on_minicard: args.labels_above_title_on_minicard,
                allows_checklist_due_date: args.allows_checklist_due_date,
                allows_checklist_title: args.allows_checklist_title,
                ask_for_move_reason: args.ask_for_move_reason,
                allows_comments_on_minicard: args.allows_comments_on_minicard,
                allows_spent_time: args.allows_spent_time,
                allows_spent_time_on_minicard: args.allows_spent_time_on_minicard,
                allows_swimlane_name_on_minicard: args.allows_swimlane_name_on_minicard,
                show_label_text: args.show_label_text,
                allows_stickers: args.allows_stickers,
                allows_stickers_on_minicard: args.allows_stickers_on_minicard,
                allows_location: args.allows_location,
                allows_dependencies: args.allows_dependencies,
                allows_dependencies_on_minicard: args.allows_dependencies_on_minicard,
                allows_flowtime: args.allows_flowtime,
                allows_pomodoro: args.allows_pomodoro,
                allows_vote: args.allows_vote,
                allows_vote_on_minicard: args.allows_vote_on_minicard,
                allows_poker: args.allows_poker,
                allows_poker_on_minicard: args.allows_poker_on_minicard,
                allows_text_notes: args.allows_text_notes,
                allows_comment_count_on_minicard: args.allows_comment_count_on_minicard,
                allows_issue_type_on_minicard: args.allows_issue_type_on_minicard,
                allows_card_color_on_card: args.allows_card_color_on_card,
                allows_card_color_on_minicard: args.allows_card_color_on_minicard,
                allows_parent_card_on_card: args.allows_parent_card_on_card,
                allows_parent_card_on_minicard: args.allows_parent_card_on_minicard,
                allows_linked_card_on_card: args.allows_linked_card_on_card,
                allows_linked_card_on_minicard: args.allows_linked_card_on_minicard,
                allows_card_buttons_on_card: args.allows_card_buttons_on_card,
                allows_description_badge_on_minicard: args.allows_description_badge_on_minicard,
                allows_unread_comments_on_minicard: args.allows_unread_comments_on_minicard,
                allows_card_collapse: args.allows_card_collapse,
                allows_checklist_title_on_minicard: args.allows_checklist_title_on_minicard,
                allows_label_text_on_card: args.allows_label_text_on_card,

                allows_card_counter_list: args.allows_card_counter_list,
                card_aging: args.card_aging,
                allows_board_member_list: args.allows_board_member_list,
                allows_show_lists: args.allows_show_lists,
                allows_attachments: args.allows_attachments,
                allows_checklists: args.allows_checklists,
                allows_comments: args.allows_comments,
                allows_description_title: args.allows_description_title,
                allows_description_text: args.allows_description_text,
                allows_activities: args.allows_activities,
                allows_labels: args.allows_labels,
                allows_creator: args.allows_creator,
                allows_assignee: args.allows_assignee,
                allows_members: args.allows_members,
                allows_requested_by: args.allows_requested_by,
                allows_card_sorting_by_number: args.allows_card_sorting_by_number,
                allows_card_number: args.allows_card_number,
                allows_assigned_by: args.allows_assigned_by,
                allows_received_date: args.allows_received_date,
                allows_start_date: args.allows_start_date,
                allows_end_date: args.allows_end_date,
                allows_due_date: args.allows_due_date,
                allows_subtasks: args.allows_subtasks,
                allows_creator_on_minicard: args.allows_creator_on_minicard,
                allows_attachments_on_minicard: args.allows_attachments_on_minicard,
                allows_checklists_on_minicard: args.allows_checklists_on_minicard,
                allows_checklist_at_minicard: args.allows_checklist_at_minicard,
                allows_cover_attachment_on_minicard: args.allows_cover_attachment_on_minicard,
                allows_badge_attachment_on_minicard: args.allows_badge_attachment_on_minicard,
                allows_card_sorting_by_number_on_minicard: args
                    .allows_card_sorting_by_number_on_minicard,
                allows_card_number_on_minicard: args.allows_card_number_on_minicard,
                allows_description_title_on_minicard: args.allows_description_title_on_minicard,
                allows_description_text_on_minicard: args.allows_description_text_on_minicard,
                allows_labels_on_minicard: args.allows_labels_on_minicard,
                allows_assignee_on_minicard: args.allows_assignee_on_minicard,
                allows_members_on_minicard: args.allows_members_on_minicard,
                allows_requested_by_on_minicard: args.allows_requested_by_on_minicard,
                allows_assigned_by_on_minicard: args.allows_assigned_by_on_minicard,
                allows_received_date_on_minicard: args.allows_received_date_on_minicard,
                allows_start_date_on_minicard: args.allows_start_date_on_minicard,
                allows_end_date_on_minicard: args.allows_end_date_on_minicard,
                allows_due_date_on_minicard: args.allows_due_date_on_minicard,
                allows_subtasks_on_minicard: args.allows_subtasks_on_minicard,
                allows_show_lists_on_minicard: args.allows_show_lists_on_minicard,
                card_aging_days1: args.card_aging_days1,
                card_aging_days2: args.card_aging_days2,
                card_aging_days3: args.card_aging_days3,
                allows_card_buttons_on_minicard: args.allows_card_buttons_on_minicard,
                allows_description_badge_on_card: args.allows_description_badge_on_card,
                allows_unread_comments_on_card: args.allows_unread_comments_on_card,
                allows_due_from_title_on_card: args.allows_due_from_title_on_card,
                allows_due_from_title_on_minicard: args.allows_due_from_title_on_minicard,
                allows_similar_cards_on_card: args.allows_similar_cards_on_card,
                allows_similar_cards_on_minicard: args.allows_similar_cards_on_minicard,
                allows_at_risk_on_card: args.allows_at_risk_on_card,
                allows_at_risk_on_minicard: args.allows_at_risk_on_minicard,
                allows_suggested_members_on_card: args.allows_suggested_members_on_card,
                allows_forecast_on_card: args.allows_forecast_on_card,
                allows_forecast_on_minicard: args.allows_forecast_on_minicard,
                allows_checklist_from_description_on_card: args
                    .allows_checklist_from_description_on_card,
                allows_action_items_on_card: args.allows_action_items_on_card,
                allows_whiteboard_on_card: args.allows_whiteboard_on_card,
                labels_above_title_on_card: args.labels_above_title_on_card,
                allows_checklist_due_date_on_minicard: args.allows_checklist_due_date_on_minicard,
                allows_location_on_minicard: args.allows_location_on_minicard,
                allows_swimlane_name_on_card: args.allows_swimlane_name_on_card,
                allows_flowtime_on_minicard: args.allows_flowtime_on_minicard,
                allows_pomodoro_on_minicard: args.allows_pomodoro_on_minicard,
                allows_attachment_list_on_minicard: args.allows_attachment_list_on_minicard,
                allows_text_notes_on_minicard: args.allows_text_notes_on_minicard,
                allows_comment_count_on_card: args.allows_comment_count_on_card,
                allows_activities_on_minicard: args.allows_activities_on_minicard,
            };
            let settings = context
                .client()
                .update_board_card_settings(
                    &args.target.board_id,
                    &request,
                    context.record().token(),
                )
                .await
                .map_err(|e| {
                    map_client_error_with_not_found(
                        e,
                        &redactor,
                        "board card-settings update",
                        "the requested Wekan board was not found",
                    )
                })?;
            Ok(CommandSuccess::BoardCardSettingsUpdated(
                BoardCardSettingsUpdateSuccess {
                    board_id: args.target.board_id,
                    settings,
                    submitted_fields,
                },
            ))
        }
    }
}
