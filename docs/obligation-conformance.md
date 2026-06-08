<!-- Conformance summary for the Activation Criteria + Durable Obligations feature (ADR 0096; gate WOS-GATE-2902). Maps each OBL-* runtime fixture and each ACT-* lint rule to the normative §16 claim it exercises. Normative contract: specs/governance/workflow-governance.md §16. Non-normative; this is a traceability index, not a new contract. -->

# Durable Obligations — conformance summary (ADR 0096)

Traceability index for the Activation Criteria + Durable Obligations feature. It maps each runtime conformance fixture (`OBL-*`) and each static lint rule (`ACT-*`) to the normative claim it exercises in [Governance §16](../specs/governance/workflow-governance.md). Non-normative: the contract is §16; the lint registry is [`LINT-MATRIX.md`](../LINT-MATRIX.md).

> **Execution status.** The `OBL-*` fixtures and the `ACT-*` rules are **authored but not yet executed in this repository**. The standalone WOS checkout cannot compile Rust — `fel-core` and the private sibling repos (`formspec/`, `trellis/`, `workspec-server/`) are absent from the sandbox, so `cargo nextest` / `make ci` do not run here. The authoritative gate is the formspec-stack monorepo CI ([`.github/workflows/rust-tests.yml`](../.github/workflows/rust-tests.yml)) once the `STACK_REPOS_TOKEN` secret is set; until then the build step self-skips with a notice. Counts and pass/fail status below describe the *authored* fixtures and *registered* rules, not observed green runs (WOS-GATE-2903 / WOS-GATE-2904).

## 1. Runtime conformance — `OBL-*` (13 fixtures)

Fixtures live under `crates/wos-conformance/tests/fixtures/OBL-*.json`; harness in `crates/wos-conformance/tests/kernel_conformance.rs`. Each row maps a fixture to the §16 claim it exercises against the in-memory reference adapter.

| Fixture | Exercises (§16 claim) | Normative anchor |
|---------|-----------------------|------------------|
| `OBL-001` activation-creates-pending | An activating event creates exactly one `PendingObligation` (`ObligationActivated`). | §16.2.1, §16.2.2 (activation) |
| `OBL-002` no-activation-when-where-false | A false `activateWhen.where` guard activates nothing (zero `ObligationActivated`); no truthy coercion. | §16.1.2 (`where` boolean), §16.2.2 |
| `OBL-003` satisfy | An independent satisfier discharges the duty (`ObligationSatisfied`). | §16.2.2 (satisfaction + triggering actor) |
| `OBL-004` violation-before-satisfaction-blocks | A premature `violateWhen` event is blocked before the kernel applies it; the transition does not fire. | §16.2.3 step 4 (pre-gate), §16.2.4 (`block`) |
| `OBL-005` deadline-expiry-violates | A lazily-materialized deadline elapse violates/expires the obligation (`ObligationExpired`). | §16.2.1 (`deadline`), §16.2.3 step 1 (timers) |
| `OBL-006` cancellation | A `cancelWhen` match cancels (`ObligationCancelled`); the cancelled obligation's deadline does not later expire. | §16.2.2 (cancel + timer cancellation) |
| `OBL-007` actor-role-mismatch | A wrong-role actor does not satisfy; only the correct-role independent actor discharges (exactly one `ObligationSatisfied`). | §16.1.2 (actor clause), §16.2.5 (independence) |
| `OBL-008` duplicate-ignore-while-pending | A duplicate trigger under `ignoreWhilePending` yields a single pending obligation (one `ObligationActivated`). | §16.2.2 (`duplicatePolicy`) |
| `OBL-009` replay-determinism | Re-running the same event stream yields identical provenance kind/data order and identical transitions. | §16.2.3 (deterministic order + idempotent replay) |
| `OBL-010` business-calendar-deadline | A `deadline.within` resolved against a named calendar (best-effort; business-day expiry is the TIME-1002 follow-up). | §16.2.1 (`deadline` + `calendarRef`) |
| `OBL-011` dcr-bridge | DCR-bridge placeholder — exercises only the plain activation path (per-activity gating deferred, §16.3 WOS-INTEG-DCR-1602). | §16.3 (DCR boundary) |
| `OBL-012` policy-engine-materialization | `obligationHandling: { mode: "materialize" }` materializes a policy-engine directive into a `PendingObligation`. | §16.3 (PolicyObligationHandling, WOS-INTEG-POLICY-1801) |
| `OBL-013` ai-review-window | A supervisory agent's assessment activates an independent-review obligation; a human review satisfies it. | §16.2.5 (same-agent independence, WOS-INTEG-AI-1705) |

