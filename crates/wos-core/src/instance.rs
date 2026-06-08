// Rust guideline compliant 2026-02-21

//! WorkflowProcess serialization format (Runtime Companion S3).
//!
//! A WorkflowProcess captures the complete runtime state of a workflow
//! process — enough to resume after a crash, migrate between
//! processors, or audit past behavior.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use stack_common_typeid as typeid;

use crate::model::governance::HoldType;
use crate::model::obligation::PendingObligation;

fn default_tenant() -> String {
    typeid::DEFAULT_TENANT.to_string()
}

/// A running workflow process (Runtime Companion S3.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowProcess {
    /// Workflow runtime process identifier.
    pub process_id: String,

    /// Durable case-ledger identifier this process writes into.
    pub case_ledger_id: String,

    /// Tenant this process belongs to (ADR 0068 D-1 / PLN-0004).
    ///
    /// MUST match the TypeID prefix when `process_id` is a WOS process TypeID.
    /// At creation, processors resolve this from `CreateProcessRequest.tenant`
    /// when set (and consistent with the TypeID prefix when both are present),
    /// else from the `process_id` prefix, else [`stack_common_typeid::DEFAULT_TENANT`].
    #[serde(default = "default_tenant")]
    pub tenant: String,

    /// Canonical URL of the governing Kernel Document.
    pub definition_url: String,

    /// Version of the Kernel Document, pinned at creation.
    pub definition_version: String,

    /// Active leaf states in deterministic document order.
    pub configuration: Vec<String>,

    /// Current case file field values.
    pub case_state: serde_json::Value,

    /// Provenance log cursor.
    pub provenance_position: u64,

    /// Next task sequence number for stable task identifiers.
    #[serde(default)]
    pub next_task_sequence: u64,

    /// Pending timer state.
    pub timers: Vec<TimerState>,

    /// Active nonterminal task state.
    pub active_tasks: Vec<ActiveTask>,

    /// Saved history state configurations.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub history_store: HashMap<String, Vec<String>>,

    /// Active compensation logs.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub compensation_logs: HashMap<String, CompensationLog>,

    /// Process status.
    pub status: InstanceStatus,

    /// RFC3339 timestamp marking when the instance entered
    /// [`InstanceStatus::Stalled`] (ADR 0070 D-4.1). MUST be populated
    /// when `status == Stalled`; otherwise omitted from the wire.
    /// Schema-side guard at `wos-process.schema.json` enforces the
    /// `if status == "stalled" then stalledSince required` invariant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stalled_since: Option<String>,

    /// Reason provided when a signer or participant declined the envelope.
    /// MUST be populated when `status == Declined`; otherwise omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decline_reason: Option<String>,

    /// Actor identifier of the principal who voided the instance.
    /// MUST be populated when `status == Voided`; otherwise omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voided_by: Option<String>,

    /// RFC3339 timestamp when the instance was voided.
    /// MUST be populated when `status == Voided`; otherwise omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voided_at: Option<String>,

    /// RFC3339 timestamp when the instance expired (envelope deadline).
    /// MUST be populated when `status == Expired`; otherwise omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expired_at: Option<String>,

    /// Events enqueued but not yet processed.
    #[serde(default)]
    pub pending_events: Vec<PendingEvent>,

    /// Governance runtime state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance_state: Option<GovernanceState>,

    /// AI volume constraint counters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_counters: Option<VolumeCounters>,

    /// ISO 8601 creation timestamp.
    pub created_at: String,

    /// ISO 8601 last-modified timestamp.
    pub updated_at: String,

    /// Milestone identifiers that have already fired for this instance.
    ///
    /// Once a milestone condition first becomes true its id is recorded here,
    /// preventing it from firing again (Kernel S4.13 — once fired, stays fired).
    #[serde(default, skip_serializing_if = "std::collections::HashSet::is_empty")]
    pub fired_milestones: std::collections::HashSet<String>,

    /// Pending callback registrations awaiting inbound CloudEvents (NB.3).
    ///
    /// Keyed by the CloudEvents `subject` string used for correlation:
    /// `{correlationInstanceId}:{bindingId}:{invocationId}` where
    /// `correlationInstanceId` is `WorkflowProcess::correlation_process_id`.
    /// When a matching inbound event arrives its entry is removed and a
    /// `CallbackReceived` provenance record is emitted.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub pending_callbacks: HashMap<String, PendingCallback>,

    /// Extension data (keys prefixed with `x-`).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl WorkflowProcess {
    /// Mints a new process identifier.
    #[must_use]
    pub fn mint_id() -> String {
        typeid::mint_process_id()
    }

    /// Returns whether `value` already matches the reserved case TypeID shape.
    #[must_use]
    pub fn is_case_id(value: &str) -> bool {
        typeid::is_case_ledger_id(value)
    }

    /// Returns whether `value` matches the reserved process TypeID shape.
    #[must_use]
    pub fn is_process_id(value: &str) -> bool {
        typeid::is_process_id(value)
    }

    /// Returns whether `value` matches the WOS resource URN shape
    /// `urn:wos:<typeid>` per ADR 0092 D-1. Semantics: valid TypeID
    /// wrapped in the `urn:wos:` namespace prefix.
    #[must_use]
    pub fn is_instance_urn(value: &str) -> bool {
        let rest = match value.strip_prefix("urn:wos:") {
            Some(r) => r,
            None => return false,
        };
        typeid::is_valid_type_id(rest, None)
    }

    /// Extracts the tenant prefix from a WOS resource URN.
    ///
    /// Strips the `urn:wos:` prefix then extracts the TypeID tenant.
    /// Returns `None` when `value` is not a valid WOS resource URN or
    /// contains an invalid TypeID.
    #[must_use]
    pub fn extract_urn_parts(value: &str) -> Option<&str> {
        let rest = value.strip_prefix("urn:wos:")?;
        typeid::extract_tenant(rest)
    }

    /// Extracts the TypeID payload from a WOS resource URN.
    ///
    /// Returns `None` when `value` is not a valid `urn:wos:<typeid>` string.
    #[must_use]
    pub fn extract_urn_type_id(value: &str) -> Option<&str> {
        let rest = value.strip_prefix("urn:wos:")?;
        if typeid::is_valid_type_id(rest, None) {
            Some(rest)
        } else {
            None
        }
    }

    /// Returns the identifier used for external correlation (CloudEvents
    /// `subject`, broker routing keys, conformance fixtures).
    #[must_use]
    pub fn correlation_process_id(&self) -> &str {
        self.process_id.as_str()
    }
}

