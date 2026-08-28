use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

struct TestDirectory(std::path::PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "wekan-cli-black-box-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn no_target_cli(directory: &TestDirectory) -> assert_cmd::Command {
    let mut command = cargo_bin_cmd!("wekan");
    command
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .env_remove("WEKAN_URL")
        .env_remove("WEKAN_PROFILE");
    command
}

fn add_profile(directory: &TestDirectory, name: &str, server: &str) {
    cargo_bin_cmd!("wekan")
        .env("WEKAN_CONFIG_DIR", &directory.0)
        .args(["profile", "add", name, server])
        .assert()
        .success();
}

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
fn board_help_lists_the_core_lifecycle() {
    cargo_bin_cmd!("wekan")
        .args(["board", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("count"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("rename"))
        .stdout(predicate::str::contains("delete"))
        .stderr(predicate::str::is_empty());

    cargo_bin_cmd!("wekan")
        .args(["board", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--title <TITLE>"))
        .stdout(predicate::str::contains("--owner <OWNER>"))
        .stdout(predicate::str::contains("--permission <PERMISSION>"))
        .stdout(predicate::str::contains("--color <COLOR>"))
        .stdout(predicate::str::contains("--no-comments"))
        .stdout(predicate::str::contains("--comment-only"))
        .stdout(predicate::str::contains("--worker"));
}

#[test]
fn only_board_delete_accepts_yes() {
    cargo_bin_cmd!("wekan")
        .args(["board", "delete", "board-1", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--yes"));

    cargo_bin_cmd!("wekan")
        .args(["board", "get", "board-1", "--yes"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unexpected argument '--yes'"));
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
fn yes_is_command_local_to_destructive_commands() {
    for arguments in [
        vec!["--yes", "auth", "status"],
        vec![
            "--server",
            "https://wekan.example",
            "auth",
            "login",
            "--username",
            "alice",
            "--yes",
        ],
        vec![
            "--server",
            "https://wekan.example",
            "auth",
            "register",
            "--username",
            "alice",
            "--yes",
        ],
        vec![
            "--server",
            "https://wekan.example",
            "auth",
            "status",
            "--yes",
        ],
        vec!["profile", "add", "work", "https://wekan.example", "--yes"],
        vec!["profile", "list", "--yes"],
        vec!["profile", "show", "work", "--yes"],
        vec!["profile", "use", "work", "--yes"],
        vec![
            "profile",
            "update",
            "work",
            "https://wekan.example",
            "--yes",
        ],
    ] {
        cargo_bin_cmd!("wekan")
            .args(arguments)
            .assert()
            .code(2)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("unexpected argument '--yes'"));
    }

    cargo_bin_cmd!("wekan")
        .args(["profile", "remove", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("--yes"));
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
