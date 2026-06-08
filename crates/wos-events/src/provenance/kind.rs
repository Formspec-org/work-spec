// Rust guideline compliant 2026-02-21

use serde::{Deserialize, Serialize};

/// Provenance record type discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProvenanceKind {
    /// Lifecycle state transition.
    StateTransition,
    /// Event that matched no transition (Kernel S4.9).
    UnmatchedEvent,
    /// Case state field mutation (Kernel S5.4).
    CaseStateMutation,
    /// Governed case identity was created.
    ///
    /// `wos-events` owns this discriminant (`ProvenanceKind`); binding crates or
    /// host runtime code own payload assembly until WOS specifies a canonical
    /// binding-agnostic `caseCreated` data shape in spec and schema.
    CaseCreated,
    /// Intake handoff was accepted by host runtime policy.
    IntakeAccepted,
    /// A freeform note was appended to the case ledger by an authorized actor.
    ///
    /// Flat Facts-tier kind. `data` carries arbitrary note payload keyed by
    /// the calling adapter; no fixed schema is imposed beyond a key uniqueness
    /// constraint to avoid collision with kind-keyed overlays.
    NoteAdded,
    /// Intake handoff was rejected by host runtime policy.
    IntakeRejected,
    /// Intake handoff was deferred by host runtime policy.
    IntakeDeferred,
    /// Timer created (Lifecycle Detail S6.7).
    TimerCreated,
    /// Timer fired (Lifecycle Detail S6.7).
    TimerFired,

    // ── ForEach iteration (Kernel §4.3.1) ──────────────────────────
    /// One iteration of a `ForEach` state began. `data` carries
    /// `{"foreachState": "<state-id>", "index": <0-based>, "item": <value>}`
    /// so downstream replay can reconstruct iteration order. The matching
    /// completion record is [`ProvenanceKind::ForEachIterationCompleted`].
    /// Emitted once per iteration BEFORE body execution.
    ForEachIterationStarted,
    /// One iteration of a `ForEach` state finished. `data` carries
    /// `{"foreachState": "<state-id>", "index": <0-based>}` plus an
    /// optional `"breakTriggered": true` when iteration terminated early
    /// via `breakCondition`. Emitted once per iteration AFTER body
    /// execution (and AFTER per-iteration variable bindings are
    /// restored).
    ForEachIterationCompleted,
    /// All iterations of a `ForEach` state completed (or the empty-collection
    /// fast path fired). `data` carries `{"foreachState": "<state-id>",
    /// "iterations": <count>, "broke": <bool>}`. Emitted exactly once per
    /// foreach state entry, immediately before the foreach state's outgoing
    /// transition fires.
    ForEachCompleted,
    /// Timer cancelled (Lifecycle Detail S6.7).
    TimerCancelled,
    /// An `onEntry` lifecycle hook executed.
    OnEntry,
    /// An `onExit` lifecycle hook executed.
    OnExit,
    /// Action executed during onEntry, onExit, or transition.
    ActionExecuted,
    /// Duration string could not be parsed; timer deadline set to zero.
    InvalidDuration,
    /// Timer fired beyond its tolerance window (LCD S6.6, Runtime S7.2).
    ToleranceViolation,
    /// Continuous-mode re-evaluation hit the 100-cycle convergence cap for
    /// a single triggering mutation (Runtime §10.3). The record's `outcome`
    /// field carries the reserved literal `"convergenceCapReached"` (kernel
    /// `$defs/ProvenanceOutcome`); `data` carries `triggeringMutation` and
    /// `cyclesUsed` so downstream tooling can locate the cycle.
    ConvergenceCapReached,

    // ── Capability preconditions (AI S3.3.1) ───────────────────────
    /// A capability precondition was evaluated.
    ///
    /// Required `data` shape: `{"capabilityId": "<id>", "invocationBlocked":
    /// <bool>, ...}`. When `invocationBlocked` is `true` the record's
    /// `outcome` MUST be the reserved kernel literal
    /// `"preconditionNotSatisfied"` (Kernel §8.2.2). Schema-validated by
    /// `$defs/CapabilityInvocationRecord` in `wos-workflow.schema.json`,
    /// composed onto `FactsTierRecord` via `allOf` so every conformant
    /// provenance log participates in the MUST.
    CapabilityInvocation,

    // ── Deontic enforcement (AI S4) ────────────────────────────────
    /// A deontic constraint was violated (AI S4.2–S4.4).
    DeonticViolation,
    /// Deontic evaluation order record (AI S4.6).
    DeonticEvaluation,
    /// Resolved effective action from multiple violations (AI S4.6).
    DeonticResolution,
    /// Deontic constraint bypass with rationale (AI S4.7).
    DeonticBypass,
    /// Rights violation not attributed to agent (AI S4.5).
    RightsViolation,
    /// Consistency check contradiction (AI S4.7).
    ConsistencyViolation,

    // ── Autonomy (AI S5) ───────────────────────────────────────────
    /// Agent attempted to override a human decision (AI S3.7).
    AutonomyViolation,
    /// Autonomy level was capped by impact level or calibration (AI S5.3).
    AutonomyCapped,
    /// Effective autonomy computed from multiple sources (AI S5.3).
    AutonomyComputed,
    /// Assistive agent required human confirmation task (AI S5.3).
    HumanTaskCreated,
    /// Tool governance violation (AdvGov S6.1).
    ToolViolation,
    /// Escalation pending human approval (AI S5.4).
    EscalationPending,
    /// Autonomy demotion applied (AI S5.5).
    AutonomyDemotion,
    /// Autonomy escalation approved after configured escalation conditions.
    AutonomyEscalation,

    // ── Confidence (AI S7) ─────────────────────────────────────────
    /// Confidence violation — missing, uncalibrated, or below floor (AI S7).
    ConfidenceViolation,
    /// Confidence decay applied (AI S7.5).
    ConfidenceDecay,
    /// Cumulative confidence below threshold (AI S7.7).
    CumulativeConfidenceViolation,
    /// Session paused due to confidence threshold (AdvGov S5.4).
    SessionPaused,
    /// Ground truth label recorded from human review (AdvGov S9.3).
    GroundTruthLabel,

    // ── Agent lifecycle (AI S3, S6) ────────────────────────────────
    AgentOutput,
    ActorTypeViolation,
    AgentProvenanceAnnotation,
    AgentVersionChange,
    NarrativeTierRecorded,
    ConstraintTamperBlocked,
    DriftReclassification,
    AgentStateTransition,
    ProxyInvocation,
    DispositiveViolation,

    // ── Fallback (AI S8) ───────────────────────────────────────────
    FallbackTriggered,
    FallbackAttempt,
    FallbackTerminal,

    // ── Due process (WG S4, S6, S7) ────────────────────────────────
    NoticeSent,
    SeparationViolation,
    AppealFiled,
    ProtocolViolation,
    IndependentFirstEnforced,
    SamplingDecision,
    OverrideViolation,
    OverrideRecorded,
    /// Legal hold was placed over a case or record set.
    LegalHoldPlaced,
    /// Legal hold was released by authorized actor or policy.
    LegalHoldReleased,
    /// Destruction was rejected because an active legal hold applied.
    LegalHoldDestructionRejected,
    /// Continuation of services was activated for an appeal window.
    ContinuationOfServicesActivated,

    // ── Pipeline (WG S8) ───────────────────────────────────────────
    PipelineStageCompleted,
    PipelineRiskProfile,
    PipelineRejection,
    TaskCreated,
    TaskPresented,
    TaskDismissed,
    TaskDraftPersisted,
    TaskResponseSubmitted,
    TaskResponseRejected,
    DataMapping,
    TaskCompleted,
    TaskFailed,
    TaskSkipped,
    ParameterResolved,

    // ── Compensation (Kernel S9.8) ─────────────────────────────────
    CompensationLogEntry,
    CompensationExecuted,
    CompensationScopeBoundary,

    // ── Delegation (WG S9) ─────────────────────────────────────────
    DelegationViolation,

    // ── Durability (Kernel S10) ────────────────────────────────────
    /// Instance was suspended through a lifecycle-control operation.
    InstanceSuspended,
    InstanceResumed,
    /// Instance was terminated through a lifecycle-control operation.
    InstanceTerminated,
    StepResultPersisted,
    IdempotencyDedup,
    InstanceMigrated,
    ContractValidation,
    HistoryCleared,

    // ── DCR (Advanced Governance) ──────────────────────────────────
    DcrActivityExecuted,
    DcrRelationEvaluated,
    DcrResolutionError,
    ZoneSatisfied,
    /// DCR constraint zone violation was observed.
    DcrZoneViolation,
    EquityAlert,
    /// Circuit breaker opened because guarded failures crossed threshold.
    CircuitBreakerTripped,
    /// Circuit breaker reset after governed cooldown or recovery.
    CircuitBreakerReset,
    /// Shadow-mode output diverged materially from the configured baseline.
    ShadowModeDivergence,
    /// Drift monitoring produced an alert threshold crossing.
    DriftAlert,

    // ── Verification (Advanced Governance) ─────────────────────────
    VerificationReportProduced,
    ImmutabilityViolation,
    ActivationBlocked,

    // ── Sidecar (Business Calendar, Notification) ──────────────────
    CalendarIgnored,
    NotificationSuppressed,
    /// Report execution exceeded its wall-clock limit and was terminated.
    ReportTimedOut,

    // ── Configuration warnings (cross-cutting) ──────────────────────
    /// A configuration reference failed to resolve, or a configured
    /// operation (e.g. notification render) failed at runtime.
    ///
    /// Generic carrier for four spec MUSTs that require provenance for
    /// declarative-config failures without binding a more specific
    /// `recordKind`:
    /// - `specs/ai/drift-monitor.md:77` — unresolvable `policyRef`.
    /// - `specs/governance/workflow-governance.md:154` — unresolvable
    ///   `continuationPolicyRef`.
    /// - `specs/sidecars/notification-template.md:199` — template key not
    ///   found.
    /// - `specs/sidecars/notification-template.md:222` — notification
    ///   rendering failure.
    ///
    /// Required `data.subject` discriminator selects the failure site;
    /// reserved literals are `drift-monitor.policyRef`,
    /// `governance.continuationPolicyRef`, `notification-template.key`,
    /// `notification-template.render`. Vendor extensions use an `x-`
    /// prefix. Distinct from `CalendarIgnored` / `NotificationSuppressed`,
    /// which are sidecar-fallback semantics, not config-resolution
    /// failures.
    ConfigurationWarning,

    // ── Relationship provenance (Kernel S7) ────────────────────────
    RelationshipChanged,

    // ── Milestones (Kernel S4.13) ──────────────────────────────────
    /// A milestone condition became true for the first time (Kernel S4.13).
    ///
    /// `data` carries `{"milestoneId": "<id>"}`.
    MilestoneFired,

    // ── CloudEvents bindings (Integration Profile NB.3) ───────────
    /// An outbound CloudEvent was emitted by an `event-emit` binding.
    ///
    /// `data` carries the full CloudEvent envelope (all CE attributes + `data`).
    EventEmitted,

    /// An inbound CloudEvent was successfully consumed by an `event-consume` binding.
    ///
    /// `data` carries the full CloudEvent envelope (all CE attributes + `data`).
    EventConsumed,

    /// An inbound CloudEvent resolved a pending callback registered by a `callback` binding.
    ///
    /// `data` carries the full CloudEvent envelope and the `subject` used for correlation.
    CallbackReceived,

    /// A `callback` binding fired and is waiting for a matching inbound CloudEvent.
    ///
    /// `data` carries `{"subject": "<subject>", "bindingId": "<id>", "expectedUntil": "<iso>"}`.
    CallbackPending,

    // ── Arazzo / Tool / Policy-engine bindings (Integration Profile NB.4) ─
    /// A single step of an Arazzo multi-step sequence completed (or failed).
    ///
    /// `data` carries `{"stepId": "<id>", "outcome": "ok"|"failed", "durationMs": <n>, ...}`.
    ArazzoStep,

    /// A non-HTTP tool binding was invoked and produced a result.
    ///
    /// `data` carries `{"toolId": "<id>", "outcome": "ok"|"failed", ...}`.
    ToolInvoked,

    /// An external policy engine evaluated a request and returned a decision.
    ///
    /// `data` carries `{"decision": "allow"|"deny"|"indeterminate",
    /// "reasonsCount": <n>, "obligationsCount": <n>, ...}`.
    PolicyDecision,

    // ── Signature Profile (WOS-T4) ─────────────────────────────────
    /// A signer affirmed a document under a Signature Profile.
    ///
    /// `data` carries signer, role, document, identity-binding, consent,
    /// ceremony, profile, source response, and custody eligibility fields.
    SignatureAffirmation,
    /// Signature admission was rejected by the runtime.
    ///
    /// `data` carries `reason`, `evidenceBindings`, `signerId`, `signerAuthority`,
    /// and `emittedAt`. Counterpart to `SignatureAffirmation`; emitted when a
    /// signing gate (primitive verification, intent registration, posture floor,
    /// or adapter availability) rejects the submission.
    SignatureAdmissionFailed,

    // ── Amendment & supersession (ADR 0066) ─────────────────────────
    /// A correction to a non-determination event was authorized (ADR 0066 §1).
    ///
    /// Mode 1 of the closed five-mode supersession taxonomy
    /// (Correction / Amendment / Supersession / Rescission / Reinstatement).
    /// `data` carries `correctionTargetEventHash`, `correctedFieldSet`
    /// (JSON-pointer strings), `reason`, `authorizingActorId`, and
    /// `authorityBasis` (discriminated union: `{kind, value}` where
    /// `kind` is `"uri"` or `"actorPolicyRef"`).
    CorrectionAuthorized,
    /// An amendment to a prior determination was authorized (ADR 0066 §2).
    ///
    /// Mode 2 of the five-mode supersession taxonomy. Pairs with
    /// `DeterminationAmended`, which carries the new value. `data` carries
    /// `amendmentTargetEventHash`, `priorDeterminationHash`, `reason`,
    /// `authorizingActorId`, `authorityBasis`.
    AmendmentAuthorized,
    /// A determination was amended; new value supersedes the prior (ADR 0066 §2).
    ///
    /// `data` carries `priorDeterminationHash`, `newDeterminationValue`,
    /// `amendmentAuthorizationEventHash` (back-reference to the authorizing
    /// `AmendmentAuthorized` record).
    DeterminationAmended,
    /// A rescission of a prior determination was authorized (ADR 0066 §3).
    ///
    /// Mode 4 of the five-mode supersession taxonomy. `data` carries
    /// `rescissionTargetEventHash`, `priorDeterminationHash`, `reason`,
    /// `authorizingActorId`, `authorityBasis`, and an optional
    /// `migrationPinChange` payload (Q32 cross-chain hash linkage —
    /// supersession that also changes a version pin carries
    /// `{newChainPinEventHash, priorPinSet, newPinSet}`).
    RescissionAuthorized,
    /// A determination was rescinded (ADR 0066 §3).
    ///
    /// `data` carries `priorDeterminationHash` and
    /// `rescissionAuthorizationEventHash` (back-reference to the authorizing
    /// `RescissionAuthorized` record).
    DeterminationRescinded,
    /// A previously rescinded determination was reinstated (ADR 0066 §4).
    ///
    /// Mode 5 of the closed five-mode supersession taxonomy
    /// (owner directive Q26). Re-activates a determination from a
    /// non-operative (post-rescission) state and is distinct from amendment:
    /// the substantive value is unchanged; only the operative status flips
    /// back. `data` carries `priorRescissionEventHash`,
    /// `reactivationAuthorizationEventHash`, and `reason`.
    Reinstated,
    /// A standalone authorization attestation supporting an amendment,
    /// rescission, or reinstatement chain (ADR 0066 §5).
    ///
    /// Records the authorizing actor's policy basis for the supersession
    /// act. `data` carries `authorizingActorId`, `authorityBasis`,
    /// `policyPredicate` (e.g. `"amendment-authority"`,
    /// `"rescission-authority"`, `"reinstatement-authority"`), and an
    /// optional `assuranceLevel` (e.g. `"high"`, `"standard"`).
    AuthorizationAttestation,

    // ── Statutory clocks (ADR 0067) ──────────────────────────────
    /// A statutory or SLA clock was started (ADR 0067 §2).
    ///
    /// `data` carries `clockId`, `clockKind` (open enum:
    /// `"AppealClock"` | `"ProcessingSLA"` | `"GrantExpiry"` |
    /// `"StatuteClock"` | `x-*`), `originEventHash`, `duration`
    /// (ISO 8601 duration), `computedDeadline` (RFC 3339), and
    /// optional `calendarRef` and `statuteReference`.
    ClockStarted,
    /// A statutory or SLA clock resolved (ADR 0067 §3).
    ///
    /// `data` carries `clockId`, `originClockHash`, `resolution`
    /// (closed enum: `"satisfied"` | `"elapsed"` | `"paused"` |
    /// `"cancelled"` — see [`ClockResolvedResolution`], `resolvedAt`
    /// (RFC 3339), and `resolvingEventHash` (required when `resolution` is
    /// `"paused"`; otherwise optional).
    ClockResolved,

    // ── Identity attestation (ADR 0068) ──────────────────────────
    /// A cross-tenant identity attestation was recorded (ADR 0068 §4, Q15).
    ///
    /// Pulled inline as a first-class kind per maximalist Q15. `data`
    /// carries `subjectGlobalId`, `assuranceLevel` (open enum, e.g.
    /// `"low"` | `"standard"` | `"high"` | `"very-high"`),
    /// `attestationProvider`, `providerAttestationId`, `attestedAt`
    /// (RFC 3339), optional `validUntil` (RFC 3339), and
    /// `attestedPredicates` (open list, e.g. `["legal-name-verified",
    /// "age-of-majority"]`).
    IdentityAttestation,
    /// A subject rebound signing authority from one key identifier to another.
    ///
    /// `data` carries `priorKid`, `newKid`, `priorAssurance`, `newAssurance`,
    /// `rebindAttestationRef`, and optional `reason`. The forward-only
    /// assurance rule is semantic: the new assurance must rank at least as high
    /// as the prior assurance, so recovery cannot silently downgrade or upgrade
    /// the identity ceremony outside policy.
    KeyRebind,

    // ── Clock skew (ADR 0069) ────────────────────────────────────
    /// Clock skew between processor and substrate was observed (ADR 0069 §3).
    ///
    /// Emitted when processor and substrate timestamps diverge beyond the
    /// deployment-configured threshold (default: 1000ms). `data` carries
    /// `processorAuthoredAt` (RFC 3339), `substrateCreatedAt` (RFC 3339),
    /// `skewMilliseconds` (signed: positive = processor ahead),
    /// `thresholdMilliseconds`, and `eventHash`.
    ClockSkewObserved,

    // ── Failure & compensation (ADR 0070) ────────────────────────
    /// A commit attempt against the substrate failed (ADR 0070 §2).
    ///
    /// `data` carries `targetEventHash`, `failureKind` (closed enum
    /// via [`CommitFailureKind`]: `networkTimeout` | `substrateDown` |
    /// `hashConflict` | `other`), `attemptCount`,
    /// `retryBudgetRemainingMs`, and optional `errorPayload`.
    CommitAttemptFailure,
    /// An authorization attempt was rejected by policy (ADR 0070 §4).
    ///
    /// `data` carries `attemptedActorId`, `attemptedAction` (e.g.
    /// `"transition:approve"`, `"submit:taskResponse"`),
    /// `targetResourceId`, `rejectionReason`, and optional
    /// `policyDecisionRef`.
    AuthorizationRejected,

    // ── Migration & version pins (ADR 0071) ──────────────────────
    /// A migration pin set changed (ADR 0071 §3).
    ///
    /// `data` carries `priorPinSet` and `newPinSet` (4-field pin trees
    /// per maximalist Q33: `formspec.definitionVersion`,
    /// `wos.$wosWorkflowVersion`, `trellis.envelopeVersion`,
    /// `trellis.conformanceClass`), `authorizingActorId`,
    /// `authorityBasis`, and `migrationRationale`.
    MigrationPinChanged,

    // ── Durable obligations (ADR 0096) ───────────────────────────
    /// A durable obligation was activated (a `PendingObligation` created).
    ///
    /// `data` carries `policyId`, `obligationId`, optional `triggerEvent`,
    /// and optional `deadline` (Governance §16.2).
    ObligationActivated,
    /// A pending obligation was satisfied (discharged).
    ///
    /// `data` carries `policyId`, `obligationId`, and optional
    /// `satisfyingActor`.
    ObligationSatisfied,
    /// A pending obligation was violated (by a `violateWhen` match or, when
    /// distinct from expiry, a guarded event).
    ///
    /// `data` carries `policyId`, `obligationId`, `reason`, and
    /// `effectiveAction` (the strictest applied action; Governance §16.2.4).
    ObligationViolated,
    /// A pending obligation was cancelled (a `cancelWhen` match).
    ///
    /// `data` carries `policyId` and `obligationId`.
    ObligationCancelled,
    /// A pending obligation expired at its deadline.
    ///
    /// `data` carries `policyId`, `obligationId`, and `effectiveAction`.
    ObligationExpired,
    /// A pending obligation was bypassed by an authorized actor.
    ///
    /// `data` carries `policyId`, `obligationId`, `bypassActor`, and
    /// `rationale` (Governance §16.2.5).
    ObligationBypassed,
    /// A pre-breach warning fired for an obligation deadline.
    ///
    /// `data` carries `policyId`, `obligationId`, and `beforeBreach`. Does
    /// not change obligation status.
    ObligationWarning,
}

