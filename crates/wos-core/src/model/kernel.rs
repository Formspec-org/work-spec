// Rust guideline compliant 2026-02-21

//! Typed model for WOS workflows.
//!
//! After ADR 0076 the author-time envelope is one merged document
//! (`$wosWorkflow`) carrying lifecycle, actors, case file, contracts, plus
//! optional embedded blocks for governance, agents, AI oversight, signature,
//! custody, advanced, and assurance concerns. The canonical Rust name for
//! this shape is [`WorkflowDocument`]; [`KernelDocument`] is retained as the
//! historical alias used by ~200 call sites and refers to the same type.
//!
//! Consumers that only care about the kernel-relevant slice can borrow a
//! [`KernelView`] from a [`WorkflowDocument`] via [`WorkflowDocument::kernel_view`]
//! (or the alias `KernelDocument::kernel_view`). The view is a zero-cost
//! projection that exposes lifecycle, actors, case_file, contracts, and the
//! kernel execution config without copying.
//!
//! Deserialized from JSON via serde. The evaluation algorithm in [`crate::eval`]
//! operates on these types, not on raw `serde_json::Value`.

use indexmap::IndexMap;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

use crate::model::activation::ActivationCriteria;
use crate::model::decision_table::{DecisionTable, Guard};
pub use wos_events::{ActorKind, AuditLayer, MutationSource, VerificationLevel};

// `GovernanceDocument` (in `crate::model::governance`) provides the typed view
// over the `governance` embedded block; consumers deserialize on demand. The
// embedded field on this document is carried as raw `serde_json::Value` to
// keep deserialization tolerant of fixtures whose deeper nested shapes may
// not yet round-trip through the strict typed model.

