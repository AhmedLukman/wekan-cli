use super::admin_commands::{json_data, run, run_with_confirmation};
#[path = "../support/admin_people.rs"]
mod fixtures;
use clap::{CommandFactory, Parser};
use serde_json::{Value, json};
use wekan_cli::{
    cli::Cli,
    client::{ClientError, admin::people::*},
    error::{AppError, ErrorCode},
    input::{ConfirmationProvider, ConfirmationRequest},
    output::{OutputFormat, render_success},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path},
};

#[tokio::test]
async fn all_native_requests_and_complete_stable_results() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/admin/domains"))
        .and(header("authorization", "Bearer card-token"))
        .and(header("accept", "application/json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                json!([{"domain":"b.test","count":4},{"domain":"a.test","count":2}]),
            ),
        )
        .expect(1)
        .mount(&server)
        .await;
    let result = run(&server, &["domain", "list"]).await.unwrap();
    assert_eq!(
        json_data(&result)["domains"],
        json!([{"domain":"b.test","count":4},{"domain":"a.test","count":2}])
    );
    for (kind, collection, output_key, document) in [
        ("org", "orgs", "organization", fixtures::organization()),
        ("team", "teams", "team", fixtures::team()),
    ] {
        let summary = fixtures::summary(kind, &document);
        Mock::given(method("GET"))
            .and(path(format!("/api/admin/{collection}")))
            .and(header("authorization", "Bearer card-token"))
            .and(header("accept", "application/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([summary])))
            .expect(1)
            .mount(&server)
            .await;
        let result = run(&server, &[kind, "list"]).await.unwrap();
        let plural = format!("{output_key}s");
        let stable = &json_data(&result)[&plural][0];
        assert_eq!(stable[format!("{output_key}_id")], document["_id"]);
        assert_eq!(stable["shared_templates"], false);
        assert_eq!(stable["propagate_members_to_boards"], true);

        for boolean in [false, true] {
            let value = boolean.to_string();
            let mut body = json!({});
            let mut response = document.clone();
            for (suffix, _, _) in fixtures::FEATURES {
                let key = format!("{kind}{suffix}");
                body[&key] = json!(boolean);
                response[key] = json!(boolean);
            }
            Mock::given(method("PUT"))
                .and(path(format!("/api/admin/{collection}/{kind}-1/features")))
                .and(header("authorization", "Bearer card-token"))
                .and(header("accept", "application/json"))
                .and(header("content-type", "application/json"))
                .and(body_json(body))
                .respond_with(ResponseTemplate::new(200).set_body_json(response.clone()))
                .expect(1)
                .mount(&server)
                .await;
            let selector = format!("--{kind}");
            let id = format!("{kind}-1");
            let result = run(
                &server,
                &[
                    kind,
                    "features",
                    "update",
                    &selector,
                    &id,
                    "--sync-members-from-auth",
                    &value,
                    "--shared-templates",
                    &value,
                    "--propagate-members-to-boards",
                    &value,
                ],
            )
            .await
            .unwrap();
            let data = json_data(&result);
            assert_eq!(
                data["submitted_fields"],
                json!([
                    "shared_templates",
                    "propagate_members_to_boards",
                    "sync_members_from_auth"
                ])
            );
            let fields = data[output_key].as_object().unwrap();
            assert_eq!(fields.len(), response.as_object().unwrap().len());
            for (wire, expected) in response.as_object().unwrap() {
                let stable = stable_field(kind, output_key, wire);
                assert_eq!(fields[&stable], *expected, "{wire}");
            }
            let human = render_success(OutputFormat::Human, &result);
            assert!(!human.contains('\u{1b}'));
            assert!(human.contains("display_name"));
            assert!(human.contains("modified_at"));
        }
        for (suffix, flag, stable) in fixtures::FEATURES {
            for boolean in [false, true] {
                Mock::given(method("PUT"))
                    .and(path(format!("/api/admin/{collection}/features")))
                    .and(header("authorization", "Bearer card-token"))
                    .and(header("accept", "application/json"))
                    .and(header("content-type", "application/json"))
                    .and(body_json(
                        json!({"field":format!("{kind}{suffix}"),"value":boolean}),
                    ))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json!({"updated":0})))
                    .expect(1)
                    .mount(&server)
                    .await;
                let result = run(
                    &server,
                    &[
                        kind,
                        "features",
                        "update-all",
                        "--field",
                        flag,
                        "--value",
                        &boolean.to_string(),
                        "--yes",
                    ],
                )
                .await
                .unwrap();
                assert_eq!(
                    json_data(&result),
                    json!({"field":stable,"value":boolean,"updated":0})
                );
            }
        }
    }
    assert_eq!(server.received_requests().await.unwrap().len(), 19);
}

