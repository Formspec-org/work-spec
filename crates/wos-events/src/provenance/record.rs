// Rust guideline compliant 2026-02-21

use serde::{Deserialize, Deserializer, Serialize, de};
use stack_common_typeid as typeid;

use crate::{ActorKind, AuditLayer, MutationSource, VerificationLevel};

use super::kind::ProvenanceKind;
use super::snapshot::CaseFileSnapshot;

// F-13 event literals are consumed by Trellis custody/export verification and
// WOS D26 schema dispatch; changing them requires a coordinated registry
// migration and fixture regeneration.
/// Configuration-warning provenance input (cross-cutting; covers AI
/// `drift-monitor.policyRef`, governance `continuationPolicyRef`, and
/// notification-template key/render failures).
///
/// Carrier for the four spec MUSTs at `drift-monitor.md:77`,
/// `workflow-governance.md:154`, and `notification-template.md:199,222`.
/// `subject` is the discriminator literal naming the failure site; the
/// reserved set is `drift-monitor.policyRef`,
/// `governance.continuationPolicyRef`, `notification-template.key`,
/// `notification-template.render`. Vendor extensions use an `x-` prefix.
pub struct ConfigurationWarningInput<'a> {
    /// Failure-site discriminator (see type docstring for reserved set).
    pub subject: &'a str,
    /// The configuration reference that failed to resolve, when the
    /// failure mode is "ref unresolvable" (drift-monitor, governance,
    /// notification-template key). Omit for render-failure subjects
    /// where the failing identity is the template key carried in
    /// `context.templateKey`.
    pub unresolved_ref: Option<&'a str>,
    /// Additional context payload merged into `data` — failure reason
    /// string, the workflow URI, the case-file fields consulted at
    /// fallback time, etc. Keys in `context` that collide with the
    /// constructor's required fields (`subject`, `unresolvedRef`) are
    /// silently dropped: the typed input is the source of truth, and
    /// `context` (which may originate from caller-supplied scratch) MUST
    /// NOT overwrite the schema-shaping discriminators.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Capability-invocation provenance input (AI Integration §3.3.1).
///
/// Holds the precondition-evaluation outcome for an agent capability before
/// it is serialized into a `CapabilityInvocation` provenance record. The
/// constructor enforces the Kernel §8.2.2 invariant that a blocked
/// invocation carries the reserved outcome literal
/// `"preconditionNotSatisfied"`.
pub struct CapabilityInvocationInput<'a> {
    /// Capability identifier from the agent declaration (AI §3.3).
    pub capability_id: &'a str,
    /// Stable identifier for the agent actor that owns the capability.
    pub agent_id: &'a str,
    /// `true` when a precondition evaluated to non-`true` (false or
    /// non-boolean) and the processor skipped invocation; `false` when all
    /// preconditions passed and the capability proceeds.
    pub invocation_blocked: bool,
    /// Optional context payload merged into `data` — failed expression
    /// source, evaluation snapshot, fallback-chain reference, resolved
    /// precondition value, etc. Keys in `context` that collide with
    /// `capabilityId` / `invocationBlocked` are silently dropped: the
    /// agent declaration is the source of truth for capability identity,
    /// and `context` (which may originate from FEL-evaluator output or
    /// other untrusted scratch) MUST NOT be able to overwrite the
    /// schema-required discriminators that drive the
    /// `CapabilityInvocationRecord` if/then guard.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Resolution payload for [`ProvenanceRecord::clock_resolved`] (ADR 0067 §3).
///
/// `Paused` carries the pause-event hash at the type level so a paused
/// resolution cannot be constructed without it (Q11 maximalist; matches JSON
/// Schema `if/then` on `ClockResolvedRecord`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClockResolvedResolution<'a> {
    /// Terminal satisfied outcome; optional hash when a concrete resolving event exists.
    Satisfied {
        resolving_event_hash: Option<&'a str>,
    },
    /// Deadline elapsed without a resolving event (synthetic elapsed).
    Elapsed {
        resolving_event_hash: Option<&'a str>,
    },
    /// Cancelled (e.g. on supersession); optional hash of the cancelling event.
    Cancelled {
        resolving_event_hash: Option<&'a str>,
    },
    /// Paused — the pause event itself is the resolving event (hash required).
    Paused { resolving_event_hash: &'a str },
}

/// Closed failure-kind discriminant for
/// [`ProvenanceRecord::commit_attempt_failure`] (ADR 0070 §2). Typed at
/// the Rust seam so invalid failure kinds cannot reach the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommitFailureKind {
    NetworkTimeout,
    SubstrateDown,
    HashConflict,
    Other,
}

impl CommitFailureKind {
    fn as_camel_str(self) -> &'static str {
        match self {
            Self::NetworkTimeout => "networkTimeout",
            Self::SubstrateDown => "substrateDown",
            Self::HashConflict => "hashConflict",
            Self::Other => "other",
        }
    }
}

/// Amendment-authorization provenance input (ADR 0066 §2).
///
/// Authorizes a substantive change to a prior determination. Pairs with
/// [`DeterminationAmendedInput`] which carries the new value.
pub struct AmendmentAuthorizedInput<'a> {
    /// Hash of the event being amended (the amendment target).
    pub amendment_target_event_hash: &'a str,
    /// Hash of the prior determination value being superseded.
    pub prior_determination_hash: &'a str,
    /// Free-text rationale captured from the authorizing actor.
    pub reason: &'a str,
    /// Stable identifier for the actor authorizing the amendment.
    pub authorizing_actor_id: &'a str,
    /// Discriminated union: `{"kind": "uri", "value": "..."}` or
    /// `{"kind": "actorPolicyRef", "value": "..."}`.
    pub authority_basis: serde_json::Value,
    /// Optional context payload merged into `data`. Constructor's
    /// required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Authorization-attestation provenance input (ADR 0066 §5).
pub struct AuthorizationAttestationInput<'a> {
    /// Stable identifier for the attesting (authorizing) actor.
    pub authorizing_actor_id: &'a str,
    /// Discriminated-union policy basis (see
    /// [`AmendmentAuthorizedInput::authority_basis`]).
    pub authority_basis: serde_json::Value,
    /// Closed-namespace policy predicate. Reserved literals include
    /// `"amendment-authority"`, `"rescission-authority"`,
    /// `"reinstatement-authority"`.
    pub policy_predicate: &'a str,
    /// Optional assurance level (e.g. `"high"`, `"standard"`).
    pub assurance_level: Option<&'a str>,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Authorization-rejected provenance input (ADR 0070 §4).
pub struct AuthorizationRejectedInput<'a> {
    /// Identifier of the actor whose attempt was rejected.
    pub attempted_actor_id: &'a str,
    /// Action verb that was attempted, e.g. `"transition:approve"`,
    /// `"submit:taskResponse"`.
    pub attempted_action: &'a str,
    /// Identifier of the resource the actor tried to act upon.
    pub target_resource_id: &'a str,
    /// Free-text rationale (typically copied from the policy decision).
    pub rejection_reason: &'a str,
    /// Optional reference to the upstream policy decision record.
    pub policy_decision_ref: Option<&'a str>,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Clock-resolved provenance input (ADR 0067 §3).
pub struct ClockResolvedInput<'a> {
    /// Identifier of the clock that resolved.
    pub clock_id: &'a str,
    /// Hash of the originating `ClockStarted` event.
    pub origin_clock_hash: &'a str,
    /// Resolution outcome; see [`ClockResolvedResolution`].
    pub resolution: ClockResolvedResolution<'a>,
    /// RFC 3339 timestamp at which resolution occurred.
    pub resolved_at: &'a str,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Clock-skew-observed provenance input (ADR 0069 §3).
pub struct ClockSkewObservedInput<'a> {
    /// Processor-side authoring timestamp (RFC 3339).
    pub processor_authored_at: &'a str,
    /// Substrate-side creation timestamp (RFC 3339).
    pub substrate_created_at: &'a str,
    /// Signed skew (positive = processor ahead). Stored as i64 because
    /// negative values are valid observations.
    pub skew_milliseconds: i64,
    /// Configured threshold above which skew triggers a record.
    pub threshold_milliseconds: u64,
    /// Hash of the event whose timestamps revealed the skew.
    pub event_hash: &'a str,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Clock-started provenance input (ADR 0067 §2).
pub struct ClockStartedInput<'a> {
    /// Identifier of the new clock.
    pub clock_id: &'a str,
    /// Open-enum kind label (`"AppealClock"`, `"ProcessingSLA"`,
    /// `"GrantExpiry"`, `"StatuteClock"`, `x-*`).
    pub clock_kind: &'a str,
    /// Hash of the event that started the clock.
    pub origin_event_hash: &'a str,
    /// ISO 8601 duration string.
    pub duration: &'a str,
    /// Computed deadline (RFC 3339).
    pub computed_deadline: &'a str,
    /// Optional reference to a business calendar definition.
    pub calendar_ref: Option<&'a str>,
    /// Optional URI naming the governing statute.
    pub statute_reference: Option<&'a str>,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Commit-attempt-failure provenance input (ADR 0070 §2).
pub struct CommitAttemptFailureInput<'a> {
    /// Hash of the event whose commit attempt failed.
    pub target_event_hash: &'a str,
    /// Closed failure-kind discriminant.
    pub failure_kind: CommitFailureKind,
    /// Number of attempts that have occurred so far (1-based).
    pub attempt_count: u32,
    /// Remaining retry budget in milliseconds.
    pub retry_budget_remaining_ms: u64,
    /// Optional adapter-specific error payload.
    pub error_payload: Option<serde_json::Value>,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Correction-authorized provenance input (ADR 0066 §1).
///
/// Mode 1 of the closed five-mode supersession taxonomy. Records the
/// authorizing act for a non-determination correction (e.g. typo fix).
pub struct CorrectionAuthorizedInput<'a> {
    /// Hash of the event being corrected.
    pub correction_target_event_hash: &'a str,
    /// JSON-pointer strings naming the corrected fields.
    pub corrected_field_set: Vec<&'a str>,
    /// Free-text rationale.
    pub reason: &'a str,
    /// Identifier of the authorizing actor.
    pub authorizing_actor_id: &'a str,
    /// Discriminated-union policy basis.
    pub authority_basis: serde_json::Value,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Determination-amended provenance input (ADR 0066 §2).