/// The merged-envelope WOS workflow document (canonical name; see also the
/// [`KernelDocument`] alias). Carries the kernel surface (lifecycle, actors,
/// case file, contracts, execution) plus optional embedded blocks
/// (`governance`, `agents`, `aiOversight`, `signature`, `custody`, `advanced`,
/// `assurance`).
///
/// Embedded blocks are typed where a stable Rust shape exists (`governance`),
/// and carried as raw `serde_json::Value` otherwise. Consumers that need a
/// typed view of a raw block deserialize on demand into the corresponding
/// `model::ai`, advanced, or signature type.
///
/// Per ADR 0063, embedded blocks govern this enclosing envelope and never
/// declare `targetWorkflow` of their own. Sidecars (`$wosDelivery`,
/// `$wosOntologyAlignment`) are separate document types and DO target a
/// workflow URI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelDocument {
    /// Document type marker. Must be `"1.0"`.
    #[serde(rename = "$wosWorkflow")]
    pub wos_workflow: String,

    /// Optional JSON Schema URI for editor validation.
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,

    /// Canonical URL identifying this workflow definition.
    #[serde(default)]
    pub url: Option<String>,

    /// Document version.
    #[serde(default)]
    pub version: Option<String>,

    /// Human-readable title.
    #[serde(default)]
    pub title: Option<String>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Document status.
    #[serde(default)]
    pub status: Option<PublicationStatus>,

    /// Impact level classification (Kernel S6).
    #[serde(default)]
    pub impact_level: Option<ImpactLevel>,

    /// Actor declarations.
    #[serde(default)]
    pub actors: Vec<Actor>,

    /// Lifecycle topology.
    pub lifecycle: Lifecycle,

    /// Case state schema.
    #[serde(default)]
    pub case_file: Option<CaseFile>,

    /// Named contract references (Kernel S11).
    #[serde(default)]
    pub contracts: HashMap<String, ContractReference>,

    /// Provenance configuration (Kernel S8).
    #[serde(default)]
    pub provenance: Option<serde_json::Value>,

    /// Execution configuration (Kernel S9).
    #[serde(default)]
    pub execution: Option<ExecutionConfig>,

    /// Evaluation mode (Runtime Companion S10).
    #[serde(default)]
    pub evaluation_mode: Option<EvaluationMode>,

    /// Cascade depth cap for `$related.*` events (Kernel S4.10).
    #[serde(default)]
    pub max_relationship_event_depth: Option<u32>,

    // ── Embedded blocks (ADR 0076) ─────────────────────────────────────────
    //
    // Each block governs the enclosing envelope. Per ADR 0063, embedded blocks
    // MUST NOT declare targetWorkflow — that is a sidecar concept. Lint rule
    // WOS-EMBED-TARGET-001 catches violations at the JSON layer; this Rust
    // surface assumes well-formed envelopes.
    /// Embedded due-process / review-protocol / pipeline / task-catalog
    /// governance (Governance spec). Required for `rights-impacting` and
    /// `safety-impacting` workflows; the schema's `allOf` enforces this
    /// at parse time.
    ///
    /// Carried as raw JSON because fixtures exercise deep governance shapes
    /// (pipelines, review protocols, hold policies) that the strict typed
    /// model in `crate::model::governance::GovernanceDocument` does not yet
    /// round-trip cleanly. Consumers that need a typed view deserialize the
    /// `Value` into `GovernanceDocument` on demand:
    ///
    /// ```ignore
    /// if let Some(gov_value) = doc.governance.as_ref() {
    ///     let typed: GovernanceDocument = serde_json::from_value(gov_value.clone())?;
    /// }
    /// ```
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance: Option<serde_json::Value>,

    /// Per-agent runtime declarations (model identity, autonomy, deontic
    /// constraints, fallback chain, drift monitoring, invoker spec). Carried
    /// as raw JSON because the canonical typed shape
    /// (`crate::model::ai::AgentDeclaration`) is intentionally stricter than
    /// the schema and would refuse otherwise-valid envelopes; consumers that
    /// need typed access deserialize per-entry on demand.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<serde_json::Value>,

    /// Cross-cutting AI oversight (disclosure, drift detection, narrative
    /// tier, volume constraints). Pairs with `agents`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_oversight: Option<serde_json::Value>,

    /// DocuSign-tier signature workflow (roles, documents, signing flow,
    /// evidence). Required when any transition gates on `event.kind ==
    /// "signature"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<serde_json::Value>,

    /// Trellis custody binding (trust profile, anchor requirements). Loaded
    /// whenever a workflow claims anchoring on transitions or signature
    /// events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custody: Option<serde_json::Value>,

    /// Advanced governance (constraint zones, equity guardrails, verifiable
    /// constraints, circuit breaker, shadow mode).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advanced: Option<serde_json::Value>,

    /// Assurance level / attestation / subject continuity declarations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assurance: Option<serde_json::Value>,

    /// Public intake handoff configuration (per ADR 0073, formspec→WOS
    /// boundary).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intake: Option<serde_json::Value>,

    /// Output bindings (governed-output pipeline projections per ADR 0080).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bindings: Vec<serde_json::Value>,

    /// First-class decision tables per Kernel §4.5.1 (landed 2026-05-01).
    /// Each entry is referenced from a transition guard of the
    /// [`Guard::DecisionTable`] form. Empty when the workflow uses only
    /// FEL-string guards.
    #[serde(default)]
    pub decision_tables: Vec<DecisionTable>,

    /// Extension data. Keys MUST start with `x-`.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// `WorkflowDocument` is the canonical name for the merged-envelope
/// author-time document; `KernelDocument` is retained as the legacy alias for
/// the same type. New code SHOULD prefer `WorkflowDocument`; the alias exists
/// because ~200 call sites still spell it `KernelDocument`.
pub type WorkflowDocument = KernelDocument;

/// Borrow-only projection over a [`WorkflowDocument`] that exposes the
/// kernel-relevant slice (lifecycle, actors, case file, contracts, execution
/// config, evaluation mode, relationship-event depth). Constructed via
/// [`WorkflowDocument::kernel_view`]; zero-cost (just borrows the underlying
/// document).
///
/// Use a [`KernelView`] when you want to make it explicit that a function
/// only reads the kernel surface and ignores embedded blocks. The view does
/// not deep-copy; the lifetime ties the view to the source document.
#[derive(Debug, Clone, Copy)]
pub struct KernelView<'a> {
    doc: &'a WorkflowDocument,
}

