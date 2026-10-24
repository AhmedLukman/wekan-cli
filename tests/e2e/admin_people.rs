use super::support::*;
use serde_json::{Value, json};
#[path = "../support/admin_people.rs"]
mod fixtures;

type LiveApp = App<CapturingStore, FixedPassword, FileProfileStore>;

async fn run(app: &LiveApp, server: &str, args: &[&str]) -> Result<Value, AppError> {
    let command = Cli::try_parse_from(
        ["wekan", "--server", server]
            .into_iter()
            .chain(args.iter().copied()),
    )
    .unwrap()
    .command;
    let result = app.execute(command).await?;
    Ok(
        serde_json::from_str::<Value>(&wekan_cli::output::render_success(
            OutputFormat::Json,
            &result,
        ))
        .unwrap()["data"]
            .clone(),
    )
}
fn mongo(script: &str) -> Value {
    let script = format!("const d=db.getSiblingDB('wekan');{script}");
    let output = Command::new("docker")
        .args([
            "compose", "exec", "-T", "mongodb", "mongosh", "--quiet", "--eval", &script,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Mongo fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn stored(kind: &str, id: &str) -> Value {
    mongo(&format!(
        "print(JSON.stringify(d.getCollection({}).findOne({{_id:{}}})));",
        json!(kind),
        json!(id)
    ))
}
fn seed(kind: &str, mut document: Value) {
    document["createdAt"] = json!("2026-10-15T00:00:00.000Z");
    document["modifiedAt"] = json!("2026-10-15T00:00:00.000Z");
    mongo(&format!(
        "const doc={document};doc.createdAt=new Date(doc.createdAt);doc.modifiedAt=new Date(doc.modifiedAt);d.getCollection({}).insertOne(doc);print(JSON.stringify({{ok:true}}));",
        json!(kind)
    ));
}
fn production(directory: &Path, server: &str, args: &[&str], format: &str) -> ProcessOutput {
    let child = Command::new(env!("CARGO_BIN_EXE_wekan"))
        .env("WEKAN_CONFIG_DIR", directory)
        .env_remove("WEKAN_URL")
        .env_remove("WEKAN_PROFILE")
        .args(["--server", server, "--output", format, "admin"])
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    finish_production_cli(child)
}

fn compare_document(kind: &str, output: &Value, saved: &Value) {
    for (wire, value) in saved.as_object().unwrap() {
        let key = if wire == "_id" {
            if kind == "org" {
                "organization_id".into()
            } else {
                "team_id".into()
            }
        } else {
            let name = wire.strip_prefix(kind).unwrap_or(wire);
            let mut key = String::new();
            for (index, c) in name.chars().enumerate() {
                if c.is_uppercase() && index > 0 {
                    key.push('_');
                }
                key.extend(c.to_lowercase());
            }
            key
        };
        assert_eq!(output[&key], *value, "{wire}");
    }
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v12.57 stack and WEKAN_ADMIN_PEOPLE_E2E_URL"]
async fn complete_admin_people_lifecycle_matches_wekan_v12_57() {
    let server = env::var("WEKAN_ADMIN_PEOPLE_E2E_URL")
        .expect("set WEKAN_ADMIN_PEOPLE_E2E_URL to a fresh isolated v12.57 stack");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let username = format!("admin_people_{nonce}");
    let password = format!("Admin-people-{nonce}!");
    let directory = TestDirectory::new("admin-people-live");
    let normal_directory = TestDirectory::new("admin-people-normal");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password.clone())),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let normal = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password.clone())),
        FileProfileStore::at(normal_directory.path().to_owned()),
    );
    for (app, name, email) in [
        (&app, username.clone(), format!("{username}@example.test")),
        (
            &normal,
            format!("normal_{nonce}"),
            format!("normal_{nonce}@example.test"),
        ),
    ] {
        run(
            app,
            &server,
            &[
                "auth",
                "register",
                "--username",
                &name,
                "--email",
                &email,
                "--password-stdin",
            ],
        )
        .await
        .unwrap();
    }

    // First-address-only grouping, native normalization, invalid/missing-address skips, and ties.
    mongo(
        "d.users.insertMany([{_id:'domain-a',emails:[{address:'a@ ALPHA.TEST '},{address:'ignored@example.test'}]},{_id:'domain-b',emails:[{address:'b@alpha.test'}]},{_id:'domain-c',emails:[{address:'c@beta.test'}]},{_id:'domain-d',emails:[{address:'invalid'},{address:'ignored2@example.test'}]},{_id:'domain-e',emails:[{address:'no-domain@'}]},{_id:'domain-f'}]);print(JSON.stringify({ok:true}));",
    );
    let domains = run(&app, &server, &["admin", "domain", "list"])
        .await
        .unwrap();
    assert_eq!(
        domains["domains"],
        json!([{"domain":"alpha.test","count":2},{"domain":"example.test","count":2},{"domain":"beta.test","count":1}])
    );

    for kind in ["org", "team"] {
        let resource = if kind == "org" { "orgs" } else { "teams" };
        let plural = if kind == "org" {
            "organizations"
        } else {
            "teams"
        };
        let singular = if kind == "org" {
            "organization"
        } else {
            "team"
        };
        let selector = format!("--{kind}");
        let id = format!("{kind}-1");
        let sparse_id = format!("{kind}-2");
        assert_eq!(
            run(&app, &server, &["admin", kind, "list"]).await.unwrap()[plural],
            json!([])
        );
        assert_eq!(
            run(
                &app,
                &server,
                &[
                    "admin",
                    kind,
                    "features",
                    "update-all",
                    "--field",
                    "shared-templates",
                    "--value",
                    "true",
                    "--yes"
                ]
            )
            .await
            .unwrap()["updated"],
            0
        );
        let full = if kind == "org" {
            fixtures::organization()
        } else {
            fixtures::team()
        };
        seed(kind, full.clone());
        seed(
            kind,
            json!({"_id":sparse_id,format!("{kind}IsActive"):false}),
        );
        let listed = run(&app, &server, &["admin", kind, "list"]).await.unwrap();
        let entries = listed[plural].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        let sparse = entries
            .iter()
            .find(|entry| entry[format!("{singular}_id")] == sparse_id)
            .unwrap();
        assert!(sparse["display_name"].is_null());
        for (_, _, field) in fixtures::FEATURES {
            assert_eq!(sparse[field], false);
        }
        let summary = fixtures::summary(kind, &full);
        assert_eq!(summary.as_object().unwrap().len(), 6);

        for (suffix, flag, field) in fixtures::FEATURES {
            for state in [false, true] {
                let before = stored(kind, &id);
                let value = state.to_string();
                let result = run(
                    &app,
                    &server,
                    &[
                        "admin",
                        kind,
                        "features",
                        "update",
                        &selector,
                        &id,
                        &format!("--{flag}"),
                        &value,
                    ],
                )
                .await
                .unwrap();
                assert_eq!(result["submitted_fields"], json!([field]));
                assert_eq!(result[singular][field], state);
                let saved = stored(kind, &id);
                assert_eq!(saved[format!("{kind}{suffix}")], state);
                compare_document(kind, &result[singular], &saved);
                for (other, _, _) in fixtures::FEATURES
                    .into_iter()
                    .filter(|(other, _, _)| *other != suffix)
                {
                    assert_eq!(
                        saved[format!("{kind}{other}")],
                        before[format!("{kind}{other}")]
                    );
                }
            }
        }
        for (suffix, flag, field) in fixtures::FEATURES {
            for state in [false, true] {
                let result = run(
                    &app,
                    &server,
                    &[
                        "admin",
                        kind,
                        "features",
                        "update-all",
                        "--field",
                        flag,
                        "--value",
                        &state.to_string(),
                        "--yes",
                    ],
                )
                .await
                .unwrap();
                assert_eq!(result, json!({"field":field,"value":state,"updated":2}));
                let listed = run(&app, &server, &["admin", kind, "list"]).await.unwrap();
                for entry in listed[plural].as_array().unwrap() {
                    assert_eq!(entry[field], state);
                }
                assert_eq!(stored(kind, &sparse_id)[format!("{kind}{suffix}")], state);
            }
        }

        for args in [
            vec!["admin", kind, "list"],
            vec![
                "admin",
                kind,
                "features",
                "update",
                &selector,
                &id,
                "--shared-templates",
                "false",
            ],
            vec![
                "admin",
                kind,
                "features",
                "update-all",
                "--field",
                "shared-templates",
                "--value",
                "false",
                "--yes",
            ],
        ] {
            let error = run(&normal, &server, &args).await.unwrap_err();
            assert_eq!(error.code(), ErrorCode::PermissionDenied);
            assert_eq!(error.details().http_status, Some(403));
            assert_eq!(error.details().wekan_status_code, None);
        }
        let missing = run(
            &app,
            &server,
            &[
                "admin",
                kind,
                "features",
                "update",
                &selector,
                "absent",
                "--shared-templates",
                "true",
            ],
        )
        .await
        .unwrap_err();
        assert_eq!(missing.code(), ErrorCode::ProtocolError);
        assert_eq!(missing.details().http_status, Some(200));
        assert!(
            serde_json::to_value(missing.details())
                .unwrap()
                .get("outcome_unknown")
                .is_none()
        );

        // A field outside the finite pinned DTO fails after the native write, without a preflight.
        mongo(&format!(
            "d.getCollection({}).updateOne({{_id:{}}},{{$set:{{futureField:{{secret:'unmapped'}}}}}});print(JSON.stringify({{ok:true}}));",
            json!(kind),
            json!(id)
        ));
        let error = run(
            &app,
            &server,
            &[
                "admin",
                kind,
                "features",
                "update",
                &selector,
                &id,
                "--shared-templates",
                "false",
            ],
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::ProtocolError);
        assert_eq!(
            error.details().response_error,
            Some(wekan_cli::client::ResponseDecodeKind::UnknownField)
        );
        assert!(
            serde_json::to_value(error.details())
                .unwrap()
                .get("outcome_unknown")
                .is_none()
        );
        assert_eq!(stored(kind, &id)[format!("{kind}SharedTemplates")], false);
        mongo(&format!(
            "d.getCollection({}).updateOne({{_id:{}}},{{$unset:{{futureField:''}}}});print(JSON.stringify({{ok:true}}));",
            json!(kind),
            json!(id)
        ));

        let sparse = run(
            &app,
            &server,
            &[
                "admin",
                kind,
                "features",
                "update",
                &selector,
                &sparse_id,
                "--shared-templates",
                "true",
            ],
        )
        .await
        .unwrap();
        assert!(sparse[singular]["display_name"].is_null());
        assert!(sparse[singular]["desc"].is_null());

        // Unauthenticated calls expose the native HTTP200 + embedded 401 contract.
        for (method, path, body) in [
            (reqwest::Method::GET, format!("api/admin/{resource}"), None),
            (
                reqwest::Method::PUT,
                format!("api/admin/{resource}/{id}/features"),
                Some(json!({format!("{kind}SharedTemplates"):true})),
            ),
            (
                reqwest::Method::PUT,
                format!("api/admin/{resource}/features"),
                Some(json!({"field":format!("{kind}SharedTemplates"),"value":true})),
            ),
        ] {
            let mut request = reqwest::Client::new()
                .request(method, format!("{}/{path}", server.trim_end_matches('/')));
            if let Some(body) = body {
                request = request.json(&body);
            }
            let response = request.send().await.unwrap();
            assert_eq!(response.status(), 401);
            assert_eq!(
                response.json::<Value>().await.unwrap()["error"],
                "Unauthorized"
            );
        }
    }
    let error = run(&normal, &server, &["admin", "domain", "list"])
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PermissionDenied);
    assert_eq!(error.details().wekan_status_code, None);
    assert_eq!(error.details().http_status, Some(403));
    let response = reqwest::get(format!(
        "{}/api/admin/domains",
        server.trim_end_matches('/')
    ))
    .await
    .unwrap();
    assert_eq!(response.status(), 401);
    assert_eq!(
        response.json::<Value>().await.unwrap()["error"],
        "Unauthorized"
    );

    // Exercise all seven operations through the real binary and native credential store.
    let process_directory = TestDirectory::new("admin-people-process");
    let _cleanup = NativeCredentialCleanup {
        config_directory: process_directory.path().to_owned(),
        server: server.clone(),
    };
    success_json(&run_production_cli(
        process_directory.path(),
        &server,
        &["login", "--username", &username, "--password-stdin"],
        Some(&password),
    ));
    for args in [
        vec!["domain", "list"],
        vec!["org", "list"],
        vec!["team", "list"],
        vec![
            "org",
            "features",
            "update",
            "--org",
            "org-1",
            "--shared-templates",
            "false",
        ],
        vec![
            "team",
            "features",
            "update",
            "--team",
            "team-1",
            "--shared-templates",
            "false",
        ],
        vec![
            "org",
            "features",
            "update-all",
            "--field",
            "shared-templates",
            "--value",
            "true",
            "--yes",
        ],
        vec![
            "team",
            "features",
            "update-all",
            "--field",
            "shared-templates",
            "--value",
            "true",
            "--yes",
        ],
    ] {
        let output = production(process_directory.path(), &server, &args, "json");
        success_json(&output);
        assert_secret_absent(&output, &password);
    }
    let human = production(process_directory.path(), &server, &["org", "list"], "human");
    assert!(human.status.success());
    assert!(human.stdout.contains("display_name"));
    assert!(!human.stdout.contains('\u{1b}'));
    for kind in ["org", "team"] {
        let before = stored(kind, &format!("{kind}-1"));
        let output = production(
            process_directory.path(),
            &server,
            &[
                kind,
                "features",
                "update-all",
                "--field",
                "shared-templates",
                "--value",
                "false",
            ],
            "json",
        );
        error_json(&output, 2, "invalid_input");
        assert_eq!(stored(kind, &format!("{kind}-1")), before);
    }
}
