// Rust guideline compliant 2026-02-21

//! Rule metadata registry for all currently-implemented lint rules.
//!
//! The registry is the single source of truth for "what rules exist in code"
//! — separate from [`LINT-MATRIX.md`], which is the source of truth for
//! "what rules the spec normatively requires." Gaps between the two are
//! rule-coverage gaps.
//!
//! Every rule currently starts at [`Graduation::Draft`] with an empty
//! [`RuleMetadata::fixtures`] list. Promotion through the ladder
//! (Draft → Tested → Stable → LoadBearing) is handled by follow-up work
//! described in `thoughts/plans/2026-04-16-wos-rule-coverage-conformance.md`.
//!
//! # Adding a new rule
//!
//! When introducing a new lint diagnostic, add a [`RuleMetadata`] entry to
//! [`ALL_LINT_RULES`] with `graduation: Graduation::Draft` and an empty
//! `fixtures` slice. Promotion is performed in a separate change once the
//! rule has at least one passing fixture.

use crate::diagnostic::LintSeverity;

/// Verification tier a rule belongs to.
///
/// See [`LINT-MATRIX.md`] for the authoritative tier catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Single-document structural checks (`wos-lint`).
    T1,
    /// Cross-document resolution and FEL AST analysis (`wos-lint --project`).
    T2,
    /// Dynamic runtime conformance (`wos-conformance`).
    T3,
}

/// Maturity level of a rule's fixture coverage.
///
/// The ladder goes `Draft → Tested → Stable → LoadBearing`. Rules promote
/// monotonically. See the rule-coverage plan for the promotion criteria.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Graduation {
    /// No passing fixture exercises this rule yet.
    Draft,
    /// At least one passing fixture exercises this rule.
    Tested,
    /// `Tested`, plus unchanged across 3+ consecutive releases.
    Stable,
    /// Removing the rule would break at least one reference-implementation
    /// test. Requires spec_ref + suggested_fix + fixtures + ratchet check.
    LoadBearing,
}

/// Static metadata describing one lint rule.
///
/// Metadata is compile-time — all fields are `'static`. `spec_ref` and
/// `suggested_fix` are `Option` because only `LoadBearing` rules are
/// required to supply them.
#[derive(Debug, Clone, Copy)]
pub struct RuleMetadata {
    /// Rule identifier (e.g. `"K-001"`).
    pub id: &'static str,
    /// Verification tier.
    pub tier: Tier,
    /// Severity of the primary diagnostic this rule emits.
    pub severity: LintSeverity,
    /// One-line summary of what the rule checks.
    pub summary: &'static str,
    /// Fixture paths (relative to `fixtures/`) that exercise this rule.
    pub fixtures: &'static [&'static str],
    /// Graduation ladder state.
    pub graduation: Graduation,
    /// Canonical spec reference (e.g. `"kernel/spec.md#§5.3"`), if recorded.
    pub spec_ref: Option<&'static str>,
    /// Imperative remediation hint, if recorded.
    pub suggested_fix: Option<&'static str>,
}

/// Return the full static registry of currently-implemented lint rules.
///
/// Ordering is by rule id to keep diffs readable; downstream code should
/// not rely on the ordering.
pub fn all_lint_rules() -> &'static [RuleMetadata] {
    ALL_LINT_RULES
}