fn stable_field(prefix: &str, resource: &str, wire: &str) -> String {
    if wire == "_id" {
        return format!("{resource}_id");
    }
    let name = wire.strip_prefix(prefix).unwrap_or(wire);
    let mut output = String::new();
    for (i, character) in name.chars().enumerate() {
        if character.is_uppercase() && i > 0 {
            output.push('_');
        }
        output.extend(character.to_lowercase());
    }
    output
}

#[tokio::test]
async fn individual_updates_encode_ids_and_preserve_omitted_request_fields() {
    for (kind, resource, mut document) in [
        ("org", "orgs", fixtures::organization()),
        ("team", "teams", fixtures::team()),
    ] {
        let server = MockServer::start().await;
        let id = "a/b?c#d% e";
        document["_id"] = json!(id);
        Mock::given(method("PUT"))
            .and(path(format!(
                "/api/admin/{resource}/a%2Fb%3Fc%23d%25%20e/features"
            )))
            .and(body_json(json!({format!("{kind}SharedTemplates"):false})))
            .respond_with(ResponseTemplate::new(200).set_body_json(document))
            .expect(1)
            .mount(&server)
            .await;
        run(
            &server,
            &[
                kind,
                "features",
                "update",
                &format!("--{kind}"),
                id,
                "--shared-templates",
                "false",
            ],
        )
        .await
        .unwrap();
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn every_response_field_is_strict_and_optional_fields_accept_null_or_omission() {
    for (kind, resource, document) in [
        ("org", "orgs", fixtures::organization()),
        ("team", "teams", fixtures::team()),
    ] {
        let server = MockServer::start().await;
        let c = super::client(&server);
        let token = super::status_token();
        let mut cases = Vec::new();
        for (wire, value) in document.as_object().unwrap() {
            let mut bad = document.clone();
            bad[wire] = if value.is_string() {
                json!(42)
            } else {
                json!("invalid")
            };
            cases.push(bad);
        }
        let mut unknown = document.clone();
        unknown["future"] = json!({"secret":"never-print"});
        cases.push(unknown);
        for wire in ["_id", "createdAt", "modifiedAt"] {
            let mut missing = document.clone();
            missing.as_object_mut().unwrap().remove(wire);
            cases.push(missing);
            let mut null = document.clone();
            null[wire] = Value::Null;
            cases.push(null);
        }
        for date in ["createdAt", "modifiedAt"] {
            let mut bad = document.clone();
            bad[date] = json!("not-a-date");
            cases.push(bad);
        }
        let mut wrong_id = document.clone();
        wrong_id["_id"] = json!("another-id");
        cases.push(wrong_id);
        cases.push(json!([]));
        for bad in cases {
            server.reset().await;
            Mock::given(method("PUT"))
                .respond_with(ResponseTemplate::new(200).set_body_json(bad.clone()))
                .mount(&server)
                .await;
            let result = if kind == "org" {
                c.update_organization_features(
                    "org-1",
                    &UpdateOrganizationFeatures::default(),
                    &token,
                )
                .await
                .map(|_| ())
            } else {
                c.update_team_features("team-1", &UpdateTeamFeatures::default(), &token)
                    .await
                    .map(|_| ())
            };
            assert!(
                matches!(
                    result,
                    Err(ClientError::Protocol {
                        success_status_received: true,
                        ..
                    })
                ),
                "{bad}: {result:?}"
            );
        }
        for null in [false, true] {
            let mut sparse = document.clone();
            for wire in document
                .as_object()
                .unwrap()
                .keys()
                .filter(|key| !["_id", "createdAt", "modifiedAt"].contains(&key.as_str()))
            {
                if null {
                    sparse[wire] = Value::Null;
                } else {
                    sparse.as_object_mut().unwrap().remove(wire);
                }
            }
            server.reset().await;
            Mock::given(method("PUT"))
                .and(path(format!("/api/admin/{resource}/{kind}-1/features")))
                .respond_with(ResponseTemplate::new(200).set_body_json(sparse))
                .mount(&server)
                .await;
            let result = run(
                &server,
                &[
                    kind,
                    "features",
                    "update",
                    &format!("--{kind}"),
                    &format!("{kind}-1"),
                    "--shared-templates",
                    "true",
                ],
            )
            .await
            .unwrap();
            let key = if kind == "org" {
                "organization"
            } else {
                "team"
            };
            assert!(json_data(&result)[key]["display_name"].is_null());
        }
        let summary = fixtures::summary(kind, &document);
        let mut list_cases = vec![json!({"unexpected":true}), json!([[]])];
        for (wire, value) in summary.as_object().unwrap() {
            let mut bad = summary.clone();
            bad[wire] = if value.is_boolean() {
                json!("false")
            } else {
                json!(12)
            };
            list_cases.push(json!([bad]));
        }
        for suffix in [
            "SharedTemplates",
            "PropagateMembersToBoards",
            "SyncMembersFromAuth",
        ] {
            let mut bad = summary.clone();
            bad.as_object_mut()
                .unwrap()
                .remove(&format!("{kind}{suffix}"));
            list_cases.push(json!([bad]));
        }
        let mut bad = summary.clone();
        bad["unmapped"] = json!(true);
        list_cases.push(json!([bad]));
        for bad in list_cases {
            server.reset().await;
            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(200).set_body_json(bad))
                .mount(&server)
                .await;
            assert_eq!(
                run(&server, &[kind, "list"]).await.unwrap_err().code(),
                ErrorCode::ProtocolError
            );
        }
    }
}

#[tokio::test]
async fn count_objects_reject_unknown_fields_invalid_counts_and_non_objects() {
    for kind in ["domain", "org", "team"] {
        let bulk = kind != "domain";
        let key = if bulk { "updated" } else { "count" };
        let base = if bulk {
            json!({"updated":1})
        } else {
            json!({"domain":"example.test","count":1})
        };
        let mut cases = vec![json!([])];
        for value in [json!(-1), json!(1.5), json!("2"), Value::Null, json!(true)] {
            let mut bad = base.clone();
            bad[key] = value;
            cases.push(bad);
        }
        let mut missing = base.clone();
        missing.as_object_mut().unwrap().remove(key);
        cases.push(missing);
        if !bulk {
            let mut missing_domain = base.clone();
            missing_domain.as_object_mut().unwrap().remove("domain");
            cases.push(missing_domain);
            for value in [json!(42), Value::Null] {
                let mut bad = base.clone();
                bad["domain"] = value;
                cases.push(bad);
            }
        }
        let mut unknown = base;
        unknown["future"] = json!(true);
        cases.push(unknown);
        for bad in cases {
            let server = MockServer::start().await;
            Mock::given(method(if bulk { "PUT" } else { "GET" }))
                .respond_with(ResponseTemplate::new(200).set_body_json(if bulk {
                    bad
                } else {
                    json!([bad])
                }))
                .mount(&server)
                .await;
            let args = if bulk {
                vec![
                    kind,
                    "features",
                    "update-all",
                    "--field",
                    "shared-templates",
                    "--value",
                    "true",
                    "--yes",
                ]
            } else {
                vec![kind, "list"]
            };
            let error = run(&server, &args).await.unwrap_err();
            assert_eq!(error.code(), ErrorCode::ProtocolError);
            if bulk {
                assert_eq!(error.details().outcome_unknown, Some(true));
            }
        }
    }
}

struct Answer(bool);
impl ConfirmationProvider for Answer {
    fn confirm(&self, request: &ConfirmationRequest) -> Result<bool, AppError> {
        assert!(request.prompt().contains("entire instance"));
        Ok(self.0)
    }
}

#[tokio::test]
async fn bulk_confirmation_cancels_without_http_and_acceptance_sends_one_request() {
    for kind in ["org", "team"] {
        let server = MockServer::start().await;
        let args = [
            kind,
            "features",
            "update-all",
            "--field",
            "shared-templates",
            "--value",
            "true",
        ];
        let result = run_with_confirmation(&server, &args, Answer(false))
            .await
            .unwrap();
        assert_eq!(
            json_data(&result)["operation"],
            format!(
                "admin_{}_features_update_all",
                if kind == "org" {
                    "organization"
                } else {
                    "team"
                }
            )
        );
        assert!(server.received_requests().await.unwrap().is_empty());
        assert_eq!(
            run(&server, &args).await.unwrap_err().code(),
            ErrorCode::InvalidInput
        );
        assert!(server.received_requests().await.unwrap().is_empty());
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"updated":2})))
            .expect(1)
            .mount(&server)
            .await;
        assert_eq!(
            json_data(
                &run_with_confirmation(&server, &args, Answer(true))
                    .await
                    .unwrap()
            )["updated"],
            2
        );
    }
}

