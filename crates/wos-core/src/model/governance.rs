// Rust guideline compliant 2026-02-21

//! Typed model for WOS Workflow Governance Documents (Layer 1).
//!
//! Deserialized from JSON via serde. Governance documents target a
//! kernel workflow and attach due process, review protocols, data
//! validation pipelines, audit tiers, quality controls, delegation,
//! and hold policies.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

use crate::model::activation::ActivationCriteria;

/// Governance content — the embedded `governance` block of a $wosWorkflow
/// document per ADR 0076 D-1. Was a standalone document with
/// `$wosWorkflowGovernance` marker; the marker now lives on the envelope
/// (`$wosWorkflow`) and this type represents only the block's interior shape.
/// Type name retained for consumer compatibility; the standalone-document
/// framing is gone.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceDocument {
    /// Optional JSON Schema URI.
    #[serde(rename = "$schema", default)]
    pub schema: Option<String>,

    /// Kernel document this governance targets.
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

    /// Due process configuration (Governance S3).
    #[serde(default)]
    pub due_process: Option<DueProcess>,

    /// Review protocol bindings (Governance S4).
    #[serde(default)]
    pub review_protocols: Vec<ReviewProtocolBinding>,

    /// Data validation pipelines (Governance S5).
    #[serde(default)]
    pub pipelines: Vec<Pipeline>,

    /// Structured audit configuration (Governance S6).
    #[serde(default)]
    pub audit: Option<AuditConfig>,

    /// Quality controls (Governance S7).
    #[serde(default)]
    pub quality_controls: Option<QualityControls>,

    /// Task catalog patterns (Governance S9).
    #[serde(default)]
    pub task_catalog: Vec<TaskPattern>,

    /// Delegation declarations (Governance S11).
    #[serde(default)]
    pub delegations: Vec<Delegation>,

    /// Maximum delegation chain depth.
    #[serde(default = "default_max_delegation_depth")]
    pub max_delegation_depth: u32,

    /// Hold policies (Governance S12).
    #[serde(default)]
    pub hold_policies: Vec<HoldPolicy>,

    /// Extension data. Keys MUST start with `x-`.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

fn default_max_delegation_depth() -> u32 {
    1
}

/// Due process policy (Governance S3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DueProcess {
    /// FEL expression scoping when this policy applies.
    #[serde(default)]
    pub scope: Option<String>,

    /// Adverse decision policy.
    #[serde(default)]
    pub adverse_decision_policy: Option<AdverseDecisionPolicy>,
}

/// Adverse decision policy (Governance S3.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdverseDecisionPolicy {
    /// Whether notice is required before adverse decisions.
    #[serde(default)]
    pub notice_required: bool,

    /// When notice is delivered relative to the decision.
    #[serde(default)]
    pub notice_timing: Option<NoticeTiming>,

    /// Minimum delay between notice and effect (ISO 8601).
    #[serde(default)]
    pub notice_grace_period: Option<String>,

    /// Notification template key.
    #[serde(default)]
    pub notice_template_key: Option<String>,

    /// Required explanation level.
    #[serde(default)]
    pub explanation_level: Option<ExplanationLevel>,

    /// Whether counterfactual explanation is required.
    #[serde(default)]
    pub counterfactual_required: bool,

    /// Appeal mechanism configuration.
    #[serde(default)]
    pub appeal_mechanism: Option<AppealMechanism>,
}

/// Notice timing relative to adverse decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NoticeTiming {
    BeforeEffective,
    OnEffective,
    AfterEffective,
}

/// Explanation level for adverse decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExplanationLevel {
    Individualized,
    Categorical,
    Aggregate,
}

