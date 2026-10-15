use super::support::{TestDirectory, add_profile, no_target_cli};
use clap::Parser;
use wekan_cli::cli::Cli;

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(["wekan", "admin"].into_iter().chain(args.iter().copied()))
}

#[test]
fn admin_people_parser_enforces_typed_inputs_and_global_options() {
    for kind in ["org", "team"] {
        let selector = format!("--{kind}");
        for value in ["true", "false"] {
            for flag in [
                "--shared-templates",
                "--propagate-members-to-boards",
                "--sync-members-from-auth",
            ] {
                parse(&[kind, "features", "update", &selector, "id", flag, value]).unwrap();
            }
            for field in [
                "shared-templates",
                "propagate-members-to-boards",
                "sync-members-from-auth",
            ] {
                parse(&[
                    kind,
                    "features",
                    "update-all",
                    "--field",
                    field,
                    "--value",
                    value,
                    "--yes",
                ])
                .unwrap();
            }
        }
        for args in [
            vec![kind, "features", "update", &selector, "id"],
            vec![kind, "features", "update", "--shared-templates", "true"],
            vec![
                kind,
                "features",
                "update",
                &selector,
                "id",
                "--shared-templates",
            ],
            vec![
                kind,
                "features",
                "update",
                &selector,
                "id",
                "--shared-templates",
                "yes",
            ],
            vec![
                kind,
                "features",
                "update",
                &selector,
                "id",
                "--shared-templates",
                "true",
                "--shared-templates",
                "false",
            ],
            vec![
                kind,
                "features",
                "update",
                &selector,
                "id",
                "--shared-templates",
                "true",
                "--yes",
            ],
            vec![
                kind,
                "features",
                "update-all",
                "--field",
                "unknown",
                "--value",
                "true",
                "--yes",
            ],
            vec![
                kind,
                "features",
                "update-all",
                "--field",
                "shared-templates",
                "--yes",
            ],
            vec![kind, "features", "update-all", "--value", "true", "--yes"],
            vec![
                kind,
                "features",
                "update-all",
                "--field",
                "shared-templates",
                "--value",
                "yes",
            ],
            vec![
                kind,
                "features",
                "update-all",
                "--field",
                "shared-templates",
                "--value",
                "true",
                &selector,
                "id",
            ],
        ] {
            assert!(parse(&args).is_err(), "{args:?}");
        }
        for id in ["", " ", ".", ".."] {
            assert!(
                parse(&[
                    kind,
                    "features",
                    "update",
                    &selector,
                    id,
                    "--shared-templates",
                    "true"
                ])
                .is_err()
            );
        }
        let cli = parse(&[
            kind,
            "list",
            "--server",
            "http://localhost:3127",
            "--profile",
            "local",
            "--output",
            "json",
            "--allow-insecure-http",
        ])
        .unwrap();
        assert_eq!(cli.profile_name.as_deref(), Some("local"));
        assert!(cli.allow_insecure_http);
    }
    parse(&["domain", "list"]).unwrap();
    assert!(parse(&["domain", "list", "--yes"]).is_err());
}

#[test]
fn help_and_invalid_inputs_work_without_credentials_or_network() {
    let directory = TestDirectory::new("admin-people");
    for args in [
        vec!["domain", "list"],
        vec!["org", "list"],
        vec!["team", "list"],
        vec!["org", "features", "update"],
        vec!["org", "features", "update-all"],
        vec!["team", "features", "update"],
        vec!["team", "features", "update-all"],
    ] {
        no_target_cli(&directory)
            .arg("admin")
            .args(&args)
            .arg("--help")
            .assert()
            .success();
    }
    add_profile(&directory, "default", "http://127.0.0.1:1");
    for kind in ["org", "team"] {
        let output = no_target_cli(&directory)
            .args([
                "--output=json",
                "admin",
                kind,
                "features",
                "update",
                &format!("--{kind}"),
                "id",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let envelope: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(envelope["error"]["code"], "invalid_input");
        no_target_cli(&directory)
            .args(["admin", kind, "features", "update-all", "--help"])
            .assert()
            .success()
            .stdout(predicates::str::contains("--yes"))
            .stdout(predicates::str::contains("sync-members-from-auth"));
    }
}
