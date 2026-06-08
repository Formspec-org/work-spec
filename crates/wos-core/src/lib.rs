// Rust guideline compliant 2026-02-21

//! Domain model and evaluation algorithm for WOS workflows.
//!
//! This crate owns the typed representation of WOS documents and the
//! deterministic lifecycle evaluation algorithm. It is format-independent:
//! consumers deserialize from JSON (or any other format) into these types,
//! and the evaluation logic operates on the typed model.
//!
//! # Architecture
//!
//! ```text
//!   wos-core (this crate)
//!      ↑           ↑           ↑
//!   wos-lint    wos-conformance  wos-runtime (future)
//! ```
//!
//! - `wos-lint` deserializes documents and runs static checks on the typed model.
//! - `wos-conformance` feeds events through the evaluation algorithm.
//! - A future `wos-runtime` adapts the algorithm to Temporal, Step Functions, etc.

pub mod activation;
pub mod agent;
pub mod autonomy;
pub mod business_calendar;
pub mod confidence;
pub mod context;
pub mod deontic;
pub mod eval;
pub mod eval_mode;
pub mod event_handler;
pub mod explain;
pub mod instance;
pub mod model;
pub mod project;
pub mod proxy;
pub mod studio_api;
pub mod timer;
pub mod traits;

#[cfg(test)]
mod provenance_tests;

pub use agent::{
    AgentContext, AgentInvocationError, AgentInvoker, AgentInvokerRegistry, AgentResult, AgentTask,
    InvokerKind, InvokerSpec, StubResponse,
};
pub use activation::{
    ActivationContext, ActivationDecision, ActivationDecisionReason, DeadlineHint,
    evaluate_activation_criteria,
};
pub use context::EvalContext;
pub use eval::{
    Configuration, EvalError, Evaluator, IndexedState, ObservedAction, ObservedTransition,
    parse_iso_duration_to_ms,
};
pub use model::activation::{
    ActivationCriteria, ActivationCriteriaRef, ActivationCriteriaUse, ActivationTrigger,
    ActorConstraint, EventKind, RequiredDataPath,
};
pub use model::ai::AIIntegrationDocument;
pub use model::obligation::{
    DuplicatePolicy, ObligationDeadline, ObligationPolicy, ObligationStatus,
    ObligationViolationAction, ObligationWarningThreshold, PendingObligation,
    ShorthandViolationAction, ViolationActionKind, ViolationActionSpec,
};
pub use model::business_calendar::BusinessCalendarDocument;
pub use model::governance::{GovernanceDocument, HoldType};
pub use model::kernel::{
    Action, ActionKind, KernelDocument, KernelView, SignalScope, State, StateKind,
    TimerEventSource, Transition, TransitionEvent, WorkflowDocument,
};
pub use model::kernel::{Actor, ActorKind, CaseFile, CaseRelationship, FieldDefinition};
pub use model::kernel::{ContractReference, EvaluationMode, ExecutionConfig};
pub use model::kernel::{ImpactLevel, Lifecycle, Milestone, Region};
pub use model::notification_template::NotificationTemplateDocument;
pub use project::Project;
pub use proxy::{AssistGovernanceProxyEvidence, observe_assist_governance_proxy};
pub use timer::Timers;
pub use wos_events::provenance::{
    AmendmentAuthorizedInput, AuthorizationAttestationInput, AuthorizationRejectedInput,
    CapabilityInvocationInput, CaseFileSnapshot, ClockResolvedInput, ClockResolvedResolution,
    ClockSkewObservedInput, ClockStartedInput, CommitAttemptFailureInput, CommitFailureKind,
    ConfigurationWarningInput, CorrectionAuthorizedInput, DeterminationAmendedInput,
    DeterminationRescindedInput, IdentityAttestationInput, InstanceMigratedInput, KeyRebindError,
    KeyRebindInput, MigrationPinChangedInput, ProvenanceAuditTier, ProvenanceKind, ProvenanceLog,
    ProvenanceRecord, ReinstatedInput, RescissionAuthorizedInput, SignatureAdmissionFailedInput,
    SignatureAffirmationInput, WOS_CANONICAL_EVENT_LITERALS, audit_layer_for_kind,
};
