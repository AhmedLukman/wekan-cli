//! Native Scrum transfer v2, verified against v12.57 scrumTransferExport.js.
#![allow(dead_code)] // Artifact validation retains the original bytes.
use super::validation::{Date, optional, required_nullable};
use crate::client::metadata::{ScrumCard, ScrumList, ScrumSettings, ScrumSwimlane};
use serde::Deserialize;
use serde_json::Number;
#[derive(Deserialize)]
enum TransferFormat {
    #[serde(rename = "wekan-scrum-2")]
    V2,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ScrumTransfer {
    format: TransferFormat,
    settings: ScrumSettings,
    sprints: Vec<Sprint>,
    releases: Vec<Release>,
    events: Vec<Event>,
    cards: Vec<CardMetadata>,
    lists: Vec<ListMetadata>,
    swimlanes: Vec<SwimlaneMetadata>,
    daily_observations: Vec<DailyObservation>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Sprint {
    #[serde(rename = "_id")]
    id: String,
    name: String,
    state: String,
    #[serde(default, deserialize_with = "optional")]
    goal: Option<String>,
    planned_start: Option<Date>,
    planned_end: Option<Date>,
    capacity: Option<Number>,
    #[serde(default, deserialize_with = "optional")]
    capacity_unit: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    provenance: Option<Provenance>,
    #[serde(default, deserialize_with = "optional")]
    started_at: Option<Date>,
    #[serde(default, deserialize_with = "optional")]
    completed_at: Option<Date>,
    #[serde(default, deserialize_with = "optional")]
    cancelled_at: Option<Date>,
    #[serde(default, deserialize_with = "optional")]
    cancellation_reason: Option<String>,
    rollover_sprint_id: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    start_snapshot: Option<Snapshot>,
    #[serde(default, deserialize_with = "optional")]
    close_snapshot: Option<Snapshot>,
    #[serde(default, deserialize_with = "optional")]
    created_at: Option<Date>,
    #[serde(default, deserialize_with = "optional")]
    created_by: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    updated_at: Option<Date>,
    #[serde(default, deserialize_with = "optional")]
    updated_by: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Release {
    #[serde(rename = "_id")]
    id: String,
    name: String,
    #[serde(default, deserialize_with = "optional")]
    goal: Option<String>,
    planned_start: Option<Date>,
    planned_end: Option<Date>,
    #[serde(default, deserialize_with = "optional")]
    notes: Option<String>,
    released_at: Option<Date>,
    #[serde(default, deserialize_with = "optional")]
    state: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    provenance: Option<Provenance>,
    #[serde(default, deserialize_with = "optional")]
    created_at: Option<Date>,
    #[serde(default, deserialize_with = "optional")]
    created_by: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    updated_at: Option<Date>,
    #[serde(default, deserialize_with = "optional")]
    updated_by: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Event {
    #[serde(rename = "_id")]
    id: String,
    sprint_id: String,
    kind: String,
    starts_at: Date,
    #[serde(default, deserialize_with = "optional")]
    name: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    timebox_minutes: Option<Number>,
    #[serde(default, deserialize_with = "optional")]
    notes: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    follow_up_card_ids: Option<Vec<String>>,
    #[serde(default, deserialize_with = "optional")]
    provenance: Option<Provenance>,
    #[serde(default, deserialize_with = "optional")]
    created_at: Option<Date>,
    #[serde(default, deserialize_with = "optional")]
    created_by: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    updated_at: Option<Date>,
    #[serde(default, deserialize_with = "optional")]
    updated_by: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Provenance {
    #[serde(default, deserialize_with = "optional")]
    system: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    project_id: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    record_id: Option<String>,
    source_timestamp: Option<Date>,
    #[serde(default, deserialize_with = "optional")]
    unit: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Snapshot {
    at: Date,
    unit: String,
    estimate_source: String,
    #[serde(deserialize_with = "required_nullable")]
    estimate_custom_field_id: Option<String>,
    completion_policy: String,
    #[serde(default, deserialize_with = "optional")]
    working_days: Option<Vec<u8>>,
    cards: Vec<SnapshotCard>,
    missing_estimates: u64,
    total_estimate: Number,
    #[serde(default, deserialize_with = "optional")]
    partial: Option<bool>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct SnapshotCard {
    card_id: String,
    list_id: String,
    #[serde(deserialize_with = "required_nullable")]
    estimate: Option<Number>,
    done: bool,
    archived: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct DailyObservation {
    sprint_id: String,
    started_at: Date,
    day: String,
    captured_at: Date,
    snapshot: Snapshot,
    consistency: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct CardMetadata {
    #[serde(rename = "_id")]
    id: String,
    scrum: ScrumCard,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ListMetadata {
    #[serde(rename = "_id")]
    id: String,
    scrum: ScrumList,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct SwimlaneMetadata {
    #[serde(rename = "_id")]
    id: String,
    scrum: ScrumSwimlane,
}

#[cfg(test)]
mod tests {
    use super::ScrumTransfer;
    use serde_json::{Value, json};

    fn native_transfer() -> Value {
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/responses/exports/v12.57/scrum-transfer.json"
        ))
        .unwrap()
    }

    #[test]
    fn v12_57_scrum_transfer_preserves_planning_and_history_shapes() {
        serde_json::from_value::<ScrumTransfer>(native_transfer()).unwrap();
        for path in [
            "",
            "/settings",
            "/sprints/0",
            "/sprints/0/provenance",
            "/sprints/0/startSnapshot",
            "/sprints/0/startSnapshot/cards/0",
            "/releases/0",
            "/events/0",
            "/cards/0",
            "/cards/0/scrum",
            "/lists/0/scrum",
            "/swimlanes/0/scrum",
            "/dailyObservations/0",
            "/dailyObservations/0/snapshot",
        ] {
            let mut value = native_transfer();
            value
                .pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("unmappedField".into(), json!(true));
            assert!(
                serde_json::from_value::<ScrumTransfer>(value).is_err(),
                "{path}"
            );
        }
        for path in [
            "/format",
            "/sprints/0/name",
            "/sprints/0/startSnapshot/at",
            "/sprints/0/startSnapshot/cards/0/done",
            "/dailyObservations/0/snapshot",
        ] {
            let mut value = native_transfer();
            *value.pointer_mut(path).unwrap() = json!(42);
            assert!(
                serde_json::from_value::<ScrumTransfer>(value).is_err(),
                "{path}"
            );
        }
        let mut value = native_transfer();
        value["sprints"][0]["startSnapshot"]
            .as_object_mut()
            .unwrap()
            .remove("estimateCustomFieldId");
        assert!(serde_json::from_value::<ScrumTransfer>(value).is_err());
    }
}
