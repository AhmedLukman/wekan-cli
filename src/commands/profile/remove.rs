use clap::Args;

use crate::{
    command_result::{CommandSuccess, ProfileRemoveSuccess},
    config::profiles::{ProfileStore, parse_profile_name},
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
};

use super::{ensure_logged_out, profile_error, profile_not_found};

#[derive(Debug, Args)]
pub struct RemoveArgs {
    #[arg(value_parser = parse_profile_name)]
    pub name: String,
    /// Remove an active profile and clear the active selection.
    #[arg(long)]
    pub force: bool,
}

pub(super) fn execute(
    args: RemoveArgs,
    profile_store: &dyn ProfileStore,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
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
