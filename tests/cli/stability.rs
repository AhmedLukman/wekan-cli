use clap::Command;
use serde_json::{Value, json};

use crate::support::wekan;

fn commands(command: &Command, path: Vec<String>, result: &mut Vec<Vec<String>>) {
    result.push(path.clone());
    for child in command.get_subcommands().filter(|c| c.get_name() != "help") {
        let mut child_path = path.clone();
        child_path.push(child.get_name().to_owned());
        commands(child, child_path, result);
    }
}

fn command_paths() -> Vec<Vec<String>> {
    let mut root = wekan_cli::command();
    root.build();
    let mut result = Vec::new();
    commands(&root, Vec::new(), &mut result);
    result
}

#[test]
fn every_command_has_text_help_and_structured_usage_errors() {
    for path in command_paths() {
        let label = format!("wekan {}", path.join(" "));
        let help = wekan()
            .args(["--output=json"])
            .args(&path)
            .arg("--help")
            .output()
            .unwrap();
        assert!(help.status.success(), "{label}: {help:?}");
        assert!(help.stderr.is_empty(), "{label}: {help:?}");
        let text = std::str::from_utf8(&help.stdout).unwrap();
        assert!(text.contains("Usage: wekan"), "{label}: {text}");
        for global in ["--output", "--server", "--profile", "--allow-insecure-http"] {
            assert!(text.contains(global), "{label}: missing {global}");
        }

        // Run the actual executable at every depth, including groups. These
        // must fail before configuration, credentials, stdin, or HTTP access.
        for nested_output in [false, true] {
            let mut command = wekan();
            if !nested_output {
                command.arg("--output=json");
            }
            command.args(&path).arg("--unknown-stability-test-option");
            if nested_output {
                command.args(["--output", "json"]);
            }
            let output = command.output().unwrap();
            assert_eq!(output.status.code(), Some(2), "{label}: {output:?}");
            assert!(output.stdout.is_empty(), "{label}: {output:?}");
            assert!(output.stderr.ends_with(b"\n"), "{label}: {output:?}");
            let envelope: Value = serde_json::from_slice(&output.stderr)
                .unwrap_or_else(|error| panic!("{label}: {error}: {output:?}"));
            assert_eq!(envelope["ok"], false, "{label}");
            assert_eq!(envelope["error"]["code"], "invalid_input", "{label}");
            assert!(envelope["error"]["message"].is_string(), "{label}");
            assert_eq!(envelope["error"]["details"], json!({}), "{label}");
            assert!(envelope.get("data").is_none(), "{label}");
        }
    }
}

#[test]
fn stability_inventory_covers_every_leaf_command() {
    let actual: std::collections::BTreeSet<_> = command_paths()
        .into_iter()
        .filter(|path| {
            let mut root = wekan_cli::command();
            root.build();
            let mut command = &root;
            for segment in path {
                command = command.find_subcommand(segment).unwrap();
            }
            !command.has_subcommands()
        })
        .map(|path| format!("wekan {}", path.join(" ")))
        .collect();
    let documented: std::collections::BTreeSet<_> = include_str!("../../docs/command-contract.md")
        .lines()
        .filter_map(|line| line.strip_prefix("| `wekan "))
        .map(|line| format!("wekan {}", line.split('`').next().unwrap()))
        .collect();
    assert_eq!(
        actual, documented,
        "update the complete stability inventory"
    );
}
