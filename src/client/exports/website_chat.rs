//! Structured board exports from the website and chat formatters.
#![allow(dead_code)]

use super::validation::Date;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WpPost {
    id: u64,
    date_gmt: String,
    slug: String,
    status: String,
    #[serde(rename = "type")]
    record_type: String,
    title: WpTitle,
    content: WpContent,
    parent: u64,
    tags_names: Vec<String>,
    author_name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WpTitle {
    rendered: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WpContent {
    rendered: String,
    protected: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GhostExport {
    db: Vec<GhostDatabase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GhostDatabase {
    meta: GhostMetadata,
    data: GhostData,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GhostMetadata {
    exported_on: u64,
    version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GhostData {
    posts: Vec<GhostPost>,
    tags: Vec<GhostTag>,
    posts_tags: Vec<GhostPostTag>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GhostPost {
    id: String,
    title: String,
    slug: String,
    html: String,
    status: String,
    visibility: String,
    #[serde(rename = "type")]
    record_type: String,
    created_at: Date,
    #[serde(deserialize_with = "super::validation::required_nullable")]
    published_at: Option<Date>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GhostTag {
    id: String,
    name: String,
    slug: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GhostPostTag {
    id: String,
    post_id: String,
    tag_id: String,
    sort_order: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TeamsExport {
    teams: bool,
    chats: Vec<TeamsChat>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TeamsChat {
    id: String,
    topic: String,
    #[serde(rename = "chatType")]
    chat_type: String,
    kind: String,
    description: String,
    column: String,
    swimlane: String,
    messages: Vec<TeamsMessage>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TeamsMessage {
    id: String,
    #[serde(rename = "messageType")]
    message_type: String,
    #[serde(rename = "createdDateTime")]
    created_date_time: Date,
    from: TeamsSender,
    body: TeamsBody,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TeamsSender {
    user: TeamsUser,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TeamsUser {
    id: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TeamsBody {
    #[serde(rename = "contentType")]
    content_type: String,
    content: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MattermostTeam {
    name: String,
    display_name: String,
    #[serde(rename = "type")]
    record_type: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MattermostChannel {
    team: String,
    name: String,
    display_name: String,
    #[serde(rename = "type")]
    record_type: String,
    purpose: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MattermostUser {
    username: String,
    email: String,
    teams: Vec<MattermostUserTeam>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MattermostUserTeam {
    name: String,
    channels: Vec<MattermostChannelName>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MattermostChannelName {
    name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MattermostPost {
    team: String,
    channel: String,
    user: String,
    message: String,
    create_at: i64,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub(super) enum MattermostRecord {
    Version { version: u64 },
    Team { team: MattermostTeam },
    Channel { channel: MattermostChannel },
    User { user: MattermostUser },
    Post { post: MattermostPost },
}
