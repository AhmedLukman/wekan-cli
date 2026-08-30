pub(crate) use std::{
    env, fs,
    io::Write,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) use clap::Parser;
pub(crate) use reqwest::header::AUTHORIZATION;
pub(crate) use secrecy::{ExposeSecret, SecretString};
pub(crate) use time::{Duration, OffsetDateTime};
pub(crate) use wekan_cli::{
    app::App,
    cli::Cli,
    client::ServerUrl,
    command_result::{
        CardDeleteMode, CommandSuccess, CommentDeleteMode, ListDeleteMode, LogoutScope,
        SwimlaneDeleteMode,
    },
    config::{
        ServerSelection,
        profiles::{FileProfileStore, Profile, ProfileDocument, ProfileStore},
    },
    credentials::{
        CredentialCreateOutcome, CredentialDeleteOutcome, CredentialError, CredentialRecord,
        CredentialStore, CredentialTarget, KeyringCredentialStore, LoginSecretMode, LoginSecrets,
        SecretInputProvider,
    },
    error::{AppError, ErrorCode},
    output::{OutputFormat, write_success},
};

pub(crate) type CapturedCredential = (String, String, String, String, OffsetDateTime);

pub(crate) struct TestDirectory(PathBuf);

impl TestDirectory {
    pub(crate) fn new(label: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let path = env::temp_dir().join(format!(
            "wekan-cli-e2e-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("the E2E profile directory must be created");
        Self(path)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Default)]
pub(crate) struct CapturingStore {
    pub(crate) credential: Mutex<Option<CapturedCredential>>,
}

impl CredentialStore for CapturingStore {
    fn check_available(&self, _target: &CredentialTarget) -> Result<(), CredentialError> {
        Ok(())
    }

    fn exists(&self, target: &CredentialTarget) -> Result<bool, CredentialError> {
        Ok(self
            .credential
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|(stored_account, ..)| stored_account == target.account()))
    }

    fn load(&self, target: &CredentialTarget) -> Result<Option<CredentialRecord>, CredentialError> {
        Ok(self
            .credential
            .lock()
            .unwrap()
            .as_ref()
            .filter(|(stored_account, ..)| stored_account == target.account())
            .map(|(_, server, user_id, token, token_expires)| {
                CredentialRecord::new(
                    server.clone(),
                    user_id.clone(),
                    SecretString::from(token.clone()),
                    *token_expires,
                )
            }))
    }

    fn create(
        &self,
        target: &CredentialTarget,
        record: &CredentialRecord,
    ) -> Result<CredentialCreateOutcome, CredentialError> {
        let mut credential = self.credential.lock().unwrap();
        if credential
            .as_ref()
            .is_some_and(|(stored_account, ..)| stored_account == target.account())
        {
            return Ok(CredentialCreateOutcome::AlreadyExists);
        }
        *credential = Some((
            target.account().to_owned(),
            record.server_url().to_owned(),
            record.user_id().to_owned(),
            record.token().expose_secret().to_owned(),
            record.token_expires(),
        ));
        Ok(CredentialCreateOutcome::Created)
    }

    fn delete(&self, target: &CredentialTarget) -> Result<bool, CredentialError> {
        let mut credential = self.credential.lock().unwrap();
        if credential
            .as_ref()
            .is_some_and(|(stored_account, ..)| stored_account == target.account())
        {
            credential.take();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn delete_if_matches(
        &self,
        target: &CredentialTarget,
        expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        let mut credential = self.credential.lock().unwrap();
        let Some((account, server, user_id, token, token_expires)) = credential.as_ref() else {
            return Ok(CredentialDeleteOutcome::Absent);
        };
        if account != target.account() {
            return Ok(CredentialDeleteOutcome::Absent);
        }
        let current = CredentialRecord::new(
            server.clone(),
            user_id.clone(),
            SecretString::from(token.clone()),
            *token_expires,
        );
        if !current.matches(expected) {
            return Ok(CredentialDeleteOutcome::Mismatch);
        }

        credential.take();
        Ok(CredentialDeleteOutcome::Removed)
    }
}

pub(crate) struct FixedPassword(pub(crate) SecretString);

impl SecretInputProvider for FixedPassword {
    fn read_new_account_password(&self, _from_stdin: bool) -> Result<SecretString, AppError> {
        Ok(self.0.clone())
    }

    fn read_login_secrets(&self, _mode: LoginSecretMode) -> Result<LoginSecrets, AppError> {
        Ok(LoginSecrets::new(self.0.clone(), None))
    }
}

pub(crate) async fn execute_live_api_output(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
    format: OutputFormat,
) -> Vec<u8> {
    let (exit, stdout, stderr) = execute_live_api_result(app, server, arguments, format).await;
    assert_eq!(
        exit,
        std::process::ExitCode::SUCCESS,
        "{}",
        String::from_utf8_lossy(&stderr)
    );
    assert!(stderr.is_empty());
    stdout
}

pub(crate) async fn execute_live_api_result(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
    format: OutputFormat,
) -> (std::process::ExitCode, Vec<u8>, Vec<u8>) {
    let mut argv = vec!["wekan", "--server", server, "api"];
    argv.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(argv).expect("the live raw API command must parse");
    let result = app
        .execute(cli.command)
        .await
        .expect("the live raw API command must reach Wekan");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = write_success(format, result, &mut stdout, &mut stderr)
        .await
        .expect("the live raw API response must render");
    (exit, stdout, stderr)
}

pub(crate) async fn wait_for_live_raw_checklist(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    checklist_path: &str,
    checklist_id: &str,
    title: &str,
) {
    for _ in 0..40 {
        let output = execute_live_api_output(
            app,
            server,
            &["request", "GET", checklist_path],
            OutputFormat::Json,
        )
        .await;
        let value: serde_json::Value =
            serde_json::from_slice(&output).expect("the checklist collection must be JSON");
        if value["data"]["body"]["value"]
            .as_array()
            .is_some_and(|checklists| {
                checklists.iter().any(|checklist| {
                    checklist["_id"].as_str() == Some(checklist_id)
                        && checklist["title"].as_str() == Some(title)
                })
            })
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("checklist `{checklist_id}` was not observed within 10 seconds")
}

pub(crate) async fn wait_for_live_raw_checklist_absent(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    checklist_path: &str,
    checklist_id: &str,
) {
    for _ in 0..40 {
        let output = execute_live_api_output(
            app,
            server,
            &["request", "GET", checklist_path],
            OutputFormat::Json,
        )
        .await;
        let value: serde_json::Value =
            serde_json::from_slice(&output).expect("the checklist collection must be JSON");
        if value["data"]["body"]["value"]
            .as_array()
            .is_some_and(|checklists| {
                checklists
                    .iter()
                    .all(|checklist| checklist["_id"].as_str() != Some(checklist_id))
            })
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("deleted checklist `{checklist_id}` remained visible for 10 seconds")
}

pub(crate) async fn execute_live_user(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output=json", "user"];
    argv.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(argv).expect("the live user command must parse");
    app.execute(cli.command).await
}

pub(crate) async fn register_live_board_role(
    server: &str,
    canonical_server: &str,
    admin_token: &str,
    board_id: &str,
    nonce: u128,
    role: &str,
) -> (
    TestDirectory,
    App<CapturingStore, FixedPassword, FileProfileStore>,
    String,
) {
    let username = format!("wekan_cli_card_{role}_{nonce}");
    let email = format!("{username}@example.test");
    let password = format!("Wekan-card-{role}-{nonce}!");
    let directory = TestDirectory::new(&format!("card-role-{role}"));
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.to_owned()), Some("default".to_owned()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password)),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let registration = Cli::try_parse_from([
        "wekan",
        "--server",
        server,
        "auth",
        "register",
        "--username",
        &username,
        "--email",
        &email,
        "--password-stdin",
    ])
    .expect("the card-role registration command must parse");
    let CommandSuccess::Registration(registration) = app
        .execute(registration.command)
        .await
        .unwrap_or_else(|error| panic!("the {role} card-test account must register: {error:?}"))
    else {
        panic!("expected {role} registration output")
    };
    let user_id = registration.user_id;
    let membership = live_api_json(
        &reqwest::Client::new(),
        canonical_server,
        admin_token,
        reqwest::Method::POST,
        &format!("api/boards/{board_id}/members/{user_id}/add"),
        Some(serde_json::json!({
            "action": "add",
            "role": role
        })),
    )
    .await;
    assert!(
        membership.as_array().is_some_and(|boards| {
            boards
                .iter()
                .any(|summary| summary["_id"].as_str() == Some(board_id))
        }),
        "the board-admin request must add the {role} card-test member"
    );
    wait_for_live_board_membership(&app, server, board_id).await;
    (directory, app, user_id)
}

pub(crate) async fn execute_live_board(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output=json", "board"];
    argv.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(argv).expect("the live board command must parse");
    app.execute(cli.command).await
}

pub(crate) async fn execute_live_list(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output=json", "list"];
    argv.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(argv).expect("the live list command must parse");
    app.execute(cli.command).await
}

pub(crate) async fn execute_live_card(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output=json", "card"];
    argv.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(argv).expect("the live card command must parse");
    app.execute(cli.command).await
}

pub(crate) async fn execute_live_comment(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output=json", "comment"];
    argv.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(argv).expect("the live comment command must parse");
    app.execute(cli.command).await
}

pub(crate) async fn execute_live_swimlane(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output=json", "swimlane"];
    argv.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(argv).expect("the live swimlane command must parse");
    app.execute(cli.command).await
}

pub(crate) async fn wait_for_live_board_membership(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    board_id: &str,
) {
    for _ in 0..40 {
        let result = execute_live_user(app, server, &["current"])
            .await
            .expect("the ordinary member must remain readable while polling membership");
        let CommandSuccess::UserCurrent(user) = result else {
            panic!("expected current-user output")
        };
        if user
            .boards
            .iter()
            .any(|board| board.board_id == board_id && board.is_active == Some(true))
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("ordinary member was not added to board `{board_id}` within 10 seconds")
}

pub(crate) async fn wait_for_live_swimlane_absent(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    board_id: &str,
    swimlane_id: &str,
) {
    for _ in 0..40 {
        match execute_live_swimlane(app, server, &["get", board_id, swimlane_id]).await {
            Err(error) if error.code() == ErrorCode::NotFound => {
                assert_eq!(error.details().http_status, Some(200));
                assert_eq!(error.details().wekan_status_code, Some(404));
                return;
            }
            Ok(CommandSuccess::SwimlaneShown(_)) => {}
            Ok(other) => panic!("unexpected swimlane absence probe result: {other:?}"),
            Err(error) => panic!("unexpected swimlane absence probe error: {error:?}"),
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("deleted swimlane `{swimlane_id}` remained retrievable for 10 seconds")
}

pub(crate) async fn wait_for_live_swimlane_absent_from_collection(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    board_id: &str,
    swimlane_id: &str,
) {
    for _ in 0..40 {
        let result = execute_live_swimlane(app, server, &["list", board_id])
            .await
            .expect("the post-delete swimlane collection must remain readable");
        let CommandSuccess::SwimlaneCollection(collection) = result else {
            panic!("expected swimlane collection output")
        };
        if collection
            .swimlanes
            .iter()
            .all(|swimlane| swimlane.swimlane_id != swimlane_id)
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("deleted swimlane `{swimlane_id}` remained in its collection for 10 seconds")
}

pub(crate) async fn wait_for_live_list_absent(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    board_id: &str,
    list_id: &str,
) {
    for _ in 0..40 {
        match execute_live_list(app, server, &["get", board_id, list_id]).await {
            Err(error) if error.code() == ErrorCode::NotFound => return,
            Ok(CommandSuccess::ListShown(_)) => {}
            Ok(other) => panic!("unexpected list absence probe result: {other:?}"),
            Err(error) => panic!("unexpected list absence probe error: {error:?}"),
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("cascaded list `{list_id}` remained retrievable for 10 seconds")
}

pub(crate) async fn wait_for_live_card_absent(
    client: &reqwest::Client,
    canonical_server: &str,
    token: &str,
    board_id: &str,
    list_id: &str,
    card_id: &str,
) {
    let card_url = url::Url::parse(canonical_server)
        .expect("the canonical server URL must parse")
        .join(&format!(
            "api/boards/{board_id}/lists/{list_id}/cards/{card_id}"
        ))
        .expect("the cascade-card API path must join");
    for _ in 0..40 {
        let response = client
            .get(card_url.clone())
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .expect("the cascade-card probe must complete");
        assert!(response.status().is_success());
        let body = response
            .bytes()
            .await
            .expect("the cascade-card response body must be readable");
        if body.is_empty() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("cascaded card `{card_id}` remained retrievable for 10 seconds")
}

pub(crate) async fn wait_for_live_user(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    username: &str,
) -> String {
    for _ in 0..40 {
        let result = execute_live_user(app, server, &["list"])
            .await
            .expect("site admin must be able to poll the user list");
        let CommandSuccess::UserList(users) = result else {
            panic!("expected user-list output")
        };
        if let Some(user) = users
            .users
            .iter()
            .find(|user| user.username.as_deref() == Some(username))
        {
            return user.user_id.clone();
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("asynchronous creation of user `{username}` was not observed within 10 seconds")
}

pub(crate) async fn wait_for_live_user_absent(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    user_id: &str,
) {
    for _ in 0..40 {
        let result = execute_live_user(app, server, &["list"])
            .await
            .expect("site admin must be able to poll the user list after deletion");
        let CommandSuccess::UserList(users) = result else {
            panic!("expected user-list output")
        };
        if !users.users.iter().any(|user| user.user_id == user_id) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("deleted user `{user_id}` remained visible for 10 seconds")
}

pub(crate) async fn wait_for_live_board_admin_membership(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    board_id: &str,
) {
    for _ in 0..40 {
        let result = execute_live_user(app, server, &["current"])
            .await
            .expect("the current user must remain readable while polling ownership");
        let CommandSuccess::UserCurrent(user) = result else {
            panic!("expected current-user output")
        };
        if user
            .boards
            .iter()
            .any(|board| board.board_id == board_id && board.is_admin == Some(true))
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!(
        "asynchronous ownership change for board `{board_id}` was not observed within 10 seconds"
    )
}

pub(crate) async fn wait_for_live_board_absent(
    client: &reqwest::Client,
    canonical_server: &str,
    token: &str,
    board_id: &str,
) {
    let url = url::Url::parse(canonical_server)
        .expect("the canonical server URL must parse")
        .join(&format!("api/boards/{board_id}"))
        .expect("the live board path must join");
    for _ in 0..40 {
        let response = client
            .get(url.clone())
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .expect("the deleted-board probe must complete");
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return;
        }
        let status = response.status();
        let body = response
            .text()
            .await
            .expect("the deleted-board probe body must be readable");
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body)
            && value.get("statusCode").and_then(serde_json::Value::as_u64) == Some(404)
        {
            return;
        }
        assert!(
            status.is_success(),
            "deleted-board probe returned unexpected HTTP {status}: {body}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("deleted board `{board_id}` remained retrievable for 10 seconds")
}

pub(crate) async fn live_api_json(
    client: &reqwest::Client,
    canonical_server: &str,
    token: &str,
    method: reqwest::Method,
    path: &str,
    body: Option<serde_json::Value>,
) -> serde_json::Value {
    let url = url::Url::parse(canonical_server)
        .expect("the canonical server URL must parse")
        .join(path)
        .expect("the live API path must join");
    let mut request = client
        .request(method, url)
        .header(AUTHORIZATION, format!("Bearer {token}"));
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request
        .send()
        .await
        .expect("the live setup or cleanup request must complete");
    assert!(response.status().is_success());
    let value: serde_json::Value = response
        .json()
        .await
        .expect("the live API response must be JSON");
    assert!(
        value.get("statusCode").is_none(),
        "Wekan returned an embedded error: {value}"
    );
    value
}

pub(crate) fn required_id(value: &serde_json::Value, resource: &str) -> String {
    value["_id"]
        .as_str()
        .unwrap_or_else(|| panic!("{resource} response must contain a string _id: {value}"))
        .to_owned()
}

pub(crate) async fn assert_token_authenticates(canonical_server: &str, user_id: &str, token: &str) {
    let current_user_url = url::Url::parse(canonical_server)
        .expect("the canonical server URL must parse")
        .join("api/user")
        .expect("the authenticated-user endpoint must join");
    let response = reqwest::Client::new()
        .get(current_user_url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .expect("the authenticated request must complete");

    assert!(
        response.status().is_success(),
        "the registration token must authenticate"
    );
    let user: serde_json::Value = response
        .json()
        .await
        .expect("the current-user response must be JSON");
    let returned_id = user
        .get("_id")
        .or_else(|| user.get("id"))
        .and_then(serde_json::Value::as_str);
    assert_eq!(returned_id, Some(user_id));
}

pub(crate) async fn assert_token_is_rejected(canonical_server: &str, token: &str) {
    let current_user_url = url::Url::parse(canonical_server)
        .expect("the canonical server URL must parse")
        .join("api/user")
        .expect("the authenticated-user endpoint must join");
    let response = reqwest::Client::new()
        .get(current_user_url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .expect("the rejected-token request must complete");

    assert!(response.status().is_success());
    let body: serde_json::Value = response
        .json()
        .await
        .expect("the rejected-token response must be JSON");
    assert_eq!(body["statusCode"], 401);
}

#[derive(Debug)]
pub(crate) struct ProcessOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) fn spawn_production_cli(
    config_directory: &Path,
    server: &str,
    args: &[&str],
    stdin: Option<&str>,
) -> Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wekan"));
    command
        .env("WEKAN_CONFIG_DIR", config_directory)
        .env_remove("WEKAN_PROFILE")
        .env_remove("WEKAN_URL")
        .args(["--server", server, "--output=json", "auth"])
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if args.first() == Some(&"logout") && !args.contains(&"--yes") {
        command.arg("--yes");
    }
    let mut child = command.spawn().expect("the CLI child process must start");
    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .expect("piped stdin must exist")
            .write_all(input.as_bytes())
            .expect("the CLI test input must be written");
    }
    child
}

pub(crate) fn finish_production_cli(child: Child) -> ProcessOutput {
    let output = child
        .wait_with_output()
        .expect("the CLI child process must finish");
    ProcessOutput {
        status: output.status,
        stdout: String::from_utf8(output.stdout).expect("CLI stdout must be UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("CLI stderr must be UTF-8"),
    }
}

pub(crate) fn run_production_cli(
    config_directory: &Path,
    server: &str,
    args: &[&str],
    stdin: Option<&str>,
) -> ProcessOutput {
    finish_production_cli(spawn_production_cli(config_directory, server, args, stdin))
}

pub(crate) fn success_json(output: &ProcessOutput) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let value: serde_json::Value =
        serde_json::from_str(&output.stdout).expect("success stdout must be JSON");
    assert_eq!(value["ok"], true, "{output:?}");
    value
}

pub(crate) fn error_json(output: &ProcessOutput, exit_code: i32, code: &str) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(exit_code), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let value: serde_json::Value =
        serde_json::from_str(&output.stderr).expect("error stderr must be JSON");
    assert_eq!(value["ok"], false, "{output:?}");
    assert_eq!(value["error"]["code"], code, "{output:?}");
    value
}

pub(crate) fn assert_secret_absent(output: &ProcessOutput, secret: &str) {
    assert!(!output.stdout.contains(secret), "secret leaked on stdout");
    assert!(!output.stderr.contains(secret), "secret leaked on stderr");
}

pub(crate) async fn token_state(server: &str, token: &str) -> (u16, serde_json::Value) {
    let response = reqwest::Client::new()
        .get(format!("{server}api/user"))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .expect("the token probe must complete");
    let status = response.status().as_u16();
    let body = response
        .json()
        .await
        .expect("the token probe response must be JSON");
    (status, body)
}

pub(crate) struct NativeCredentialCleanup {
    pub(crate) config_directory: PathBuf,
    pub(crate) server: String,
}

impl Drop for NativeCredentialCleanup {
    fn drop(&mut self) {
        let _ = run_production_cli(
            &self.config_directory,
            &self.server,
            &["logout", "--local-only"],
            None,
        );
    }
}
