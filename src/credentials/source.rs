use std::{future::Future, path::PathBuf, pin::Pin};

use secrecy::SecretString;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SecretSource {
    File(PathBuf),
    Stdin,
}

pub type SecretInputFuture<'a> =
    Pin<Box<dyn Future<Output = Result<SecretString, AppError>> + Send + 'a>>;

pub(super) async fn read_storage_secret(
    source: SecretSource,
    limit: usize,
) -> Result<SecretString, AppError> {
    match source {
        SecretSource::Stdin => read_bounded(tokio::io::stdin(), limit).await,
        SecretSource::File(path) => {
            // Reject FIFOs before opening, then verify the opened descriptor too.
            if !tokio::fs::metadata(&path)
                .await
                .map_err(io_error)?
                .is_file()
            {
                return Err(not_regular_file());
            }
            let file = tokio::fs::File::open(&path).await.map_err(io_error)?;
            if !file.metadata().await.map_err(io_error)?.is_file() {
                return Err(not_regular_file());
            }
            read_bounded(file, limit).await
        }
    }
}

async fn read_bounded(
    reader: impl AsyncRead + Unpin,
    limit: usize,
) -> Result<SecretString, AppError> {
    let mut bytes = Vec::new();
    reader
        .take((limit as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .await
        .map_err(io_error)?;
    if bytes.len() > limit {
        return Err(AppError::invalid_input(format!(
            "storage credential input exceeds the {limit}-byte limit; no request was sent"
        )));
    }
    let mut value = String::from_utf8(bytes).map_err(|_| {
        AppError::invalid_input("storage credential input must be UTF-8; no request was sent")
    })?;
    if value.ends_with('\n') {
        value.pop();
        if value.ends_with('\r') {
            value.pop();
        }
    }
    Ok(SecretString::from(value))
}

fn not_regular_file() -> AppError {
    AppError::invalid_input(
        "storage credential file input must be a regular file; no request was sent",
    )
}

fn io_error(_: std::io::Error) -> AppError {
    AppError::new(
        ErrorCode::FileIoError,
        "could not read storage credential input; no request was sent",
        StableExitCode::Transport,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    #[tokio::test]
    async fn bounded_secret_input_preserves_whitespace_and_removes_only_one_line_ending() {
        for (input, expected) in [
            ("", ""),
            (" secret \r\n", " secret "),
            ("secret\n\n", "secret\n"),
            ("secret\r", "secret\r"),
            ("secret", "secret"),
        ] {
            let secret = read_bounded(input.as_bytes(), input.len()).await.unwrap();
            assert_eq!(secret.expose_secret(), expected);
        }
        for input in [&b"oversized"[..], &b"\xff"[..]] {
            let error = read_bounded(input, 1).await.unwrap_err();
            assert_eq!(error.code(), ErrorCode::InvalidInput);
            assert!(!error.message().contains("oversized"));
        }
    }

    #[tokio::test]
    async fn credential_files_are_bounded_regular_files_with_value_free_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.txt");
        std::fs::write(&path, "secret\r\n").unwrap();
        let secret = read_storage_secret(SecretSource::File(path.clone()), 8)
            .await
            .unwrap();
        assert_eq!(secret.expose_secret(), "secret");
        let oversized = read_storage_secret(SecretSource::File(path), 7)
            .await
            .unwrap_err();
        assert_eq!(oversized.code(), ErrorCode::InvalidInput);
        let directory = read_storage_secret(SecretSource::File(dir.path().into()), 8)
            .await
            .unwrap_err();
        assert_eq!(directory.code(), ErrorCode::InvalidInput);
        let missing = read_storage_secret(SecretSource::File(dir.path().join("missing-secret")), 8)
            .await
            .unwrap_err();
        assert_eq!(missing.code(), ErrorCode::FileIoError);
        assert!(!missing.message().contains("missing-secret"));
    }
}
