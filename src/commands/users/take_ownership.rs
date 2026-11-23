use clap::Args;

use crate::{
    client::{UserAction, UserActionResult, WekanClientFactory},
    command_result::{
        CancellationSuccess, CommandSuccess, DestructiveOperation, UserBoardSummary,
        UserOwnershipSuccess,
    },
    commands::authenticated::AuthenticatedContext,
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    input::{
        ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
        confirm_or_skip,
    },
    redaction::Redactor,
};

use super::non_empty;
use crate::commands::client_error::map_client_error_with_not_found;

#[derive(Debug, Args)]
#[command(
    after_help = "Transfers boards where the target user holds an administrator membership. Self-targeting can deactivate your membership when another active board administrator exists."
)]
pub struct TakeOwnershipArgs {
    /// Wekan user ID whose board memberships Wekan will select.
    #[arg(value_parser = non_empty)]
    pub user_id: String,

    #[command(flatten)]
    pub confirmation: ConfirmationArgs,
}

pub(super) async fn execute(
    args: TakeOwnershipArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let effect = if args.user_id == context.record().user_id() {
        "Run Wekan's self ownership action, which can deactivate your board memberships?".to_owned()
    } else {
        format!(
            "Take ownership of boards administered by user `{}`?",
            args.user_id
        )
    };
    let request = ConfirmationRequest::new(DestructiveOperation::UserTakeOwnership, effect);
    if confirm_or_skip(&args.confirmation, confirmation, &request)?
        == ConfirmationDecision::Cancelled
    {
        return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::UserTakeOwnership,
        )));
    }
    let redactor = Redactor::with_secret(context.record().token());
    let boards = context
        .client()
        .user_action(
            &args.user_id,
            UserAction::TakeOwnership,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "ownership transfer",
                "user not found",
            )
        })?;
    let UserActionResult::OwnershipTransferred(boards) = boards else {
        return Err(AppError::new(
            ErrorCode::ProtocolError,
            "Wekan returned the wrong response shape for takeOwnership",
            StableExitCode::Server,
        ));
    };
    let boards = boards
        .into_iter()
        .map(|board| UserBoardSummary {
            board_id: board.board_id,
            title: board.title,
        })
        .collect();
    Ok(CommandSuccess::UserOwnershipTaken(UserOwnershipSuccess {
        from_user_id: args.user_id,
        to_user_id: context.record().user_id().to_owned(),
        boards,
    }))
}
