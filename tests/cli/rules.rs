use super::support::{TestDirectory, add_profile, no_target_cli};
use clap::Parser;
use wekan_cli::{cli::Cli, commands::RootCommand, output::OutputFormat};
fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    let mut argv = vec!["wekan", "rule"];
    argv.extend_from_slice(args);
    Cli::try_parse_from(argv)
}
#[test]
fn rule_parser_enforces_targets_sources_conflicts_and_global_options() {
    for args in [
        vec!["list", "--board", "b"],
        vec!["get", "r", "--board", "b"],
        vec!["create", "--board", "b", "--rule", "{}"],
        vec!["create", "--board", "b", "--rule-file", "rule.json"],
        vec!["update", "r", "--board", "b", "--rule-stdin"],
        vec!["delete", "r", "--board", "b", "--yes"],
    ] {
        parse(&args).unwrap();
    }
    for args in [
        vec!["list"],
        vec!["get", "--board", "b"],
        vec!["create", "--board", "b"],
        vec!["update", "r", "--board", "b"],
        vec!["create", "--board", "b", "--rule", "{}", "--rule-stdin"],
        vec![
            "update",
            "r",
            "--board",
            "b",
            "--rule",
            "{}",
            "--rule-file",
            "r.json",
        ],
        vec![
            "update",
            "r",
            "--board",
            "b",
            "--rule-file",
            "r.json",
            "--rule-stdin",
        ],
        vec!["update", "r", "--board", "b", "--rule", "{}", "--yes"],
    ] {
        assert!(parse(&args).is_err(), "{args:?}");
    }
    for id in ["", " ", ".", "..", "\u{feff}"] {
        assert!(parse(&["get", id, "--board", "b"]).is_err());
        assert!(parse(&["list", "--board", id]).is_err());
    }
    let cli = parse(&[
        "list",
        "--board",
        "b",
        "--server",
        "http://localhost:3117",
        "--profile",
        "local",
        "--output",
        "json",
        "--allow-insecure-http",
    ])
    .unwrap();
    assert!(matches!(cli.command, RootCommand::Rule(_)));
    assert_eq!(cli.output, OutputFormat::Json);
    assert!(cli.allow_insecure_http);
    assert!(cli.command.validate_output(OutputFormat::Raw).is_err());
}
#[test]
fn help_needs_no_target_and_explains_mutation_semantics() {
    let dir = TestDirectory::new("rule-help");
    for sub in ["list", "get", "create", "update", "delete"] {
        no_target_cli(&dir)
            .args(["rule", sub, "--help"])
            .assert()
            .success();
    }
    no_target_cli(&dir)
        .args(["rule", "update", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("shallow-merges"));
}
#[test]
fn stdin_is_read_as_a_bounded_typed_definition_before_credentials() {
    let dir = TestDirectory::new("rule-stdin");
    add_profile(&dir, "default", "http://localhost:1");
    for body in [
        "[]",
        "null",
        r#"{"trigger":{"extra":"do not echo"}}"#,
        "{",
        r#"{"trigger":{"lastRunAt":"do not echo"}}"#,
    ] {
        let result = no_target_cli(&dir)
            .args([
                "--server",
                "http://localhost:1",
                "--output",
                "json",
                "rule",
                "update",
                "r",
                "--board",
                "b",
                "--rule-stdin",
            ])
            .write_stdin(body)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(error["error"]["code"], "invalid_input");
        assert!(!String::from_utf8_lossy(&result.stderr).contains("do not echo"));
    }
    // An empty update is valid JSON; it advances to the normal credential boundary.
    let result = no_target_cli(&dir)
        .args([
            "--server",
            "http://localhost:1",
            "--output",
            "json",
            "rule",
            "update",
            "r",
            "--board",
            "b",
            "--rule-stdin",
        ])
        .write_stdin("{}")
        .output()
        .unwrap();
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_ne!(error["error"]["code"], "invalid_input");
}
