use secrecy::{ExposeSecret, SecretString};

#[derive(Default)]
pub struct Redactor<'a> {
    secrets: Vec<&'a SecretString>,
}

impl<'a> Redactor<'a> {
    pub fn with_secret(secret: &'a SecretString) -> Self {
        Self {
            secrets: vec![secret],
        }
    }

    pub fn redact(&self, value: &str) -> String {
        self.secrets.iter().fold(value.to_owned(), |text, secret| {
            let secret = secret.expose_secret();
            if secret.is_empty() {
                text
            } else {
                text.replace(secret, "[REDACTED]")
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use secrecy::SecretString;

    use super::Redactor;

    #[test]
    fn redacts_all_occurrences_without_exposing_debug_values() {
        let secret = SecretString::from("do not print me".to_owned());
        let redactor = Redactor::with_secret(&secret);
        let result = redactor.redact("do not print me / do not print me");

        assert_eq!(result, "[REDACTED] / [REDACTED]");
        assert!(!format!("{secret:?}").contains("do not print me"));
    }
}
