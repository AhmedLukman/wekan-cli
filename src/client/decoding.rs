//! Strict response decoding with diagnostics that never include field values.

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use serde_path_to_error::Segment;

use super::ClientError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseDecodeKind {
    InvalidJson,
    UnknownField,
    MissingField,
    InvalidType,
    InvalidValue,
}

impl ResponseDecodeKind {
    fn description(self) -> &'static str {
        match self {
            Self::InvalidJson => "invalid JSON",
            Self::UnknownField => "unknown field",
            Self::MissingField => "missing required field",
            Self::InvalidType => "invalid field type",
            Self::InvalidValue => "invalid field value",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponseDiagnostic {
    /// JSON Pointer-style diagnostic location; names are escaped and bounded.
    /// An empty string identifies the response root.
    pub path: String,
    pub kind: ResponseDecodeKind,
}

fn protocol_error(
    operation: &str,
    diagnostic: ResponseDiagnostic,
    success_status_received: bool,
) -> ClientError {
    let location = if diagnostic.path.is_empty() {
        "the response root"
    } else {
        &diagnostic.path
    };
    ClientError::Protocol {
        message: format!(
            "the {operation} response contains {} at {location}",
            diagnostic.kind.description()
        ),
        diagnostic: Some(diagnostic),
        success_status_received,
    }
}

pub(super) fn parse_json(body: &[u8], operation: &str) -> Result<Value, ClientError> {
    serde_json::from_slice(body).map_err(|_| {
        protocol_error(
            operation,
            ResponseDiagnostic {
                path: String::new(),
                kind: ResponseDecodeKind::InvalidJson,
            },
            true,
        )
    })
}

pub(super) fn decode_json<T: DeserializeOwned>(
    body: &[u8],
    operation: &str,
) -> Result<T, ClientError> {
    let mut deserializer = serde_json::Deserializer::from_slice(body);
    let value = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|error| decoding_error(error, operation, true))?;
    deserializer.end().map_err(|_| {
        protocol_error(
            operation,
            ResponseDiagnostic {
                path: String::new(),
                kind: ResponseDecodeKind::InvalidJson,
            },
            true,
        )
    })?;
    Ok(value)
}

pub(super) fn decode_value<T: DeserializeOwned>(
    value: Value,
    operation: &str,
    success_status_received: bool,
) -> Result<T, ClientError> {
    serde_path_to_error::deserialize(value)
        .map_err(|error| decoding_error(error, operation, success_status_received))
}

fn decoding_error(
    error: serde_path_to_error::Error<serde_json::Error>,
    operation: &str,
    success_status_received: bool,
) -> ClientError {
    if error.inner().is_syntax() || error.inner().is_eof() {
        return protocol_error(
            operation,
            ResponseDiagnostic {
                path: String::new(),
                kind: ResponseDecodeKind::InvalidJson,
            },
            success_status_received,
        );
    }
    // Inspect only Serde's fixed error prefixes. Never render its error text:
    // invalid-type/value diagnostics can contain passwords or returned tokens.
    let message = error.inner().to_string();
    let (kind, field) = if message.starts_with("unknown field `") {
        (ResponseDecodeKind::UnknownField, None)
    } else if message.starts_with("missing field `") {
        (ResponseDecodeKind::MissingField, quoted_field(&message))
    } else if message.starts_with("invalid type:") {
        (ResponseDecodeKind::InvalidType, None)
    } else {
        (ResponseDecodeKind::InvalidValue, None)
    };
    let mut segments: Vec<String> = error
        .path()
        .iter()
        .map(|segment| match segment {
            Segment::Seq { index } => index.to_string(),
            Segment::Map { key } => key.clone(),
            Segment::Enum { .. } => "?".to_owned(),
            Segment::Unknown => "?".to_owned(),
        })
        .collect();
    if let Some(field) = field {
        if segments.last().map(String::as_str) != Some(field) {
            segments.push(field.to_owned());
        }
    }
    let mut path = String::new();
    for segment in segments.iter().take(16) {
        path.push('/');
        // Bound and escape server-controlled field names, including terminal
        // controls. JSON Pointer metacharacters are escaped independently.
        let safe: String = segment
            .chars()
            .take(80)
            .flat_map(char::escape_default)
            .collect();
        path.push_str(&safe.replace('~', "~0").replace('/', "~1"));
        if segment.chars().count() > 80 {
            path.push_str("[truncated]");
        }
    }
    if segments.len() > 16 {
        path.push_str("/[truncated]");
    }
    protocol_error(
        operation,
        ResponseDiagnostic { path, kind },
        success_status_received,
    )
}

fn quoted_field(message: &str) -> Option<&str> {
    message.split('`').nth(1)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::client::{ListDocument, ListSummary};

    #[test]
    fn strict_diagnostics_identify_fields_without_rendering_values() {
        for (response, expected_path, expected_kind) in [
            (
                json!([{"_id":"list-1","title":"Todo","modifiedAt":null,"cardsModifiedAt":null,"future":"secret-value"}]),
                "/0/future",
                ResponseDecodeKind::UnknownField,
            ),
            (
                json!([{"_id":"list-1","modifiedAt":null,"cardsModifiedAt":null}]),
                "/0/title",
                ResponseDecodeKind::MissingField,
            ),
            (
                json!([{"_id":"list-1","title":"Todo","modifiedAt":{"secret-value":true},"cardsModifiedAt":null}]),
                "/0/modifiedAt",
                ResponseDecodeKind::InvalidType,
            ),
        ] {
            let error =
                decode_value::<Vec<ListSummary>>(response, "list collection", true).unwrap_err();
            assert!(!format!("{error:?}").contains("secret-value"));
            let ClientError::Protocol {
                diagnostic: Some(diagnostic),
                success_status_received: true,
                ..
            } = error
            else {
                panic!("expected a strict decoding diagnostic")
            };
            assert_eq!(diagnostic.path, expected_path);
            assert_eq!(diagnostic.kind, expected_kind);
        }
    }

    #[test]
    fn nested_diagnostics_and_invalid_json_preserve_locations() {
        let response = json!({
            "_id":"list-1","title":"Todo","archived":false,"boardId":"board-1",
            "createdAt":"2026-08-28T00:00:00Z","modifiedAt":"2026-08-28T00:00:00Z","type":"list",
            "wipLimit":{"value":3,"enabled":true,"soft":false,"future":true}
        });
        let error = decode_value::<ListDocument>(response, "list", true).unwrap_err();
        let ClientError::Protocol {
            diagnostic: Some(diagnostic),
            ..
        } = error
        else {
            panic!("expected diagnostic")
        };
        assert_eq!(diagnostic.path, "/wipLimit/future");
        assert_eq!(diagnostic.kind, ResponseDecodeKind::UnknownField);
        for body in [b"{secret-value".as_slice(), b"{} secret-value"] {
            let error = decode_json::<Value>(body, "list").unwrap_err();
            assert!(!format!("{error:?}").contains("secret-value"));
            let ClientError::Protocol {
                diagnostic: Some(diagnostic),
                ..
            } = error
            else {
                panic!("expected diagnostic")
            };
            assert_eq!(diagnostic.path, "");
            assert_eq!(diagnostic.kind, ResponseDecodeKind::InvalidJson);
        }
    }

    #[test]
    fn server_controlled_paths_are_bounded_and_terminal_safe() {
        let key = format!("future\u{1b}[31m/~{}", "x".repeat(500));
        let response = json!([{"_id":"list-1","title":"Todo","modifiedAt":null,"cardsModifiedAt":null,key:true}]);
        let error =
            decode_value::<Vec<ListSummary>>(response, "list collection", true).unwrap_err();
        assert!(!error.to_string().contains('\u{1b}'));
        let ClientError::Protocol {
            diagnostic: Some(diagnostic),
            ..
        } = error
        else {
            panic!("expected diagnostic")
        };
        assert!(diagnostic.path.contains("~1~0"));
        assert!(diagnostic.path.ends_with("[truncated]"));
        assert!(diagnostic.path.len() < 150);
    }

    #[test]
    fn byte_decoding_rejects_duplicate_fields_without_echoing_values() {
        let body = br#"{"_id":"list-1","_id":"secret-value","title":"Todo","modifiedAt":null,"cardsModifiedAt":null}"#;
        let error = decode_json::<ListSummary>(body, "list").unwrap_err();
        assert!(!format!("{error:?}").contains("secret-value"));
        assert!(matches!(
            error,
            ClientError::Protocol {
                diagnostic: Some(_),
                ..
            }
        ));
    }
}
