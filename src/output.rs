mod auth;
mod boards;
mod cards;
mod comments;
mod formatting;
mod lists;
mod profiles;
mod raw;
mod swimlanes;
mod users;

use auth::{render_auth_status, render_auth_success, render_logout};
use boards::{
    render_board_count, render_board_created, render_board_deleted, render_board_detail,
    render_board_list, render_board_renamed,
};
use cards::{
    render_card_collection, render_card_created, render_card_deleted, render_card_detail,
    render_card_updated,
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

use std::{ffi::OsString, io::Write, process::ExitCode};

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
        let mut iter = args.iter().filter_map(|arg| arg.to_str());
        while let Some(arg) = iter.next() {
            if arg == "--output" {
                return match iter.next() {
                    Some("json") => Self::Json,
                    Some("raw") => Self::Raw,
                    _ => Self::Human,
                };
            }
            if arg == "--output=json" {
                return Self::Json;
            }
            if arg == "--output=raw" {
                return Self::Raw;
            }
        }
        Self::Human
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
        (_, CommandSuccess::ApiResponse(_)) => {
            panic!("streaming API responses must be consumed by output::write_success")
        }
        (OutputFormat::Raw, CommandSuccess::Cancelled(_)) => String::new(),
        (OutputFormat::Raw, _) => panic!("raw output is valid only for api request"),
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
        | (OutputFormat::Human, CommandSuccess::UserShown(data)) => render_user_detail(data),
        (OutputFormat::Json, CommandSuccess::UserCurrent(data))
        | (OutputFormat::Json, CommandSuccess::UserShown(data)) => render_json_success(data),
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
        (OutputFormat::Human, CommandSuccess::CardDeleted(data)) => render_card_deleted(data),
        (OutputFormat::Json, CommandSuccess::CardDeleted(data)) => render_json_success(data),
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
    let rendered = render_success(format, &success);
    if !rendered.is_empty() {
        writeln!(stdout, "{rendered}").map_err(output_write_error)?;
    }
    Ok(ExitCode::SUCCESS)
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
            if let Some(response) = &error.details().response {
                rendered.push('\n');
                rendered.push_str(&render_api_response_human(response));
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
