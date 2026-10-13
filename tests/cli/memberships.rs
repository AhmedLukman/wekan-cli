use super::support::{TestDirectory, no_target_cli};
use clap::Parser;
use wekan_cli::{cli::Cli, output::OutputFormat};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("wekan").chain(args.iter().copied()))
}

#[test]
fn membership_parser_covers_nested_commands_and_permission_inputs() {
    for role in [
        "admin",
        "normal",
        "member",
        "comment",
        "commentonly",
        "readonly",
        "worker",
        "normalassignedonly",
        "commentassignedonly",
        "readassignedonly",
        "nocomments",
        "  CommentOnly  ",
    ] {
        for command in ["add", "update"] {
            parse(&[
                "board",
                "member",
                command,
                "u",
                "--board",
                "b",
                "--role",
                role,
                "--is-admin",
                "false",
            ])
            .unwrap();
        }
    }
    for flag in [
        "--is-admin",
        "--is-no-comments",
        "--is-comment-only",
        "--is-worker",
        "--is-normal-assigned-only",
        "--is-comment-assigned-only",
        "--is-read-only",
        "--is-read-assigned-only",
    ] {
        for value in ["true", "false"] {
            parse(&[
                "board", "member", "update", "u", "--board", "b", flag, value,
            ])
            .unwrap();
        }
        assert!(
            parse(&[
                "board", "member", "update", "u", "--board", "b", flag, "yes"
            ])
            .is_err()
        );
    }
    parse(&["board", "member", "list", "--board", "b"]).unwrap();
    parse(&["board", "member", "add", "u", "--board", "b"]).unwrap();
    parse(&["board", "member", "remove", "u", "--board", "b"]).unwrap();
    assert!(parse(&["board", "member", "update", "u", "--board", "b"]).is_err());
    assert!(
        parse(&[
            "board", "member", "add", "u", "--board", "b", "--role", "future"
        ])
        .is_err()
    );
    for family in ["member", "assignee"] {
        parse(&[
            "card", family, "list", "--board", "b", "--list", "l", "--card", "c",
        ])
        .unwrap();
        for command in ["add", "remove"] {
            let args = [
                "card", family, command, "u", "--board", "b", "--list", "l", "--card", "c",
            ];
            parse(&args).unwrap();
            for selector in ["--board", "--list", "--card"] {
                let missing = args.iter().position(|arg| *arg == selector).unwrap();
                let mut incomplete = args.to_vec();
                incomplete.drain(missing..missing + 2);
                assert!(parse(&incomplete).is_err());
            }
            for invalid in ["", " ", ".", ".."] {
                let mut invalid_args = args;
                invalid_args[3] = invalid;
                assert!(parse(&invalid_args).is_err());
            }
            let mut with_yes = args.to_vec();
            with_yes.push("--yes");
            assert!(parse(&with_yes).is_err());
        }
    }
    let cli = parse(&[
        "board",
        "member",
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
    assert_eq!(cli.output, OutputFormat::Json);
    assert!(cli.allow_insecure_http);
    assert!(cli.command.validate_output(OutputFormat::Raw).is_err());
}

#[test]
fn every_membership_help_command_works_without_a_profile() {
    let directory = TestDirectory::new("membership-help");
    for (resource, family, verbs) in [
        ("board", "member", vec!["list", "add", "remove", "update"]),
        ("card", "member", vec!["list", "add", "remove"]),
        ("card", "assignee", vec!["list", "add", "remove"]),
    ] {
        for verb in verbs {
            no_target_cli(&directory)
                .args([resource, family, verb, "--help"])
                .assert()
                .success();
        }
    }
}
