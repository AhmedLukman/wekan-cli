use crate::{
    client::{WekanClientFactory, boards::card_settings::UpdateBoardCardSettingsRequest},
    command_result::{
        BoardCardSettingField as Field, BoardCardSettingsSuccess, BoardCardSettingsUpdateSuccess,
        CommandSuccess,
    },
    commands::{
        authenticated::AuthenticatedContext,
        client_error::{map_client_error_with_not_found, map_mutation_client_error_with_not_found},
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
])), after_help = "Supply at least one setting. Boolean flags require true or false; aging thresholds require nonnegative integer days. Raw API requests can expose Wekan's permissive coercions.")]
pub struct UpdateArgs {
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
            let request = UpdateBoardCardSettingsRequest {
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
                    map_mutation_client_error_with_not_found(
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
