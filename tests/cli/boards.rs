use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn board_help_lists_the_core_lifecycle() {
    cargo_bin_cmd!("wekan")
        .args(["board", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("count"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("rename"))
        .stdout(predicate::str::contains("delete"))
        .stderr(predicate::str::is_empty());

    cargo_bin_cmd!("wekan")
        .args(["board", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--title <TITLE>"))
        .stdout(predicate::str::contains("--owner <OWNER>"))
        .stdout(predicate::str::contains("--permission <PERMISSION>"))
        .stdout(predicate::str::contains("--color <COLOR>"))
        .stdout(predicate::str::contains("--no-comments"))
        .stdout(predicate::str::contains("--comment-only"))
        .stdout(predicate::str::contains("--worker"));
}

#[test]
fn only_board_delete_accepts_yes() {
    cargo_bin_cmd!("wekan")
        .args(["board", "delete", "board-1", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"));

    cargo_bin_cmd!("wekan")
        .args(["board", "get", "board-1", "--yes"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unexpected argument '--yes'"));
}