`OBL-001`/`OBL-003`/`OBL-005`/`OBL-010`/`OBL-011`/`OBL-012`/`OBL-013` assert via `assert_fixture_passes`; `OBL-002`/`OBL-004`/`OBL-006`/`OBL-007`/`OBL-008`/`OBL-009` add explicit provenance-count / transition / replay assertions. The §16.4 prose enumerates `OBL-001..010` by name and reads "among others" for the three integration fixtures (`OBL-011..013`).

**Deferred runtime claims** (not covered by an executable fixture yet, tracked in `TODO.md`): SLA/Hold `ActivationCriteria` runtime wiring; true business-day deadline expiry (WOS-OBL-TIME-1002 / TIME-1008); DCR per-activity activation gating (WOS-INTEG-DCR-1602); deadline-index performance work (2801).

## 2. Static analysis — `ACT-*` (10 rules, all `draft`)

Registered in `crates/wos-lint/src/rules/fel_analysis.rs`; cataloged in [`LINT-MATRIX.md`](../LINT-MATRIX.md) Tier 2. T2 ACT rules carry **unit-test** evidence (in-crate), not conformance fixtures — they stay `draft` because the FEL-parse and graph-resolution checks depend on `fel-core`, which the sandbox cannot compile.

| Rule | Checks | §16 anchor |
|------|--------|------------|
| `ACT-001` | Activation-criteria `where` parses as valid FEL. | §16.4 (lint), §16.1.1 |
| `ACT-002` | `where` AST root is boolean-shaped (mirrors AI-058; no truthy coercion). | §16.1.2 |
| `ACT-003` | Trigger `on.event` resolves to a known workflow event (SHOULD). | §16.1.1 |
| `ACT-004` | `requiredData` paths resolve to known case-file fields (SHOULD). | §16.1.1 |
| `ACT-005` | `within` is a valid ISO 8601 / `P<N>BD` duration. | §16.1.1 (`within`) |
| `ACT-006` | Business-day `within` declares a resolvable `calendarRef` (SHOULD). | §16.1.1, §16.2.1 |
| `ACT-007` | `activationCriteriaRef` resolves to a named criteria (no duplicate ids). | §16.1.4 |
| `ACT-008` | Milestone `activationCriteria.where` parses + is boolean-shaped (WOS-INTEG-MILE-1302; legacy `condition` stays K-013, never double-reports). | §16.3 (milestone composition) |
| `ACT-009` | `satisfyWhen.on.event` is reachable in the static event graph — warns on unreachable satisfaction (WOS-TOOL-2502). | §16.2.1 (`satisfyWhen`) |
| `ACT-010` | `onViolation.createTask.taskRef` / `emitEvent.event` resolve to a known task/event (WOS-TOOL-2503). | §16.2.4 (`createTask`/`emitEvent`) |

The `ACT-001`/`ACT-002` registration backs the normative §16.4 lint catalog; `ACT-003..007` are listed there as the normative catalog and are registry-listed `draft` pending FEL-backed unit/fixture evidence. `ACT-008..010` are the integration-phase rules (milestone FEL, unreachable satisfaction, impossible violation action).

## 3. Three-way agreement posture

Per §16.4, every obligation MUST is exercised against the **in-memory reference adapter** via the `OBL-*` fixtures and MUST remain implementable in the **production (Restate) adapter**; spec + reference + production adapter agree. The fail-closed rule (§16.4) requires a processor that does not support obligation policies to fail closed for `rightsImpacting` / `safetyImpacting` workflows that declare them. These guarantees are authored into the fixtures and spec; observed three-way green is a CI deliverable (see the execution-status note above).

## See also

- Normative contract: [`specs/governance/workflow-governance.md` §16](../specs/governance/workflow-governance.md)
- Concepts + "which primitive when": [`docs/activation-and-obligations.md`](activation-and-obligations.md)
- Worked event → provenance traces: [`docs/obligation-examples.md`](obligation-examples.md)
- Migrating from hand-rolled timer+guard+milestone: [`docs/obligation-migration.md`](obligation-migration.md)
- Lint registry: [`LINT-MATRIX.md`](../LINT-MATRIX.md) (Tier 2 `ACT-*`)
- Decision record: [`thoughts/adr/0096-shared-activation-criteria-and-durable-obligations.md`](../thoughts/adr/0096-shared-activation-criteria-and-durable-obligations.md)
