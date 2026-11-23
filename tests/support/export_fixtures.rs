use serde_json::{Value, json};
pub fn board() -> Value {
    json!({"_id":"board","title":"Delivery","members":[],"_format":"wekan-board-1.0.0","attachments":[],"lists":[],"swimlanes":[],"customFields":[],"cards":[],"comments":[],"activities":[],"checklists":[],"checklistItems":[],"subtaskItems":[],"rules":[],"triggers":[],"actions":[],"users":[]})
}
pub fn external(format: &str) -> Value {
    if format == "json" {
        return board();
    }
    let fixture = match format {
        "deck" => include_str!("../fixtures/responses/exports/v13.28/deck-minimal.json"),
        "kanboard" => include_str!("../fixtures/responses/exports/v13.28/kanboard-minimal.json"),
        "openproject" => {
            include_str!("../fixtures/responses/exports/v13.28/openproject-minimal.json")
        }
        "github" => include_str!("../fixtures/responses/exports/v13.28/github-minimal.json"),
        "gitlab" => include_str!("../fixtures/responses/exports/v13.28/gitlab-minimal.json"),
        "gitea" => include_str!("../fixtures/responses/exports/v13.28/gitea-minimal.json"),
        "forgejo" => include_str!("../fixtures/responses/exports/v13.28/forgejo-minimal.json"),
        "trello" => include_str!("../fixtures/responses/exports/v13.28/trello-minimal.json"),
        "jira" => include_str!("../fixtures/responses/exports/v13.28/jira-minimal.json"),
        "asana" => include_str!("../fixtures/responses/exports/v13.28/asana-minimal.json"),
        "zenkit" => include_str!("../fixtures/responses/exports/v13.28/zenkit-minimal.json"),
        "taskwarrior" => {
            include_str!("../fixtures/responses/exports/v13.28/taskwarrior-minimal.json")
        }
        "nullboard" => include_str!("../fixtures/responses/exports/v13.28/nullboard-minimal.json"),
        "kanri" => include_str!("../fixtures/responses/exports/v13.28/kanri-minimal.json"),
        "tasksorg" => include_str!("../fixtures/responses/exports/v13.28/tasksorg-minimal.json"),
        "superproductivity" => {
            include_str!("../fixtures/responses/exports/v13.28/superproductivity-minimal.json")
        }
        "taiga" => include_str!("../fixtures/responses/exports/v13.28/taiga-minimal.json"),
        "wrikeworkflow" => {
            include_str!("../fixtures/responses/exports/v13.28/wrikeworkflow-minimal.json")
        }
        "wordpressjson" => {
            include_str!("../fixtures/responses/exports/v13.28/wordpressjson-minimal.json")
        }
        "ghost" => include_str!("../fixtures/responses/exports/v13.28/ghost-minimal.json"),
        "teams" => include_str!("../fixtures/responses/exports/v13.28/teams-minimal.json"),
        _ => panic!("not JSON"),
    };
    serde_json::from_str(fixture).expect("matching-version native export fixture")
}