impl<'a> KernelView<'a> {
    /// Construct a view over `doc`. Prefer [`WorkflowDocument::kernel_view`]
    /// at call sites for readability; this constructor exists for tests and
    /// adapter crates that don't import the inherent method.
    #[must_use]
    pub fn new(doc: &'a WorkflowDocument) -> Self {
        Self { doc }
    }

    /// The canonical workflow URL, if declared.
    #[must_use]
    pub fn url(&self) -> Option<&'a str> {
        self.doc.url.as_deref()
    }

    /// The workflow version, if declared.
    #[must_use]
    pub fn version(&self) -> Option<&'a str> {
        self.doc.version.as_deref()
    }

    /// The impact-level classification, if declared.
    #[must_use]
    pub fn impact_level(&self) -> Option<ImpactLevel> {
        self.doc.impact_level
    }

    /// Lifecycle topology (initial state + state map).
    #[must_use]
    pub fn lifecycle(&self) -> &'a Lifecycle {
        &self.doc.lifecycle
    }

    /// Actor declarations (humans, services, agents).
    #[must_use]
    pub fn actors(&self) -> &'a [Actor] {
        &self.doc.actors
    }

    /// Inline case-file shape, if declared.
    #[must_use]
    pub fn case_file(&self) -> Option<&'a CaseFile> {
        self.doc.case_file.as_ref()
    }

    /// Named contract references.
    #[must_use]
    pub fn contracts(&self) -> &'a HashMap<String, ContractReference> {
        &self.doc.contracts
    }

    /// Execution configuration (timeouts, compensability, instance
    /// versioning).
    #[must_use]
    pub fn execution(&self) -> Option<&'a ExecutionConfig> {
        self.doc.execution.as_ref()
    }

    /// Evaluation mode (`event-driven` or `continuous`).
    #[must_use]
    pub fn evaluation_mode(&self) -> Option<EvaluationMode> {
        self.doc.evaluation_mode
    }

    /// Cascade depth cap for `$related.*` events.
    #[must_use]
    pub fn max_relationship_event_depth(&self) -> Option<u32> {
        self.doc.max_relationship_event_depth
    }

    /// Underlying merged document, when a consumer needs to drop back to the
    /// full envelope (e.g., for a sibling embedded-block view).
    #[must_use]
    pub fn document(&self) -> &'a WorkflowDocument {
        self.doc
    }
}

impl KernelDocument {
    /// Borrow this document as a kernel-only projection. Zero-cost.
    #[must_use]
    pub fn kernel_view(&self) -> KernelView<'_> {
        KernelView { doc: self }
    }
}

/// A contract reference (Kernel S11).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractReference {
    /// Binding type.
    pub binding: String,

    /// Reference URI (Formspec Definition or JSON Schema).
    #[serde(rename = "ref")]
    pub reference: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Mapping used to prefill a Formspec task response.
    #[serde(default)]
    pub prefill_mapping_ref: Option<String>,

    /// Mapping used to project a completed Formspec response.
    #[serde(default)]
    pub response_mapping_ref: Option<String>,
}

/// Execution configuration (Kernel S9).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionConfig {
    /// Maximum total workflow duration (ISO 8601).
    #[serde(default)]
    pub workflow_timeout: Option<String>,

    /// Default task timeout (ISO 8601).
    #[serde(default)]
    pub default_task_timeout: Option<String>,

    /// Default service timeout (ISO 8601).
    #[serde(default)]
    pub default_service_timeout: Option<String>,

    /// Whether the workflow scope is compensable (Kernel S9.5).
    #[serde(default)]
    pub compensable: bool,

    /// Instance versioning policy.
    #[serde(default)]
    pub instance_versioning: Option<String>,

    /// Extension data.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Evaluation mode (Runtime Companion S10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvaluationMode {
    EventDriven,
    Continuous,
}

/// Publication status for a workflow document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PublicationStatus {
    Draft,
    Active,
    Deprecated,
}

/// Impact level classification (Kernel S6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImpactLevel {
    RightsImpacting,
    SafetyImpacting,
    Operational,
    Informational,
}

impl ImpactLevel {
    /// Whether this impact level requires due process (rights or safety).
    pub fn requires_due_process(self) -> bool {
        matches!(self, Self::RightsImpacting | Self::SafetyImpacting)
    }
}

