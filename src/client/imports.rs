//! Bounded preparation shared by the native JSON import endpoints.
use std::{io::Write, marker::PhantomData};

use serde::Serialize;
use thiserror::Error;

pub const MAX_IMPORT_JSON_BYTES: usize = 50 * 1024 * 1024;

/// An endpoint's typed request, encoded once before authentication or transport.
pub struct PreparedImportRequest<T> {
    body: Vec<u8>,
    _request: PhantomData<T>,
}

#[derive(Debug, Error)]
#[error("encoded import request exceeds the 50 MiB JSON limit; no request was sent")]
pub struct ImportEncodeError;

impl<T: Serialize> PreparedImportRequest<T> {
    pub fn new(request: &T) -> Result<Self, ImportEncodeError> {
        let mut output = BoundedJson(Vec::new());
        serde_json::to_writer(&mut output, request).map_err(|_| ImportEncodeError)?;
        Ok(Self {
            body: output.0,
            _request: PhantomData,
        })
    }
}

impl<T> PreparedImportRequest<T> {
    pub(super) fn into_body(self) -> Vec<u8> {
        self.body
    }
}

struct BoundedJson(Vec<u8>);

impl Write for BoundedJson {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.len() > MAX_IMPORT_JSON_BYTES - self.0.len() {
            return Err(std::io::Error::other("import JSON exceeds 50 MiB"));
        }
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_encoding_limit_includes_json_escaping() {
        let prepared = PreparedImportRequest::new(&"line\n").unwrap();
        assert_eq!(prepared.body, br#""line\n""#);
        assert!(PreparedImportRequest::new(&"\n".repeat(MAX_IMPORT_JSON_BYTES / 2)).is_err());
    }
}
