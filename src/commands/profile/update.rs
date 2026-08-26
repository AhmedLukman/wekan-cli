use clap::Args;

use crate::{
    command_result::{CommandSuccess, ProfileItem},
    config::profiles::{Profile, ProfileStore, canonical_server, parse_profile_name},
    credentials::CredentialStore,
    error::AppError,
};

use super::{ensure_logged_out, profile_not_found};

#[derive(Debug, Args)]
pub struct UpdateArgs {
    #[arg(value_parser = parse_profile_name)]
    pub name: String,
    pub url: String,
}

pub(super) fn execute(
    args: UpdateArgs,
    profile_store: &dyn ProfileStore,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let server = canonical_server(&args.url).map_err(AppError::from)?;
    let mut mutation = profile_store.lock_mutation().map_err(AppError::from)?;
    let credential_namespace = profile_store
        .credential_namespace()
        .map_err(AppError::from)?;
    let current = mutation
        .document()
        .profile(&args.name)
        .ok_or_else(|| profile_not_found(&args.name))?;
    let active = mutation.document().active_profile() == Some(args.name.as_str());
    if current.server() == server {
        return Ok(CommandSuccess::ProfileUpdated(ProfileItem {
            name: args.name,
            server,
            active,
        }));
    }

    ensure_logged_out(
        credential_store,
        &args.name,
        current.server(),
        &credential_namespace,
    )?;
    let mut document = mutation.document().clone();
    document.insert(args.name.clone(), Profile::new(server.clone()));
    mutation.save(document)?;
    Ok(CommandSuccess::ProfileUpdated(ProfileItem {
        name: args.name,
        server,
        active,
    }))
}
