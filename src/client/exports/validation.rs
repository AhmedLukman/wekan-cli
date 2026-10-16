//! Reader-based decoding drops each array element before reading the next one.
use super::{ExportError, ExportFormat, documents::*};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{
    Deserialize, Deserializer,
    de::{self, DeserializeOwned, SeqAccess, Visitor},
};
use std::{
    fmt,
    fs::File,
    io::{self, BufReader, Read},
    marker::PhantomData,
};

pub(super) struct DiscardVec<T>(PhantomData<T>);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for DiscardVec<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Elements<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for Elements<T> {
            type Value = DiscardVec<T>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an array of mapped export records")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                while seq.next_element::<T>()?.is_some() {}
                Ok(DiscardVec(PhantomData))
            }
        }
        deserializer.deserialize_seq(Elements(PhantomData))
    }
}
pub(super) enum Never {}
impl<'de> Deserialize<'de> for Never {
    fn deserialize<D: Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
        Err(de::Error::custom("expected an empty array"))
    }
}
pub(super) struct Date;
impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = String::deserialize(d)?;
        crate::client::datetime::validate_response_date(&value)
            .map_err(|()| de::Error::custom("expected a valid Wekan date-time string"))?;
        Ok(Self)
    }
}
pub(super) struct Base64;
impl<'de> Deserialize<'de> for Base64 {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let value = String::deserialize(d)?;
        STANDARD
            .decode(value.as_bytes())
            .map_err(|_| de::Error::custom("expected a base64 string"))?;
        Ok(Self)
    }
}
pub(super) fn optional<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}
pub(super) fn required_nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}

fn decode<T: DeserializeOwned>(file: impl Read) -> Result<(), ExportError> {
    let mut decoder = serde_json::Deserializer::from_reader(BufReader::new(file));
    let _: T = serde_path_to_error::deserialize(&mut decoder).map_err(|error| {
        if let Some(kind) = error.inner().io_error_kind() {
            ExportError::Io(io::Error::new(kind, "could not read the staged export"))
        } else {
            ExportError::Client(crate::client::decoding::decoding_error(
                error, "export", true,
            ))
        }
    })?;
    decoder.end().map_err(|error| {
        if let Some(kind) = error.io_error_kind() {
            ExportError::Io(io::Error::new(kind, "could not read the staged export"))
        } else {
            ExportError::Client(super::protocol(
                "the export response contains trailing or incomplete JSON",
            ))
        }
    })?;
    Ok(())
}

pub(super) fn validate(
    file: &mut File,
    format: ExportFormat,
    attachment: bool,
) -> Result<(), ExportError> {
    if attachment {
        return decode::<SingleAttachmentExport>(file);
    }
    match format {
        ExportFormat::Json => decode::<BoardExportDocument>(file),
        ExportFormat::Kanboard => decode::<KanboardExport>(file),
        ExportFormat::Github | ExportFormat::Gitea | ExportFormat::Forgejo => {
            decode::<DiscardVec<GithubIssueExport>>(file)
        }
        ExportFormat::Gitlab => decode::<DiscardVec<GitlabIssueExport>>(file),
        ExportFormat::Deck => decode::<DeckExport>(file),
        ExportFormat::Openproject => decode::<OpenprojectExport>(file),
        ExportFormat::Trello => decode::<TrelloExport>(file),
        ExportFormat::Jira => decode::<JiraExport>(file),
        ExportFormat::Asana => decode::<AsanaExport>(file),
        ExportFormat::Zenkit => decode::<ZenkitExport>(file),
        _ => unreachable!("binary exports do not use JSON validation"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn array_records_are_dropped_before_the_next_record_is_decoded() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static LIVE: AtomicUsize = AtomicUsize::new(0);
        struct Tracked;
        impl<'de> Deserialize<'de> for Tracked {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                u8::deserialize(d)?;
                assert_eq!(LIVE.fetch_add(1, Ordering::SeqCst), 0);
                Ok(Self)
            }
        }
        impl Drop for Tracked {
            fn drop(&mut self) {
                LIVE.fetch_sub(1, Ordering::SeqCst);
            }
        }
        let _: DiscardVec<Tracked> = serde_json::from_str("[1,2,3,4]").unwrap();
        assert_eq!(LIVE.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn staged_read_failures_are_file_errors() {
        struct Fails;
        impl Read for Fails {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "unreadable",
                ))
            }
        }
        assert!(matches!(
            decode::<BoardExportDocument>(Fails),
            Err(ExportError::Io(_))
        ));
    }
}
