use clap::Args;

use crate::{
    command_result::{CommandSuccess, ProfileItem, ProfileListSuccess},
    config::profiles::{ProfileDocument, ProfileStore},
    error::AppError,
};

#[derive(Debug, Args)]
pub struct ListArgs {}

pub(super) fn execute(
    _args: ListArgs,
    profile_store: &dyn ProfileStore,
) -> Result<CommandSuccess, AppError> {
    let snapshot = profile_store.read().map_err(AppError::from)?;
    let document = snapshot.document();
    Ok(CommandSuccess::ProfileList(ProfileListSuccess {
        active_profile: document.active_profile().map(str::to_owned),
        profiles: profile_items(document),
    }))
}

fn profile_items(document: &ProfileDocument) -> Vec<ProfileItem> {
    document
        .profiles()
        .iter()
        .map(|(name, profile)| ProfileItem {
            name: name.clone(),
            server: profile.server().to_owned(),
            active: document.active_profile() == Some(name.as_str()),
        })
        .collect()
}
