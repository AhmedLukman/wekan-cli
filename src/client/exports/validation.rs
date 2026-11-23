//! Reader-based decoding drops each array element before reading the next one.
use super::{ExportError, ExportFormat, documents::*, external_documents::*, website_chat::*};
use crate::client::admin::Object;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{
    Deserialize, Deserializer,
    de::{self, DeserializeOwned, SeqAccess, Visitor},
};
use std::{
    fmt,
    fs::File,
    io::{self, BufRead, BufReader, Read},
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

// Serde's derived named structs also accept positional arrays. Native export
// objects, including empty author objects, must retain their JSON object type.
pub(super) fn object<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<T, D::Error> {
    struct Object<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for Object<T> {
        type Value = T;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a JSON object")
        }
        fn visit_map<M: de::MapAccess<'de>>(self, map: M) -> Result<T, M::Error> {
            T::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }
    d.deserialize_map(Object(PhantomData))
}

pub(super) fn optional_object<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    object(d).map(Some)
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
        ExportFormat::Json | ExportFormat::Wekan => decode::<BoardExportDocument>(file),
        ExportFormat::Kanboard => decode::<NativeKanboardExport>(file),
        ExportFormat::Github | ExportFormat::Gitea | ExportFormat::Forgejo => {
            decode::<DiscardVec<NativeGithubExportItem>>(file)
        }
        ExportFormat::Gitlab => decode::<DiscardVec<NativeGitlabExportItem>>(file),
        ExportFormat::Deck => decode::<NativeDeckExport>(file),
        ExportFormat::Openproject => decode::<NativeOpenprojectExport>(file),
        ExportFormat::Trello => decode::<NativeTrelloExport>(file),
        ExportFormat::Jira => decode::<NativeJiraExport>(file),
        ExportFormat::Asana => decode::<NativeAsanaExport>(file),
        ExportFormat::Zenkit => decode::<NativeZenkitExport>(file),
        ExportFormat::Wordpressjson => decode::<DiscardVec<Object<WpPost>>>(file),
        ExportFormat::Teams => decode::<Object<TeamsExport>>(file),
        ExportFormat::Ghost => decode::<Object<GhostExport>>(file),
        ExportFormat::Taskwarrior => decode::<TaskwarriorExport>(file),
        ExportFormat::Nullboard => decode::<NullboardExport>(file),
        ExportFormat::Kanri => decode::<KanriExport>(file),
        ExportFormat::Tasksorg => decode::<TasksorgExport>(file),
        ExportFormat::Superproductivity => decode::<SuperproductivityExport>(file),
        ExportFormat::Taiga => decode::<TaigaExport>(file),
        ExportFormat::Wrikeworkflow => decode::<WrikeworkflowExport>(file),
        _ => unreachable!("binary exports do not use JSON validation"),
    }
}

pub(super) fn validate_text(file: &mut File) -> Result<(), ExportError> {
    for line in BufReader::new(file).lines() {
        line.map_err(|error| {
            if error.kind() == io::ErrorKind::InvalidData {
                ExportError::Client(super::protocol("the export response is not valid UTF-8"))
            } else {
                ExportError::Io(error)
            }
        })?;
    }
    Ok(())
}

pub(super) fn validate_calendar(file: &mut File) -> Result<(), ExportError> {
    let mut lines = BufReader::new(file).lines();
    if lines.next().transpose()?.as_deref() != Some("BEGIN:VCALENDAR") {
        return Err(super::protocol("the calendar response has no VCALENDAR header").into());
    }
    let mut last = String::new();
    for line in lines {
        let line = line?;
        if !line.is_empty() {
            last = line;
        }
    }
    if last != "END:VCALENDAR" {
        return Err(super::protocol("the calendar response has no VCALENDAR terminator").into());
    }
    Ok(())
}

pub(super) fn validate_mattermost(file: &mut File) -> Result<(), ExportError> {
    let mut lines = BufReader::new(file).lines();
    let first = lines
        .next()
        .transpose()?
        .ok_or_else(|| super::protocol("the Mattermost export has no version record"))?;
    let mut decoder = serde_json::Deserializer::from_str(&first);
    let header: MattermostRecord =
        serde_path_to_error::deserialize(&mut decoder).map_err(|error| {
            crate::client::decoding::decoding_error(error, "Mattermost version", true)
        })?;
    decoder
        .end()
        .map_err(|_| super::protocol("invalid Mattermost version record"))?;
    if !matches!(header, MattermostRecord::Version { version: 1 }) {
        return Err(super::protocol("the Mattermost export requires a version 1 header").into());
    }
    for line in lines {
        let line = line?;
        if !line.is_empty() {
            decode::<MattermostRecord>(line.as_bytes())?;
        }
    }
    Ok(())
}

