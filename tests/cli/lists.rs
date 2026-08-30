use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn list_help_exposes_only_core_crud_and_full_update_fields() {
    cargo_bin_cmd!("wekan")
        .args(["list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("copy").not())
        .stdout(predicate::str::contains("move").not())
        .stderr(predicate::str::is_empty());

    cargo_bin_cmd!("wekan")
        .args(["list", "update", "board-1", "list-1", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--title <TITLE>"))
        .stdout(predicate::str::contains("--color <COLOR>"))
        .stdout(predicate::str::contains("--starred <STARRED>"))
        .stdout(predicate::str::contains("--wip-limit <WIP_LIMIT>"))
        .stdout(predicate::str::contains("--wip-enabled <WIP_ENABLED>"))
        .stdout(predicate::str::contains("--wip-soft <WIP_SOFT>"));
}

#[test]
fn only_list_delete_accepts_yes() {
    cargo_bin_cmd!("wekan")
        .args(["list", "delete", "board-1", "list-1", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"));

    cargo_bin_cmd!("wekan")
        .args(["list", "get", "board-1", "list-1", "--yes"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unexpected argument '--yes'"));
}
