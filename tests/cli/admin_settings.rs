use super::support::{TestDirectory, add_profile, no_target_cli};
use clap::Parser;
use wekan_cli::{cli::Cli, commands::RootCommand, output::OutputFormat};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    let mut argv = vec!["wekan", "admin"];
    argv.extend_from_slice(args);
    Cli::try_parse_from(argv)
}
#[test]
fn settings_parser_enforces_sources_repetition_and_stdin_ownership() {
    for family in ["settings", "attachment-settings"] {
        for action in ["get", "update"] {
            let mut args = vec![family, action];
            if action == "update" {
                args.push("--settings-stdin");
            }
            let cli = parse(&args).unwrap();
            assert!(matches!(cli.command, RootCommand::Admin(_)));
            assert!(cli.command.validate_output(OutputFormat::Raw).is_err());
        }
        for args in [
            vec![family, "update"],
            vec![
                family,
                "update",
                "--settings-stdin",
                "--settings-file",
                "settings.json",
            ],
            vec![family, "get", "--settings-stdin"],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
        let flag = if family == "settings" {
            "--disable-registration"
        } else {
            "--api-upload-blocked"
        };
        for args in [
            vec![family, "update", flag, "false", flag, "true"],
            vec![family, "update", flag],
            vec![family, "update", flag, "yes"],
            vec![family, "update", flag, "false", "--settings-stdin"],
            vec![
                family,
                "update",
                flag,
                "false",
                "--settings-file",
                "settings.json",
            ],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
    }
    for args in [
        vec![
            "--s3-secret-access-key-file",
            "key",
            "--s3-secret-access-key-stdin",
        ],
        vec!["--s3-secret-access-key-stdin", "--azure-account-key-stdin"],
        vec!["--gcs-credentials-stdin", "--settings-stdin"],
        vec![
            "--upload-allowed-mime-type",
            "image/png",
            "--clear-upload-allowed-mime-types",
        ],
        vec!["--migration-batch-size", "101"],
        vec!["--migration-delay-ms", "99"],
        vec!["--migration-cpu-threshold", "9"],
        vec!["--api-upload-max-bytes", "-1"],
        vec!["--s3-secret-access-key", "literal-secret"],
    ] {
        let mut argv = vec!["attachment-settings", "update"];
        argv.extend(args);
        assert!(parse(&argv).is_err(), "{argv:?}");
    }
    parse(&[
        "attachment-settings",
        "update",
        "--s3-secret-access-key-file",
        "key",
        "--azure-account-key-file",
        "key2",
        "--gcs-credentials-stdin",
        "--api-upload-blocked",
        "false",
    ])
    .unwrap();
    parse(&[
        "attachment-settings",
        "update",
        "--upload-allowed-mime-type",
        "image/png",
        "--upload-allowed-mime-type",
        "application/pdf",
        "--migration-batch-size",
        "1.5",
    ])
    .unwrap();
    let cli = parse(&[
        "settings",
        "get",
        "--server",
        "http://localhost:3118",
        "--profile",
        "local",
        "--output",
        "json",
        "--allow-insecure-http",
    ])
    .unwrap();
    assert_eq!(cli.output, OutputFormat::Json);
    assert!(cli.allow_insecure_http);
}

#[test]
fn help_requires_no_target_and_lists_settings_and_secret_inputs() {
    let dir = TestDirectory::new("admin-help");
    for family in ["settings", "attachment-settings"] {
        for action in ["get", "update"] {
            no_target_cli(&dir)
                .args(["admin", family, action, "--help"])
                .assert()
                .success();
        }
    }
    no_target_cli(&dir)
        .args(["admin", "attachment-settings", "update", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("--gcs-credentials-stdin"))
        .stdout(predicates::str::contains("replacement/default"));
}

#[test]
fn stdin_inputs_are_validated_without_echoing_values_before_authentication() {
    let dir = TestDirectory::new("admin-stdin");
    add_profile(&dir, "default", "http://localhost:1");
    for family in ["settings", "attachment-settings"] {
        for body in [
            "{}",
            "[]",
            "null",
            "{secret-payload",
            r#"{"unexpected":"secret-payload"}"#,
        ] {
            let result = no_target_cli(&dir)
                .args([
                    "--output",
                    "json",
                    "admin",
                    family,
                    "update",
                    "--settings-stdin",
                ])
                .write_stdin(body)
                .output()
                .unwrap();
            assert_eq!(result.status.code(), Some(2));
            assert!(result.stdout.is_empty());
            let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
            assert_eq!(error["error"]["code"], "invalid_input");
            assert!(!String::from_utf8_lossy(&result.stderr).contains("secret-payload"));
        }
    }
    for (family, body) in [
        ("settings", r#"{"disableRegistration":false}"#),
        (
            "attachment-settings",
            r#"{"limitSettings":{"apiUploadBlocked":false}}"#,
        ),
    ] {
        let result = no_target_cli(&dir)
            .args([
                "--output",
                "json",
                "admin",
                family,
                "update",
                "--settings-stdin",
            ])
            .write_stdin(body)
            .output()
            .unwrap();
        let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_ne!(error["error"]["code"], "invalid_input");
    }
    let result = no_target_cli(&dir)
        .args([
            "--output",
            "json",
            "admin",
            "attachment-settings",
            "update",
            "--s3-secret-access-key-stdin",
        ])
        .write_stdin([0xff])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
}