#[tokio::test]
async fn embedded_errors_and_empty_updates_preserve_native_status_and_uncertainty() {
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
            "true",
        ],
        vec![
            "team",
            "features",
            "update",
            "--team",
            "team-1",
            "--shared-templates",
            "true",
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
        let server = MockServer::start().await;
        for (status, expected) in [
            (401, ErrorCode::AuthenticationRejected),
            (403, ErrorCode::PermissionDenied),
        ] {
            server.reset().await;
            Mock::given(method(if args.len()==2 {"GET"} else {"PUT"})).respond_with(ResponseTemplate::new(200).set_body_json(json!({"error":"Forbidden","statusCode":status,"reason":"card-token","isClientSafe":true,"message":"card-token","errorType":"Meteor.Error"}))).mount(&server).await;
            let error = run(&server, &args).await.unwrap_err();
            assert_eq!(error.code(), expected);
            assert_eq!(error.details().http_status, Some(200));
            assert_eq!(error.details().wekan_status_code, Some(status));
            assert!(
                !wekan_cli::output::render_error(OutputFormat::Json, &error).contains("card-token")
            );
        }
        server.reset().await;
        Mock::given(method(if args.len() == 2 { "GET" } else { "PUT" }))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        let error = run(&server, &args).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::ProtocolError);
        if args.len() > 2 {
            assert_eq!(error.details().outcome_unknown, Some(true));
        }
    }
}

