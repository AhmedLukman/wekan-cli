use std::ffi::OsString;

use clap::ValueEnum;
use serde::Serialize;

use crate::error::AppError;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
}

impl OutputFormat {
    pub fn detect_from_args(args: &[OsString]) -> Self {
        let mut iter = args.iter().filter_map(|arg| arg.to_str());
        while let Some(arg) = iter.next() {
            if arg == "--output" && iter.next().is_some_and(|value| value == "json") {
                return Self::Json;
            }
            if arg == "--output=json" {
                return Self::Json;
            }
        }
        Self::Human
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum CommandSuccess {
    Registration(RegistrationSuccess),
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct RegistrationSuccess {
    pub server: String,
    pub user_id: String,
    pub token_expires: String,
    pub credential_stored: bool,
}

#[derive(Serialize)]
struct SuccessEnvelope<'a, T> {
    ok: bool,
    data: &'a T,
}

#[derive(Serialize)]
struct ErrorEnvelope<'a> {
    ok: bool,
    error: ErrorBody<'a>,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'static str,
    message: &'a str,
    details: &'a crate::error::ErrorDetails,
}

pub fn render_success(format: OutputFormat, success: &CommandSuccess) -> String {
    match (format, success) {
        (OutputFormat::Human, CommandSuccess::Registration(data)) => format!(
            "Registered user {} on {}\nToken expires: {}\nCredentials stored securely.",
            escape_terminal_controls(&data.user_id),
            escape_terminal_controls(&data.server),
            escape_terminal_controls(&data.token_expires)
        ),
        (OutputFormat::Json, CommandSuccess::Registration(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("registration success is always serializable")
        }
    }
}

fn escape_terminal_controls(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control() {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
    }
    escaped
}

pub fn render_error(format: OutputFormat, error: &AppError) -> String {
    match format {
        OutputFormat::Human => format!(
            "Error [{}]: {}",
            error.code().as_str(),
            error.message().trim()
        ),
        OutputFormat::Json => serde_json::to_string(&ErrorEnvelope {
            ok: false,
            error: ErrorBody {
                code: error.code().as_str(),
                message: error.message(),
                details: error.details(),
            },
        })
        .expect("application errors are always serializable"),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{CommandSuccess, OutputFormat, RegistrationSuccess, render_error, render_success};
    use crate::error::AppError;

    fn success() -> CommandSuccess {
        CommandSuccess::Registration(RegistrationSuccess {
            server: "https://wekan.example/".to_owned(),
            user_id: "user-1".to_owned(),
            token_expires: "2030-01-02T03:04:05Z".to_owned(),
            credential_stored: true,
        })
    }

    #[test]
    fn renders_the_stable_json_success_envelope() {
        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &success())).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["data"]["user_id"], "user-1");
        assert_eq!(value["data"]["credential_stored"], true);
        assert!(value["data"].get("token").is_none());
    }

    #[test]
    fn human_success_reports_metadata_without_a_token() {
        let rendered = render_success(OutputFormat::Human, &success());
        assert!(rendered.contains("user-1"));
        assert!(rendered.contains("https://wekan.example/"));
        assert!(rendered.contains("2030-01-02T03:04:05Z"));
        assert!(rendered.contains("Credentials stored securely."));
        assert!(!rendered.contains("token\":\""));
    }

    #[test]
    fn human_success_escapes_terminal_control_sequences() {
        let success = CommandSuccess::Registration(RegistrationSuccess {
            server: "https://wekan.example/".to_owned(),
            user_id: "user\u{1b}]52;c;clipboard\u{7}\nnext-line".to_owned(),
            token_expires: "2030-01-02T03:04:05Z".to_owned(),
            credential_stored: true,
        });

        let rendered = render_success(OutputFormat::Human, &success);

        assert!(!rendered.contains('\u{1b}'));
        assert!(!rendered.contains('\u{7}'));
        assert!(rendered.contains(r"user\u{1b}]52;c;clipboard\u{7}\nnext-line"));
    }

    #[test]
    fn renders_the_stable_json_error_envelope() {
        let value: serde_json::Value = serde_json::from_str(&render_error(
            OutputFormat::Json,
            &AppError::configuration("missing server"),
        ))
        .unwrap();
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "configuration_error");
        assert_eq!(value["error"]["details"], serde_json::json!({}));
    }

    #[test]
    fn detects_json_output_before_clap_parsing() {
        assert_eq!(
            OutputFormat::detect_from_args(&[
                OsString::from("wekan"),
                OsString::from("auth"),
                OsString::from("--output=json"),
                OsString::from("register"),
            ]),
            OutputFormat::Json
        );
    }
}
