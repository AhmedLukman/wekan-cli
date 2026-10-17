use crate::support::wekan;
use predicates::prelude::*;

#[test]
fn swimlane_help_exposes_crud_transfers_and_create_sort() {
    wekan()
        .args(["swimlane", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("copy"))
        .stdout(predicate::str::contains("move"))
        .stdout(predicate::str::contains("archive").not())
        .stderr(predicate::str::is_empty());

    wekan()
        .args(["swimlane", "create", "--board", "board-1", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--title <TITLE>"))
        .stdout(predicate::str::contains("--sort <SORT>"));

    wekan()
        .args(["swimlane", "update", "--board", "board-1", "swimlane-1"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("--title <TITLE>"));
}

#[test]
fn only_swimlane_delete_accepts_yes() {
    wekan()
        .args([
            "swimlane",
            "delete",
            "--board",
            "board-1",
            "swimlane-1",
            "--help",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"));

    wekan()
        .args([
            "swimlane",
            "get",
            "--board",
            "board-1",
            "swimlane-1",
            "--yes",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unexpected argument '--yes'"));
}
