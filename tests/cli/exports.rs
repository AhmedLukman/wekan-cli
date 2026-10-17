use super::support::{TestDirectory, add_profile, no_target_cli};
use clap::Parser;
use serde_json::{Value, json};
use wekan_cli::{cli::Cli, output::OutputFormat};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
#[path = "../support/export_fixtures.rs"]
mod fixtures;

#[test]
fn export_help_and_required_arguments_match_resource_commands() {
    for resource in ["board", "card", "attachment"] {
        let output = crate::support::wekan()
            .args([resource, "export", "--help"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        for flag in [
            "--destination",
            "--overwrite",
            "--auth-token-query",
            "--no-auth",
            "--timeout",
            "--output",
            "--profile",
            "--server",
            "--allow-insecure-http",
        ] {
            assert!(help.contains(flag), "{resource}: {flag}");
        }
    }
    for args in [
        vec!["board", "export", "b"],
        vec![
            "card",
            "export",
            "c",
            "--format",
            "pdf",
            "--destination",
            "file",
        ],
        vec!["attachment", "export", "a", "--destination", "file"],
    ] {
        assert!(Cli::try_parse_from(["wekan"].into_iter().chain(args)).is_err());
    }
}

#[test]
fn invalid_options_are_rejected_before_target_or_credentials() {
    let directory = TestDirectory::new("invalid-export");
    for extra in [
        vec!["--format", "kanboard", "--field", "people"],
        vec!["--format", "csv", "--card", "c"],
        vec!["--format", "csv", "--attachments", "false"],
        vec!["--format", "tsv", "--delimiter", ";"],
        vec!["--format", "json", "--language", "fi"],
        vec!["--format", "json", "--field", "unknown"],
        vec!["--format", "json", "--list", "bad/scope"],
        vec!["--format", "json", "--auth-token-query", "--no-auth"],
        vec!["--format", "json", "--timeout", "0"],
    ] {
        let mut args = vec![
            "--output",
            "json",
            "board",
            "export",
            "b",
            "--destination",
            "file",
        ];
        args.extend(extra);
        let output = no_target_cli(&directory).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], "invalid_input");
    }
    for (format, destination, overwrite) in [
        ("json", "-", false),
        ("raw", "file", false),
        ("raw", "-", true),
    ] {
        let mut args = vec![
            "wekan",
            "--output",
            format,
            "board",
            "export",
            "b",
            "--format",
            "json",
            "--destination",
            destination,
        ];
        if overwrite {
            args.push("--overwrite");
        }
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(
            cli.command
                .validate_output(if format == "raw" {
                    OutputFormat::Raw
                } else {
                    OutputFormat::Json
                })
                .is_err()
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn files_and_stdout_are_delivered_only_after_validation() {
    let server = MockServer::start().await;
    let directory = TestDirectory::new("export-delivery");
    add_profile(&directory, "default", &server.uri());
    let destination = directory.0.join("board.json");
    let path = destination.to_str().unwrap();
    let body = serde_json::to_string_pretty(&fixtures::external("json")).unwrap();
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body.clone(), "application/json"))
        .mount(&server)
        .await;
    let args = [
        "--profile",
        "default",
        "--output",
        "json",
        "board",
        "export",
        "board",
        "--format",
        "json",
        "--destination",
        path,
        "--no-auth",
    ];
    let output = no_target_cli(&directory).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(metadata["data"]["resource"], "board");
    assert_eq!(metadata["data"]["file_saved"], true);
    assert_eq!(metadata["data"]["http_status"], 200);
    assert_eq!(metadata["data"]["bytes_written"], body.len());
    assert!(metadata["data"].get("staged").is_none());
    assert_eq!(std::fs::read(&destination).unwrap(), body.as_bytes());
    assert_eq!(
        no_target_cli(&directory)
            .args(args)
            .output()
            .unwrap()
            .status
            .code(),
        Some(4)
    );
    let output = no_target_cli(&directory)
        .args(args)
        .arg("--overwrite")
        .output()
        .unwrap();
    assert!(output.status.success());
    let raw = [
        "--profile",
        "default",
        "--output",
        "raw",
        "board",
        "export",
        "board",
        "--format",
        "json",
        "--destination",
        "-",
        "--no-auth",
    ];
    let output = no_target_cli(&directory).args(raw).output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, body.as_bytes());
    assert!(output.stderr.is_empty());
    server.reset().await;
    let mut invalid = fixtures::external("json");
    invalid["unmapped"] = json!("secret-response-value");
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(invalid))
        .mount(&server)
        .await;
    let before = std::fs::read_dir(&directory.0).unwrap().count();
    let output = no_target_cli(&directory)
        .args(args)
        .arg("--overwrite")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    assert_eq!(std::fs::read(&destination).unwrap(), body.as_bytes());
    assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), before);
    assert!(!String::from_utf8_lossy(&output.stderr).contains("secret-response-value"));
    let output = no_target_cli(&directory).args(raw).output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|r| !r.headers.contains_key("authorization"))
    );
}
