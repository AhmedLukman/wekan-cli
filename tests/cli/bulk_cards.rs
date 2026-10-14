use super::support::{TestDirectory, no_target_cli};
use clap::Parser;
use wekan_cli::{cli::Cli, output::OutputFormat};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("wekan").chain(args.iter().copied()))
}

#[test]
fn bulk_and_label_parser_enforces_sources_targets_and_changes() {
    for source in ["--cards", "--cards-file", "--cards-stdin"] {
        let mut args = vec![
            "card", "bulk", "create", "--board", "b", "--list", "l", source,
        ];
        if source != "--cards-stdin" {
            args.push(if source == "--cards" {
                "[{}]"
            } else {
                "cards.json"
            });
        }
        let cli = parse(&args).unwrap();
        assert!(cli.command.validate_output(OutputFormat::Raw).is_err());
        args.push("--yes");
        assert!(parse(&args).is_err());
    }
    for args in [
        vec!["card", "bulk", "create", "--board", "b", "--list", "l"],
        vec![
            "card",
            "bulk",
            "create",
            "--board",
            "b",
            "--list",
            "l",
            "--cards",
            "[]",
            "--cards-stdin",
        ],
        vec!["card", "bulk", "delete", "--board", "b"],
        vec!["card", "bulk", "labels", "--board", "b", "--card", "c"],
        vec!["card", "label", "list", "--board", "b", "--card", "c"],
        vec![
            "board", "label", "add", "--board", "b", "--color", "#123456",
        ],
    ] {
        assert!(parse(&args).is_err(), "{args:?}");
    }
    for args in [
        vec![
            "card", "bulk", "delete", "--board", "b", "--card", "c", "--card", "c", "--yes",
        ],
        vec![
            "card",
            "bulk",
            "labels",
            "--board",
            "b",
            "--card",
            "c",
            "--add-label",
            "a",
            "--remove-label",
            "a",
        ],
        vec![
            "card", "label", "list", "--board", "b", "--list", "l", "--card", "c",
        ],
        vec!["board", "label", "list", "--board", "b"],
        vec!["board", "label", "add", "--board", "b"],
    ] {
        parse(&args).unwrap();
    }
}

#[test]
fn bulk_help_and_input_failures_have_correct_streams_and_exit_statuses() {
    let directory = TestDirectory::new("bulk-help-input");
    for args in [
        vec!["card", "bulk", "--help"],
        vec!["card", "bulk", "delete", "--help"],
    ] {
        let output = no_target_cli(&directory).args(args).output().unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .contains("Wekan v11.06 retains attachments")
        );
    }
    for args in [
        vec!["card", "bulk", "create"],
        vec!["card", "bulk", "delete"],
        vec!["card", "bulk", "labels"],
        vec!["card", "label", "list"],
        vec!["board", "label", "add"],
        vec!["board", "label", "list"],
    ] {
        no_target_cli(&directory)
            .args(args)
            .arg("--help")
            .assert()
            .success();
    }
    no_target_cli(&directory)
        .args([
            "profile",
            "add",
            "default",
            "http://localhost:3116",
            "--use",
        ])
        .assert()
        .success();
    for input in [
        "[]",
        "null",
        "[{\"unknown\":1}]",
        "[{\"title\":4}]",
        "[{\"swimlaneId\":null}]",
    ] {
        let out = no_target_cli(&directory)
            .args([
                "--output", "json", "card", "bulk", "create", "--board", "b", "--list", "l",
                "--cards", input,
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(out.stdout.is_empty());
        let value: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
        assert_eq!(value["error"]["code"], "invalid_input");
    }
    no_target_cli(&directory)
        .args([
            "--output",
            "json",
            "card",
            "bulk",
            "create",
            "--board",
            "b",
            "--list",
            "l",
            "--cards-stdin",
        ])
        .write_stdin("[]")
        .assert()
        .code(2);
    let file = directory.0.join("cards.json");
    std::fs::write(&file, b"[]").unwrap();
    no_target_cli(&directory)
        .args([
            "card",
            "bulk",
            "create",
            "--board",
            "b",
            "--list",
            "l",
            "--cards-file",
        ])
        .arg(&file)
        .assert()
        .code(2);
    no_target_cli(&directory)
        .args([
            "card",
            "bulk",
            "create",
            "--board",
            "b",
            "--list",
            "l",
            "--cards-file",
        ])
        .arg(directory.0.join("missing.json"))
        .assert()
        .code(4);
    for count in [1, 500, 501] {
        let cards = serde_json::to_string(&vec![serde_json::json!({}); count]).unwrap();
        let out = no_target_cli(&directory)
            .args([
                "--output", "json", "card", "bulk", "create", "--board", "b", "--list", "l",
                "--cards", &cards,
            ])
            .output()
            .unwrap();
        let error: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
        assert_eq!(error["error"]["code"] == "invalid_input", count == 501);
    }
}
