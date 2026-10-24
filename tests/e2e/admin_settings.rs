use super::admin_fixtures as fixtures;
use super::support::*;
use serde_json::{Value, json};
type LiveApp = App<CapturingStore, FixedPassword, FileProfileStore>;

async fn run(app: &LiveApp, server: &str, args: &[&str]) -> Result<Value, AppError> {
    let mut argv = vec!["wekan", "--server", server];
    argv.extend_from_slice(args);
    let result = app
        .execute(Cli::try_parse_from(argv).unwrap().command)
        .await?;
    Ok(
        serde_json::from_str::<Value>(&wekan_cli::output::render_success(
            OutputFormat::Json,
            &result,
        ))
        .unwrap()["data"]
            .clone(),
    )
}
async fn update(app: &LiveApp, server: &str, family: &str, body: Value) -> Value {
    let dir = TestDirectory::new("admin-settings-json");
    let file = dir.path().join("settings.json");
    fs::write(&file, body.to_string()).unwrap();
    run(
        app,
        server,
        &[
            "admin",
            family,
            "update",
            "--settings-file",
            file.to_str().unwrap(),
        ],
    )
    .await
    .unwrap()
}
fn mongo(script: &str) -> Value {
    let output = Command::new("docker")
        .args([
            "compose", "exec", "-T", "mongodb", "mongosh", "--quiet", "--eval", script,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "fixture operation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn stored(collection: &str) -> Value {
    mongo(&format!(
        "print(JSON.stringify(db.getSiblingDB('wekan').getCollection({}).findOne({{}})));",
        json!(collection)
    ))
}
fn fixture(collection: &str, operation: &str, body: Value) {
    mongo(&format!(
        "const r=db.getSiblingDB('wekan').getCollection({}).{}({{}},{});print(JSON.stringify({{ok:true}}));",
        json!(collection),
        operation,
        body
    ));
}
fn assert_secret(provider: &str, key: &str, expected: Value) {
    // Compare inside the disposable fixture; never print credential contents.
    mongo(&format!(
        "const d=db.getSiblingDB('wekan').attachmentStorageSettings.findOne({{}});if(JSON.stringify(d.storageConfig[{}][{}])!==JSON.stringify({}))throw new Error('stored credential mismatch');print(JSON.stringify({{ok:true}}));",
        json!(provider),
        json!(key),
        expected
    ));
}
fn production_admin(directory: &Path, server: &str, args: &[&str], stdin: &str) -> ProcessOutput {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wekan"))
        .env("WEKAN_CONFIG_DIR", directory)
        .env_remove("WEKAN_PROFILE")
        .env_remove("WEKAN_URL")
        .args(["--server", server, "--output=json", "admin"])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    finish_production_cli(child)
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v12.57 stack and WEKAN_ADMIN_SETTINGS_E2E_URL"]
async fn complete_admin_settings_lifecycle_matches_wekan_v12_57() {
    let server = env::var("WEKAN_ADMIN_SETTINGS_E2E_URL")
        .expect("set WEKAN_ADMIN_SETTINGS_E2E_URL to a fresh isolated v12.57 stack");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let username = format!("admin_settings_{nonce}");
    let password = format!("Admin-settings-{nonce}!");
    let directory = TestDirectory::new("admin-settings-live");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password.clone())),
        FileProfileStore::at(directory.path().to_owned()),
    );
    run(
        &app,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &username,
            "--email",
            &format!("{username}@example.test"),
            "--password-stdin",
        ],
    )
    .await
    .unwrap();
    let normal_dir = TestDirectory::new("normal-settings-live");
    let normal = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password.clone())),
        FileProfileStore::at(normal_dir.path().to_owned()),
    );
    run(
        &normal,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &format!("normal_{nonce}"),
            "--email",
            &format!("normal_{nonce}@example.test"),
            "--password-stdin",
        ],
    )
    .await
    .unwrap();

    let global_baseline = run(&app, &server, &["admin", "settings", "get"])
        .await
        .unwrap();
    assert_eq!(
        global_baseline["settings"].as_object().unwrap().len(),
        fixtures::global().as_object().unwrap().len()
    );
    // Reads initialize attachment defaults through the native GET endpoint.
    let baseline = run(&app, &server, &["admin", "attachment-settings", "get"])
        .await
        .unwrap();
    assert_eq!(baseline["settings"]["default_storage"], "fs");
    assert_eq!(
        baseline["settings"]["storage_config"]["s3"]["secret_access_key_set"],
        false
    );
    assert_eq!(
        baseline["settings"]["settings_id"],
        stored("attachmentStorageSettings")["_id"]
    );
    for family in ["settings", "attachment-settings"] {
        for action in ["get", "update"] {
            let mut args = vec!["admin", family, action];
            if action == "update" {
                args.extend(if family == "settings" {
                    vec!["--product-name", "denied"]
                } else {
                    vec!["--default-storage", "fs"]
                });
            }
            let error = run(&normal, &server, &args).await.unwrap_err();
            assert_eq!(error.code(), ErrorCode::PermissionDenied);
            assert_eq!(error.details().http_status, Some(403));
            assert_eq!(error.details().wekan_status_code, None);
        }
    }
    // Every REST-allowlisted field, including both boolean states, travels through flags.
    for state in [false, true] {
        let mut args = vec!["admin".to_owned(), "settings".into(), "update".into()];
        for (wire, _, flag, boolean) in fixtures::GLOBAL_FIELDS {
            let value = if *boolean {
                state.to_string()
            } else if *wire == "defaultAuthenticationMethod" {
                "password".into()
            } else {
                format!("admin-{wire}")
            };
            args.extend([(*flag).into(), value]);
        }
        let output = run(
            &app,
            &server,
            &args.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .await
        .unwrap();
        let saved = stored("settings");
        assert_eq!(
            output["submitted_fields"].as_array().unwrap().len(),
            fixtures::GLOBAL_FIELDS.len()
        );
        for (wire, snake, _, _) in fixtures::GLOBAL_FIELDS {
            assert_eq!(output["settings"][*snake], saved[*wire], "{wire}");
        }
    }
    let prior = stored("settings");
    let partial = update(
        &app,
        &server,
        "settings",
        json!({"productName":"Configured via JSON","disableRegistration":false}),
    )
    .await;
    assert_eq!(partial["settings"]["product_name"], "Configured via JSON");
    assert_eq!(partial["settings"]["support_title"], prior["supportTitle"]);

    // Every non-secret attachment setting maps through its documented flag.
    let mut args = vec![
        "admin".into(),
        "attachment-settings".into(),
        "update".into(),
        "--default-storage".into(),
        "gridfs".into(),
    ];
    for (wire, _, flag, kind) in fixtures::ATTACHMENT_FIELDS {
        let source = fixtures::attachment();
        let mut value = source.pointer(wire).unwrap().clone();
        if *flag == "--filesystem-path" {
            value = baseline["settings"]["storage_config"]["filesystem"]["path"].clone();
        }
        if [
            "--s3-enabled",
            "--azure-enabled",
            "--gcs-enabled",
            "--migration-auto-migrate",
        ]
        .contains(flag)
        {
            value = json!(false);
        }
        args.extend([
            (*flag).into(),
            if *kind == "Vec<String>" {
                value[0].as_str().unwrap().into()
            } else if *kind == "String" {
                value.as_str().unwrap().into()
            } else {
                value.to_string()
            },
        ]);
    }
    let output = run(
        &app,
        &server,
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
    )
    .await
    .unwrap();
    let saved = stored("attachmentStorageSettings");
    for (wire, snake, _, _) in fixtures::ATTACHMENT_FIELDS {
        assert_eq!(
            output["settings"].pointer(snake),
            saved.pointer(wire),
            "{wire}"
        );
    }
    for backend in ["fs", "gridfs", "s3", "azure", "gcs"] {
        assert_eq!(
            update(
                &app,
                &server,
                "attachment-settings",
                json!({"defaultStorage":backend})
            )
            .await["settings"]["default_storage"],
            backend
        );
    }

    // JSON updates preserve native cloud merging, including object-form GCS credentials.
    let credentials =
        json!({"private_key":"dummy-private-key","client_email":"dummy@example.test"});
    let output=update(&app,&server,"attachment-settings",json!({"defaultStorage":"fs","storageConfig":{"s3":{"enabled":false,"bucket":"dummy-bucket","secretAccessKey":"dummy-s3-secret"},"azure":{"enabled":false,"accountKey":"dummy-azure-key","connectionString":"dummy-connection"},"gcs":{"enabled":false,"credentials":credentials}}})).await;
    assert_eq!(
        output["settings"]["storage_config"]["s3"]["secret_access_key_set"],
        true
    );
    assert_eq!(
        output["settings"]["storage_config"]["gcs"]["credentials_set"],
        true
    );
    assert_secret("s3", "secretAccessKey", json!("dummy-s3-secret"));
    assert_secret("gcs", "credentials", credentials.clone());
    update(&app,&server,"attachment-settings",json!({"storageConfig":{"s3":{"region":"changed-region","secretAccessKey":""},"azure":{"accountKey":"","connectionString":""},"gcs":{"credentials":""}}})).await;
    assert_secret("s3", "secretAccessKey", json!("dummy-s3-secret"));
    assert_secret("azure", "accountKey", json!("dummy-azure-key"));
    assert_secret("azure", "connectionString", json!("dummy-connection"));
    assert_secret("gcs", "credentials", credentials);
    assert_eq!(
        stored("attachmentStorageSettings")["storageConfig"]["s3"]["bucket"],
        "dummy-bucket"
    );

    // Replacement sections reset defaults and remove omitted non-default siblings.
    update(&app,&server,"attachment-settings",json!({"limitSettings":{"apiUploadMaxBytes":123,"apiDownloadMaxBytes":456,"apiUploadBlocked":true},"uploadSettings":{"maxFileSize":999,"allowedMimeTypes":["image/png"]},"migrationSettings":{"autoMigrate":false,"batchSize":20,"delayMs":2000,"cpuThreshold":80}})).await;
    let output=update(&app,&server,"attachment-settings",json!({"limitSettings":{"avatarsUploadBlocked":true},"uploadSettings":{"allowedMimeTypes":[]},"migrationSettings":{"batchSize":1.5},"storageConfig":{"filesystem":{"path":"/data/files/attachments"},"gridfs":{"read":false}}})).await;
    assert!(output["settings"]["limit_settings"]["api_upload_max_bytes"].is_null());
    assert_eq!(
        output["settings"]["limit_settings"]["api_upload_blocked"],
        false
    );
    assert!(output["settings"]["upload_settings"]["max_file_size"].is_null());
    assert_eq!(
        output["settings"]["upload_settings"]["allowed_mime_types"],
        json!([])
    );
    assert_eq!(
        output["settings"]["migration_settings"]["batch_size"],
        json!(1.5)
    );
    assert_eq!(output["settings"]["migration_settings"]["delay_ms"], 1000);
    assert_eq!(
        output["settings"]["storage_config"]["gridfs"]["write"],
        true
    );
    assert_eq!(
        output["settings"]["storage_config"]["gridfs"]["read"],
        false
    );
    update(&app,&server,"attachment-settings",json!({"limitSettings":{"apiUploadMaxBytes":0,"apiDownloadMaxBytes":1.5},"migrationSettings":{"autoMigrate":false,"batchSize":100,"delayMs":10000,"cpuThreshold":90}})).await;

    // Exercise real binary stdin modes and keep native credentials scoped to this fixture.
    let process_dir = TestDirectory::new("admin-settings-process");
    let _cleanup = NativeCredentialCleanup {
        config_directory: process_dir.path().to_owned(),
        server: server.clone(),
    };
    success_json(&run_production_cli(
        process_dir.path(),
        &server,
        &["login", "--username", &username, "--password-stdin"],
        Some(&password),
    ));
    let output = production_admin(
        process_dir.path(),
        &server,
        &["settings", "update", "--settings-stdin"],
        r#"{"productName":"From stdin"}"#,
    );
    assert_eq!(
        success_json(&output)["data"]["settings"]["product_name"],
        "From stdin"
    );
    let output = production_admin(
        process_dir.path(),
        &server,
        &["attachment-settings", "update", "--settings-stdin"],
        r#"{"limitSettings":{"apiDownloadBlocked":false}}"#,
    );
    success_json(&output);
    let output = production_admin(
        process_dir.path(),
        &server,
        &[
            "attachment-settings",
            "update",
            "--s3-secret-access-key-stdin",
        ],
        "replacement-from-stdin\r\n",
    );
    success_json(&output);
    assert_secret_absent(&output, "replacement-from-stdin");
    assert_secret("s3", "secretAccessKey", json!("replacement-from-stdin"));
    let secret_file = directory.path().join("azure-key.txt");
    fs::write(&secret_file, "replacement-from-file\n").unwrap();
    run(
        &app,
        &server,
        &[
            "admin",
            "attachment-settings",
            "update",
            "--azure-account-key-file",
            secret_file.to_str().unwrap(),
            "--clear-upload-allowed-mime-types",
        ],
    )
    .await
    .unwrap();
    assert_secret("azure", "accountKey", json!("replacement-from-file"));

    // Fixture-only unknown fields must fail at the production strict-decoding boundary.
    fixture(
        "attachmentStorageSettings",
        "updateOne",
        json!({"$set":{"storageConfig.s3.futureOption":true}}),
    );
    let error = run(&app, &server, &["admin", "attachment-settings", "get"])
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::ProtocolError);
    assert_eq!(
        error.details().response_error,
        Some(wekan_cli::client::ResponseDecodeKind::UnknownField)
    );
    fixture(
        "attachmentStorageSettings",
        "updateOne",
        json!({"$unset":{"storageConfig":"","uploadSettings":"","limitSettings":"","migrationSettings":"","createdBy":"","updatedBy":""}}),
    );
    let sparse = run(&app, &server, &["admin", "attachment-settings", "get"])
        .await
        .unwrap();
    assert!(sparse["settings"]["storage_config"].is_null());
    assert!(sparse["settings"]["created_by"].is_null());

    // Finish with the documented empty-document branches on this disposable stack.
    mongo("db.getSiblingDB('wekan').settings.deleteMany({});print(JSON.stringify({ok:true}));");
    assert!(
        run(&app, &server, &["admin", "settings", "get"])
            .await
            .unwrap()["settings"]["settings_id"]
            .is_null()
    );
    let error = run(
        &app,
        &server,
        &["admin", "settings", "update", "--product-name", "missing"],
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::NotFound);
    assert_eq!(error.details().http_status, Some(404));
}