pub struct DeterminationAmendedInput<'a> {
    /// Hash of the prior determination value being amended.
    pub prior_determination_hash: &'a str,
    /// New determination value (any JSON shape per binding).
    pub new_determination_value: serde_json::Value,
    /// Hash of the authorizing `AmendmentAuthorized` record.
    pub amendment_authorization_event_hash: &'a str,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Determination-rescinded provenance input (ADR 0066 §3).
pub struct DeterminationRescindedInput<'a> {
    /// Hash of the prior determination value being rescinded.
    pub prior_determination_hash: &'a str,
    /// Hash of the authorizing `RescissionAuthorized` record.
    pub rescission_authorization_event_hash: &'a str,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Identity-attestation provenance input (ADR 0068 §4, Q15).
pub struct IdentityAttestationInput<'a> {
    /// Cross-tenant subject identifier.
    pub subject_global_id: &'a str,
    /// Open-enum assurance level (`"low"` | `"standard"` | `"high"` |
    /// `"very-high"` | vendor extension).
    pub assurance_level: &'a str,
    /// Identifier of the attestation provider (issuer).
    pub attestation_provider: &'a str,
    /// Provider-issued identifier for this attestation event.
    pub provider_attestation_id: &'a str,
    /// RFC 3339 timestamp at which the provider attested.
    pub attested_at: &'a str,
    /// Optional RFC 3339 expiry of the attestation.
    pub valid_until: Option<&'a str>,
    /// Open-list of attested predicates (e.g.
    /// `["legal-name-verified", "age-of-majority"]`).
    pub attested_predicates: Vec<&'a str>,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Key-rebind recovery provenance input (ADR 0091).
pub struct KeyRebindInput<'a> {
    /// Sixteen-byte lowercase-hex key identifier for the prior signing key.
    pub prior_kid: &'a str,
    /// Sixteen-byte lowercase-hex key identifier for the replacement key.
    pub new_kid: &'a str,
    /// Assurance level held by the prior key binding.
    pub prior_assurance: &'a str,
    /// Assurance reached by the recovery ceremony.
    pub new_assurance: &'a str,
    /// URI for the recovery attestation evidence.
    pub rebind_attestation_ref: &'a str,
    /// Optional human-readable recovery rationale.
    pub reason: Option<&'a str>,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Error returned when a key-rebind record violates ADR 0091.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyRebindError {
    /// A key identifier is not a 16-byte lowercase-hex string.
    InvalidKid { field: &'static str, value: String },
    /// An assurance token has no portable ordering.
    UnrankedAssurance { field: &'static str, value: String },
    /// The recovery ceremony would lower the subject's assurance.
    AssuranceDowngrade { prior: String, new: String },
}

impl std::fmt::Display for KeyRebindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidKid { field, value } => {
                write!(f, "{field} must be 16-byte lowercase hex, got {value:?}")
            }
            Self::UnrankedAssurance { field, value } => {
                write!(f, "{field} has no portable assurance ordering: {value:?}")
            }
            Self::AssuranceDowngrade { prior, new } => {
                write!(
                    f,
                    "key rebind assurance downgrade rejected: prior={prior:?}, new={new:?}"
                )
            }
        }
    }
}

impl std::error::Error for KeyRebindError {}

/// Instance-migrated provenance input (Kernel S11.2, ADR 0083).
pub struct InstanceMigratedInput<'a> {
    /// Definition version before migration.
    pub from_version: &'a str,
    /// Definition version after migration.
    pub to_version: &'a str,
    /// JSON object matching Kernel S11.2 migration-map shape
    /// (`fieldRenames`, `fieldRemovals`, `fieldDefaults`, `fieldCoercions`).
    pub migration_map: serde_json::Value,
    /// Operator or system actor when known.
    pub actor_id: Option<&'a str>,
    /// Optional extra `data` fields; keys colliding with required fields drop.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Migration-pin-changed provenance input (ADR 0071 §3).
pub struct MigrationPinChangedInput<'a> {
    /// Prior 4-field pin tree (per maximalist Q33: `formspec.definitionVersion`,
    /// `wos.$wosWorkflowVersion`, `trellis.envelopeVersion`,
    /// `trellis.conformanceClass`).
    pub prior_pin_set: serde_json::Value,
    /// New 4-field pin tree (same shape as `prior_pin_set`).
    pub new_pin_set: serde_json::Value,
    /// Identifier of the actor authorizing the pin change.
    pub authorizing_actor_id: &'a str,
    /// Discriminated-union policy basis.
    pub authority_basis: serde_json::Value,
    /// Free-text rationale.
    pub migration_rationale: &'a str,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Reinstated provenance input (ADR 0066 §4 — Mode 5, owner directive Q26).
///
/// Re-activates a determination from a non-operative (post-rescission)
/// state. Distinct from amendment: the substantive value is unchanged;
/// only the operative status flips back.
pub struct ReinstatedInput<'a> {
    /// Hash of the prior `DeterminationRescinded` (or
    /// `RescissionAuthorized`) event being undone.
    pub prior_rescission_event_hash: &'a str,
    /// Hash of the authorizing `AuthorizationAttestation` record
    /// (predicate `"reinstatement-authority"`).
    pub reactivation_authorization_event_hash: &'a str,
    /// Free-text rationale.
    pub reason: &'a str,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Rescission-authorized provenance input (ADR 0066 §3).
///
/// Mode 4 of the closed five-mode supersession taxonomy. The optional
/// `migration_pin_change` carries the maximalist Q32 cross-chain hash
/// linkage for supersession that also changes a version pin.
pub struct RescissionAuthorizedInput<'a> {
    /// Hash of the event whose authorization is being rescinded.
    pub rescission_target_event_hash: &'a str,
    /// Hash of the prior determination value being rescinded.
    pub prior_determination_hash: &'a str,
    /// Free-text rationale.
    pub reason: &'a str,
    /// Identifier of the authorizing actor.
    pub authorizing_actor_id: &'a str,
    /// Discriminated-union policy basis.
    pub authority_basis: serde_json::Value,
    /// Optional cross-chain hash linkage when supersession also changes
    /// a version pin (Q32). Carries
    /// `{newChainPinEventHash, priorPinSet, newPinSet}`.
    pub migration_pin_change: Option<serde_json::Map<String, serde_json::Value>>,
    /// Optional context payload; required-field keys win on collision.
    pub context: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Signature affirmation provenance input.
///
/// Holds the required WOS Signature Profile evidence fields before they are
/// serialized into a `SignatureAffirmation` provenance record.
pub struct SignatureAffirmationInput<'a> {
    /// Stable signer identifier from the signature ceremony context.
    pub signer_id: &'a str,
    /// Signature Profile role id.
    pub role_id: &'a str,
    /// Signature Profile role literal.
    pub role: &'a str,
    /// Signature Profile document id.
    pub document_id: &'a str,
    /// Opaque identifier for the signing act. One signing act may produce
    /// multiple localized `SignatureAffirmation` records that share this id.
    pub signing_act_id: &'a str,
    /// Structured rendered-document reference carrying at least
    /// `{documentId, locale}` for K-2 downstream grouping.
    pub document_ref: serde_json::Value,
    /// Digest of the document bytes the signer affirmed.
    pub document_hash: &'a str,
    /// Digest of the rendered presentation bytes under
    /// `trellis-presentation-artifact-v1`.
    pub presentation_hash: &'a str,
    /// Digest algorithm used for `document_hash`.
    pub document_hash_algorithm: &'a str,
    /// Source system that supplied the verified signature evidence.
    pub source_signature_system: &'a str,
    /// Source-system signature identifier consumed by WOS.
    pub source_signature_id: &'a str,
    /// Source signature evidence signed-payload digest value.
    pub signed_payload_digest: &'a str,
    /// Digest algorithm used for `signed_payload_digest`.
    pub signed_payload_digest_algorithm: &'a str,
    /// WOS signing-intent URI accepted for this affirmation.
    pub signing_intent: &'a str,
    /// RFC 3339 timestamp for the signing act.
    pub signed_at: &'a str,
    /// Provider-neutral identity-binding evidence.
    pub identity_binding: serde_json::Value,
    /// Consent text/version and affirmation evidence reference.
    pub consent_reference: serde_json::Value,
    /// Signature provider identifier.
    pub signature_provider: &'a str,
    /// Provider or adapter ceremony identifier.
    pub ceremony_id: &'a str,
    /// URI reference to the Signature Profile, when cross-artifact.
    pub profile_ref: Option<&'a str>,
    /// Package-local Signature Profile key, when resolved in-document.
    pub profile_key: Option<&'a str>,
    /// URI reference to the source response or evidence artifact.
    pub source_response_ref: &'a str,
    /// Signer-authority claim when the accepted signing intent requires one.
    pub signer_authority: Option<serde_json::Value>,
    /// Whether the record is eligible for `custodyHook` admission.
    pub custody_hook_eligible: bool,
    /// Cryptographic primitive-verification status from the binding's
    /// signature parser, encoded as the JSON shape of
    /// `wos_runtime::SignaturePrimitiveStatus`
    /// (`{ "status": "verified" | "deferredPendingHelper" | "failed",
    /// "reason"?: "..." }`). Carried forward verbatim onto the
    /// `SignatureAffirmation` record so downstream verifiers can see whether
    /// the cryptographic primitive actually executed.
    pub primitive_verification: serde_json::Value,
    /// Optional base64-encoded COSE_Sign1 VerificationReceipt bytes.
    pub verification_receipt: Option<&'a str>,
    /// Optional reference edge to a witnessed signature, when this affirmation
    /// records a witness/counter-signature relationship.
    pub witnessed_signature_ref: Option<&'a str>,
}

/// Signature admission failure provenance input.
///
/// Holds the ADR-0089 payload for signatures that were evaluated but not
/// admitted into workflow state.
pub struct SignatureAdmissionFailedInput<'a> {
    /// Machine-readable failure reason.
    pub reason: &'a str,
    /// Source response id tied to the failed admission.
    pub response_id: &'a str,
    /// Signed-payload digest tied to the failed admission.
    pub signed_payload_digest: &'a str,
    /// Source-system signature id tied to the failed admission.
    pub signature_id: &'a str,
    /// Signing-intent URI tied to the failed admission.
    pub signing_intent: &'a str,
    /// Optional signer id when known.
    pub signer_id: Option<&'a str>,
    /// Optional signer-authority claim when present.
    pub signer_authority: Option<serde_json::Value>,
    /// Optional reason-specific structured context.
    pub failure_context: Option<serde_json::Map<String, serde_json::Value>>,
    /// RFC 3339 timestamp when the admission decision was emitted.
    pub emitted_at: &'a str,
}

