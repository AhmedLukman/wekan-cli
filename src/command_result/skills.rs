use serde::Serialize;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SkillTopicSummary {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SkillListSuccess {
    pub cli_version: String,
    pub topics: Vec<SkillTopicSummary>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SkillShowSuccess {
    pub cli_version: String,
    pub topic: String,
    pub content: String,
}
