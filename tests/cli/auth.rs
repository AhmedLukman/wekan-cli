use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

use super::support::{TestDirectory, add_profile, no_target_cli};

#[test]
fn login_help_documents_secret_input_modes() {
    cargo_bin_cmd!("wekan")
        .args(["auth", "login", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--username <USERNAME>"))
        .stdout(predicate::str::contains("--email <EMAIL>"))
        .stdout(predicate::str::contains("--password-stdin"))
        .stdout(predicate::str::contains("--code-stdin"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn auth_help_lists_the_status_command() {
    cargo_bin_cmd!("wekan")
        .args(["auth", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains(
            "Validate and display the stored authentication session",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn auth_help_lists_logout_and_logout_help_documents_scopes() {
    cargo_bin_cmd!("wekan")
        .args(["auth", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("logout"))
        .stdout(predicate::str::contains(
            "Revoke Wekan login tokens and remove the stored credential",
        ))
        .stderr(predicate::str::is_empty());

    cargo_bin_cmd!("wekan")
        .args(["auth", "logout", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--all"))
        .stdout(predicate::str::contains("--local-only"))
        .stdout(predicate::str::contains("--yes"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn logout_scopes_conflict_using_the_json_parse_error_contract() {
    cargo_bin_cmd!("wekan")
        .args([
            "--output=json",
            "--server",
            "https://wekan.example",
            "auth",
            "logout",
            "--all",
            "--local-only",
        ])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"invalid_input""#))
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn missing_local_only_profile_is_reported_before_vault_access() {
    let directory = TestDirectory::new("missing-local-only-server");
    no_target_cli(&directory)
        .args(["--output=json", "auth", "logout", "--local-only"])
        .assert()
        .code(3)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"profile_not_found""#))
        .stderr(predicate::str::contains(r#""profile":"default""#));
}

#[test]
fn missing_profile_local_only_with_server_reaches_confirmation_without_creating_metadata() {
    let directory = TestDirectory::new("orphaned-local-only-recovery");
    no_target_cli(&directory)
        .args([
            "--output=json",
            "--server",
            "https://wekan.example",
            "--profile",
            "orphaned",
            "auth",
            "logout",
            "--local-only",
        ])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"invalid_input""#))
        .stderr(predicate::str::contains("--yes"));
    assert!(!directory.0.join("profiles.json").exists());
}

#[test]
fn json_logout_requires_yes_before_vault_access() {
    let directory = TestDirectory::new("json-logout-confirmation");
    add_profile(&directory, "default", "https://wekan.example");
    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args(["--output=json", "auth", "logout", "--local-only"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"invalid_input""#))
        .stderr(predicate::str::contains("--yes"));
}

#[test]
fn non_terminal_human_logout_requires_yes_before_vault_access() {
    let directory = TestDirectory::new("human-logout-confirmation");
    add_profile(&directory, "default", "https://wekan.example");
    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args(["auth", "logout", "--local-only"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Error [invalid_input]"))
        .stderr(predicate::str::contains("--yes"));
}

#[test]
fn status_rejects_command_specific_arguments() {
    cargo_bin_cmd!("wekan")
        .args(["--output=json", "auth", "status", "unexpected"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"invalid_input""#))
        .stderr(predicate::str::contains("unexpected"));
}

#[test]
fn missing_status_profile_is_reported_before_vault_access() {
    let directory = TestDirectory::new("missing-status-server");
    no_target_cli(&directory)
        .args(["--output=json", "auth", "status"])
        .assert()
        .code(3)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"profile_not_found""#))
        .stderr(predicate::str::contains(r#""profile":"default""#));
}

#[test]
fn login_identity_conflicts_use_the_json_parse_error_contract() {
    cargo_bin_cmd!("wekan")
        .args([
            "--output=json",
            "auth",
            "login",
            "--username",
            "alice",
            "--email",
            "alice@example.com",
        ])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"invalid_input""#));
}

#[test]
fn login_code_stdin_requires_password_stdin() {
    cargo_bin_cmd!("wekan")
        .args([
            "--output=json",
            "auth",
            "login",
            "--username",
            "alice",
            "--code-stdin",
        ])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"invalid_input""#))
        .stderr(predicate::str::contains("--password-stdin"));
}

#[test]
fn missing_login_profile_without_a_server_is_reported_before_stdin_is_read() {
    let directory = TestDirectory::new("missing-login-server");
    no_target_cli(&directory)
        .args([
            "--output=json",
            "auth",
            "login",
            "--username",
            "alice",
            "--password-stdin",
        ])
        .assert()
        .code(3)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"profile_not_found""#))
        .stderr(predicate::str::contains(r#""profile":"default""#));
}

#[test]
fn json_parse_errors_use_stderr_and_exit_two() {
    cargo_bin_cmd!("wekan")
        .args([
            "--output",
            "json",
            "--server",
            "https://wekan.example",
            "auth",
            "register",
            "--password-stdin",
        ])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"invalid_input""#));
}

#[test]
fn missing_registration_profile_without_a_server_is_reported() {
    let directory = TestDirectory::new("missing-registration-server");
    no_target_cli(&directory)
        .args([
            "--output=json",
            "auth",
            "register",
            "--username",
            "alice",
            "--password-stdin",
        ])
        .write_stdin("password\n")
        .assert()
        .code(3)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"profile_not_found""#))
        .stderr(predicate::str::contains(r#""profile":"default""#));
}

#[test]
fn command_line_server_overrides_the_environment() {
    cargo_bin_cmd!("wekan")
        .env("WEKAN_URL", "https://from-environment.example")
        .args([
            "--output=json",
            "--server",
            "not a url",
            "auth",
            "register",
            "--username",
            "alice",
            "--password-stdin",
        ])
        .write_stdin("password\n")
        .assert()
        .code(3)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"configuration_error""#));
}

#[test]
fn environment_supplies_the_server_when_the_flag_is_absent() {
    cargo_bin_cmd!("wekan")
        .env("WEKAN_URL", "not a url")
        .args([
            "--output=json",
            "auth",
            "register",
            "--username",
            "alice",
            "--password-stdin",
        ])
        .write_stdin("password\n")
        .assert()
        .code(3)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("relative URL without a base"));
}

#[test]
fn remote_plaintext_http_requires_explicit_opt_in() {
    let directory = TestDirectory::new("remote-plaintext-http");
    no_target_cli(&directory)
        .args([
            "--output=json",
            "--server",
            "http://wekan.example",
            "auth",
            "register",
            "--username",
            "alice",
            "--password-stdin",
        ])
        .write_stdin("password\n")
        .assert()
        .code(3)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"insecure_transport""#));
}