pub(super) fn validate_focalboard(file: &mut File) -> Result<(), ExportError> {
    let mut lines = BufReader::new(file).lines();
    let header = lines
        .next()
        .transpose()?
        .ok_or_else(|| super::protocol("the Focalboard archive has no header"))?;
    decode::<FocalboardHeader>(header.as_bytes())?;
    for line in lines {
        let line = line?;
        if !line.is_empty() {
            decode::<FocalboardRecord>(line.as_bytes())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn artifact(format: &str, mode: &str) -> Vec<u8> {
        let extension = if matches!(format, "focalboard" | "mattermost") {
            "jsonl"
        } else {
            "json"
        };
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "tests/fixtures/responses/exports/v13.28/{format}-{mode}.{extension}"
            )),
        )
        .unwrap()
    }
    fn stage(bytes: &[u8]) -> File {
        use std::io::{Seek, Write};
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(bytes).unwrap();
        file.rewind().unwrap();
        file
    }
    #[test]
    fn v13_28_external_exports_accept_native_rich_and_selected_shapes() {
        for format in [
            "wordpressjson",
            "ghost",
            "teams",
            "taskwarrior",
            "nullboard",
            "kanri",
            "tasksorg",
            "superproductivity",
            "taiga",
            "wrikeworkflow",
            "deck",
            "kanboard",
            "openproject",
            "github",
            "gitlab",
            "gitea",
            "forgejo",
            "trello",
            "jira",
            "asana",
            "zenkit",
        ] {
            for mode in ["live", "rich", "minimal"] {
                validate(
                    &mut stage(&artifact(format, mode)),
                    format.parse().unwrap(),
                    false,
                )
                .unwrap_or_else(|e| panic!("{format} {mode}: {e:?}"));
            }
        }
        for mode in ["live", "rich", "minimal"] {
            validate_mattermost(&mut stage(&artifact("mattermost", mode)))
                .unwrap_or_else(|e| panic!("mattermost {mode}: {e:?}"));
            validate_focalboard(&mut stage(&artifact("focalboard", mode)))
                .unwrap_or_else(|e| panic!("focalboard {mode}: {e:?}"));
        }
    }
    #[test]
    fn mattermost_rejects_unknown_fields_in_records_and_their_payloads() {
        for invalid in [
            r#"{"type":"team","team":{"name":"team","display_name":"Team","type":"I"},"extra":1}"#,
            r#"{"type":"post","post":{"team":"team","channel":"ch","user":"u","message":"m","create_at":1,"extra":1}}"#,
            r#"{"type":"post","post":{"team":"team","channel":"ch","message":"m","create_at":1}}"#,
        ] {
            let body = format!("{{\"type\":\"version\",\"version\":1}}\n{invalid}\n");
            assert!(validate_mattermost(&mut stage(body.as_bytes())).is_err());
        }
    }

    #[test]
    fn v13_28_external_exports_reject_unknown_and_malformed_fields() {
        let locations = [
            ("wordpressjson", "/0/content"),
            ("teams", "/chats/0/messages/0/body"),
            ("ghost", "/db/0/data/posts/0"),
            ("taskwarrior", "/0"),
            ("nullboard", "/lists/0"),
            ("kanri", "/columns/0"),
            ("tasksorg", "/data/tasks/0/task"),
            ("superproductivity", "/data/task/entities/card0"),
            ("taiga", "/user_stories/0"),
            ("wrikeworkflow", ""),
        ];
        for (format, nested) in locations {
            let original: serde_json::Value =
                serde_json::from_slice(&artifact(format, "rich")).unwrap();
            for pointer in if original.is_array() {
                vec!["/0", nested]
            } else {
                vec!["", nested]
            } {
                let mut value = original.clone();
                value
                    .pointer_mut(pointer)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .insert("unmappedField".into(), serde_json::json!(true));
                assert!(
                    validate(
                        &mut stage(&serde_json::to_vec(&value).unwrap()),
                        format.parse().unwrap(),
                        false
                    )
                    .is_err(),
                    "{format} {pointer}"
                );
            }
        }

        for (format, field) in [
            ("github", "/0/comments_data/0/user/login"),
            ("gitlab", "/0/time_stats/total_time_spent"),
            ("asana", "/data/0/actual_time_minutes"),
            ("trello", "/cards/0/customFieldItems/0/value"),
            ("superproductivity", "/data/taskRepeatCfg/ids"),
            ("kanboard", "/tasks/0/score"),
            ("openproject", "/_embedded/elements/0/spentTime"),
            ("deck", "/stacks/0/cards/0/done"),
            ("tasksorg", "/data/tasks/0/task/recurrence"),
            ("taskwarrior", "/0/recur"),
            ("wordpressjson", "/0/content/protected"),
            ("ghost", "/db/0/data/posts/0/published_at"),
            ("teams", "/chats/0/messages/0/body/contentType"),
        ] {
            let mut value: serde_json::Value =
                serde_json::from_slice(&artifact(format, "rich")).unwrap();
            *value.pointer_mut(field).unwrap() = serde_json::json!({"unexpected": true});
            assert!(
                validate(
                    &mut stage(&serde_json::to_vec(&value).unwrap()),
                    format.parse().unwrap(),
                    false
                )
                .is_err(),
                "{format} accepted an invalid {field}"
            );
        }
        let original: serde_json::Value =
            serde_json::from_slice(&artifact("kanri", "rich")).unwrap();
        for value in [
            serde_json::json!({}),
            {
                let mut v = original.clone();
                v["title"] = serde_json::json!(42);
                v
            },
            {
                let mut v = original;
                v.as_object_mut().unwrap().remove("title");
                v
            },
        ] {
            assert!(
                validate(
                    &mut stage(&serde_json::to_vec(&value).unwrap()),
                    ExportFormat::Kanri,
                    false
                )
                .is_err()
            );
        }
        let mut lines: Vec<serde_json::Value> = String::from_utf8(artifact("focalboard", "rich"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        lines[1]["data"]["unmappedField"] = serde_json::json!(true);
        let text = lines
            .into_iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(validate_focalboard(&mut stage(text.as_bytes())).is_err());
    }
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
