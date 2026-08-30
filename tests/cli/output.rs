use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn help_is_text_on_stdout() {
    cargo_bin_cmd!("wekan")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: wekan"))
        .stdout(predicate::str::contains("auth"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn yes_is_command_local_to_destructive_commands() {
    for arguments in [
        vec!["--yes", "auth", "status"],
        vec![
            "--server",
            "https://wekan.example",
            "auth",
            "login",
            "--username",
            "alice",
            "--yes",
        ],
        vec![
            "--server",
            "https://wekan.example",
            "auth",
            "register",
            "--username",
            "alice",
            "--yes",
        ],
        vec![
            "--server",
            "https://wekan.example",
            "auth",
            "status",
            "--yes",
        ],
        vec!["profile", "add", "work", "https://wekan.example", "--yes"],
        vec!["profile", "list", "--yes"],
        vec!["profile", "show", "work", "--yes"],
        vec!["profile", "use", "work", "--yes"],
        vec![
            "profile",
            "update",
            "work",
            "https://wekan.example",
            "--yes",
        ],
    ] {
        cargo_bin_cmd!("wekan")
            .args(arguments)
            .assert()
            .code(2)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("unexpected argument '--yes'"));
    }

    cargo_bin_cmd!("wekan")
        .args(["profile", "remove", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("--yes"));
}
