use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use clap::{Parser, error::ErrorKind};
use serde_json::{Value, json};

use crate::support::wekan;

fn json_success(args: &[&str]) -> Value {
    let output = wekan()
        .arg("--output=json")
        .args(args)
        .assert()
        .success()
        .get_output()
        .clone();
    assert!(output.stderr.is_empty());
    assert!(output.stdout.ends_with(b"\n"));
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope.as_object().unwrap().len(), 2);
    envelope["data"].clone()
}

#[test]
fn default_and_explicit_core_print_markdown_and_the_normal_json_envelope() {
    let core = include_str!("../../docs/agent-guide.md");
    for args in [vec!["skill", "show"], vec!["skill", "show", "core"]] {
        assert_eq!(
            json_success(&args),
            json!({"cli_version": env!("CARGO_PKG_VERSION"), "topic": "core", "content": core})
        );
        let output = wekan().args(args).assert().success().get_output().clone();
        assert!(output.stderr.is_empty());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("{}\n", core.trim_end_matches(['\r', '\n']))
        );
    }
}

#[test]
fn every_listed_topic_is_unique_sorted_and_matches_its_maintained_source() {
    let data = json_success(&["skill", "list"]);
    assert_eq!(data.as_object().unwrap().len(), 2);
    assert_eq!(data["cli_version"], env!("CARGO_PKG_VERSION"));
    let topics = data["topics"].as_array().unwrap();
    let mut previous = "";
    let mut names = BTreeSet::new();
    let mut guides = BTreeMap::new();
    for topic in topics {
        let name = topic["name"].as_str().unwrap();
        assert!(name > previous, "topics must be sorted without duplicates");
        previous = name;
        assert_eq!(topic.as_object().unwrap().len(), 2);
        assert!(!topic["description"].as_str().unwrap().is_empty());
        names.insert(name);
        let filename = if name == "core" { "agent-guide" } else { name };
        let source = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("docs/{filename}.md")),
        )
        .unwrap();
        assert_eq!(
            json_success(&["skill", "show", name]),
            json!({"cli_version": env!("CARGO_PKG_VERSION"), "topic": name, "content": source})
        );
        guides.insert(format!("{filename}.md"), source);
    }
    for excluded in [
        "api-coverage",
        "architecture",
        "auth-architecture",
        "command-contract",
        "folder-structure",
        "interface-audit",
        "local-wekan",
        "skills",
    ] {
        assert!(!names.contains(excluded));
    }
    // Every local document linked from embedded guidance must be retrievable
    // without a checkout. Follow destinations, independent of link labels.
    for (source_name, content) in &guides {
        for link in content.split("](").skip(1) {
            let destination = link.split_once(')').unwrap().0;
            let filename = destination.split('#').next().unwrap();
            if !filename.starts_with("https://")
                && !filename.starts_with("http://")
                && filename.ends_with(".md")
            {
                assert!(
                    guides.contains_key(filename),
                    "unavailable guide document in {source_name}: {filename}"
                );
            }
        }
    }
    let output = wekan()
        .args(["skill", "list"])
        .assert()
        .success()
        .get_output()
        .clone();
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert_eq!(text.lines().count(), topics.len() + 1);
    for (row, topic) in text.lines().skip(1).zip(topics) {
        assert_eq!(row.split_whitespace().next(), topic["name"].as_str());
    }
}

#[test]
fn unknown_and_path_like_topics_fail_with_local_structured_usage_errors() {
    for topic in [
        "missing-topic",
        "command-contract",
        "skills",
        "Core",
        "../README.md",
        "cards.md",
        "",
        "--output=json",
    ] {
        let output = wekan()
            .args(["skill", "show", "--output=json", "--", topic])
            .assert()
            .code(2)
            .get_output()
            .clone();
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["ok"], false);
        assert_eq!(error["error"]["code"], "invalid_input");
        assert_eq!(error["error"]["details"], json!({}));
        assert!(
            error["error"]["message"]
                .as_str()
                .unwrap()
                .contains("wekan skill list")
        );
    }
}