macro_rules! define_canonical_substrate_events {
    ($($lit:literal => $variant:ident,)*) => {
        /// Canonical WOS-owned event literals.
        ///
        /// The name says WOS because the table is WOS-specific. The original
        /// `WOS_CANONICAL_EVENT_LITERALS` symbol was retired in the
        /// Trellis DI-topology pass — Trellis substrate is producer-neutral,
        /// so a substrate-prefixed name on a WOS-only literal table was
        /// misleading. Phase-1 system (no released consumers); no alias kept.
        pub const WOS_CANONICAL_EVENT_LITERALS: &[&str] = &[$($lit),*];

        const CANONICAL_SUBSTRATE_EVENT_PAIRS: &[(&str, ProvenanceKind)] = &[
            $(($lit, ProvenanceKind::$variant),)*
        ];
    };
}

/// Wire prefix shared by `wos.governance.determination_*` substrate events.
///
/// Verification rules (for example rescission terminality in `trellis-verify-wos`)
/// treat any ledger `event_type` beginning with this prefix as determination-shaped.
pub const GOVERNANCE_DETERMINATION_WIRE_EVENT_PREFIX: &str = "wos.governance.determination";

define_canonical_substrate_events! {
    "wos.ai.capability_invocation" => CapabilityInvocation,
    "wos.assurance.identity_attestation" => IdentityAttestation,
    "wos.assurance.key_rebind" => KeyRebind,
    "wos.governance.amendment_authorized" => AmendmentAuthorized,
    "wos.governance.authorization_attestation" => AuthorizationAttestation,
    "wos.governance.authorization_rejected" => AuthorizationRejected,
    "wos.governance.clock_resolved" => ClockResolved,
    "wos.governance.clock_skew_observed" => ClockSkewObserved,
    "wos.governance.clock_started" => ClockStarted,
    "wos.governance.correction_authorized" => CorrectionAuthorized,
    "wos.governance.determination_amended" => DeterminationAmended,
    "wos.governance.determination_rescinded" => DeterminationRescinded,
    "wos.governance.reinstated" => Reinstated,
    "wos.governance.rescission_authorized" => RescissionAuthorized,
    "wos.kernel.case_created" => CaseCreated,
    "wos.kernel.commit_attempt_failure" => CommitAttemptFailure,
    "wos.kernel.for_each_completed" => ForEachCompleted,
    "wos.kernel.for_each_iteration_completed" => ForEachIterationCompleted,
    "wos.kernel.for_each_iteration_started" => ForEachIterationStarted,
    "wos.kernel.instance_migrated" => InstanceMigrated,
    "wos.kernel.intake_accepted" => IntakeAccepted,
    "wos.kernel.intake_deferred" => IntakeDeferred,
    "wos.kernel.intake_rejected" => IntakeRejected,
    "wos.kernel.migration_pin_changed" => MigrationPinChanged,
    "wos.kernel.note_added" => NoteAdded,
    "wos.kernel.signature_admission_failed" => SignatureAdmissionFailed,
    "wos.kernel.signature_affirmation" => SignatureAffirmation,
    "wos.kernel.state_transition" => StateTransition,
}

