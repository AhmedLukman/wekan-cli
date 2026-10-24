//! Typed validation for native formatter properties whose numeric field identifiers are dynamic.
//! These are modeled custom-field key spaces; all other unknown properties are rejected.
#![allow(dead_code)]
use super::external_documents::*;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItem {
    #[serde(skip)]
    fixed: NativeOpenprojectExportEmbeddedElementsItemFixed,
    #[serde(skip)]
    custom_fields: BTreeMap<String, AttributeValue>,
}
impl<'de> Deserialize<'de> for NativeOpenprojectExportEmbeddedElementsItem {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let mut properties = Map::<String, Value>::deserialize(d)?;
        let identifiers: Vec<_> = properties
            .keys()
            .filter(|key| {
                key.strip_prefix("customField").is_some_and(|id| {
                    !id.is_empty() && id.bytes().all(|c| c.is_ascii_digit()) && !id.starts_with('0')
                })
            })
            .cloned()
            .collect();
        let mut custom_fields = BTreeMap::new();
        for key in identifiers {
            let value = serde_json::from_value::<AttributeValue>(
                properties
                    .remove(&key)
                    .expect("an identified property exists"),
            )
            .map_err(serde::de::Error::custom)?;
            custom_fields.insert(key, value);
        }
        let fixed = serde_json::from_value::<NativeOpenprojectExportEmbeddedElementsItemFixed>(
            Value::Object(properties),
        )
        .map_err(serde::de::Error::custom)?;
        Ok(Self {
            fixed,
            custom_fields,
        })
    }
}
#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFields {
    #[serde(skip)]
    fixed: NativeJiraExportIssuesItemFieldsFixed,
    #[serde(skip)]
    custom_fields: BTreeMap<String, AttributeValue>,
}
impl<'de> Deserialize<'de> for NativeJiraExportIssuesItemFields {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let mut properties = Map::<String, Value>::deserialize(d)?;
        let identifiers: Vec<_> = properties
            .keys()
            .filter(|key| {
                key.strip_prefix("customfield_")
                    .is_some_and(|id| !id.is_empty() && id.bytes().all(|c| c.is_ascii_digit()))
            })
            .cloned()
            .collect();
        let mut custom_fields = BTreeMap::new();
        for key in identifiers {
            let value = serde_json::from_value::<AttributeValue>(
                properties
                    .remove(&key)
                    .expect("an identified property exists"),
            )
            .map_err(serde::de::Error::custom)?;
            custom_fields.insert(key, value);
        }
        let fixed = serde_json::from_value::<NativeJiraExportIssuesItemFieldsFixed>(Value::Object(
            properties,
        ))
        .map_err(serde::de::Error::custom)?;
        Ok(Self {
            fixed,
            custom_fields,
        })
    }
}
