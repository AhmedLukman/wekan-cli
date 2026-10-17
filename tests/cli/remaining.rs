use super::support::{TestDirectory, add_profile, no_target_cli};
use predicates::prelude::*;
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

#[test]
fn remaining_commands_expose_help_and_reject_invalid_input_before_network_access() {
    for args in [
        vec!["board", "attachment", "list"],
        vec!["system", "schema-upgrade-status"],
        vec!["auth", "preflight"],
    ] {
        let directory = TestDirectory::new("remaining-help");
        let mut command = no_target_cli(&directory);
        command
            .args(&args)
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("--server"))
            .stdout(predicate::str::contains("--profile"))
            .stdout(predicate::str::contains("--output"))
            .stdout(predicate::str::contains("--allow-insecure-http"));
    }
    for args in [
        vec!["auth", "preflight"],
        vec!["auth", "preflight", "invalid"],
        vec!["auth", "preflight", "/users/login"],
        vec!["auth", "preflight", "login", "--yes"],
        vec!["board", "attachment", "list"],
        vec!["board", "attachment", "list", "--board", ".."],
        vec!["system", "schema-upgrade-status", "--output", "raw"],
        vec!["auth", "preflight", "logout", "--output", "raw"],
    ] {
        let directory = TestDirectory::new("remaining-input");
        no_target_cli(&directory)
            .args(args)
            .assert()
            .failure()
            .code(2)
            .stdout(predicate::str::is_empty());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn anonymous_commands_resolve_environment_profiles_and_explicit_overrides_without_writes() {
    let server = MockServer::start().await;
    Mock::given(method("OPTIONS"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let directory = TestDirectory::new("remaining-targets");
    // A missing named profile can use the environment server, without login or profile creation.
    no_target_cli(&directory)
        .env("WEKAN_URL", server.uri())
        .env("WEKAN_PROFILE", "probe")
        .args(["auth", "preflight", "login", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"http_status\":200"));
    assert!(!directory.0.join("profiles.json").exists());
    no_target_cli(&directory)
        .args(["auth", "preflight", "login", "--output", "json"])
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("profile_not_found"));
    add_profile(&directory, "saved", &server.uri());
    let bytes = std::fs::read(directory.0.join("profiles.json")).unwrap();
    no_target_cli(&directory)
        .args(["--profile", "saved", "auth", "preflight", "login"])
        .assert()
        .success()
        .stdout(predicate::str::contains("http_status: 200"));
    no_target_cli(&directory)
        .env("WEKAN_URL", "invalid ignored URL")
        .env("WEKAN_PROFILE", "invalid ignored profile")
        .args([
            "--server",
            &server.uri(),
            "--profile",
            "saved",
            "auth",
            "preflight",
            "login",
        ])
        .assert()
        .success();
    let output = no_target_cli(&directory)
        .args([
            "--profile",
            "saved",
            "--server",
            "https://different.example",
            "auth",
            "preflight",
            "login",
            "--output",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "profile_server_mismatch");
    assert_eq!(
        std::fs::read(directory.0.join("profiles.json")).unwrap(),
        bytes
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn system_cli_reports_typed_failure_states_and_protocol_errors() {
    let server = MockServer::start().await;
    let mut state: Value = serde_json::from_str(include_str!(
        "../fixtures/responses/schema_upgrade_status.json"
    ))
    .unwrap();
    state["product"] = json!("WeKan\u{001b}[2J");
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(state))
        .mount(&server)
        .await;
    let directory = TestDirectory::new("system-output");
    for format in ["human", "json"] {
        let output = no_target_cli(&directory)
            .args([
                "--server",
                &server.uri(),
                "system",
                "schema-upgrade-status",
                "--output",
                format,
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(!output.stdout.contains(&0x1b));
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("fixture failure"));
        if format == "json" {
            let decoded: Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(decoded["ok"], true);
            assert_eq!(decoded["data"]["app_version"], "v11.06.0");
        }
    }
    server.reset().await;
    Mock::given(method("OPTIONS"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"unexpected":true})))
        .mount(&server)
        .await;
    no_target_cli(&directory)
        .args([
            "--server",
            &server.uri(),
            "auth",
            "preflight",
            "logout",
            "--output",
            "json",
        ])
        .assert()
        .failure()
        .code(4)
        .stderr(predicate::str::contains("protocol_error"));
    assert!(!directory.0.join("profiles.json").exists());
}
