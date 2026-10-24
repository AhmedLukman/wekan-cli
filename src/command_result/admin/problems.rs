use crate::client::admin::problems as wire;
use serde::Serialize;
use serde_json::Number;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProgressProblem {
    pub kind: String,
    pub active: bool,
    pub message: String,
}

impl From<wire::ProgressProblem> for ProgressProblem {
    fn from(problem: wire::ProgressProblem) -> Self {
        Self {
            kind: problem.kind,
            active: problem.active,
            message: problem.message,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DetectedProblem {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub count: Option<Number>,
}

impl From<wire::DetectedProblem> for DetectedProblem {
    fn from(problem: wire::DetectedProblem) -> Self {
        Self {
            id: problem.id,
            severity: problem.severity,
            title: problem.title,
            detail: problem.detail,
            count: problem.count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NewProblemCount {
    pub stream: String,
    pub count: u64,
}

impl From<wire::NewProblemCount> for NewProblemCount {
    fn from(problem: wire::NewProblemCount) -> Self {
        Self {
            stream: problem.stream,
            count: problem.count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProblemsOverview {
    pub any_in_progress: bool,
    pub in_progress: Vec<ProgressProblem>,
    pub any_problems: bool,
    pub problems: Vec<DetectedProblem>,
    pub new_problems: Vec<NewProblemCount>,
    pub streams: Vec<String>,
}

impl From<wire::ProblemsOverview> for ProblemsOverview {
    fn from(overview: wire::ProblemsOverview) -> Self {
        Self {
            any_in_progress: overview.any_in_progress,
            in_progress: overview.in_progress.into_iter().map(Into::into).collect(),
            any_problems: overview.any_problems,
            problems: overview.problems.into_iter().map(Into::into).collect(),
            new_problems: overview.new_problems.into_iter().map(Into::into).collect(),
            streams: overview.streams,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EventActor {
    pub kind: String,
    pub value: String,
    pub count: Number,
    pub at: String,
    pub family: Option<String>,
}

impl From<wire::EventActor> for EventActor {
    fn from(actor: wire::EventActor) -> Self {
        Self {
            kind: actor.kind,
            value: actor.value,
            count: actor.count,
            at: actor.at,
            family: actor.family,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EventLocation {
    pub country: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    pub latitude: Option<Number>,
    pub longitude: Option<Number>,
    pub via: Option<String>,
}

impl From<wire::EventLocation> for EventLocation {
    fn from(location: wire::EventLocation) -> Self {
        Self {
            country: location.country,
            city: location.city,
            region: location.region,
            latitude: location.latitude,
            longitude: location.longitude,
            via: location.via,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProblemEvent {
    pub event_id: String,
    pub stream: String,
    pub at: String,
    pub severity: Option<String>,
    pub category: Option<String>,
    pub bleed: Option<String>,
    pub action: Option<String>,
    pub source: Option<String>,
    pub cwe: Option<String>,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub ip: Option<String>,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
    pub location: Option<EventLocation>,
    pub count: Option<Number>,
    pub first_at: Option<String>,
    pub actors: Option<BTreeMap<String, EventActor>>,
    pub actors_overflow: Option<Number>,
    pub detail: Option<String>,
    pub event_type: Option<String>,
    pub db: Option<String>,
    pub kind: Option<String>,
    pub message: Option<String>,
    pub api: Option<String>,
    pub api_user_id: Option<String>,
}

impl From<wire::ProblemEvent> for ProblemEvent {
    fn from(event: wire::ProblemEvent) -> Self {
        Self {
            event_id: event.event_id,
            stream: event.stream,
            at: event.at,
            severity: event.severity,
            category: event.category,
            bleed: event.bleed,
            action: event.action,
            source: event.source,
            cwe: event.cwe,
            user_id: event.user_id,
            username: event.username,
            ip: event.ip,
            ipv4: event.ipv4,
            ipv6: event.ipv6,
            location: event.location.map(Into::into),
            count: event.count,
            first_at: event.first_at,
            actors: event.actors.map(|actors| {
                actors
                    .into_iter()
                    .map(|(key, actor)| (key, actor.into()))
                    .collect()
            }),
            actors_overflow: event.actors_overflow,
            detail: event.detail,
            event_type: event.event_type,
            db: event.db,
            kind: event.kind,
            message: event.message,
            api: event.api,
            api_user_id: event.api_user_id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProblemPage {
    pub stream: String,
    pub total: u64,
    pub limit: u64,
    pub skip: u64,
    pub rows: Vec<ProblemEvent>,
}

impl From<wire::ProblemPage> for ProblemPage {
    fn from(page: wire::ProblemPage) -> Self {
        Self {
            stream: page.stream,
            total: page.total,
            limit: page.limit,
            skip: page.skip,
            rows: page.rows.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProblemAcknowledgment {
    pub acknowledged: String,
}

impl From<wire::ProblemAcknowledgment> for ProblemAcknowledgment {
    fn from(response: wire::ProblemAcknowledgment) -> Self {
        Self {
            acknowledged: response.acknowledged,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn overview_and_acknowledgment_keep_stable_output_names_and_value_types() {
        let response: wire::ProblemsOverview = serde_json::from_value(json!({
            "anyInProgress":true,
            "inProgress":[{"kind":"schema","active":false,"message":"Checking"}],
            "anyProblems":true,
            "problems":[
                {"id":"p1","severity":"warning","title":"Fractional","detail":"Details","count":1.5},
                {"id":"p2","severity":"info","title":"Uncounted","detail":"Details"}
            ],
            "newProblems":[{"stream":"integrity","count":0}],
            "streams":["integrity"]
        }))
        .unwrap();
        assert_eq!(
            serde_json::to_value(ProblemsOverview::from(response)).unwrap(),
            json!({
                "any_in_progress":true,
                "in_progress":[{"kind":"schema","active":false,"message":"Checking"}],
                "any_problems":true,
                "problems":[
                    {"id":"p1","severity":"warning","title":"Fractional","detail":"Details","count":1.5},
                    {"id":"p2","severity":"info","title":"Uncounted","detail":"Details","count":null}
                ],
                "new_problems":[{"stream":"integrity","count":0}],
                "streams":["integrity"]
            })
        );
        assert_eq!(
            serde_json::to_value(ProblemAcknowledgment::from(wire::ProblemAcknowledgment {
                acknowledged: "integrity".into(),
            }))
            .unwrap(),
            json!({"acknowledged":"integrity"})
        );
    }

    #[test]
    fn stream_results_preserve_event_names_nested_maps_numbers_and_nulls() {
        let at = "2026-10-21T08:00:00Z";
        let response: wire::ProblemPage = serde_json::from_value(json!({
            "stream":"security","total":2,"limit":50,"skip":0,
            "rows":[{
                "_id":"event1","stream":"security","at":at,
                "severity":"warning","category":"auth","bleed":"none","action":"read",
                "source":"api","cwe":"CWE-1","userId":"user1","username":"alice",
                "ip":"127.0.0.1","ipv4":"127.0.0.1","ipv6":"::1",
                "location":{"country":"KE","city":"Nairobi","latitude":-1.5,"longitude":36,"via":"fixture"},
                "count":2,"firstAt":at,
                "actors":{"actor1":{"kind":"user","value":"user1","count":1.25,"at":at}},
                "actorsOverflow":0,"detail":"Details","type":"attempt","db":"wekan",
                "kind":"access","message":"Message","api":"/api/user","apiUserId":"user1"
            },{"_id":"event2","stream":"security","at":at}]
        }))
        .unwrap();
        let mut absent_event = json!({"event_id":"event2","stream":"security","at":at});
        for field in [
            "severity",
            "category",
            "bleed",
            "action",
            "source",
            "cwe",
            "user_id",
            "username",
            "ip",
            "ipv4",
            "ipv6",
            "location",
            "count",
            "first_at",
            "actors",
            "actors_overflow",
            "detail",
            "event_type",
            "db",
            "kind",
            "message",
            "api",
            "api_user_id",
        ] {
            absent_event[field] = serde_json::Value::Null;
        }
        assert_eq!(
            serde_json::to_value(ProblemPage::from(response)).unwrap(),
            json!({
                "stream":"security","total":2,"limit":50,"skip":0,
                "rows":[{
                    "event_id":"event1","stream":"security","at":at,
                    "severity":"warning","category":"auth","bleed":"none","action":"read",
                    "source":"api","cwe":"CWE-1","user_id":"user1","username":"alice",
                    "ip":"127.0.0.1","ipv4":"127.0.0.1","ipv6":"::1",
                    "location":{"country":"KE","city":"Nairobi","region":null,"latitude":-1.5,"longitude":36,"via":"fixture"},
                    "count":2,"first_at":at,
                    "actors":{"actor1":{"kind":"user","value":"user1","count":1.25,"at":at,"family":null}},
                    "actors_overflow":0,"detail":"Details","event_type":"attempt","db":"wekan",
                    "kind":"access","message":"Message","api":"/api/user","api_user_id":"user1"
                },absent_event]
            })
        );
    }
}
