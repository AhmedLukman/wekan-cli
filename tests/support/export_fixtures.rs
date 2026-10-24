use serde_json::{Value, json};
pub fn board() -> Value {
    json!({"_id":"board","title":"Delivery","members":[],"_format":"wekan-board-1.0.0","attachments":[],"lists":[],"swimlanes":[],"customFields":[],"cards":[],"comments":[],"activities":[],"checklists":[],"checklistItems":[],"subtaskItems":[],"rules":[],"triggers":[],"actions":[],"users":[]})
}
pub fn external(format: &str) -> Value {
    if format == "json" {
        return board();
    }
    let fixture = match format {
        "deck" => include_str!("../fixtures/responses/exports/v12.57/deck-minimal.json"),
        "kanboard" => include_str!("../fixtures/responses/exports/v12.57/kanboard-minimal.json"),
        "openproject" => {
            include_str!("../fixtures/responses/exports/v12.57/openproject-minimal.json")
        }
        "github" => include_str!("../fixtures/responses/exports/v12.57/github-minimal.json"),
        "gitlab" => include_str!("../fixtures/responses/exports/v12.57/gitlab-minimal.json"),
        "gitea" => include_str!("../fixtures/responses/exports/v12.57/gitea-minimal.json"),
        "forgejo" => include_str!("../fixtures/responses/exports/v12.57/forgejo-minimal.json"),
        "trello" => include_str!("../fixtures/responses/exports/v12.57/trello-minimal.json"),
        "jira" => include_str!("../fixtures/responses/exports/v12.57/jira-minimal.json"),
        "asana" => include_str!("../fixtures/responses/exports/v12.57/asana-minimal.json"),
        "zenkit" => include_str!("../fixtures/responses/exports/v12.57/zenkit-minimal.json"),
        "taskwarrior" => {
            include_str!("../fixtures/responses/exports/v12.57/taskwarrior-minimal.json")
        }
        "nullboard" => include_str!("../fixtures/responses/exports/v12.57/nullboard-minimal.json"),
        "kanri" => include_str!("../fixtures/responses/exports/v12.57/kanri-minimal.json"),
        "tasksorg" => include_str!("../fixtures/responses/exports/v12.57/tasksorg-minimal.json"),
        "superproductivity" => {
            include_str!("../fixtures/responses/exports/v12.57/superproductivity-minimal.json")
        }
        "taiga" => include_str!("../fixtures/responses/exports/v12.57/taiga-minimal.json"),
        "wrikeworkflow" => {
            include_str!("../fixtures/responses/exports/v12.57/wrikeworkflow-minimal.json")
        }
        _ => panic!("not JSON"),
    };
    serde_json::from_str(fixture).expect("matching-version native export fixture")
}
