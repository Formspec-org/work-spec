# wos-governance CHANGELOG

All changes scoped to the `wos-governance` release train. See
[`RELEASE-STREAMS.md`](../RELEASE-STREAMS.md) for scope.

**Stability commitment:** semver-strict at 1.0+. Breaking changes to governance
schemas, spec prose, or fixtures require a major bump once this stream reaches 1.0.

Versions are tagged as `governance-v<X.Y.Z>` in git.

## [Unreleased]

### Added

- **Activation criteria + durable obligation policies** ([ADR 0096](../thoughts/adr/0096-shared-activation-criteria-and-durable-obligations.md)).
  New `governance.obligationPolicies[]` array and a reusable `$defs/ActivationCriteria`
  shape (with `$defs/ObligationPolicy`, `ObligationDeadline`, `ObligationViolationAction`,
  `PolicyObligationHandling`) on `wos-workflow.schema.json`. Expresses "after X, Y must
  happen by deadline T by role R, else action A" as a durable lifecycle
  (`pending → satisfied/violated/cancelled/expired/bypassed`) with first-class provenance.
  Normative contract: workflow-governance.md §16. Distinct from DCR zones, task SLAs,
  milestones, the deontic `Obligation`, and the policy-engine obligation (§16 enumerates
  the distinctions). No FEL grammar change — FEL stays the local boolean predicate inside
  `where` (no temporal operators, ADR 0096 D-1). Required for `rightsImpacting` /
  `safetyImpacting` workflows that declare obligation policies; processors that do not
  support them fail closed for those impact levels.

  Composes under the `$wosWorkflow` envelope version per [ADR 0076](../../thoughts/adr/0076-product-tier-consolidation.md):
  exercising obligation policies adds the `governance` block (obligation subset) to a
  vendor's claims map — `$wosWorkflow@1.0 [governance(obligations)]`. See
  [`RELEASE-STREAMS.md`](../RELEASE-STREAMS.md) and [`COMPATIBILITY-MATRIX.md`](../COMPATIBILITY-MATRIX.md).

  Landed on branch (PR #4): schema, `wos-core` model + deterministic activation evaluator,
  `wos-lint` `ACT-001..010` (draft), `wos-events` obligation provenance kinds + export,
  `wos-runtime` monitor, and 13 `OBL-001..013` conformance fixtures (authored, not yet run —
  compilation + exact conformance traces are CI-gated). See
  [`docs/obligation-conformance.md`](../docs/obligation-conformance.md) and
  [`WOS-IMPLEMENTATION-STATUS.md`](../WOS-IMPLEMENTATION-STATUS.md) §5.

## [1.0.0] — 2026-04-20

Initial release of this stream; see [`COMPLETED.md`](../COMPLETED.md) for the
delivery trail.
