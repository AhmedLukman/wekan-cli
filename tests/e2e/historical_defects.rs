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
    // W11-018: a missing scoped card now returns the native JSON 404.
    let response = http
        .get(format!(
            "{canonical}api/boards/{board}/lists/{list}/cards/missing"
        ))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    assert_eq!(
        response.json::<Value>().await.unwrap(),
        json!({"error":"Card not found"})
    );
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

    // W11-020: reject missing placement references before creating the card.
    for (target_list, target_lane, reason) in [
        (
            "missing-list",
            lane,
            "listId does not belong to this board.",
        ),
        (
            list,
            "missing-lane",
            "swimlaneId does not belong to this board.",
        ),
    ] {
        let response = http
            .post(format!(
                "{canonical}api/boards/{board}/lists/{target_list}/cards"
            ))
            .bearer_auth(token)
            .json(&json!({"title":"Relationship probe", "swimlaneId":target_lane}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 400);
        assert_eq!(
            response.json::<Value>().await.unwrap(),
            json!({"error":reason})
        );
    }

    // W11-019: a title-only update on an archived card reports no match.
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
    let response = http
        .put(format!(
            "{canonical}api/boards/{board}/lists/{list}/cards/{id}"
        ))
        .bearer_auth(token)
        .json(&json!({"title":"Ignored archive title"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);
    assert_eq!(
        response.json::<Value>().await.unwrap(),
        json!({"message":"Error"})
    );
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

    // W11-024: the export switch rejects the request before starting a stream.
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
    let data = response.text().await.unwrap();
    fixture_command(
        "mongodb",
        "mongosh",
        "const modifier=p.present?{$set:{disableAllExport:p.value}}:{$unset:{disableAllExport:''}};db.getSiblingDB('wekan').settings.updateOne({_id:p.id},modifier);print('{}');",
        &previous,
    );
    assert_eq!(status, 403);
    assert_eq!(data, "Export is disabled by the administrator.");
}
