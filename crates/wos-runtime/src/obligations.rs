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
use wos_core::model::activation::ActivationCriteria;
use wos_core::model::obligation::{
    DuplicatePolicy, ObligationPolicy, ObligationStatus, ObligationViolationAction,
    PendingObligation, ViolationActionKind,
};
use wos_core::model::kernel::ImpactLevel;
use wos_core::{ActorKind, ProvenanceRecord};
use wos_events::ObligationViolationWitness;

/// Default safety cap on the number of *pending* obligations a single process
/// may hold concurrently (WOS-PERF-2802). A malformed policy set or an
/// activation loop could otherwise materialize pending obligations without
/// bound, exhausting process state. The cap is deliberately generous: real
/// workflows hold a handful of concurrent duties, so any process approaching
/// this count is almost certainly misbehaving. Override via
/// [`ObligationMonitorConfig::max_pending`].
pub const DEFAULT_MAX_PENDING_OBLIGATIONS: usize = 1024;

/// Tunable safety limits for the obligation monitor (WOS-PERF-2802).
///
/// Additive: every field has a default that preserves pre-cap behavior for the
/// realistic workload (the cap is only reached by a runaway policy set), so
/// existing callers that pass [`ObligationMonitorConfig::default`] see no
/// behavioral change. The struct is `Copy` and cheap to thread per drain step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObligationMonitorConfig {
    /// Maximum number of `Pending` obligations a process may hold at once.
    /// When activation would push the count past this bound it is refused and a
    /// deterministic `ObligationWarning` is emitted instead of a new pending row.
    pub max_pending: usize,
}

impl Default for ObligationMonitorConfig {
    fn default() -> Self {
        Self {
            max_pending: DEFAULT_MAX_PENDING_OBLIGATIONS,
        }
    }
}

/// Whether obligation support is engaged for a workflow, and the fail-closed
/// posture to take when it is not (WOS-MIG-2604).
///
/// A workflow that declares `obligationPolicies` but runs on a processor where
/// obligation support is disabled or of unknown version MUST NOT silently drop
/// the duties: for a rights-/safety-impacting workflow the conservative posture
/// is to fail closed (block the event and escalate); operational/informational
/// workflows may proceed with a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObligationSupportPosture {
    /// Support is present; the monitor runs normally.
    Supported,
    /// Support is absent and the impact level is rights-/safety-impacting:
    /// block the event and emit a configuration violation (fail closed).
    FailClosed,
    /// Support is absent but the impact level is operational/informational:
    /// proceed, emitting a configuration warning.
    WarnOnly,
}

/// Decide the fail-closed posture for a workflow that declares obligation
/// policies given whether the processor supports obligations (WOS-MIG-2604).
///
/// `supported == true` always yields [`ObligationSupportPosture::Supported`].
/// When support is absent, a rights-/safety-impacting workflow fails closed and
/// everything else warns. This is a pure decision function so it is testable in
/// isolation and reusable by any adapter that must gate on its own capability.
pub fn obligation_support_posture(
    impact_level: ImpactLevel,
    supported: bool,
) -> ObligationSupportPosture {
    if supported {
        ObligationSupportPosture::Supported
    } else if impact_level.requires_due_process() {
        ObligationSupportPosture::FailClosed
    } else {
        ObligationSupportPosture::WarnOnly
    }
}

/// Outcome of the unsupported-feature guard (WOS-MIG-2604).
#[derive(Debug, Default)]
pub struct ObligationSupportGate {
    /// Provenance witnessing the unsupported declaration (warning or violation).
    pub provenance: Vec<ProvenanceRecord>,
    /// When `true`, the event MUST NOT be applied (rights/safety fail-closed).
    pub block: bool,
}

/// Fail-closed guard run at the monitor entry when a workflow declares
/// obligation policies but the processor cannot honor them (WOS-MIG-2604).
///
/// Returns an empty (non-blocking) gate when there are no policies or support
/// is present. Otherwise emits a deterministic configuration record per policy:
/// a blocking `ObligationViolated` for rights/safety workflows, or a
/// non-blocking `ObligationWarning` for operational/informational ones. The
/// records are PII-free (policy id only), so they are safe to persist verbatim.
pub fn evaluate_obligation_support_gate(
    policies: &[ObligationPolicy],
    impact_level: ImpactLevel,
    supported: bool,
) -> ObligationSupportGate {
    let mut gate = ObligationSupportGate::default();
    if policies.is_empty() {
        return gate;
    }
    match obligation_support_posture(impact_level, supported) {
        ObligationSupportPosture::Supported => {}
        ObligationSupportPosture::FailClosed => {
            gate.block = true;
            for policy in policies {
                gate.provenance.push(ProvenanceRecord::obligation_violated(
                    &policy.id,
                    &policy.id,
                    "obligation support unavailable on this processor; failing closed (WOS-MIG-2604)",
                    "block",
                    ObligationViolationWitness {
                        trigger_event: None,
                        deadline: None,
                        responsible_actor: None,
                        responsible_role: None,
                        event_witness: None,
                        case_state_witness: None,
                    },
                ));
            }
        }
        ObligationSupportPosture::WarnOnly => {
            for policy in policies {
                gate.provenance.push(ProvenanceRecord::obligation_warning(
                    &policy.id,
                    &policy.id,
                    "unsupported",
                ));
            }
        }
    }
    gate
}

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
    /// Idempotency token of the draining event, when one was supplied. Used to
    /// dedupe activation on replay (WOS-OBL-RUNTIME-0916); `None` disables the
    /// token-based guard (the per-policy `duplicatePolicy` still applies).
    pub idempotency_token: Option<&'a str>,
    /// Whether this event was surfaced from a related case (WOS-INTEG-REL-2101).
    /// Threaded into the [`ActivationContext`] so a policy criteria scoped to
    /// `related` matches only related-case events. The reference drain has no
    /// related-case event source yet, so it always passes `false` (own case).
    pub is_related_event: bool,
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
            is_related_event: self.is_related_event,
        }
    }
}

