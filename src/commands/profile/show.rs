use clap::Args;

use crate::{
    command_result::{CommandSuccess, ProfileItem},
    config::profiles::{ProfileDocument, ProfileStore, parse_profile_name},
    error::AppError,
};

use super::profile_not_found;

#[derive(Debug, Args)]
pub struct ShowArgs {
    #[arg(value_parser = parse_profile_name)]
    pub name: String,
}

pub(super) fn execute(
    args: ShowArgs,
    profile_store: &dyn ProfileStore,
) -> Result<CommandSuccess, AppError> {
    let snapshot = profile_store.read().map_err(AppError::from)?;
    let item = item_for(snapshot.document(), &args.name)?;
    Ok(CommandSuccess::ProfileShown(item))
}

fn item_for(document: &ProfileDocument, name: &str) -> Result<ProfileItem, AppError> {
    let profile = document
        .profile(name)
        .ok_or_else(|| profile_not_found(name))?;
    Ok(ProfileItem {
        name: name.to_owned(),
        server: profile.server().to_owned(),
        active: document.active_profile() == Some(name),
    })
}