/// Witness payload for [`ProvenanceRecord::obligation_violated`]
/// (ADR 0096; WOS-OBL-PROV-1103).
///
/// A violation record is a Facts-tier audit artifact that a regulator or
/// downstream verifier may read in isolation, so it carries enough context to
/// reconstruct *why* the obligation was deemed violated without re-running the
/// engine. The witness is the **minimized** evidence set, NOT a full snapshot.
///
/// # PII minimization (load-bearing)
///
/// `event_witness` and `case_state_witness` MUST carry only the JSON paths the
/// obligation's `ActivationCriteria` actually referenced (its `where` FEL
/// inputs, `requiredData` presence paths, and the trigger event's matched
/// fields) — never the full triggering event payload or the full case-file
/// snapshot. The caller (runtime obligation monitor) is responsible for
/// projecting those referenced paths before constructing the witness; this
/// constructor stores whatever subset it is handed verbatim. Keeping the
/// projection at the caller keeps WOS from leaking benefit amounts, SSNs, or
/// other sensitive fields into an audit record that may be exported more
/// widely than the case file itself. Both witness subsets are `Option` so a
/// violation with no referenced data (e.g. a pure deadline-elapsed violation)
/// emits no witness subset at all.
#[derive(Debug, Clone, Default)]
pub struct ObligationViolationWitness<'a> {
    /// Hash or URI of the event that triggered violation evaluation, when a
    /// concrete event drove the decision. Omit for time-driven violations.
    pub trigger_event: Option<&'a str>,
    /// The obligation's deadline (RFC 3339), when one applies.
    pub deadline: Option<&'a str>,
    /// Identifier of the actor responsible for discharging the obligation,
    /// when the obligation is actor-scoped.
    pub responsible_actor: Option<&'a str>,
    /// Role responsible for discharging the obligation, when role-scoped.
    pub responsible_role: Option<&'a str>,
    /// PII-minimized subset of the triggering event payload: ONLY the paths
    /// the obligation criteria referenced. MUST NOT be the full event.
    pub event_witness: Option<serde_json::Value>,
    /// PII-minimized subset of case-file state: ONLY the paths the obligation
    /// criteria referenced. MUST NOT be the full case-file snapshot.
    pub case_state_witness: Option<serde_json::Value>,
}

/// A single provenance record.
///
/// Records carry an RFC 3339 / ISO 8601 `timestamp` populated by the runtime
/// (or test harness) at the moment the record is appended to the instance log.
/// Constructors leave the field empty; the runtime stamps any empty timestamp
/// with the active clock before persisting the record (see
/// `wos_runtime::stamp_provenance`). Records produced in unit tests that never
/// reach the runtime may carry an empty `timestamp` — exporters and
/// downstream consumers (PROV-O, XES, OCEL) MUST treat an empty value as
/// "unknown" rather than emitting it verbatim.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceRecord {
    /// TypeID-structured identifier minted at authoring time.
    pub id: String,

    /// Record type.
    pub record_kind: ProvenanceKind,

    /// RFC 3339 / ISO 8601 timestamp set by the runtime when the record is
    /// appended to a log. Empty until stamped.
    #[serde(default)]
    pub timestamp: String,

    /// Actor who triggered the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,

    /// Source state (for transitions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_state: Option<String>,

    /// Target state (for transitions).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_state: Option<String>,

    /// Triggering event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,

    /// Additional context data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,

    /// Provenance tier: `"facts"`, `"reasoning"`, `"counterfactual"`, or
    /// `"narrative"` (SP §5.4, §6.5). Defaults to `"facts"` at construction;
    /// populated by the runtime tier classifier before persistence.
    /// Compatibility string surface; use [`ProvenanceRecord::audit_layer_kind`]
    /// and [`ProvenanceRecord::set_audit_layer_kind`] for typed access.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_layer: Option<String>,

    /// Actor type: `"human"`, `"system"`, or `"agent"` (SP §5.3, §5.5, §6.3).
    /// Populated at construction from the kernel `ActorKind` registry lookup
    /// (or from the AI Integration agent registry for `"agent"`).
    /// Compatibility string surface; use [`ProvenanceRecord::actor_kind`]
    /// and [`ProvenanceRecord::set_actor_kind`] for typed access.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<String>,

    /// Canonical lifecycle state at action time, distinct from `from_state`
    /// (which carries the pre-transition label). Maps to `wos:atLifecycleState`
    /// (PROV-O §5.3) and `wos:lifecycleState` (XES §6.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle_state: Option<String>,

    /// Version of the governing WOS Kernel Document (SP §5.3, §6.3).
    /// Populated from the workflow definition's `version` field at runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_version: Option<String>,

    /// Input entity references used by this activity (SP §5.3 `prov:used`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,

    /// Output entity references generated by this activity (SP §5.3
    /// `prov:wasGeneratedBy` inverse).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<String>,

    /// Tamper-detection digest for the inputs snapshot (SP §5.3, §6.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_digest: Option<String>,

    /// Tamper-detection digest for the outputs snapshot (SP §5.3, §6.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_digest: Option<String>,

    /// Trellis `canonical_event_hash` stamped after custody admission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_event_hash: Option<String>,

    /// Semantic tags copied from the firing transition.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transition_tags: Vec<String>,

    /// Case-file snapshot used by a determination-tagged transition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_file_snapshot: Option<CaseFileSnapshot>,

    /// Open-enum outcome literal recorded by the processor (Kernel §8.2.2).
    ///
    /// Optional; the kernel `$defs/ProvenanceOutcome` schema validates any
    /// populated value against the reserved-literal set
    /// (`preconditionNotSatisfied`, `convergenceCapReached`) plus an
    /// `x-`-prefixed vendor-extension fallback. The `skip_serializing_if`
    /// keeps existing fixtures byte-identical: records that leave the field
    /// `None` still serialize without an `"outcome"` key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
}

impl<'de> Deserialize<'de> for ProvenanceRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Wire {
            id: String,
            #[serde(default)]
            record_kind: Option<ProvenanceKind>,
            #[serde(default)]
            timestamp: String,
            #[serde(default)]
            actor_id: Option<String>,
            #[serde(default)]
            from_state: Option<String>,
            #[serde(default)]
            to_state: Option<String>,
            #[serde(default)]
            event: Option<String>,
            #[serde(default)]
            data: Option<serde_json::Value>,
            #[serde(default)]
            audit_layer: Option<String>,
            #[serde(default)]
            actor_type: Option<String>,
            #[serde(default)]
            lifecycle_state: Option<String>,
            #[serde(default)]
            definition_version: Option<String>,
            #[serde(default)]
            inputs: Vec<String>,
            #[serde(default)]
            outputs: Vec<String>,
            #[serde(default)]
            input_digest: Option<String>,
            #[serde(default)]
            output_digest: Option<String>,
            #[serde(default)]
            canonical_event_hash: Option<String>,
            #[serde(default)]
            transition_tags: Vec<String>,
            #[serde(default)]
            case_file_snapshot: Option<CaseFileSnapshot>,
            #[serde(default)]
            outcome: Option<String>,
        }

        let wire = Wire::deserialize(deserializer)?;
        let record_kind = match wire.record_kind {
            Some(record_kind) => record_kind,
            None => wire
                .event
                .as_deref()
                .and_then(ProvenanceKind::from_canonical_event_literal)
                .ok_or_else(|| de::Error::missing_field("recordKind"))?,
        };
        Ok(Self {
            id: wire.id,
            record_kind,
            timestamp: wire.timestamp,
            actor_id: wire.actor_id,
            from_state: wire.from_state,
            to_state: wire.to_state,
            event: wire.event,
            data: wire.data,
            audit_layer: wire.audit_layer,
            actor_type: wire.actor_type,
            lifecycle_state: wire.lifecycle_state,
            definition_version: wire.definition_version,
            inputs: wire.inputs,
            outputs: wire.outputs,
            input_digest: wire.input_digest,
            output_digest: wire.output_digest,
            canonical_event_hash: wire.canonical_event_hash,
            transition_tags: wire.transition_tags,
            case_file_snapshot: wire.case_file_snapshot,
            outcome: wire.outcome,
        })
    }
}

