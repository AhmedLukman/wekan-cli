#[path = "cli/api.rs"]
mod api;
#[path = "cli/auth.rs"]
mod auth;
#[path = "cli/board_settings.rs"]
mod board_settings;
#[path = "cli/boards.rs"]
mod boards;
#[path = "cli/cards.rs"]
mod cards;
#[path = "cli/checklists.rs"]
mod checklists;
#[path = "cli/comments.rs"]
mod comments;
#[path = "cli/lists.rs"]
mod lists;
#[path = "cli/output.rs"]
mod output;
#[path = "cli/profiles.rs"]
mod profiles;
#[path = "cli/support.rs"]
mod support;
#[path = "cli/swimlanes.rs"]
mod swimlanes;
#[path = "cli/users.rs"]
mod users;

#[path = "cli/attachments.rs"]
mod attachments;
#[path = "cli/bulk_cards.rs"]
mod bulk_cards;
#[path = "cli/custom_fields.rs"]
mod custom_fields;
#[path = "cli/dependencies.rs"]
mod dependencies;
#[path = "cli/integrations.rs"]
mod integrations;
#[path = "cli/memberships.rs"]
mod memberships;
#[path = "cli/transfers.rs"]
mod transfers;

#[path = "cli/rules.rs"]
mod rules;

#[path = "cli/admin_people.rs"]
mod admin_people;
#[path = "cli/admin_settings.rs"]
mod admin_settings;

#[test]
fn item_command_help_exposes_the_nested_lifecycles() {
    use assert_cmd::cargo::cargo_bin_cmd;
    use clap::CommandFactory;
    use predicates::prelude::*;
    use wekan_cli::cli::Cli;

    let mut root = Cli::command();
    root.build();
    let root_help = root.render_help().to_string();
    for (family, old, expected) in [
        (
            "checklist",
            "checklist-item",
            vec!["create", "delete", "get", "update"],
        ),
        (
            "custom-field",
            "custom-field-item",
            vec!["create", "delete", "update"],
        ),
    ] {
        assert!(root.find_subcommand(old).is_none());
        assert!(!root_help.contains(old));
        let parent = root.find_subcommand(family).unwrap();
        let item = parent.find_subcommand("item").unwrap();
        let mut verbs: Vec<_> = item
            .get_subcommands()
            .map(clap::Command::get_name)
            .filter(|name| *name != "help")
            .collect();
        verbs.sort_unstable();
        assert_eq!(verbs, expected);
        cargo_bin_cmd!("wekan")
            .args([family, "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("  item "))
            .stderr(predicate::str::is_empty());
        cargo_bin_cmd!("wekan")
            .args([family, "item", "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains(format!(
                "{family} item [OPTIONS] <COMMAND>"
            )))
            .stderr(predicate::str::is_empty());
    }
}

#[test]
fn old_root_item_spellings_are_invalid_commands() {
    use assert_cmd::cargo::cargo_bin_cmd;
    use clap::Parser;
    use wekan_cli::cli::Cli;

    for old in ["checklist-item", "custom-field-item"] {
        let error = Cli::try_parse_from(["wekan", old, "--help"]).unwrap_err();
        assert_eq!(error.kind(), clap::error::ErrorKind::InvalidSubcommand);
        let output = cargo_bin_cmd!("wekan")
            .args(["--output", "json", old, "--help"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], "invalid_input");
        assert!(error["error"]["message"].as_str().unwrap().contains(old));
    }
}

#[test]
fn nested_item_commands_accept_global_options_at_every_level() {
    use clap::Parser;
    use wekan_cli::{cli::Cli, output::OutputFormat};

    for (family, scope) in [
        ("checklist", vec!["--card", "c", "--checklist", "p"]),
        ("custom-field", vec!["--custom-field", "p"]),
    ] {
        for target in ["--server", "--profile"] {
            for position in 1..=4 {
                let mut argv = vec![
                    "wekan", family, "item", "delete", "i", "--board", "b", "--yes",
                ];
                argv.extend_from_slice(&scope);
                argv.splice(
                    position..position,
                    [
                        target,
                        if target == "--server" {
                            "https://wekan.example"
                        } else {
                            "team"
                        },
                        "--output",
                        "json",
                        "--allow-insecure-http",
                    ],
                );
                let parsed = Cli::try_parse_from(argv).unwrap();
                assert_eq!(parsed.output, OutputFormat::Json);
                assert!(parsed.allow_insecure_http);
                if target == "--server" {
                    assert_eq!(parsed.server.as_deref(), Some("https://wekan.example"));
                } else {
                    assert_eq!(parsed.profile_name.as_deref(), Some("team"));
                }
            }
        }
    }
}

#[path = "cli/exports.rs"]
mod exports;
