// Rust guideline compliant 2026-06-08

//! Typed model for durable obligation policies and pending-obligation state
//! (ADR 0096; WOS-OBL-MODEL-0801..0805).
//!
//! [`ObligationPolicy`] is the author-time declaration of a durable,
//! future-tense, cross-event workflow duty; [`PendingObligation`] is the
//! runtime state instance it materializes. Both are distinct from the deontic
//! `Obligation` ([`crate::model::ai::Obligation`], an immediate pre-commit
//! check on agent output) and the policy-engine obligation (a per-decision
//! directive). The bare noun `Obligation` is reserved for the deontic concept;
//! the durable concept is always the two-word form (ADR 0096 D-2).
//!
//! Mirrors `$defs/ObligationPolicy` and friends in
//! `schemas/wos-workflow.schema.json` in camelCase.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::model::activation::ActivationCriteria;

/// Author-time durable obligation policy (`$defs/ObligationPolicy`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObligationPolicy {
    /// Policy identifier, unique within the governance block.
    pub id: String,
    /// Human-readable statement of the duty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// When a pending obligation is created.
    pub activate_when: ActivationCriteria,
    /// When a pending obligation is discharged.
    pub satisfy_when: ActivationCriteria,
    /// When a pending obligation is cancelled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel_when: Option<ActivationCriteria>,
    /// An event that violates the obligation if it occurs while pending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub violate_when: Option<ActivationCriteria>,
    /// Deadline configuration; expiry violates per `on_violation`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<ObligationDeadline>,
    /// Accountable actor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub responsible_actor: Option<String>,
    /// Accountable role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub responsible_role: Option<String>,
    /// Re-activation behavior while an obligation is already pending.
    #[serde(default)]
    pub duplicate_policy: DuplicatePolicy,
    /// FEL coalescing key; required when `duplicate_policy` is `coalesceByKey`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_key: Option<String>,
    /// Action applied on violation.
    pub on_violation: ObligationViolationAction,
}

/// How re-activation behaves while an obligation from the same policy is pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DuplicatePolicy {
    /// Always create a new pending obligation.
    CreateEachTime,
    /// Do not duplicate while one is pending (default).
    #[default]
    IgnoreWhilePending,
    /// Cancel and replace the existing pending obligation.
    ReplaceExisting,
    /// Coalesce obligations sharing the resolved `correlation_key`.
    CoalesceByKey,
}

/// Deadline configuration for a durable obligation (`$defs/ObligationDeadline`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObligationDeadline {
    /// Time allowed after activation (ISO 8601 duration; `P<N>BD` business days).
    pub within: String,
    /// Business Calendar sidecar URI for `P<N>BD` resolution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar_ref: Option<String>,
    /// Pre-breach warnings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warning_thresholds: Vec<ObligationWarningThreshold>,
}

/// A pre-breach warning for an obligation deadline (`$defs/ObligationWarningThreshold`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObligationWarningThreshold {
    /// Lead time before the deadline at which this warning fires.
    pub before_breach: String,
    /// Actor identifiers (roles or individuals) that receive the warning.
    pub notify: Vec<String>,
    /// Optional notification-template key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_key: Option<String>,
}

/// The set of violation actions (`$defs/ObligationViolationAction.action`).
///
/// `warn`/`escalate`/`fail`/`block` form the strictness ladder (§16.2.4);
/// `createTask`/`emitEvent` compose additively and require parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ViolationActionKind {
    Warn,
    Block,
    Escalate,
    Fail,
    CreateTask,
    EmitEvent,
}

impl ViolationActionKind {
    /// Strictness rank for the blocking ladder `warn < escalate < fail < block`
    /// (WOS-OBL-SPEC-0703). The composing actions `createTask`/`emitEvent` do
    /// not gate and rank below `warn`.
    pub fn severity(self) -> u8 {
        match self {
            ViolationActionKind::CreateTask | ViolationActionKind::EmitEvent => 0,
            ViolationActionKind::Warn => 1,
            ViolationActionKind::Escalate => 2,
            ViolationActionKind::Fail => 3,
            ViolationActionKind::Block => 4,
        }
    }
}

/// Parameter-free violation actions accepted as a string shorthand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ShorthandViolationAction {
    Warn,
    Block,
    Escalate,
    Fail,
}

