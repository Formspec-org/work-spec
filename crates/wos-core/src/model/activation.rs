// Rust guideline compliant 2026-06-08

//! Typed model for shared activation criteria (ADR 0096; WOS-ACT-0301).
//!
//! [`ActivationCriteria`] is the reusable "when does this become active?"
//! shape: an optional event/transition trigger, an actor constraint,
//! required-data presence paths, an FEL boolean guard, and a deadline window.
//! It is referenced by obligation policies and, optionally, by milestones,
//! task SLAs, holds, DCR activities, and agent preconditions. FEL gains no
//! temporal operators — `where` is a local boolean predicate (ADR 0096 D-1).
//!
//! The evaluation algorithm lives in [`crate::activation`]; this module is the
//! data model only and mirrors `$defs/ActivationCriteria` in
//! `schemas/wos-workflow.schema.json` byte-for-byte in camelCase.

use serde::{Deserialize, Serialize};
use wos_events::ActorKind;

/// Event- or transition-shaped trigger half of an [`ActivationCriteria`].
///
/// At least one field is meaningful; an empty trigger matches nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationTrigger {
    /// Exact kernel event name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// Semantic event tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_tag: Option<String>,
    /// Kernel event family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_kind: Option<EventKind>,
    /// Transition semantic tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition_tag: Option<String>,
    /// Which case's events this trigger matches (ADR 0096; WOS-INTEG-REL-2101).
    /// `this` (default) matches only events on the case being evaluated;
    /// `related` widens matching to related-case events (e.g. `$related.*`).
    /// Absent means `this` (backward-compatible).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_scope: Option<EventScope>,
}

/// Scope of events an [`ActivationTrigger`] matches (WOS-INTEG-REL-2101).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventScope {
    /// Only events on the case the criteria is evaluated for (default).
    This,
    /// Also events surfaced from a related case (`$related.*`).
    Related,
}

/// Kernel event family (BPMN-derived taxonomy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventKind {
    Message,
    Signal,
    Timer,
    Error,
    Condition,
}

/// Actor half of an [`ActivationCriteria`].
///
/// `not_same_as_trigger_actor` is load-bearing for separation of duties: in a
/// satisfaction/violation context it prevents the actor who triggered a duty
/// from also discharging it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorConstraint {
    /// Exact actor identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    /// Required actor role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Required actor kind (`human` | `system` | `agent`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<ActorKind>,
    /// When true, the matching actor MUST differ from the triggering actor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_same_as_trigger_actor: Option<bool>,
}

/// A required-data presence path — a dotted path into a WOS namespace
/// (`caseFile`, `event`, `actor`, `context`). Resolved by a presence resolver,
/// not full FEL: missing fails, null fails, present-non-null passes.
pub type RequiredDataPath = String;

/// The reusable activation predicate (`$defs/ActivationCriteria`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationCriteria {
    /// Optional stable identifier for trace and reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Human-readable description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Event/transition trigger.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on: Option<ActivationTrigger>,
    /// FEL boolean guard. MUST evaluate to boolean; non-boolean fails activation.
    #[serde(rename = "where", default, skip_serializing_if = "Option::is_none")]
    pub where_fel: Option<String>,
    /// Actor constraint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<ActorConstraint>,
    /// Required-data presence paths.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_data: Vec<RequiredDataPath>,
    /// Deadline window (ISO 8601 duration; `P<N>BD` = N business days).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub within: Option<String>,
    /// Business Calendar sidecar URI for `P<N>BD` resolution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar_ref: Option<String>,
}

/// A reference to a named [`ActivationCriteria`] (JSON-Pointer fragment or URI),
/// resolved at load/lint time (WOS-ACT-0102/0203).
pub type ActivationCriteriaRef = String;

/// Inline-or-reference use of an activation criteria at a consuming site.
///
/// Inline and reference forms are mutually exclusive; this untagged union
/// distinguishes a JSON object (inline) from a JSON string (reference).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ActivationCriteriaUse {
    /// A `#/$defs/...` pointer or URI to a named criteria.
    Ref(ActivationCriteriaRef),
    /// An inline criteria.
    Inline(Box<ActivationCriteria>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_event_only_criteria() {
        let c: ActivationCriteria = serde_json::from_value(serde_json::json!({
            "on": { "event": "incomeChanged" }
        }))
        .unwrap();
        assert_eq!(
            c.on.as_ref().unwrap().event.as_deref(),
            Some("incomeChanged")
        );
        assert!(c.where_fel.is_none());
    }

    #[test]
    fn deserializes_event_plus_fel_criteria() {
        let c: ActivationCriteria = serde_json::from_value(serde_json::json!({
            "on": { "event": "caseFileUpdated" },
            "where": "event.field = 'income'"
        }))
        .unwrap();
        assert_eq!(c.where_fel.as_deref(), Some("event.field = 'income'"));
    }

    #[test]
    fn deserializes_actor_and_deadline_criteria() {
        let c: ActivationCriteria = serde_json::from_value(serde_json::json!({
            "actor": { "role": "caseworker" },
            "within": "P2D",
            "calendarRef": "urn:wos:calendar:federal-fy2026"
        }))
        .unwrap();
        assert_eq!(
            c.actor.as_ref().unwrap().role.as_deref(),
            Some("caseworker")
        );
        assert_eq!(c.within.as_deref(), Some("P2D"));
    }

    #[test]
    fn round_trips_camel_case() {
        let c = ActivationCriteria {
            on: Some(ActivationTrigger {
                event: Some("reviewCompleted".into()),
                ..Default::default()
            }),
            actor: Some(ActorConstraint {
                role: Some("independentReviewer".into()),
                actor_type: Some(ActorKind::Human),
                not_same_as_trigger_actor: Some(true),
                ..Default::default()
            }),
            where_fel: Some("caseFile.amount > 0".into()),
            ..Default::default()
        };
        let v = serde_json::to_value(&c).unwrap();
        // camelCase wire shape
        assert_eq!(v["where"], serde_json::json!("caseFile.amount > 0"));
        assert_eq!(v["actor"]["notSameAsTriggerActor"], serde_json::json!(true));
        assert_eq!(v["actor"]["actorType"], serde_json::json!("human"));
        let back: ActivationCriteria = serde_json::from_value(v).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn criteria_use_distinguishes_inline_from_ref() {
        let r: ActivationCriteriaUse =
            serde_json::from_value(serde_json::json!("#/$defs/commonIncomeChange")).unwrap();
        assert!(matches!(r, ActivationCriteriaUse::Ref(_)));
        let i: ActivationCriteriaUse =
            serde_json::from_value(serde_json::json!({ "on": { "event": "x" } })).unwrap();
        assert!(matches!(i, ActivationCriteriaUse::Inline(_)));
    }
}
