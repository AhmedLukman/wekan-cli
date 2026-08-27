use std::ffi::OsString;

use clap::ValueEnum;
use serde::Serialize;

use crate::command_result::{
    AuthStatusSuccess, AuthSuccess, CommandSuccess, LogoutScope, LogoutSuccess, ProfileItem,
    ProfileListSuccess, ProfileRemoveSuccess,
};
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
        (OutputFormat::Human, CommandSuccess::Cancelled(_)) => {
            "Cancelled; no changes made.".to_owned()
        }
        (OutputFormat::Json, CommandSuccess::Cancelled(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("cancellation success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::Registration(data)) => {
            render_auth_success("Registered user", data)
        }
        (OutputFormat::Json, CommandSuccess::Registration(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("registration success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::Login(data)) => {
            render_auth_success("Logged in user", data)
        }
        (OutputFormat::Json, CommandSuccess::Login(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("login success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::Logout(data)) => render_logout(data),
        (OutputFormat::Json, CommandSuccess::Logout(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("logout success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::AuthStatus(data)) => render_auth_status(data),
        (OutputFormat::Json, CommandSuccess::AuthStatus(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("authentication status success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::ProfileAdded(data)) => {
            render_profile_item("Added profile", data)
        }
        (OutputFormat::Json, CommandSuccess::ProfileAdded(data))
        | (OutputFormat::Json, CommandSuccess::ProfileShown(data))
        | (OutputFormat::Json, CommandSuccess::ProfileUsed(data))
        | (OutputFormat::Json, CommandSuccess::ProfileUpdated(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("profile success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::ProfileList(data)) => render_profile_list(data),
        (OutputFormat::Json, CommandSuccess::ProfileList(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("profile list success is always serializable")
        }
        (OutputFormat::Human, CommandSuccess::ProfileShown(data)) => {
            render_profile_item("Profile", data)
        }
        (OutputFormat::Human, CommandSuccess::ProfileUsed(data)) => {
            render_profile_item("Using profile", data)
        }
        (OutputFormat::Human, CommandSuccess::ProfileUpdated(data)) => {
            render_profile_item("Updated profile", data)
        }
        (OutputFormat::Human, CommandSuccess::ProfileRemoved(data)) => render_profile_removed(data),
        (OutputFormat::Json, CommandSuccess::ProfileRemoved(data)) => {
            serde_json::to_string(&SuccessEnvelope { ok: true, data })
                .expect("profile removal success is always serializable")
        }
    }
}

fn render_auth_success(action: &str, data: &AuthSuccess) -> String {
    let mut lines = vec![format!(
        "{action} {} on {}",
        escape_terminal_controls(&data.user_id),
        escape_terminal_controls(&data.server)
    )];
    lines.push(format!(
        "Profile: {}",
        escape_terminal_controls(&data.profile)
    ));
    lines.push(format!(
        "Profile created: {}",
        if data.profile_created { "yes" } else { "no" }
    ));
    lines.push(format!(
        "Profile active: {}",
        if data.profile_active { "yes" } else { "no" }
    ));
    lines.push(format!(
        "Token expires: {}",
        escape_terminal_controls(&data.token_expires)
    ));
    lines.push("Credentials stored securely.".to_owned());
    lines.join("\n")
}

fn render_profile_item(action: &str, data: &ProfileItem) -> String {
    format!(
        "{action}: {}\nServer: {}\nActive: {}",
        escape_terminal_controls(&data.name),
        escape_terminal_controls(&data.server),
        if data.active { "yes" } else { "no" }
    )
}

fn render_profile_list(data: &ProfileListSuccess) -> String {
    if data.profiles.is_empty() {
        return "No profiles configured.".to_owned();
    }
    let name_width = data
        .profiles
        .iter()
        .map(|profile| profile.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let server_width = data
        .profiles
        .iter()
        .map(|profile| profile.server.len())
        .max()
        .unwrap_or(6)
        .max(6);
    let mut lines = vec![format!(
        "{:<name_width$}  {:<server_width$}  ACTIVE",
        "NAME", "SERVER"
    )];
    lines.extend(data.profiles.iter().map(|profile| {
        format!(
            "{:<name_width$}  {:<server_width$}  {}",
            escape_terminal_controls(&profile.name),
            escape_terminal_controls(&profile.server),
            if profile.active { "yes" } else { "no" }
        )
    }));
    lines.join("\n")
}

fn render_profile_removed(data: &ProfileRemoveSuccess) -> String {
    let mut message = format!(
        "Removed profile: {}\nServer: {}",
        escape_terminal_controls(&data.name),
        escape_terminal_controls(&data.server)
    );
    if data.active_profile.is_none() {
        message.push_str("\nNo profile is active.");
    }
    message
}

fn render_logout(data: &LogoutSuccess) -> String {
    let server = escape_terminal_controls(&data.server);
    let mut rendered = match data.logout_scope {
        LogoutScope::CurrentToken => format!(
            "Logged out from {server}.\nCurrent login token revoked.\n{}",
            render_local_credential_state(data)
        ),
        LogoutScope::AllTokens => format!(
            "Logged out from {server}.\nAll login tokens revoked.\n{}",
            render_local_credential_state(data)
        ),
        LogoutScope::LocalOnly => format!(
            "{} for {server}.\nNo Wekan login tokens were revoked.",
            if data.local_credential_removed {
                "Removed the stored credential"
            } else {
                "No stored credential was present"
            }
        ),
    };
    let first_newline = rendered.find('\n').unwrap_or(rendered.len());
    rendered.insert_str(
        first_newline,
        &format!("\nProfile: {}", escape_terminal_controls(&data.profile)),
    );
    rendered
}

fn render_local_credential_state(data: &LogoutSuccess) -> &'static str {
    if data.local_credential_removed {
        "Stored credential removed."
    } else if data.credential_stored {
        match data.logout_scope {
            LogoutScope::AllTokens => {
                "A concurrently changed stored credential was preserved; verify it before use."
            }
            LogoutScope::CurrentToken | LogoutScope::LocalOnly => {
                "A newer stored credential was preserved."
            }
        }
    } else {
        "Stored credential was already absent."
    }
}

fn render_auth_status(data: &AuthStatusSuccess) -> String {
    let identity = data.user.username.as_deref().unwrap_or(&data.user.user_id);
    let mut lines = vec![
        format!(
            "Authenticated as {} on {}",
            escape_terminal_controls(identity),
            escape_terminal_controls(&data.server)
        ),
        format!("User ID: {}", escape_terminal_controls(&data.user.user_id)),
    ];

    lines.push(format!(
        "Profile: {}",
        escape_terminal_controls(&data.profile)
    ));

    if let Some(full_name) = &data.user.full_name {
        lines.push(format!(
            "Full name: {}",
            escape_terminal_controls(full_name)
        ));
    }
    if let Some(is_admin) = data.user.is_admin {
        lines.push(format!(
            "Administrator: {}",
            if is_admin { "yes" } else { "no" }
        ));
    }
    if !data.user.emails.is_empty() {
        let emails = data
            .user
            .emails
            .iter()
            .map(|email| {
                let address = email.address.as_deref().unwrap_or("unknown address");
                let verification = match email.verified {
                    Some(true) => "verified",
                    Some(false) => "unverified",
                    None => "verification unknown",
                };
                format!("{} ({verification})", escape_terminal_controls(address))
            })
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("Emails: {emails}"));
    }
    lines.push(format!(
        "Token expires: {}",
        escape_terminal_controls(&data.token_expires)
    ));
    lines.push("Credentials stored securely.".to_owned());
    lines.join("\n")
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

    use super::{OutputFormat, render_error, render_success};
    use crate::{
        command_result::{
            AuthStatusEmail, AuthStatusSuccess, AuthStatusUser, AuthSuccess, CancellationSuccess,
            CommandSuccess, DestructiveOperation, LogoutScope, LogoutSuccess, ProfileItem,
            ProfileListSuccess, ProfileRemoveSuccess,
        },
        error::AppError,
    };

    fn success() -> CommandSuccess {
        CommandSuccess::Registration(AuthSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            user_id: "user-1".to_owned(),
            token_expires: "2030-01-02T03:04:05Z".to_owned(),
            credential_stored: true,
            profile_created: true,
            profile_active: true,
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
    fn cancellation_has_stable_human_and_json_output() {
        let cancellation = CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::ProfileRemove,
        ));

        assert_eq!(
            render_success(OutputFormat::Human, &cancellation),
            "Cancelled; no changes made."
        );
        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &cancellation)).unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(value["data"]["cancelled"], true);
        assert_eq!(value["data"]["operation"], "profile_remove");
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
        let success = CommandSuccess::Registration(AuthSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            user_id: "user\u{1b}]52;c;clipboard\u{7}\nnext-line".to_owned(),
            token_expires: "2030-01-02T03:04:05Z".to_owned(),
            credential_stored: true,
            profile_created: false,
            profile_active: true,
        });

        let rendered = render_success(OutputFormat::Human, &success);

        assert!(!rendered.contains('\u{1b}'));
        assert!(!rendered.contains('\u{7}'));
        assert!(rendered.contains(r"user\u{1b}]52;c;clipboard\u{7}\nnext-line"));
    }

    #[test]
    fn login_success_uses_the_same_secret_free_json_shape() {
        let success = CommandSuccess::Login(AuthSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            user_id: "user-1".to_owned(),
            token_expires: "2030-01-02T03:04:05Z".to_owned(),
            credential_stored: true,
            profile_created: false,
            profile_active: true,
        });

        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &success)).unwrap();
        assert_eq!(value["data"]["user_id"], "user-1");
        assert!(value["data"].get("token").is_none());
        assert!(render_success(OutputFormat::Human, &success).starts_with("Logged in user"));
    }

    #[test]
    fn logout_success_reports_scope_and_local_credential_state() {
        let success = CommandSuccess::Logout(LogoutSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            logout_scope: LogoutScope::AllTokens,
            remote_logout_completed: true,
            credential_stored: false,
            local_credential_removed: true,
        });

        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &success)).unwrap();
        assert_eq!(value["data"]["logout_scope"], "all_tokens");
        assert_eq!(value["data"]["remote_logout_completed"], true);
        assert_eq!(value["data"]["credential_stored"], false);
        assert_eq!(value["data"]["local_credential_removed"], true);
        assert!(value["data"].get("token").is_none());
        assert!(render_success(OutputFormat::Human, &success).contains("All login tokens revoked"));
    }

    #[test]
    fn local_only_output_warns_that_remote_tokens_were_not_revoked() {
        let success = CommandSuccess::Logout(LogoutSuccess {
            server: "https://wekan.example/\u{1b}]52;c;clipboard\u{7}".to_owned(),
            profile: "default".to_owned(),
            logout_scope: LogoutScope::LocalOnly,
            remote_logout_completed: false,
            credential_stored: false,
            local_credential_removed: true,
        });

        let rendered = render_success(OutputFormat::Human, &success);
        assert!(rendered.contains("No Wekan login tokens were revoked"));
        assert!(!rendered.contains('\u{1b}'));
        assert!(!rendered.contains('\u{7}'));
    }

    #[test]
    fn local_only_output_distinguishes_an_already_absent_credential() {
        let success = CommandSuccess::Logout(LogoutSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            logout_scope: LogoutScope::LocalOnly,
            remote_logout_completed: false,
            credential_stored: false,
            local_credential_removed: false,
        });

        let rendered = render_success(OutputFormat::Human, &success);
        assert!(rendered.contains("No stored credential was present"));
        assert!(rendered.contains("No Wekan login tokens were revoked"));
    }

    #[test]
    fn remote_output_reports_that_a_newer_credential_was_preserved() {
        let success = CommandSuccess::Logout(LogoutSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            logout_scope: LogoutScope::CurrentToken,
            remote_logout_completed: true,
            credential_stored: true,
            local_credential_removed: false,
        });

        assert!(
            render_success(OutputFormat::Human, &success)
                .contains("A newer stored credential was preserved")
        );
    }

    #[test]
    fn all_tokens_output_requires_verification_of_a_changed_credential() {
        let success = CommandSuccess::Logout(LogoutSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            logout_scope: LogoutScope::AllTokens,
            remote_logout_completed: true,
            credential_stored: true,
            local_credential_removed: false,
        });

        assert!(render_success(OutputFormat::Human, &success).contains("verify it before use"));
    }

    #[test]
    fn status_success_uses_the_stable_nested_profile_shape() {
        let success = CommandSuccess::AuthStatus(AuthStatusSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            authenticated: true,
            token_expires: "2030-01-02T03:04:05Z".to_owned(),
            credential_stored: true,
            user: AuthStatusUser {
                user_id: "user-1".to_owned(),
                username: Some("alice".to_owned()),
                full_name: None,
                is_admin: Some(false),
                emails: vec![AuthStatusEmail {
                    address: Some("alice@example.com".to_owned()),
                    verified: None,
                }],
            },
        });

        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &success)).unwrap();
        assert_eq!(value["data"]["authenticated"], true);
        assert_eq!(value["data"]["user"]["user_id"], "user-1");
        assert_eq!(value["data"]["user"]["full_name"], serde_json::Value::Null);
        assert_eq!(
            value["data"]["user"]["emails"][0]["verified"],
            serde_json::Value::Null
        );
        assert!(value["data"].get("token").is_none());
    }

    #[test]
    fn human_status_omits_absent_profile_lines_and_escapes_controls() {
        let success = CommandSuccess::AuthStatus(AuthStatusSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "default".to_owned(),
            authenticated: true,
            token_expires: "2030-01-02T03:04:05Z".to_owned(),
            credential_stored: true,
            user: AuthStatusUser {
                user_id: "user-1".to_owned(),
                username: Some("alice\u{1b}]52;c;clipboard\u{7}".to_owned()),
                full_name: None,
                is_admin: None,
                emails: Vec::new(),
            },
        });

        let rendered = render_success(OutputFormat::Human, &success);
        assert!(!rendered.contains('\u{1b}'));
        assert!(!rendered.contains("Full name:"));
        assert!(!rendered.contains("Administrator:"));
        assert!(!rendered.contains("Emails:"));
        assert!(rendered.contains("Credentials stored securely."));
    }

    #[test]
    fn profile_list_output_is_stable_and_human_readable() {
        let success = CommandSuccess::ProfileList(ProfileListSuccess {
            active_profile: Some("local".to_owned()),
            profiles: vec![
                ProfileItem {
                    name: "local".to_owned(),
                    server: "http://localhost:3000/".to_owned(),
                    active: true,
                },
                ProfileItem {
                    name: "work".to_owned(),
                    server: "https://wekan.example/".to_owned(),
                    active: false,
                },
            ],
        });
        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &success)).unwrap();
        assert_eq!(value["data"]["active_profile"], "local");
        assert_eq!(value["data"]["profiles"][0]["active"], true);
        let human = render_success(OutputFormat::Human, &success);
        assert_eq!(
            human
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .collect::<Vec<_>>(),
            ["NAME", "SERVER", "ACTIVE"]
        );
        assert!(human.contains("http://localhost:3000/"));
    }

    #[test]
    fn empty_profile_list_and_removal_report_the_result() {
        let empty = CommandSuccess::ProfileList(ProfileListSuccess {
            active_profile: None,
            profiles: Vec::new(),
        });
        assert_eq!(
            render_success(OutputFormat::Human, &empty),
            "No profiles configured."
        );

        let removed = CommandSuccess::ProfileRemoved(ProfileRemoveSuccess {
            name: "local".to_owned(),
            server: "http://localhost:3000/".to_owned(),
            removed: true,
            active_profile: None,
        });
        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &removed)).unwrap();
        assert_eq!(value["data"]["removed"], true);
        assert_eq!(value["data"]["active_profile"], serde_json::Value::Null);
        assert!(render_success(OutputFormat::Human, &removed).contains("No profile is active"));
    }

    #[test]
    fn auth_output_identifies_profile_and_initialization_state() {
        let named = CommandSuccess::Login(AuthSuccess {
            server: "https://wekan.example/".to_owned(),
            profile: "work".to_owned(),
            user_id: "user-1".to_owned(),
            token_expires: "2030-01-02T03:04:05Z".to_owned(),
            credential_stored: true,
            profile_created: true,
            profile_active: false,
        });
        let value: serde_json::Value =
            serde_json::from_str(&render_success(OutputFormat::Json, &named)).unwrap();
        assert_eq!(value["data"]["profile"], "work");
        assert_eq!(value["data"]["profile_created"], true);
        assert_eq!(value["data"]["profile_active"], false);
        assert!(render_success(OutputFormat::Human, &named).contains("Profile: work"));
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
