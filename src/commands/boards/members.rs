use crate::{
    client::{WekanClientFactory, boards::members::BoardMemberPermissions},
    command_result::{
        BoardMemberCollectionSuccess, BoardMemberMutationSuccess, BoardMemberSubmittedField,
        BoardMemberUpdateSuccess, BoardSummary, CommandSuccess, MembershipAction,
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
pub struct MemberArgs {
    #[command(subcommand)]
    pub command: MemberCommand,
}

#[derive(Debug, Subcommand)]
pub enum MemberCommand {
    /// List all board membership records, including inactive members.
    List(ListArgs),
    /// Add or reactivate a board member; active members keep their permissions.
    Add(AddArgs),
    /// Request member deactivation; Wekan preserves its sole active administrator.
    Remove(MutationTarget),
    /// Set member permissions directly; omitted flags are not preserved as a patch.
    Update(UpdateArgs),
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    board_id: String,
}

#[derive(Debug, Args)]
pub struct MutationTarget {
    /// Existing Wekan user ID.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    user_id: String,
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    board_id: String,
}

#[derive(Debug, Args)]
#[command(
    after_help = "The response acknowledges the request without confirming a membership change. A named role wins over individual flags."
)]
pub struct AddArgs {
    #[command(flatten)]
    target: MutationTarget,
    #[command(flatten)]
    permissions: PermissionArgs,
}

#[derive(Debug, Args)]
#[command(group(clap::ArgGroup::new("permission_input").required(true).multiple(true).args([
    "role", "is_admin", "is_no_comments", "is_comment_only", "is_worker", "is_normal_assigned_only", "is_comment_assigned_only", "is_read_only", "is_read_assigned_only"
])), after_help = "Wekan writes permission flags directly; this is not a partial patch. A named role wins over flags. Self-update preserves the current administrator flag. The result reports the affected-document count, not saved permissions.")]
pub struct UpdateArgs {
    #[command(flatten)]
    target: MutationTarget,
    #[command(flatten)]
    permissions: PermissionArgs,
}

#[derive(Debug, Args)]
#[command(next_help_heading = "Permissions")]
struct PermissionArgs {
    /// Named role: admin, normal, member, comment, commentonly, readonly, worker,
    /// normalassignedonly, commentassignedonly, readassignedonly, or nocomments.
    /// Case-insensitive; overrides individual flags.
    #[arg(long, value_parser = role, value_name = "ROLE")]
    role: Option<String>,
    /// Board administrator flag.
    #[arg(long)]
    is_admin: Option<bool>,
    /// Disable comments.
    #[arg(long)]
    is_no_comments: Option<bool>,
    /// Comment-only flag.
    #[arg(long)]
    is_comment_only: Option<bool>,
    /// Worker flag.
    #[arg(long)]
    is_worker: Option<bool>,
    /// Normal access limited to assigned cards.
    #[arg(long)]
    is_normal_assigned_only: Option<bool>,
    /// Comment access limited to assigned cards.
    #[arg(long)]
    is_comment_assigned_only: Option<bool>,
    /// Read-only flag.
    #[arg(long)]
    is_read_only: Option<bool>,
    /// Read access limited to assigned cards.
    #[arg(long)]
    is_read_assigned_only: Option<bool>,
}

impl PermissionArgs {
    fn into_request(self) -> BoardMemberPermissions {
        BoardMemberPermissions {
            role: self.role,
            is_admin: self.is_admin,
            is_no_comments: self.is_no_comments,
            is_comment_only: self.is_comment_only,
            is_worker: self.is_worker,
            is_normal_assigned_only: self.is_normal_assigned_only,
            is_comment_assigned_only: self.is_comment_assigned_only,
            is_read_only: self.is_read_only,
            is_read_assigned_only: self.is_read_assigned_only,
        }
    }
}