impl ProvenanceRecord {
    /// Mints a new provenance-record identifier.
    #[must_use]
    pub fn mint_id() -> String {
        typeid::mint_provenance_id()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn blank(record_kind: ProvenanceKind) -> Self {
        Self {
            id: Self::mint_id(),
            record_kind,
            timestamp: String::new(),
            actor_id: None,
            from_state: None,
            to_state: None,
            event: record_kind.canonical_event_literal().map(str::to_string),
            data: None,
            audit_layer: None,
            actor_type: None,
            lifecycle_state: None,
            definition_version: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            input_digest: None,
            output_digest: None,
            canonical_event_hash: None,
            transition_tags: Vec::new(),
            case_file_snapshot: None,
            outcome: None,
        }
    }

    /// Returns the typed actor kind, if the stored string is canonical.
    #[must_use]
    pub fn actor_kind(&self) -> Option<ActorKind> {
        match self.actor_type.as_deref() {
            Some("human") => Some(ActorKind::Human),
            Some("system") => Some(ActorKind::System),
            Some("agent") => Some(ActorKind::Agent),
            _ => None,
        }
    }

    /// Stores the canonical actor kind string.
    pub fn set_actor_kind(&mut self, actor_kind: ActorKind) {
        self.actor_type = Some(match actor_kind {
            ActorKind::Human => "human".to_string(),
            ActorKind::System => "system".to_string(),
            ActorKind::Agent => "agent".to_string(),
        });
    }

    /// Returns the typed audit layer, if the stored string is canonical.
    #[must_use]
    pub fn audit_layer_kind(&self) -> Option<AuditLayer> {
        match self.audit_layer.as_deref() {
            Some("facts") => Some(AuditLayer::Facts),
            Some("reasoning") => Some(AuditLayer::Reasoning),
            Some("counterfactual") => Some(AuditLayer::Counterfactual),
            Some("narrative") => Some(AuditLayer::Narrative),
            _ => None,
        }
    }

    /// Stores the canonical audit layer string.
    pub fn set_audit_layer_kind(&mut self, audit_layer: AuditLayer) {
        self.audit_layer = Some(match audit_layer {
            AuditLayer::Facts => "facts".to_string(),
            AuditLayer::Reasoning => "reasoning".to_string(),
            AuditLayer::Counterfactual => "counterfactual".to_string(),
            AuditLayer::Narrative => "narrative".to_string(),
        });
    }

    /// Create a state transition record.
    pub fn state_transition(from: &str, to: &str, event: &str, actor_id: Option<&str>) -> Self {
        let mut record = Self::blank(ProvenanceKind::StateTransition);
        record.actor_id = actor_id.map(String::from);
        record.from_state = Some(from.to_string());
        record.to_state = Some(to.to_string());
        record.data = Some(serde_json::json!({ "transitionEvent": event }));
        record
    }

    /// Create a state transition record with transition tags and an optional
    /// determination snapshot.
    pub fn tagged_state_transition(
        from: &str,
        to: &str,
        event: &str,
        actor_id: Option<&str>,
        transition_tags: &[String],
        case_file_snapshot: Option<CaseFileSnapshot>,
    ) -> Self {
        let mut record = Self::state_transition(from, to, event, actor_id);
        record.transition_tags = transition_tags.to_vec();
        record.case_file_snapshot = case_file_snapshot;
        record
    }

    /// Create an unmatched event record (Kernel S4.9).
    pub fn unmatched_event(event: &str, actor_id: Option<&str>) -> Self {
        let mut record = Self::blank(ProvenanceKind::UnmatchedEvent);
        record.actor_id = actor_id.map(String::from);
        record.event = Some(event.to_string());
        record
    }

    /// Create a case state mutation record (Kernel S5.4).
    pub fn case_state_mutation(
        path: &str,
        new_value: &serde_json::Value,
        actor_id: Option<&str>,
        lifecycle_state: &str,
    ) -> Self {
        Self::case_state_mutation_with_source(
            path,
            new_value,
            actor_id,
            lifecycle_state,
            None,
            None,
        )
    }

    /// Create a case state mutation with typed provenance vocab.
    pub fn case_state_mutation_with_source(
        path: &str,
        new_value: &serde_json::Value,
        actor_id: Option<&str>,
        lifecycle_state: &str,
        mutation_source: Option<MutationSource>,
        verification_level: Option<VerificationLevel>,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::CaseStateMutation);
        record.actor_id = actor_id.map(String::from);
        let mut data = serde_json::json!({
            "path": path,
            "newValue": new_value,
            "lifecycleState": lifecycle_state,
            "viaExplicitAction": true,
        });
        if let Some(src) = mutation_source {
            data["mutationSource"] = serde_json::to_value(src)
                .expect("mutationSource serializes to canonical JSON string");
        }
        if let Some(vl) = verification_level {
            data["verificationLevel"] = serde_json::to_value(vl)
                .expect("verificationLevel serializes to canonical JSON string");
        }
        record.data = Some(data);
        record.to_state = Some(lifecycle_state.to_string());
        record.lifecycle_state = Some(lifecycle_state.to_string());
        record
    }