/// A callback registration that is waiting for a matching inbound CloudEvent (NB.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingCallback {
    /// Stable invocation identifier for this callback firing.
    pub invocation_id: String,

    /// Binding identifier that registered this callback.
    pub binding_id: String,

    /// ISO 8601 deadline after which this callback is considered expired, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_until: Option<String>,
}

/// Instance status (Runtime Companion S3.4; ADR 0070 D-4.1).
///
/// `Stalled` is the reserved status for instances that have exhausted their
/// adapter-side commit-retry budget without successful Trellis append (per
/// ADR 0070 maximalist Q18 / Q21). It is orthogonal to the kernel statechart
/// node taxonomy (`atomic | compound | parallel | final`) — `Stalled` is
/// instance execution metadata, not a lifecycle node type. When `status ==
/// Stalled`, [`WorkflowProcess::stalled_since`] MUST be populated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InstanceStatus {
    Active,
    Suspended,
    Migrating,
    Completed,
    Terminated,
    /// Adapter-side commit-retry budget exhausted; explicit operator
    /// intervention required (ADR 0070 D-4.1, OQ-2 — no auto-recovery).
    Stalled,
    /// Envelope signer or participant declined; instance carries
    /// [`WorkflowProcess::decline_reason`]. Tasks in the envelope are
    /// individually cancelled. Irreversible terminal state.
    Declined,
    /// Instance explicitly voided; carries [`WorkflowProcess::voided_by`]
    /// and [`WorkflowProcess::voided_at`]. Irreversible terminal state.
    Voided,
    /// Instance expired (envelope deadline elapsed); carries
    /// [`WorkflowProcess::expired_at`]. Irreversible terminal state.
    Expired,
}

/// Pending timer state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerState {
    /// Timer identifier.
    pub timer_id: String,
    /// Absolute deadline (ISO 8601).
    pub deadline: String,
    /// Event to emit when fired.
    pub event: String,
    /// State that scoped this timer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_state: Option<String>,

    /// Original ISO 8601 duration string, if preserved by the runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_iso: Option<String>,

    /// Original duration in milliseconds, if preserved by the runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Simulated time in milliseconds when this timer was created.
    ///
    /// Preserved across serialization so `business_deadline_ms` can always
    /// compute the calendar-adjusted deadline from the original creation time,
    /// regardless of how many times the instance has been drained and persisted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at_ms: Option<u64>,
}

/// Active nonterminal task state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTask {
    /// Stable task identifier.
    pub task_id: String,

    /// Task definition reference.
    pub task_ref: String,

    /// Current nonterminal task status.
    pub status: ActiveTaskStatus,

    /// Assigned actor identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_actor: Option<String>,

    /// Kernel contract reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_ref: Option<String>,

    /// Contract binding type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,

    /// Formspec Definition URL for Formspec-backed tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_url: Option<String>,

    /// Formspec Definition version for Formspec-backed tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_version: Option<String>,

    /// Mapping used to prefill a Formspec task response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefill_mapping_ref: Option<String>,

    /// Mapping used to project a completed Formspec response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_mapping_ref: Option<String>,

    /// Task deadline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,

    /// Task impact level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact_level: Option<crate::model::kernel::ImpactLevel>,

    /// Presentation context for a Formspec-backed task.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<FormspecTaskContext>,

    /// Last Formspec task validation outcome.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_validation_outcome: Option<ValidationOutcome>,

    /// Creation timestamp.
    pub created_at: String,

    /// Last update timestamp.
    pub updated_at: String,

    /// Extension data.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Active task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActiveTaskStatus {
    Created,
    Assigned,
    Claimed,
    Delegated,
    Escalated,
}

