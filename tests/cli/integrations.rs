use super::support::{TestDirectory, no_target_cli};
use clap::Parser;
use wekan_cli::{cli::Cli, commands::RootCommand, output::OutputFormat};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    let mut argv = vec!["wekan", "integration"];
    argv.extend_from_slice(args);
    Cli::try_parse_from(argv)
}

#[test]
fn integration_parser_covers_required_inputs_and_conflicts() {
    for args in [
        vec!["list", "--board", "b"],
        vec!["get", "i", "--board", "b"],
        vec!["create", "--board", "b", "--url", ""],
        vec!["update", "i", "--board", "b"],
        vec![
            "update",
            "i",
            "--board",
            "b",
            "--enabled",
            "false",
            "--title",
            "",
            "--activity",
            "all",
            "--activity",
            "future",
        ],
        vec!["update", "i", "--board", "b", "--token-stdin"],
        vec![
            "update",
            "i",
            "--board",
            "b",
            "--clear-token",
            "--clear-activities",
        ],
        vec!["delete", "i", "--board", "b", "--yes"],
        vec![
            "activity",
            "add",
            "i",
            "--board",
            "b",
            "--activity",
            "",
            "--activity",
            "a",
            "--activity",
            "a",
        ],
        vec![
            "activity",
            "remove",
            "i",
            "--board",
            "b",
            "--activity",
            "all",
        ],
    ] {
        parse(&args).unwrap();
    }
    for args in [
        vec!["list"],
        vec!["get", "i"],
        vec!["get", "--board", "b"],
        vec!["create", "--board", "b"],
        vec!["update", "i", "--board", "b", "--token", "secret"],
        vec![
            "update",
            "i",
            "--board",
            "b",
            "--token-stdin",
            "--clear-token",
        ],
        vec![
            "update",
            "i",
            "--board",
            "b",
            "--activity",
            "all",
            "--clear-activities",
        ],
        vec!["update", "i", "--board", "b", "--enabled", "yes"],
        vec!["activity", "add", "i", "--board", "b"],
        vec![
            "activity",
            "remove",
            "i",
            "--board",
            "b",
            "--activity",
            "all",
            "--yes",
        ],
    ] {
        assert!(parse(&args).is_err(), "{args:?}");
    }
    for id in ["", " ", ".", ".."] {
        assert!(parse(&["get", id, "--board", "b"]).is_err());
        assert!(parse(&["list", "--board", id]).is_err());
    }
    let cli = parse(&[
        "list",
        "--board",
        "b",
        "--server",
        "http://localhost:3000",
        "--profile",
        "local",
        "--output",
        "json",
        "--allow-insecure-http",
    ])
    .unwrap();
    assert!(matches!(cli.command, RootCommand::Integration(_)));
    assert_eq!(cli.output, OutputFormat::Json);
    assert!(cli.allow_insecure_http);
    assert!(cli.command.validate_output(OutputFormat::Raw).is_err());
}

#[test]
fn integration_help_is_available_without_a_profile() {
    let directory = TestDirectory::new("integration-help");
    for command in ["list", "get", "create", "update", "delete", "activity"] {
        no_target_cli(&directory)
            .args(["integration", command, "--help"])
            .assert()
            .success();
    }
    for command in ["add", "remove"] {
        no_target_cli(&directory)
            .args(["integration", "activity", command, "--help"])
            .assert()
            .success();
    }
    no_target_cli(&directory)
        .args(["integration", "list", "--board", "b", "--output", "raw"])
        .assert()
        .failure();
}
