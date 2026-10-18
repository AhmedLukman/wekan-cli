mod attachments;
mod auth;
mod boards;
mod cards;
mod checklist_items;
mod checklists;
mod comments;
mod custom_field_items;
mod custom_fields;
mod dependencies;
mod formatting;
mod imports;
mod integrations;
mod lists;
mod profiles;
mod raw;
mod rules;
mod swimlanes;
mod users;

use auth::{render_auth_status, render_auth_success, render_logout};
use boards::{
    render_board_count, render_board_created, render_board_deleted, render_board_detail,
    render_board_list, render_board_renamed,
};
use cards::{
    render_card_archived, render_card_collection, render_card_created, render_card_deleted,
    render_card_detail, render_card_restored, render_card_updated,
};
use checklist_items::{
    render_checklist_item_created, render_checklist_item_deleted, render_checklist_item_detail,
    render_checklist_item_updated,
};
use checklists::{
    render_checklist_collection, render_checklist_created, render_checklist_deleted,
    render_checklist_detail,
};
use comments::{
    render_comment_collection, render_comment_created, render_comment_deleted,
    render_comment_detail,
};
use lists::{
    render_list_collection, render_list_created, render_list_deleted, render_list_detail,
    render_list_updated,
};
use profiles::{render_profile_item, render_profile_list, render_profile_removed};
use swimlanes::{
    render_swimlane_collection, render_swimlane_created, render_swimlane_deleted,
    render_swimlane_detail, render_swimlane_updated,
};
use users::{
    render_user_boards, render_user_cards, render_user_created, render_user_deleted,
    render_user_detail, render_user_list, render_user_login_change, render_user_ownership,
};

use std::{
    ffi::OsString,
    io::{Read, Write},
    process::ExitCode,
};

use clap::ValueEnum;
use serde::Serialize;

use crate::command_result::CommandSuccess;
use crate::{
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
};

pub(crate) use raw::render_human_response as render_api_response_human;
#[cfg(test)]
pub(crate) use raw::structured_api_response;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
    Raw,
}

impl OutputFormat {
    pub fn detect_from_args(args: &[OsString]) -> Self {
        let mut format = Self::Human;
        let mut command = crate::command();
        command.build();
        let mut current = &command;
        let mut iter = args.iter().skip(1).peekable();
        while let Some(argument) = iter.next() {
            let Some(arg) = argument.to_str() else {
                continue;
            };
            if arg == "--" {
                break;
            }
            if arg == "--output" {
                format = match iter.next().and_then(|value| value.to_str()) {
                    Some("json") => Self::Json,
                    Some("raw") => Self::Raw,
                    _ => Self::Human,
                };
            } else if let Some(value) = arg.strip_prefix("--output=") {
                format = match value {
                    "json" => Self::Json,
                    "raw" => Self::Raw,
                    _ => Self::Human,
                };
            } else if let Some(subcommand) = current.find_subcommand(arg) {
                current = subcommand;
            } else if let Some(option) = arg.strip_prefix("--").and_then(|name| {
                current
                    .get_arguments()
                    .find(|option| option.get_long() == Some(name))
            }) {
                // Hyphen-prefixed values (for example user cards --from) are
                // data even when they happen to look like an output option.
                if option
                    .get_num_args()
                    .is_some_and(|arity| arity.takes_values())
                    && iter.peek().is_some_and(|value| {
                        option.is_allow_hyphen_values_set()
                            || !value.to_string_lossy().starts_with('-')
                    })
                {
                    iter.next();
                }
            }
        }
        format
    }
}

#[derive(Serialize)]
struct SuccessEnvelope<'a, T> {
    ok: bool,
    data: &'a T,
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    ok: bool,
    error: ErrorBody<'a>,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'static str,
    message: &'a str,
    details: &'a crate::error::ErrorDetails,
}

