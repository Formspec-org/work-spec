// Rust guideline compliant 2026-06-08

//! Durable pending-obligation monitor (ADR 0096; Governance §16.2).
//!
//! Centralizes obligation lifecycle logic as pure functions over a
//! [`WorkflowProcess`] and an [`ObligationEvent`]. Mirrors the
//! [`crate::milestones`] shape: each function reads the process state, mutates
//! [`GovernanceState::pending_obligations`], and returns the provenance records
//! it appended (in deterministic order). The functions have no side effects
//! beyond the provided process and the returned provenance buffer, so they are
//! unit-testable in isolation and safe to slot into the runtime drain loop.
//!
//! Event-processing order within a drain step (Governance §16.2.3):
//! `pre_event_gate` (may block) → kernel event → `evaluate_activations` /
//! `evaluate_satisfactions` / `evaluate_cancellations`. Deadline timers
//! (WOS-OBL-TIME-*) are layered on top of this module by the timer path.

use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use wos_core::activation::{ActivationContext, evaluate_activation_criteria};
use wos_core::instance::WorkflowProcess;
use wos_core::model::obligation::{
    DuplicatePolicy, ObligationPolicy, ObligationStatus, PendingObligation, ViolationActionKind,
};
use wos_core::{ActorKind, ProvenanceRecord};

/// The event being processed, plus the actor and case context an activation
/// criteria is evaluated against. Borrowed; cheap to construct per drain step.
pub struct ObligationEvent<'a> {
    /// Concrete runtime event name.
    pub event_name: &'a str,
    /// Event payload object, if any.
    pub event_data: Option<&'a Value>,
    /// Semantic tags carried by the event.
    pub event_tags: &'a [String],
    /// Acting actor identifier.
    pub actor_id: Option<&'a str>,
    /// Acting actor roles.
    pub actor_roles: &'a [String],
    /// Acting actor kind.
    pub actor_type: Option<ActorKind>,
    /// Case state object (keys are case-file fields).
    pub case_state: &'a Value,
    /// Semantic tags of the firing transition.
    pub transition_tags: &'a [String],
    /// Current wall-clock time in epoch milliseconds.
    pub now_ms: u64,
    /// Current ISO 8601 timestamp (activation stamp).
    pub now_iso: &'a str,
}

impl<'a> ObligationEvent<'a> {
    /// Build an [`ActivationContext`] for this event, with the given triggering
    /// actor (set on satisfaction/violation evaluation so
    /// `notSameAsTriggerActor` can compare; `None` on activation).
    ///
    /// The returned context uses a fresh lifetime `'b` (shorter than the
    /// event's `'a`) so callers may pass a borrow of a locally-owned trigger-id
    /// `String` snapshotted from a pending obligation.
    fn ctx<'b>(&'b self, trigger_actor_id: Option<&'b str>) -> ActivationContext<'b>
    where
        'a: 'b,
    {
        ActivationContext {
            event_name: self.event_name,
            event_data: self.event_data,
            event_tags: self.event_tags,
            event_kind: None,
            actor_id: self.actor_id,
            actor_roles: self.actor_roles,
            actor_type: self.actor_type,
            case_state: self.case_state,
            transition_tags: self.transition_tags,
            now_ms: self.now_ms,
            trigger_actor_id,
        }
    }
}

/// Outcome of the pre-event obligation gate.
#[derive(Debug, Default)]
pub struct ObligationGateOutcome {
    /// Provenance records appended while evaluating the gate.
    pub provenance: Vec<ProvenanceRecord>,
    /// When `true`, a pending obligation's `violateWhen` matched with a `block`
    /// action; the kernel event MUST NOT be applied (Governance §16.2.3 step 4).
    pub block: bool,
}

/// Load the obligation policies declared under `governance.obligationPolicies`.
///
/// `governance` is the workflow's governance block as raw JSON (as held by the
/// runtime). A missing block or missing array yields an empty policy set;
/// malformed entries are rejected by schema/lint upstream, so a deserialization
/// failure here yields an empty set rather than panicking (WOS-OBL-RUNTIME-0902).
pub fn load_obligation_policies(governance: Option<&Value>) -> Vec<ObligationPolicy> {
    let Some(arr) = governance
        .and_then(|g| g.get("obligationPolicies"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|p| serde_json::from_value::<ObligationPolicy>(p.clone()).ok())
        .collect()
}

fn find_policy<'p>(policies: &'p [ObligationPolicy], id: &str) -> Option<&'p ObligationPolicy> {
    policies.iter().find(|p| p.id == id)
}

