use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

use super::support::{TestDirectory, no_target_cli};

#[test]
fn api_help_documents_the_complete_raw_request_surface() {
    cargo_bin_cmd!("wekan")
        .args(["api", "request", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<METHOD> <PATH>"))
        .stdout(predicate::str::contains("--query"))
        .stdout(predicate::str::contains("--header"))
        .stdout(predicate::str::contains("--body-file"))
        .stdout(predicate::str::contains("--body-stdin"))
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("--no-auth"))
        .stdout(predicate::str::contains("Do not load or inject"))
        .stdout(predicate::str::contains("--auth-token-query"))
        .stdout(predicate::str::contains("--timeout"))
        .stdout(predicate::str::contains("--yes"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn raw_api_rejects_reflection_and_tunneling_methods_during_parsing() {
    for method in ["TRACE", "track", "CONNECT"] {
        cargo_bin_cmd!("wekan")
            .args(["api", "request", method, "/api/items"])
            .assert()
            .code(2)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("not permitted"));
    }
}

#[test]
fn raw_output_is_scoped_to_api_requests_before_target_resolution() {
    let directory = TestDirectory::new("raw-output-scope");
    no_target_cli(&directory)
        .args(["--output=raw", "board", "list"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("invalid_input"))
        .stderr(predicate::str::contains("api request"));
}

#[test]
fn api_parser_rejects_conflicting_body_and_auth_sources() {
    for arguments in [
        vec![
            "api",
            "request",
            "POST",
            "/api/items",
            "--body",
            "x",
            "--json",
            "{}",
        ],
        vec![
            "api",
            "request",
            "GET",
            "/api/items",
            "--no-auth",
            "--auth-token-query",
        ],
    ] {
        cargo_bin_cmd!("wekan")
            .args(arguments)
            .assert()
            .code(2)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("cannot be used with"));
    }
}
