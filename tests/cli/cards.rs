use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

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
    let output = cargo_bin_cmd!("wekan")
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
fn card_help_exposes_only_core_crud_and_supported_fields() {
    cargo_bin_cmd!("wekan")
        .args(["card", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("move").not())
        .stdout(predicate::str::contains("archive").not())
        .stdout(predicate::str::contains("comment").not())
        .stderr(predicate::str::is_empty());

    cargo_bin_cmd!("wekan")
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

    cargo_bin_cmd!("wekan")
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
fn only_card_delete_accepts_yes() {
    cargo_bin_cmd!("wekan")
        .args([
            "card", "delete", "--board", "board-1", "--list", "list-1", "card-1", "--help",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"));

    cargo_bin_cmd!("wekan")
        .args([
            "card", "get", "--board", "board-1", "--list", "list-1", "card-1", "--yes",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unexpected argument '--yes'"));
}