fn role(value: &str) -> Result<String, String> {
    let value = crate::client::trim_javascript_whitespace(value).to_ascii_lowercase();
    if matches!(
        value.as_str(),
        "admin"
            | "normal"
            | "member"
            | "comment"
            | "commentonly"
            | "readonly"
            | "worker"
            | "normalassignedonly"
            | "commentassignedonly"
            | "readassignedonly"
            | "nocomments"
    ) {
        Ok(value)
    } else {
        Err("use admin, normal, member, comment, commentonly, readonly, worker, normalassignedonly, commentassignedonly, readassignedonly, or nocomments".into())
    }
}

pub(super) async fn dispatch(
    command: MemberCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let client = context.client();
    let token = context.record().token();
    match command {
        MemberCommand::List(args) => {
            let board = client.board(&args.board_id, token).await.map_err(|error| {
                map_client_error_with_not_found(
                    error,
                    &redactor,
                    "board member list",
                    "the requested Wekan board was not found",
                )
            })?;
            Ok(CommandSuccess::BoardMembers(BoardMemberCollectionSuccess {
                board_id: board.board_id,
                members: board
                    .members
                    .into_iter()
                    .map(super::get::board_detail_member)
                    .collect(),
            }))
        }
        MemberCommand::Update(args) => {
            let permissions = args.permissions.into_request();
            let submitted_fields = submitted_fields(&permissions);
            let affected_documents = client
                .update_board_member(
                    &args.target.board_id,
                    &args.target.user_id,
                    &permissions,
                    token,
                )
                .await
                .map_err(|error| {
                    map_mutation_client_error_with_not_found(
                        error,
                        &redactor,
                        "board member update",
                        "the requested Wekan board member scope was not found",
                    )
                })?;
            Ok(CommandSuccess::BoardMemberUpdated(
                BoardMemberUpdateSuccess {
                    board_id: args.target.board_id,
                    user_id: args.target.user_id,
                    affected_documents,
                    submitted_fields,
                },
            ))
        }
        mutation => {
            let (target, action, response) = match mutation {
                MemberCommand::Add(args) => {
                    let response = client
                        .add_board_member(
                            &args.target.board_id,
                            &args.target.user_id,
                            &args.permissions.into_request(),
                            token,
                        )
                        .await;
                    (args.target, MembershipAction::Add, response)
                }
                MemberCommand::Remove(target) => {
                    let response = client
                        .remove_board_member(&target.board_id, &target.user_id, token)
                        .await;
                    (target, MembershipAction::Remove, response)
                }
                _ => unreachable!("list and update are handled above"),
            };
            let boards = response.map_err(|error| {
                map_mutation_client_error_with_not_found(
                    error,
                    &redactor,
                    "board membership mutation",
                    "the requested Wekan board member scope was not found",
                )
            })?;
            Ok(CommandSuccess::BoardMemberChanged(
                BoardMemberMutationSuccess {
                    board_id: target.board_id,
                    user_id: target.user_id,
                    action,
                    boards: boards
                        .into_iter()
                        .map(|board| BoardSummary {
                            board_id: board.board_id,
                            title: board.title,
                        })
                        .collect(),
                    mutation_confirmed: false,
                },
            ))
        }
    }
}

fn submitted_fields(request: &BoardMemberPermissions) -> Vec<BoardMemberSubmittedField> {
    use BoardMemberSubmittedField::*;
    [
        (request.role.is_some(), Role),
        (request.is_admin.is_some(), IsAdmin),
        (request.is_no_comments.is_some(), IsNoComments),
        (request.is_comment_only.is_some(), IsCommentOnly),
        (request.is_worker.is_some(), IsWorker),
        (
            request.is_normal_assigned_only.is_some(),
            IsNormalAssignedOnly,
        ),
        (
            request.is_comment_assigned_only.is_some(),
            IsCommentAssignedOnly,
        ),
        (request.is_read_only.is_some(), IsReadOnly),
        (request.is_read_assigned_only.is_some(), IsReadAssignedOnly),
    ]
    .into_iter()
    .filter_map(|(present, field)| present.then_some(field))
    .collect()
}
