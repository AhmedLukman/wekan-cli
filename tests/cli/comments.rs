use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn comment_help_exposes_the_supported_lifecycle_and_text_field() {
    cargo_bin_cmd!("wekan")
        .args(["comment", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("update").not())
        .stderr(predicate::str::is_empty());

    cargo_bin_cmd!("wekan")
        .args([
            "comment", "create", "--board", "board-1", "--card", "card-1", "--help",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("--text <TEXT>"));
}

#[test]
fn only_comment_delete_accepts_yes() {
    cargo_bin_cmd!("wekan")
        .args([
            "comment",
            "delete",
            "--board",
            "board-1",
            "--card",
            "card-1",
            "comment-1",
            "--help",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"));

    cargo_bin_cmd!("wekan")
        .args([
            "comment",
            "get",
            "--board",
            "board-1",
            "--card",
            "card-1",
            "comment-1",
            "--yes",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unexpected argument '--yes'"));
}