    /// Create a timer created record (Lifecycle Detail S6.7).
    pub fn timer_created(timer_id: &str, duration: &str, fires_event: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::TimerCreated);
        record.data = Some(serde_json::json!({
            "timerId": timer_id,
            "duration": duration,
            "firesEvent": fires_event,
        }));
        record
    }

    /// Create a timer fired record (Lifecycle Detail S6.7).
    pub fn timer_fired(timer_id: &str, fires_event: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::TimerFired);
        record.data = Some(serde_json::json!({
            "timerId": timer_id,
            "firesEvent": fires_event,
        }));
        record
    }

    // ── Durable obligation builders (ADR 0096; WOS-OBL-PROV-1102) ───────────

    /// A durable obligation was activated (a `PendingObligation` created).
    pub fn obligation_activated(
        policy_id: &str,
        obligation_id: &str,
        trigger_event: Option<&str>,
        deadline: Option<&str>,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::ObligationActivated);
        let mut data = serde_json::json!({
            "policyId": policy_id,
            "obligationId": obligation_id,
        });
        if let Some(event) = trigger_event {
            data["triggerEvent"] = serde_json::Value::String(event.to_string());
        }
        if let Some(deadline) = deadline {
            data["deadline"] = serde_json::Value::String(deadline.to_string());
        }
        record.data = Some(data);
        record
    }

    /// A pending obligation was satisfied (discharged).
    pub fn obligation_satisfied(
        policy_id: &str,
        obligation_id: &str,
        satisfying_actor: Option<&str>,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::ObligationSatisfied);
        let mut data = serde_json::json!({
            "policyId": policy_id,
            "obligationId": obligation_id,
        });
        if let Some(actor) = satisfying_actor {
            data["satisfyingActor"] = serde_json::Value::String(actor.to_string());
        }
        record.data = Some(data);
        record
    }

    /// A pending obligation was violated. `effective_action` is the strictest
    /// applied action (Governance §16.2.4).
    ///
    /// The `witness` carries the PII-minimized evidence subset
    /// (WOS-OBL-PROV-1103): the trigger event, deadline, responsible
    /// actor/role, and the *referenced-paths-only* event and case-state
    /// witnesses. See [`ObligationViolationWitness`] for the minimization
    /// contract. Optional witness fields that are `None` are omitted from
    /// `data` entirely, so a bare violation (no witness populated) serializes
    /// to the same four required keys as before this enrichment.
    pub fn obligation_violated(
        policy_id: &str,
        obligation_id: &str,
        reason: &str,
        effective_action: &str,
        witness: ObligationViolationWitness<'_>,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::ObligationViolated);
        let mut data = serde_json::Map::from_iter([
            (
                "policyId".to_string(),
                serde_json::Value::String(policy_id.to_string()),
            ),
            (
                "obligationId".to_string(),
                serde_json::Value::String(obligation_id.to_string()),
            ),
            (
                "reason".to_string(),
                serde_json::Value::String(reason.to_string()),
            ),
            (
                "effectiveAction".to_string(),
                serde_json::Value::String(effective_action.to_string()),
            ),
        ]);
        if let Some(trigger_event) = witness.trigger_event {
            data.insert(
                "triggerEvent".to_string(),
                serde_json::Value::String(trigger_event.to_string()),
            );
        }
        if let Some(deadline) = witness.deadline {
            data.insert(
                "deadline".to_string(),
                serde_json::Value::String(deadline.to_string()),
            );
        }
        if let Some(responsible_actor) = witness.responsible_actor {
            data.insert(
                "responsibleActor".to_string(),
                serde_json::Value::String(responsible_actor.to_string()),
            );
        }
        if let Some(responsible_role) = witness.responsible_role {
            data.insert(
                "responsibleRole".to_string(),
                serde_json::Value::String(responsible_role.to_string()),
            );
        }
        if let Some(event_witness) = witness.event_witness {
            data.insert("eventWitness".to_string(), event_witness);
        }
        if let Some(case_state_witness) = witness.case_state_witness {
            data.insert("caseStateWitness".to_string(), case_state_witness);
        }
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// A pending obligation was cancelled (a `cancelWhen` match).
    pub fn obligation_cancelled(policy_id: &str, obligation_id: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::ObligationCancelled);
        record.data = Some(serde_json::json!({
            "policyId": policy_id,
            "obligationId": obligation_id,
        }));
        record
    }

    /// A pending obligation expired at its deadline.
    pub fn obligation_expired(
        policy_id: &str,
        obligation_id: &str,
        effective_action: &str,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::ObligationExpired);
        record.data = Some(serde_json::json!({
            "policyId": policy_id,
            "obligationId": obligation_id,
            "effectiveAction": effective_action,
        }));
        record
    }

    /// A pending obligation was bypassed by an authorized actor.
    pub fn obligation_bypassed(
        policy_id: &str,
        obligation_id: &str,
        bypass_actor: &str,
        rationale: &str,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::ObligationBypassed);
        record.data = Some(serde_json::json!({
            "policyId": policy_id,
            "obligationId": obligation_id,
            "bypassActor": bypass_actor,
            "rationale": rationale,
        }));
        record
    }

    /// A pre-breach warning fired for an obligation deadline.
    pub fn obligation_warning(
        policy_id: &str,
        obligation_id: &str,
        before_breach: &str,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::ObligationWarning);
        record.data = Some(serde_json::json!({
            "policyId": policy_id,
            "obligationId": obligation_id,
            "beforeBreach": before_breach,
        }));
        record
    }

    // ── ForEach iteration builders (Kernel §4.3.1; Sub-PR D-2) ──────────────

    /// One iteration of a `ForEach` state is starting.
    pub fn foreach_iteration_started(
        foreach_state: &str,
        index: u32,
        item: &serde_json::Value,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::ForEachIterationStarted);
        record.data = Some(serde_json::json!({
            "foreachState": foreach_state,
            "index": index,
            "item": item,
        }));
        record
    }

    /// One iteration of a `ForEach` state has completed. When iteration
    /// terminated early via `breakCondition`, `break_triggered` is `true`.
    pub fn foreach_iteration_completed(
        foreach_state: &str,
        index: u32,
        break_triggered: bool,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::ForEachIterationCompleted);
        let mut data = serde_json::json!({
            "foreachState": foreach_state,
            "index": index,
        });
        if break_triggered {
            data["breakTriggered"] = serde_json::Value::Bool(true);
        }
        record.data = Some(data);
        record
    }

    /// All iterations of a `ForEach` state have completed (or the empty-
    /// collection fast path fired). Emitted exactly once per foreach state
    /// entry, immediately before the foreach state's outgoing transition
    /// fires. `iterations` is the number of iterations actually executed
    /// (0 for empty-collection fast path); `broke` indicates whether the
    /// loop terminated early via `breakCondition`.
    pub fn foreach_completed(foreach_state: &str, iterations: u32, broke: bool) -> Self {
        let mut record = Self::blank(ProvenanceKind::ForEachCompleted);
        record.data = Some(serde_json::json!({
            "foreachState": foreach_state,
            "iterations": iterations,
            "broke": broke,
        }));
        record
    }

    /// Create a timer cancelled record (Lifecycle Detail S6.7).
    pub fn timer_cancelled(timer_id: &str, reason: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::TimerCancelled);
        record.data = Some(serde_json::json!({
            "timerId": timer_id,
            "reason": reason,
        }));
        record
    }

    /// Create a state-entry record.
    pub fn state_entered(state: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::OnEntry);
        record.to_state = Some(state.to_string());
        record.data = Some(serde_json::json!({ "state": state }));
        record
    }

    /// Create an onEntry action record.
    pub fn on_entry(state: &str, action_type: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::OnEntry);
        record.to_state = Some(state.to_string());
        record.data = Some(serde_json::json!({ "actionType": action_type }));
        record
    }

    /// Create an onExit action record.
    pub fn on_exit(state: &str, action_type: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::OnExit);
        record.from_state = Some(state.to_string());
        record.data = Some(serde_json::json!({ "actionType": action_type }));
        record
    }

    /// Create a generic action-executed record.
    pub fn action_executed(state: &str, action_type: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::ActionExecuted);
        record.to_state = Some(state.to_string());
        record.data = Some(serde_json::json!({ "actionType": action_type }));
        record
    }

    /// Create a timer tolerance violation record (LCD S6.6, Runtime S7.2).
    pub fn tolerance_violation(
        timer_id: &str,
        duration_iso: &str,
        max_tolerance_iso: &str,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::ToleranceViolation);
        record.data = Some(serde_json::json!({
            "timerId": timer_id,
            "duration": duration_iso,
            "maxTolerance": max_tolerance_iso,
        }));
        record
    }

    /// Create a history-cleared record.
    pub fn history_cleared(state: &str, reason: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::HistoryCleared);
        record.data = Some(serde_json::json!({
            "state": state,
            "reason": reason,
        }));
        record
    }

    /// Create an invalid-duration warning record.
    pub fn invalid_duration(raw_duration: &str, timer_id: &str) -> Self {
        let mut record = Self::blank(ProvenanceKind::InvalidDuration);
        record.data = Some(serde_json::json!({
            "rawDuration": raw_duration,
            "timerId": timer_id,
            "note": "unrecognized ISO 8601 duration; deadline set to zero (fires immediately)",
        }));
        record
    }

    /// Create a task lifecycle record emitted by the runtime layer.
    pub fn task_lifecycle(
        record_kind: ProvenanceKind,
        task_id: &str,
        actor_id: Option<&str>,
        data: Option<serde_json::Value>,
    ) -> Self {
        let mut record = Self::blank(record_kind);
        record.actor_id = actor_id.map(String::from);
        record.data = Some(match data {
            Some(extra) => {
                let mut object = serde_json::Map::new();
                object.insert(
                    "taskId".to_string(),
                    serde_json::Value::String(task_id.to_string()),
                );
                object.insert("details".to_string(), extra);
                serde_json::Value::Object(object)
            }
            None => serde_json::json!({ "taskId": task_id }),
        });
        record
    }

    /// Create a configuration-warning record for an unresolvable
    /// configuration reference or a configured operation failure
    /// (`drift-monitor.md:77`, `workflow-governance.md:154`,
    /// `notification-template.md:199,222`).
    ///
    /// `subject` is recorded verbatim; callers supply one of the four
    /// reserved literals or an `x-` vendor extension. `unresolvedRef` is
    /// merged into `data` only when the input carries it; render-failure
    /// records typically omit it and convey the failing template key /
    /// reason via `context`.
    #[must_use]
    pub fn configuration_warning(input: ConfigurationWarningInput<'_>) -> Self {
        let mut data = serde_json::Map::new();
        if let Some(context) = input.context {
            for (k, v) in context {
                if k == "subject" || k == "unresolvedRef" {
                    continue;
                }
                data.insert(k, v);
            }
        }
        data.insert(
            "subject".to_string(),
            serde_json::Value::String(input.subject.to_string()),
        );
        if let Some(unresolved_ref) = input.unresolved_ref {
            data.insert(
                "unresolvedRef".to_string(),
                serde_json::Value::String(unresolved_ref.to_string()),
            );
        }

        let mut record = Self::blank(ProvenanceKind::ConfigurationWarning);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create a capability-invocation record (AI Integration §3.3.1).
    ///
    /// When `invocation_blocked` is `true`, the record's `outcome` is set to
    /// the reserved kernel literal `"preconditionNotSatisfied"` (Kernel §8.2.2)
    /// so audit tooling can distinguish a declarative gate from an agent
    /// failure. When `false`, the outcome is left unset — the invocation
    /// proceeded normally and downstream records carry the agent outcome.
    #[must_use]
    pub fn capability_invocation(input: CapabilityInvocationInput<'_>) -> Self {
        let mut data = serde_json::Map::new();
        if let Some(context) = input.context {
            for (k, v) in context {
                if k == "capabilityId" || k == "invocationBlocked" {
                    continue;
                }
                data.insert(k, v);
            }
        }
        data.insert(
            "capabilityId".to_string(),
            serde_json::Value::String(input.capability_id.to_string()),
        );
        data.insert(
            "invocationBlocked".to_string(),
            serde_json::Value::Bool(input.invocation_blocked),
        );

        let mut record = Self::blank(ProvenanceKind::CapabilityInvocation);
        record.actor_id = Some(input.agent_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        if input.invocation_blocked {
            record.outcome = Some("preconditionNotSatisfied".to_string());
        }
        record
    }

    /// Create a contract validation record emitted by runtime task flows.
    pub fn contract_validation(
        task_id: &str,
        actor_id: Option<&str>,
        data: serde_json::Value,
    ) -> Self {
        let mut record = Self::blank(ProvenanceKind::ContractValidation);
        record.actor_id = actor_id.map(String::from);
        record.data = Some(serde_json::json!({
            "taskId": task_id,
            "details": data,
        }));
        record
    }

    /// Create a Signature Profile affirmation record.
    #[must_use]
    pub fn signature_affirmation(input: SignatureAffirmationInput<'_>) -> Self {
        let mut data = serde_json::Map::from_iter([
            (
                "signerId".to_string(),
                serde_json::Value::String(input.signer_id.to_string()),
            ),
            (
                "roleId".to_string(),
                serde_json::Value::String(input.role_id.to_string()),
            ),
            (
                "role".to_string(),
                serde_json::Value::String(input.role.to_string()),
            ),
            (
                "documentId".to_string(),
                serde_json::Value::String(input.document_id.to_string()),
            ),
            (
                "signingActId".to_string(),
                serde_json::Value::String(input.signing_act_id.to_string()),
            ),
            ("documentRef".to_string(), input.document_ref),
            (
                "documentHash".to_string(),
                serde_json::Value::String(input.document_hash.to_string()),
            ),
            (
                "presentationHash".to_string(),
                serde_json::Value::String(input.presentation_hash.to_string()),
            ),
            (
                "documentHashAlgorithm".to_string(),
                serde_json::Value::String(input.document_hash_algorithm.to_string()),
            ),
            (
                "sourceSignatureSystem".to_string(),
                serde_json::Value::String(input.source_signature_system.to_string()),
            ),
            (
                "sourceSignatureId".to_string(),
                serde_json::Value::String(input.source_signature_id.to_string()),
            ),
            (
                "signedPayloadDigest".to_string(),
                serde_json::Value::String(input.signed_payload_digest.to_string()),
            ),
            (
                "signedPayloadDigestAlgorithm".to_string(),
                serde_json::Value::String(input.signed_payload_digest_algorithm.to_string()),
            ),
            (
                "signingIntent".to_string(),
                serde_json::Value::String(input.signing_intent.to_string()),
            ),
            (
                "signedAt".to_string(),
                serde_json::Value::String(input.signed_at.to_string()),
            ),
            ("identityBinding".to_string(), input.identity_binding),
            ("consentReference".to_string(), input.consent_reference),
            (
                "signatureProvider".to_string(),
                serde_json::Value::String(input.signature_provider.to_string()),
            ),
            (
                "ceremonyId".to_string(),
                serde_json::Value::String(input.ceremony_id.to_string()),
            ),
            (
                "sourceResponseRef".to_string(),
                serde_json::Value::String(input.source_response_ref.to_string()),
            ),
            (
                "custodyHookEligible".to_string(),
                serde_json::Value::Bool(input.custody_hook_eligible),
            ),
            (
                "primitiveVerification".to_string(),
                input.primitive_verification,
            ),
            (
                "witnessedSignatureRef".to_string(),
                input
                    .witnessed_signature_ref
                    .map_or(serde_json::Value::Null, |value| {
                        serde_json::Value::String(value.to_string())
                    }),
            ),
        ]);

        if let Some(profile_ref) = input.profile_ref {
            data.insert(
                "profileRef".to_string(),
                serde_json::Value::String(profile_ref.to_string()),
            );
        }
        if let Some(profile_key) = input.profile_key {
            data.insert(
                "profileKey".to_string(),
                serde_json::Value::String(profile_key.to_string()),
            );
        }
        if let Some(signer_authority) = input.signer_authority {
            data.insert("signerAuthority".to_string(), signer_authority);
        }
        if let Some(verification_receipt) = input.verification_receipt {
            data.insert(
                "verificationReceipt".to_string(),
                serde_json::Value::String(verification_receipt.to_string()),
            );
        }

        let mut record = Self::blank(ProvenanceKind::SignatureAffirmation);
        record.actor_id = Some(input.signer_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create a Signature Profile admission-failed record.
    #[must_use]
    pub fn signature_admission_failed(input: SignatureAdmissionFailedInput<'_>) -> Self {
        let mut data = serde_json::Map::from_iter([
            (
                "reason".to_string(),
                serde_json::Value::String(input.reason.to_string()),
            ),
            (
                "evidenceBindings".to_string(),
                serde_json::json!({
                    "responseId": input.response_id,
                    "signedPayloadDigest": input.signed_payload_digest,
                    "signatureId": input.signature_id,
                    "signingIntent": input.signing_intent,
                }),
            ),
            (
                "emittedAt".to_string(),
                serde_json::Value::String(input.emitted_at.to_string()),
            ),
        ]);
        if let Some(signer_id) = input.signer_id {
            data.insert(
                "signerId".to_string(),
                serde_json::Value::String(signer_id.to_string()),
            );
        }
        if let Some(signer_authority) = input.signer_authority {
            data.insert("signerAuthority".to_string(), signer_authority);
        }
        if let Some(failure_context) = input.failure_context {
            data.insert(
                "failureContext".to_string(),
                serde_json::Value::Object(failure_context),
            );
        }

        let mut record = Self::blank(ProvenanceKind::SignatureAdmissionFailed);
        record.actor_id = input.signer_id.map(String::from);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    // ── Amendment & supersession (ADR 0066) ─────────────────────────

    /// Create a correction-authorized record (ADR 0066 §1, Mode 1).
    #[must_use]
    pub fn correction_authorized(input: CorrectionAuthorizedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "correctionTargetEventHash",
            "correctedFieldSet",
            "reason",
            "authorizingActorId",
            "authorityBasis",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "correctionTargetEventHash".to_string(),
            serde_json::Value::String(input.correction_target_event_hash.to_string()),
        );
        data.insert(
            "correctedFieldSet".to_string(),
            serde_json::Value::Array(
                input
                    .corrected_field_set
                    .into_iter()
                    .map(|p| serde_json::Value::String(p.to_string()))
                    .collect(),
            ),
        );
        data.insert(
            "reason".to_string(),
            serde_json::Value::String(input.reason.to_string()),
        );
        data.insert(
            "authorizingActorId".to_string(),
            serde_json::Value::String(input.authorizing_actor_id.to_string()),
        );
        data.insert("authorityBasis".to_string(), input.authority_basis);

        let mut record = Self::blank(ProvenanceKind::CorrectionAuthorized);
        record.actor_id = Some(input.authorizing_actor_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create an amendment-authorized record (ADR 0066 §2, Mode 2).
    #[must_use]
    pub fn amendment_authorized(input: AmendmentAuthorizedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "amendmentTargetEventHash",
            "priorDeterminationHash",
            "reason",
            "authorizingActorId",
            "authorityBasis",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "amendmentTargetEventHash".to_string(),
            serde_json::Value::String(input.amendment_target_event_hash.to_string()),
        );
        data.insert(
            "priorDeterminationHash".to_string(),
            serde_json::Value::String(input.prior_determination_hash.to_string()),
        );
        data.insert(
            "reason".to_string(),
            serde_json::Value::String(input.reason.to_string()),
        );
        data.insert(
            "authorizingActorId".to_string(),
            serde_json::Value::String(input.authorizing_actor_id.to_string()),
        );
        data.insert("authorityBasis".to_string(), input.authority_basis);

        let mut record = Self::blank(ProvenanceKind::AmendmentAuthorized);
        record.actor_id = Some(input.authorizing_actor_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create a determination-amended record (ADR 0066 §2).
    #[must_use]
    pub fn determination_amended(input: DeterminationAmendedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "priorDeterminationHash",
            "newDeterminationValue",
            "amendmentAuthorizationEventHash",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "priorDeterminationHash".to_string(),
            serde_json::Value::String(input.prior_determination_hash.to_string()),
        );
        data.insert(
            "newDeterminationValue".to_string(),
            input.new_determination_value,
        );
        data.insert(
            "amendmentAuthorizationEventHash".to_string(),
            serde_json::Value::String(input.amendment_authorization_event_hash.to_string()),
        );

        let mut record = Self::blank(ProvenanceKind::DeterminationAmended);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create a rescission-authorized record (ADR 0066 §3, Mode 4).
    #[must_use]
    pub fn rescission_authorized(input: RescissionAuthorizedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "rescissionTargetEventHash",
            "priorDeterminationHash",
            "reason",
            "authorizingActorId",
            "authorityBasis",
            "migrationPinChange",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "rescissionTargetEventHash".to_string(),
            serde_json::Value::String(input.rescission_target_event_hash.to_string()),
        );
        data.insert(
            "priorDeterminationHash".to_string(),
            serde_json::Value::String(input.prior_determination_hash.to_string()),
        );
        data.insert(
            "reason".to_string(),
            serde_json::Value::String(input.reason.to_string()),
        );
        data.insert(
            "authorizingActorId".to_string(),
            serde_json::Value::String(input.authorizing_actor_id.to_string()),
        );
        data.insert("authorityBasis".to_string(), input.authority_basis);
        if let Some(migration_pin_change) = input.migration_pin_change {
            data.insert(
                "migrationPinChange".to_string(),
                serde_json::Value::Object(migration_pin_change),
            );
        }

        let mut record = Self::blank(ProvenanceKind::RescissionAuthorized);
        record.actor_id = Some(input.authorizing_actor_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create a determination-rescinded record (ADR 0066 §3).
    #[must_use]
    pub fn determination_rescinded(input: DeterminationRescindedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &["priorDeterminationHash", "rescissionAuthorizationEventHash"];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "priorDeterminationHash".to_string(),
            serde_json::Value::String(input.prior_determination_hash.to_string()),
        );
        data.insert(
            "rescissionAuthorizationEventHash".to_string(),
            serde_json::Value::String(input.rescission_authorization_event_hash.to_string()),
        );

        let mut record = Self::blank(ProvenanceKind::DeterminationRescinded);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create a reinstated record (ADR 0066 §4, Mode 5 — Q26).
    #[must_use]
    pub fn reinstated(input: ReinstatedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "priorRescissionEventHash",
            "reactivationAuthorizationEventHash",
            "reason",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "priorRescissionEventHash".to_string(),
            serde_json::Value::String(input.prior_rescission_event_hash.to_string()),
        );
        data.insert(
            "reactivationAuthorizationEventHash".to_string(),
            serde_json::Value::String(input.reactivation_authorization_event_hash.to_string()),
        );
        data.insert(
            "reason".to_string(),
            serde_json::Value::String(input.reason.to_string()),
        );

        let mut record = Self::blank(ProvenanceKind::Reinstated);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create an authorization-attestation record (ADR 0066 §5).
    #[must_use]
    pub fn authorization_attestation(input: AuthorizationAttestationInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "authorizingActorId",
            "authorityBasis",
            "policyPredicate",
            "assuranceLevel",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "authorizingActorId".to_string(),
            serde_json::Value::String(input.authorizing_actor_id.to_string()),
        );
        data.insert("authorityBasis".to_string(), input.authority_basis);
        data.insert(
            "policyPredicate".to_string(),
            serde_json::Value::String(input.policy_predicate.to_string()),
        );
        if let Some(assurance_level) = input.assurance_level {
            data.insert(
                "assuranceLevel".to_string(),
                serde_json::Value::String(assurance_level.to_string()),
            );
        }

        let mut record = Self::blank(ProvenanceKind::AuthorizationAttestation);
        record.actor_id = Some(input.authorizing_actor_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    // ── Statutory clocks (ADR 0067) ──────────────────────────────

    /// Create a clock-started record (ADR 0067 §2).
    #[must_use]
    pub fn clock_started(input: ClockStartedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "clockId",
            "clockKind",
            "originEventHash",
            "duration",
            "computedDeadline",
            "calendarRef",
            "statuteReference",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "clockId".to_string(),
            serde_json::Value::String(input.clock_id.to_string()),
        );
        data.insert(
            "clockKind".to_string(),
            serde_json::Value::String(input.clock_kind.to_string()),
        );
        data.insert(
            "originEventHash".to_string(),
            serde_json::Value::String(input.origin_event_hash.to_string()),
        );
        data.insert(
            "duration".to_string(),
            serde_json::Value::String(input.duration.to_string()),
        );
        data.insert(
            "computedDeadline".to_string(),
            serde_json::Value::String(input.computed_deadline.to_string()),
        );
        if let Some(calendar_ref) = input.calendar_ref {
            data.insert(
                "calendarRef".to_string(),
                serde_json::Value::String(calendar_ref.to_string()),
            );
        }
        if let Some(statute_reference) = input.statute_reference {
            data.insert(
                "statuteReference".to_string(),
                serde_json::Value::String(statute_reference.to_string()),
            );
        }

        let mut record = Self::blank(ProvenanceKind::ClockStarted);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create a clock-resolved record (ADR 0067 §3).
    #[must_use]
    pub fn clock_resolved(input: ClockResolvedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "clockId",
            "originClockHash",
            "resolution",
            "resolvedAt",
            "resolvingEventHash",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "clockId".to_string(),
            serde_json::Value::String(input.clock_id.to_string()),
        );
        data.insert(
            "originClockHash".to_string(),
            serde_json::Value::String(input.origin_clock_hash.to_string()),
        );
        let (resolution_str, resolving_event_hash): (&str, Option<&str>) = match &input.resolution {
            ClockResolvedResolution::Satisfied {
                resolving_event_hash,
            } => ("satisfied", *resolving_event_hash),
            ClockResolvedResolution::Elapsed {
                resolving_event_hash,
            } => ("elapsed", *resolving_event_hash),
            ClockResolvedResolution::Cancelled {
                resolving_event_hash,
            } => ("cancelled", *resolving_event_hash),
            ClockResolvedResolution::Paused {
                resolving_event_hash,
            } => ("paused", Some(*resolving_event_hash)),
        };
        data.insert(
            "resolution".to_string(),
            serde_json::Value::String(resolution_str.to_string()),
        );
        data.insert(
            "resolvedAt".to_string(),
            serde_json::Value::String(input.resolved_at.to_string()),
        );
        if let Some(resolving_event_hash) = resolving_event_hash {
            data.insert(
                "resolvingEventHash".to_string(),
                serde_json::Value::String(resolving_event_hash.to_string()),
            );
        }

        let mut record = Self::blank(ProvenanceKind::ClockResolved);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    // ── Identity attestation (ADR 0068) ──────────────────────────

    /// Create an identity-attestation record (ADR 0068 §4, Q15).
    #[must_use]
    pub fn identity_attestation(input: IdentityAttestationInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "subjectGlobalId",
            "assuranceLevel",
            "attestationProvider",
            "providerAttestationId",
            "attestedAt",
            "validUntil",
            "attestedPredicates",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "subjectGlobalId".to_string(),
            serde_json::Value::String(input.subject_global_id.to_string()),
        );
        data.insert(
            "assuranceLevel".to_string(),
            serde_json::Value::String(input.assurance_level.to_string()),
        );
        data.insert(
            "attestationProvider".to_string(),
            serde_json::Value::String(input.attestation_provider.to_string()),
        );
        data.insert(
            "providerAttestationId".to_string(),
            serde_json::Value::String(input.provider_attestation_id.to_string()),
        );
        data.insert(
            "attestedAt".to_string(),
            serde_json::Value::String(input.attested_at.to_string()),
        );
        if let Some(valid_until) = input.valid_until {
            data.insert(
                "validUntil".to_string(),
                serde_json::Value::String(valid_until.to_string()),
            );
        }
        data.insert(
            "attestedPredicates".to_string(),
            serde_json::Value::Array(
                input
                    .attested_predicates
                    .into_iter()
                    .map(|p| serde_json::Value::String(p.to_string()))
                    .collect(),
            ),
        );

        let mut record = Self::blank(ProvenanceKind::IdentityAttestation);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    // ── Key rebind recovery (ADR 0091) ─────────────────────────────

    /// Create a key-rebind recovery record (ADR 0091).
    ///
    /// # Errors
    /// Returns an error when the key ids are malformed, when either assurance
    /// token has no portable ordering, or when the recovery would lower
    /// assurance.
    pub fn key_rebind(input: KeyRebindInput<'_>) -> Result<Self, KeyRebindError> {
        const REQUIRED: &[&str] = &[
            "priorKid",
            "newKid",
            "priorAssurance",
            "newAssurance",
            "rebindAttestationRef",
            "reason",
        ];
        validate_key_rebind_input(&input)?;
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "priorKid".to_string(),
            serde_json::Value::String(input.prior_kid.to_string()),
        );
        data.insert(
            "newKid".to_string(),
            serde_json::Value::String(input.new_kid.to_string()),
        );
        data.insert(
            "priorAssurance".to_string(),
            serde_json::Value::String(input.prior_assurance.to_string()),
        );
        data.insert(
            "newAssurance".to_string(),
            serde_json::Value::String(input.new_assurance.to_string()),
        );
        data.insert(
            "rebindAttestationRef".to_string(),
            serde_json::Value::String(input.rebind_attestation_ref.to_string()),
        );
        if let Some(reason) = input.reason {
            data.insert(
                "reason".to_string(),
                serde_json::Value::String(reason.to_string()),
            );
        }

        let mut record = Self::blank(ProvenanceKind::KeyRebind);
        record.data = Some(serde_json::Value::Object(data));
        Ok(record)
    }

    // ── Clock skew (ADR 0069) ────────────────────────────────────

    /// Create a clock-skew-observed record (ADR 0069 §3).
    #[must_use]
    pub fn clock_skew_observed(input: ClockSkewObservedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "processorAuthoredAt",
            "substrateCreatedAt",
            "skewMilliseconds",
            "thresholdMilliseconds",
            "eventHash",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "processorAuthoredAt".to_string(),
            serde_json::Value::String(input.processor_authored_at.to_string()),
        );
        data.insert(
            "substrateCreatedAt".to_string(),
            serde_json::Value::String(input.substrate_created_at.to_string()),
        );
        data.insert(
            "skewMilliseconds".to_string(),
            serde_json::Value::Number(serde_json::Number::from(input.skew_milliseconds)),
        );
        data.insert(
            "thresholdMilliseconds".to_string(),
            serde_json::Value::Number(serde_json::Number::from(input.threshold_milliseconds)),
        );
        data.insert(
            "eventHash".to_string(),
            serde_json::Value::String(input.event_hash.to_string()),
        );

        let mut record = Self::blank(ProvenanceKind::ClockSkewObserved);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    // ── Failure & compensation (ADR 0070) ────────────────────────

    /// Create a commit-attempt-failure record (ADR 0070 §2).
    #[must_use]
    pub fn commit_attempt_failure(input: CommitAttemptFailureInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "targetEventHash",
            "failureKind",
            "attemptCount",
            "retryBudgetRemainingMs",
            "errorPayload",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "targetEventHash".to_string(),
            serde_json::Value::String(input.target_event_hash.to_string()),
        );
        data.insert(
            "failureKind".to_string(),
            serde_json::Value::String(input.failure_kind.as_camel_str().to_string()),
        );
        data.insert(
            "attemptCount".to_string(),
            serde_json::Value::Number(serde_json::Number::from(input.attempt_count)),
        );
        data.insert(
            "retryBudgetRemainingMs".to_string(),
            serde_json::Value::Number(serde_json::Number::from(input.retry_budget_remaining_ms)),
        );
        if let Some(error_payload) = input.error_payload {
            data.insert("errorPayload".to_string(), error_payload);
        }

        let mut record = Self::blank(ProvenanceKind::CommitAttemptFailure);
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create an authorization-rejected record (ADR 0070 §4).
    #[must_use]
    pub fn authorization_rejected(input: AuthorizationRejectedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "attemptedActorId",
            "attemptedAction",
            "targetResourceId",
            "rejectionReason",
            "policyDecisionRef",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "attemptedActorId".to_string(),
            serde_json::Value::String(input.attempted_actor_id.to_string()),
        );
        data.insert(
            "attemptedAction".to_string(),
            serde_json::Value::String(input.attempted_action.to_string()),
        );
        data.insert(
            "targetResourceId".to_string(),
            serde_json::Value::String(input.target_resource_id.to_string()),
        );
        data.insert(
            "rejectionReason".to_string(),
            serde_json::Value::String(input.rejection_reason.to_string()),
        );
        if let Some(policy_decision_ref) = input.policy_decision_ref {
            data.insert(
                "policyDecisionRef".to_string(),
                serde_json::Value::String(policy_decision_ref.to_string()),
            );
        }

        let mut record = Self::blank(ProvenanceKind::AuthorizationRejected);
        record.actor_id = Some(input.attempted_actor_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    // ── Migration & version pins (ADR 0071) ──────────────────────

    /// Create an instance-migrated record (Kernel S11.2, ADR 0083).
    ///
    /// `data` carries `fromVersion`, `toVersion`, and `migrationMap` per
    /// Kernel S11.2 steps 2–3.
    #[must_use]
    pub fn instance_migrated(input: InstanceMigratedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &["fromVersion", "toVersion", "migrationMap"];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert(
            "fromVersion".to_string(),
            serde_json::Value::String(input.from_version.to_string()),
        );
        data.insert(
            "toVersion".to_string(),
            serde_json::Value::String(input.to_version.to_string()),
        );
        data.insert("migrationMap".to_string(), input.migration_map);

        let mut record = Self::blank(ProvenanceKind::InstanceMigrated);
        if let Some(actor) = input.actor_id {
            record.actor_id = Some(actor.to_string());
        }
        record.data = Some(serde_json::Value::Object(data));
        record
    }

    /// Create a migration-pin-changed record (ADR 0071 §3).
    #[must_use]
    pub fn migration_pin_changed(input: MigrationPinChangedInput<'_>) -> Self {
        const REQUIRED: &[&str] = &[
            "priorPinSet",
            "newPinSet",
            "authorizingActorId",
            "authorityBasis",
            "migrationRationale",
        ];
        let mut data = merge_context(input.context, REQUIRED);
        data.insert("priorPinSet".to_string(), input.prior_pin_set);
        data.insert("newPinSet".to_string(), input.new_pin_set);
        data.insert(
            "authorizingActorId".to_string(),
            serde_json::Value::String(input.authorizing_actor_id.to_string()),
        );
        data.insert("authorityBasis".to_string(), input.authority_basis);
        data.insert(
            "migrationRationale".to_string(),
            serde_json::Value::String(input.migration_rationale.to_string()),
        );

        let mut record = Self::blank(ProvenanceKind::MigrationPinChanged);
        record.actor_id = Some(input.authorizing_actor_id.to_string());
        record.data = Some(serde_json::Value::Object(data));
        record
    }
}

/// Merge an optional caller-supplied `context` map into a fresh data map,
/// dropping any keys that collide with the constructor's required fields.
/// Constructor args remain the source of truth — `context` (which may
/// originate from untrusted scratch) MUST NOT be able to overwrite the
/// schema-shaping discriminators.
fn merge_context(
    context: Option<serde_json::Map<String, serde_json::Value>>,
    required: &[&str],
) -> serde_json::Map<String, serde_json::Value> {
    let mut data = serde_json::Map::new();
    if let Some(context) = context {
        for (k, v) in context {
            if required.iter().any(|r| *r == k) {
                continue;
            }
            data.insert(k, v);
        }
    }
    data
}

fn validate_key_rebind_input(input: &KeyRebindInput<'_>) -> Result<(), KeyRebindError> {
    validate_key_rebind_kid("priorKid", input.prior_kid)?;
    validate_key_rebind_kid("newKid", input.new_kid)?;
    let prior_rank =
        assurance_rank(input.prior_assurance).ok_or_else(|| KeyRebindError::UnrankedAssurance {
            field: "priorAssurance",
            value: input.prior_assurance.to_string(),
        })?;
    let new_rank =
        assurance_rank(input.new_assurance).ok_or_else(|| KeyRebindError::UnrankedAssurance {
            field: "newAssurance",
            value: input.new_assurance.to_string(),
        })?;
    if new_rank < prior_rank {
        return Err(KeyRebindError::AssuranceDowngrade {
            prior: input.prior_assurance.to_string(),
            new: input.new_assurance.to_string(),
        });
    }
    Ok(())
}

fn validate_key_rebind_kid(field: &'static str, value: &str) -> Result<(), KeyRebindError> {
    if value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(KeyRebindError::InvalidKid {
            field,
            value: value.to_string(),
        })
    }
}

fn assurance_rank(value: &str) -> Option<u8> {
    match value {
        "low" => Some(1),
        "standard" => Some(2),
        "high" => Some(3),
        "very-high" => Some(4),
        _ => None,
    }
}

impl std::fmt::Display for ProvenanceRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{:?}", self.id, self.record_kind)?;
        if !self.timestamp.is_empty() {
            write!(f, " at={}", self.timestamp)?;
        }
        if let Some(from) = &self.from_state {
            write!(f, " from={from}")?;
        }
        if let Some(to) = &self.to_state {
            write!(f, " to={to}")?;
        }
        if let Some(event) = &self.event {
            write!(f, " event={event}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod constructor_literal_drift_tests {
    use super::*;
    use crate::provenance::kind::WOS_CANONICAL_EVENT_LITERALS;
    use serde_json::json;

    fn assert_event_matches_kind(record: &ProvenanceRecord) {
        assert_eq!(
            record.event.as_deref(),
            record.record_kind.canonical_event_literal(),
            "constructor for {:?} must not drift from canonical_event_literal",
            record.record_kind
        );
    }

    /// Every substrate-canonical literal must map to a [`ProvenanceKind`] whose
    /// [`ProvenanceRecord::blank`] stamping matches Trellis admission tables
    /// (`WOS_CANONICAL_EVENT_LITERALS` / `canonical_event_literal`).
    #[test]
    fn given_each_substrate_canonical_literal_when_blank_record_minted_then_event_matches_table() {
        for literal in WOS_CANONICAL_EVENT_LITERALS {
            let kind = ProvenanceKind::from_canonical_event_literal(literal)
                .unwrap_or_else(|| panic!("substrate literal `{literal}` must parse to a kind"));
            let record = ProvenanceRecord::blank(kind);
            assert_eq!(
                record.event.as_deref(),
                Some(*literal),
                "blank({kind:?}) must stamp substrate literal `{literal}`"
            );
            assert_eq!(
                kind.canonical_event_literal(),
                Some(*literal),
                "canonical_event_literal must agree with substrate table for `{literal}`"
            );
        }
    }

    #[test]
    fn given_state_transition_constructor_when_built_then_event_matches_canonical_literal() {
        let record = ProvenanceRecord::state_transition("a", "b", "trigger", Some("actor"));
        assert_event_matches_kind(&record);
    }

    #[test]
    fn given_foreach_constructors_when_built_then_events_match_canonical_literals() {
        assert_event_matches_kind(&ProvenanceRecord::foreach_iteration_started(
            "fe",
            0,
            &json!("x"),
        ));
        assert_event_matches_kind(&ProvenanceRecord::foreach_iteration_completed(
            "fe", 0, false,
        ));
        assert_event_matches_kind(&ProvenanceRecord::foreach_completed("fe", 2, false));
    }

    #[test]
    fn given_signature_constructors_when_built_then_events_match_canonical_literals() {
        let digest64 = format!("sha256:{}", "a".repeat(64));
        assert_event_matches_kind(&ProvenanceRecord::signature_admission_failed(
            SignatureAdmissionFailedInput {
                reason: "test",
                response_id: "r1",
                signed_payload_digest: digest64.as_str(),
                signature_id: "s1",
                signing_intent: "intent:test",
                signer_id: None,
                signer_authority: None,
                failure_context: None,
                emitted_at: "2020-01-01T00:00:00Z",
            },
        ));
        let doc_hash = format!("sha256:{}", "b".repeat(64));
        let pres_hash = format!("sha256:{}", "c".repeat(64));
        let signed_digest = format!("sha256:{}", "d".repeat(64));
        assert_event_matches_kind(&ProvenanceRecord::signature_affirmation(
            SignatureAffirmationInput {
                signer_id: "signer",
                role_id: "role-id",
                role: "signer",
                document_id: "doc",
                signing_act_id: "act",
                document_ref: json!({"documentId": "doc", "locale": "en"}),
                document_hash: doc_hash.as_str(),
                presentation_hash: pres_hash.as_str(),
                document_hash_algorithm: "sha256",
                source_signature_system: "sys",
                source_signature_id: "sid",
                signed_payload_digest: signed_digest.as_str(),
                signed_payload_digest_algorithm: "sha256",
                signing_intent: "intent:test",
                signed_at: "2020-01-01T00:00:00Z",
                identity_binding: json!({}),
                consent_reference: json!({}),
                signature_provider: "prov",
                ceremony_id: "cer",
                source_response_ref: "resp",
                profile_ref: None,
                profile_key: None,
                signer_authority: None,
                custody_hook_eligible: false,
                primitive_verification: json!({"status": "verified"}),
                verification_receipt: None,
                witnessed_signature_ref: None,
            },
        ));
    }

    #[test]
    fn given_governance_clock_constructors_when_built_then_events_match_canonical_literals() {
        assert_event_matches_kind(&ProvenanceRecord::determination_rescinded(
            DeterminationRescindedInput {
                prior_determination_hash: "h1",
                rescission_authorization_event_hash: "h2",
                context: None,
            },
        ));
        assert_event_matches_kind(&ProvenanceRecord::reinstated(ReinstatedInput {
            prior_rescission_event_hash: "h1",
            reactivation_authorization_event_hash: "h2",
            reason: "r",
            context: None,
        }));
        assert_event_matches_kind(&ProvenanceRecord::clock_started(ClockStartedInput {
            clock_id: "c1",
            clock_kind: "AppealClock",
            origin_event_hash: "oh",
            duration: "P1D",
            computed_deadline: "2020-01-02T00:00:00Z",
            calendar_ref: None,
            statute_reference: None,
            context: None,
        }));
        assert_event_matches_kind(&ProvenanceRecord::clock_resolved(ClockResolvedInput {
            clock_id: "c1",
            origin_clock_hash: "oh",
            resolution: ClockResolvedResolution::Satisfied {
                resolving_event_hash: None,
            },
            resolved_at: "2020-01-02T00:00:00Z",
            context: None,
        }));
    }

    #[test]
    fn given_assurance_constructors_when_built_then_events_match_canonical_literals() {
        assert_event_matches_kind(&ProvenanceRecord::identity_attestation(
            IdentityAttestationInput {
                subject_global_id: "subj",
                assurance_level: "high",
                attestation_provider: "prov",
                provider_attestation_id: "pid",
                attested_at: "2020-01-01T00:00:00Z",
                valid_until: None,
                attested_predicates: vec!["pred"],
                context: None,
            },
        ));
        assert_event_matches_kind(
            &ProvenanceRecord::key_rebind(KeyRebindInput {
                prior_kid: "0123456789abcdef0123456789abcdef",
                new_kid: "fedcba9876543210fedcba9876543210",
                prior_assurance: "high",
                new_assurance: "high",
                rebind_attestation_ref: "https://example.invalid/ref",
                reason: None,
                context: None,
            })
            .expect("valid key-rebind fixture"),
        );
    }

    // ── Durable obligations (ADR 0096) ───────────────────────────

    #[test]
    fn given_obligation_constructors_when_built_then_record_kind_and_required_data_present() {
        let activated = ProvenanceRecord::obligation_activated(
            "policy-1",
            "obl-1",
            Some("event:submitted"),
            Some("2026-06-10T00:00:00Z"),
        );
        assert_eq!(activated.record_kind, ProvenanceKind::ObligationActivated);
        let data = activated.data.as_ref().expect("activated carries data");
        assert_eq!(data["policyId"], "policy-1");
        assert_eq!(data["obligationId"], "obl-1");
        assert_eq!(data["triggerEvent"], "event:submitted");
        assert_eq!(data["deadline"], "2026-06-10T00:00:00Z");

        let satisfied = ProvenanceRecord::obligation_satisfied("policy-1", "obl-1", Some("worker"));
        assert_eq!(satisfied.record_kind, ProvenanceKind::ObligationSatisfied);
        assert_eq!(
            satisfied.data.as_ref().expect("data")["satisfyingActor"],
            "worker"
        );

        let cancelled = ProvenanceRecord::obligation_cancelled("policy-1", "obl-1");
        assert_eq!(cancelled.record_kind, ProvenanceKind::ObligationCancelled);

        let expired = ProvenanceRecord::obligation_expired("policy-1", "obl-1", "suspend");
        assert_eq!(expired.record_kind, ProvenanceKind::ObligationExpired);
        assert_eq!(
            expired.data.as_ref().expect("data")["effectiveAction"],
            "suspend"
        );

        let bypassed =
            ProvenanceRecord::obligation_bypassed("policy-1", "obl-1", "supervisor", "override");
        assert_eq!(bypassed.record_kind, ProvenanceKind::ObligationBypassed);
        assert_eq!(
            bypassed.data.as_ref().expect("data")["bypassActor"],
            "supervisor"
        );

        let warning = ProvenanceRecord::obligation_warning("policy-1", "obl-1", "P1D");
        assert_eq!(warning.record_kind, ProvenanceKind::ObligationWarning);
        assert_eq!(warning.data.as_ref().expect("data")["beforeBreach"], "P1D");
    }

    /// A bare violation (default/empty witness) serializes to exactly the four
    /// required keys — proving the witness enrichment is additive and does not
    /// regress the pre-enrichment payload shape (WOS-OBL-PROV-1103).
    #[test]
    fn given_obligation_violated_with_empty_witness_then_only_required_keys_present() {
        let record = ProvenanceRecord::obligation_violated(
            "policy-1",
            "obl-1",
            "deadline elapsed",
            "suspend",
            ObligationViolationWitness::default(),
        );
        assert_eq!(record.record_kind, ProvenanceKind::ObligationViolated);
        let data = record
            .data
            .as_ref()
            .and_then(|value| value.as_object())
            .expect("violation carries an object payload");
        let mut keys: Vec<&str> = data.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["effectiveAction", "obligationId", "policyId", "reason"],
            "empty witness must add no optional keys: {keys:?}"
        );
    }

    /// A fully-populated witness carries the trigger event, deadline,
    /// responsible actor/role, and the PII-minimized event / case-state witness
    /// subsets verbatim (WOS-OBL-PROV-1103).
    #[test]
    fn given_obligation_violated_with_full_witness_then_minimized_subsets_carried() {
        let record = ProvenanceRecord::obligation_violated(
            "policy-1",
            "obl-1",
            "required submission missing",
            "block",
            ObligationViolationWitness {
                trigger_event: Some("sha256:deadbeef"),
                deadline: Some("2026-06-10T00:00:00Z"),
                responsible_actor: Some("caseworker-7"),
                responsible_role: Some("adjudicator"),
                // Minimized subset: ONLY the referenced path, not the full event.
                event_witness: Some(json!({ "form.section": "B" })),
                case_state_witness: Some(json!({ "status": "pending" })),
            },
        );
        let data = record
            .data
            .as_ref()
            .and_then(|value| value.as_object())
            .expect("violation carries an object payload");
        assert_eq!(data["triggerEvent"], "sha256:deadbeef");
        assert_eq!(data["deadline"], "2026-06-10T00:00:00Z");
        assert_eq!(data["responsibleActor"], "caseworker-7");
        assert_eq!(data["responsibleRole"], "adjudicator");
        assert_eq!(data["eventWitness"], json!({ "form.section": "B" }));
        assert_eq!(data["caseStateWitness"], json!({ "status": "pending" }));
    }
}