/// An actor declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    /// Unique actor identifier.
    pub id: String,

    /// Actor type.
    #[serde(rename = "type")]
    pub kind: ActorKind,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Extension data.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Lifecycle topology (Kernel S4).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lifecycle {
    /// Initial state identifier.
    pub initial_state: String,

    /// Map of state identifiers to state definitions.
    pub states: IndexMap<String, State>,

    /// Named milestones.
    #[serde(default)]
    pub milestones: HashMap<String, Milestone>,
}

/// A lifecycle state (Kernel S4.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    /// State type.
    #[serde(rename = "type")]
    pub kind: StateKind,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Outgoing transitions, evaluated in document order.
    #[serde(default)]
    pub transitions: Vec<Transition>,

    /// Semantic tags for governance attachment.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Entry actions.
    #[serde(default)]
    pub on_entry: Vec<Action>,

    /// Exit actions.
    #[serde(default)]
    pub on_exit: Vec<Action>,

    /// Initial substate for compound states.
    #[serde(default)]
    pub initial_state: Option<String>,

    /// Substates for compound states.
    #[serde(default)]
    pub states: IndexMap<String, State>,

    /// Regions for parallel states.
    #[serde(default)]
    pub regions: IndexMap<String, Region>,

    /// Cancellation policy for parallel states.
    #[serde(default)]
    pub cancellation_policy: Option<CancellationPolicy>,

    /// History state mode for compound states.
    #[serde(default)]
    pub history_state: Option<HistoryMode>,

    /// Machine-readable outcome code for final states (Kernel S4.3).
    #[serde(default)]
    pub outcome_code: Option<String>,

    // ── ForEach iteration fields (Kernel S4.10 ForEach states) ─────────────
    //
    // Property names mirror the schema's `State` $def so authoring JSON
    // round-trips through the typed model without coercion. Authoring +
    // schema validity ship in this PR; full runtime iteration semantics are
    // tracked as Sub-PR D-2.
    /// FEL expression evaluated against case-state at entry into a `ForEach`
    /// state. MUST evaluate to a bounded array; each element drives one
    /// iteration of the body. Required when `kind == ForEach`; ignored on
    /// other state kinds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,

    /// Case-state binding name for the current iteration's item. Defaults to
    /// `"$item"` when omitted (matches the schema `itemVariable.default`).
    /// Authors reference the bound name inside the body (e.g.
    /// `$item.amount > 1000`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_variable: Option<String>,

    /// Case-state binding name for the current iteration's zero-based index.
    /// Defaults to `"$index"` when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_variable: Option<String>,

    /// Maximum number of items processed concurrently. Integer for bounded
    /// concurrency; `None` for unbounded (processor decides). Sequential
    /// iteration treats `Some(1)` and `None` identically; parallel iteration
    /// (Sub-PR D-2) honors the bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<u32>,

    /// FEL expression evaluated after each iteration; when true, terminates
    /// the foreach early. Useful for early-exit on first match or threshold
    /// reached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub break_condition: Option<String>,

    /// Case-file path where iteration results are written per
    /// `merge_strategy`. The write goes through the governed output-commit
    /// pipeline (ADR 0080).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,

    /// How per-iteration outputs are merged into `output_path`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_strategy: Option<MergeStrategy>,

    /// Body state executed once per iteration. Boxed because `State` is
    /// recursively-typed (the body MAY itself be a Compound or Parallel
    /// state with further nesting).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Box<State>>,

    /// Extension data.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// State type discriminator (Kernel S4.3).
///
/// `ForEach` is a compound-shaped state with iteration semantics: the body
/// subtree (rooted at `State::initial_state` and stored in `State::states`)
/// runs once per element of the FEL-evaluated `State::collection`. Per-iteration
/// case-state bindings expose the current item and index under
/// `State::item_variable` / `State::index_variable` (defaults `$item` / `$index`).
/// Sequential execution is the canonical semantics; parallel iteration capped
/// by `State::max_concurrency` is a future extension whose runtime
/// implementation is tracked separately. Authoring + schema validity ship in
/// this PR; full runtime iteration semantics in Sub-PR D-2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StateKind {
    Atomic,
    Compound,
    Parallel,
    /// JSON wire value is lowercase `foreach` (Kernel schema `lifecycle.states.*.type` enum), not default camelCase `forEach`.
    #[serde(rename = "foreach")]
    ForEach,
    Final,
}

