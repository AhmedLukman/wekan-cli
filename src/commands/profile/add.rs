use clap::Args;

use crate::{
    command_result::{CommandSuccess, ProfileItem},
    config::profiles::{Profile, ProfileStore, canonical_server, parse_profile_name},
    error::{AppError, ErrorCode},
};

use super::profile_error;

#[derive(Debug, Args)]
pub struct AddArgs {
    #[arg(value_parser = parse_profile_name)]
    pub name: String,
    pub url: String,
    /// Make this profile active, even when other profiles already exist.
    #[arg(long)]
    pub r#use: bool,
}

pub(super) fn execute(
    args: AddArgs,
    profile_store: &dyn ProfileStore,
) -> Result<CommandSuccess, AppError> {
    let server = canonical_server(&args.url).map_err(AppError::from)?;
    let mut mutation = profile_store.lock_mutation().map_err(AppError::from)?;
    if mutation.document().profile(&args.name).is_some() {
        return Err(profile_error(
            ErrorCode::ProfileAlreadyExists,
            &args.name,
            format!("profile `{}` already exists", args.name),
        ));
    }

    let mut document = mutation.document().clone();
    let activate = document.is_empty() || args.r#use;
    document.insert(args.name.clone(), Profile::new(server.clone()));
    if activate {
        document.set_active_profile(Some(args.name.clone()));
    }
    mutation.save(document)?;

    Ok(CommandSuccess::ProfileAdded(ProfileItem {
        name: args.name,
        server,
        active: activate,
    }))
}
