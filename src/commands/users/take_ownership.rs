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

const MEMBERSHIP_EFFECT: &str = "Wekan can deactivate your existing membership first, then reassigns the target's first membership entry to you. You can lose access if that entry is inactive. If the target has multiple entries, they can retain administrator access.";

#[derive(Debug, Args)]
#[command(
    after_help = format!("Wekan selects boards where the target has an active or inactive administrator membership. {MEMBERSHIP_EFFECT} Targeting yourself can deactivate your membership when another active board administrator exists.")
)]
pub struct TakeOwnershipArgs {
    /// Wekan user ID whose board administrator memberships to select.
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
        "Run the ownership action on your own account? Wekan can deactivate your membership on boards with another active administrator.".to_owned()
    } else {
        format!(
            "Take ownership of boards where user `{}` has an active or inactive administrator membership? {MEMBERSHIP_EFFECT}",
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
