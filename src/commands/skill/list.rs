use clap::Args;

use crate::command_result::{CommandSuccess, SkillListSuccess, SkillTopicSummary};

use super::topics::TOPICS;

#[derive(Debug, Args)]
pub struct ListArgs {}

pub(super) fn execute(_args: ListArgs) -> CommandSuccess {
    let mut topics: Vec<_> = TOPICS
        .iter()
        .map(|topic| SkillTopicSummary {
            name: topic.name.to_owned(),
            description: topic.description.to_owned(),
        })
        .collect();
    topics.sort_by(|a, b| a.name.cmp(&b.name));
    CommandSuccess::SkillList(SkillListSuccess {
        cli_version: env!("CARGO_PKG_VERSION").to_owned(),
        topics,
    })
}