/// Cancellation policy for parallel states (Kernel S4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CancellationPolicy {
    WaitAll,
    CancelSiblings,
    FailFast,
}

/// How per-iteration outputs of a `ForEach` state are merged into
/// [`State::output_path`] (Kernel S4.10 ForEach states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    /// Per-iteration output replaces top-level keys at `output_path`.
    Shallow,
    /// Per-iteration output deep-merges into existing structure at
    /// `output_path`.
    Deep,
    /// Per-iteration outputs accumulate as an array at `output_path`.
    Collect,
}

/// History state mode (Kernel S4.14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HistoryMode {
    Shallow,
    Deep,
}

/// Kernel-generated or author-declared timer categories for [`TransitionEvent::Timer`]
/// (Kernel §4.10, §9.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimerEventSource {
    Task,
    Service,
    State,
    Signal,
    Workflow,
    Custom,
}

/// Signal delivery scope for [`TransitionEvent::Signal`] (Kernel §4.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SignalScope {
    Instance,
    Related,
    Broadcast,
}

/// Typed transition / timer-fire event (Kernel §4.5–§4.10, TODO #20).
///
/// Replaces free-form event strings with a closed five-kind union. Runtime
/// `process_event` still receives string names; [`TransitionEvent::matches_runtime_dispatch`]
/// compares those names to this shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TransitionEvent {
    Timer {
        #[serde(rename = "timerId")]
        timer_id: String,
        source: TimerEventSource,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none", rename = "expiresAt")]
        expires_at: Option<String>,
        /// When set, the timer fires this exact string (e.g. dotted author names).
        #[serde(default, skip_serializing_if = "Option::is_none", rename = "firesAs")]
        fires_as: Option<String>,
    },
    Message {
        name: String,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            rename = "correlationKey"
        )]
        correlation_key: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    },
    Signal {
        name: String,
        scope: SignalScope,
    },
    Condition {
        expression: String,
    },
    Error {
        code: String,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            rename = "actionPath"
        )]
        action_path: Option<String>,
    },
}

impl TransitionEvent {
    /// String used for `process_event` matching and the same labels the
    /// runtime compares against [`Self::matches_runtime_dispatch`].
    #[must_use]
    pub fn runtime_dispatch_label(&self) -> String {
        match self {
            Self::Message { name, .. } | Self::Signal { name, .. } => name.clone(),
            Self::Timer {
                timer_id,
                source,
                fires_as,
                ..
            } => Self::timer_fires_string(timer_id, *source, fires_as.as_deref()),
            Self::Condition { expression } => format!("condition:{expression}"),
            Self::Error { .. } => "$error".to_string(),
        }
    }

    /// Human-readable event summary for graphs, notices, and search — not
    /// necessarily equal to [`Self::runtime_dispatch_label`] (e.g. `error`
    /// events carry a code in the typed object but dispatch as `$error`).
    #[must_use]
    pub fn authoring_display_label(&self) -> String {
        match self {
            Self::Error { code, .. } => format!("error:{code}"),
            Self::Condition { expression } => format!("condition:{expression}"),
            _ => self.runtime_dispatch_label(),
        }
    }

    #[must_use]
    pub fn matches_runtime_dispatch(&self, event: &str) -> bool {
        match self {
            Self::Message { name, .. } => name == event,
            Self::Signal { name, .. } => name == event,
            Self::Timer {
                timer_id,
                source,
                fires_as,
                ..
            } => Self::timer_fires_string(timer_id, *source, fires_as.as_deref()) == event,
            Self::Condition { .. } => false,
            Self::Error { .. } => event == "$error",
        }
    }