/// A request to materialize a task as the effect of a `createTask` violation
/// action (WOS-OBL-RUNTIME-0912). The runtime drain realizes it through the
/// existing task pipeline, linking the task back to `obligation_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObligationTaskRequest {
    /// Task-catalog reference from the violation action's `taskRef`.
    pub task_ref: String,
    /// The obligation whose violation requested the task.
    pub obligation_id: String,
    /// The policy that produced the obligation.
    pub policy_id: String,
}

/// The composed effect of one or more violation actions (WOS-OBL-RUNTIME-0908..0913).
///
/// `block` / `failed` / `reroute_to` are *gating* outcomes governed by the
/// strictness ladder (§16.2.4, WOS-OBL-SPEC-0703): when multiple obligations
/// are violated by a single event, every violation is RECORDED, but only the
/// strictest gating action takes effect. `create_tasks` / `emit_events` are
/// *composing* outcomes (`createTask` / `emitEvent`) that accumulate additively
/// across all violated obligations.
#[derive(Debug, Default, Clone)]
pub struct ViolationEffects {
    /// When `true`, the strictest applied action was `block`; the kernel event
    /// MUST NOT be applied (Governance §16.2.3 step 4).
    pub block: bool,
    /// When `true`, the strictest applied action was `fail`; the operation is
    /// marked failed rather than silently dropped (WOS-OBL-RUNTIME-0911).
    pub failed: bool,
    /// When set, the strictest applied action was `escalate`; the runtime
    /// reroutes by enqueuing this event name (WOS-OBL-RUNTIME-0910). Defaults to
    /// `escalated` when the policy declares no `escalateTo`.
    pub reroute_to: Option<String>,
    /// Tasks requested by `createTask` actions, in violation order.
    pub create_tasks: Vec<ObligationTaskRequest>,
    /// Event names requested by `emitEvent` actions, in violation order.
    pub emit_events: Vec<String>,
}

impl ViolationEffects {
    /// Apply one obligation's violation action to the accumulated effects,
    /// preserving the strictness ladder for gating actions and accumulating the
    /// composing actions. `escalate_to` is the policy-declared reroute target.
    fn apply(&mut self, action: &ObligationViolationAction, request: ObligationTaskRequest) {
        match action.kind() {
            ViolationActionKind::Block => self.block = true,
            ViolationActionKind::Fail => self.failed = true,
            ViolationActionKind::Escalate => {
                // Only the first escalate target is retained; a later stricter
                // gating action (`fail`/`block`) supersedes the reroute below.
                if self.reroute_to.is_none() {
                    self.reroute_to = Some(escalate_target(action));
                }
            }
            ViolationActionKind::Warn => {}
            ViolationActionKind::CreateTask => {
                let task_ref = action_task_ref(action).unwrap_or(&request.policy_id).to_string();
                self.create_tasks.push(ObligationTaskRequest { task_ref, ..request });
            }
            ViolationActionKind::EmitEvent => {
                if let Some(event) = action_event(action) {
                    self.emit_events.push(event.to_string());
                }
            }
        }
    }

    /// Whether any gating action (`block`/`fail`/`escalate`) is in effect. A
    /// pure `warn`/`createTask`/`emitEvent` violation leaves the event flowing.
    pub fn gates(&self) -> bool {
        self.block || self.failed || self.reroute_to.is_some()
    }

    /// Fold another effects accumulation into this one, preserving the
    /// strictness ladder for gating actions (`block`/`fail` win; the first
    /// `escalate` target is kept) and concatenating the composing actions. Used
    /// to combine a deadline-expiry pass with the event-driven gate pass within
    /// one drain step (WOS-OBL-RUNTIME-0913).
    pub fn merge(&mut self, other: ViolationEffects) {
        self.block |= other.block;
        self.failed |= other.failed;
        if self.reroute_to.is_none() {
            self.reroute_to = other.reroute_to;
        }
        self.create_tasks.extend(other.create_tasks);
        self.emit_events.extend(other.emit_events);
    }
}

/// Outcome of the pre-event obligation gate.
#[derive(Debug, Default)]
pub struct ObligationGateOutcome {
    /// Provenance records appended while evaluating the gate.
    pub provenance: Vec<ProvenanceRecord>,
    /// When `true`, a pending obligation's `violateWhen` matched with a `block`
    /// action; the kernel event MUST NOT be applied (Governance §16.2.3 step 4).
    /// Retained as a convenience mirror of [`ViolationEffects::block`].
    pub block: bool,
    /// Composed effects of every violation matched in this gate pass.
    pub effects: ViolationEffects,
}

/// Read the `escalateTo` target from a violation action, defaulting to the
/// reserved `escalated` event (mirrors the companion-policy reroute target).
fn escalate_target(action: &ObligationViolationAction) -> String {
    match action {
        ObligationViolationAction::Detailed(spec) => spec
            .escalate_to
            .clone()
            .unwrap_or_else(|| "escalated".to_string()),
        ObligationViolationAction::Shorthand(_) => "escalated".to_string(),
    }
}

fn action_task_ref(action: &ObligationViolationAction) -> Option<&str> {
    match action {
        ObligationViolationAction::Detailed(spec) => spec.task_ref.as_deref(),
        ObligationViolationAction::Shorthand(_) => None,
    }
}

