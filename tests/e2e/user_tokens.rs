use super::support::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use wekan_cli::credentials::CredentialTarget;

fn run(
    directory: &Path,
    server: &str,
    args: &[&str],
    stdin: Option<&str>,
    format: &str,
) -> ProcessOutput {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wekan"))
        .env("WEKAN_CONFIG_DIR", directory)
        .env_remove("WEKAN_URL")
        .env_remove("WEKAN_PROFILE")
        .args(["--server", server, "--output", format])
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
    }
    finish_production_cli(child)
}

fn success(output: &ProcessOutput) -> Value {
    // Creation stdout deliberately contains a secret; never print it on failure.
    assert!(output.status.success(), "command failed: {}", output.stderr);
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_str(&output.stdout).expect("success must be JSON");
    assert_eq!(value["ok"], true);
    value["data"].clone()
}

fn credential(directory: &Path, server: &str) -> CredentialRecord {
    KeyringCredentialStore
        .load(&credential_target(directory, server))
        .unwrap()
        .unwrap()
}

fn credential_target(directory: &Path, server: &str) -> CredentialTarget {
    let namespace = FileProfileStore::at(directory.to_owned())
        .credential_namespace()
        .unwrap();
    CredentialTarget::profile_in_store("default", &namespace, server.to_owned())
}

