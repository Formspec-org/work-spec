// Rust guideline compliant 2026-02-21

use super::kind::ProvenanceKind as K;

/// Closed audit-tier taxonomy for provenance records (SP §5.4, §6.5).
///
/// The tier for a record is deterministic from its [`ProvenanceKind`]. Only
/// [`ProvenanceKind::NarrativeTierRecorded`] maps to [`Self::Narrative`] today;
/// every other variant is a factual observation ([`Self::Facts`]). The
/// `"reasoning"` and `"counterfactual"` tiers (SP §5.4) are reserved for Layer
/// 1 injection paths not yet wired to a dedicated `ProvenanceKind` variant.
///
/// The [`From`] implementation is written exhaustively (no wildcard arm) so
/// that adding a new `ProvenanceKind` variant upstream forces the author to
/// consciously decide its tier — silent mis-classification via a wildcard
/// fallback is the exact failure mode this indirection is here to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProvenanceAuditTier {
    Facts,
    Narrative,
}

impl ProvenanceAuditTier {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Facts => "facts",
            Self::Narrative => "narrative",
        }
    }
}

impl From<K> for ProvenanceAuditTier {
    fn from(kind: K) -> Self {
        match kind {
            // Narrative tier (SP §5.4): the sole variant carrying narrative-layer
            // annotations today.
            K::NarrativeTierRecorded => Self::Narrative,

            // Facts tier (SP §5.4): every other variant records an observable
            // fact about workflow execution. Deliberately enumerated rather than
            // collapsed into `_` so a new variant triggers a compile error until
            // its tier is assigned.
            K::StateTransition
            | K::UnmatchedEvent
            | K::CaseStateMutation
            | K::CaseCreated
            | K::IntakeAccepted
            | K::IntakeRejected
            | K::IntakeDeferred
            | K::NoteAdded
            | K::TimerCreated
            | K::TimerFired
            | K::ForEachIterationStarted
            | K::ForEachIterationCompleted
            | K::ForEachCompleted
            | K::TimerCancelled
            | K::OnEntry
            | K::OnExit
            | K::ActionExecuted
            | K::InvalidDuration
            | K::ToleranceViolation
            | K::ConvergenceCapReached
            | K::CapabilityInvocation
            | K::DeonticViolation
            | K::DeonticEvaluation
            | K::DeonticResolution
            | K::DeonticBypass
            | K::RightsViolation
            | K::ConsistencyViolation
            | K::AutonomyViolation
            | K::AutonomyCapped
            | K::AutonomyComputed
            | K::HumanTaskCreated
            | K::ToolViolation
            | K::EscalationPending
            | K::AutonomyDemotion
            | K::AutonomyEscalation
            | K::ConfidenceViolation
            | K::ConfidenceDecay
            | K::CumulativeConfidenceViolation
            | K::SessionPaused
            | K::GroundTruthLabel
            | K::AgentOutput
            | K::ActorTypeViolation
            | K::AgentProvenanceAnnotation
            | K::AgentVersionChange
            | K::ConstraintTamperBlocked
            | K::DriftReclassification
            | K::AgentStateTransition
            | K::ProxyInvocation
            | K::DispositiveViolation
            | K::FallbackTriggered
            | K::FallbackAttempt
            | K::FallbackTerminal
            | K::NoticeSent
            | K::SeparationViolation
            | K::AppealFiled
            | K::ProtocolViolation
            | K::IndependentFirstEnforced
            | K::SamplingDecision
            | K::OverrideViolation
            | K::OverrideRecorded
            | K::LegalHoldPlaced
            | K::LegalHoldReleased
            | K::LegalHoldDestructionRejected
            | K::ContinuationOfServicesActivated
            | K::PipelineStageCompleted
            | K::PipelineRiskProfile
            | K::PipelineRejection
            | K::TaskCreated
            | K::TaskPresented
            | K::TaskDismissed
            | K::TaskDraftPersisted
            | K::TaskResponseSubmitted
            | K::TaskResponseRejected
            | K::DataMapping
            | K::TaskCompleted
            | K::TaskFailed
            | K::TaskSkipped
            | K::ParameterResolved
            | K::CompensationLogEntry
            | K::CompensationExecuted
            | K::CompensationScopeBoundary
            | K::DelegationViolation
            | K::InstanceSuspended
            | K::InstanceResumed
            | K::InstanceTerminated
            | K::StepResultPersisted
            | K::IdempotencyDedup
            | K::InstanceMigrated
            | K::ContractValidation
            | K::HistoryCleared
            | K::DcrActivityExecuted
            | K::DcrRelationEvaluated
            | K::DcrResolutionError
            | K::ZoneSatisfied
            | K::DcrZoneViolation
            | K::EquityAlert
            | K::CircuitBreakerTripped
            | K::CircuitBreakerReset
            | K::ShadowModeDivergence
            | K::DriftAlert
            | K::VerificationReportProduced
            | K::ImmutabilityViolation
            | K::ActivationBlocked
            | K::CalendarIgnored
            | K::NotificationSuppressed
            | K::ReportTimedOut
            | K::ConfigurationWarning
            | K::RelationshipChanged
            | K::MilestoneFired
            | K::EventEmitted
            | K::EventConsumed
            | K::CallbackReceived
            | K::CallbackPending
            | K::ArazzoStep
            | K::ToolInvoked
            | K::PolicyDecision
            | K::SignatureAffirmation
            | K::SignatureAdmissionFailed
            | K::CorrectionAuthorized
            | K::AmendmentAuthorized
            | K::DeterminationAmended
            | K::RescissionAuthorized
            | K::DeterminationRescinded
            | K::Reinstated
            | K::AuthorizationAttestation
            | K::ClockStarted
            | K::ClockResolved
            | K::IdentityAttestation
            | K::KeyRebind
            | K::ClockSkewObserved
            | K::CommitAttemptFailure
            | K::AuthorizationRejected
            | K::MigrationPinChanged
            | K::ObligationActivated
            | K::ObligationSatisfied
            | K::ObligationViolated
            | K::ObligationCancelled
            | K::ObligationExpired
            | K::ObligationBypassed
            | K::ObligationWarning => Self::Facts,
        }
    }
}

/// Classify a provenance record kind into its tier string (SP §5.4, §6.5).
///
/// Prefer [`ProvenanceAuditTier::from`] when you want the typed discriminant;
/// this function preserves the historical `&'static str` surface.
#[must_use]
pub fn audit_layer_for_kind(kind: K) -> &'static str {
    ProvenanceAuditTier::from(kind).as_str()
}