impl From<ShorthandViolationAction> for ViolationActionKind {
    fn from(s: ShorthandViolationAction) -> Self {
        match s {
            ShorthandViolationAction::Warn => ViolationActionKind::Warn,
            ShorthandViolationAction::Block => ViolationActionKind::Block,
            ShorthandViolationAction::Escalate => ViolationActionKind::Escalate,
            ShorthandViolationAction::Fail => ViolationActionKind::Fail,
        }
    }
}

/// Parameterized violation action (object form).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViolationActionSpec {
    /// The action.
    pub action: ViolationActionKind,
    /// Task catalog id (required when `action` is `createTask`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_ref: Option<String>,
    /// Event name (required when `action` is `emitEvent`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// Escalation target when `action` is `escalate`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escalate_to: Option<String>,
    /// Human-readable rationale recorded in provenance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// What happens when an obligation is violated (`$defs/ObligationViolationAction`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ObligationViolationAction {
    /// String shorthand for a parameter-free action.
    Shorthand(ShorthandViolationAction),
    /// Object form for a parameterized action.
    Detailed(ViolationActionSpec),
}

impl ObligationViolationAction {
    /// The effective action kind, whether the source was shorthand or object.
    pub fn kind(&self) -> ViolationActionKind {
        match self {
            ObligationViolationAction::Shorthand(s) => (*s).into(),
            ObligationViolationAction::Detailed(d) => d.action,
        }
    }
}

/// Lifecycle status of a [`PendingObligation`] (`$defs` enum; WOS-OBL-MODEL-0804).
///
/// `Pending` is the only non-terminal state; the rest are terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ObligationStatus {
    Pending,
    Satisfied,
    Violated,
    Cancelled,
    Expired,
    Bypassed,
}

impl ObligationStatus {
    /// Whether this is a terminal status (no further transitions permitted).
    pub fn is_terminal(self) -> bool {
        !matches!(self, ObligationStatus::Pending)
    }
}

