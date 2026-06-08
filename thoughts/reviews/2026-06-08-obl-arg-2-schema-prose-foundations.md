# ARG-2 — Architecture Review Gate: schema + spec-prose foundations

**Date:** 2026-06-08
**Gate:** ARG-2 (after Phase 1 — Epics 1, 2, 6, 7)
**Reviews:** `schemas/wos-workflow.schema.json` (+8 `$defs`, `governance.obligationPolicies[]`); `specs/governance/workflow-governance.md` §16.
**Verdict:** **PASS** — proceed to Phase 2 (Rust model + activation evaluator).

Skills (`formspec-specs:wos-expert`/`wos-spec-author`) unavailable; fallback invariant checklist used.

## Standing invariants

| Invariant | Status | Note |
|---|---|---|
| FEL-only, no temporal operators | ✅ | `where` annotated `openStringKind: fel`, boolean-required; §16.1.2 step 4 forbids coercion; §16 intro + ADR D-1 reject `G`/`F`/`U`. |
| Six-seam invariant | ✅ | `obligationPolicies` in `governance` block; §16.3 attaches via `lifecycleHook` + `provenanceLayer`; no new seam. |
| Trellis boundary | ✅ | §16.3 + §16.4: WOS provenance only; anchoring via `custodyHook`. |
| Naming distinction (3 obligations) | ✅ | §16.3 disambiguation table (normative); bare `Obligation` reserved; `ObligationPolicy`/`PendingObligation` two-word form. Resolves ARG-1 F-2. |
| Backward compatibility | ✅ | `Governance` block has no `additionalProperties:false` → `obligationPolicies` additive; `tests/schemas` 488 pass; `check-canonical-seams` OK (71 files). |
| Three-way agreement (posture) | ✅ | §16.4 commits OBL-* fixtures + Restate implementability; tested at ARG-4. |
| Deterministic ordering / replay | ✅ (spec) | §16.2.3 fixes processing order; §16.2.2 duplicate semantics; replay determinism normative. Executable at ARG-4. |

## Gate-specific focus

1. **Schema/prose agreement** — property tables in §16.1.1 / §16.2.1 mirror the `$defs` field-for-field (`on`/`where`/`actor`/`requiredData`/`within`/`calendarRef`; `activateWhen`…`onViolation`, `duplicatePolicy` default `ignoreWhilePending`, `coalesceByKey`⇒`correlationKey`). ✅
2. **ActivationCriteria genuinely reused (ARG-1 F-1)** — all four `ObligationPolicy` clauses `$ref #/$defs/ActivationCriteria`; verified structurally. The "shared primitive" claim is real, not prose-only. ✅
3. **SCHEMA-DOC-001 coverage** — every new property carries `description`; critical nodes (`ActivationCriteria`, `ObligationPolicy`, `obligationPolicies`) carry `x-lm.{critical,intent}` + `examples` (≥5 on `ActivationCriteria`, ≥1 on `ObligationPolicy`). ✅
4. **Three-section rubric** — §16.1–16.2 Normative Contract, §16.3 Composition, §16.4 Conformance, labeled. ✅

## Validation evidence

- `python3` JSON Schema 2020-12 meta-validity: schema valid; 113 `$defs`; 9 new defs present.
- Examples validated (format-checked): `ActivationCriteria` ×5, `ObligationPolicy` ×1 all OK.
- Negative checks pass: `coalesceByKey` without `correlationKey` rejected; unknown property rejected; `createTask` without `taskRef` rejected; empty `ActivationCriteria` rejected (`minProperties`).
- `tests/schemas`: 488 passed, 1 xfailed, 2 failed. **The 2 failures (`test_assertion_reference_shape`) are pre-existing** — reproduced with the schema change stashed; cause is the in-session `jsonschema` lacking a registered `format` checker for `format: uri`, unrelated to this work.
- `scripts/check-canonical-seams.py`: OK (71 files).

## Findings carried forward

- **F-5 (Phase 2):** Rust `ActivationCriteria`/`ObligationPolicy` models must round-trip the schema examples verbatim (camelCase). Add the schema example JSON as a deserialization fixture in `wos-core`.
- **F-6 (Phase 2):** Re-verify backward-compat with executable evidence once `GovernanceState.pendingObligations` lands — existing process fixtures must deserialize unchanged (carries ARG-1 F-3).
- **F-7 (env):** `jsonschema`/`pytest` were installed into the container to run `tests/schemas`; the 2 `format: uri` failures are an in-session checker gap, not a regression. CI (which registers a format checker) is authoritative.

## Next

Phase 2 — Epics 3, 4, 8: `model/activation.rs`, `activation.rs` evaluator (reuse `deontic.rs` FEL-env), `model/obligation.rs`, `GovernanceState.pendingObligations`. Stop at ARG-3 before lint.
