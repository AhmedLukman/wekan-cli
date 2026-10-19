use clap::Args;

use crate::{
    command_result::{CommandSuccess, SkillShowSuccess},
    error::AppError,
};

use super::topics::TOPICS;

#[derive(Debug, Args)]
#[command(
    after_help = "Examples:\n  wekan skill show\n  wekan skill show cards\n  wekan --output json skill show agent-contract"
)]
pub struct ShowArgs {
    /// Topic name from `wekan skill list`.
    #[arg(default_value = "core", value_name = "TOPIC")]
    pub topic: String,
}

pub(super) fn execute(args: ShowArgs) -> Result<CommandSuccess, AppError> {
    let topic = TOPICS
        .iter()
        .find(|topic| topic.name == args.topic)
        .ok_or_else(|| {
            AppError::invalid_input(
                "unknown skill topic; run `wekan skill list` for available topics",
            )
        })?;
    Ok(CommandSuccess::SkillShown(SkillShowSuccess {
        cli_version: env!("CARGO_PKG_VERSION").to_owned(),
        topic: topic.name.to_owned(),
        content: topic.content.to_owned(),
    }))
}
