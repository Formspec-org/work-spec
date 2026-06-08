# wos-kernel CHANGELOG

All changes scoped to the `wos-kernel` release train. See
[`RELEASE-STREAMS.md`](../RELEASE-STREAMS.md) for scope (paths, cadence, and the
cross-cutting artifacts that follow this stream).

**Stability commitment:** semver-strict after 1.0. Breaking changes require a major
bump; additive changes are minor; bug fixes are patch. Pre-1.0 (if any future
branches revisit it) has no such guarantee.

Versions are tagged as `kernel-v<X.Y.Z>` in git and (once Task 4 of the
release-trains plan lands) are computed by Changesets.

## [Unreleased]

### Added

- **Durable-obligation kernel/runtime + checking surface** ([ADR 0096](../thoughts/adr/0096-shared-activation-criteria-and-durable-obligations.md);
  cross-cutting artifacts follow kernel per [`RELEASE-STREAMS.md`](../RELEASE-STREAMS.md)).
  `wos-core` gains the `ActivationCriteria` / `ObligationPolicy` model and a deterministic
  activation evaluator; `wos-runtime` gains the obligation monitor wired into `drain_once`
  (activation / satisfaction / cancellation, pre-event violate-block, lazy deadline expiry,
  violation-action effects with strictest-action gating, replay dedupe, count cap,
  fail-closed posture, bypass/extension authorizer); `wos-events` adds seven obligation
  `ProvenanceKind` variants plus witness and PROV-O/XES/OCEL export; `wos-lint` registers
  `ACT-001..010` (Tier 2, all `draft` — unit-test evidence, fixtures land with the OBL-*
  batch); `wos-conformance` carries 13 `OBL-001..013` fixtures (authored, not yet run).
  The standalone repo cannot compile Rust (fel-core + private siblings absent); the
  authoritative gate is the formspec-stack monorepo CI / `rust-tests.yml` once
  `STACK_REPOS_TOKEN` is set. See [`docs/obligation-conformance.md`](../docs/obligation-conformance.md).
  Governance semantics are detailed in [`changelogs/governance.md`](governance.md).

## [1.0.0] — 2026-04-20

Initial release of this stream; see [`COMPLETED.md`](../COMPLETED.md) for the
delivery trail.
