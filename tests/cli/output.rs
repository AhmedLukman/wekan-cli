use crate::support::wekan;
use predicates::prelude::*;

#[test]
fn help_is_text_on_stdout() {
    wekan()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: wekan"))
        .stdout(predicate::str::contains("auth"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn parser_errors_honor_output_overrides_across_command_levels() {
    let success = wekan()
        .args(["--output", "human", "profile", "list", "--output=json"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .clone();
    let envelope: serde_json::Value = serde_json::from_slice(&success.stdout).unwrap();
    assert_eq!(envelope["ok"], true);

    for output in ["--output=json", "--output"] {
        let mut command = wekan();
        command.args(["--output", "human", "profile", "show", output]);
        if output == "--output" {
            command.arg("json");
        }
        let result = command.assert().code(2).get_output().clone();
        assert!(result.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
        assert_eq!(error["ok"], false);
        assert_eq!(error["error"]["code"], "invalid_input");
    }

    wekan()
        .args(["--output=json", "profile", "show", "--output", "human"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::starts_with("Error [invalid_input]:"));

    // A positional argument after -- cannot select an output format.
    wekan()
        .args(["profile", "show", "--", "--output=json"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::starts_with("Error [invalid_input]:"));

    let result = wekan()
        .args([
            "--output=json",
            "user",
            "cards",
            "--from",
            "--output=human",
            "--unknown",
        ])
        .assert()
        .code(2)
        .get_output()
        .clone();
    assert!(result.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["error"]["code"], "invalid_input");
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
        wekan()
            .args(arguments)
            .assert()
            .code(2)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("unexpected argument '--yes'"));
    }

    wekan()
        .args(["profile", "remove", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("--yes"));
}
