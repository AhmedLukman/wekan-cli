use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn help_is_text_on_stdout() {
    cargo_bin_cmd!("wekan")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: wekan"))
        .stdout(predicate::str::contains("auth"))
        .stderr(predicate::str::is_empty());
}

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
fn missing_status_server_is_a_json_configuration_error_before_vault_access() {
    cargo_bin_cmd!("wekan")
        .env_remove("WEKAN_URL")
        .args(["--output=json", "auth", "status"])
        .assert()
        .code(3)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"configuration_error""#));
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
fn missing_login_server_is_a_json_configuration_error_before_stdin_is_read() {
    cargo_bin_cmd!("wekan")
        .env_remove("WEKAN_URL")
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
        .stderr(predicate::str::contains(r#""code":"configuration_error""#));
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
fn missing_server_is_a_json_configuration_error() {
    cargo_bin_cmd!("wekan")
        .env_remove("WEKAN_URL")
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
        .stderr(predicate::str::contains(r#""code":"configuration_error""#));
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
    cargo_bin_cmd!("wekan")
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
