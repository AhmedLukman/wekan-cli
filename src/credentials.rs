mod keyring;
mod prompt;
mod stdin;

use std::io::{self, IsTerminal};

use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub use keyring::KeyringCredentialStore;

const PASSWORD_STDIN_GUIDANCE: &str = "pass --password-stdin to read the password from stdin";
const TWO_FACTOR_STDIN_GUIDANCE: &str = "replace --code with --password-stdin --code-stdin to read the password and two-factor code from stdin";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoginSecretMode {
    PromptPassword,
    PromptPasswordAndCode,
    StdinPassword,
    StdinPasswordAndCode,
}

#[derive(Debug)]
pub struct LoginSecrets {
    password: SecretString,
    code: Option<SecretString>,
}

impl LoginSecrets {
    pub fn new(password: SecretString, code: Option<SecretString>) -> Self {
        Self { password, code }
    }

    pub fn into_parts(self) -> (SecretString, Option<SecretString>) {
        (self.password, self.code)
    }
}

pub trait SecretInputProvider: Send + Sync {
    fn read_registration_password(
        &self,
        from_stdin: bool,
    ) -> Result<SecretString, crate::error::AppError>;

    fn read_login_secrets(
        &self,
        mode: LoginSecretMode,
    ) -> Result<LoginSecrets, crate::error::AppError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemSecretInputProvider;

impl SecretInputProvider for SystemSecretInputProvider {
    fn read_registration_password(
        &self,
        from_stdin: bool,
    ) -> Result<SecretString, crate::error::AppError> {
        if from_stdin {
            return stdin::read_password_line(io::stdin().lock());
        }
        require_terminal(PASSWORD_STDIN_GUIDANCE)?;

        prompt::read_confirmed_password()
    }

    fn read_login_secrets(
        &self,
        mode: LoginSecretMode,
    ) -> Result<LoginSecrets, crate::error::AppError> {
        match mode {
            LoginSecretMode::PromptPassword => {
                require_terminal(PASSWORD_STDIN_GUIDANCE)?;
                let password = prompt::read_password()?;
                Ok(LoginSecrets::new(password, None))
            }
            LoginSecretMode::PromptPasswordAndCode => {
                require_terminal(TWO_FACTOR_STDIN_GUIDANCE)?;
                let password = prompt::read_password()?;
                let code = prompt::read_code()?;
                Ok(LoginSecrets::new(password, Some(code)))
            }
            LoginSecretMode::StdinPassword | LoginSecretMode::StdinPasswordAndCode => {
                let mut input = io::stdin().lock();
                let password = stdin::read_password_line(&mut input)?;
                let code = if mode == LoginSecretMode::StdinPasswordAndCode {
                    Some(stdin::read_code_line(&mut input)?)
                } else {
                    None
                };
                Ok(LoginSecrets::new(password, code))
            }
        }
    }
}

fn require_terminal(guidance: &str) -> Result<(), crate::error::AppError> {
    if io::stdin().is_terminal() {
        Ok(())
    } else {
        Err(crate::error::AppError::invalid_input(format!(
            "standard input is not a terminal; {guidance}"
        )))
    }
}

fn validate_password(
    password: String,
    confirmation: Option<String>,
) -> Result<SecretString, crate::error::AppError> {
    if password.is_empty() {
        return Err(crate::error::AppError::invalid_input(
            "password must not be empty",
        ));
    }
    if confirmation.is_some_and(|confirmation| password != confirmation) {
        return Err(crate::error::AppError::invalid_input(
            "password and confirmation do not match",
        ));
    }
    Ok(SecretString::from(password))
}

fn validate_code(code: String) -> Result<SecretString, crate::error::AppError> {
    if code.is_empty() {
        return Err(crate::error::AppError::invalid_input(
            "two-factor code must not be empty",
        ));
    }
    Ok(SecretString::from(code))
}

pub trait CredentialStore: Send + Sync {
    fn check_available(&self, account: &str) -> Result<(), CredentialError>;
    fn save(&self, account: &str, record: &CredentialRecord) -> Result<(), CredentialError>;
}

#[derive(Debug)]
pub struct CredentialRecord {
    server_url: String,
    user_id: String,
    token: SecretString,
    token_expires: OffsetDateTime,
}

impl CredentialRecord {
    pub fn new(
        server_url: String,
        user_id: String,
        token: SecretString,
        token_expires: OffsetDateTime,
    ) -> Self {
        Self {
            server_url,
            user_id,
            token,
            token_expires,
        }
    }

    pub fn server_url(&self) -> &str {
        &self.server_url
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn token(&self) -> &SecretString {
        &self.token
    }

    pub const fn token_expires(&self) -> OffsetDateTime {
        self.token_expires
    }

    fn encode(&self) -> Result<Vec<u8>, CredentialError> {
        let token_expires = self
            .token_expires
            .format(&Rfc3339)
            .map_err(CredentialError::Timestamp)?;
        let stored = StoredCredential {
            version: 1,
            server_url: &self.server_url,
            user_id: &self.user_id,
            token: self.token.expose_secret(),
            token_expires: &token_expires,
        };
        serde_json::to_vec(&stored).map_err(CredentialError::Serialize)
    }
}

#[derive(Serialize)]
struct StoredCredential<'a> {
    version: u8,
    server_url: &'a str,
    user_id: &'a str,
    token: &'a str,
    token_expires: &'a str,
}

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("the operating-system credential store is unavailable: {0}")]
    Unavailable(String),
    #[error("the credential could not be stored: {0}")]
    Store(String),
    #[error("the credential record could not be serialized")]
    Serialize(#[source] serde_json::Error),
    #[error("the credential expiry could not be formatted")]
    Timestamp(#[source] time::error::Format),
}

#[cfg(test)]
mod tests {
    use secrecy::SecretString;
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};

    use super::{
        CredentialRecord, PASSWORD_STDIN_GUIDANCE, TWO_FACTOR_STDIN_GUIDANCE, validate_password,
    };

    #[test]
    fn interactive_confirmation_must_match() {
        let error = validate_password("first".to_owned(), Some("second".to_owned()))
            .expect_err("mismatched confirmation must fail");
        assert_eq!(error.message(), "password and confirmation do not match");
    }

    #[test]
    fn login_password_does_not_require_confirmation() {
        assert!(validate_password("password".to_owned(), None).is_ok());
    }

    #[test]
    fn non_terminal_guidance_distinguishes_two_factor_login() {
        assert_eq!(
            PASSWORD_STDIN_GUIDANCE,
            "pass --password-stdin to read the password from stdin"
        );
        assert_eq!(
            TWO_FACTOR_STDIN_GUIDANCE,
            "replace --code with --password-stdin --code-stdin to read the password and two-factor code from stdin"
        );
    }

    #[test]
    fn encodes_a_versioned_credential_record() {
        let expiry = OffsetDateTime::parse("2030-01-02T03:04:05Z", &Rfc3339).unwrap();
        let record = CredentialRecord::new(
            "https://wekan.example/".to_owned(),
            "user-1".to_owned(),
            SecretString::from("server-token".to_owned()),
            expiry,
        );

        let value: serde_json::Value = serde_json::from_slice(&record.encode().unwrap()).unwrap();
        assert_eq!(value["version"], 1);
        assert_eq!(value["server_url"], "https://wekan.example/");
        assert_eq!(value["user_id"], "user-1");
        assert_eq!(value["token"], "server-token");
        assert_eq!(value["token_expires"], "2030-01-02T03:04:05Z");
        assert!(!format!("{record:?}").contains("server-token"));
    }
}
