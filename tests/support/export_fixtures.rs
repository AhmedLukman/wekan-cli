use serde_json::{Value, json};
pub fn board() -> Value {
    json!({"_id":"board","title":"Delivery","members":[],"_format":"wekan-board-1.0.0","attachments":[],"lists":[],"swimlanes":[],"customFields":[],"cards":[],"comments":[],"activities":[],"checklists":[],"checklistItems":[],"subtaskItems":[],"rules":[],"triggers":[],"actions":[],"users":[]})
}
pub fn external(format: &str) -> Value {
    match format {
        "json" => board(),
        "kanboard" => {
            json!({"board":{"name":"Delivery"},"columns":[{"title":"Todo"}],"swimlanes":[{"name":"Default"}],"tasks":[{"title":"Ship","description":"Details","column_name":"Todo","swimlane_name":"Default","tags":["label"]}]})
        }
        "deck" => {
            json!({"title":"Delivery","stacks":[{"title":"Todo","cards":[{"title":"Ship","description":"Details","labels":[{"title":"label"}]}]}]})
        }
        "openproject" => {
            json!({"_embedded":{"elements":[{"subject":"Ship","description":{"raw":"Details"},"_links":{"status":{"title":"Todo"}}}]}})
        }
        "github" | "gitea" | "forgejo" => {
            json!([{"title":"Ship","body":"Details","state":"open","labels":[{"name":"label"}]}])
        }
        "gitlab" => {
            json!([{"title":"Ship","description":"Details","state":"opened","labels":["label"]}])
        }
        "trello" => {
            json!({"name":"Delivery","prefs":{"background":"blue","permissionLevel":"private"},"labels":[],"lists":[],"cards":[],"checklists":[],"actions":[]})
        }
        "jira" => {
            json!({"board":{"name":"Delivery"},"issues":[{"key":"WEKAN-1","fields":{"summary":"Ship","description":"Details","status":{"name":"Todo"},"labels":["label"]}}]})
        }
        "asana" => {
            json!({"data":[{"name":"Ship","notes":"Details","completed":false,"memberships":[{"section":{"name":"Todo"}}],"tags":[{"name":"label"}]}]})
        }
        "zenkit" => {
            json!({"title":"Delivery","stages":[{"name":"Todo"}],"items":[{"title":"Ship","description":"Details","stage_name":"Todo","tags":["label"]}]})
        }
        _ => panic!("not JSON"),
    }
}
