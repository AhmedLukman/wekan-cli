use clap::Args;

use crate::{
    command_result::{CommandSuccess, ProfileItem},
    config::profiles::{ProfileStore, parse_profile_name},
    error::AppError,
};

use super::profile_not_found;

#[derive(Debug, Args)]
pub struct UseArgs {
    #[arg(value_parser = parse_profile_name)]
    pub name: String,
}

pub(super) fn execute(
    args: UseArgs,
    profile_store: &dyn ProfileStore,
) -> Result<CommandSuccess, AppError> {
    let mut mutation = profile_store.lock_mutation().map_err(AppError::from)?;
    let profile = mutation
        .document()
        .profile(&args.name)
        .ok_or_else(|| profile_not_found(&args.name))?;
    let server = profile.server().to_owned();
    if mutation.document().active_profile() != Some(args.name.as_str()) {
        let mut document = mutation.document().clone();
        document.set_active_profile(Some(args.name.clone()));
        mutation.save(document)?;
    }
    Ok(CommandSuccess::ProfileUsed(ProfileItem {
        name: args.name,
        server,
        active: true,
    }))
}
