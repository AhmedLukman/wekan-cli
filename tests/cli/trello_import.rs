use super::support::{TestDirectory, add_profile, no_target_cli};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_bytes, body_json, method, path},
};

#[test]
fn invalid_arguments_and_inputs_fail_before_credentials_or_http() {
    let directory = TestDirectory::new("trello-invalid");
    add_profile(&directory, "default", "http://localhost:9");
    for args in [
        vec![],
        vec!["file"],
        vec!["-", "--format", "csv"],
        vec!["-", "--format", "json", "--auth", "query"],
        vec!["-", "--format", "zip", "--members-mapping", "{}"],
        vec!["-", "--format", "zip", "--members-mapping-file", "map"],
        vec!["-", "--format", "json", "--members-mapping-file", "-"],
        vec![
            "-",
            "--format",
            "json",
            "--members-mapping",
            "{}",
            "--members-mapping-file",
            "map",
        ],
        vec!["-", "--format", "json", "--timeout", "0"],
        vec!["-", "--format", "json", "--timeout", "1.5"],
        vec!["-", "--format", "json", "--yes"],
        vec!["-", "--format", "json", "--output", "raw"],
    ] {
        no_target_cli(&directory)
            .args(["--output", "json", "board", "import-trello"])
            .args(args)
            .assert()
            .code(2);
    }
    for input in [b"[]".as_slice(), b"null", b"42", b"{bad", &[255]] {
        no_target_cli(&directory)
            .args([
                "--output",
                "json",
                "board",
                "import-trello",
                "-",
                "--format",
                "json",
            ])
            .write_stdin(input)
            .assert()
            .code(2);
    }
    for mapping in ["[]", "null", "{\"member\":42}", "{bad"] {
        no_target_cli(&directory)
            .args([
                "--output",
                "json",
                "board",
                "import-trello",
                "-",
                "--format",
                "json",
                "--members-mapping",
                mapping,
            ])
            .write_stdin("{}")
            .assert()
            .code(2);
    }
    for (format, limit) in [("json", 50 * 1024 * 1024), ("zip", 200 * 1024 * 1024)] {
        let file = directory.0.join(format);
        std::fs::File::create(&file)
            .unwrap()
            .set_len(limit + 1)
            .unwrap();
        no_target_cli(&directory)
            .args(["--output", "json", "board", "import-trello"])
            .arg(file)
            .args(["--format", format])
            .assert()
            .code(2);
    }
    no_target_cli(&directory)
        .args(["--output", "json", "board", "import-trello"])
        .arg(&directory.0)
        .args(["--format", "zip"])
        .assert()
        .code(2);
    no_target_cli(&directory)
        .args([
            "--output",
            "json",
            "board",
            "import-trello",
            "missing-file",
            "--format",
            "json",
        ])
        .assert()
        .code(4);
    let file = directory.0.join("wrapped-limit.json");
    let content = format!("{{\"x\":\"{}\"}}", "a".repeat(50 * 1024 * 1024 - 9));
    std::fs::write(&file, content).unwrap();
    no_target_cli(&directory)
        .args(["--output", "json", "board", "import-trello"])
        .arg(file)
        .args(["--format", "json"])
        .assert()
        .code(2);
    let help = no_target_cli(&directory)
        .args(["board", "import-trello", "--help"])
        .output()
        .unwrap();
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    for option in [
        "--format",
        "--auth",
        "--timeout",
        "--members-mapping",
        "--members-mapping-file",
        "--server",
        "--profile",
        "--output",
        "--allow-insecure-http",
    ] {
        assert!(help.contains(option));
    }
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
async fn executable_imports_preserve_input_and_render_complete_and_partial_outcomes() {
    let session = Session(TestDirectory::new("trello-stdio"));
    let server = MockServer::start().await;
    let token = "trello-cli-secret";
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"id":"user","token":token,"tokenExpires":"2099-01-01T00:00:00Z"}),
        ))
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
    server.reset().await;
    for format in ["json", "zip"] {
        let input = if format == "json" {
            b"{\"name\":\"Board\",\"extra\":{\"keep\":true}}".to_vec()
        } else {
            vec![b'P', b'K', 0, 255, 13, 10]
        };
        let file = session.0.0.join("input");
        std::fs::write(&file, &input).unwrap();
        let mapping = session.0.0.join("mapping");
        std::fs::write(&mapping, "{\"upstream\":\"local\"}").unwrap();
        for stdin in [false, true] {
            let response = if format == "json" {
                json!({"boardIds":["created"]})
            } else {
                json!({"boardIds":["created"],"errors":[]})
            };
            let mock = Mock::given(method("POST")).and(path("/import-trello"));
            let mock = if format == "json" {
                mock.and(body_json(json!({"board":{"name":"Board","extra":{"keep":true}},"membersMapping":{"upstream":"local"}})))
            } else {
                mock.and(body_bytes(input.clone()))
            };
            mock.respond_with(ResponseTemplate::new(200).set_body_json(response))
                .expect(1)
                .mount(&server)
                .await;
            let mut command = no_target_cli(&session.0);
            command.args(["--output", "json", "board", "import-trello"]);
            if stdin {
                command.arg("-").write_stdin(input.clone());
            } else {
                command.arg(&file);
            }
            command.args(["--format", format]);
            if format == "json" {
                if stdin {
                    command.args(["--members-mapping", "{\"upstream\":\"local\"}"]);
                } else {
                    command.arg("--members-mapping-file").arg(&mapping);
                }
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.stderr.is_empty());
            let value: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(value["data"]["format"], format);
            assert_eq!(value["data"]["board_ids"], json!(["created"]));
            assert!(value["data"].get("mutation_confirmed").is_none());
            assert_eq!(value["data"].get("errors").is_some(), format == "zip");
            server.verify().await;
            server.reset().await;
        }
    }
    for ids in [json!(["created"]), json!([])] {
        for format in ["json", "human"] {
            Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"boardIds":ids,"errors":[{"board":"Bad\u{001b}[2J","error":format!("failed {token}\n")}]}))).expect(1).mount(&server).await;
            let output = no_target_cli(&session.0)
                .args([
                    "--output",
                    format,
                    "board",
                    "import-trello",
                    "-",
                    "--format",
                    "zip",
                ])
                .write_stdin("ZIP")
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(5));
            assert!(output.stdout.is_empty());
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert!(!stderr.contains(token));
            assert!(!stderr.contains('\u{001b}'));
            if format == "json" {
                let value: Value = serde_json::from_str(&stderr).unwrap();
                assert_eq!(value["error"]["code"], "batch_incomplete");
                let details = &value["error"]["details"];
                assert_eq!(details["trello_import_result"]["board_ids"], ids);
                assert_eq!(
                    details["trello_import_result"]["errors"][0]["error"],
                    "failed [REDACTED]\n"
                );
                assert!(details.get("outcome_unknown").is_none());
                assert_eq!(details["retry_safe"], false);
            } else {
                assert!(stderr.contains("Failed boards: 1"));
                assert!(stderr.contains("Board IDs:"));
            }
            server.verify().await;
            server.reset().await;
        }
    }
    for (status, body, code) in [
        (
            400,
            json!({"error":format!("failed {token}")}),
            "server_error",
        ),
        (200, json!({"boardIds":[],"extra":true}), "protocol_error"),
    ] {
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let output = no_target_cli(&session.0)
            .args([
                "--output",
                "json",
                "board",
                "import-trello",
                "-",
                "--format",
                "json",
            ])
            .write_stdin("{}")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(if status == 400 { 5 } else { 4 })
        );
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains(token));
        let value: Value = serde_json::from_str(&stderr).unwrap();
        assert_eq!(value["error"]["code"], code);
        assert!(value["error"]["details"].get("outcome_unknown").is_none());
        assert_eq!(value["error"]["details"]["retry_safe"], false);
        server.verify().await;
        server.reset().await;
    }
}