/// Pre-event gate: check each pending obligation's `violateWhen` against the
/// incoming event *before* the kernel applies it. A match emits an
/// `ObligationViolated` record, marks the obligation `Violated`, and — when the
/// policy's `onViolation` is `block` — signals that the event must be blocked.
pub fn evaluate_pre_event_gate(
    policies: &[ObligationPolicy],
    instance: &mut WorkflowProcess,
    ev: &ObligationEvent<'_>,
) -> ObligationGateOutcome {
    let mut outcome = ObligationGateOutcome::default();
    let count = pending_len(instance);
    for i in 0..count {
        let Some((policy_id, obligation_id, trigger_actor)) = pending_snapshot(instance, i) else {
            continue;
        };
        let Some(policy) = find_policy(policies, &policy_id) else {
            continue;
        };
        let Some(violate_when) = &policy.violate_when else {
            continue;
        };
        let matched = {
            let ctx = ev.ctx(trigger_actor.as_deref());
            evaluate_activation_criteria(violate_when, &ctx).matched
        };
        if !matched {
            continue;
        }
        let action = policy.on_violation.kind();
        set_status(instance, i, ObligationStatus::Violated);
        outcome.provenance.push(ProvenanceRecord::obligation_violated(
            &policy_id,
            &obligation_id,
            "violateWhen matched while obligation pending",
            violation_action_str(action),
        ));
        if action == ViolationActionKind::Block {
            outcome.block = true;
        }
    }
    outcome
}

/// Post-event: for each policy whose `activateWhen` matches, create a pending
/// obligation (subject to `duplicatePolicy`) and emit `ObligationActivated`.
pub fn evaluate_activations(
    policies: &[ObligationPolicy],
    instance: &mut WorkflowProcess,
    ev: &ObligationEvent<'_>,
) -> Vec<ProvenanceRecord> {
    let mut records = Vec::new();
    for policy in policies {
        let matched = {
            let ctx = ev.ctx(None);
            evaluate_activation_criteria(&policy.activate_when, &ctx).matched
        };
        if !matched {
            continue;
        }

        let governance = instance.governance_state.get_or_insert_with(Default::default);

        // Duplicate policy: inspect existing pending obligations for this policy.
        let pending_same = governance
            .pending_obligations
            .iter()
            .filter(|o| o.policy_id == policy.id && o.status == ObligationStatus::Pending)
            .count();
        match policy.duplicate_policy {
            DuplicatePolicy::IgnoreWhilePending | DuplicatePolicy::CoalesceByKey
                if pending_same > 0 =>
            {
                // CoalesceByKey currently coalesces per-policy (correlation-key
                // resolution is a follow-up); both skip while one is pending.
                continue;
            }
            DuplicatePolicy::ReplaceExisting => {
                for o in governance
                    .pending_obligations
                    .iter_mut()
                    .filter(|o| o.policy_id == policy.id && o.status == ObligationStatus::Pending)
                {
                    let oid = o.obligation_id.clone();
                    o.status = ObligationStatus::Cancelled;
                    records.push(ProvenanceRecord::obligation_cancelled(&policy.id, &oid));
                }
            }
            _ => {}
        }

        // Deterministic obligation id: policy id + count of all obligations
        // ever materialized for this policy (terminal or not).
        let seq = governance
            .pending_obligations
            .iter()
            .filter(|o| o.policy_id == policy.id)
            .count();
        let obligation_id = format!("{}#{}", policy.id, seq);

        // Deadline timestamp (WOS-OBL-TIME-1001): computed from the activation
        // time + `deadline.within` for wall-clock durations. Business-day
        // (`P<N>BD`) deadlines need a business calendar and are left for the
        // calendar-aware timer path (WOS-OBL-TIME-1002); timer scheduling +
        // expiry firing are WOS-OBL-TIME-1003/1004.
        let deadline = policy
            .deadline
            .as_ref()
            .and_then(|d| compute_deadline_iso(ev.now_ms, &d.within));

        governance.pending_obligations.push(PendingObligation {
            obligation_id: obligation_id.clone(),
            policy_id: policy.id.clone(),
            status: ObligationStatus::Pending,
            trigger_event: Some(ev.event_name.to_string()),
            trigger_actor_id: ev.actor_id.map(str::to_string),
            activated_at: ev.now_iso.to_string(),
            deadline,
            responsible_actor: policy.responsible_actor.clone(),
            responsible_role: policy.responsible_role.clone(),
            correlation_key: None,
            extensions: Default::default(),
        });
        records.push(ProvenanceRecord::obligation_activated(
            &policy.id,
            &obligation_id,
            Some(ev.event_name),
            None,
        ));
    }
    records
}