    /// Event name emitted when a `startTimer` timer expires (may differ from `timer_id`).
    #[must_use]
    pub fn start_timer_fires_string(&self) -> String {
        match self {
            Self::Timer {
                timer_id,
                source,
                fires_as,
                ..
            } => Self::timer_fires_string(timer_id, *source, fires_as.as_deref()),
            Self::Message { name, .. } => name.clone(),
            Self::Signal { name, .. } => name.clone(),
            Self::Condition { expression } => expression.clone(),
            Self::Error { code, .. } => code.clone(),
        }
    }

    fn timer_fires_string(
        timer_id: &str,
        source: TimerEventSource,
        fires_as: Option<&str>,
    ) -> String {
        if let Some(f) = fires_as {
            if !f.is_empty() {
                return f.to_string();
            }
        }
        match source {
            TimerEventSource::Task => "$timeout.task".to_string(),
            TimerEventSource::Service => "$timeout.service".to_string(),
            TimerEventSource::State => "$timeout.state".to_string(),
            TimerEventSource::Signal => "$timeout.signal".to_string(),
            TimerEventSource::Workflow => "$timeout.workflow".to_string(),
            TimerEventSource::Custom => format!("$timeout.{timer_id}"),
        }
    }

    /// Map a bare trigger token (authoring `add_transition`, tests) to the typed union.
    /// JSON documents may still supply the same strings under `transition.event`; serde
    /// uses the same coercion via [`transition_event_coerce_from_str`].
    #[must_use]
    pub fn from_authoring_trigger(s: &str) -> Self {
        transition_event_coerce_from_str(s)
    }
}

fn transition_event_coerce_from_str(s: &str) -> TransitionEvent {
    let s = s.trim();
    match s {
        "$join" => TransitionEvent::Signal {
            name: "$join".to_string(),
            scope: SignalScope::Instance,
        },
        "$error" => TransitionEvent::Error {
            code: "kernel.error".to_string(),
            action_path: None,
        },
        _ if s.starts_with("$timeout.") => {
            let rest = s.strip_prefix("$timeout.").unwrap_or(s);
            let source = match rest {
                "task" => TimerEventSource::Task,
                "service" => TimerEventSource::Service,
                "state" => TimerEventSource::State,
                "signal" => TimerEventSource::Signal,
                "workflow" => TimerEventSource::Workflow,
                _ => TimerEventSource::Custom,
            };
            TransitionEvent::Timer {
                timer_id: rest.to_string(),
                source,
                duration: None,
                expires_at: None,
                fires_as: None,
            }
        }
        _ if s.starts_with("$related.") => TransitionEvent::Signal {
            // Preserve the full `$related.*` name so that `matches_runtime_dispatch`
            // (which compares Signal.name == event by exact equality) matches the
            // kernel-defined relationship event names emitted at runtime
            // (`$related.stateChanged`, `$related.resolved`, `$related.holdReleased`,
            // …). Stripping the prefix here previously made bare-string transitions
            // for relationship events silently unmatchable.
            name: s.to_string(),
            scope: SignalScope::Related,
        },
        "$compensation.complete" => TransitionEvent::Signal {
            name: "$compensation.complete".to_string(),
            scope: SignalScope::Instance,
        },
        _ if s.starts_with('$') => TransitionEvent::Message {
            name: s.to_string(),
            correlation_key: None,
            data: None,
        },
        _ => TransitionEvent::Message {
            name: s.to_string(),
            correlation_key: None,
            data: None,
        },
    }
}

fn deserialize_opt_transition_event<'de, D>(
    deserializer: D,
) -> Result<Option<TransitionEvent>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Option::<serde_json::Value>::deserialize(deserializer)?;
    let Some(v) = v else {
        return Ok(None);
    };
    match v {
        serde_json::Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                Ok(None)
            } else {
                Ok(Some(transition_event_coerce_from_str(t)))
            }
        }
        serde_json::Value::Object(_) => serde_json::from_value::<TransitionEvent>(v)
            .map(Some)
            .map_err(D::Error::custom),
        _ => Err(D::Error::custom(
            "transition event must be a string or a TransitionEvent object",
        )),
    }
}

/// A parallel state region.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    /// Initial state within this region.
    pub initial_state: String,

    /// States within this region.
    #[serde(default)]
    pub states: IndexMap<String, State>,
}

