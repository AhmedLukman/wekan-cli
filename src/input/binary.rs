use std::{io, path::Path};

use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

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

pub(crate) struct BinaryStreamInput {
    pub reader: Box<dyn AsyncRead + Send + Unpin>,
    pub length: u64,
    // Retain staged stdin until the command finishes using its stream.
    _staged: Option<tempfile::NamedTempFile>,
}

fn stream_io(source: io::Error) -> BinaryInputError {
    BinaryInputError::Io {
        operation: "stage or read",
        source,
    }
}

async fn stage_binary_stream(
    reader: impl AsyncRead + Unpin,
    limit: u64,
) -> Result<BinaryStreamInput, BinaryInputError> {
    let staged = tempfile::NamedTempFile::new().map_err(stream_io)?;
    let mut file = tokio::fs::File::from_std(staged.reopen().map_err(stream_io)?);
    let length = tokio::io::copy(&mut reader.take(limit.saturating_add(1)), &mut file)
        .await
        .map_err(stream_io)?;
    if length > limit {
        return Err(BinaryInputError::TooLarge);
    }
    file.flush().await.map_err(stream_io)?;
    file.rewind().await.map_err(stream_io)?;
    Ok(BinaryStreamInput {
        reader: Box::new(file),
        length,
        _staged: Some(staged),
    })
}

pub(crate) async fn acquire_binary_stream(
    path: &Path,
    limit: u64,
) -> Result<BinaryStreamInput, BinaryInputError> {
    if path == Path::new("-") {
        return stage_binary_stream(tokio::io::stdin(), limit).await;
    }
    // Check before opening so named pipes do not wait for a writer.
    if !tokio::fs::metadata(path)
        .await
        .map_err(stream_io)?
        .is_file()
    {
        return Err(BinaryInputError::NotRegularFile);
    }
    let file = tokio::fs::File::open(path).await.map_err(stream_io)?;
    let metadata = file.metadata().await.map_err(stream_io)?;
    if !metadata.is_file() {
        return Err(BinaryInputError::NotRegularFile);
    }
    if metadata.len() > limit {
        return Err(BinaryInputError::TooLarge);
    }
    Ok(BinaryStreamInput {
        reader: Box::new(file.take(metadata.len())),
        length: metadata.len(),
        _staged: None,
    })
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
    async fn staged_stream_preserves_bytes_and_rejects_overflow() {
        let bytes = b"PK\0\xff\r\n";
        let mut acquired = stage_binary_stream(&bytes[..], bytes.len() as u64)
            .await
            .unwrap();
        let path = acquired._staged.as_ref().unwrap().path().to_owned();
        let mut actual = Vec::new();
        acquired.reader.read_to_end(&mut actual).await.unwrap();
        assert_eq!(actual, bytes);
        assert_eq!(acquired.length, bytes.len() as u64);
        drop(acquired);
        assert!(!path.exists());
        assert!(matches!(
            stage_binary_stream(&bytes[..], 1).await,
            Err(BinaryInputError::TooLarge)
        ));
    }

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