/// Post-event: for each pending obligation whose policy `satisfyWhen` matches,
/// mark it `Satisfied` and emit `ObligationSatisfied`. Honors
/// `notSameAsTriggerActor` via the obligation's recorded triggering actor.
pub fn evaluate_satisfactions(
    policies: &[ObligationPolicy],
    instance: &mut WorkflowProcess,
    ev: &ObligationEvent<'_>,
) -> Vec<ProvenanceRecord> {
    let mut records = Vec::new();
    let count = pending_len(instance);
    for i in 0..count {
        let Some((policy_id, obligation_id, trigger_actor)) = pending_snapshot(instance, i) else {
            continue;
        };
        let Some(policy) = find_policy(policies, &policy_id) else {
            continue;
        };
        let matched = {
            let ctx = ev.ctx(trigger_actor.as_deref());
            evaluate_activation_criteria(&policy.satisfy_when, &ctx).matched
        };
        if !matched {
            continue;
        }
        set_status(instance, i, ObligationStatus::Satisfied);
        records.push(ProvenanceRecord::obligation_satisfied(
            &policy_id,
            &obligation_id,
            ev.actor_id,
        ));
    }
    records
}

/// Post-event: for each pending obligation whose policy `cancelWhen` matches,
/// mark it `Cancelled` and emit `ObligationCancelled`.
pub fn evaluate_cancellations(
    policies: &[ObligationPolicy],
    instance: &mut WorkflowProcess,
    ev: &ObligationEvent<'_>,
) -> Vec<ProvenanceRecord> {
    let mut records = Vec::new();
    let count = pending_len(instance);
    for i in 0..count {
        let Some((policy_id, obligation_id, trigger_actor)) = pending_snapshot(instance, i) else {
            continue;
        };
        let Some(policy) = find_policy(policies, &policy_id) else {
            continue;
        };
        let Some(cancel_when) = &policy.cancel_when else {
            continue;
        };
        let matched = {
            let ctx = ev.ctx(trigger_actor.as_deref());
            evaluate_activation_criteria(cancel_when, &ctx).matched
        };
        if !matched {
            continue;
        }
        set_status(instance, i, ObligationStatus::Cancelled);
        records.push(ProvenanceRecord::obligation_cancelled(
            &policy_id,
            &obligation_id,
        ));
    }
    records
}

// ── Internal helpers (snapshot-then-mutate to keep borrows disjoint) ────────

fn pending_len(instance: &WorkflowProcess) -> usize {
    instance
        .governance_state
        .as_ref()
        .map(|g| g.pending_obligations.len())
        .unwrap_or(0)
}

/// Snapshot the `(policy_id, obligation_id, trigger_actor_id)` of pending
/// obligation `i` as owned values, but only when it is still `Pending`.
fn pending_snapshot(
    instance: &WorkflowProcess,
    i: usize,
) -> Option<(String, String, Option<String>)> {
    let o = instance.governance_state.as_ref()?.pending_obligations.get(i)?;
    if o.status != ObligationStatus::Pending {
        return None;
    }
    Some((
        o.policy_id.clone(),
        o.obligation_id.clone(),
        o.trigger_actor_id.clone(),
    ))
}

fn set_status(instance: &mut WorkflowProcess, i: usize, status: ObligationStatus) {
    if let Some(g) = instance.governance_state.as_mut() {
        if let Some(o) = g.pending_obligations.get_mut(i) {
            o.status = status;
        }
    }
}

/// Compute an obligation deadline timestamp from the activation time and a
/// `within` duration. Returns `None` for durations the wall-clock parser
/// rejects (notably the business-day `P<N>BD` form, which requires a calendar).
fn compute_deadline_iso(now_ms: u64, within: &str) -> Option<String> {
    let duration_ms = wos_core::parse_iso_duration_to_ms(within).ok()?;
    format_ms_to_iso(now_ms.checked_add(duration_ms)?)
}

/// Format epoch milliseconds as an RFC 3339 timestamp (mirrors the runtime's
/// `format_timestamp`, which is not visible from this sibling module).
fn format_ms_to_iso(ms: u64) -> Option<String> {
    let nanos = i128::from(ms).checked_mul(1_000_000)?;
    let nanos_i64 = i64::try_from(nanos).ok()?;
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(nanos_i64))
        .ok()?
        .format(&Rfc3339)
        .ok()
}