pub fn render_success(format: OutputFormat, success: &CommandSuccess) -> String {
    match (format, success) {
        (OutputFormat::Human, CommandSuccess::AuthPreflight(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::AuthPreflight(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::SchemaUpgradeStatus(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::SchemaUpgradeStatus(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::BoardAttachments(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::BoardAttachments(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardImported(data)) => imports::board(data),
        (OutputFormat::Json, CommandSuccess::BoardImported(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::TrelloImported(data)) => imports::trello(data),
        (OutputFormat::Json, CommandSuccess::TrelloImported(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ZipImported(data)) => imports::zip(data),
        (OutputFormat::Json, CommandSuccess::ZipImported(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::IcsImported(data)) => imports::ics(data),
        (OutputFormat::Json, CommandSuccess::IcsImported(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::Exported(data)) => formatting::render_object(data),
        (OutputFormat::Json, CommandSuccess::Exported(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::AdminDomains(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::AdminDomains(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::AdminOrganizations(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::AdminOrganizations(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::AdminTeams(data)) => formatting::render_object(data),
        (OutputFormat::Json, CommandSuccess::AdminTeams(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::AdminOrganizationUpdated(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::AdminOrganizationUpdated(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::AdminTeamUpdated(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::AdminTeamUpdated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::AdminOrganizationFeaturesUpdatedAll(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::AdminOrganizationFeaturesUpdatedAll(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::AdminTeamFeaturesUpdatedAll(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::AdminTeamFeaturesUpdatedAll(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::GlobalSettings(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::GlobalSettings(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::GlobalSettingsUpdated(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::GlobalSettingsUpdated(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::AttachmentSettings(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::AttachmentSettings(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::AttachmentSettingsUpdated(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::AttachmentSettingsUpdated(data)) => {
            render_json_success(data)
        }

        (OutputFormat::Human, CommandSuccess::BoardCardSettings(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::BoardCardSettings(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardCardSettingsUpdated(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::BoardCardSettingsUpdated(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::BoardDomains(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::BoardDomains(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardDomainChanged(data)) => format!(
            "Board domains returned for the requested sharing change:\n{}",
            formatting::render_object(data)
        ),
        (OutputFormat::Json, CommandSuccess::BoardDomainChanged(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::RuleCollection(data)) => rules::collection(data),
        (OutputFormat::Json, CommandSuccess::RuleCollection(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::RuleShown(data)) => rules::detail(data),
        (OutputFormat::Json, CommandSuccess::RuleShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::RuleCreated(data)) => rules::created(data),
        (OutputFormat::Json, CommandSuccess::RuleCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::RuleUpdated(data)) => rules::updated(data),
        (OutputFormat::Json, CommandSuccess::RuleUpdated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::RuleDeleted(data)) => rules::deleted(data),
        (OutputFormat::Json, CommandSuccess::RuleDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::IntegrationCollection(data)) => {
            integrations::collection(data)
        }
        (OutputFormat::Json, CommandSuccess::IntegrationCollection(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::IntegrationShown(data)) => integrations::detail(data),
        (OutputFormat::Json, CommandSuccess::IntegrationShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::IntegrationCreated(data)) => {
            integrations::created(data)
        }
        (OutputFormat::Json, CommandSuccess::IntegrationCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::IntegrationUpdated(data)) => {
            integrations::updated(data)
        }
        (OutputFormat::Json, CommandSuccess::IntegrationUpdated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::IntegrationDeleted(data)) => {
            integrations::deleted(data)
        }
        (OutputFormat::Json, CommandSuccess::IntegrationDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::IntegrationActivitiesChanged(data)) => {
            integrations::activities(data)
        }
        (OutputFormat::Json, CommandSuccess::IntegrationActivitiesChanged(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::BoardCopied(data)) => {
            format!(
                "Board copy acknowledged; effects are unconfirmed:\n{}",
                formatting::render_object(data)
            )
        }
        (OutputFormat::Json, CommandSuccess::BoardCopied(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardCopied(data)) => {
            format!(
                "Card copy acknowledged; effects are unconfirmed:\n{}",
                formatting::render_object(data)
            )
        }
        (OutputFormat::Json, CommandSuccess::CardCopied(data)) => render_json_success(data),
        (
            OutputFormat::Human,
            CommandSuccess::ListCopied(data) | CommandSuccess::ListMoved(data),
        ) => format!(
            "List transfer acknowledged; effects are unconfirmed:\n{}",
            formatting::render_object(data)
        ),
        (
            OutputFormat::Json,
            CommandSuccess::ListCopied(data) | CommandSuccess::ListMoved(data),
        ) => render_json_success(data),
        (
            OutputFormat::Human,
            CommandSuccess::SwimlaneCopied(data) | CommandSuccess::SwimlaneMoved(data),
        ) => format!(
            "Swimlane transfer acknowledged; effects are unconfirmed:\n{}",
            formatting::render_object(data)
        ),
        (
            OutputFormat::Json,
            CommandSuccess::SwimlaneCopied(data) | CommandSuccess::SwimlaneMoved(data),
        ) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardCount(data)) => cards::render_card_count(data),
        (OutputFormat::Json, CommandSuccess::CardCount(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::SwimlaneCards(data)) => {
            cards::render_swimlane_cards(data)
        }
        (OutputFormat::Json, CommandSuccess::SwimlaneCards(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardBatch(data)) => cards::render_batch(data),
        (OutputFormat::Json, CommandSuccess::CardBatch(data)) => match data {
            crate::command_result::CardBatchResult::CardBulkCreate(data) => {
                render_json_success(data)
            }
            crate::command_result::CardBatchResult::CardBulkDelete(data) => {
                render_json_success(data)
            }
            crate::command_result::CardBatchResult::CardBulkLabels(data) => {
                render_json_success(data)
            }
        },
        (OutputFormat::Human, CommandSuccess::BoardLabels(data)) => formatting::render_object(data),
        (OutputFormat::Json, CommandSuccess::BoardLabels(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardLabelAdded(data)) => {
            boards::render_label_added(data)
        }
        (OutputFormat::Json, CommandSuccess::BoardLabelAdded(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardLabels(data)) => formatting::render_object(data),
        (OutputFormat::Json, CommandSuccess::CardLabels(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardMembers(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::BoardMembers(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardMemberChanged(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::BoardMemberChanged(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardMemberUpdated(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::BoardMemberUpdated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardMembers(data)) => formatting::render_object(data),
        (OutputFormat::Json, CommandSuccess::CardMembers(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardAssignees(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::CardAssignees(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardMembershipChanged(data)) => {
            formatting::render_object(data)
        }
        (OutputFormat::Json, CommandSuccess::CardMembershipChanged(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::CardDependencies(data)) => {
            dependencies::card_collection(data)
        }
        (OutputFormat::Json, CommandSuccess::CardDependencies(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardDependencies(data)) => {
            dependencies::board_collection(data)
        }
        (OutputFormat::Json, CommandSuccess::BoardDependencies(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::DependencyCreated(data)) => {
            dependencies::created(data)
        }
        (OutputFormat::Json, CommandSuccess::DependencyCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::DependencyUpdated(data)) => {
            dependencies::updated(data)
        }
        (OutputFormat::Json, CommandSuccess::DependencyUpdated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::DependencyDeleted(data)) => {
            dependencies::deleted(data)
        }
        (OutputFormat::Json, CommandSuccess::DependencyDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::AttachmentList(data)) => attachments::list(data),
        (OutputFormat::Json, CommandSuccess::AttachmentList(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::AttachmentShown(data)) => attachments::detail(data),
        (OutputFormat::Json, CommandSuccess::AttachmentShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::AttachmentUploaded(data)) => {
            attachments::detail(data)
        }
        (OutputFormat::Json, CommandSuccess::AttachmentUploaded(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BackgroundUploaded(data)) => {
            attachments::detail(data)
        }
        (OutputFormat::Json, CommandSuccess::BackgroundUploaded(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::AttachmentCopied(data)) => attachments::detail(data),
        (OutputFormat::Json, CommandSuccess::AttachmentCopied(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::AttachmentMoved(data)) => attachments::detail(data),
        (OutputFormat::Json, CommandSuccess::AttachmentMoved(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::AttachmentDeleted(data)) => attachments::detail(data),
        (OutputFormat::Json, CommandSuccess::AttachmentDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::AttachmentDownloaded(data)) => {
            attachments::detail(data)
        }
        (OutputFormat::Json, CommandSuccess::AttachmentDownloaded(data)) => {
            render_json_success(data)
        }
        (_, CommandSuccess::ApiResponse(_)) => {
            panic!("streaming API responses must be consumed by output::write_success")
        }
        (OutputFormat::Raw, CommandSuccess::Cancelled(_)) => String::new(),
        (OutputFormat::Raw, _) => panic!("raw results must be consumed by output::write_success"),
        (OutputFormat::Human, CommandSuccess::Cancelled(_)) => {
            "Cancelled; no changes made.".to_owned()
        }
        (OutputFormat::Json, CommandSuccess::Cancelled(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::Registration(data)) => {
            render_auth_success("Registered user", data)
        }
        (OutputFormat::Json, CommandSuccess::Registration(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::Login(data)) => {
            render_auth_success("Logged in user", data)
        }
        (OutputFormat::Json, CommandSuccess::Login(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::Logout(data)) => render_logout(data),
        (OutputFormat::Json, CommandSuccess::Logout(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::AuthStatus(data)) => render_auth_status(data),
        (OutputFormat::Json, CommandSuccess::AuthStatus(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserCurrent(data))
        | (OutputFormat::Human, CommandSuccess::UserShown(data))
        | (OutputFormat::Human, CommandSuccess::UserUpdated(data)) => render_user_detail(data),
        (OutputFormat::Json, CommandSuccess::UserCurrent(data))
        | (OutputFormat::Json, CommandSuccess::UserShown(data))
        | (OutputFormat::Json, CommandSuccess::UserUpdated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserList(data)) => render_user_list(data),
        (OutputFormat::Json, CommandSuccess::UserList(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserCards(data)) => render_user_cards(data),
        (OutputFormat::Json, CommandSuccess::UserCards(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserCreated(data)) => render_user_created(data),
        (OutputFormat::Json, CommandSuccess::UserCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserBoards(data)) => render_user_boards(data),
        (OutputFormat::Json, CommandSuccess::UserBoards(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserOwnershipTaken(data)) => {
            render_user_ownership(data)
        }
        (OutputFormat::Json, CommandSuccess::UserOwnershipTaken(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserLoginChanged(data)) => {
            render_user_login_change(data)
        }
        (OutputFormat::Json, CommandSuccess::UserLoginChanged(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserDeleted(data)) => render_user_deleted(data),
        (OutputFormat::Json, CommandSuccess::UserDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserTokenCreated(data)) => {
            users::render_token_created(data)
        }
        (OutputFormat::Json, CommandSuccess::UserTokenCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::UserTokenDeleted(data)) => {
            users::render_token_deleted(data)
        }
        (OutputFormat::Json, CommandSuccess::UserTokenDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardList(data)) => render_board_list(data),
        (OutputFormat::Json, CommandSuccess::BoardList(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardCount(data)) => render_board_count(data),
        (OutputFormat::Json, CommandSuccess::BoardCount(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardShown(data)) => render_board_detail(data),
        (OutputFormat::Json, CommandSuccess::BoardShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardCreated(data)) => render_board_created(data),
        (OutputFormat::Json, CommandSuccess::BoardCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardRenamed(data)) => render_board_renamed(data),
        (OutputFormat::Json, CommandSuccess::BoardRenamed(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::BoardDeleted(data)) => render_board_deleted(data),
        (OutputFormat::Json, CommandSuccess::BoardDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ListCollection(data)) => render_list_collection(data),
        (OutputFormat::Json, CommandSuccess::ListCollection(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ListShown(data)) => render_list_detail(data),
        (OutputFormat::Json, CommandSuccess::ListShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ListCreated(data)) => render_list_created(data),
        (OutputFormat::Json, CommandSuccess::ListCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ListUpdated(data)) => render_list_updated(data),
        (OutputFormat::Json, CommandSuccess::ListUpdated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ListDeleted(data)) => render_list_deleted(data),
        (OutputFormat::Json, CommandSuccess::ListDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardCollection(data)) => render_card_collection(data),
        (OutputFormat::Json, CommandSuccess::CardCollection(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardShown(data)) => render_card_detail(data),
        (OutputFormat::Json, CommandSuccess::CardShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardCreated(data)) => render_card_created(data),
        (OutputFormat::Json, CommandSuccess::CardCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardUpdated(data)) => render_card_updated(data),
        (OutputFormat::Json, CommandSuccess::CardUpdated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardArchived(data)) => render_card_archived(data),
        (OutputFormat::Json, CommandSuccess::CardArchived(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardRestored(data)) => render_card_restored(data),
        (OutputFormat::Json, CommandSuccess::CardRestored(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardDeleted(data)) => render_card_deleted(data),
        (OutputFormat::Json, CommandSuccess::CardDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ChecklistCollection(data)) => {
            render_checklist_collection(data)
        }
        (OutputFormat::Json, CommandSuccess::ChecklistCollection(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::ChecklistShown(data)) => {
            render_checklist_detail(data)
        }
        (OutputFormat::Json, CommandSuccess::ChecklistShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ChecklistCreated(data)) => {
            render_checklist_created(data)
        }
        (OutputFormat::Json, CommandSuccess::ChecklistCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ChecklistDeleted(data)) => {
            render_checklist_deleted(data)
        }
        (OutputFormat::Json, CommandSuccess::ChecklistDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ChecklistItemShown(data)) => {
            render_checklist_item_detail(data)
        }
        (OutputFormat::Json, CommandSuccess::ChecklistItemShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ChecklistItemCreated(data)) => {
            render_checklist_item_created(data)
        }
        (OutputFormat::Json, CommandSuccess::ChecklistItemCreated(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::ChecklistItemUpdated(data)) => {
            render_checklist_item_updated(data)
        }
        (OutputFormat::Json, CommandSuccess::ChecklistItemUpdated(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::ChecklistItemDeleted(data)) => {
            render_checklist_item_deleted(data)
        }
        (OutputFormat::Json, CommandSuccess::ChecklistItemDeleted(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::CustomFieldCollection(data)) => {
            custom_fields::collection(data)
        }
        (OutputFormat::Json, CommandSuccess::CustomFieldCollection(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::CustomFieldShown(data)) => {
            custom_fields::detail(data)
        }
        (OutputFormat::Json, CommandSuccess::CustomFieldShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CustomFieldCreated(data)) => {
            custom_fields::created(data)
        }
        (OutputFormat::Json, CommandSuccess::CustomFieldCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CustomFieldUpdated(data)) => {
            custom_fields::updated(data)
        }
        (OutputFormat::Json, CommandSuccess::CustomFieldUpdated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CustomFieldDeleted(data)) => {
            custom_fields::deleted(data)
        }
        (OutputFormat::Json, CommandSuccess::CustomFieldDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CustomFieldItemsCreated(data)) => {
            custom_field_items::created(data)
        }
        (OutputFormat::Json, CommandSuccess::CustomFieldItemsCreated(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::CustomFieldItemUpdated(data)) => {
            custom_field_items::updated(data)
        }
        (OutputFormat::Json, CommandSuccess::CustomFieldItemUpdated(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::CustomFieldItemDeleted(data)) => {
            custom_field_items::deleted(data)
        }
        (OutputFormat::Json, CommandSuccess::CustomFieldItemDeleted(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::CardCustomFieldSet(data)) => {
            cards::render_card_custom_field_set(data)
        }
        (OutputFormat::Json, CommandSuccess::CardCustomFieldSet(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CardCustomFieldQuery(data)) => {
            cards::render_card_custom_field_query(data)
        }
        (OutputFormat::Json, CommandSuccess::CardCustomFieldQuery(data)) => {
            render_json_success(data)
        }
        (OutputFormat::Human, CommandSuccess::CommentCollection(data)) => {
            render_comment_collection(data)
        }
        (OutputFormat::Json, CommandSuccess::CommentCollection(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CommentShown(data)) => render_comment_detail(data),
        (OutputFormat::Json, CommandSuccess::CommentShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CommentCreated(data)) => render_comment_created(data),
        (OutputFormat::Json, CommandSuccess::CommentCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::CommentDeleted(data)) => render_comment_deleted(data),
        (OutputFormat::Json, CommandSuccess::CommentDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::SwimlaneCollection(data)) => {
            render_swimlane_collection(data)
        }
        (OutputFormat::Json, CommandSuccess::SwimlaneCollection(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::SwimlaneShown(data)) => render_swimlane_detail(data),
        (OutputFormat::Json, CommandSuccess::SwimlaneShown(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::SwimlaneCreated(data)) => {
            render_swimlane_created(data)
        }
        (OutputFormat::Json, CommandSuccess::SwimlaneCreated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::SwimlaneUpdated(data)) => {
            render_swimlane_updated(data)
        }
        (OutputFormat::Json, CommandSuccess::SwimlaneUpdated(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::SwimlaneDeleted(data)) => {
            render_swimlane_deleted(data)
        }
        (OutputFormat::Json, CommandSuccess::SwimlaneDeleted(data)) => render_json_success(data),
        (OutputFormat::Human, CommandSuccess::ProfileAdded(data)) => {
            render_profile_item("Added profile", data)
        }
        (OutputFormat::Json, CommandSuccess::ProfileAdded(data))
        | (OutputFormat::Json, CommandSuccess::ProfileShown(data))
        | (OutputFormat::Json, CommandSuccess::ProfileUsed(data))
        | (OutputFormat::Json, CommandSuccess::ProfileUpdated(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("profile success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::ProfileList(data)) => render_profile_list(data),
        (OutputFormat::Json, CommandSuccess::ProfileList(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("profile list success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::ProfileShown(data)) => {
            render_profile_item("Profile", data)
        }
        (OutputFormat::Human, CommandSuccess::ProfileUsed(data)) => {
            render_profile_item("Using profile", data)
        }
        (OutputFormat::Human, CommandSuccess::ProfileUpdated(data)) => {
            render_profile_item("Updated profile", data)
        }
        (OutputFormat::Human, CommandSuccess::ProfileRemoved(data)) => render_profile_removed(data),
        (OutputFormat::Json, CommandSuccess::ProfileRemoved(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("profile removal success is always serializable")
        }
    }
}

pub async fn write_success(
    format: OutputFormat,
    success: CommandSuccess,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<ExitCode, AppError> {
    if let CommandSuccess::ApiResponse(response) = success {
        return raw::write_response(format, response, stdout, stderr).await;
    }
    if format == OutputFormat::Raw {
        if let CommandSuccess::AttachmentDownloaded(data) = &success {
            let bytes = data.bytes.as_deref().ok_or_else(|| {
                AppError::invalid_input("raw attachment output requires --destination -")
            })?;
            stdout
                .write_all(bytes)
                .and_then(|()| stdout.flush())
                .map_err(output_write_error)?;
            return Ok(ExitCode::SUCCESS);
        }
    }
    if format == OutputFormat::Raw {
        if let CommandSuccess::Exported(data) = &success {
            let staged = data.staged.as_ref().ok_or_else(|| {
                AppError::invalid_input("raw export output requires --destination -")
            })?;
            let mut file = staged.0.reopen().map_err(export_read_error)?;
            let mut buffer = [0; 64 * 1024];
            loop {
                let read = file.read(&mut buffer).map_err(export_read_error)?;
                if read == 0 {
                    break;
                }
                stdout
                    .write_all(&buffer[..read])
                    .map_err(output_write_error)?;
            }
            stdout.flush().map_err(output_write_error)?;
            return Ok(ExitCode::SUCCESS);
        }
    }
    let rendered = render_success(format, &success);
    if !rendered.is_empty() {
        let written = writeln!(stdout, "{rendered}").and_then(|()| {
            if matches!(&success, CommandSuccess::AttachmentDownloaded(data) if data.file_saved)
                || matches!(&success, CommandSuccess::Exported(data) if data.file_saved)
            {
                stdout.flush()
            } else {
                Ok(())
            }
        });
        written.map_err(|e| {
            let error = output_write_error(e);
            if let CommandSuccess::Exported(data) = &success {
                if data.file_saved {
                    return error.with_details(crate::error::ErrorDetails {
                        file_saved: Some(true),
                        destination: Some(data.destination.clone()),
                        ..crate::error::ErrorDetails::default()
                    });
                }
            }
            if let CommandSuccess::AttachmentDownloaded(data) = &success {
                if data.file_saved {
                    return error.with_details(crate::error::ErrorDetails {
                        file_saved: Some(true),
                        destination: Some(data.destination.clone()),
                        ..crate::error::ErrorDetails::default()
                    });
                }
            }
            error
        })?;
    }
    Ok(ExitCode::SUCCESS)
}
fn export_read_error(error: std::io::Error) -> AppError {
    AppError::new(
        ErrorCode::FileIoError,
        format!(
            "could not read the staged export file: {}",
            crate::input::escape_terminal_text(&error.to_string())
        ),
        StableExitCode::Transport,
    )
}
pub(crate) fn output_write_error(error: std::io::Error) -> AppError {
    AppError::new(
        ErrorCode::InternalError,
        format!("could not write command output: {error}"),
        StableExitCode::Transport,
    )
}

pub(crate) fn render_json_success<T: Serialize>(data: &T) -> String {
    serde_json::to_string(&SuccessEnvelope { ok: true, data })
        .expect("command success is always serializable")
}

pub fn render_error(format: OutputFormat, error: &AppError) -> String {
    match format {
        OutputFormat::Human => {
            let mut rendered = format!(
                "Error [{}]: {}",
                error.code().as_str(),
                error.message().trim()
            );
            if let Some(batch) = &error.details().batch_result {
                rendered.push('\n');
                rendered.push_str(&cards::render_batch(batch));
            }
            if let Some(result) = &error.details().trello_import_result {
                rendered.push('\n');
                rendered.push_str(&imports::trello(result));
            }
            if let Some(ids) = &error.details().invalid_label_ids {
                rendered.push_str(&format!(
                    "\nInvalid label IDs: {}",
                    formatting::escape_terminal_controls(&ids.join(", "))
                ));
            }
            if let Some(response) = &error.details().response {
                rendered.push('\n');
                rendered.push_str(&render_api_response_human(response));
            }
            if error.details().file_saved == Some(true) {
                if let Some(destination) = &error.details().destination {
                    rendered.push_str(&format!(
                        "\nDownload saved to `{}`; completion output failed.",
                        formatting::escape_terminal_controls(destination)
                    ));
                }
            }
            rendered
        }
        OutputFormat::Json => serde_json::to_string(&ErrorEnvelope {
            ok: false,
            error: ErrorBody {
                code: error.code().as_str(),
                message: error.message(),
                details: error.details(),
            },
        })
        .expect("application errors are always serializable"),
        OutputFormat::Raw => format!(
            "Error [{}]: {}",
            error.code().as_str(),
            error.message().trim()
        ),
    }
}

#[cfg(test)]
mod tests;
