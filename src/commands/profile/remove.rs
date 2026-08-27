use clap::Args;

use crate::{
    command_result::{
        CancellationSuccess, CommandSuccess, DestructiveOperation, ProfileRemoveSuccess,
    },
    config::profiles::{ProfileStore, parse_profile_name},
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    input::{
        ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
        confirm_or_skip,
    },
};

use super::{ensure_logged_out, profile_error, profile_not_found};

#[derive(Debug, Args)]
pub struct RemoveArgs {
    #[arg(value_parser = parse_profile_name)]
    pub name: String,
    /// Remove an active profile and clear the active selection.
    #[arg(long)]
    pub force: bool,

    #[command(flatten)]
    pub confirmation: ConfirmationArgs,
}

struct RemovalPreview {
    revision: u64,
    server: String,
    active: bool,
}

pub(super) fn execute(
    args: RemoveArgs,
    profile_store: &dyn ProfileStore,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    let preview = preview_removal(&args, profile_store, credential_store)?;
    let prompt = if preview.active {
        format!(
            "Remove active profile `{}` for {} and clear the active selection?",
            args.name, preview.server
        )
    } else {
        format!("Remove profile `{}` for {}?", args.name, preview.server)
    };
    let request = ConfirmationRequest::new(DestructiveOperation::ProfileRemove, prompt);
    match confirm_or_skip(&args.confirmation, confirmation, &request)? {
        ConfirmationDecision::Proceed => {}
        ConfirmationDecision::Cancelled => {
            return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
                DestructiveOperation::ProfileRemove,
            )));
        }
    }

    let mut mutation = profile_store.lock_mutation().map_err(AppError::from)?;
    let credential_namespace = profile_store
        .credential_namespace()
        .map_err(AppError::from)?;
    let profile = mutation
        .document()
        .profile(&args.name)
        .ok_or_else(|| profile_not_found(&args.name))?;
    let server = profile.server().to_owned();
    let active = mutation.document().active_profile() == Some(args.name.as_str());
    if preview.revision != mutation.document().revision()
        || preview.server != server
        || preview.active != active
    {
        return Err(AppError::configuration(format!(
            "profile `{}` changed while awaiting confirmation; retry the removal",
            args.name
        )));
    }
    if active && !args.force {
        return Err(profile_error(
            ErrorCode::ProfileInUse,
            &args.name,
            format!(
                "profile `{}` is active; switch profiles first or pass --force to remove it and clear the active selection",
                args.name
            ),
        ));
    }

    ensure_logged_out(credential_store, &args.name, &server, &credential_namespace)?;
    let mut document = mutation.document().clone();
    document.remove(&args.name);
    if active {
        document.set_active_profile(None);
    }
    let active_profile = document.active_profile().map(str::to_owned);
    mutation.save(document)?;
    Ok(CommandSuccess::ProfileRemoved(ProfileRemoveSuccess {
        name: args.name,
        server,
        removed: true,
        active_profile,
    }))
}

fn preview_removal(
    args: &RemoveArgs,
    profile_store: &dyn ProfileStore,
    credential_store: &dyn CredentialStore,
) -> Result<RemovalPreview, AppError> {
    let snapshot = profile_store.read().map_err(AppError::from)?;
    let credential_namespace = profile_store
        .credential_namespace()
        .map_err(AppError::from)?;
    let profile = snapshot
        .document()
        .profile(&args.name)
        .ok_or_else(|| profile_not_found(&args.name))?;
    let server = profile.server().to_owned();
    let active = snapshot.document().active_profile() == Some(args.name.as_str());
    if active && !args.force {
        return Err(profile_error(
            ErrorCode::ProfileInUse,
            &args.name,
            format!(
                "profile `{}` is active; switch profiles first or pass --force to remove it and clear the active selection",
                args.name
            ),
        ));
    }
    ensure_logged_out(credential_store, &args.name, &server, &credential_namespace)?;

    Ok(RemovalPreview {
        revision: snapshot.document().revision(),
        server,
        active,
    })
}