/// Every lint rule currently implemented in `wos-lint`.
///
/// Entries are derived from the concrete `LintDiagnostic::{t1_error,t2_error,...}`
/// call sites in `tier1.rs`, `tier2.rs`, `fel_analysis.rs`, and
/// `schema_doc.rs`. Rules listed in `LINT-MATRIX.md` but not yet emitted by
/// code are intentionally absent — the registry describes present reality,
/// not the normative catalog.
static ALL_LINT_RULES: &[RuleMetadata] = &[
    // --- ACT (Activation Criteria / Durable Obligations, ADR 0096) ----
    RuleMetadata {
        id: "ACT-001",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Obligation-policy activation criteria `where` MUST be valid FEL.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("governance/workflow-governance.md#16.1.2"),
        suggested_fix: Some(
            "Fix the FEL syntax in the activation criteria `where` expression.",
        ),
    },
    RuleMetadata {
        id: "ACT-002",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Obligation-policy activation criteria `where` AST root MUST be boolean-shaped (no truthy coercion).",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("governance/workflow-governance.md#16.1.2"),
        suggested_fix: Some(
            "Make `where` a boolean expression (comparison/logical), not a bare field or literal.",
        ),
    },
    // --- AG (Advanced Governance) -------------------------------------
    RuleMetadata {
        id: "AG-008",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Side-effect tools at `autonomous` autonomy MUST declare a `sideEffectPolicy`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AG-010",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Verifiable constraints MUST satisfy all SMT subset restrictions (parse failures).",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AG-011",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "`let` bindings in verifiable expressions MUST NOT be recursive.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AG-012",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Quantifiers MUST quantify over finite domains (non-standard every/some arity).",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AG-013",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Verifiable arithmetic MUST be linear (no variable*variable products).",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AG-014",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Verifiable subset MUST NOT include extension function calls.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AG-017",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Shadow mode is RECOMMENDED for rights-impacting workflows.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // --- AI (AI Integration) -------------------------------------------
    RuleMetadata {
        id: "AI-007",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Cascading autonomous agents MUST be declared via `cascadingInvocations`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-018",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "`autonomous` actions MUST have associated deontic constraints.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-020",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "`supervisory` actions MUST define `reviewWindow`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-023",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Every agent invocation MUST have a reachable path to completion without any agent.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-024",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Escalation conditions MUST be valid FEL referencing `@agent` context.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-026",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Escalation MUST have `escalationExpiry`; agent reverts when expired.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-031",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Agent output contract MUST apply same rules as human-facing form.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // AI-041: Tested via inline JSON in the `ai041_*` unit tests in
    // crates/wos-lint/tests/tier1_rules.rs (e.g.
    // `ai041_fallback_chain_without_terminal_action_flagged`).
    // The linked T3 fixture exists but is intentionally excluded from the
    // conformance trace-parity harness (see the doc comment in
    // crates/wos-conformance/tests/trace_parity.rs — the fixture has no
    // `kernel` document and is lint-only), so the evidence is indirect.
    // (See 2026-04-18 review.)
    RuleMetadata {
        id: "AI-041",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Fallback chain MUST terminate in `escalateToHuman` or `fail`; MUST NOT cycle.",
        fixtures: &["crates/wos-conformance/fixtures/AI-041-negative-fallback-cycle.json"],
        graduation: Graduation::Tested,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-042",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Agent config MUST disclose training data characteristics.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-043",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Agent config MUST disclose optimization objective.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-046",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "`rights-impacting` workflows MUST have `discloseThatAgentAssisted: true`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-049",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Narrative records MUST have `authoritative: false`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-056",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Autonomy is an action-site property, not an agent property.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "AI-057",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Capability `preconditions` entries MUST be valid FEL.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("specs/ai/ai-integration.md#331-capability-preconditions"),
        suggested_fix: None,
    },
    // AI-058: Unit tests live in `crates/wos-lint/src/rules/fel_analysis.rs`
    // (`ai058_binary_comparison_is_boolean_shaped`,
    // `ai058_bare_field_ref_fires`, `ai058_string_literal_fires`,
    // `ai058_boolean_returning_builtin_is_clean`, plus the §4.3b #F4a
    // allowlist tests `ai058_every_builtin_is_clean`,
    // `ai058_some_builtin_is_clean`, `ai058_boolean_cast_is_clean`,
    // `ai058_is_boolean_is_not_a_builtin`). Shares the
    // `check_capability_preconditions` entry point with AI-057.
    RuleMetadata {
        id: "AI-058",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Capability `preconditions` AST root MUST be boolean-shaped (no truthy coercion).",
        fixtures: &[],
        graduation: Graduation::Tested,
        spec_ref: Some("specs/ai/ai-integration.md#331-capability-preconditions"),
        suggested_fix: None,
    },
    // --- CM (Correspondence Metadata) ---------------------------------
    RuleMetadata {
        id: "CM-001",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Entry template `id` values MUST be unique within the sidecar.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // --- DM (Drift Monitor) -------------------------------------------
    RuleMetadata {
        id: "DM-002",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Rights/safety workflows SHOULD follow shadow/canary/production sequence.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // --- G (Governance) -----------------------------------------------
    RuleMetadata {
        id: "G-001",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Due process MUST be enforced for `rights-impacting` or `safety-impacting` kernels.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-003",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Notice MUST include specific determination, reason codes, and appeal instructions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-004",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Explanation level MUST be `individualized` when kernel impact is `rights-impacting`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-005",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Adverse decisions MUST include positive and negative counterfactuals when rights-impacting.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-008",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "`continuationOfServices: true` requires kernel topology to freeze adverse impacts.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-009",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Transitions tagged `adverse-decision` MUST trigger due process policy enforcement.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-011",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Review protocol tags MUST match tags declared in the target kernel.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-014",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Reasoning tier MUST be present for `determination`-tagged transitions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-015",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Counterfactual tier MUST be present for `adverse-decision` transitions in rights-impacting workflows.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-022",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "`excludedOwner` MUST override `potentialOwner` when actor appears in both.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-023",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "SLA evaluation SHOULD use business calendar when BC sidecar is present.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-024",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Determination-tagged transitions MUST verify the actor has valid delegation.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-027",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Sub-delegation MUST respect `maxDelegationDepth`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-028",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Hold policies MUST attach to kernel states tagged `hold`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-029",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "`resumeTrigger` event name MUST reference an event in the target kernel.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-031",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "`resolutionDateRef` MUST refer to a field path in the kernel's case state.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-033",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Parameter `values` SHOULD cover every resolution date (no coverage gap).",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-034",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "`targetWorkflow` MUST match the `url` of the target kernel document.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-035",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "`targetGovernance` MUST reference a valid governance document.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-036",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "`independenceConstraint` MUST describe a mechanism preventing self-review.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-037",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Assertion `id` values MUST be unique within the library.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-038",
        tier: Tier::T1,
        severity: LintSeverity::Warning,
        summary: "Assertions of type `arithmetic`/`range`/`temporal` SHOULD include `expression`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-039",
        tier: Tier::T1,
        severity: LintSeverity::Warning,
        summary: "Assertions of type `source-grounded`/`consistency` SHOULD include `fields`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-040",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "`consistency` assertions `referenceStage` MUST refer to an earlier pipeline stage.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-041",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Pipeline-stage assertion ids MUST exist in the targeted assertion library.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-042",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "FEL expressions in assertion `expression` fields MUST be syntactically valid.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-043",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "FEL expressions in delegation `conditions` MUST be syntactically valid.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-044",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Delegation `expirationDate` MUST be strictly after `effectiveDate`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-045",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Delegation `revokedDate` MUST be on or after `effectiveDate`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-046",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Delegation `delegator`/`delegate` MUST reference declared kernel actors.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-047",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Parameter `values` entries MUST be in ascending `effectiveDate` order.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-048",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Binding `id` MUST match the key under which it appears in the `bindings` map.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-050",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Resolved parameter value MUST be type-consistent with declared `type`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-053",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Sub-delegation is only permitted if the original delegation explicitly allows it.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-055",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "`expectedDuration` MUST be an ISO 8601 duration or the literal `\"indefinite\"`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-056",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Binding `resolutionDateRef` MUST reference a field path in kernel case state.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-057",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Binding `values` entries MUST be in ascending `effectiveDate` order.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-058",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Each Holiday entry MUST specify exactly one of `date` or `rule`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-059",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Operating hours `end` MUST be strictly after `start`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-060",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Business Calendar target requires SLA evaluation in business days.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-062",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Adverse-decision templates MUST cover determination, reasons, rights, and instructions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-063",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Notification template keys MUST resolve to a template in a targeting sidecar.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-065",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Notification template section `id` values MUST be unique within a template.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "G-066",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "BreachPolicy escalationStepId MUST resolve within the task pattern.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // --- I (Integration Profile, absorbed into Kernel §9.2 per ADR 0076 D-8) ---
    RuleMetadata {
        id: "I-001",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "`outputBinding` JSONPath MUST NOT use filter expressions or recursive descent.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some(
            "kernel/spec.md §9.2 (per ADR 0076 step 12 — Integration Profile §3.3.1 absorbed)",
        ),
        suggested_fix: None,
    },
    // --- K (Kernel + Lifecycle Detail + Correspondence Metadata) -----
    RuleMetadata {
        id: "K-001",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Final states MUST NOT have outgoing transitions.",
        fixtures: &["crates/wos-conformance/fixtures/K-001-negative-final-transitions.json"],
        graduation: Graduation::Tested,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-002",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Compound states MUST have `initialState` and `states`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-003",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Parallel states MUST have `regions`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-004",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "`cancellationPolicy` MUST only appear on `parallel` states.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-005",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "`historyState` MUST only appear on `compound` states.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-006",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Transition `target` MUST reference an existing state.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-007",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Event names MUST NOT use the `$` prefix (kernel-reserved).",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-008",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Parallel state outgoing transitions MUST use `$join` as event.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-009",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Actor identifiers MUST be unique.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-010",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "createTask `assignTo` MUST reference a declared kernel actor.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-012",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Guards MUST be valid FEL expressions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-013",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Milestone conditions MUST be valid FEL expressions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-014",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Milestone `id` values MUST be unique.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-015",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "`setData` path MUST reference a declared `caseFile.fields` entry.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // K-016: Tested via inline JSON in the `k016_*` unit tests in
    // `crates/wos-lint/tests/tier1_rules.rs` — six cases covering
    // top-level known/unknown initialState, compound known/unknown,
    // deeply-nested compound (3 levels), parallel-region compound,
    // bare-region initialState (G1), and forEach body (G1).
    // Each "flagged" test asserts severity=Error AND the exact path
    // string. Inline-evidence annotation per K-EXT-002 precedent.
    // An empty-string `initialState` is treated like any other key
    // and emits K-016 because empty is never a valid map key (a state
    // map cannot legitimately carry the empty key).
    RuleMetadata {
        id: "K-016",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "`lifecycle.initialState` MUST key into `lifecycle.states`; \
                  compound `initialState` MUST key into its substates; \
                  `region.initialState` MUST key into `region.states`; \
                  forEach `body` is walked recursively.",
        fixtures: &[],
        graduation: Graduation::Tested,
        spec_ref: Some(
            "specs/kernel/spec.md §4.1 (lifecycle.initialState) + §4.3 (compound state semantics) + §4.8 (parallel-state region semantics)",
        ),
        suggested_fix: Some(
            "Set `lifecycle.initialState` to a key that exists in `lifecycle.states`, OR (for compound states) set the state's `initialState` to a key that exists in its own `states` substate map, OR (for parallel-state regions) set `region.initialState` to a key that exists in `region.states`.",
        ),
    },
    RuleMetadata {
        id: "K-017",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "FEL guards MUST NOT reference related case state.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-019",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "FEL functions MUST be declared built-ins or registered extensions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-021",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Provenance `actorId` MUST reference a declared actor.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-022",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Digest present implies algorithm recorded in extensions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-029",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "`startTimer` MUST specify exactly one of `duration` or `deadline`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-030",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Extension keys MUST be `x-` prefixed.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-031",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "`transition.actor` MUST match `actors[].id` pattern and name a declared `actors[].id`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-032",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "`lifecycle.initialState` (root) and compound `initialState` MUST name a key in the relevant `states` map; diagnostic `path` distinguishes cases.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-037",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Fail-fast `$join` fires only on an error final state.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    RuleMetadata {
        id: "K-048",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Non-standard case relationship `type` values MUST use `x-` prefix.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // K-049: Unit tests in `crates/wos-lint/src/rules/continuous_mode.rs`; executable
    // fixtures in `fixtures/validation/k-049-load-bearing-*.json` wired from
    // `crates/wos-lint/tests/tier2_rules.rs`.
    RuleMetadata {
        id: "K-049",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Continuous-mode kernels MUST NOT contain `setData` → guard dependency cycles.",
        fixtures: &[
            "fixtures/validation/k-049-load-bearing-self-loop.json",
            "fixtures/validation/k-049-load-bearing-two-node-cycle.json",
        ],
        graduation: Graduation::LoadBearing,
        spec_ref: Some("specs/kernel/spec.md#123-continuous-mode"),
        suggested_fix: Some(
            "Break the write→read cycle (split transitions, narrow guards, or move writes off the hot path); use guard-only transitions (omit `event`) for §10.3 re-scan — never author `$`-prefixed event names on transitions (K-007).",
        ),
    },
    RuleMetadata {
        id: "K-050",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Final state `outcomeCode` MUST NOT duplicate any entry in `tags` (Kernel S4.3).",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("Kernel S4.3"),
        suggested_fix: Some(
            "Remove the duplicate from `tags` or choose a different `outcomeCode` value.",
        ),
    },
    // K-051 / K-052 / K-053 — DecisionTable lint rules per Kernel §4.5.1.
    // Unit tests in `crates/wos-lint/src/rules/decision_table.rs`; executable
    // fixtures in `crates/wos-conformance/fixtures/K-05[123]-*.json` wired
    // from `crates/wos-lint/tests/decision_table_fixtures.rs`.
    RuleMetadata {
        id: "K-051",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "DecisionTableGuard ref/outputColumn/inputBindings MUST resolve.",
        fixtures: &[
            "K-051-resolved-decision-table-guard.json",
            "K-051-negative-unresolved-table-ref.json",
            "K-051-negative-unresolved-output-column.json",
            "K-051-negative-missing-input-binding.json",
        ],
        graduation: Graduation::Tested,
        spec_ref: Some("specs/kernel/spec.md#§4.5.1"),
        suggested_fix: Some(
            "Ensure DecisionTableGuard.ref names a top-level decisionTables[] entry, outputColumn names a declared output, and inputBindings covers every declared input.",
        ),
    },
    RuleMetadata {
        id: "K-052",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "DecisionTable rows for hitPolicy=unique/priority MUST be pairwise disjoint (or have distinct priorities under priority).",
        fixtures: &[
            "K-052-disjoint-unique-rows.json",
            "K-052-negative-overlapping-unique-rows.json",
            "K-052-negative-priority-tie.json",
        ],
        graduation: Graduation::Tested,
        spec_ref: Some("specs/kernel/spec.md#§4.5.1.4"),
        suggested_fix: Some(
            "Make rows pairwise disjoint, switch to hitPolicy=first, or assign distinct priority integers.",
        ),
    },
    RuleMetadata {
        id: "K-053",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "DecisionTable cell-shape: input cells boolean; transition-guard outputColumn boolean-typed; no collect hit policy on guards.",
        fixtures: &[
            "K-053-boolean-output-column.json",
            "K-053-negative-non-boolean-output.json",
            "K-053-negative-collect-hit-policy-on-guard.json",
        ],
        graduation: Graduation::Tested,
        spec_ref: Some("specs/kernel/spec.md#§4.5.1.4"),
        suggested_fix: Some(
            "Select a boolean-typed output column for transition-guard usage; avoid collect hit policy on tables referenced by guards.",
        ),
    },
    // K-EXT-002: Tested via inline JSON in the `k_ext_002_*` unit tests in
    // crates/wos-lint/src/rules/tier2.rs (e.g. `k_ext_002_root_level_x_wos_key_flagged`).
    // The two linked `fixtures/validation/x-wos-*.json` files are authoring
    // artifacts, not executed by the conformance harness. (See 2026-04-18 review.)
    RuleMetadata {
        id: "K-EXT-002",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "`x-wos-*` namespace is reserved for future normative WOS use.",
        fixtures: &[
            "fixtures/validation/x-wos-reserved-warn.json",
            "fixtures/validation/x-vendor-custom-ok.json",
        ],
        graduation: Graduation::Tested,
        spec_ref: None,
        suggested_fix: None,
    },
    // ── K-FOREACH-* (foreach state semantics; ADR 0076 / Sub-PR D) ─────────
    // K-FOREACH-001..004: Lint emit is exercised via the kernel-typed
    // `check_state_type_semantics_typed` walker in `tier1.rs`; runtime
    // conformance fixtures at `crates/wos-conformance/fixtures/K-FOREACH-{001,
    // 002,003,BODY-001,OUTPUT-001}-*.json` cover the end-to-end behavior
    // (foreach iteration / bounded concurrency / empty collection / body
    // actions / collect-strategy outputs). Inline-evidence annotation per
    // the K-EXT-002 precedent — the lint emit is the ratchet-relevant
    // surface; runtime fixtures exercise the same shape. (See 2026-04-18
    // review for the K-EXT-002 precedent.)
    RuleMetadata {
        id: "K-FOREACH-001",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "ForEach states MUST declare a non-empty `collection` FEL expression.",
        fixtures: &[],
        graduation: Graduation::Tested,
        spec_ref: Some("specs/kernel/spec.md ForEach states"),
        suggested_fix: Some(
            "Declare an FEL expression in `collection` that evaluates to a bounded array against case state (e.g., `caseFile.attachments`).",
        ),
    },
    // K-FOREACH-002: same evidence as K-FOREACH-001 above (lint emit in
    // tier1.rs walker; runtime conformance in `crates/wos-conformance/
    // fixtures/K-FOREACH-BODY-001-iteration-body-actions.json`). Inline
    // annotation per K-EXT-002 precedent.
    RuleMetadata {
        id: "K-FOREACH-002",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "ForEach states MUST declare a `body` State to execute per iteration.",
        fixtures: &[],
        graduation: Graduation::Tested,
        spec_ref: Some("specs/kernel/spec.md ForEach states"),
        suggested_fix: Some(
            "Declare an inline `body: State` describing the work performed per iteration. The body MAY be atomic, compound, or parallel.",
        ),
    },
    // K-FOREACH-003: same evidence pattern as K-FOREACH-001/002 (lint
    // emit + runtime conformance). Inline annotation per K-EXT-002
    // precedent.
    RuleMetadata {
        id: "K-FOREACH-003",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "ForEach `concurrency` MUST be at least 1 when present.",
        fixtures: &[],
        graduation: Graduation::Tested,
        spec_ref: Some("specs/kernel/spec.md ForEach states"),
        suggested_fix: Some(
            "Set `concurrency` to a positive integer or omit it (sequential iteration is the canonical default).",
        ),
    },
    // K-FOREACH-004: iteration-field-isolation check; emit in tier1.rs
    // walker tested via the inline tests `k_foreach_004_*` (sentinel
    // patterns in tier1_rules.rs). Inline annotation per K-EXT-002.
    RuleMetadata {
        id: "K-FOREACH-004",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Iteration fields (collection, itemVariable, indexVariable, concurrency, breakCondition, outputPath, mergeStrategy, body) are valid only on `foreach`-typed states.",
        fixtures: &[],
        graduation: Graduation::Tested,
        spec_ref: Some("specs/kernel/spec.md ForEach states"),
        suggested_fix: Some(
            "Move iteration fields to a `foreach`-typed state, or remove them from this state.",
        ),
    },
    // --- SCHEMA-DOC (schema documentation coverage) -------------------
    RuleMetadata {
        id: "SCHEMA-DOC-001",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Schema leaf properties MUST carry sufficient `description` and `examples`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // SCHEMA-OPEN-001: inline-evidence in tests/schema_open_string_kind.rs.
    RuleMetadata {
        id: "SCHEMA-OPEN-001",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Open string leaves MUST declare `x-wos.openStringKind` unless `enum`/`const`/`pattern` already close the leaf.",
        fixtures: &[],
        graduation: Graduation::Tested,
        spec_ref: None,
        suggested_fix: None,
    },
    // --- SIG (Signature Profile) --------------------------------------
    RuleMetadata {
        id: "SIG-001",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Signature Profile `targetWorkflow.url` MUST match the loaded kernel URL.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("specs/profiles/signature.md#21-signature-profile-document"),
        suggested_fix: None,
    },
    RuleMetadata {
        id: "SIG-002",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Signature Profile roles MUST reference declared kernel actors.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("specs/profiles/signature.md#22-signer-roles"),
        suggested_fix: None,
    },
    RuleMetadata {
        id: "SIG-003",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Signature Profile roles MUST bind to human kernel actors.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("specs/profiles/signature.md#22-signer-roles"),
        suggested_fix: None,
    },
    RuleMetadata {
        id: "SIG-004",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Signature role authenticationPolicyKey values MUST resolve.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("specs/profiles/signature.md#26-signer-authentication-policy"),
        suggested_fix: None,
    },
    RuleMetadata {
        id: "SIG-005",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Signature signing-step roleId values MUST resolve.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("specs/profiles/signature.md#23-signing-flow-patterns"),
        suggested_fix: None,
    },
    RuleMetadata {
        id: "SIG-006",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Signature signing-step documentId values MUST resolve.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("specs/profiles/signature.md#27-document-binding"),
        suggested_fix: None,
    },
    RuleMetadata {
        id: "SIG-007",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Signature signing-step dependencies MUST resolve and MUST NOT cycle.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("specs/profiles/signature.md#23-signing-flow-patterns"),
        suggested_fix: None,
    },
    RuleMetadata {
        id: "SIG-008",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Routed signing guards MUST parse as valid FEL.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("specs/profiles/signature.md#23-signing-flow-patterns"),
        suggested_fix: None,
    },
    RuleMetadata {
        id: "SIG-009",
        tier: Tier::T2,
        severity: LintSeverity::Warning,
        summary: "Signature lifecycle tags SHOULD appear in the target kernel.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("specs/profiles/signature.md#24-lifecycle-tags"),
        suggested_fix: None,
    },
    RuleMetadata {
        id: "SIG-010",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Signature reminder and expiry events MUST map to kernel events.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some(
            "specs/profiles/signature.md#29-reminders-expiry-decline-void-and-reassignment",
        ),
        suggested_fix: None,
    },
    RuleMetadata {
        id: "SIG-011",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "SignatureAffirmation evidence inputs MUST be satisfiable.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("specs/profiles/signature.md#28-signatureaffirmation-provenance"),
        suggested_fix: None,
    },
    RuleMetadata {
        id: "SIG-012",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Signature Profile fields MUST follow Ref/Key/Id naming conventions.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: Some("thoughts/adr/0060-cross-reference-naming-ref-key-id.md"),
        suggested_fix: None,
    },
    // --- VR (Verification Report) -------------------------------------
    RuleMetadata {
        id: "VR-003",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "`counterexample` MUST be present when result is `proven-unsafe`.",
        fixtures: &[],
        graduation: Graduation::Draft,
        spec_ref: None,
        suggested_fix: None,
    },
    // --- WOS-* (cross-reference rules per ADR 0076 D-2 / step 12) -----
    // WOS-AGENT-XREF-001: inline-evidence in `wos_agent_xref_001_*` tests.
    RuleMetadata {
        id: "WOS-AGENT-XREF-001",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Every actor with `type=='agent'` MUST have a matching `agents[].id`.",
        fixtures: &[],
        graduation: Graduation::Tested,
        spec_ref: Some("ADR 0076 D-2 + step 12; schemas/wos-workflow.schema.json allOf $comment"),
        suggested_fix: Some(
            "Add an `agents[]` entry whose `id` matches the agent-typed actor, or change the actor `type` to `human`/`system`.",
        ),
    },
    // --- WOS-EMBED-* / WOS-SIDECAR-* (identity-boundary rules per ADR 0063) ---
    // WOS-EMBED-IDENTITY-001: Tested via inline JSON in
    // `embed_identity_001_governance_with_url_flagged` and
    // `embed_identity_001_advanced_with_version_flagged` unit tests in
    // `tier1.rs` (the lint emit fires on embedded blocks declaring
    // `url`/`version`). Inline annotation per K-EXT-002 precedent.
    RuleMetadata {
        id: "WOS-EMBED-IDENTITY-001",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Embedded blocks MUST NOT declare independent `url` or `version`. The enclosing $wosWorkflow envelope is the sole identity.",
        fixtures: &[],
        graduation: Graduation::Tested,
        spec_ref: Some("ADR 0063 §2.1; schemas/wos-workflow.schema.json allOf"),
        suggested_fix: Some(
            "Remove `url` and `version` from the embedded block. The merged envelope's identity (url, version) governs every embedded block.",
        ),
    },
    // WOS-EMBED-TARGET-001: Tested via inline JSON in
    // `embed_target_001_governance_with_target_workflow_flagged`,
    // `embed_target_001_agents_array_entry_with_target_workflow_flagged`,
    // and `embed_target_001_clean_envelope_silent` unit tests in
    // `tier1.rs`. Inline annotation per K-EXT-002 precedent.
    RuleMetadata {
        id: "WOS-EMBED-TARGET-001",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Embedded blocks (governance, agents, aiOversight, signature, custody, advanced, assurance) MUST NOT declare `targetWorkflow`.",
        fixtures: &[],
        graduation: Graduation::Tested,
        spec_ref: Some("ADR 0063 §2.1; schemas/wos-workflow.schema.json allOf"),
        suggested_fix: Some(
            "Remove `targetWorkflow` from the embedded block. Embedded blocks govern the enclosing $wosWorkflow envelope; only sidecars target workflows by URI.",
        ),
    },
    // WOS-SIDECAR-TARGET-001: Tested via inline JSON in
    // `sidecar_target_001_delivery_without_target_workflow_flagged`,
    // `sidecar_target_001_delivery_with_empty_target_workflow_flagged`,
    // `sidecar_target_001_delivery_with_valid_target_workflow_silent`,
    // and `sidecar_target_001_ontology_alignment_without_target_workflow_flagged`
    // unit tests in `tier1.rs`. Inline annotation per K-EXT-002 precedent.
    RuleMetadata {
        id: "WOS-SIDECAR-TARGET-001",
        tier: Tier::T1,
        severity: LintSeverity::Error,
        summary: "Sidecar documents ($wosDelivery, $wosOntologyAlignment) MUST declare `targetWorkflow` as a non-empty workflow URI.",
        fixtures: &[],
        graduation: Graduation::Tested,
        spec_ref: Some("ADR 0063 §2.2"),
        suggested_fix: Some(
            "Declare `targetWorkflow` on the sidecar root pointing at the $wosWorkflow envelope's `url`.",
        ),
    },
    // WOS-SIG-COVER-001: Tested via inline JSON in
    // `wos_sig_cover_001_signature_transition_without_signature_block_flagged`,
    // `wos_sig_cover_001_covered_signer_clean`, and
    // `wos_sig_cover_001_signers_missing_actor_flagged` unit tests in
    // `crates/wos-lint/src/rules/tier2.rs::tests` (the inline test
    // module, NOT `crates/wos-lint/tests/tier2_rules.rs`). Inline
    // annotation per K-EXT-002 precedent.
    RuleMetadata {
        id: "WOS-SIG-COVER-001",
        tier: Tier::T2,
        severity: LintSeverity::Error,
        summary: "Signature-gated transitions MUST be covered by a `signature.signers[]` entry.",
        fixtures: &[],
        graduation: Graduation::Tested,
        spec_ref: Some("ADR 0076 D-2 + step 12; schemas/wos-workflow.schema.json allOf $comment"),
        suggested_fix: Some(
            "Declare a `signature` block at the document root with `signers[]` covering the actor that signs the gating transition.",
        ),
    },
    // WOS-VER-LEVEL-001: Tested via inline JSON in
    // `ver_level_001_fallback_without_verification_level_warns`,
    // `ver_level_001_fallback_with_binding_verification_level_clean`, and
    // `ver_level_001_no_fallback_chain_silent` unit tests in `tier1.rs`.
    // Inline annotation per K-EXT-002 precedent.
    RuleMetadata {
        id: "WOS-VER-LEVEL-001",
        tier: Tier::T1,
        severity: LintSeverity::Warning,
        summary: "Agents declaring `fallbackChain` SHOULD have at least one `verificationLevel` declared on output bindings.",
        fixtures: &[],
        graduation: Graduation::Tested,
        spec_ref: Some("ADR 0076 step 12 (Q6 owner decision)"),
        suggested_fix: Some(
            "Declare `verificationLevel` on the `bindings[]` entries that govern outputs from this agent's fallback path.",
        ),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_ids_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for rule in ALL_LINT_RULES {
            assert!(
                seen.insert(rule.id),
                "duplicate rule id in registry: {}",
                rule.id
            );
        }
    }

    #[test]
    fn registry_ids_are_sorted() {
        let ids: Vec<&str> = ALL_LINT_RULES.iter().map(|r| r.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted, "registry entries must be sorted by id");
    }
}
