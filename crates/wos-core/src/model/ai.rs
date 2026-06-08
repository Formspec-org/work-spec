// Rust guideline compliant 2026-02-21

//! Typed model for WOS AI Integration Documents (Layer 2).
//!
//! Deserialized from JSON via serde. AI integration documents target a
//! kernel workflow and attach agent declarations, deontic constraints,
//! autonomy levels, confidence framework, fallback chains, oversight
//! extensions, volume constraints, and drift detection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::model::activation::ActivationCriteria;

/// AI integration content — the embedded `aiOversight` block of a $wosWorkflow
/// document per ADR 0076 D-1. Was a standalone document with `$wosAIIntegration`
/// marker; the marker now lives on the envelope (`$wosWorkflow`) and this type
/// represents only the block's interior shape. Type name retained for consumer
/// compatibility; the standalone-document framing is gone.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIIntegrationDocument {
    /// Optional JSON Schema URI.
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,

    /// Kernel document this AI integration targets.
    #[serde(default)]
    pub target_workflow: String,

    /// Document version.
    #[serde(default)]
    pub version: Option<String>,

    /// Human-readable title.
    #[serde(default)]
    pub title: Option<String>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Default autonomy level for all agents.
    #[serde(default)]
    pub default_autonomy: Option<AutonomyLevel>,

    /// Agent declarations (AI Integration S3).
    pub agents: Vec<AgentDeclaration>,

    /// Deontic constraints (AI Integration S4).
    #[serde(default)]
    pub deontic_constraints: Option<DeonticConstraints>,

    /// Confidence floor (AI Integration S7.4).
    #[serde(default)]
    pub confidence_floor: Option<ConfidenceFloor>,

    /// Fallback chain (AI Integration S8).
    #[serde(default)]
    pub fallback_chain: Vec<FallbackLevel>,

    /// Oversight extensions (AI Integration S10).
    #[serde(default)]
    pub oversight_extensions: Option<OversightExtensions>,

    /// Volume constraints (AI Integration S11.1).
    #[serde(default)]
    pub volume_constraints: Option<VolumeConstraints>,

    /// Agent-specific review sampling (AI Integration S11.2).
    #[serde(default)]
    pub review_sampling: Option<AgentReviewSampling>,

    /// Agent disclosure requirements (AI Integration S12).
    #[serde(default)]
    pub agent_disclosure: Option<AgentDisclosure>,

    /// Narrative provenance tier (AI Integration S13).
    #[serde(default)]
    pub narrative_tier: Option<NarrativeTierConfig>,

    /// Drift detection (AI Integration S9).
    #[serde(default)]
    pub drift_detection: Option<DriftDetectionConfig>,

    /// Assist governance proxy (AI Integration S14).
    #[serde(default)]
    pub assist_governance_proxy: Option<AssistGovernanceProxy>,

    /// Extension data. Keys MUST start with `x-`.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Autonomy level (AI Integration S5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AutonomyLevel {
    Autonomous,
    Supervisory,
    Assistive,
    Manual,
}

/// Agent declaration (AI Integration S3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDeclaration {
    /// Unique agent identifier.
    pub id: String,

    /// Actor type (always `"agent"`).
    #[serde(rename = "type")]
    pub kind: String,

    /// Agent type taxonomy.
    pub agent_type: AgentType,

    /// Model identifier.
    pub model_identifier: String,

    /// Model version.
    pub model_version: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Agent capabilities.
    #[serde(default)]
    pub capabilities: Vec<Capability>,

    /// Model version policy.
    #[serde(default)]
    pub model_version_policy: Option<ModelVersionPolicy>,

    /// Confidence decay configuration.
    #[serde(default)]
    pub confidence_decay: Option<ConfidenceDecay>,

    /// Agent-level fallback chain override.
    #[serde(default)]
    pub fallback_chain: Vec<FallbackLevel>,

    /// Agent IDs this agent may invoke autonomously.
    #[serde(default)]
    pub cascading_invocations: Vec<String>,

    /// Agent-level deontic constraints (AI Integration S4.7).
    #[serde(default)]
    pub deontic_constraints: Option<DeonticConstraints>,

    /// Substrate adapter discriminator (ADR 0064). Selects which
    /// `AgentInvoker` implementation handles invocations of this agent.
    /// Optional so existing fixtures and authoring flows that have not yet
    /// declared an invoker continue to deserialize; the runtime fails fast
    /// when an agent is invoked without an `invoker`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoker: Option<crate::agent::InvokerSpec>,

    /// Extension data.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Agent type taxonomy (AI Integration S3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentType {
    Deterministic,
    Statistical,
    Generative,
}