/// Runtime state of a single pending obligation (WOS-OBL-MODEL-0802).
///
/// Stored on [`crate::instance::GovernanceState::pending_obligations`].
///
/// Not `Eq`: `extensions` holds `serde_json::Value`, which is only `PartialEq`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingObligation {
    /// Stable per-instance obligation id.
    pub obligation_id: String,
    /// The policy that produced this obligation.
    pub policy_id: String,
    /// Current lifecycle status.
    pub status: ObligationStatus,
    /// Event that activated this obligation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_event: Option<String>,
    /// Actor that activated this obligation (for `notSameAsTriggerActor`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_actor_id: Option<String>,
    /// ISO 8601 activation timestamp.
    pub activated_at: String,
    /// ISO 8601 deadline, when a deadline is configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    /// Accountable actor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub responsible_actor: Option<String>,
    /// Accountable role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub responsible_role: Option<String>,
    /// Resolved coalescing key, when `coalesceByKey`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_key: Option<String>,
    /// Vendor extension bag.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_minimal_policy() {
        let p: ObligationPolicy = serde_json::from_value(serde_json::json!({
            "id": "review-required",
            "activateWhen": { "on": { "event": "incomeChanged" } },
            "satisfyWhen": { "on": { "event": "reviewCompleted" } },
            "onViolation": "block"
        }))
        .unwrap();
        assert_eq!(p.id, "review-required");
        assert_eq!(p.duplicate_policy, DuplicatePolicy::IgnoreWhilePending);
        assert_eq!(p.on_violation.kind(), ViolationActionKind::Block);
        assert!(p.deadline.is_none());
    }

    #[test]
    fn deserializes_full_policy_from_schema_example() {
        let p: ObligationPolicy = serde_json::from_value(serde_json::json!({
            "id": "income-change-review-required",
            "activateWhen": { "on": { "event": "caseFileUpdated" }, "where": "event.field = 'income'" },
            "satisfyWhen": {
                "on": { "event": "underwritingReviewCompleted" },
                "actor": { "role": "underwriter", "notSameAsTriggerActor": true }
            },
            "violateWhen": { "on": { "event": "finalApprovalRequested" } },
            "deadline": { "within": "P2D", "calendarRef": "urn:wos:calendar:federal-fy2026" },
            "responsibleRole": "underwriter",
            "duplicatePolicy": "ignoreWhilePending",
            "onViolation": "block"
        }))
        .unwrap();
        assert_eq!(p.deadline.as_ref().unwrap().within, "P2D");
        assert_eq!(
            p.satisfy_when
                .actor
                .as_ref()
                .unwrap()
                .not_same_as_trigger_actor,
            Some(true)
        );
        // round-trip preserves the wire shape
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(
            v["activateWhen"]["where"],
            serde_json::json!("event.field = 'income'")
        );
        let back: ObligationPolicy = serde_json::from_value(v).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn deserializes_detailed_violation_action() {
        let p: ObligationPolicy = serde_json::from_value(serde_json::json!({
            "id": "notice-required",
            "activateWhen": { "on": { "event": "adverseDecisionPrepared" } },
            "satisfyWhen": { "on": { "event": "noticeSent" } },
            "onViolation": { "action": "createTask", "taskRef": "supervisorReview" }
        }))
        .unwrap();
        match &p.on_violation {
            ObligationViolationAction::Detailed(d) => {
                assert_eq!(d.action, ViolationActionKind::CreateTask);
                assert_eq!(d.task_ref.as_deref(), Some("supervisorReview"));
            }
            _ => panic!("expected detailed action"),
        }
    }

    #[test]
    fn pending_obligation_round_trips_camel_case() {
        let o = PendingObligation {
            obligation_id: "obl-1".into(),
            policy_id: "income-change-review-required".into(),
            status: ObligationStatus::Pending,
            trigger_event: Some("caseFileUpdated".into()),
            trigger_actor_id: Some("caseworker-7".into()),
            activated_at: "2026-06-08T12:00:00Z".into(),
            deadline: Some("2026-06-10T12:00:00Z".into()),
            responsible_actor: None,
            responsible_role: Some("underwriter".into()),
            correlation_key: None,
            extensions: HashMap::new(),
        };
        let v = serde_json::to_value(&o).unwrap();
        assert_eq!(v["obligationId"], serde_json::json!("obl-1"));
        assert_eq!(v["triggerActorId"], serde_json::json!("caseworker-7"));
        assert_eq!(v["status"], serde_json::json!("pending"));
        // absent optional fields are omitted
        assert!(v.get("responsibleActor").is_none());
        let back: PendingObligation = serde_json::from_value(v).unwrap();
        assert_eq!(back, o);
    }

    #[test]
    fn status_terminality() {
        assert!(!ObligationStatus::Pending.is_terminal());
        for s in [
            ObligationStatus::Satisfied,
            ObligationStatus::Violated,
            ObligationStatus::Cancelled,
            ObligationStatus::Expired,
            ObligationStatus::Bypassed,
        ] {
            assert!(s.is_terminal());
        }
    }

    #[test]
    fn violation_severity_ladder() {
        assert!(ViolationActionKind::Warn.severity() < ViolationActionKind::Escalate.severity());
        assert!(ViolationActionKind::Escalate.severity() < ViolationActionKind::Fail.severity());
        assert!(ViolationActionKind::Fail.severity() < ViolationActionKind::Block.severity());
    }

    #[test]
    fn governance_state_backward_compat_without_pending_obligations() {
        // Process JSON written before this field existed MUST still deserialize.
        let gs: crate::instance::GovernanceState = serde_json::from_value(serde_json::json!({
            "activeDelegations": [],
            "activeHolds": [],
            "reviewState": {}
        }))
        .unwrap();
        assert!(gs.pending_obligations.is_empty());
        // An empty obligation set is omitted from the wire shape (round-trip stable).
        let v = serde_json::to_value(&gs).unwrap();
        assert!(v.get("pendingObligations").is_none());
    }

    #[test]
    fn governance_state_round_trips_with_pending_obligation() {
        let mut gs = crate::instance::GovernanceState::default();
        gs.pending_obligations.push(PendingObligation {
            obligation_id: "obl-1".into(),
            policy_id: "income-change-review-required".into(),
            status: ObligationStatus::Pending,
            trigger_event: Some("caseFileUpdated".into()),
            trigger_actor_id: Some("caseworker-7".into()),
            activated_at: "2026-06-08T12:00:00Z".into(),
            deadline: Some("2026-06-10T12:00:00Z".into()),
            responsible_actor: None,
            responsible_role: Some("underwriter".into()),
            correlation_key: None,
            extensions: HashMap::new(),
        });
        let v = serde_json::to_value(&gs).unwrap();
        assert_eq!(
            v["pendingObligations"][0]["obligationId"],
            serde_json::json!("obl-1")
        );
        let back: crate::instance::GovernanceState = serde_json::from_value(v).unwrap();
        assert_eq!(back.pending_obligations, gs.pending_obligations);
    }
}