#[test]
fn guides_ignore_unusable_configuration_targets_and_work_outside_the_checkout() {
    let unrelated = tempfile::tempdir().unwrap();
    for args in [
        vec!["skill", "list"],
        vec!["skill", "show"],
        vec!["skill", "show", "auth"],
    ] {
        let output = wekan()
            .current_dir(unrelated.path())
            .env("WEKAN_CONFIG_DIR", "relative-and-invalid")
            .env("WEKAN_PROFILE", "INVALID/PROFILE")
            .env("WEKAN_URL", "not a URL")
            .args([
                "--server",
                "also-not-a-url",
                "--profile",
                "missing",
                "--allow-insecure-http",
                "--output=json",
            ])
            .args(args)
            .assert()
            .success()
            .get_output()
            .clone();
        assert!(output.stderr.is_empty());
        let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(envelope["ok"], true);
    }
    assert_eq!(fs::read_dir(unrelated.path()).unwrap().count(), 0);
}

#[test]
fn raw_guide_output_is_rejected_before_target_resolution() {
    for args in [
        vec!["skill", "list"],
        vec!["skill", "show"],
        vec!["skill", "show", "core"],
    ] {
        let output = wekan()
            .env("WEKAN_CONFIG_DIR", "relative-and-invalid")
            .args(["--output=raw"])
            .args(args)
            .assert()
            .code(2)
            .get_output()
            .clone();
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.starts_with("Error [invalid_input]:"));
        assert!(error.contains("--output raw requires"));
    }
}

#[test]
fn discovery_skill_metadata_is_valid_and_examples_match_the_parser() {
    let skill = include_str!("../../skills/wekan-cli/SKILL.md");
    let mut lines = skill.lines();
    assert_eq!(lines.next(), Some("---"));
    let header = lines
        .by_ref()
        .take_while(|line| *line != "---")
        .collect::<Vec<_>>()
        .join("\n");
    let metadata: serde_yaml_ng::Value = serde_yaml_ng::from_str(&header).unwrap();
    assert_eq!(metadata["name"], "wekan-cli");
    assert_eq!(metadata["license"], "Apache-2.0");
    let description = metadata["description"].as_str().unwrap();
    assert!(!description.is_empty() && description.len() <= 1024);
    for document in [skill, include_str!("../../docs/agent-guide.md")] {
        let mut console = false;
        for line in document.lines() {
            if line.starts_with("```") {
                console = line == "```console";
            } else if console && !line.is_empty() {
                // These portable examples intentionally need no shell expansion
                // or quoting. Parse them only; never execute sample mutations.
                if let Err(error) = wekan_cli::cli::Cli::try_parse_from(line.split_whitespace()) {
                    assert!(
                        matches!(
                            error.kind(),
                            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
                        ),
                        "invalid guide example `{line}`: {error}"
                    );
                }
            }
        }
    }
}

struct UnusedDependencies;

impl wekan_cli::config::profiles::ProfileStore for UnusedDependencies {
    fn read(
        &self,
    ) -> Result<
        wekan_cli::config::profiles::ProfileSnapshot,
        wekan_cli::config::profiles::ProfileStoreError,
    > {
        panic!("guide commands must not read profiles")
    }
    fn lock_mutation(
        &self,
    ) -> Result<
        Box<dyn wekan_cli::config::profiles::ProfileMutation + Send + '_>,
        wekan_cli::config::profiles::ProfileStoreError,
    > {
        panic!("guide commands must not mutate profiles")
    }
    fn credential_namespace(
        &self,
    ) -> Result<String, wekan_cli::config::profiles::ProfileStoreError> {
        panic!("guide commands must not resolve credential targets")
    }
}

