use super::support::{TestDirectory, add_profile, no_target_cli};
use clap::Parser;
use serde_json::{Value, json};
use wekan_cli::{cli::Cli, output::OutputFormat};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_bytes, body_json, header, method, path},
};

#[test]
fn import_parser_help_sources_conflicts_and_output_are_explicit() {
    let directory = TestDirectory::new("import-parser");
    for args in [
        vec!["board", "import", "-"],
        vec!["board", "import-zip", "-", "--board", "b"],
        vec![
            "card",
            "import-ics",
            "-",
            "--board",
            "b",
            "--list",
            "l",
            "--swimlane-id",
            "s",
        ],
    ] {
        let cli = Cli::try_parse_from(std::iter::once("wekan").chain(args.clone())).unwrap();
        assert!(cli.command.validate_output(OutputFormat::Raw).is_err());
        assert!(
            Cli::try_parse_from(
                std::iter::once("wekan")
                    .chain(args.clone())
                    .chain(["--yes"])
            )
            .is_err()
        );
        let output = no_target_cli(&directory)
            .args(args)
            .arg("--help")
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let help = String::from_utf8(output.stdout).unwrap();
        for option in [
            "--timeout",
            "--server",
            "--profile",
            "--output",
            "--allow-insecure-http",
        ] {
            assert!(help.contains(option));
        }
    }
    for args in [
        vec!["board", "import"],
        vec!["board", "import-zip", "file"],
        vec!["card", "import-ics", "file", "--board", "b", "--list", "l"],
        vec!["board", "import", "file", "--source", "unknown"],
        vec!["board", "import", "file", "--timeout", "0"],
        vec!["board", "import", "file", "--timeout", "1.5"],
        vec![
            "board",
            "import",
            "file",
            "--members-mapping",
            "{}",
            "--members-mapping-file",
            "map",
        ],
        vec![
            "board",
            "import-zip",
            "file",
            "--board",
            "b",
            "--card",
            "bad/scope",
        ],
        vec![
            "board",
            "import",
            "file",
            "--source",
            "csv",
            "--delimiter",
            "é",
        ],
    ] {
        let out = no_target_cli(&directory)
            .args(["--output", "json"])
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty());
        let error: Value = serde_json::from_slice(&out.stderr).unwrap();
        assert_eq!(error["error"]["code"], "invalid_input");
    }
    for extra in [
        vec!["--delimiter", ";"],
        vec!["--members-mapping-file", "-"],
    ] {
        no_target_cli(&directory)
            .args(["--output", "json", "board", "import", "missing"])
            .args(extra)
            .assert()
            .code(2);
    }
}

#[test]
fn import_input_errors_do_not_attempt_http() {
    let directory = TestDirectory::new("import-input-errors");
    add_profile(&directory, "default", "http://localhost:9");
    for (input, extra) in [
        (b"null".as_slice(), vec![]),
        (b"{bad".as_slice(), vec![]),
        (b"{}".as_slice(), vec!["--members-mapping", "{\"a\":1}"]),
        (b"\xff".as_slice(), vec!["--source", "csv"]),
    ] {
        let output = no_target_cli(&directory)
            .args(["--output", "json", "board", "import", "-"])
            .args(extra)
            .write_stdin(input)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], "invalid_input");
    }
    let output = no_target_cli(&directory)
        .args([
            "--output",
            "json",
            "card",
            "import-ics",
            "-",
            "--board",
            "b",
            "--list",
            "l",
            "--swimlane-id",
            "s",
        ])
        .write_stdin(vec![255])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    no_target_cli(&directory)
        .args(["--output", "json", "board", "import"])
        .arg(&directory.0)
        .assert()
        .code(2);
    no_target_cli(&directory)
        .args(["--output", "json", "board", "import", "missing-import-file"])
        .assert()
        .code(4);
}