#[tokio::test]
async fn large_admin_documents_and_lists_use_the_explicit_response_limit() {
    let server = MockServer::start().await;
    let text = "x".repeat(2 * 1024 * 1024);
    let mut document = fixtures::organization();
    document["orgLegalNotice"] = json!(text);
    Mock::given(method("PUT"))
        .and(path("/api/admin/orgs/org-1/features"))
        .respond_with(ResponseTemplate::new(200).set_body_json(document))
        .expect(1)
        .mount(&server)
        .await;
    let result = run(
        &server,
        &[
            "org",
            "features",
            "update",
            "--org",
            "org-1",
            "--shared-templates",
            "true",
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        json_data(&result)["organization"]["legal_notice"]
            .as_str()
            .unwrap()
            .len(),
        2 * 1024 * 1024
    );
    Mock::given(method("GET"))
        .and(path("/api/admin/domains"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"domain":text,"count":1}])))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(
        json_data(&run(&server, &["domain", "list"]).await.unwrap())["domains"][0]["domain"]
            .as_str()
            .unwrap()
            .len(),
        2 * 1024 * 1024
    );
    // Use a raw response: the mock server normalizes Content-Length to its body size.
    for update in [false, true] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = std::thread::spawn(move || {
            use std::io::{Read, Write};
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0; 8192];
            assert!(socket.read(&mut request).unwrap() > 0);
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 67108865\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
        });
        let client = super::client_from_url(&url);
        let token = super::status_token();
        let result = if update {
            client
                .update_organization_features(
                    "org-1",
                    &UpdateOrganizationFeatures::default(),
                    &token,
                )
                .await
                .map(|_| ())
        } else {
            client.admin_domains(&token).await.map(|_| ())
        };
        task.join().unwrap();
        assert!(
            matches!(
                result,
                Err(ClientError::ResponseTooLarge {
                    limit_bytes: 67_108_864,
                    ..
                })
            ),
            "{result:?}"
        );
    }
}

#[test]
fn coverage_rows_match_every_new_parser_option() {
    let coverage = include_str!("../../docs/api-coverage.md");
    let mut root = Cli::command();
    root.build();
    for (operation, command) in [
        ("get_admin_domains", vec!["domain", "list"]),
        ("get_admin_orgs", vec!["org", "list"]),
        ("get_admin_teams", vec!["team", "list"]),
        (
            "update_admin_org_features",
            vec!["org", "features", "update"],
        ),
        (
            "update_all_admin_org_features",
            vec!["org", "features", "update-all"],
        ),
        (
            "update_admin_team_features",
            vec!["team", "features", "update"],
        ),
        (
            "update_all_admin_team_features",
            vec!["team", "features", "update-all"],
        ),
    ] {
        let row = coverage
            .lines()
            .find(|line| line.contains(&format!("| `{operation}` |")))
            .unwrap();
        assert!(row.contains(&format!("wekan admin {}", command.join(" "))));
        assert!(row.ends_with("| ✓ |"));
        let mut leaf = root.find_subcommand("admin").unwrap();
        for segment in command {
            leaf = leaf.find_subcommand(segment).unwrap();
        }
        for arg in leaf.get_arguments() {
            if let Some(long) = arg.get_long() {
                assert!(
                    row.contains(&format!("--{long}")),
                    "missing {long} in {operation}"
                );
            }
        }
    }
    Cli::try_parse_from(["wekan", "admin", "org", "list"]).unwrap();
}