/// Agent capability (AI Integration S3.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    /// Capability identifier.
    pub id: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Formspec Definition or JSON Schema for input.
    #[serde(default)]
    pub input_contract_ref: Option<String>,

    /// Formspec Definition or JSON Schema for output.
    #[serde(default)]
    pub output_contract_ref: Option<String>,

    /// FEL boolean expressions evaluated before capability invocation
    /// (AI Integration §3.3.1). All entries MUST evaluate to `true`;
    /// otherwise the capability is skipped and the processor falls
    /// through to the fallback chain.
    #[serde(default)]
    pub preconditions: Vec<String>,

    /// Optional activation criteria evaluated before capability invocation,
    /// offered additively alongside the FEL-string `preconditions` (ADR 0096;
    /// WOS-INTEG-AI-1701). Every entry MUST match (event/transition trigger,
    /// actor constraint, required-data presence, and/or FEL guard) for the
    /// capability to be invoked; otherwise the processor falls through to the
    /// fallback chain, exactly like an unmet `preconditions` entry.
    /// Backward-compatible: empty when absent and the existing
    /// `preconditions` semantics are unchanged.
    #[serde(
        default,
        rename = "preconditionCriteria",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub precondition_criteria: Vec<ActivationCriteria>,
}

/// Model version policy (AI Integration S3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelVersionPolicy {
    Pinned,
    Approved,
    Latest,
}

/// Deontic constraints (AI Integration S4).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeonticConstraints {
    /// Permissions.
    #[serde(default)]
    pub permissions: Vec<Permission>,

    /// Prohibitions.
    #[serde(default)]
    pub prohibitions: Vec<Prohibition>,

    /// Obligations.
    #[serde(default)]
    pub obligations: Vec<Obligation>,

    /// Rights.
    #[serde(default)]
    pub rights: Vec<Right>,
}

/// A deontic permission (AI Integration S4.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    /// Permission identifier.
    pub id: String,

    /// Action on violation.
    pub on_violation: ViolationAction,

    /// Fields the agent is allowed to access.
    #[serde(default)]
    pub allowed_fields: Vec<String>,

    /// Specific field for value-bounds permissions.
    #[serde(default)]
    pub field: Option<String>,

    /// FEL expression for value bounds.
    #[serde(default)]
    pub bounds: Option<String>,

    /// Null propagation behavior.
    #[serde(default)]
    pub null_behavior: Option<NullBehavior>,

    /// Whether this constraint can be bypassed.
    #[serde(default)]
    pub bypassable: bool,
}

/// A deontic prohibition (AI Integration S4.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prohibition {
    /// Prohibition identifier.
    pub id: String,

    /// FEL condition that triggers the prohibition.
    pub condition: String,

    /// Action on violation.
    pub on_violation: ViolationAction,

    /// Human-readable reason.
    #[serde(default)]
    pub reason: Option<String>,

    /// Null propagation behavior.
    #[serde(default)]
    pub null_behavior: Option<NullBehavior>,

    /// Whether this constraint can be bypassed.
    #[serde(default)]
    pub bypassable: bool,
}

/// A deontic obligation (AI Integration S4.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Obligation {
    /// Obligation identifier.
    pub id: String,

    /// FEL requirement (obligation unmet when false).
    pub requirement: String,

    /// Action on violation.
    pub on_violation: ViolationAction,

    /// Human-readable reason.
    #[serde(default)]
    pub reason: Option<String>,

    /// Null propagation behavior.
    #[serde(default)]
    pub null_behavior: Option<NullBehavior>,

    /// Whether this constraint can be bypassed.
    #[serde(default)]
    pub bypassable: bool,
}

/// A deontic right (AI Integration S4.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Right {
    /// Right identifier.
    pub id: String,

    /// Entitlement description.
    pub entitlement: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,
}

/// Action taken when a deontic constraint is violated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ViolationAction {
    Reject,
    EscalateToHuman,
    SwitchToAssistive,
    Flag,
}

/// Null propagation behavior for deontic constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NullBehavior {
    Pass,
    Deny,
    Escalate,
}

/// Confidence floor (AI Integration S7.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidenceFloor {
    /// Minimum confidence threshold [0.0, 1.0].
    pub threshold: f64,

    /// Action when confidence is below threshold.
    pub on_violation: ConfidenceViolationAction,
}

/// Action when confidence is below the floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfidenceViolationAction {
    EscalateToHuman,
    Reject,
}

/// Confidence decay (AI Integration S7.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidenceDecay {
    /// Whether decay is enabled.
    pub enabled: bool,

    /// Half-life duration (ISO 8601).
    #[serde(default)]
    pub half_life: Option<String>,

    /// Events that trigger confidence decay.
    #[serde(default)]
    pub triggers: Vec<DecayTrigger>,
}

/// A confidence decay trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecayTrigger {
    /// Event name that triggers decay.
    pub event: String,

    /// Factor multiplied against effective confidence [0.0, 1.0].
    pub decay_factor: f64,
}

