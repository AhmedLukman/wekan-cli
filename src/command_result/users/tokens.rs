use secrecy::{ExposeSecret, SecretString};
use serde::{Serialize, Serializer};

#[derive(Debug, Serialize)]
pub struct UserTokenCreateSuccess {
    pub user_id: String,
    #[serde(serialize_with = "serialize_issued_token")]
    auth_token: SecretString,
    pub creation_requested: bool,
    pub mutation_confirmed: bool,
}

impl UserTokenCreateSuccess {
    pub fn new(user_id: String, auth_token: SecretString) -> Self {
        Self {
            user_id,
            auth_token,
            creation_requested: true,
            mutation_confirmed: false,
        }
    }

    pub fn auth_token(&self) -> &SecretString {
        &self.auth_token
    }
}

impl PartialEq for UserTokenCreateSuccess {
    fn eq(&self, other: &Self) -> bool {
        self.user_id == other.user_id
            && self.auth_token.expose_secret() == other.auth_token.expose_secret()
            && self.creation_requested == other.creation_requested
            && self.mutation_confirmed == other.mutation_confirmed
    }
}
impl Eq for UserTokenCreateSuccess {}

fn serialize_issued_token<S: Serializer>(token: &SecretString, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(token.expose_secret())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserTokenDeleteScope {
    Single,
    All,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserTokenDeleteSuccess {
    pub user_id: String,
    pub scope: UserTokenDeleteScope,
    pub message: String,
    pub deletion_requested: bool,
    pub mutation_confirmed: bool,
}