/// Presentation context for a Formspec-backed task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormspecTaskContext {
    /// Stable task identifier.
    pub task_id: String,

    /// Workflow process identifier.
    pub process_id: String,

    /// Kernel contract map key.
    pub contract_ref: String,

    /// Formspec Definition URL.
    pub definition_url: String,

    /// Formspec Definition version.
    pub definition_version: String,

    /// Task binding discriminator.
    pub binding: String,

    /// Assigned actor identifier.
    pub assigned_actor: String,

    /// Host-provided prefill data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefill_data: Option<serde_json::Value>,

    /// Mapping used to prefill the Response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefill_mapping_ref: Option<String>,

    /// Mapping used to project a completed Response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_mapping_ref: Option<String>,

    /// Task deadline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,

    /// Effective task impact level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact_level: Option<String>,

    /// Extension data.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Validation outcome for a Formspec-backed task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationOutcome {
    /// Whether the Response envelope is valid.
    pub envelope_valid: bool,

    /// Whether the Response pin matches the task pin.
    pub pin_match: bool,

    /// Whether Definition validation passed.
    pub definition_valid: bool,

    /// WOS-level validation errors.
    pub errors: Vec<serde_json::Value>,

    /// Formspec-shaped validation results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_results: Option<Vec<serde_json::Value>>,
}

/// A pending event in the instance queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingEvent {
    /// Event name.
    pub event: String,
    /// Actor who submitted the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
    /// Event payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Submission timestamp (ISO 8601).
    pub timestamp: String,
    /// Deduplication token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_token: Option<String>,
}

/// Governance runtime state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceState {
    /// Active delegation chains.
    #[serde(default)]
    pub active_delegations: Vec<ActiveDelegation>,
    /// Active holds.
    #[serde(default)]
    pub active_holds: Vec<ActiveHold>,
    /// Review protocol state (keyed by binding ID).
    #[serde(default)]
    pub review_state: HashMap<String, serde_json::Value>,
    /// Durable pending obligations (ADR 0096; WOS-OBL-MODEL-0803).
    ///
    /// Default-empty so process JSON written before this field existed
    /// deserializes unchanged and round-trips without the key.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_obligations: Vec<PendingObligation>,
    /// Replay-dedupe keys for obligation activation (ADR 0096; WOS-OBL-RUNTIME-0916).
    ///
    /// One entry per `(policyId, idempotencyToken)` pair already processed for
    /// activation, so re-draining an event carrying the same idempotency token
    /// does not materialize a second pending obligation for the same policy.
    /// Default-empty and omitted when empty so pre-existing process JSON
    /// deserializes unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seen_obligation_activation_keys: Vec<String>,
}

/// An active delegation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveDelegation {
    /// Delegating actor.
    pub delegator_id: String,
    /// Receiving actor.
    pub delegate_id: String,
    /// Delegation scope.
    pub scope: String,
    /// Authority type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority: Option<String>,
    /// Grant timestamp (ISO 8601).
    pub granted_at: String,
    /// Expiry timestamp (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// An active hold.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveHold {
    /// Hold type (same vocabulary as governance [`HoldPolicy`](crate::model::governance::HoldPolicy)).
    pub hold_type: HoldType,
    /// Start timestamp (ISO 8601).
    pub started_at: String,
    /// Expected end timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_end: Option<String>,
    /// Event that resumes the case.
    pub resume_trigger: String,
    /// State the instance was in when hold started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_state: Option<String>,
}

/// AI volume constraint counters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeCounters {
    /// Per-agent hourly counters.
    #[serde(default)]
    pub hourly: HashMap<String, VolumeCounter>,
    /// Per-agent daily counters.
    #[serde(default)]
    pub daily: HashMap<String, VolumeCounter>,
}

/// A single volume counter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeCounter {
    /// Count in the current window.
    pub count: u64,
    /// Window start (ISO 8601).
    pub window_start: String,
}

/// Compensation log for a compensable scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompensationLog {
    /// Scope identifier.
    pub scope_id: String,
    /// Entries in forward completion order.
    pub entries: Vec<CompensationEntry>,
}

/// A compensation log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompensationEntry {
    /// Original action ID.
    pub action_id: String,
    /// Original action type.
    pub action_type: String,
    /// Compensating action.
    pub compensating_action: serde_json::Value,
    /// Completion timestamp (ISO 8601).
    pub completed_at: String,
    /// Persisted output of the original action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
}
