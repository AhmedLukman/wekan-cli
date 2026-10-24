//! Native administrator routes, permissions, and independent problem-state oracles.
use super::super::support::{
    attachment_fixtures::fixture_command,
    live::{LiveApp, run},
    *,
};
use serde_json::{Value, json};

pub(super) async fn verify(app: &LiveApp, server: &str, nonce: u128) {
    let directory = TestDirectory::new("native-admin-permissions");
    let ordinary = App::with_profile_store(
        ServerSelection::new(Some(server.into()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Native-admin-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let user = run(
        &ordinary,
        server,
        &[
            "auth",
            "register",
            "--username",
            &format!("admin_reader_{nonce}"),
            "--password-stdin",
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        run(&ordinary, server, &["user", "current"]).await.unwrap()["is_admin"],
        false
    );
    for command in [
        vec!["admin", "oauth-provider", "get"],
        vec![
            "admin",
            "oauth-provider",
            "update",
            "google",
            "--enabled",
            "false",
            "--id",
            "denied",
        ],
        vec!["admin", "passwordless", "update", "--enabled", "false"],
        vec!["admin", "problem", "list"],
        vec!["admin", "problem", "get", "tests"],
        vec!["admin", "problem", "acknowledge", "tests"],
    ] {
        let error = run(&ordinary, server, &command).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::PermissionDenied, "{command:?}");
        assert_eq!(error.details().http_status, Some(403), "{command:?}");
    }
    let http = reqwest::Client::new();
    let canonical = ServerUrl::parse(server).unwrap().as_str().to_owned();
    for (method, path, body) in [
        (reqwest::Method::GET, "api/admin/oauth-providers", None),
        (
            reqwest::Method::PUT,
            "api/admin/oauth-providers/google",
            Some(json!({"enabled":false,"id":"denied"})),
        ),
        (
            reqwest::Method::PUT,
            "api/admin/passwordless",
            Some(json!({"enabled":false})),
        ),
        (reqwest::Method::GET, "api/admin/problems", None),
        (reqwest::Method::GET, "api/admin/problems/tests", None),
        (
            reqwest::Method::POST,
            "api/admin/problems/tests/acknowledge",
            None,
        ),
    ] {
        let mut request = http.request(method, format!("{canonical}{path}"));
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.unwrap();
        assert_eq!(response.status(), 401, "{path}");
        assert_eq!(
            response.json::<Value>().await.unwrap()["error"],
            "Unauthorized"
        );
    }

    let providers = run(app, server, &["admin", "oauth-provider", "get"])
        .await
        .unwrap();
    assert!(providers["providers"]["google"]["secret"]["has_value"].is_boolean());
    run(
        app,
        server,
        &[
            "admin",
            "oauth-provider",
            "update",
            "google",
            "--enabled",
            "false",
            "--id",
            "upgrade-client",
            "--global-login-style",
            "popup",
            "--merge-existing-users",
            "true",
        ],
    )
    .await
    .unwrap();
    let providers = run(app, server, &["admin", "oauth-provider", "get"])
        .await
        .unwrap();
    assert_eq!(
        providers["providers"]["google"]["id"]["value"],
        "upgrade-client"
    );
    assert_eq!(providers["providers"]["google"]["enabled"]["value"], false);
    assert_eq!(providers["merge_existing_users"]["value"], true);
    for enabled in ["true", "false"] {
        let result = run(
            app,
            server,
            &["admin", "passwordless", "update", "--enabled", enabled],
        )
        .await
        .unwrap();
        assert_eq!(result["passwordless"]["value"], enabled == "true");
        let providers = run(app, server, &["admin", "oauth-provider", "get"])
            .await
            .unwrap();
        assert_eq!(providers["passwordless"]["value"], enabled == "true");
    }
    verify_problems(app, server, nonce).await;
    verify_user_channels(app, &ordinary, server, user["user_id"].as_str().unwrap()).await;
    run(
        app,
        server,
        &["user", "delete", user["user_id"].as_str().unwrap(), "--yes"],
    )
    .await
    .unwrap();
    run(
        &ordinary,
        server,
        &["auth", "logout", "--local-only", "--yes"],
    )
    .await
    .unwrap();
}

async fn verify_user_channels(admin: &LiveApp, ordinary: &LiveApp, server: &str, user: &str) {
    let before = run(ordinary, server, &["user", "current"]).await.unwrap();
    assert!(before["web_push_subscriptions"].is_null());
    assert!(before["ntfy"].is_null());
    let token = ordinary
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let endpoint = "https://fcm.googleapis.com/fcm/send/native-private-endpoint-sentinel";
    let auth = "native-private-auth-sentinel";
    let topic = "https://ntfy.example.test/native-private-topic-sentinel";
    let ntfy_token = "native-ntfy-secret-sentinel";
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        // Native Member Settings producers validate and save these shapes;
        // a disabled ntfy setting and subscription registration send no mail.
        let mut session = ddp::Session::login(server, &token).await;
        session
            .call(
                "webPushSubscribe",
                json!([{"endpoint":endpoint,"keys":{"p256dh":"public_key_demo","auth":auth}}]),
            )
            .await;
        session
            .call(
                "saveNtfySettings",
                json!([{"enabled":false,"url":topic,"token":ntfy_token}]),
            )
            .await;
        for (reader, args) in [
            (ordinary, vec!["user", "current"]),
            (admin, vec!["user", "get", user]),
            (admin, vec!["user", "update", user]),
        ] {
            let output = run(reader, server, &args).await.unwrap();
            assert_eq!(
                output["web_push_subscriptions"][0]["endpoint_present"],
                true
            );
            assert_eq!(
                output["web_push_subscriptions"][0]["keys"]["auth_present"],
                true
            );
            assert!(
                output["web_push_subscriptions"][0]["created_at"]
                    .as_str()
                    .is_some()
            );
            assert_eq!(
                output["ntfy"],
                json!({"enabled":false,"url_present":true,"token_present":true})
            );
            for secret in [endpoint, auth, topic, ntfy_token] {
                assert!(!output.to_string().contains(secret));
            }
        }
        session.call("webPushUnsubscribe", json!([endpoint])).await;
        session
            .call(
                "saveNtfySettings",
                json!([{"enabled":false,"url":"","token":""}]),
            )
            .await;
        session.close().await;
    })
    .await
    .expect("native user-channel producers timed out");
    let after = run(ordinary, server, &["user", "current"]).await.unwrap();
    assert_eq!(after["web_push_subscriptions"], json!([]));
    assert_eq!(
        after["ntfy"],
        json!({"enabled":false,"url_present":false,"token_present":false})
    );
}

fn problem_state() -> Value {
    // models/lib/eventLogProblems.js counts documents newer than the stored
    // acknowledgment, excluding informational rows; it does not sum count.
    fixture_command(
        "mongodb",
        "mongosh",
        r#"
        const w=db.getSiblingDB('wekan'),ack=w.eventlogAcks.findOne({stream:'tests'});
        const selector={stream:'tests',severity:{$ne:'info'}};
        if(ack)selector.at={$gt:ack.at};
        print(JSON.stringify({newCount:w.eventlog.countDocuments(selector),ackAt:ack&&ack.at}));
    "#,
        &json!({}),
    )
}

fn new_tests_count(overview: &Value) -> u64 {
    overview["new_problems"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["stream"] == "tests")
        .map_or(0, |entry| entry["count"].as_u64().unwrap())
}

async fn verify_problems(app: &LiveApp, server: &str, nonce: u128) {
    let overview = run(app, server, &["admin", "problem", "list"])
        .await
        .unwrap();
    for stream in overview["streams"].as_array().unwrap() {
        let stream = stream.as_str().unwrap();
        let page = run(
            app,
            server,
            &[
                "admin", "problem", "get", stream, "--limit", "1", "--skip", "0",
            ],
        )
        .await
        .unwrap();
        assert_eq!(page["stream"], stream);
        assert_eq!(page["limit"], 1);
        assert_eq!(page["skip"], 0);
    }
    run(
        app,
        server,
        &["admin", "problem", "get", "api", "--search", "/api/boards"],
    )
    .await
    .unwrap();
    run(app, server, &["admin", "problem", "acknowledge", "tests"])
        .await
        .unwrap();
    let marker = format!("problem-oracle-{nonce}");
    let inserted = fixture_command(
        "mongodb",
        "mongosh",
        r#"
        const w=db.getSiblingDB('wekan'),base=Date.now();
        const rows=[0,1,2].map(i=>({_id:p.marker+'-'+i,stream:'tests',at:new Date(base+i),
            severity:i===2?'info':'high',source:p.marker,detail:p.marker+(i===1?' excluded':' selected'),count:7}));
        w.eventlog.insertMany(rows);
        print(JSON.stringify(rows.map(row=>row._id)));
    "#,
        &json!({"marker":marker}),
    );
    let selected = format!("{} SELECTED", marker.to_uppercase());
    let full = run(
        app,
        server,
        &["admin", "problem", "get", "tests", "--search", &selected],
    )
    .await
    .unwrap();
    assert_eq!(full["total"], 2);
    assert_eq!(
        full["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| &row["event_id"])
            .collect::<Vec<_>>(),
        vec![&inserted[2], &inserted[0]]
    );
    let page = run(
        app,
        server,
        &[
            "admin", "problem", "get", "tests", "--search", &selected, "--limit", "1", "--skip",
            "1",
        ],
    )
    .await
    .unwrap();
    assert_eq!(page["total"], 2);
    assert_eq!(page["limit"], 1);
    assert_eq!(page["skip"], 1);
    assert_eq!(page["rows"].as_array().unwrap().len(), 1);
    assert_eq!(page["rows"][0]["event_id"], inserted[0]);
    let absent = run(
        app,
        server,
        &[
            "admin",
            "problem",
            "get",
            "tests",
            "--search",
            &format!("{marker}-missing"),
        ],
    )
    .await
    .unwrap();
    assert_eq!(absent["total"], 0);
    assert_eq!(absent["rows"], json!([]));
    let before = problem_state();
    assert!(before["newCount"].as_u64().unwrap() >= 2);
    let overview = run(app, server, &["admin", "problem", "list"])
        .await
        .unwrap();
    assert_eq!(
        new_tests_count(&overview),
        before["newCount"].as_u64().unwrap()
    );
    let acknowledgment = run(app, server, &["admin", "problem", "acknowledge", "tests"])
        .await
        .unwrap();
    assert_eq!(acknowledgment["acknowledged"], "tests");
    let after = problem_state();
    assert!(after["ackAt"].as_str().unwrap() > before["ackAt"].as_str().unwrap());
    assert_eq!(after["newCount"], 0);
    let overview = run(app, server, &["admin", "problem", "list"])
        .await
        .unwrap();
    assert_eq!(new_tests_count(&overview), 0);
    fixture_command(
        "mongodb",
        "mongosh",
        "db.getSiblingDB('wekan').eventlog.deleteMany({source:p.marker});print('{}');",
        &json!({"marker":marker}),
    );
}
