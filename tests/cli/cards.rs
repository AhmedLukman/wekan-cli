use crate::support::wekan;
use predicates::prelude::*;

#[test]
fn card_list_help_and_errors_show_one_required_scope_selector() {
    use clap::Parser;
    use wekan_cli::cli::Cli;

    for selector in ["--list", "--swimlane"] {
        assert!(
            Cli::try_parse_from(["wekan", "card", "list", "--board", "b", selector, "scope"])
                .is_ok()
        );
    }
    for args in [
        vec!["card", "list", "--help"],
        vec!["card", "list", "--board", "b"],
    ] {
        let output = wekan().args(&args).output().unwrap();
        let text = if args.contains(&"--help") {
            assert!(output.status.success());
            String::from_utf8(output.stdout).unwrap()
        } else {
            assert_eq!(output.status.code(), Some(2));
            String::from_utf8(output.stderr).unwrap()
        };
        let usage = text
            .lines()
            .find(|line| line.starts_with("Usage:"))
            .unwrap();
        assert!(
            usage.contains("--list <LIST_ID>|--swimlane <SWIMLANE_ID>"),
            "{usage}"
        );
    }
}

#[test]
fn scoped_card_syntax_requires_named_parents_and_help_is_readable() {
    let directory = super::support::TestDirectory::new("named-card-parents");
    for args in [
        vec!["card", "get", "board-1", "list-1", "card-1"],
        vec!["card", "get", "card-1", "--board", "board-1"],
        vec!["card", "get", "card-1", "--list", "list-1"],
    ] {
        super::support::no_target_cli(&directory)
            .args(args)
            .assert()
            .code(2)
            .stdout(predicate::str::is_empty());
    }
    let output = wekan()
        .args(["card", "update", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let help = String::from_utf8(output).unwrap();
    let usage = help
        .lines()
        .find(|line| line.starts_with("Usage:"))
        .unwrap();
    assert!(usage.len() < 120, "{usage}");
    for heading in [
        "Target:",
        "Fields:",
        "Dates:",
        "People and labels:",
        "Global options:",
        "Example:",
    ] {
        assert!(help.contains(heading), "missing {heading}: {help}");
    }
    assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 0);
}

#[test]
fn card_help_exposes_core_crud_archive_restore_and_supported_fields() {
    wekan()
        .args(["card", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("move").not())
        .stdout(predicate::str::contains("archive"))
        .stdout(predicate::str::contains("restore"))
        .stdout(predicate::str::contains("comment").not())
        .stderr(predicate::str::is_empty());

    wekan()
        .args([
            "card", "create", "--board", "board-1", "--list", "list-1", "--help",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("--title <TITLE>"))
        .stdout(predicate::str::contains("--swimlane-id <SWIMLANE_ID>"))
        .stdout(predicate::str::contains("--member <MEMBERS>"))
        .stdout(predicate::str::contains("--assignee <ASSIGNEES>"))
        .stdout(predicate::str::contains("--due-at <DUE_AT>"));

    wekan()
        .args([
            "card", "update", "--board", "board-1", "--list", "list-1", "card-1", "--help",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("--clear-labels"))
        .stdout(predicate::str::contains("--clear-members"))
        .stdout(predicate::str::contains("--clear-assignees"))
        .stdout(predicate::str::contains("--clear-due-at"))
        .stdout(predicate::str::contains("--spent-time <SPENT_TIME>"))
        .stdout(predicate::str::contains("--sort <SORT>"))
        .stdout(predicate::str::contains("--is-over-time <IS_OVER_TIME>"))
        .stdout(predicate::str::contains("--due-complete <DUE_COMPLETE>"));
}

#[test]
fn archive_restore_require_scoped_identifiers_and_expose_global_help() {
    let directory = super::support::TestDirectory::new("archive-restore-parser");
    for action in ["archive", "restore"] {
        for args in [
            vec!["card", action, "card-1"],
            vec!["card", action, "card-1", "--board", "board-1"],
            vec!["card", action, "card-1", "--list", "list-1"],
            vec![
                "card", action, " ", "--board", "board-1", "--list", "list-1",
            ],
            vec!["card", action, "card-1", "--board", " ", "--list", "list-1"],
            vec![
                "card", action, "card-1", "--board", "board-1", "--list", " ",
            ],
            vec![
                "card", action, "card-1", "--board", "board-1", "--list", "list-1", "--yes",
            ],
        ] {
            super::support::no_target_cli(&directory)
                .args(args)
                .assert()
                .code(2)
                .stdout(predicate::str::is_empty());
        }
        wekan()
            .args(["card", action, "--help"])
            .assert()
            .success()
            .stderr(predicate::str::is_empty())
            .stdout(predicate::str::contains("--board <BOARD_ID>"))
            .stdout(predicate::str::contains("--list <LIST_ID>"))
            .stdout(predicate::str::contains("<CARD_ID>"))
            .stdout(predicate::str::contains("--server"))
            .stdout(predicate::str::contains("--profile"))
            .stdout(predicate::str::contains("--output"))
            .stdout(predicate::str::contains("--allow-insecure-http"))
            .stdout(predicate::str::contains("--yes").not())
            .stdout(predicate::str::contains("Example:"));
    }
    assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 0);
}

#[test]
fn only_card_delete_accepts_yes() {
    wekan()
        .args([
            "card", "delete", "--board", "board-1", "--list", "list-1", "card-1", "--help",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"));

    wekan()
        .args([
            "card", "get", "--board", "board-1", "--list", "list-1", "card-1", "--yes",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unexpected argument '--yes'"));
}