/// Appeal mechanism (Governance S3.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppealMechanism {
    /// Whether appeals are enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Appeal window duration (ISO 8601).
    #[serde(default)]
    pub appeal_window: Option<String>,

    /// Reviewer constraint description.
    #[serde(default)]
    pub reviewer_constraint: Option<String>,

    /// Allowed reviewer roles.
    #[serde(default)]
    pub reviewer_roles: Vec<String>,

    /// Whether services continue during appeal.
    #[serde(default)]
    pub continuation_of_services: bool,

    /// Scope of continuation.
    #[serde(default)]
    pub continuation_scope: Option<String>,
}

/// Review protocol binding (Governance S4).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewProtocolBinding {
    /// Semantic transition tags this binding matches.
    pub tags: Vec<String>,

    /// Review protocols to apply.
    pub protocols: Vec<ReviewProtocol>,

    /// Override: target a specific transition by ID.
    #[serde(default)]
    pub transition_override: Option<String>,

    /// FEL expression scoping when this binding applies.
    #[serde(default)]
    pub scope: Option<String>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,
}

/// Review protocol type (Governance S4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewProtocol {
    IndependentFirst,
    ConsiderOpposite,
    CalibratedConfidence,
    DualBlind,
    Unassisted,
}

/// Data validation pipeline (Governance S5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    /// Pipeline identifier.
    pub id: String,

    /// Ordered stages.
    pub stages: Vec<PipelineStage>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,
}

/// A pipeline stage (Governance S5.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStage {
    /// Stage identifier.
    pub id: String,

    /// Stage type.
    #[serde(rename = "type")]
    pub kind: StageKind,

    /// Contract reference for `contract-validation` stages.
    #[serde(default)]
    pub contract_ref: Option<String>,

    /// Assertions for `assertion-gate` stages.
    #[serde(default)]
    pub assertions: Vec<Assertion>,

    /// Rejection policy for this stage.
    #[serde(default)]
    pub rejection_policy: Option<RejectionPolicy>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Extension data.
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

/// Pipeline stage type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StageKind {
    ContractValidation,
    AssertionGate,
    Transform,
    HumanReview,
}

/// An assertion within an assertion gate (Governance S5.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assertion {
    /// Assertion type.
    #[serde(rename = "type")]
    pub kind: AssertionKind,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// FEL expression.
    #[serde(default)]
    pub expression: Option<String>,

    /// Fields referenced by this assertion.
    #[serde(default)]
    pub fields: Vec<String>,

    /// Reference stage for consistency checks.
    #[serde(default)]
    pub reference_stage: Option<String>,

    /// Per-assertion rejection policy override.
    #[serde(default)]
    pub rejection_policy: Option<RejectionPolicy>,
}

/// Assertion type (Governance S5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssertionKind {
    SourceGrounded,
    Arithmetic,
    Range,
    Consistency,
    Format,
    CrossDocument,
    Temporal,
}

/// Rejection policy (Governance S8.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RejectionPolicy {
    RetryWithCorrections,
    EscalateToSupervisor,
    HoldPendingData,
    FailWithExplanation,
}

/// Structured audit configuration (Governance S6).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditConfig {
    /// Reasoning tier configuration.
    #[serde(default)]
    pub reasoning_tier: Option<ReasoningTierConfig>,

    /// Counterfactual tier configuration.
    #[serde(default)]
    pub counterfactual_tier: Option<CounterfactualTierConfig>,
}

/// Reasoning tier configuration (Governance S6.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningTierConfig {
    /// Tags that require reasoning tier records.
    #[serde(default)]
    pub required_for_tags: Vec<String>,

    /// Whether decision requirements are mandatory.
    #[serde(default)]
    pub require_decision_requirements: bool,
}

/// Counterfactual tier configuration (Governance S6.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterfactualTierConfig {
    /// Tags that require counterfactual tier records.
    #[serde(default)]
    pub required_for_tags: Vec<String>,

    /// Whether negative counterfactuals must address protected characteristics.
    #[serde(default = "default_true")]
    pub require_protected_characteristics: bool,
}

fn default_true() -> bool {
    true
}

