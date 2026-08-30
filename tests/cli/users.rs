use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn user_help_lists_the_complete_command_family() {
    cargo_bin_cmd!("wekan")
        .args(["user", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("current"))
        .stdout(predicate::str::contains("cards"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("boards"))
        .stdout(predicate::str::contains("take-ownership"))
        .stdout(predicate::str::contains("disable-login"))
        .stdout(predicate::str::contains("enable-login"))
        .stdout(predicate::str::contains("delete"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn user_create_help_keeps_passwords_out_of_arguments() {
    cargo_bin_cmd!("wekan")
        .args(["user", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--username <USERNAME>"))
        .stdout(predicate::str::contains("--email <EMAIL>"))
        .stdout(predicate::str::contains("--password-stdin"))
        .stdout(predicate::str::contains("--password <").not())
        .stderr(predicate::str::is_empty());
}

#[test]
fn only_destructive_user_actions_accept_yes() {
    for action in ["take-ownership", "disable-login", "delete"] {
        cargo_bin_cmd!("wekan")
            .args(["user", action, "user-1", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--yes"));
    }
    cargo_bin_cmd!("wekan")
        .args(["user", "enable-login", "user-1", "--yes"])
        .assert()
        .failure()
        .code(2);
}