fn violation_action_str(action: ViolationActionKind) -> &'static str {
    match action {
        ViolationActionKind::Warn => "warn",
        ViolationActionKind::Block => "block",
        ViolationActionKind::Escalate => "escalate",
        ViolationActionKind::Fail => "fail",
        ViolationActionKind::CreateTask => "createTask",
        ViolationActionKind::EmitEvent => "emitEvent",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wos_core::ProvenanceKind;

    fn policies_from(json: serde_json::Value) -> Vec<ObligationPolicy> {
        load_obligation_policies(Some(&json))
    }

    fn income_policy_json() -> serde_json::Value {
        serde_json::json!({
            "obligationPolicies": [{
                "id": "income-change-review-required",
                "activateWhen": { "on": { "event": "caseFileUpdated" }, "where": "event.field = 'income'" },
                "satisfyWhen": {
                    "on": { "event": "underwritingReviewCompleted" },
                    "actor": { "role": "underwriter", "notSameAsTriggerActor": true }
                },
                "violateWhen": { "on": { "event": "finalApprovalRequested" } },
                "responsibleRole": "underwriter",
                "duplicatePolicy": "ignoreWhilePending",
                "onViolation": "block"
            }]
        })
    }

    // Mirrors `milestones::tests::bare_instance` (struct literal, not
    // deserialization) so the test fixture stays valid against the exact
    // `WorkflowProcess` field set.
    fn bare_instance() -> WorkflowProcess {
        WorkflowProcess {
            process_id: "p1".to_string(),
            case_ledger_id: "case-1".to_string(),
            tenant: stack_common_typeid::DEFAULT_TENANT.to_string(),
            definition_url: "urn:test".to_string(),
            definition_version: "1.0.0".to_string(),
            configuration: Vec::new(),
            case_state: serde_json::json!({}),
            provenance_position: 0,
            next_task_sequence: 0,
            timers: Vec::new(),
            active_tasks: Vec::new(),
            history_store: Default::default(),
            compensation_logs: Default::default(),
            status: wos_core::instance::InstanceStatus::Active,
            stalled_since: None,
            decline_reason: None,
            voided_by: None,
            voided_at: None,
            expired_at: None,
            pending_events: Vec::new(),
            governance_state: None,
            volume_counters: None,
            fired_milestones: Default::default(),
            pending_callbacks: Default::default(),
            created_at: "2026-06-08T00:00:00Z".to_string(),
            updated_at: "2026-06-08T00:00:00Z".to_string(),
            extensions: Default::default(),
        }
    }

    fn event<'a>(
        name: &'a str,
        data: Option<&'a Value>,
        case_state: &'a Value,
        actor_id: Option<&'a str>,
        roles: &'a [String],
    ) -> ObligationEvent<'a> {
        ObligationEvent {
            event_name: name,
            event_data: data,
            event_tags: &[],
            actor_id,
            actor_roles: roles,
            actor_type: None,
            case_state,
            transition_tags: &[],
            now_ms: 0,
            now_iso: "2026-06-08T12:00:00Z",
        }
    }

    #[test]
    fn loads_policies_or_empty() {
        assert_eq!(load_obligation_policies(None).len(), 0);
        assert_eq!(load_obligation_policies(Some(&serde_json::json!({}))).len(), 0);
        assert_eq!(policies_from(income_policy_json()).len(), 1);
    }

    #[test]
    fn activation_creates_pending_and_provenance() {
        let policies = policies_from(income_policy_json());
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        let ed = serde_json::json!({ "field": "income" });
        let ev = event("caseFileUpdated", Some(&ed), &cs, Some("caseworker-7"), &[]);

        let recs = evaluate_activations(&policies, &mut inst, &ev);

        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].record_kind, ProvenanceKind::ObligationActivated);
        let g = inst.governance_state.as_ref().unwrap();
        assert_eq!(g.pending_obligations.len(), 1);
        let o = &g.pending_obligations[0];
        assert_eq!(o.status, ObligationStatus::Pending);
        assert_eq!(o.policy_id, "income-change-review-required");
        assert_eq!(o.trigger_actor_id.as_deref(), Some("caseworker-7"));
        assert_eq!(o.responsible_role.as_deref(), Some("underwriter"));
    }

    #[test]
    fn activation_does_not_fire_when_fel_false() {
        let policies = policies_from(income_policy_json());
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        let ed = serde_json::json!({ "field": "address" });
        let ev = event("caseFileUpdated", Some(&ed), &cs, None, &[]);

        let recs = evaluate_activations(&policies, &mut inst, &ev);
        assert!(recs.is_empty());
        assert!(inst.governance_state.is_none() || inst
            .governance_state
            .as_ref()
            .unwrap()
            .pending_obligations
            .is_empty());
    }

    #[test]
    fn ignore_while_pending_does_not_duplicate() {
        let policies = policies_from(income_policy_json());
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        let ed = serde_json::json!({ "field": "income" });
        let ev = event("caseFileUpdated", Some(&ed), &cs, None, &[]);

        evaluate_activations(&policies, &mut inst, &ev);
        let recs2 = evaluate_activations(&policies, &mut inst, &ev);

        assert!(recs2.is_empty(), "ignoreWhilePending must not duplicate");
        assert_eq!(
            inst.governance_state.as_ref().unwrap().pending_obligations.len(),
            1
        );
    }

    #[test]
    fn satisfaction_marks_satisfied_and_respects_independence() {
        let policies = policies_from(income_policy_json());
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        let ed = serde_json::json!({ "field": "income" });
        // Activate, recording trigger actor "agent-1".
        let act = event("caseFileUpdated", Some(&ed), &cs, Some("agent-1"), &[]);
        evaluate_activations(&policies, &mut inst, &act);

        // Same actor attempts to satisfy → notSameAsTriggerActor blocks it.
        let roles = vec!["underwriter".to_string()];
        let same = event("underwritingReviewCompleted", None, &cs, Some("agent-1"), &roles);
        let recs_same = evaluate_satisfactions(&policies, &mut inst, &same);
        assert!(recs_same.is_empty(), "trigger actor must not satisfy");
        assert_eq!(
            inst.governance_state.as_ref().unwrap().pending_obligations[0].status,
            ObligationStatus::Pending
        );

        // A different underwriter satisfies it.
        let other = event("underwritingReviewCompleted", None, &cs, Some("underwriter-2"), &roles);
        let recs = evaluate_satisfactions(&policies, &mut inst, &other);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].record_kind, ProvenanceKind::ObligationSatisfied);
        assert_eq!(
            inst.governance_state.as_ref().unwrap().pending_obligations[0].status,
            ObligationStatus::Satisfied
        );
    }

    #[test]
    fn pre_event_gate_blocks_premature_event() {
        let policies = policies_from(income_policy_json());
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        let ed = serde_json::json!({ "field": "income" });
        let act = event("caseFileUpdated", Some(&ed), &cs, None, &[]);
        evaluate_activations(&policies, &mut inst, &act);

        // finalApprovalRequested while obligation pending → violateWhen + block.
        let approval = event("finalApprovalRequested", None, &cs, None, &[]);
        let outcome = evaluate_pre_event_gate(&policies, &mut inst, &approval);

        assert!(outcome.block, "block action must signal a block");
        assert_eq!(outcome.provenance.len(), 1);
        assert_eq!(
            outcome.provenance[0].record_kind,
            ProvenanceKind::ObligationViolated
        );
        assert_eq!(
            inst.governance_state.as_ref().unwrap().pending_obligations[0].status,
            ObligationStatus::Violated
        );
    }

    #[test]
    fn gate_is_quiet_with_no_pending_obligations() {
        let policies = policies_from(income_policy_json());
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        let ev = event("finalApprovalRequested", None, &cs, None, &[]);
        let outcome = evaluate_pre_event_gate(&policies, &mut inst, &ev);
        assert!(!outcome.block);
        assert!(outcome.provenance.is_empty());
    }

    fn policy_with_deadline(within: &str) -> Vec<ObligationPolicy> {
        policies_from(serde_json::json!({
            "obligationPolicies": [{
                "id": "p-deadline",
                "activateWhen": { "on": { "event": "started" } },
                "satisfyWhen": { "on": { "event": "done" } },
                "deadline": { "within": within },
                "onViolation": "block"
            }]
        }))
    }

    #[test]
    fn activation_computes_wall_clock_deadline() {
        let policies = policy_with_deadline("P2D");
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        // now_ms = 0 (epoch) + 2 days → 1970-01-03T00:00:00Z.
        let ev = event("started", None, &cs, None, &[]);
        evaluate_activations(&policies, &mut inst, &ev);
        let o = &inst.governance_state.as_ref().unwrap().pending_obligations[0];
        assert_eq!(o.deadline.as_deref(), Some("1970-01-03T00:00:00Z"));
    }

    #[test]
    fn activation_leaves_business_day_deadline_uncomputed() {
        // `P5BD` needs a business calendar; the wall-clock path returns None.
        let policies = policy_with_deadline("P5BD");
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        let ev = event("started", None, &cs, None, &[]);
        evaluate_activations(&policies, &mut inst, &ev);
        let o = &inst.governance_state.as_ref().unwrap().pending_obligations[0];
        assert!(o.deadline.is_none());
    }

    #[test]
    fn format_ms_to_iso_epoch() {
        assert_eq!(format_ms_to_iso(0).as_deref(), Some("1970-01-01T00:00:00Z"));
    }
}
