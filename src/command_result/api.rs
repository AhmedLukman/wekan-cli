use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Eq, PartialEq)]
pub struct ApiResponseSuccess {
    pub profile: String,
    pub unsafe_method: bool,
    pub authenticated: bool,
    pub response: crate::client::RawApiResponse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RawValueEncoding {
    Text,
    Base64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RawResponseHeader {
    pub name: String,
    pub encoding: RawValueEncoding,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "encoding", rename_all = "snake_case")]
pub enum RawResponseBody {
    Json { value: Value },
    Text { value: String },
    Base64 { value: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RawApiResponseData {
    pub http_status: u16,
    pub mutation_attempted: bool,
    pub mutation_confirmed: bool,
    pub headers: Vec<RawResponseHeader>,
    pub body: RawResponseBody,
}
