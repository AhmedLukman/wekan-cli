use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn copy_move_and_read_help_expose_complete_interfaces() {
    for (family, action, flags) in [
        ("board", "copy", vec!["--title"]),
        (
            "card",
            "copy",
            vec![
                "--board",
                "--list",
                "--target-board",
                "--target-list",
                "--target-swimlane-id",
                "--position",
                "--position-json",
            ],
        ),
        (
            "list",
            "copy",
            vec![
                "--board",
                "--target-board",
                "--target-swimlane-id",
                "--position-json",
            ],
        ),
        (
            "list",
            "move",
            vec!["--board", "--target-board", "--position"],
        ),
        (
            "swimlane",
            "copy",
            vec!["--board", "--target-board", "--title", "--position-json"],
        ),
        (
            "swimlane",
            "move",
            vec!["--board", "--target-board", "--title", "--position"],
        ),
        ("card", "count", vec!["--board", "--list"]),
        ("card", "list", vec!["--board", "--list", "--swimlane"]),
        ("card", "get", vec!["--board", "--list"]),
    ] {
        let output = cargo_bin_cmd!("wekan")
            .args([family, action, "--help"])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let help = String::from_utf8(output).unwrap();
        for flag in flags.into_iter().chain([
            "--server",
            "--profile",
            "--output",
            "--allow-insecure-http",
            "--help",
        ]) {
            assert!(help.contains(flag), "{family} {action}: {flag}");
        }
        assert!(!help.contains("--yes"));
    }
}

#[test]
fn transfer_and_card_read_invalid_inputs_fail_before_target_resolution() {
    for args in [
        vec!["card", "get", "c", "--board", "b"],
        vec!["card", "get", "c", "--list", "l"],
        vec!["card", "list", "--board", "b"],
        vec![
            "card",
            "list",
            "--board",
            "b",
            "--list",
            "l",
            "--swimlane",
            "s",
        ],
        vec!["card", "count"],
        vec!["card", "copy", "c", "--board", "b", "--list", "l"],
        vec![
            "card",
            "copy",
            "c",
            "--board",
            "b",
            "--list",
            "l",
            "--target-swimlane-id",
            "s",
            "--position-json",
            "false",
        ],
        vec![
            "list",
            "copy",
            "l",
            "--board",
            "b",
            "--position",
            "1",
            "--position-json",
            "null",
        ],
        vec!["swimlane", "move", "s", "--board", "b", "--position", "NaN"],
        vec!["list", "copy", "l", "--board", "b", "--position-json", "{"],
        vec!["board", "copy", ".."],
        vec!["card", "get", "."],
        vec!["list", "move", "l", "--board", " "],
    ] {
        cargo_bin_cmd!("wekan")
            .args(args)
            .assert()
            .code(2)
            .stdout(predicate::str::is_empty());
    }
}
