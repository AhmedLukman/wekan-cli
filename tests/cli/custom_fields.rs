use crate::support::wekan;
use clap::Parser;
use wekan_cli::cli::Cli;
use wekan_cli::commands::{
    RootCommand, custom_field_items::CustomFieldItemCommand, custom_fields::CustomFieldCommand,
};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    let mut argv = vec!["wekan"];
    argv.extend_from_slice(args);
    argv.extend(["--board", "board-1"]);
    Cli::try_parse_from(argv)
}

#[test]
fn custom_field_cli_exposes_all_commands_and_exact_input_modes() {
    for field_type in [
        "text",
        "number",
        "date",
        "dropdown",
        "dropdownMultiSelect",
        "checkbox",
        "currency",
        "stringtemplate",
    ] {
        parse(&["custom-field", "create", "--name", "", "--type", field_type]).unwrap();
    }
    for args in [
        vec!["custom-field", "list"],
        vec!["custom-field", "get", " f "],
        vec!["custom-field", "update", "f"],
        vec![
            "custom-field",
            "item",
            "create",
            "--custom-field",
            "f",
            "--item",
            "",
            "--item",
            " N ",
        ],
        vec!["custom-field", "item", "update", "i", "--custom-field", "f"],
        vec![
            "card",
            "set-custom-field",
            "c",
            "--list",
            "l",
            "--custom-field",
            "f",
        ],
        vec![
            "card",
            "find-by-custom-field",
            "--custom-field",
            "f",
            "--value",
            " Ready/#? ",
        ],
    ] {
        parse(&args).unwrap();
    }
    for value in [
        "null",
        "[]",
        "{}",
        "false",
        "2.5",
        r#"["a",null,2,{"x":1}]"#,
    ] {
        parse(&[
            "custom-field",
            "item",
            "create",
            "--custom-field",
            "f",
            "--items",
            value,
        ])
        .unwrap();
        parse(&[
            "card",
            "set-custom-field",
            "c",
            "--list",
            "l",
            "--custom-field",
            "f",
            "--value-json",
            value,
        ])
        .unwrap();
    }
    parse(&["custom-field","update","f","--show-on-card","false","--automatically-on-card","true","--always-on-card","false","--show-label-on-mini-card","true","--show-sum-at-top-of-list","false","--settings",r#"{"currencyCode":"USD","dropdownItems":[{"_id":"i","name":"N"}],"stringtemplateFormat":"x","stringtemplateSeparator":" ","jiraTimeField":"original","jiraEstimateFieldId":"customfield_10001","jiraEstimateUnit":"points"}"#]).unwrap();
    for (family, verbs) in [
        (
            "custom-field",
            vec!["list", "get", "create", "update", "delete"],
        ),
        ("custom-field item", vec!["create", "update", "delete"]),
    ] {
        let output = wekan()
            .args(family.split_whitespace().chain(["--help"]))
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let help = String::from_utf8(output.stdout).unwrap();
        for verb in verbs {
            assert!(help.contains(verb));
        }
        let output = wekan()
            .args(family.split_whitespace().chain(["delete", "--help"]))
            .output()
            .unwrap();
        assert!(String::from_utf8(output.stdout).unwrap().contains("--yes"));
    }
}

#[test]
fn parser_rejects_conflicts_unknown_settings_and_missing_targeting() {
    for args in [
        vec!["custom-field", "create", "--name", "N"],
        vec!["custom-field", "create", "--name", "N", "--type", "bad"],
        vec![
            "custom-field",
            "create",
            "--name",
            "N",
            "--type",
            "text",
            "--always-on-card",
            "true",
        ],
        vec!["custom-field", "get", " "],
        vec!["custom-field", "item", "get", "i", "--custom-field", "f"],
        vec!["custom-field", "item", "list", "--custom-field", "f"],
        vec![
            "custom-field",
            "item",
            "create",
            "--custom-field",
            "f",
            "--item",
            "N",
            "--items",
            "[]",
        ],
        vec![
            "card",
            "set-custom-field",
            "c",
            "--list",
            "l",
            "--custom-field",
            "f",
            "--value",
            "N",
            "--value-json",
            "null",
        ],
        vec!["card", "find-by-custom-field", "--custom-field", "f"],
        vec!["custom-field", "list", "--yes"],
        vec![
            "custom-field",
            "item",
            "create",
            "--custom-field",
            "f",
            "--items",
            "invalid",
        ],
    ] {
        assert!(parse(&args).is_err(), "{args:?}");
    }
    for settings in [
        "null",
        "[]",
        r#"{"future":true}"#,
        r#"{"currencyCode":42}"#,
        r#"{"jiraTimeField":true}"#,
        r#"{"jiraEstimateFieldId":42}"#,
        r#"{"jiraEstimateUnit":[]}"#,
        r#"{"dropdownItems":false}"#,
        r#"{"dropdownItems":[{"_id":"i","name":"N","future":true}]}"#,
        r#"{"dropdownItems":[{"_id":"i"}]}"#,
    ] {
        assert!(
            parse(&["custom-field", "update", "f", "--settings", settings]).is_err(),
            "{settings}"
        );
    }
    assert!(Cli::try_parse_from(["wekan", "custom-field", "list"]).is_err());
    assert!(
        Cli::try_parse_from([
            "wekan",
            "custom-field",
            "item",
            "update",
            "i",
            "--board",
            "b"
        ])
        .is_err()
    );
}

#[test]
fn settings_accept_nullable_properties_for_creation_and_update() {
    for settings in [
        r#"{"currencyCode":null}"#,
        r#"{"dropdownItems":null}"#,
        r#"{"stringtemplateFormat":null}"#,
        r#"{"stringtemplateSeparator":null}"#,
        r#"{"jiraTimeField":null,"jiraEstimateFieldId":null,"jiraEstimateUnit":null}"#,
        r#"{"currencyCode":null,"dropdownItems":null,"stringtemplateFormat":null,"stringtemplateSeparator":null}"#,
    ] {
        parse(&[
            "custom-field",
            "create",
            "--name",
            "Nullable",
            "--type",
            "text",
            "--settings",
            settings,
        ])
        .unwrap();
        parse(&["custom-field", "update", "f", "--settings", settings]).unwrap();
    }
}

#[test]
fn dropdown_selectors_preserve_javascript_non_whitespace() {
    for item in ["option", "\u{85}option\u{85}", "\u{85}", "\u{85}.\u{85}"] {
        for verb in ["update", "delete"] {
            let parsed = parse(&[
                "custom-field",
                "item",
                verb,
                item,
                "--custom-field",
                "field-1",
            ])
            .unwrap();
            let RootCommand::CustomField(args) = parsed.command else {
                panic!("expected custom-field command")
            };
            let CustomFieldCommand::Item(args) = args.command else {
                panic!("expected custom-field item command")
            };
            let selector = match args.command {
                CustomFieldItemCommand::Update(args) => args.item_id,
                CustomFieldItemCommand::Delete(args) => args.item_id,
                CustomFieldItemCommand::Create(_) => panic!("expected update or delete"),
            };
            assert_eq!(selector, item, "{verb} {item:?}");
        }
    }
}

#[test]
fn dot_only_dropdown_ids_are_rejected_before_credentials_or_http() {
    for item in [
        ".",
        "..",
        " . ",
        " .. ",
        "\u{feff}.\u{feff}",
        "\u{feff}..\u{feff}",
    ] {
        for verb in ["update", "delete"] {
            let output = wekan()
                .args([
                    "--output",
                    "json",
                    "custom-field",
                    "item",
                    verb,
                    item,
                    "--board",
                    "board-1",
                    "--custom-field",
                    "field-1",
                ])
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(2), "{verb} {item:?}");
            assert!(output.stdout.is_empty());
            let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["error"]["code"], "invalid_input");
            assert!(
                error["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("dot-only")
            );
        }
    }
    for item in [".", "..", " . ", "\u{feff}..\u{feff}"] {
        let settings =
            serde_json::json!({"dropdownItems":[{"_id":item,"name":"Option"}]}).to_string();
        assert!(
            parse(&[
                "custom-field",
                "create",
                "--name",
                "Options",
                "--type",
                "dropdown",
                "--settings",
                &settings
            ])
            .is_err(),
            "{item:?}"
        );
        assert!(
            parse(&["custom-field", "update", "field-1", "--settings", &settings]).is_err(),
            "{item:?}"
        );
    }
    for item in ["...", "%2E", "./option", "a/../b"] {
        let settings =
            serde_json::json!({"dropdownItems":[{"_id":item,"name":"Option"}]}).to_string();
        parse(&[
            "custom-field",
            "create",
            "--name",
            "Options",
            "--type",
            "dropdown",
            "--settings",
            &settings,
        ])
        .unwrap();
        parse(&[
            "custom-field",
            "item",
            "update",
            item,
            "--custom-field",
            "field-1",
        ])
        .unwrap();
    }
    // Wekan's JavaScript trimming preserves U+0085 in supplied settings.
    let settings =
        serde_json::json!({"dropdownItems":[{"_id":"\u{85}.\u{85}","name":"Option"}]}).to_string();
    parse(&[
        "custom-field",
        "create",
        "--name",
        "Options",
        "--type",
        "dropdown",
        "--settings",
        &settings,
    ])
    .unwrap();
}

#[test]
fn lookup_rejects_unrepresentable_values_before_target_or_credential_resolution() {
    for value in ["", ".", ".."] {
        let output = wekan()
            .args([
                "--output",
                "json",
                "card",
                "find-by-custom-field",
                "--board",
                "board-1",
                "--custom-field",
                "field-1",
                "--value",
                value,
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], "invalid_input");
    }
    for value in [" . ", "...", "%2E", "./value", "a/../b"] {
        parse(&[
            "card",
            "find-by-custom-field",
            "--custom-field",
            "field-1",
            "--value",
            value,
        ])
        .unwrap();
    }
}