async fn assert_token_state(server: &str, user: &str, token: &str, accepted: bool) {
    // Native insertion/destruction does not await the Accounts database write.
    // Observe eventual persistence only in the test; production never retries.
    for _ in 0..100 {
        let (_, value) = token_state(server, token).await;
        let matches = if accepted {
            value["_id"].as_str() == Some(user)
        } else {
            value["statusCode"] == 401
        };
        if matches {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("token authentication did not reach the expected state (accepted={accepted})");
}

fn saved_tokens(user: &str) -> Value {
    // Return only type/count; token material stays inside MongoDB.
    let script = format!(
        "const u=db.getSiblingDB('wekan').users.findOne({{_id:{}}}); if(!u){{print(JSON.stringify({{exists:false}}));quit(0);}} const t=u.services.resume.loginTokens; print(JSON.stringify({{type:typeof t,array:Array.isArray(t),count:Array.isArray(t)?t.length:null}}));",
        json!(user)
    );
    let output = Command::new("docker")
        .args([
            "compose", "exec", "-T", "mongodb", "mongosh", "--quiet", "--eval", &script,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Mongo readback failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Compose Wekan v11.06 stack and WEKAN_TOKEN_E2E_URL"]
async fn complete_user_token_lifecycle_matches_wekan_v11_06() {
    let server = env::var("WEKAN_TOKEN_E2E_URL")
        .expect("set WEKAN_TOKEN_E2E_URL to a fresh isolated v11.06 stack");
    let canonical = ServerUrl::parse(&server).unwrap().as_str().to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let password = format!("Token-e2e-{nonce}!");
    let admin_dir = TestDirectory::new("token-admin");
    let member_dir = TestDirectory::new("token-member");
    let admin_cleanup = NativeCredentialCleanup {
        config_directory: admin_dir.path().to_owned(),
        server: server.clone(),
    };
    let member_cleanup = NativeCredentialCleanup {
        config_directory: member_dir.path().to_owned(),
        server: server.clone(),
    };
    let register = |dir: &Path, name: &str| {
        success(&run(
            dir,
            &server,
            &[
                "auth",
                "register",
                "--username",
                name,
                "--email",
                &format!("{name}@example.test"),
                "--password-stdin",
            ],
            Some(&password),
            "json",
        ))
    };
    let admin = register(admin_dir.path(), &format!("token_admin_{nonce}"))["user_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let member_username = format!("token_member_{nonce}");
    let member = register(member_dir.path(), &member_username)["user_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let admin_credential = credential(admin_dir.path(), &canonical);
    let member_credential = credential(member_dir.path(), &canonical);
    let before = fs::read(admin_dir.path().join("profiles.json")).unwrap();
    assert_eq!(saved_tokens(&member)["count"], 1);

    let issue = |user: &str| {
        success(&run(
            admin_dir.path(),
            &server,
            &["user", "token", "create", user],
            None,
            "json",
        ))
    };
    let first = issue(&member);
    let first_token = first["auth_token"].as_str().unwrap();
    assert_eq!(first["user_id"], member);
    assert!(first.get("creation_requested").is_none());
    assert!(first.get("mutation_confirmed").is_none());
    assert_eq!(first.as_object().unwrap().len(), 2);
    assert!(first.get("token_expires").is_none());
    assert_token_state(&canonical, &member, first_token, true).await;
    let second = issue(&member);
    let second_token = second["auth_token"].as_str().unwrap();
    assert_token_state(&canonical, &member, second_token, true).await;
    assert_eq!(saved_tokens(&member)["count"], 3);

    // W11-058: raw issued tokens are not matched against stored hashes.
    let raw_delete = run(
        admin_dir.path(),
        &server,
        &["user", "token", "delete", &member, "--token-stdin", "--yes"],
        Some(&format!("{first_token}\r\n")),
        "json",
    );
    let data = success(&raw_delete);
    assert_eq!(data["scope"], "single");
    assert!(data.get("mutation_confirmed").is_none());
    assert!(data.get("deletion_requested").is_none());
    assert!(data["message"].as_str().unwrap().contains("[REDACTED]"));
    assert_secret_absent(&raw_delete, first_token);
    assert_secret_absent(&raw_delete, admin_credential.token().expose_secret());
    assert_token_state(&canonical, &member, first_token, true).await;
    assert_eq!(saved_tokens(&member)["count"], 3);

    // Pass an explicit hash as API input to establish native matching semantics.
    // The CLI itself performs no hashing or automatic substitution.
    let hash = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        Sha256::digest(first_token.as_bytes()),
    );
    let hash_delete = run(
        admin_dir.path(),
        &server,
        &["user", "token", "delete", &member, "--token-stdin", "--yes"],
        Some(&hash),
        "human",
    );
    assert!(hash_delete.status.success(), "{}", hash_delete.stderr);
    assert_secret_absent(&hash_delete, &hash);
    assert_token_state(&canonical, &member, first_token, false).await;
    assert_token_state(&canonical, &member, second_token, true).await;
    assert_token_state(
        &canonical,
        &member,
        member_credential.token().expose_secret(),
        true,
    )
    .await;
    assert_eq!(saved_tokens(&member)["count"], 2);

    for user in [&member, "missing-token-user"] {
        let output = run(
            admin_dir.path(),
            &server,
            &["user", "token", "delete", user, "--token-stdin", "--yes"],
            Some("nonexistent-token\n"),
            "json",
        );
        assert!(success(&output).get("mutation_confirmed").is_none());
        assert_secret_absent(&output, "nonexistent-token");
    }
    // W11-059: creation returns an unusable token even for a nonexistent user.
    assert_eq!(saved_tokens("missing-token-user"), json!({"exists":false}));
    let missing = issue("missing-token-user");
    assert!(missing.get("creation_requested").is_none());
    assert!(missing.get("mutation_confirmed").is_none());
    assert_token_state(
        &canonical,
        "missing-token-user",
        missing["auth_token"].as_str().unwrap(),
        false,
    )
    .await;
    assert_eq!(saved_tokens("missing-token-user"), json!({"exists":false}));
    let missing_all = run(
        admin_dir.path(),
        &server,
        &[
            "user",
            "token",
            "delete",
            "missing-token-user",
            "--all",
            "--yes",
        ],
        None,
        "json",
    );
    assert!(success(&missing_all).get("mutation_confirmed").is_none());

    for args in [
        vec!["user", "token", "create", &member],
        vec!["user", "token", "delete", &member, "--all", "--yes"],
    ] {
        let output = run(member_dir.path(), &server, &args, None, "json");
        let error = error_json(&output, 5, "permission_denied");
        assert_eq!(error["error"]["details"]["http_status"], 200);
        assert_eq!(error["error"]["details"]["wekan_status_code"], 403);
    }
    let no_confirmation = run(
        admin_dir.path(),
        &server,
        &["user", "token", "delete", &member, "--all"],
        None,
        "json",
    );
    error_json(&no_confirmation, 2, "invalid_input");
    let empty = run(
        admin_dir.path(),
        &server,
        &["user", "token", "delete", &member, "--token-stdin", "--yes"],
        Some("\r\n"),
        "json",
    );
    error_json(&empty, 2, "invalid_input");
    let invalid_utf8 = {
        let mut child = Command::new(env!("CARGO_BIN_EXE_wekan"))
            .env("WEKAN_CONFIG_DIR", admin_dir.path())
            .env_remove("WEKAN_PROFILE")
            .env_remove("WEKAN_URL")
            .args([
                "--server",
                &server,
                "--output",
                "json",
                "user",
                "token",
                "delete",
                &member,
                "--token-stdin",
                "--yes",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&[0xff]).unwrap();
        finish_production_cli(child)
    };
    error_json(&invalid_utf8, 2, "invalid_input");
    assert_eq!(saved_tokens(&member)["count"], 2);

    let all = run(
        admin_dir.path(),
        &server,
        &["user", "token", "delete", &member, "--all", "--yes"],
        None,
        "json",
    );
    assert_eq!(success(&all)["scope"], "all");
    assert_token_state(&canonical, &member, second_token, false).await;
    assert_token_state(
        &canonical,
        &member,
        member_credential.token().expose_secret(),
        false,
    )
    .await;
    // The native empty-string update is cleaned into an omitted token field.
    assert_eq!(
        saved_tokens(&member),
        json!({"type":"undefined","array":false,"count":null})
    );
    assert!(credential(member_dir.path(), &canonical).matches(&member_credential));

    // The user can obtain a new session after delete-all through normal login.
    success(&run(
        member_dir.path(),
        &server,
        &["auth", "logout", "--local-only", "--yes"],
        None,
        "json",
    ));
    success(&run(
        member_dir.path(),
        &server,
        &[
            "auth",
            "login",
            "--username",
            &member_username,
            "--password-stdin",
        ],
        Some(&password),
        "json",
    ));
    let replacement = credential(member_dir.path(), &canonical);
    assert_token_state(
        &canonical,
        &member,
        replacement.token().expose_secret(),
        true,
    )
    .await;

    let self_token = issue(&admin);
    assert_token_state(
        &canonical,
        &admin,
        self_token["auth_token"].as_str().unwrap(),
        true,
    )
    .await;
    let self_all = run(
        admin_dir.path(),
        &server,
        &["user", "token", "delete", &admin, "--all", "--yes"],
        None,
        "json",
    );
    assert!(success(&self_all).get("mutation_confirmed").is_none());
    assert_token_state(
        &canonical,
        &admin,
        admin_credential.token().expose_secret(),
        false,
    )
    .await;
    assert_token_state(
        &canonical,
        &admin,
        self_token["auth_token"].as_str().unwrap(),
        false,
    )
    .await;
    assert!(credential(admin_dir.path(), &canonical).matches(&admin_credential));
    assert!(fs::read(admin_dir.path().join("profiles.json")).unwrap() == before);
    for cleanup in [member_cleanup, admin_cleanup] {
        let target = credential_target(&cleanup.config_directory, &canonical);
        drop(cleanup);
        assert!(!KeyringCredentialStore.exists(&target).unwrap());
    }
}