/// A lifecycle transition (Kernel S4.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    /// Event that triggers this transition for explicit `process_event`
    /// delivery.
    ///
    /// When omitted (or serialized with only whitespace, treated as absent),
    /// the transition does **not** match any external event name. In
    /// `continuous` evaluation mode it still participates in the post-mutation
    /// guard re-scan (Runtime Companion §10.3).
    #[serde(
        default,
        deserialize_with = "deserialize_opt_transition_event",
        skip_serializing_if = "Option::is_none"
    )]
    pub event: Option<TransitionEvent>,

    /// Target state identifier.
    pub target: String,

    /// Transition guard per Kernel §4.5/§4.6 — evaluated in document order.
    ///
    /// Polymorphic per Kernel §4.5.1.1 (landed 2026-05-01): may be a FEL
    /// expression (string form) or a structured
    /// [`crate::model::decision_table::DecisionTableGuard`] (object form).
    /// `serde_json` deserializes either via the untagged
    /// [`Guard`] enum.
    ///
    /// Most existing wos-runtime / wos-lint paths walk only the FEL
    /// variant; use [`Guard::as_fel_str`] to preserve the legacy
    /// `Option<&str>` shape those call sites previously got from
    /// `transition.guard.as_deref()`.
    #[serde(default)]
    pub guard: Option<Guard>,

    /// Actions executed during this transition.
    #[serde(default)]
    pub actions: Vec<Action>,

    /// Actor responsible for this transition, when a single actor is named.
    /// MUST match `actors[].id` and the same lexical pattern as actor `id` values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Semantic tags.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Transition {
    /// Whether this transition participates in continuous-mode post-mutation
    /// guard re-evaluation (Runtime Companion §10.3).
    pub fn participates_in_continuous_rescan(&self) -> bool {
        match &self.event {
            None => true,
            Some(TransitionEvent::Condition { .. }) => true,
            Some(_) => false,
        }
    }

    /// Event label for diagnostics: matches runtime `process_event` names.
    #[must_use]
    pub fn event_dispatch_label(&self) -> String {
        self.event
            .as_ref()
            .map(TransitionEvent::runtime_dispatch_label)
            .unwrap_or_else(|| "(guard-only)".to_string())
    }

    /// Parallel join: signal `$join` scoped to this instance (Kernel §4.8).
    #[must_use]
    pub fn is_parallel_join_transition(&self) -> bool {
        matches!(
            &self.event,
            Some(TransitionEvent::Signal {
                name,
                scope: SignalScope::Instance
            }) if name == "$join"
        )
    }
}

/// A lifecycle action (Kernel S9.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    /// Action type.
    pub action: ActionKind,

    /// Task reference (createTask).
    #[serde(default)]
    pub task_ref: Option<String>,

    /// Actor to assign (createTask).
    #[serde(default)]
    pub assign_to: Option<String>,

    /// Service reference (invokeService).
    #[serde(default)]
    pub service_ref: Option<String>,

    /// Idempotency key (invokeService).
    #[serde(default)]
    pub idempotency_key: Option<String>,

    /// Correlation key (invokeService).
    #[serde(default)]
    pub correlation_key: Option<String>,

    /// Case file field path (setData).
    #[serde(default)]
    pub path: Option<String>,

    /// Value to set (setData).
    #[serde(default)]
    pub value: Option<serde_json::Value>,

    /// Event type (emitEvent).
    #[serde(default)]
    pub event_type: Option<String>,

    /// Event data (emitEvent).
    #[serde(default)]
    pub data: Option<serde_json::Value>,

    /// Timer identifier (startTimer, cancelTimer).
    #[serde(default)]
    pub timer_id: Option<String>,

    /// Duration (startTimer).
    #[serde(default)]
    pub duration: Option<String>,

    /// Deadline (startTimer).
    #[serde(default)]
    pub deadline: Option<String>,

    /// Event to fire (startTimer).
    #[serde(
        default,
        deserialize_with = "deserialize_opt_transition_event",
        skip_serializing_if = "Option::is_none"
    )]
    pub event: Option<TransitionEvent>,

    /// Log message (log).
    #[serde(default)]
    pub message: Option<String>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Contract reference for validation.
    #[serde(default)]
    pub contract_ref: Option<String>,

    /// Mapping used to prefill a Formspec task response.
    #[serde(default)]
    pub prefill_mapping_ref: Option<String>,

    /// Mapping used to project a completed Formspec response.
    #[serde(default)]
    pub response_mapping_ref: Option<String>,

    /// Event emitted when a Formspec-backed task completes.
    #[serde(default)]
    pub completion_event: Option<String>,

    /// Event emitted when a Formspec-backed task fails.
    #[serde(default)]
    pub failure_event: Option<String>,

    /// Compensating action (Kernel S9.5).
    #[serde(default)]
    pub compensating_action: Option<Box<Action>>,

    /// Extension data.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Action type discriminator (Kernel S9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActionKind {
    CreateTask,
    InvokeService,
    SetData,
    EmitEvent,
    StartTimer,
    CancelTimer,
    Log,
}

