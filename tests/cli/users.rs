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

#[test]
fn token_commands_have_explicit_modes_and_inherit_global_options() {
    use clap::{CommandFactory, Parser};
    use wekan_cli::{cli::Cli, output::OutputFormat};
    let mut root = Cli::command();
    root.build();
    let token = root
        .find_subcommand_mut("user")
        .unwrap()
        .find_subcommand_mut("token")
        .unwrap();
    let help = token.render_help().to_string();
    assert!(help.contains("create"));
    assert!(help.contains("delete"));
    for args in [
        vec!["create"],
        vec!["create", ""],
        vec!["create", "u", "--yes"],
        vec!["delete", "u"],
        vec!["delete", "u", "--all", "--token-stdin"],
        vec!["delete", "u", "--token", "secret"],
    ] {
        assert!(Cli::try_parse_from([vec!["wekan", "user", "token"], args].concat()).is_err());
    }
    for position in 1..=4 {
        let mut args = vec!["wekan", "user", "token", "delete", "u", "--all", "--yes"];
        args.splice(
            position..position,
            [
                "--output",
                "json",
                "--profile",
                "team",
                "--server",
                "https://wekan.example",
                "--allow-insecure-http",
            ],
        );
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.output, OutputFormat::Json);
        assert_eq!(cli.profile_name.as_deref(), Some("team"));
        assert!(cli.allow_insecure_http);
    }
    cargo_bin_cmd!("wekan")
        .args(["user", "token", "delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--token-stdin"))
        .stdout(predicate::str::contains("--all"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--token <").not());
}