fn action_event(action: &ObligationViolationAction) -> Option<&str> {
    match action {
        ObligationViolationAction::Detailed(spec) => spec.event.as_deref(),
        ObligationViolationAction::Shorthand(_) => None,
    }
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
        // Snapshot the obligation's deadline / responsible fields for the witness
        // before re-borrowing `instance` mutably to flip the status.
        let (deadline, responsible_actor, responsible_role) =
            obligation_witness_fields(instance, i);
        set_status(instance, i, ObligationStatus::Violated);
        // PII-minimized witness: only the JSON paths `violateWhen` references.
        let event_witness = project_referenced_event(violate_when, ev.event_data);
        let case_state_witness = project_referenced_case_state(violate_when, ev.case_state);
        outcome.provenance.push(ProvenanceRecord::obligation_violated(
            &policy_id,
            &obligation_id,
            "violateWhen matched while obligation pending",
            violation_action_str(action),
            ObligationViolationWitness {
                trigger_event: Some(ev.event_name),
                deadline: deadline.as_deref(),
                responsible_actor: responsible_actor.as_deref(),
                responsible_role: responsible_role.as_deref(),
                event_witness,
                case_state_witness,
            },
        ));
        outcome.effects.apply(
            &policy.on_violation,
            ObligationTaskRequest {
                task_ref: policy.id.clone(),
                obligation_id: obligation_id.clone(),
                policy_id: policy.id.clone(),
            },
        );
    }
    outcome.block = outcome.effects.block;
    outcome
}

/// The lazy deadline-expiry scan (WOS-OBL-TIME-1004). For each `Pending`
/// obligation whose stored ISO-8601 `deadline` is at or before `now`, mark it
/// `Expired`, emit `ObligationExpired`, and apply the policy's `onViolation`
/// effect (composed by the strictness ladder, exactly as the pre-event gate).
///
/// Satisfied / cancelled / already-expired obligations are skipped because
/// [`pending_snapshot`] only yields `Pending` rows — so an obligation that was
/// discharged before its deadline never fires a late expiry
/// (WOS-OBL-TIME-1005/1006).
pub fn evaluate_deadline_expiries(
    policies: &[ObligationPolicy],
    instance: &mut WorkflowProcess,
    now_ms: u64,
    _now_iso: &str,
) -> ObligationGateOutcome {
    let mut outcome = ObligationGateOutcome::default();
    let count = pending_len(instance);
    for i in 0..count {
        let Some((policy_id, obligation_id, _trigger)) = pending_snapshot(instance, i) else {
            continue;
        };
        let (deadline, responsible_actor, responsible_role) =
            obligation_witness_fields(instance, i);
        let Some(deadline) = deadline else {
            continue;
        };
        let Some(deadline_ms) = parse_iso_to_ms(&deadline) else {
            continue;
        };
        if now_ms < deadline_ms {
            continue;
        }
        let Some(policy) = find_policy(policies, &policy_id) else {
            continue;
        };
        let action = policy.on_violation.kind();
        set_status(instance, i, ObligationStatus::Expired);
        outcome.provenance.push(ProvenanceRecord::obligation_expired(
            &policy_id,
            &obligation_id,
            violation_action_str(action),
        ));
        // The deadline-elapsed violation carries no event witness (it is
        // time-driven, not event-driven); only the deadline + responsible
        // metadata accompany it (WOS-OBL-PROV-1103).
        outcome.provenance.push(ProvenanceRecord::obligation_violated(
            &policy_id,
            &obligation_id,
            "deadline elapsed while obligation pending",
            violation_action_str(action),
            ObligationViolationWitness {
                trigger_event: None,
                deadline: Some(&deadline),
                responsible_actor: responsible_actor.as_deref(),
                responsible_role: responsible_role.as_deref(),
                event_witness: None,
                case_state_witness: None,
            },
        ));
        outcome.effects.apply(
            &policy.on_violation,
            ObligationTaskRequest {
                task_ref: policy.id.clone(),
                obligation_id: obligation_id.clone(),
                policy_id: policy.id.clone(),
            },
        );
    }
    outcome.block = outcome.effects.block;
    outcome
}

/// Lazy pre-breach warning scan (WOS-OBL-TIME-1007). For each `Pending`
/// obligation with a deadline and one or more `warningThresholds`, emit
/// `ObligationWarning` once per threshold whose `beforeBreach` window has been
/// entered (i.e. `now >= deadline - beforeBreach`). Fired thresholds are
/// recorded in the obligation's `extensions` under
/// [`FIRED_WARNINGS_EXT_KEY`] so a replay or a later drain does not re-emit.
pub fn evaluate_deadline_warnings(
    policies: &[ObligationPolicy],
    instance: &mut WorkflowProcess,
    now_ms: u64,
) -> Vec<ProvenanceRecord> {
    let mut records = Vec::new();
    let count = pending_len(instance);
    for i in 0..count {
        let Some((policy_id, obligation_id, _trigger)) = pending_snapshot(instance, i) else {
            continue;
        };
        let (deadline, _actor, _role) = obligation_witness_fields(instance, i);
        let Some(deadline) = deadline else { continue };
        let Some(deadline_ms) = parse_iso_to_ms(&deadline) else {
            continue;
        };
        let Some(policy) = find_policy(policies, &policy_id) else {
            continue;
        };
        let Some(cfg) = &policy.deadline else { continue };
        for threshold in &cfg.warning_thresholds {
            let Ok(lead_ms) = wos_core::parse_iso_duration_to_ms(&threshold.before_breach) else {
                continue;
            };
            let window_start = deadline_ms.saturating_sub(lead_ms);
            // Within the window but not yet past the deadline (expiry, not
            // warning, governs once the deadline elapses).
            if now_ms < window_start || now_ms >= deadline_ms {
                continue;
            }
            if warning_already_fired(instance, i, &threshold.before_breach) {
                continue;
            }
            mark_warning_fired(instance, i, &threshold.before_breach);
            records.push(ProvenanceRecord::obligation_warning(
                &policy_id,
                &obligation_id,
                &threshold.before_breach,
            ));
        }
    }
    records
}