impl wekan_cli::credentials::CredentialStore for UnusedDependencies {
    fn check_available(
        &self,
        _: &wekan_cli::credentials::CredentialTarget,
    ) -> Result<(), wekan_cli::credentials::CredentialError> {
        panic!("guide commands must not check credentials")
    }
    fn exists(
        &self,
        _: &wekan_cli::credentials::CredentialTarget,
    ) -> Result<bool, wekan_cli::credentials::CredentialError> {
        panic!("guide commands must not inspect credentials")
    }
    fn load(
        &self,
        _: &wekan_cli::credentials::CredentialTarget,
    ) -> Result<
        Option<wekan_cli::credentials::CredentialRecord>,
        wekan_cli::credentials::CredentialError,
    > {
        panic!("guide commands must not load credentials")
    }
    fn create(
        &self,
        _: &wekan_cli::credentials::CredentialTarget,
        _: &wekan_cli::credentials::CredentialRecord,
    ) -> Result<
        wekan_cli::credentials::CredentialCreateOutcome,
        wekan_cli::credentials::CredentialError,
    > {
        panic!("guide commands must not save credentials")
    }
    fn delete(
        &self,
        _: &wekan_cli::credentials::CredentialTarget,
    ) -> Result<bool, wekan_cli::credentials::CredentialError> {
        panic!("guide commands must not delete credentials")
    }
    fn delete_if_matches(
        &self,
        _: &wekan_cli::credentials::CredentialTarget,
        _: &wekan_cli::credentials::CredentialRecord,
    ) -> Result<
        wekan_cli::credentials::CredentialDeleteOutcome,
        wekan_cli::credentials::CredentialError,
    > {
        panic!("guide commands must not delete credentials")
    }
    fn lock_mutation(
        &self,
        _: &wekan_cli::credentials::CredentialTarget,
    ) -> Result<
        Box<dyn wekan_cli::credentials::CredentialMutation + Send + '_>,
        wekan_cli::credentials::CredentialError,
    > {
        panic!("guide commands must not lock credentials")
    }
}

impl wekan_cli::credentials::SecretInputProvider for UnusedDependencies {
    fn read_storage_secret(
        &self,
        _: wekan_cli::credentials::SecretSource,
        _: usize,
    ) -> wekan_cli::credentials::SecretInputFuture<'_> {
        panic!("guide commands must not read storage secrets")
    }
    fn read_raw_secret(&self) -> Result<secrecy::SecretString, wekan_cli::error::AppError> {
        panic!("guide commands must not read stdin")
    }
    fn read_new_account_password(
        &self,
        _: bool,
    ) -> Result<secrecy::SecretString, wekan_cli::error::AppError> {
        panic!("guide commands must not request a password")
    }
    fn read_login_secrets(
        &self,
        _: wekan_cli::credentials::LoginSecretMode,
    ) -> Result<wekan_cli::credentials::LoginSecrets, wekan_cli::error::AppError> {
        panic!("guide commands must not request login secrets")
    }
}

impl wekan_cli::input::ConfirmationProvider for UnusedDependencies {
    fn confirm(
        &self,
        _: &wekan_cli::input::ConfirmationRequest,
    ) -> Result<bool, wekan_cli::error::AppError> {
        panic!("guide commands must not prompt")
    }
}

#[tokio::test]
async fn local_dispatch_never_touches_injected_external_dependencies_or_http() {
    let server = wiremock::MockServer::start().await;
    let app = wekan_cli::app::App::with_dependencies(
        wekan_cli::config::ServerSelection::new(
            Some(server.uri()),
            Some("missing".to_owned()),
            false,
        ),
        UnusedDependencies,
        UnusedDependencies,
        UnusedDependencies,
        UnusedDependencies,
    );
    for args in [
        vec!["wekan", "skill", "list"],
        vec!["wekan", "skill", "show"],
        vec!["wekan", "skill", "show", "cards"],
        vec!["wekan", "skill", "show", "missing-topic"],
    ] {
        let expected_error = args.last() == Some(&"missing-topic");
        let cli = wekan_cli::cli::Cli::try_parse_from(args).unwrap();
        let result = app.execute(cli.command).await;
        if expected_error {
            assert_eq!(
                result.unwrap_err().code(),
                wekan_cli::error::ErrorCode::InvalidInput
            );
        } else {
            assert!(result.is_ok());
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}