impl ProvenanceKind {
    /// Returns the registry-seeded D26 event literal, when one is fixed.
    #[must_use]
    pub fn canonical_event_literal(&self) -> Option<&'static str> {
        CANONICAL_SUBSTRATE_EVENT_PAIRS
            .iter()
            .find(|(_, kind)| kind == self)
            .map(|(literal, _)| *literal)
    }

    /// Returns the D26 record kind selected by a canonical event literal.
    #[must_use]
    pub fn from_canonical_event_literal(event: &str) -> Option<Self> {
        CANONICAL_SUBSTRATE_EVENT_PAIRS
            .iter()
            .find(|(literal, _)| *literal == event)
            .map(|(_, kind)| *kind)
    }

    /// Whether this kind represents a governance / AI policy or rule that
    /// applied during event processing.
    ///
    /// Used by the runtime to decide which records should have their `event`
    /// field stamped with the drain's processed event (for records whose
    /// constructors left `event = None` — the governance layer does this
    /// uniformly today), and by the conformance trace builder to decide
    /// which records contribute a `PolicyApplication` entry on a trace step.
    ///
    /// Semantics are "applied" not "violated". Violation-shaped kinds
    /// (`DeonticViolation`, `AutonomyViolation`, `ConfidenceViolation`, ...)
    /// signal that a rule FAILED, not that one applied, so they are
    /// intentionally excluded. `DeonticBypass` and `AutonomyDemotion` are
    /// semantically "policy overridden / demoted", not accept-and-fire —
    /// they are included because downstream teaching-signal consumers want
    /// to see that an override/demotion DID happen (with its rationale)
    /// when reasoning about a workflow's actual behaviour. Consumers can
    /// filter them out if they specifically want accept-and-fire semantics.
    pub fn is_policy_application(&self) -> bool {
        matches!(
            self,
            ProvenanceKind::DeonticEvaluation
                | ProvenanceKind::DeonticResolution
                | ProvenanceKind::DeonticBypass
                | ProvenanceKind::AutonomyComputed
                | ProvenanceKind::AutonomyDemotion
                | ProvenanceKind::AutonomyEscalation
                | ProvenanceKind::OverrideRecorded
                | ProvenanceKind::PolicyDecision
                | ProvenanceKind::PipelineRiskProfile
        )
    }
}

