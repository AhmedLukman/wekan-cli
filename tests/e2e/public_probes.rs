use super::support::*;

#[tokio::test]
#[ignore = "requires an isolated Wekan v11.06 stack and WEKAN_PUBLIC_PROBE_E2E_URL"]
async fn public_probes_match_wekan_v11_06() {
    let server = env::var("WEKAN_PUBLIC_PROBE_E2E_URL").expect("set WEKAN_PUBLIC_PROBE_E2E_URL");
    let directory = TestDirectory::new("public-probes");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("unconfigured".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from("unused")),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let http = reqwest::Client::new();
    for endpoint in ["login", "register", "logout"] {
        let response = http
            .request(
                reqwest::Method::OPTIONS,
                format!("{}/users/{endpoint}", server.trim_end_matches('/')),
            )
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert!(response.headers().get("content-type").is_none());
        assert!(response.bytes().await.unwrap().is_empty());
        let result = app
            .execute(
                Cli::try_parse_from(["wekan", "auth", "preflight", endpoint])
                    .unwrap()
                    .command,
            )
            .await
            .unwrap();
        let CommandSuccess::AuthPreflight(data) = &result else {
            panic!("preflight")
        };
        assert_eq!(data.http_status, 200);
        for format in [OutputFormat::Human, OutputFormat::Json] {
            let text = wekan_cli::output::render_success(format, &result);
            assert!(text.contains("200"));
        }
    }
    // Wekan serves requests before its background upgrade initializes the state.
    // Keep decoding each response strictly while waiting for that initialization.
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            let result = app
                .execute(
                    Cli::try_parse_from(["wekan", "system", "schema-upgrade-status"])
                        .unwrap()
                        .command,
                )
                .await
                .unwrap();
            let CommandSuccess::SchemaUpgradeStatus(state) = &result else {
                panic!("schema status")
            };
            if !state.app_version.is_empty() {
                break result;
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    })
    .await
    .expect("schema-upgrade status did not initialize within 30 seconds");
    let CommandSuccess::SchemaUpgradeStatus(state) = &result else {
        panic!("schema status")
    };
    assert_eq!(state.app_version, "v11.06.0");
    assert!(!state.steps.is_empty());
    let output = wekan_cli::output::render_success(OutputFormat::Json, &result);
    let output: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(output["ok"], true);
    assert!(output["data"]["steps"].is_object());
    let html = http
        .get(format!(
            "{}/schema-upgrade-status",
            server.trim_end_matches('/')
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(html.status(), reqwest::StatusCode::OK);
    assert!(
        html.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert!(html.text().await.unwrap().contains("Schema Upgrade"));
    assert!(app.credential_store().credential.lock().unwrap().is_none());
    assert!(!directory.path().join("profiles.json").exists());
}
