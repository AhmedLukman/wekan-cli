use crate::{
    client::WekanClientFactory,
    command_result::{
        CardAssigneeCollectionSuccess, CardMemberCollectionSuccess, CardMembershipKind,
        CardMembershipMutationSuccess, CommandSuccess, MembershipAction,
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
pub struct MembershipArgs {
    #[command(subcommand)]
    pub command: MembershipCommand,
}

#[derive(Debug, Subcommand)]
pub enum MembershipCommand {
    /// List user IDs in server order.
    List(Target),
    /// Add one active board member without replacing other assignments.
    Add(MutationArgs),
    /// Remove one user without replacing other assignments.
    Remove(MutationArgs),
}

#[derive(Debug, Args)]
pub struct Target {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    board_id: String,
    /// Wekan list ID.
    #[arg(long = "list", value_parser = non_empty, help_heading = "Target")]
    list_id: String,
    /// Wekan card ID.
    #[arg(long = "card", value_parser = non_empty, help_heading = "Target")]
    card_id: String,
}

#[derive(Debug, Args)]
pub struct MutationArgs {
    /// Existing Wekan user ID.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    user_id: String,
    #[command(flatten)]
    target: Target,
}

pub(super) async fn dispatch(
    command: MembershipCommand,
    kind: CardMembershipKind,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let client = context.client();
    let token = context.record().token();
    let (args, action) = match command {
        MembershipCommand::List(target) => {
            let card = client
                .card(&target.board_id, &target.list_id, &target.card_id, token)
                .await
                .map_err(|error| {
                    map_client_error_with_not_found(
                        error,
                        &redactor,
                        "card membership list",
                        "the requested Wekan card was not found in that board list",
                    )
                })?;
            return Ok(match kind {
                CardMembershipKind::Member => {
                    CommandSuccess::CardMembers(CardMemberCollectionSuccess {
                        board_id: target.board_id,
                        list_id: target.list_id,
                        card_id: card.card_id,
                        members: card.members,
                    })
                }
                CardMembershipKind::Assignee => {
                    CommandSuccess::CardAssignees(CardAssigneeCollectionSuccess {
                        board_id: target.board_id,
                        list_id: target.list_id,
                        card_id: card.card_id,
                        assignees: card.assignees,
                    })
                }
            });
        }
        MembershipCommand::Add(args) => (args, MembershipAction::Add),
        MembershipCommand::Remove(args) => (args, MembershipAction::Remove),
    };
    let target = args.target;
    let response = match (kind, action) {
        (CardMembershipKind::Member, MembershipAction::Add) => {
            client
                .add_card_member(
                    &target.board_id,
                    &target.list_id,
                    &target.card_id,
                    &args.user_id,
                    token,
                )
                .await
        }
        (CardMembershipKind::Member, MembershipAction::Remove) => {
            client
                .remove_card_member(
                    &target.board_id,
                    &target.list_id,
                    &target.card_id,
                    &args.user_id,
                    token,
                )
                .await
        }
        (CardMembershipKind::Assignee, MembershipAction::Add) => {
            client
                .add_card_assignee(
                    &target.board_id,
                    &target.list_id,
                    &target.card_id,
                    &args.user_id,
                    token,
                )
                .await
        }
        (CardMembershipKind::Assignee, MembershipAction::Remove) => {
            client
                .remove_card_assignee(
                    &target.board_id,
                    &target.list_id,
                    &target.card_id,
                    &args.user_id,
                    token,
                )
                .await
        }
    }
    .map_err(|error| {
        map_client_error_with_not_found(
            error,
            &redactor,
            "card membership mutation",
            "the requested active Wekan card was not found in that board list",
        )
    })?;
    Ok(CommandSuccess::CardMembershipChanged(
        CardMembershipMutationSuccess {
            board_id: target.board_id,
            list_id: target.list_id,
            card_id: response.card_id,
            user_id: args.user_id,
            kind,
            action,
            members: response.members,
            assignees: response.assignees,
        },
    ))
}
