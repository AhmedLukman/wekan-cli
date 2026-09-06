use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn swimlane_help_exposes_only_core_crud_and_create_sort() {
    cargo_bin_cmd!("wekan")
        .args(["swimlane", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("copy").not())
        .stdout(predicate::str::contains("move").not())
        .stdout(predicate::str::contains("archive").not())
        .stderr(predicate::str::is_empty());

    cargo_bin_cmd!("wekan")
        .args(["swimlane", "create", "--board", "board-1", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--title <TITLE>"))
        .stdout(predicate::str::contains("--sort <SORT>"));

    cargo_bin_cmd!("wekan")
        .args(["swimlane", "update", "--board", "board-1", "swimlane-1"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("--title <TITLE>"));
}

#[test]
fn only_swimlane_delete_accepts_yes() {
    cargo_bin_cmd!("wekan")
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

    cargo_bin_cmd!("wekan")
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
