use std::path::PathBuf;

use thiserror::Error;
use tokio::io::AsyncRead;

pub(crate) enum PayloadSource {
    Text(String),
    File(PathBuf),
    Stdin,
}

pub(crate) enum PayloadBody {
    Text(String),
    Reader(Box<dyn AsyncRead + Send + Unpin>),
}

pub(crate) struct AcquiredPayload {
    pub(crate) body: PayloadBody,
    pub(crate) content_length: Option<u64>,
}

#[derive(Debug, Error)]
pub(crate) enum PayloadError {
    #[error("could not open raw API body file `{path}`: {source}")]
    OpenFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("could not inspect raw API body file `{path}`: {source}")]
    InspectFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

impl PayloadSource {
    pub(crate) async fn acquire(self) -> Result<AcquiredPayload, PayloadError> {
        match self {
            Self::Text(body) => Ok(AcquiredPayload {
                body: PayloadBody::Text(body),
                content_length: None,
            }),
            Self::File(path) => {
                let display_path = super::escape_terminal_text(&path.display().to_string());
                let file = tokio::fs::File::open(&path).await.map_err(|source| {
                    PayloadError::OpenFile {
                        path: display_path.clone(),
                        source,
                    }
                })?;
                let content_length = file
                    .metadata()
                    .await
                    .map_err(|source| PayloadError::InspectFile {
                        path: display_path,
                        source,
                    })?
                    .len();
                Ok(AcquiredPayload {
                    body: PayloadBody::Reader(Box::new(file)),
                    content_length: Some(content_length),
                })
            }
            Self::Stdin => Ok(AcquiredPayload {
                body: PayloadBody::Reader(Box::new(tokio::io::stdin())),
                content_length: None,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::PayloadSource;

    #[tokio::test]
    async fn body_file_errors_escape_terminal_control_characters() {
        let path = PathBuf::from(format!(
            "missing-body-{}\n\u{1b}]52;c;clipboard\u{7}",
            std::process::id()
        ));

        let message = match PayloadSource::File(path).acquire().await {
            Ok(_) => panic!("the terminal-safety fixture must not name an existing file"),
            Err(error) => error.to_string(),
        };

        assert!(!message.contains('\n'));
        assert!(!message.contains('\u{1b}'));
        assert!(!message.contains('\u{7}'));
        assert!(message.contains(r"\n"));
        assert!(message.contains(r"\u{1b}"));
        assert!(message.contains(r"\u{7}"));
    }
}
