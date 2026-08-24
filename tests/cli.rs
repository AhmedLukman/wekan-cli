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
