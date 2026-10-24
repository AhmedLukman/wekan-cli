//! Historical cases that need direct requests in addition to typed commands.
use super::*;

pub(super) async fn verify(
    app: &LiveApp,
    server: &str,
    canonical: &str,
    token: &str,
    scope: (&str, &str, &str),
) {
    let (board, lane, list) = scope;
    let http = reqwest::Client::new();
    // W11-018 survives: an absent scoped card is an empty HTTP-200 response.
    let response = http
        .get(format!(
            "{canonical}api/boards/{board}/lists/{list}/cards/missing"
        ))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert!(response.bytes().await.unwrap().is_empty());
    // W11-019's missing/wrong-scope cases are fixed by the beforeEdit guard.
    let response = http
        .put(format!(
            "{canonical}api/boards/{board}/lists/{list}/cards/missing"
        ))
        .bearer_auth(token)
        .json(&json!({"title":"No match"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    assert_eq!(
        response.json::<Value>().await.unwrap(),
        json!({"error":"Card not found"})
    );

    // W11-020 survives: creation stores relationships without checking their existence.
    for (target_list, target_lane) in [("missing-list", lane), (list, "missing-lane")] {
        let made = run(
            app,
            server,
            &[
                "card",
                "create",
                "--board",
                board,
                "--list",
                target_list,
                "--swimlane-id",
                target_lane,
                "--title",
                "Relationship probe",
            ],
        )
        .await;
        let id = made["card_id"].as_str().unwrap();
        let stored = run(app, server, &["card", "get", id]).await;
        assert_eq!(stored["list_id"], target_list);
        assert_eq!(stored["swimlane_id"], target_lane);
        run(
            app,
            server,
            &[
                "card",
                "delete",
                id,
                "--board",
                board,
                "--list",
                target_list,
                "--yes",
            ],
        )
        .await;
    }

    // The archived-card portion of W11-019 still acknowledges an unmatched update.
    let made = run(
        app,
        server,
        &[
            "card",
            "create",
            "--board",
            board,
            "--list",
            list,
            "--swimlane-id",
            lane,
            "--title",
            "Archived update probe",
        ],
    )
    .await;
    let id = made["card_id"].as_str().unwrap();
    run(
        app,
        server,
        &["card", "archive", id, "--board", board, "--list", list],
    )
    .await;
    let accepted = run(
        app,
        server,
        &[
            "card",
            "update",
            id,
            "--board",
            board,
            "--list",
            list,
            "--title",
            "Ignored archive title",
        ],
    )
    .await;
    assert_eq!(accepted["submitted_fields"], json!(["title"]));
    let stored = run(app, server, &["card", "get", id]).await;
    assert_eq!(stored["archived"], true);
    assert_eq!(stored["title"], "Archived update probe");
    run(
        app,
        server,
        &[
            "card", "delete", id, "--board", board, "--list", list, "--yes",
        ],
    )
    .await;

    // W11-024: the streaming JSON wrapper still changes export-disabled to HTTP 500.
    use crate::support::attachment_fixtures::fixture_command;
    let previous = fixture_command(
        "mongodb",
        "mongosh",
        "const d=db.getSiblingDB('wekan'),s=d.settings.findOne({});print(JSON.stringify({id:s._id,present:Object.hasOwn(s,'disableAllExport'),value:s.disableAllExport}));",
        &json!({}),
    );
    fixture_command(
        "mongodb",
        "mongosh",
        "db.getSiblingDB('wekan').settings.updateOne({_id:p.id},{$set:{disableAllExport:true}});print('{}');",
        &previous,
    );
    let response = http
        .get(format!("{canonical}api/boards/{board}/export"))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let data = response.json::<Value>().await.unwrap();
    fixture_command(
        "mongodb",
        "mongosh",
        "const modifier=p.present?{$set:{disableAllExport:p.value}}:{$unset:{disableAllExport:''}};db.getSiblingDB('wekan').settings.updateOne({_id:p.id},modifier);print('{}');",
        &previous,
    );
    assert_eq!(status, 500);
    assert_eq!(data, json!({"error":"Export failed"}));
}