/// Quality controls (Governance S7).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityControls {
    /// Review sampling configuration.
    #[serde(default)]
    pub review_sampling: Option<ReviewSampling>,

    /// Separation of duties configuration.
    #[serde(default)]
    pub separation_of_duties: Option<SeparationOfDuties>,

    /// Override authority configuration.
    #[serde(default)]
    pub override_authority: Option<OverrideAuthority>,
}

/// Review sampling configuration (Governance S7.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSampling {
    /// Sampling rate [0.0, 1.0].
    pub rate: f64,

    /// Sampling method.
    #[serde(default)]
    pub method: Option<SamplingMethod>,

    /// Sampling scope.
    #[serde(default)]
    pub scope: Option<SamplingScope>,
}

/// Sampling method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SamplingMethod {
    Random,
    Stratified,
}

/// Sampling scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SamplingScope {
    Workflow,
    Actor,
}

/// Separation of duties (Governance S7.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeparationOfDuties {
    /// Scope of separation.
    pub scope: SeparationScope,

    /// Excluded roles.
    #[serde(default)]
    pub exclude_roles: Vec<String>,
}

/// Separation of duties scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SeparationScope {
    SameInstance,
    Global,
}

/// Override authority (Governance S7.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverrideAuthority {
    /// Whether a structured rationale is required.
    #[serde(default = "default_true")]
    pub require_structured_rationale: bool,

    /// Whether authority verification is required.
    #[serde(default = "default_true")]
    pub require_authority_verification: bool,

    /// Whether supporting evidence is required.
    #[serde(default)]
    pub require_supporting_evidence: bool,
}

/// Task catalog pattern (Governance S9.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPattern {
    /// Task pattern identifier.
    pub pattern: String,

    /// Verifiability level.
    pub verifiable: Verifiability,

    /// Verification method description.
    #[serde(default)]
    pub verification_method: Option<String>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,
}

/// Verifiability level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Verifiability {
    Yes,
    Partially,
    No,
}

/// Delegation of authority (Governance S11).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Delegation {
    /// Delegation identifier.
    pub id: String,

    /// Actor granting delegation.
    pub delegator: String,

    /// Actor receiving delegation.
    pub delegate: String,

    /// Scope of the delegation.
    pub scope: DelegationScope,

    /// Type of authority delegated.
    pub authority: DelegationAuthority,

    /// Legal instrument reference.
    #[serde(default)]
    pub legal_instrument: Option<String>,

    /// Effective date.
    #[serde(default)]
    pub effective_date: Option<String>,

    /// Expiration date.
    #[serde(default)]
    pub expiration_date: Option<String>,

    /// Whether the delegation can be revoked.
    #[serde(default = "default_true")]
    pub revocable: bool,

    /// Date the delegation was revoked.
    #[serde(default)]
    pub revoked_date: Option<String>,
}

/// Delegation authority type (Governance S11.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DelegationAuthority {
    Signing,
    Determination,
    Review,
    Override,
}

/// Delegation scope (Governance S11.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationScope {
    /// Impact levels in scope.
    #[serde(default)]
    pub impact_levels: Vec<String>,

    /// Case types in scope.
    #[serde(default)]
    pub case_types: Vec<String>,

    /// Maximum dollar threshold.
    #[serde(default)]
    pub max_dollar_threshold: Option<f64>,

    /// FEL condition.
    #[serde(default)]
    pub conditions: Option<String>,
}

/// Hold-type token for [`HoldPolicy`] and [`crate::instance::ActiveHold`].
///
/// Mirrors `HoldPolicy.holdType` in `wos-workflow.schema.json`: seven standard
/// values or a vendor token matching `^x-[a-z][a-z0-9-]*$`.
///
/// Standard-token parity with the schema enum arm is enforced by
/// `tests/hold_type_schema_enum.rs`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HoldType {
    PendingApplicantResponse,
    PendingExternalVerification,
    PendingLegalReview,
    PendingLegislation,
    PendingRelatedCase,
    VoluntaryHold,
    LegalHold,
    /// Vendor extension matching `^x-[a-z][a-z0-9-]*$`.
    Vendor(String),
}