#[cfg(test)]
mod substrate_literal_registry_tests {
    use super::{ProvenanceKind, WOS_CANONICAL_EVENT_LITERALS};
    use std::collections::HashSet;

    #[test]
    fn given_substrate_literal_table_when_scanned_then_literals_are_unique_and_round_trip() {
        let mut seen = HashSet::new();
        for literal in WOS_CANONICAL_EVENT_LITERALS {
            assert!(
                seen.insert(*literal),
                "duplicate substrate canonical literal `{literal}`"
            );
            let kind = ProvenanceKind::from_canonical_event_literal(literal)
                .unwrap_or_else(|| panic!("literal `{literal}` must parse"));
            assert_eq!(
                kind.canonical_event_literal(),
                Some(*literal),
                "literal `{literal}` must round-trip through ProvenanceKind"
            );
        }
        assert_eq!(
            seen.len(),
            WOS_CANONICAL_EVENT_LITERALS.len(),
            "substrate literal table must enumerate distinct strings"
        );
    }

    #[test]
    fn given_each_substrate_literal_when_mapped_then_kind_has_some_canonical_literal() {
        for literal in WOS_CANONICAL_EVENT_LITERALS {
            let kind = ProvenanceKind::from_canonical_event_literal(literal).unwrap();
            assert!(
                kind.canonical_event_literal().is_some(),
                "{kind:?} must expose canonical_event_literal"
            );
        }
    }

    /// Given WOS kinds reachable from `wos-server` `case_events` direct append, when literals are sorted, then each is
    /// listed in the substrate registry so Trellis `WosEventAdmissionPolicy` and the HTTP façade stay aligned (TWREF-064).
    #[test]
    fn given_direct_case_append_kinds_when_sorted_then_literals_are_substrate_registered() {
        use ProvenanceKind::{
            CaseCreated, DeterminationRescinded, IdentityAttestation, IntakeAccepted, Reinstated,
        };
        let mut literals: Vec<&'static str> = vec![
            CaseCreated,
            IntakeAccepted,
            IdentityAttestation,
            DeterminationRescinded,
            Reinstated,
        ]
        .into_iter()
        .map(|kind| {
            kind.canonical_event_literal()
                .expect("direct-append kinds must carry substrate literals")
        })
        .collect();
        literals.sort_unstable();
        for literal in literals {
            assert!(
                WOS_CANONICAL_EVENT_LITERALS.contains(&literal),
                "{literal} must remain in WOS_CANONICAL_EVENT_LITERALS"
            );
        }
    }
}
