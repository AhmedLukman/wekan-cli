use super::support::{TestDirectory, no_target_cli};
use clap::Parser;
use wekan_cli::cli::Cli;

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    let mut argv = vec!["wekan", "attachment"];
    argv.extend(args);
    Cli::try_parse_from(argv)
}

#[test]
fn selectors_follow_endpoint_hierarchy_and_transfer_targets_are_required() {
    for flags in [
        vec![],
        vec!["--swimlane-id", "s"],
        vec!["--swimlane-id", "s", "--list", "l"],
        vec!["--swimlane-id", "s", "--list", "l", "--card", "c"],
    ] {
        let mut args = vec!["list", "--board", "b"];
        args.extend(flags);
        parse(&args).unwrap();
    }
    for args in [
        vec!["list"],
        vec!["list", "--board", "b", "--list", "l"],
        vec!["list", "--board", "b", "--card", "c"],
        vec!["get", ".."],
        vec!["copy", "a"],
        vec!["move", "a"],
        vec!["upload", "file", "--board", "b"],
        vec!["download", "a"],
    ] {
        assert!(parse(&args).is_err());
    }
    for operation in ["copy", "move"] {
        parse(&[
            operation,
            "a",
            "--target-board",
            "b",
            "--target-swimlane-id",
            "s",
            "--target-list",
            "l",
            "--target-card",
            "c",
        ])
        .unwrap();
    }
}

#[test]
fn invalid_stdin_and_output_modes_fail_before_profile_or_credential_access() {
    let directory = TestDirectory::new("attachment-input");
    for args in [
        vec![
            "--output",
            "json",
            "attachment",
            "upload-background",
            "-",
            "--board",
            "b",
        ],
        vec![
            "--output",
            "json",
            "attachment",
            "download",
            "a",
            "--destination",
            "-",
        ],
        vec![
            "--output",
            "raw",
            "attachment",
            "download",
            "a",
            "--destination",
            "file",
        ],
        vec![
            "--output",
            "raw",
            "attachment",
            "download-background",
            "--board",
            "b",
            "--destination",
            "-",
            "--overwrite",
        ],
    ] {
        let output = no_target_cli(&directory).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("invalid_input"));
    }
    for operation in ["download", "download-background"] {
        let mut args = vec![operation];
        if operation == "download" {
            args.push("a");
        } else {
            args.extend(["--board", "b"]);
        }
        args.extend(["--destination", "-", "--output", "raw"]);
        let cli = parse(&args).unwrap();
        cli.command.validate_output(cli.output).unwrap();
    }
    parse(&[
        "upload-background",
        "-",
        "--board",
        "b",
        "--file-name",
        "binary.png",
    ])
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn executable_transfers_binary_stdin_and_stdout_without_text_conversion() {
    use serde_json::{Value, json};
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_json, header, method, path},
    };
    struct Session(TestDirectory);
    impl Drop for Session {
        fn drop(&mut self) {
            // The disposable mock credential is scoped to this temporary profile store.
            let _ = no_target_cli(&self.0)
                .args(["auth", "logout", "--local-only", "--yes"])
                .output();
        }
    }
    let session = Session(TestDirectory::new("attachment-binary-stdio"));
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/users/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"user","token":"attachment-cli-token","tokenExpires":"2099-01-01T00:00:00Z"}))).expect(1).mount(&server).await;
    no_target_cli(&session.0)
        .args([
            "--server",
            &server.uri(),
            "--output",
            "json",
            "auth",
            "login",
            "--username",
            "fixture",
            "--password-stdin",
        ])
        .write_stdin("fixture-password\n")
        .assert()
        .success();
    Mock::given(method("POST")).and(path("/api/attachment/upload-background"))
        .and(header("Authorization","Bearer attachment-cli-token"))
        .and(body_json(json!({"boardId":"b","fileName":"binary.png","fileData":"AP8K"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success":true,"attachmentId":"a","fileName":"binary.png","fileSize":3,"storageBackend":"fs","message":"Uploaded","backgroundImageURL":"/background"}))).expect(1).mount(&server).await;
    let uploaded = no_target_cli(&session.0)
        .args([
            "--output",
            "json",
            "attachment",
            "upload-background",
            "-",
            "--file-name",
            "binary.png",
            "--board",
            "b",
        ])
        .write_stdin(vec![0, 255, 10])
        .output()
        .unwrap();
    assert!(
        uploaded.status.success(),
        "{}",
        String::from_utf8_lossy(&uploaded.stderr)
    );
    assert!(uploaded.stderr.is_empty());
    let result: Value = serde_json::from_slice(&uploaded.stdout).unwrap();
    assert!(result["data"].get("mutation_confirmed").is_none());
    Mock::given(method("GET")).and(path("/api/attachment/download/a"))
        .and(header("Authorization","Bearer attachment-cli-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success":true,"attachmentId":"a","fileName":"binary.png","fileSize":3,"fileType":"image/png","storageBackend":"fs","base64Data":"AP8K"}))).expect(1).mount(&server).await;
    let downloaded = no_target_cli(&session.0)
        .args([
            "--output",
            "raw",
            "attachment",
            "download",
            "a",
            "--destination",
            "-",
        ])
        .output()
        .unwrap();
    assert!(
        downloaded.status.success(),
        "{}",
        String::from_utf8_lossy(&downloaded.stderr)
    );
    assert_eq!(downloaded.stdout, [0, 255, 10]);
    assert!(downloaded.stderr.is_empty());
}