/// Fallback chain level (AI Integration S8).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackLevel {
    /// Fallback action.
    pub action: FallbackAction,

    /// Task reference for `escalateToHuman`.
    #[serde(default)]
    pub task_ref: Option<String>,

    /// Maximum retry count.
    #[serde(default)]
    pub max_retries: Option<u32>,

    /// Backoff strategy.
    #[serde(default)]
    pub backoff: Option<BackoffStrategy>,

    /// Initial retry interval (ISO 8601).
    #[serde(default)]
    pub initial_interval: Option<String>,

    /// Actor to assign escalated tasks to.
    #[serde(default)]
    pub assign_to: Option<String>,

    /// Alternate agent reference.
    #[serde(default)]
    pub alternate_agent_ref: Option<String>,
}

/// Fallback action type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FallbackAction {
    EscalateToHuman,
    Retry,
    AlternateAgent,
    Fail,
}

/// Backoff strategy for retries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackoffStrategy {
    Fixed,
    Exponential,
    Linear,
}

/// Oversight extensions (AI Integration S10).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OversightExtensions {
    /// Whether agent output is suppressed during review.
    #[serde(default = "default_true")]
    pub suppress_agent_output: bool,

    /// Presentation configuration.
    #[serde(default)]
    pub presentation: Option<OversightPresentation>,
}

fn default_true() -> bool {
    true
}

/// Oversight presentation options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OversightPresentation {
    /// Show confidence scores to reviewer.
    #[serde(default)]
    pub show_confidence: bool,

    /// Show alternative outputs.
    #[serde(default)]
    pub show_alternatives: bool,

    /// Highlight fields with low confidence.
    #[serde(default)]
    pub highlight_low_confidence_fields: bool,

    /// Show diff from independent assessment.
    #[serde(default)]
    pub show_diff_from_independent: bool,
}

/// Volume constraints (AI Integration S11.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeConstraints {
    /// Maximum autonomous actions per hour.
    #[serde(default)]
    pub max_autonomous_per_hour: Option<u32>,

    /// Maximum autonomous actions per day.
    #[serde(default)]
    pub max_autonomous_per_day: Option<u32>,
}

/// Agent-specific review sampling (AI Integration S11.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentReviewSampling {
    /// Sampling rate [0.0, 1.0].
    pub rate: f64,

    /// Sampling method (adds `adversarial` over Layer 1).
    #[serde(default)]
    pub method: Option<AgentSamplingMethod>,

    /// Sampling scope.
    #[serde(default)]
    pub scope: Option<AgentSamplingScope>,
}

/// Agent sampling method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentSamplingMethod {
    Random,
    Stratified,
    Adversarial,
}

/// Agent sampling scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentSamplingScope {
    Workflow,
    Agent,
}

/// Agent disclosure requirements (AI Integration S12).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDisclosure {
    /// Disclose that an agent assisted.
    pub disclose_that_agent_assisted: bool,

    /// Disclose the model identity.
    #[serde(default)]
    pub disclose_model_identity: bool,

    /// Disclose confidence scores.
    #[serde(default)]
    pub disclose_confidence: bool,
}

/// Narrative provenance tier configuration (AI Integration S13).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeTierConfig {
    /// Whether the narrative tier is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Tags that require narrative tier records.
    #[serde(default)]
    pub required_for_tags: Vec<String>,
}

/// Drift detection configuration (AI Integration S9).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftDetectionConfig {
    /// Whether training data provenance is disclosed.
    #[serde(default)]
    pub training_data_disclosure: bool,

    /// Whether optimization objectives are disclosed.
    #[serde(default)]
    pub optimization_objective_disclosure: bool,

    /// Rubber stamp monitoring configuration.
    #[serde(default)]
    pub rubber_stamp_monitoring: Option<RubberStampConfig>,
}

/// Rubber stamp monitoring (AI Integration S9).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RubberStampConfig {
    /// Whether monitoring is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Minimum review time below which decisions are flagged (ISO 8601).
    #[serde(default)]
    pub min_review_time: Option<String>,

    /// Maximum agreement rate above which patterns are flagged [0.0, 1.0].
    #[serde(default)]
    pub max_agreement_rate: Option<f64>,

    /// Evaluation window (ISO 8601).
    #[serde(default)]
    pub evaluation_window: Option<String>,
}

/// Assist governance proxy (AI Integration S14).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistGovernanceProxy {
    /// Whether the proxy is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Tool category governance rules.
    #[serde(default)]
    pub tool_categories: Vec<ToolCategoryGovernance>,
}

/// Tool category governance (AI Integration S14).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCategoryGovernance {
    /// Tool category identifier.
    pub category: String,

    /// Deontic constraints for this category.
    #[serde(default)]
    pub constraints: Option<DeonticConstraints>,

    /// Maximum invocations per session.
    #[serde(default)]
    pub max_invocations_per_session: Option<u32>,
}
