// Rust guideline compliant 2026-02-21

//! WOS-owned event and custody wire vocabulary.
//!
//! This crate owns the event payloads that cross the runtime/core boundary:
//! provenance record kinds, typed event constructors, and custody append
//! request/receipt shapes. Kernel model crates may re-export these types for
//! path compatibility, but they are defined here so event semantics are not
//! coupled to workflow evaluation internals.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod custody;
pub mod provenance;

pub use custody::{
    CustodyAppendContext, CustodyAppendError, CustodyAppendInput, CustodyAppendMetadata,
    CustodyAppendReceipt,
};
pub use provenance::{
    AmendmentAuthorizedInput, AuthorizationAttestationInput, AuthorizationRejectedInput,
    CapabilityInvocationInput, CaseFileSnapshot, ClockResolvedInput, ClockResolvedResolution,
    ClockSkewObservedInput, ClockStartedInput, CommitAttemptFailureInput, CommitFailureKind,
    ConfigurationWarningInput, CorrectionAuthorizedInput, DeterminationAmendedInput,
    DeterminationRescindedInput, GOVERNANCE_DETERMINATION_WIRE_EVENT_PREFIX,
    IdentityAttestationInput, InstanceMigratedInput, KeyRebindError, KeyRebindInput,
    MigrationPinChangedInput, ObligationViolationWitness, ProvenanceAuditTier, ProvenanceKind,
    ProvenanceLog, ProvenanceRecord, ReinstatedInput, RescissionAuthorizedInput,
    SignatureAdmissionFailedInput, SignatureAffirmationInput, WOS_CANONICAL_EVENT_LITERALS,
    audit_layer_for_kind,
};

/// Actor type (Kernel S3).
///
/// `Agent` is a first-class variant per ADR 0064. Agent-typed actors live in
/// the `actors[]` registry alongside humans and services; per-agent runtime
/// declarations (capabilities, autonomy, deontic constraints, fallback chain,
/// drift monitoring, invoker discriminator) live in the workflow's `agents[]`
/// embedded block joined by `id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ActorKind {
    Human,
    System,
    Agent,
}

/// Provenance audit layer for facts-tier records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditLayer {
    Facts,
    Reasoning,
    Counterfactual,
    Narrative,
}

/// Origin of a case-state mutation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MutationSource {
    HumanEntered,
    HumanCorrected,
    AgentExtracted,
    SystemFetched,
    Computed,
    SelfAttested,
    /// Vendor-namespaced source token. SHOULD carry an `x-` prefix.
    Vendor(String),
}

impl MutationSource {
    /// Canonical wire token for this mutation source.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::HumanEntered => "human-entered",
            Self::HumanCorrected => "human-corrected",
            Self::AgentExtracted => "agent-extracted",
            Self::SystemFetched => "system-fetched",
            Self::Computed => "computed",
            Self::SelfAttested => "self-attested",
            Self::Vendor(value) => value.as_str(),
        }
    }

    fn from_wire(value: &str) -> Option<Self> {
        match value {
            "human-entered" => Some(Self::HumanEntered),
            "human-corrected" => Some(Self::HumanCorrected),
            "agent-extracted" => Some(Self::AgentExtracted),
            "system-fetched" => Some(Self::SystemFetched),
            "computed" => Some(Self::Computed),
            "self-attested" => Some(Self::SelfAttested),
            value if value.starts_with("x-") => Some(Self::Vendor(value.to_string())),
            _ => None,
        }
    }
}

impl Serialize for MutationSource {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MutationSource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_wire(value.as_str()).ok_or_else(|| {
            D::Error::custom(format!(
                "invalid mutation source {value:?}; expected a reserved literal or x-* token"
            ))
        })
    }
}

/// Verification strength for a case-state mutation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VerificationLevel {
    Independent,
    Attested,
    Corroborated,
    Authoritative,
    /// Vendor-namespaced verification token. SHOULD carry an `x-` prefix.
    Vendor(String),
}

impl VerificationLevel {
    /// Canonical wire token for this verification level.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Independent => "independent",
            Self::Attested => "attested",
            Self::Corroborated => "corroborated",
            Self::Authoritative => "authoritative",
            Self::Vendor(value) => value.as_str(),
        }
    }

    fn from_wire(value: &str) -> Option<Self> {
        match value {
            "independent" => Some(Self::Independent),
            "attested" => Some(Self::Attested),
            "corroborated" => Some(Self::Corroborated),
            "authoritative" => Some(Self::Authoritative),
            value if value.starts_with("x-") => Some(Self::Vendor(value.to_string())),
            _ => None,
        }
    }
}

impl Serialize for VerificationLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for VerificationLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_wire(value.as_str()).ok_or_else(|| {
            D::Error::custom(format!(
                "invalid verification level {value:?}; expected a reserved literal or x-* token"
            ))
        })
    }
}
