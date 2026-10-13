use super::support::{TestDirectory, no_target_cli};
use clap::Parser;
use wekan_cli::{cli::Cli, commands::RootCommand, output::OutputFormat};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    let mut argv = vec!["wekan", "dependency"];
    argv.extend_from_slice(args);
    Cli::try_parse_from(argv)
}

#[test]
fn dependency_parser_covers_all_modes_and_metadata_states() {
    parse(&["list", "--board", "b"]).unwrap();
    parse(&["list", "--board", "b", "--card", "c"]).unwrap();
    for command in ["create", "update"] {
        for relation in [
            "related-to",
            "blocks",
            "is-blocked-by",
            "fixes",
            "is-fixed-by",
            "",
        ] {
            parse(&[
                command, "t", "--board", "b", "--card", "c", "--type", relation,
            ])
            .unwrap();
        }
        for metadata in [
            vec![],
            vec!["--color", "", "--icon", "  link  "],
            vec!["--null-type", "--null-color", "--null-icon"],
        ] {
            let mut args = vec![command, "t", "--board", "b", "--card", "c"];
            args.extend(metadata);
            parse(&args).unwrap();
        }
        for (field, null) in [
            ("--type", "--null-type"),
            ("--color", "--null-color"),
            ("--icon", "--null-icon"),
        ] {
            assert!(
                parse(&[command, "t", "--board", "b", "--card", "c", field, "", null]).is_err()
            );
        }
        assert!(
            parse(&[
                command, "t", "--board", "b", "--card", "c", "--type", "future"
            ])
            .is_err()
        );
        assert!(parse(&[command, "t", "--board", "b", "--card", "c", "--yes"]).is_err());
    }
    for command in ["create", "update", "delete"] {
        assert!(parse(&[command, "t", "--board", "b"]).is_err());
        assert!(parse(&[command, "--board", "b", "--card", "c"]).is_err());
        for id in ["", " ", ".", ".."] {
            assert!(parse(&[command, id, "--board", "b", "--card", "c"]).is_err());
        }
    }
    let cli = parse(&[
        "delete",
        "t",
        "--board",
        "b",
        "--card",
        "c",
        "--yes",
        "--server",
        "http://localhost:3000",
        "--profile",
        "local",
        "--output",
        "json",
        "--allow-insecure-http",
    ])
    .unwrap();
    assert!(matches!(cli.command, RootCommand::Dependency(_)));
    assert_eq!(cli.output, OutputFormat::Json);
    assert!(cli.allow_insecure_http);
    assert!(cli.command.validate_output(OutputFormat::Raw).is_err());
    assert!(parse(&["list", "--board", "b", "--list", "l"]).is_err());
}

#[test]
fn dependency_help_works_without_a_profile_and_raw_output_is_rejected() {
    let directory = TestDirectory::new("dependency-help");
    for command in ["list", "create", "update", "delete"] {
        no_target_cli(&directory)
            .args(["dependency", command, "--help"])
            .assert()
            .success();
    }
    no_target_cli(&directory)
        .args(["dependency", "list", "--board", "b", "--output", "raw"])
        .assert()
        .failure();
}
