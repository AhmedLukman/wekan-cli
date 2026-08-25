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

    pub fn and_secret(mut self, secret: &'a SecretString) -> Self {
        self.secrets.push(secret);
        self
    }

    pub fn redact(&self, value: &str) -> String {
        let mut secrets: Vec<_> = self
            .secrets
            .iter()
            .map(|secret| secret.expose_secret())
            .filter(|secret| !secret.is_empty())
            .collect();
        secrets.sort_unstable_by_key(|secret| std::cmp::Reverse(secret.len()));

        secrets.into_iter().fold(value.to_owned(), |text, secret| {
            text.replace(secret, "[REDACTED]")
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

    #[test]
    fn redacts_overlapping_secrets_without_leaking_a_suffix() {
        let password = SecretString::from("123".to_owned());
        let code = SecretString::from("123456".to_owned());
        let redactor = Redactor::with_secret(&password).and_secret(&code);

        assert_eq!(
            redactor.redact("code 123456 was rejected"),
            "code [REDACTED] was rejected"
        );
    }
}
