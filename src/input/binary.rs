use std::{io, path::Path};

use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Debug, Error)]
pub(crate) enum BinaryInputError {
    #[error("could not {operation} binary input: {source}")]
    Io {
        operation: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("binary input must be a regular file or stdin (-)")]
    NotRegularFile,
    #[error("binary input exceeds the configured byte limit")]
    TooLarge,
}

pub(crate) async fn acquire_binary_file(
    path: &Path,
    limit: usize,
) -> Result<Vec<u8>, BinaryInputError> {
    if path == Path::new("-") {
        return read_bounded(tokio::io::stdin(), limit).await;
    }
    acquire_regular_file(path, limit).await
}

pub(super) async fn acquire_regular_file(
    path: &Path,
    limit: usize,
) -> Result<Vec<u8>, BinaryInputError> {
    // Opening a FIFO can block before descriptor metadata is available.
    if !tokio::fs::metadata(path)
        .await
        .map_err(|source| BinaryInputError::Io {
            operation: "inspect",
            source,
        })?
        .is_file()
    {
        return Err(BinaryInputError::NotRegularFile);
    }
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|source| BinaryInputError::Io {
            operation: "open",
            source,
        })?;
    if !file
        .metadata()
        .await
        .map_err(|source| BinaryInputError::Io {
            operation: "inspect",
            source,
        })?
        .is_file()
    {
        return Err(BinaryInputError::NotRegularFile);
    }
    read_bounded(file, limit).await
}

pub(super) async fn read_bounded(
    reader: impl AsyncRead + Unpin,
    limit: usize,
) -> Result<Vec<u8>, BinaryInputError> {
    let mut bytes = Vec::new();
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|source| BinaryInputError::Io {
            operation: "read",
            source,
        })?;
    if bytes.len() > limit {
        return Err(BinaryInputError::TooLarge);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn binary_reader_preserves_bytes_and_enforces_bound() {
        let data = [0, 255, 13, 10, 0];
        assert_eq!(read_bounded(&data[..], data.len()).await.unwrap(), data);
        assert!(matches!(
            read_bounded(tokio::io::repeat(0), data.len()).await,
            Err(BinaryInputError::TooLarge)
        ));
    }

    #[tokio::test]
    async fn binary_file_input_preserves_bytes_and_rejects_directories() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("input.bin");
        let data = [0, 255, 13, 10, 0];
        std::fs::write(&path, data).unwrap();
        assert_eq!(acquire_binary_file(&path, data.len()).await.unwrap(), data);
        assert!(matches!(
            acquire_binary_file(directory.path(), data.len()).await,
            Err(BinaryInputError::NotRegularFile)
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn binary_file_input_rejects_fifos_without_waiting_for_a_writer() {
        use std::{fs::OpenOptions, sync::mpsc, thread, time::Duration};

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("input.fifo");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );

        // Release a regressed blocking open so a failing test cannot hang the runtime.
        let (finished, completion) = mpsc::channel();
        let writer_path = path.clone();
        let writer = thread::spawn(move || {
            if completion.recv_timeout(Duration::from_secs(2)).is_ok() {
                return false;
            }
            let _file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(writer_path)
                .unwrap();
            true
        });
        let result = acquire_binary_file(&path, 16).await;
        let _ = finished.send(());
        assert!(!writer.join().unwrap(), "FIFO open waited for a writer");
        assert!(matches!(result, Err(BinaryInputError::NotRegularFile)));
    }
}