impl HoldType {
    /// Canonical wire token for this hold type.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::PendingApplicantResponse => "pending-applicant-response",
            Self::PendingExternalVerification => "pending-external-verification",
            Self::PendingLegalReview => "pending-legal-review",
            Self::PendingLegislation => "pending-legislation",
            Self::PendingRelatedCase => "pending-related-case",
            Self::VoluntaryHold => "voluntary-hold",
            Self::LegalHold => "legal-hold",
            Self::Vendor(value) => value.as_str(),
        }
    }

    fn from_wire(value: &str) -> Option<Self> {
        match value {
            "pending-applicant-response" => Some(Self::PendingApplicantResponse),
            "pending-external-verification" => Some(Self::PendingExternalVerification),
            "pending-legal-review" => Some(Self::PendingLegalReview),
            "pending-legislation" => Some(Self::PendingLegislation),
            "pending-related-case" => Some(Self::PendingRelatedCase),
            "voluntary-hold" => Some(Self::VoluntaryHold),
            "legal-hold" => Some(Self::LegalHold),
            s if is_vendor_hold_type(s) => Some(Self::Vendor(s.to_string())),
            _ => None,
        }
    }
}

fn is_vendor_hold_type(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("x-") else {
        return false;
    };
    let mut chars = rest.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

impl Serialize for HoldType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for HoldType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_wire(value.as_str()).ok_or_else(|| {
            D::Error::custom(format!(
                "invalid holdType {value:?}; expected a standard HoldPolicy literal or x-* vendor token"
            ))
        })
    }
}

/// Hold policy (Governance S12).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoldPolicy {
    /// Hold type identifier.
    pub hold_type: HoldType,

    /// Expected duration (ISO 8601 or `"indefinite"`).
    pub expected_duration: String,

    /// Event that resumes from this hold.
    pub resume_trigger: String,

    /// Optional activation criteria that resumes the case from this hold,
    /// offered additively alongside `resume_trigger` (ADR 0096;
    /// WOS-INTEG-HOLD-1501). When present, the case resumes on the first match
    /// (e.g. an event plus an FEL guard, an actor constraint, or — via
    /// `on.event_scope == related` — a related-case event for a
    /// `pending-related-case` hold). Backward-compatible: `resume_trigger`
    /// stays required for non-legal holds and governs when this is absent.
    #[serde(
        default,
        rename = "resumeWhen",
        skip_serializing_if = "Option::is_none"
    )]
    pub resume_when: Option<ActivationCriteria>,

    /// Action when duration expires without resume trigger.
    pub timeout_action: TimeoutAction,

    /// Notification template key.
    #[serde(default)]
    pub notification_template_key: Option<String>,

    /// FEL expression scoping when this policy applies.
    #[serde(default)]
    pub scope: Option<String>,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,
}

/// Timeout action for hold policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TimeoutAction {
    Escalate,
    AutoResume,
    Cancel,
}

/// Rule reference with authority ranking (Governance S6.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleReference {
    /// Rule identifier.
    pub rule_id: String,

    /// Human-readable summary.
    #[serde(default)]
    pub description: Option<String>,

    /// Authority level for explanation ordering.
    #[serde(default)]
    pub source_authority: Option<SourceAuthority>,

    /// Formal citation.
    #[serde(default)]
    pub citation: Option<String>,
}

/// Source authority ranking (Governance S6.2).
///
/// Determines ordering in explanation assembly: statute (rank 1)
/// through guideline (rank 4). Default: policy (rank 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceAuthority {
    Statute,
    Regulation,
    Policy,
    Guideline,
}

impl SourceAuthority {
    /// Numeric rank for sorting (lower = higher authority).
    pub fn rank(self) -> u8 {
        match self {
            Self::Statute => 1,
            Self::Regulation => 2,
            Self::Policy => 3,
            Self::Guideline => 4,
        }
    }
}
