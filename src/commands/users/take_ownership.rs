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

use super::{map_client_error, non_empty};

#[derive(Debug, Args)]
#[command(
    after_help = "Wekan v11.06 can select boards where the target is an inactive or non-administrator member. Returned board summaries do not confirm completed transfers. Self-targeting can deactivate your membership when another active board administrator exists. See `wekan skill show users` for details."
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
            "Run Wekan's ownership action for user `{}`, including boards where that user is not an administrator?",
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
        .map_err(|error| map_client_error(error, &redactor, "ownership transfer"))?;
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