/// Case file schema (Kernel S5).
///
/// A case file is either declared inline via `fields` or referenced via
/// `contract_ref` (recommended binding: Formspec Definition through the
/// canonical `contractHook` seam, ADR 0077 §10.2). The two shapes are mutually
/// exclusive at the schema level (`oneOf`); the Rust model carries both as
/// `Option` and trusts the schema to enforce exclusivity at parse time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseFile {
    /// Inline field definitions. Mutually exclusive with `contract_ref` per
    /// the schema's `oneOf`. When omitted, the case-file shape comes from the
    /// referenced contract.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub fields: HashMap<String, FieldDefinition>,

    /// External contract reference. Mutually exclusive with `fields`. The URI
    /// names the contract document (Formspec Definition recommended, JSON
    /// Schema baseline). Resolved through the contracts-resolver port at
    /// runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_ref: Option<String>,

    /// Optional version pin for `contract_ref`. When omitted, processors
    /// resolve the latest published version. Pinning is RECOMMENDED for case
    /// instances that must replay against archived semantics (Kernel §9.6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_version: Option<String>,

    /// Case relationships (Kernel S5.5).
    #[serde(default)]
    pub relationships: Vec<CaseRelationship>,
}

/// A case file field definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// Field type.
    #[serde(rename = "type")]
    pub kind: String,

    /// Whether the field is required at instance creation / contract
    /// validation (Kernel §5; matches the schema's `FieldDeclaration.required`
    /// surface). Authoring-time hint; runtime contract validation enforces it
    /// through the contracts-resolver port.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,

    /// Default value.
    #[serde(default)]
    pub default: Option<serde_json::Value>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,
}

/// A case relationship (Kernel S5.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseRelationship {
    /// Relationship type.
    #[serde(rename = "type")]
    pub kind: String,

    /// URI of the related case.
    pub target_case: String,

    /// Semantic relationship label.
    #[serde(default)]
    pub relationship: Option<String>,

    /// Whether the inverse should also be recorded.
    #[serde(default)]
    pub bidirectional: bool,
}

/// A milestone condition (Kernel S4.13).
///
/// The milestone identifier is the map key in `Lifecycle.milestones`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    /// FEL condition expression.
    pub condition: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// When the processor evaluates the condition (Kernel §4.13).
    ///
    /// Defaults to `writeSettled`, the only currently defined mode: the
    /// condition is evaluated after every durable case-state write. Held as
    /// a string so future trigger modes can be added without breaking
    /// roundtrip serialization on existing documents.
    #[serde(
        default,
        rename = "triggerMode",
        skip_serializing_if = "Option::is_none"
    )]
    pub trigger_mode: Option<String>,

    /// Optional activation criteria offered additively alongside the FEL
    /// `condition` (ADR 0096; WOS-INTEG-MILE-1301). When present, the milestone
    /// fires when this reusable activation predicate matches (event/transition
    /// trigger, actor constraint, required-data presence, and/or FEL guard)
    /// rather than purely on the `condition` write-settled evaluation.
    /// Backward-compatible: `condition` remains required and governs when
    /// `activationCriteria` is absent.
    #[serde(
        default,
        rename = "activationCriteria",
        skip_serializing_if = "Option::is_none"
    )]
    pub activation_criteria: Option<ActivationCriteria>,
}
