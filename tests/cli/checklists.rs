use crate::support::wekan;
use clap::Parser;
use predicates::prelude::*;
use wekan_cli::cli::Cli;

fn parse(family: &str, args: &[&str]) -> Result<Cli, clap::Error> {
    let mut argv = vec!["wekan"];
    argv.extend(family.split_whitespace());
    argv.extend_from_slice(args);
    argv.extend(["--board", "board-1", "--card", "card-1"]);
    if family == "checklist item" {
        argv.extend(["--checklist", "checklist-1"]);
    }
    Cli::try_parse_from(argv)
}

#[test]
fn checklist_help_exposes_exactly_the_supported_lifecycle() {
    for family in ["checklist", "checklist item"] {
        let assertion = wekan()
            .args(family.split_whitespace().chain(["--help"]))
            .assert()
            .success()
            .stdout(predicate::str::contains("create"))
            .stdout(predicate::str::contains("get"))
            .stdout(predicate::str::contains("delete"))
            .stderr(predicate::str::is_empty());
        if family == "checklist" {
            assertion
                .stdout(predicate::str::contains("list"))
                .stdout(predicate::str::contains("update").not());
        } else {
            assertion
                .stdout(predicate::str::contains("update"))
                .stdout(predicate::str::contains("    list").not());
        }
    }
    for (family, verb) in [
        ("checklist", "create"),
        ("checklist item", "create"),
        ("checklist item", "update"),
    ] {
        wekan()
            .args(family.split_whitespace().chain([verb, "--help"]))
            .assert()
            .success()
            .stdout(predicate::str::contains("--yes").not());
    }
}

#[test]
fn checklist_parser_supports_defaults_repetition_and_exact_json_forms() {
    for args in [
        vec!["create"],
        vec!["list"],
        vec!["get", "id"],
        vec!["create", "--title", ""],
        vec!["create", "--item", "one", "--item", "two"],
    ] {
        parse("checklist", &args).unwrap();
    }
    for value in ["null", "[]", r#""""#, r#""One""#, r#"["One","Two"]"#] {
        parse("checklist", &["create", "--items", value]).unwrap();
    }
    for value in ["null", r#""""#, r#""NaN""#, r#""0x10""#, "-1.5", "1e309"] {
        parse(
            "checklist item",
            &["create", "--title", "One", "--sort-json", value],
        )
        .unwrap();
    }
    for value in ["-1.5", "0", "1.5"] {
        parse(
            "checklist item",
            &["create", "--title", "One", "--sort", value],
        )
        .unwrap();
    }
    for value in ["true", "false", r#""TRUE""#, r#""anything""#] {
        parse(
            "checklist item",
            &["update", "id", "--is-finished-json", value],
        )
        .unwrap();
    }
    parse("checklist item", &["update", "id"]).unwrap();
    parse(
        "checklist item",
        &["update", "id", "--title", "", "--is-finished", "false"],
    )
    .unwrap();
}

#[test]
fn checklist_parser_rejects_conflicts_invalid_json_and_unsupported_commands() {
    for args in [
        vec!["update", "id"],
        vec!["create", "--item", "one", "--items", "[]"],
        vec!["create", "--items", "[null]"],
        vec!["create", "--items", "{}"],
        vec!["get", "id", "--yes"],
        vec!["get", " "],
    ] {
        assert!(parse("checklist", &args).is_err(), "{args:?}");
    }
    for args in [
        vec!["list"],
        vec!["create"],
        vec![
            "create",
            "--title",
            "One",
            "--sort",
            "1",
            "--sort-json",
            "null",
        ],
        vec!["create", "--title", "One", "--sort", "NaN"],
        vec!["create", "--title", "One", "--sort", "inf"],
        vec!["create", "--title", "One", "--sort-json", "true"],
        vec![
            "update",
            "id",
            "--is-finished",
            "true",
            "--is-finished-json",
            "false",
        ],
        vec!["update", "id", "--is-finished-json", "null"],
        vec!["update", "id", "--is-finished-json", "1"],
        vec!["update", "id", "--yes"],
    ] {
        assert!(parse("checklist item", &args).is_err(), "{args:?}");
    }
    for argv in [
        vec!["wekan", "checklist", "list", "--board", "board"],
        vec![
            "wekan",
            "checklist",
            "item",
            "get",
            "id",
            "--board",
            "board",
            "--card",
            "card",
        ],
        vec![
            "wekan",
            "checklist",
            "list",
            "--board",
            " ",
            "--card",
            "card",
        ],
    ] {
        assert!(Cli::try_parse_from(argv).is_err());
    }
}
