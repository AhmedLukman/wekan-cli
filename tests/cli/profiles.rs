use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

use super::support::{TestDirectory, no_target_cli};

#[test]
fn profile_help_lists_the_complete_command_family() {
    cargo_bin_cmd!("wekan")
        .args(["profile", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("show"))
        .stdout(predicate::str::contains("use"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("remove"));
}

#[test]
fn profile_commands_persist_and_render_the_local_lifecycle() {
    let directory = TestDirectory::new("lifecycle");
    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .env("WEKAN_URL", "ignored")
        .env("WEKAN_PROFILE", "INVALID AND IGNORED")
        .args([
            "--output=json",
            "profile",
            "add",
            "work",
            "https://wekan.example",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains(r#""name":"work""#))
        .stdout(predicate::str::contains(
            r#""server":"https://wekan.example/""#,
        ))
        .stdout(predicate::str::contains(r#""active":true"#));

    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args([
            "--output=json",
            "profile",
            "add",
            "local",
            "https://wekan.example/",
            "--use",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""active":true"#));

    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args(["--output=json", "profile", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""active_profile":"local""#))
        .stdout(predicate::str::contains(r#""profiles":[{"name":"local""#));

    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args(["--output=json", "profile", "show", "work"])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""active":false"#));

    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args([
            "--output=json",
            "profile",
            "update",
            "work",
            "https://wekan.example",
        ])
        .assert()
        .success();

    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args(["--output=json", "profile", "remove", "local"])
        .assert()
        .code(3)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"profile_in_use""#));

    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args(["--output=json", "profile", "remove", "work"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(r#""code":"invalid_input""#))
        .stderr(predicate::str::contains("--yes"));

    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args(["--output=json", "profile", "remove", "work", "--yes"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains(r#""removed":true"#));
}

#[test]
fn profile_names_and_explicit_selectors_use_cli_validation() {
    let directory = TestDirectory::new("validation");
    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args([
            "--output=json",
            "profile",
            "add",
            "Not-Portable",
            "https://wekan.example",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(r#""code":"invalid_input""#));

    no_target_cli(&directory)
        .args([
            "--output=json",
            "--server",
            "https://wekan.example",
            "--profile",
            "work",
            "auth",
            "status",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(r#""code":"profile_not_found""#))
        .stderr(predicate::str::contains(r#""profile":"work""#));

    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args(["--output=json", "--profile", "work", "profile", "list"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "cannot be used with profile-management",
        ));
}

#[test]
fn profile_selection_separates_server_identity_from_network_permission() {
    let directory = TestDirectory::new("precedence");
    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args([
            "--output=json",
            "profile",
            "add",
            "refused",
            "http://wekan.example",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("http://wekan.example/"));

    for (name, url, activate) in [
        ("active", "http://localhost:3000", false),
        ("insecure", "http://wekan.example", true),
    ] {
        let mut command = cargo_bin_cmd!("wekan");
        command
            .env("WEKAN_CONFIG_DIR", &directory.0)
            .args(["profile", "add", name, url]);
        if activate {
            command.arg("--use");
        }
        command.assert().success();
    }

    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .env("WEKAN_URL", "http://wekan.example")
        .args(["--output=json", "--profile", "insecure", "auth", "status"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(r#""code":"insecure_transport""#))
        .stderr(predicate::str::contains(r#""profile":"insecure""#));

    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args([
            "--output=json",
            "--profile",
            "insecure",
            "--server",
            "http://other.example",
            "auth",
            "status",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            r#""code":"profile_server_mismatch""#,
        ))
        .stderr(predicate::str::contains(r#""profile":"insecure""#));

    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .env("WEKAN_URL", "not a url")
        .env("WEKAN_PROFILE", "missing")
        .args(["--output=json", "auth", "status"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("relative URL without a base"))
        .stderr(predicate::str::contains(r#""profile":"missing""#));

    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .env_remove("WEKAN_URL")
        .env("WEKAN_PROFILE", "missing")
        .args(["--output=json", "auth", "status"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(r#""code":"profile_not_found""#))
        .stderr(predicate::str::contains(r#""profile":"missing""#));

    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .env_remove("WEKAN_URL")
        .env_remove("WEKAN_PROFILE")
        .args(["--output=json", "auth", "status"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(r#""code":"insecure_transport""#))
        .stderr(predicate::str::contains(r#""profile":"insecure""#));
}
