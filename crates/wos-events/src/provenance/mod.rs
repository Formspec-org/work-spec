// Rust guideline compliant 2026-02-21

//! Provenance recording for workflow execution.
//!
//! Every action that changes lifecycle or case state produces a provenance
//! record (Kernel S8). The provenance log is append-only.
//!
//! Split across `snapshot`, `kind`, `audit_tier`, `record`, and `log` so
//! `ProvenanceKind` growth does not monopolize a single merge-bottleneck file.
//! Binding-specific payload assembly stays outside this module unless WOS owns
//! the exact record shape in normative prose and schema. That keeps `wos-core`
//! aligned with shared provenance vocabulary instead of adapter-local evidence
//! conventions.

mod audit_tier;
mod kind;
mod log;
mod record;
mod snapshot;

pub use audit_tier::{ProvenanceAuditTier, audit_layer_for_kind};
pub use kind::{
    GOVERNANCE_DETERMINATION_WIRE_EVENT_PREFIX, ProvenanceKind, WOS_CANONICAL_EVENT_LITERALS,
};
pub use log::ProvenanceLog;
pub use record::{
    AmendmentAuthorizedInput, AuthorizationAttestationInput, AuthorizationRejectedInput,
    CapabilityInvocationInput, ClockResolvedInput, ClockResolvedResolution, ClockSkewObservedInput,
    ClockStartedInput, CommitAttemptFailureInput, CommitFailureKind, ConfigurationWarningInput,
    CorrectionAuthorizedInput, DeterminationAmendedInput, DeterminationRescindedInput,
    IdentityAttestationInput, InstanceMigratedInput, KeyRebindError, KeyRebindInput,
    MigrationPinChangedInput, ObligationViolationWitness, ProvenanceRecord, ReinstatedInput,
    RescissionAuthorizedInput, SignatureAdmissionFailedInput, SignatureAffirmationInput,
};
pub use snapshot::CaseFileSnapshot;