/// Post-event: for each policy whose `activateWhen` matches, create a pending
/// obligation (subject to `duplicatePolicy`) and emit `ObligationActivated`.
///
/// Policies are evaluated in document order, so the resulting provenance and
/// the pending-obligation push order are deterministic across drains
/// (WOS-PERF-2803). The number of concurrently `Pending` obligations is bounded
/// by `config.max_pending` (WOS-PERF-2802): once the cap is reached an otherwise
/// matching activation does NOT create a new pending obligation; a deterministic
/// `ObligationWarning` is emitted in its place so the audit trail records the
/// refusal.
pub fn evaluate_activations(
    policies: &[ObligationPolicy],
    instance: &mut WorkflowProcess,
    ev: &ObligationEvent<'_>,
    config: ObligationMonitorConfig,
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

        // Replay dedupe (WOS-OBL-RUNTIME-0916): a re-drained event carrying the
        // same idempotency token MUST NOT activate the same policy twice. The
        // key is deterministic in `(policy, token)`, so the second pass is a
        // no-op and the provenance stream is identical to the first.
        let dedupe_key = ev
            .idempotency_token
            .map(|token| format!("{}#{}", policy.id, token));
        if let Some(key) = &dedupe_key {
            if governance.seen_obligation_activation_keys.contains(key) {
                continue;
            }
        }

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

        // Safety cap (WOS-PERF-2802): refuse activation once the process already
        // holds `max_pending` Pending obligations. The refusal is deterministic
        // (depends only on current pending count) and witnessed by a warning so
        // the cap event is auditable. A `ReplaceExisting` policy cancelled rows
        // above, lowering the count, so this is checked after that pass.
        let pending_total = governance
            .pending_obligations
            .iter()
            .filter(|o| o.status == ObligationStatus::Pending)
            .count();
        if pending_total >= config.max_pending {
            records.push(ProvenanceRecord::obligation_warning(
                &policy.id,
                &policy.id,
                "maxPendingObligationsExceeded",
            ));
            continue;
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
        if let Some(key) = dedupe_key {
            governance.seen_obligation_activation_keys.push(key);
        }
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

/// Outcome of an attempted administrative obligation bypass (WOS-INTEG-AI-1706).
#[derive(Debug)]
pub enum BypassOutcome {
    /// The obligation was bypassed by a permitted (non-agent) actor; carries the
    /// emitted `ObligationViolated`/bypass provenance and the obligation id.
    Bypassed(Vec<ProvenanceRecord>),
    /// The bypass was REFUSED because the requesting actor is an agent
    /// (WOS-INTEG-AI-1706: agents MUST NOT bypass an obligation by default). The
    /// obligation remains `Pending`; the returned records witness the refused
    /// attempt (tamper/violation provenance) for the audit trail.
    RefusedAgent(Vec<ProvenanceRecord>),
    /// No pending obligation with that id was found; no state change.
    NotFound,
}

/// Administrative bypass of a pending obligation, guarded against agent actors
/// (WOS-INTEG-AI-1706).
///
/// WOS has no implicit bypass path: an obligation discharges only through
/// `satisfyWhen`/`cancelWhen`/deadline expiry. This helper is the ONLY
/// privileged escape hatch, and it is deliberately fenced:
///
/// - An **agent** actor (`ActorKind::Agent`) can NEVER bypass an obligation by
///   default. The attempt is refused, the obligation stays `Pending`, and a
///   tamper/`ObligationViolated` provenance record witnesses the refusal so the
///   audit trail shows an agent tried to escape a durable duty.
/// - A non-agent (human/system) actor with explicit authority may bypass; the
///   obligation transitions to `Bypassed` and a provenance record is emitted.
///
/// The runtime drain does not call this on the normal event path (there is no
/// bypass-by-default); it is invoked only by an explicit administrative command.
pub fn bypass_obligation(
    instance: &mut WorkflowProcess,
    obligation_id: &str,
    actor_id: Option<&str>,
    actor_type: Option<ActorKind>,
    reason: &str,
) -> BypassOutcome {
    // Locate the pending obligation by id.
    let idx = instance
        .governance_state
        .as_ref()
        .and_then(|g| {
            g.pending_obligations
                .iter()
                .position(|o| o.obligation_id == obligation_id && o.status == ObligationStatus::Pending)
        });
    let Some(i) = idx else {
        return BypassOutcome::NotFound;
    };
    let (policy_id, _oid, _trigger) = match pending_snapshot(instance, i) {
        Some(s) => s,
        None => return BypassOutcome::NotFound,
    };
    let (deadline, responsible_actor, responsible_role) = obligation_witness_fields(instance, i);

    // WOS-INTEG-AI-1706: agents MUST NOT bypass an obligation. Refuse and witness
    // the attempt as a tamper/violation; the obligation is left Pending.
    if actor_type == Some(ActorKind::Agent) {
        let record = ProvenanceRecord::obligation_violated(
            &policy_id,
            obligation_id,
            "agent actor attempted to bypass a pending obligation (refused; WOS-INTEG-AI-1706)",
            "block",
            ObligationViolationWitness {
                trigger_event: None,
                deadline: deadline.as_deref(),
                responsible_actor: responsible_actor.as_deref(),
                responsible_role: responsible_role.as_deref(),
                event_witness: None,
                case_state_witness: None,
            },
        );
        return BypassOutcome::RefusedAgent(vec![record]);
    }

    // Permitted (non-agent) bypass: transition to Bypassed and witness it. The
    // ObligationViolated record carries the human/system rationale.
    set_status(instance, i, ObligationStatus::Bypassed);
    let record = ProvenanceRecord::obligation_violated(
        &policy_id,
        obligation_id,
        reason,
        "bypass",
        ObligationViolationWitness {
            trigger_event: None,
            deadline: deadline.as_deref(),
            responsible_actor: actor_id.or(responsible_actor.as_deref()),
            responsible_role: responsible_role.as_deref(),
            event_witness: None,
            case_state_witness: None,
        },
    );
    BypassOutcome::Bypassed(vec![record])
}

/// Authorization seam for privileged obligation operations (WOS-SEC-2701).
///
/// WOS-runtime has no general `AccessControl`-style hook wired into the
/// obligation monitor (the kernel `AccessControl` trait gates transitions and
/// field reads, not obligation administration), so the monitor carries its own
/// minimal, default-deny authorizer for the two privileged escape hatches:
/// administrative bypass and deadline extension. The default implementation
/// ([`DefaultObligationAuthorizer`]) encodes the policy floor:
///
/// - an **agent** actor is NEVER authorized (mirrors WOS-INTEG-AI-1706);
/// - a **human/system** actor is authorized only when it carries the
///   `obligation-admin` role (admins / supervisors); everything else is denied.
///
/// Hosts may supply a stricter authorizer (e.g. one that consults OpenFGA) but
/// MUST NOT loosen the agent floor.
pub trait ObligationAuthorizer {
    /// Whether `actor` may bypass / extend the given obligation. `actor_roles`
    /// is the acting actor's role set; `actor_type` its kind.
    fn authorize(
        &self,
        obligation_id: &str,
        actor_id: Option<&str>,
        actor_type: Option<ActorKind>,
        actor_roles: &[String],
    ) -> bool;
}

/// Role that grants administrative authority over obligations (bypass/extend).
pub const OBLIGATION_ADMIN_ROLE: &str = "obligation-admin";

/// Default obligation authorizer: agents are always denied; humans/systems are
/// allowed only when they carry [`OBLIGATION_ADMIN_ROLE`] (WOS-SEC-2701).
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultObligationAuthorizer;

impl ObligationAuthorizer for DefaultObligationAuthorizer {
    fn authorize(
        &self,
        _obligation_id: &str,
        _actor_id: Option<&str>,
        actor_type: Option<ActorKind>,
        actor_roles: &[String],
    ) -> bool {
        if actor_type == Some(ActorKind::Agent) {
            return false;
        }
        actor_roles.iter().any(|r| r == OBLIGATION_ADMIN_ROLE)
    }
}

/// Outcome of an attempted authorized deadline extension (WOS-OBL-TIME-1008).
#[derive(Debug)]
pub enum ExtensionOutcome {
    /// The deadline was extended by an authorized actor; carries the emitted
    /// provenance recording the old and new deadline.
    Extended(Vec<ProvenanceRecord>),
    /// The extension was refused (unauthorized actor / agent); the obligation is
    /// untouched and the returned records witness the refused attempt.
    Refused(Vec<ProvenanceRecord>),
    /// No pending obligation with that id was found; no state change.
    NotFound,
}

/// Authorized extension of a pending obligation's deadline (WOS-OBL-TIME-1008),
/// routed through an [`ObligationAuthorizer`] (WOS-SEC-2701).
///
/// On success the pending obligation's `deadline` is replaced with `new_deadline`
/// and an `ObligationWarning`-shaped record witnesses the change with both the
/// old and new deadlines (so the audit trail shows who extended what). An
/// unauthorized attempt (agent, or a human lacking the admin role) is refused and
/// witnessed as a violation; the obligation keeps its original deadline.
pub fn extend_obligation_deadline(
    instance: &mut WorkflowProcess,
    obligation_id: &str,
    new_deadline: &str,
    actor_id: Option<&str>,
    actor_type: Option<ActorKind>,
    actor_roles: &[String],
    authorizer: &dyn ObligationAuthorizer,
) -> ExtensionOutcome {
    let idx = instance.governance_state.as_ref().and_then(|g| {
        g.pending_obligations.iter().position(|o| {
            o.obligation_id == obligation_id && o.status == ObligationStatus::Pending
        })
    });
    let Some(i) = idx else {
        return ExtensionOutcome::NotFound;
    };
    let (policy_id, _oid, _trigger) = match pending_snapshot(instance, i) {
        Some(s) => s,
        None => return ExtensionOutcome::NotFound,
    };
    let (old_deadline, responsible_actor, responsible_role) =
        obligation_witness_fields(instance, i);

    if !authorizer.authorize(obligation_id, actor_id, actor_type, actor_roles) {
        let record = ProvenanceRecord::obligation_violated(
            &policy_id,
            obligation_id,
            "unauthorized actor attempted to extend an obligation deadline (refused; WOS-SEC-2701)",
            "block",
            ObligationViolationWitness {
                trigger_event: None,
                deadline: old_deadline.as_deref(),
                responsible_actor: actor_id.or(responsible_actor.as_deref()),
                responsible_role: responsible_role.as_deref(),
                event_witness: None,
                case_state_witness: None,
            },
        );
        return ExtensionOutcome::Refused(vec![record]);
    }

    // Authorized: update the deadline in place and witness old → new.
    if let Some(g) = instance.governance_state.as_mut() {
        if let Some(o) = g.pending_obligations.get_mut(i) {
            o.deadline = Some(new_deadline.to_string());
        }
    }
    let mut record = ProvenanceRecord::obligation_warning(
        &policy_id,
        obligation_id,
        "deadlineExtended",
    );
    if let Some(data) = record.data.as_mut().and_then(|d| d.as_object_mut()) {
        if let Some(old) = &old_deadline {
            data.insert(
                "previousDeadline".to_string(),
                Value::String(old.clone()),
            );
        }
        data.insert(
            "newDeadline".to_string(),
            Value::String(new_deadline.to_string()),
        );
        if let Some(actor) = actor_id {
            data.insert(
                "extendedBy".to_string(),
                Value::String(actor.to_string()),
            );
        }
    }
    ExtensionOutcome::Extended(vec![record])
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

/// Snapshot the `(deadline, responsible_actor, responsible_role)` of pending
/// obligation `i` as owned values, for witness construction without holding a
/// borrow across the subsequent `set_status` mutation.
fn obligation_witness_fields(
    instance: &WorkflowProcess,
    i: usize,
) -> (Option<String>, Option<String>, Option<String>) {
    instance
        .governance_state
        .as_ref()
        .and_then(|g| g.pending_obligations.get(i))
        .map(|o| {
            (
                o.deadline.clone(),
                o.responsible_actor.clone(),
                o.responsible_role.clone(),
            )
        })
        .unwrap_or((None, None, None))
}

/// Extension key under which fired warning thresholds are recorded on a
/// [`PendingObligation`] (WOS-OBL-TIME-1007): a JSON array of `beforeBreach`
/// strings. Stored in `extensions` so it round-trips through process JSON and
/// survives replay without re-firing.
const FIRED_WARNINGS_EXT_KEY: &str = "x-wos-obligation-fired-warnings";

fn warning_already_fired(instance: &WorkflowProcess, i: usize, before_breach: &str) -> bool {
    instance
        .governance_state
        .as_ref()
        .and_then(|g| g.pending_obligations.get(i))
        .and_then(|o| o.extensions.get(FIRED_WARNINGS_EXT_KEY))
        .and_then(Value::as_array)
        .is_some_and(|fired| fired.iter().any(|v| v.as_str() == Some(before_breach)))
}

fn mark_warning_fired(instance: &mut WorkflowProcess, i: usize, before_breach: &str) {
    if let Some(g) = instance.governance_state.as_mut() {
        if let Some(o) = g.pending_obligations.get_mut(i) {
            let fired = o
                .extensions
                .entry(FIRED_WARNINGS_EXT_KEY.to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
            if let Some(arr) = fired.as_array_mut() {
                arr.push(Value::String(before_breach.to_string()));
            }
        }
    }
}

/// Parse a stored RFC 3339 / ISO 8601 timestamp back to epoch milliseconds, for
/// deadline comparison. Inverse of [`format_ms_to_iso`]; returns `None` for
/// unparseable or pre-epoch / overflowing values.
fn parse_iso_to_ms(iso: &str) -> Option<u64> {
    let nanos = OffsetDateTime::parse(iso, &Rfc3339)
        .ok()?
        .unix_timestamp_nanos();
    u64::try_from(nanos / 1_000_000).ok()
}

/// Project the PII-minimized event-witness subset for a violation record: only
/// the `event.*` paths the criteria's `required_data` references (WOS-OBL-PROV-1103).
/// Returns `None` when the criteria reference no event paths, so a violation
/// over data the policy never named carries no witness subset at all.
fn project_referenced_event(
    criteria: &ActivationCriteria,
    event_data: Option<&Value>,
) -> Option<Value> {
    project_namespace(criteria, "event", event_data?)
}

/// Project the PII-minimized case-state-witness subset: only the `caseFile.*`
/// paths the criteria's `required_data` references.
fn project_referenced_case_state(criteria: &ActivationCriteria, case_state: &Value) -> Option<Value> {
    project_namespace(criteria, "caseFile", case_state)
}

/// Build a `{ path: value }` map of the dotted paths under `namespace` named in
/// `criteria.required_data`, reading them out of `root`. Only present values are
/// carried; the result is `None` when nothing was projected. The `where` FEL
/// guard is intentionally NOT introspected — its inputs are not statically
/// enumerated here, so the conservative choice is to omit rather than risk
/// leaking an unreferenced field.
fn project_namespace(criteria: &ActivationCriteria, namespace: &str, root: &Value) -> Option<Value> {
    let mut map = serde_json::Map::new();
    for path in &criteria.required_data {
        let mut parts = path.split('.');
        if parts.next() != Some(namespace) {
            continue;
        }
        let rest: Vec<&str> = parts.collect();
        let mut current = root;
        let mut found = true;
        for part in &rest {
            match current.get(part) {
                Some(next) => current = next,
                None => {
                    found = false;
                    break;
                }
            }
        }
        if found {
            map.insert(path.clone(), current.clone());
        }
    }
    if map.is_empty() {
        None
    } else {
        Some(Value::Object(map))
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
            idempotency_token: None,
            is_related_event: false,
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

    #[test]
    fn parse_iso_round_trips_format() {
        let iso = format_ms_to_iso(172_800_000).expect("iso"); // 2 days
        assert_eq!(parse_iso_to_ms(&iso), Some(172_800_000));
    }

    // ── WOS-OBL-TIME-1004: lazy deadline expiry ─────────────────────────────

    #[test]
    fn deadline_expiry_marks_expired_and_blocks() {
        let policies = policy_with_deadline("P2D"); // onViolation: block
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        let ev = event("started", None, &cs, None, &[]); // now_ms = 0 → deadline 2 days
        evaluate_activations(&policies, &mut inst, &ev);

        // Before the deadline: no expiry.
        let early = evaluate_deadline_expiries(&policies, &mut inst, 1, "1970-01-01T00:00:00.001Z");
        assert!(early.provenance.is_empty());
        assert!(!early.block);
        assert_eq!(
            inst.governance_state.as_ref().unwrap().pending_obligations[0].status,
            ObligationStatus::Pending
        );

        // Past the deadline (2 days + 1ms): expiry fires + block effect.
        let now = 2 * 24 * 60 * 60 * 1000 + 1;
        let late = evaluate_deadline_expiries(&policies, &mut inst, now, "1970-01-03T00:00:00Z");
        assert!(late.block, "block onViolation must gate on expiry");
        // One ObligationExpired + one ObligationViolated record.
        assert_eq!(late.provenance.len(), 2);
        assert_eq!(
            late.provenance[0].record_kind,
            ProvenanceKind::ObligationExpired
        );
        assert_eq!(
            late.provenance[1].record_kind,
            ProvenanceKind::ObligationViolated
        );
        assert_eq!(
            inst.governance_state.as_ref().unwrap().pending_obligations[0].status,
            ObligationStatus::Expired
        );
    }

    #[test]
    fn satisfied_obligation_does_not_expire_later() {
        let policies = policy_with_deadline("P2D");
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        let ev = event("started", None, &cs, None, &[]);
        evaluate_activations(&policies, &mut inst, &ev);
        // Satisfy before deadline.
        let done = event("done", None, &cs, Some("u-2"), &[]);
        evaluate_satisfactions(&policies, &mut inst, &done);
        assert_eq!(
            inst.governance_state.as_ref().unwrap().pending_obligations[0].status,
            ObligationStatus::Satisfied
        );
        // Advance well past the deadline → no late expiry (WOS-OBL-TIME-1005).
        let now = 10 * 24 * 60 * 60 * 1000;
        let out = evaluate_deadline_expiries(&policies, &mut inst, now, "1970-01-11T00:00:00Z");
        assert!(out.provenance.is_empty());
        assert!(!out.block);
        assert_eq!(
            inst.governance_state.as_ref().unwrap().pending_obligations[0].status,
            ObligationStatus::Satisfied
        );
    }

    // ── WOS-OBL-RUNTIME-0913: strictest gating action across violations ─────

    fn two_policy_json(action_a: serde_json::Value, action_b: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "obligationPolicies": [
                {
                    "id": "policy-a",
                    "activateWhen": { "on": { "event": "startA" } },
                    "satisfyWhen": { "on": { "event": "doneA" } },
                    "violateWhen": { "on": { "event": "trigger" } },
                    "onViolation": action_a
                },
                {
                    "id": "policy-b",
                    "activateWhen": { "on": { "event": "startB" } },
                    "satisfyWhen": { "on": { "event": "doneB" } },
                    "violateWhen": { "on": { "event": "trigger" } },
                    "onViolation": action_b
                }
            ]
        })
    }

    #[test]
    fn warn_plus_block_yields_block_with_two_records() {
        let policies = load_obligation_policies(Some(&two_policy_json(
            serde_json::json!("warn"),
            serde_json::json!("block"),
        )));
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        evaluate_activations(&policies, &mut inst, &event("startA", None, &cs, None, &[]));
        evaluate_activations(&policies, &mut inst, &event("startB", None, &cs, None, &[]));

        let outcome =
            evaluate_pre_event_gate(&policies, &mut inst, &event("trigger", None, &cs, None, &[]));
        assert!(outcome.block, "block must win over warn");
        assert!(outcome.effects.block);
        // Both violations RECORDED.
        assert_eq!(outcome.provenance.len(), 2);
    }

    #[test]
    fn escalate_plus_block_yields_block() {
        let policies = load_obligation_policies(Some(&two_policy_json(
            serde_json::json!("escalate"),
            serde_json::json!("block"),
        )));
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        evaluate_activations(&policies, &mut inst, &event("startA", None, &cs, None, &[]));
        evaluate_activations(&policies, &mut inst, &event("startB", None, &cs, None, &[]));

        let outcome =
            evaluate_pre_event_gate(&policies, &mut inst, &event("trigger", None, &cs, None, &[]));
        assert!(outcome.effects.block, "block supersedes escalate");
        assert!(outcome.effects.gates());
        assert_eq!(outcome.provenance.len(), 2);
    }

    #[test]
    fn create_task_violation_composes_without_gating() {
        let policies = load_obligation_policies(Some(&serde_json::json!({
            "obligationPolicies": [{
                "id": "notice-required",
                "activateWhen": { "on": { "event": "started" } },
                "satisfyWhen": { "on": { "event": "noticeSent" } },
                "violateWhen": { "on": { "event": "trigger" } },
                "onViolation": { "action": "createTask", "taskRef": "supervisorReview" }
            }]
        })));
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        evaluate_activations(&policies, &mut inst, &event("started", None, &cs, None, &[]));
        let outcome =
            evaluate_pre_event_gate(&policies, &mut inst, &event("trigger", None, &cs, None, &[]));
        assert!(!outcome.block);
        assert!(!outcome.effects.gates(), "createTask does not gate");
        assert_eq!(outcome.effects.create_tasks.len(), 1);
        assert_eq!(outcome.effects.create_tasks[0].task_ref, "supervisorReview");
        assert_eq!(
            outcome.effects.create_tasks[0].obligation_id,
            "notice-required#0"
        );
    }

    // ── WOS-OBL-RUNTIME-0916: replay does not duplicate activations ─────────

    #[test]
    fn replay_with_same_token_does_not_duplicate() {
        let policies = load_obligation_policies(Some(&serde_json::json!({
            "obligationPolicies": [{
                "id": "p-each",
                "activateWhen": { "on": { "event": "started" } },
                "satisfyWhen": { "on": { "event": "done" } },
                "duplicatePolicy": "createEachTime",
                "onViolation": "block"
            }]
        })));
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        let mut ev = event("started", None, &cs, None, &[]);
        ev.idempotency_token = Some("tok-1");

        let first = evaluate_activations(&policies, &mut inst, &ev);
        let second = evaluate_activations(&policies, &mut inst, &ev);

        assert_eq!(first.len(), 1, "first drain activates once");
        assert!(second.is_empty(), "replay must not re-activate");
        assert_eq!(
            inst.governance_state.as_ref().unwrap().pending_obligations.len(),
            1
        );
    }

    // ── WOS-OBL-TIME-1007: warning thresholds fire once per threshold ───────

    #[test]
    fn warning_threshold_fires_once_within_window() {
        let policies = load_obligation_policies(Some(&serde_json::json!({
            "obligationPolicies": [{
                "id": "p-warn",
                "activateWhen": { "on": { "event": "started" } },
                "satisfyWhen": { "on": { "event": "done" } },
                "deadline": { "within": "P2D", "warningThresholds": [
                    { "beforeBreach": "P1D", "notify": ["underwriter"] }
                ] },
                "onViolation": "block"
            }]
        })));
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        evaluate_activations(&policies, &mut inst, &event("started", None, &cs, None, &[]));

        // Before the 1-day-before window: no warning.
        let day = 24 * 60 * 60 * 1000;
        let none = evaluate_deadline_warnings(&policies, &mut inst, day / 2);
        assert!(none.is_empty());

        // Inside the window (1.5 days in, deadline at 2 days): one warning.
        let first = evaluate_deadline_warnings(&policies, &mut inst, day + day / 2);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].record_kind, ProvenanceKind::ObligationWarning);

        // Re-scan in the same window: deduped, no re-fire.
        let again = evaluate_deadline_warnings(&policies, &mut inst, day + day / 2 + 1);
        assert!(again.is_empty(), "threshold fires once");
    }

    // ── WOS-INTEG-AI-1705: same-agent independence flows via trigger_actor_id ─

    #[test]
    fn agent_trigger_actor_cannot_self_satisfy() {
        let policies = policies_from(income_policy_json());
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        let ed = serde_json::json!({ "field": "income" });
        // An AGENT actor triggers the obligation; its id is recorded as the
        // trigger actor on the PendingObligation.
        let mut act = event("caseFileUpdated", Some(&ed), &cs, Some("agent-7"), &[]);
        act.actor_type = Some(ActorKind::Agent);
        evaluate_activations(&policies, &mut inst, &act);
        assert_eq!(
            inst.governance_state.as_ref().unwrap().pending_obligations[0]
                .trigger_actor_id
                .as_deref(),
            Some("agent-7"),
            "the acting agent id must flow as trigger_actor_id"
        );

        // The same agent attempts to satisfy → notSameAsTriggerActor blocks it.
        let roles = vec!["underwriter".to_string()];
        let mut same = event("underwritingReviewCompleted", None, &cs, Some("agent-7"), &roles);
        same.actor_type = Some(ActorKind::Agent);
        let recs_same = evaluate_satisfactions(&policies, &mut inst, &same);
        assert!(recs_same.is_empty(), "the triggering agent must not self-satisfy");
        assert_eq!(
            inst.governance_state.as_ref().unwrap().pending_obligations[0].status,
            ObligationStatus::Pending
        );

        // A different underwriter (independent actor) satisfies it.
        let other = event("underwritingReviewCompleted", None, &cs, Some("underwriter-2"), &roles);
        let recs = evaluate_satisfactions(&policies, &mut inst, &other);
        assert_eq!(recs.len(), 1);
    }

    // ── WOS-INTEG-AI-1706: agents MUST NOT bypass an obligation by default ───

    #[test]
    fn agent_bypass_is_refused_and_witnessed() {
        let policies = policies_from(income_policy_json());
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        let ed = serde_json::json!({ "field": "income" });
        evaluate_activations(
            &policies,
            &mut inst,
            &event("caseFileUpdated", Some(&ed), &cs, Some("caseworker-1"), &[]),
        );
        let oid = inst.governance_state.as_ref().unwrap().pending_obligations[0]
            .obligation_id
            .clone();

        let outcome = bypass_obligation(
            &mut inst,
            &oid,
            Some("agent-9"),
            Some(ActorKind::Agent),
            "agent self-clearing",
        );
        match outcome {
            BypassOutcome::RefusedAgent(records) => {
                assert_eq!(records.len(), 1);
                assert_eq!(records[0].record_kind, ProvenanceKind::ObligationViolated);
            }
            other => panic!("expected RefusedAgent, got {other:?}"),
        }
        // The obligation is untouched — still Pending.
        assert_eq!(
            inst.governance_state.as_ref().unwrap().pending_obligations[0].status,
            ObligationStatus::Pending
        );
    }

    #[test]
    fn human_bypass_is_permitted_and_transitions() {
        let policies = policies_from(income_policy_json());
        let mut inst = bare_instance();
        let cs = serde_json::json!({});
        let ed = serde_json::json!({ "field": "income" });
        evaluate_activations(
            &policies,
            &mut inst,
            &event("caseFileUpdated", Some(&ed), &cs, Some("caseworker-1"), &[]),
        );
        let oid = inst.governance_state.as_ref().unwrap().pending_obligations[0]
            .obligation_id
            .clone();

        let outcome = bypass_obligation(
            &mut inst,
            &oid,
            Some("supervisor-2"),
            Some(ActorKind::Human),
            "documented administrative override",
        );
        assert!(matches!(outcome, BypassOutcome::Bypassed(_)));
        assert_eq!(
            inst.governance_state.as_ref().unwrap().pending_obligations[0].status,
            ObligationStatus::Bypassed
        );
    }

    #[test]
    fn violation_witness_carries_only_referenced_paths() {
        let policies = load_obligation_policies(Some(&serde_json::json!({
            "obligationPolicies": [{
                "id": "p-witness",
                "activateWhen": { "on": { "event": "started" } },
                "satisfyWhen": { "on": { "event": "done" } },
                "violateWhen": {
                    "on": { "event": "trigger" },
                    "requiredData": ["event.field", "caseFile.income"]
                },
                "responsibleRole": "underwriter",
                "onViolation": "block"
            }]
        })));
        let mut inst = bare_instance();
        let cs = serde_json::json!({ "income": 60000, "ssn": "secret" });
        evaluate_activations(&policies, &mut inst, &event("started", None, &cs, None, &[]));

        let ed = serde_json::json!({ "field": "income", "note": "do-not-leak" });
        let outcome = evaluate_pre_event_gate(
            &policies,
            &mut inst,
            &event("trigger", Some(&ed), &cs, None, &[]),
        );
        let data = outcome.provenance[0].data.as_ref().unwrap();
        // Referenced paths present; unreferenced fields ("note", "ssn") absent.
        assert_eq!(data["eventWitness"]["event.field"], serde_json::json!("income"));
        assert!(data["eventWitness"].get("event.note").is_none());
        assert_eq!(
            data["caseStateWitness"]["caseFile.income"],
            serde_json::json!(60000)
        );
        assert!(data["caseStateWitness"].get("caseFile.ssn").is_none());
        assert_eq!(data["responsibleRole"], serde_json::json!("underwriter"));
    }
}
