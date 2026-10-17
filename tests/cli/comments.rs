use crate::support::wekan;
use predicates::prelude::*;

#[test]
fn comment_help_exposes_the_supported_lifecycle_and_text_field() {
    wekan()
        .args(["comment", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("update").not())
        .stderr(predicate::str::is_empty());

    wekan()
        .args([
            "comment", "create", "--board", "board-1", "--card", "card-1", "--help",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("--text <TEXT>"));
}

#[test]
fn only_comment_delete_accepts_yes() {
    wekan()
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

    wekan()
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
