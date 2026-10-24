use clap::Parser;
use predicates::prelude::*;

use crate::support::wekan;
use wekan_cli::{
    cli::Cli,
    commands::{RootCommand, self_management::SelfCommand},
};

#[test]
fn self_update_help_and_options_are_available_without_configuration() {
    wekan()
        .args(["self", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--check"))
        .stdout(predicate::str::contains("--yes"))
        .stderr(predicate::str::is_empty());
    let cli =
        Cli::try_parse_from(["wekan", "self", "update", "--check", "--output", "json"]).unwrap();
    let RootCommand::SelfManagement(args) = cli.command else {
        panic!()
    };
    let SelfCommand::Update(args) = args.command;
    assert!(args.check);
    assert!(!args.yes);
    wekan()
        .args(["self", "update", "--check", "--yes", "--output", "json"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("invalid_input"));
}

#[test]
fn self_update_rejects_wekan_targets_and_raw_output_before_network() {
    for flag in ["--server", "--profile"] {
        let output = wekan()
            .args([
                "self",
                "update",
                "--check",
                flag,
                if flag == "--server" {
                    "https://wekan.example"
                } else {
                    "work"
                },
                "--output",
                "json",
            ])
            .assert()
            .code(2)
            .get_output()
            .clone();
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], "invalid_input");
        assert!(
            error["error"]["message"]
                .as_str()
                .unwrap()
                .contains("self-management")
        );
    }
    wekan()
        .args(["self", "update", "--check", "--output", "raw"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("--output raw requires"));
}

#[test]
fn automated_local_commands_have_no_update_notices_with_opt_out_present_or_absent() {
    for disabled in [false, true] {
        let mut command = wekan();
        if disabled {
            command.env("WEKAN_NO_UPDATE_NOTIFIER", "");
        } else {
            command.env_remove("WEKAN_NO_UPDATE_NOTIFIER");
        }
        command
            .env("CI", "true")
            .args(["--output", "json", "profile", "list"])
            .assert()
            .success()
            .stdout(predicate::str::starts_with("{\"ok\":true,"))
            .stderr(predicate::str::is_empty());
    }
    wekan()
        .env_remove("CI")
        .env_remove("WEKAN_NO_UPDATE_NOTIFIER")
        .args(["profile", "list"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}