struct Session(TestDirectory);
impl Drop for Session {
    fn drop(&mut self) {
        let _ = no_target_cli(&self.0)
            .args(["auth", "logout", "--local-only", "--yes"])
            .output();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn executable_imports_encode_files_stdin_mapping_and_native_results() {
    let session = Session(TestDirectory::new("import-stdio"));
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"id":"user","token":"import-cli-token","tokenExpires":"2099-01-01T00:00:00Z"}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    no_target_cli(&session.0)
        .args([
            "--server",
            &server.uri(),
            "auth",
            "login",
            "--username",
            "fixture",
            "--password-stdin",
        ])
        .write_stdin("password\n")
        .assert()
        .success();
    let mapping = session.0.0.join("mapping.json");
    std::fs::write(&mapping, b"{\"upstream\":\"local\"}").unwrap();
    for source in [
        "native",
        "wekan",
        "trello",
        "csv",
        "jira",
        "kanboard",
        "excel",
        "deck",
        "openproject",
        "github",
        "gitlab",
        "gitea",
        "forgejo",
        "asana",
        "zenkit",
    ] {
        let (input,document)=match source {
            "csv" => (b"title;description\r\n\"Ship;now\";\"Line 1\nLine 2\"\r\nRagged\r\n".to_vec(),json!([["title","description"],["Ship;now","Line 1\nLine 2"],["Ragged"]])),
            "excel" => (vec![0,255,10],json!({"excelBase64":"AP8K"})),
            _ => (b"{\"board\":{\"name\":\"Keep\"},\"extra\":{\"number\":123456789012345678901234567890}}".to_vec(),serde_json::from_str::<Value>("{\"board\":{\"name\":\"Keep\"},\"extra\":{\"number\":123456789012345678901234567890}}").unwrap()),
        };
        let route = if source == "native" {
            "/api/boards/import".into()
        } else {
            format!("/api/boards/import/{source}")
        };
        Mock::given(method("POST"))
            .and(path(route))
            .and(header("Authorization", "Bearer import-cli-token"))
            .and(body_json(
                json!({"board":document,"membersMapping":{"upstream":"local"}}),
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"_id":format!("new-{source}")})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let input_path = session.0.0.join("source.unknown");
        std::fs::write(&input_path, &input).unwrap();
        let mut command = no_target_cli(&session.0);
        command.args(["--output", "json", "board", "import"]);
        if source == "native" {
            command
                .arg(&input_path)
                .arg("--members-mapping-file")
                .arg(&mapping);
        } else {
            command
                .arg("-")
                .args([
                    "--source",
                    source,
                    "--members-mapping",
                    "{\"upstream\":\"local\"}",
                ])
                .write_stdin(input);
        }
        if source == "csv" {
            command.args(["--delimiter", ";"]);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{source}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            value,
            json!({"ok":true,"data":{"board_id":format!("new-{source}"),"source":source}})
        );
    }
    let counts =
        json!({"swimlanes":1,"lists":1,"cards":1,"checklists":1,"comments":1,"attachments":0});
    Mock::given(method("POST"))
        .and(path("/api/import/zip"))
        .and(body_bytes(vec![0, 255, 10]))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok":true,"counts":counts})))
        .expect(2)
        .mount(&server)
        .await;
    for stdin in [false, true] {
        let file = session.0.0.join("archive");
        std::fs::write(&file, [0, 255, 10]).unwrap();
        let mut command = no_target_cli(&session.0);
        command.args(["--output", "json", "board", "import-zip"]);
        if stdin {
            command.arg("-").write_stdin(vec![0, 255, 10]);
        } else {
            command.arg(&file);
        }
        let output = command
            .args([
                "--board",
                "b",
                "--checklist",
                "ch",
                "--field",
                "comments",
                "--field",
                "people",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["data"]["counts"], counts);
        assert_eq!(value["data"]["checklist_id"], "ch");
    }
    let calendar = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
    Mock::given(method("POST"))
        .and(path("/api/boards/b/swimlanes/s/lists/l/ics"))
        .and(body_json(json!({"ics":calendar})))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"created":1,"cardIds":["event"]})),
        )
        .expect(2)
        .mount(&server)
        .await;
    for format in ["json", "human"] {
        let output = no_target_cli(&session.0)
            .args([
                "--output",
                format,
                "card",
                "import-ics",
                "-",
                "--board",
                "b",
                "--list",
                "l",
                "--swimlane-id",
                "s",
            ])
            .write_stdin(calendar)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        if format == "json" {
            let value: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(value["data"]["card_ids"], json!(["event"]));
        } else {
            assert!(String::from_utf8_lossy(&output.stdout).starts_with("Imported "));
        }
    }
    server.reset().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    let output = no_target_cli(&session.0)
        .args(["--output", "json", "board", "import", "-"])
        .write_stdin("{}")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "protocol_error");
    assert!(error["error"]["details"].get("outcome_unknown").is_none());

    for (server_error, code) in [
        ("not-authorized", "permission_denied"),
        ("Unauthorized", "authentication_rejected"),
    ] {
        server.reset().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({"error":server_error})))
            .expect(1)
            .mount(&server)
            .await;
        let output = no_target_cli(&session.0)
            .args([
                "--output",
                "json",
                "card",
                "import-ics",
                "-",
                "--board",
                "b",
                "--list",
                "l",
                "--swimlane-id",
                "s",
            ])
            .write_stdin(calendar)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(5));
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], code);
        assert_eq!(error["error"]["details"]["http_status"], 401);
        assert_eq!(error["error"]["details"]["server_error"], server_error);
        assert!(error["error"]["details"].get("outcome_unknown").is_none());
        if code == "permission_denied" {
            assert!(
                !error["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("credential")
            );
        }
    }
}
