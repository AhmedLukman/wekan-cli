use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn board_settings_help_exposes_the_complete_typed_interface() {
    cargo_bin_cmd!("wekan")
        .args(["board", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("card-settings"))
        .stdout(predicate::str::contains("domain"));
    cargo_bin_cmd!("wekan")
        .args(["board", "card-settings", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--allows-creator-on-minicard"))
        .stdout(predicate::str::contains("--card-aging-days3"))
        .stdout(predicate::str::contains("--board"))
        .stdout(predicate::str::contains("--server"))
        .stdout(predicate::str::contains("--profile"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--allow-insecure-http"));
    cargo_bin_cmd!("wekan")
        .args(["board", "domain", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("site administrator"))
        .stdout(predicate::str::contains("not-authorized"));
}

#[test]
fn invalid_settings_fail_before_configuration_or_network_access() {
    for args in [
        vec!["board", "card-settings", "update", "--board", "b"],
        vec![
            "board",
            "card-settings",
            "update",
            "--board",
            "b",
            "--allows-labels",
            "null",
        ],
        vec![
            "board",
            "card-settings",
            "update",
            "--board",
            "b",
            "--card-aging-days3",
            "9007199254740992",
        ],
        vec!["board", "domain", "remove", "..", "--board", "b"],
        vec![
            "board", "domain", "remove", "x.test", "--board", "b", "--yes",
        ],
    ] {
        cargo_bin_cmd!("wekan")
            .args(args)
            .assert()
            .failure()
            .code(2)
            .stdout(predicate::str::is_empty());
    }
}
