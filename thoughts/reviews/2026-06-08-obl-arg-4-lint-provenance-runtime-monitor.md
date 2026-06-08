# ARG-4 — Architecture Review Gate: lint + provenance + runtime monitor

**Date:** 2026-06-08
**Gate:** ARG-4 (folds Phase 3 lint; covers Phase 5 provenance kinds and the Phase 4 obligation monitor module).
**Reviews:** `crates/wos-lint/src/rules/fel_analysis.rs` (ACT-001/002) + registry/matrix; `crates/wos-events/src/provenance/{kind,record,audit_tier}.rs` + registry/API parity; `crates/wos-runtime/src/obligations.rs` (+ lib registration).
**Verdict:** **PASS (partial Phase 4; CI-gated compilation)** — the obligation monitor module is complete and self-contained; drain-loop integration (WOS-OBL-RUNTIME-0914) and deadline timers (Epic 10) are the documented next step and were deliberately **not** landed to keep the core event loop stable.

Skills unavailable; fallback invariant checklist used. Rust not compiled here (fel-core sibling absent); CI authoritative.

## Standing invariants

| Invariant | Status | Note |
|---|---|---|
| FEL-only | ✅ | ACT-001/002 only parse/shape-check `where`; monitor delegates all predicate eval to `wos-core` `evaluate_activation_criteria`. |
| Six-seam | ✅ | Monitor is a runtime module; emits via provenance records; no new seam. |
| Trellis boundary | ✅ | Obligation records are WOS provenance (governance category, no eventLiteral) — anchoring stays Trellis-side. |
| Naming distinction | ✅ | Provenance kinds + module use `Obligation*`/`ObligationPolicy`; deontic/policy-engine untouched. |
| Backward compatibility | ✅ | New provenance variants are additive (Facts tier); registry/API parity green; `drain.rs` untouched so existing runtime behavior is unchanged. |
| Three-way agreement | ◐ partial | Monitor is pure and reference-shaped; conformance fixtures (OBL-*) + drain integration needed to prove spec⇄reference⇄production. Deferred to Phase 4 wiring + Phase 7. |
| Deterministic ordering / replay | ◐ partial | Monitor evaluates policies in document order, pending obligations by index; obligation ids are deterministic (`{policyId}#{seq}`). Full replay/idempotency (WOS-OBL-RUNTIME-0916) lands with drain integration. |

## What landed

- **Lint (Phase 3):** ACT-001 (FEL parse, error) + ACT-002 (boolean shape, warning) over `governance.obligationPolicies[*].{activate,satisfy,cancel,violate}When.where`, reusing `fel_parse_failure_message`/`is_boolean_shaped` (mirrors AI-057/058). Registered; ACT-003..007 catalogued. Unit tests for valid/invalid/non-boolean/absent.
- **Provenance (Phase 5):** 7 `ProvenanceKind` variants (Facts tier, exhaustive match updated), 7 `ProvenanceRecord::obligation_*` constructors, registry (+7, totalCount 141) + API `FactsRecordKind` enum + ADR 0093 prose. **Python-verified green:** `check-recordkind-parity.py`, `test_record_kind_registry.py` (7/7), full `tests/schemas` (488 pass; 2 pre-existing format-checker failures unrelated).
- **Runtime monitor (Phase 4 core):** `obligations.rs` pure functions — `load_obligation_policies`, `evaluate_pre_event_gate` (violateWhen → block + ObligationViolated), `evaluate_activations` (duplicatePolicy: ignore/create/replace; coalesce simplified to per-policy), `evaluate_satisfactions` (honors `notSameAsTriggerActor` via snapshotted trigger actor), `evaluate_cancellations`. Snapshot-then-mutate-by-index keeps borrows disjoint from the `&mut WorkflowProcess`. 7 unit tests incl. the independence and pre-event-block paths.

## Findings carried forward

- **F-11 (next, WOS-OBL-RUNTIME-0914):** wire the monitor into `drain_once` — pre-event gate after companion-policy decision (~`drain.rs:95`, may block), activations/satisfactions/cancellations after kernel eval alongside milestones (~`drain.rs:173`). Construct `ObligationEvent` from the drain event path; thread `governance_json` to `load_obligation_policies`. Persist obligation state on save; append monitor provenance to `appended_provenance`.
- **F-12 (Epic 10):** deadline computation (`PendingObligation.deadline` currently `None`) + deadline timers reusing `runtime/timers.rs`; warning thresholds; cancel-on-satisfy/cancel.
- **F-13 (Phase 7):** OBL-001..010 conformance fixtures; promote ACT-001/002 Draft→Tested.
- **F-14 (CI):** first CI run validates all Phase 2–5 Rust (`wos-core`, `wos-events`, `wos-lint`, `wos-runtime`). The obligation monitor lifetime design (`ctx<'b> where 'a: 'b`) and the snapshot-then-mutate borrows are the spots to watch.

## Next

Land WOS-OBL-RUNTIME-0914 (drain integration) — the linchpin that makes obligations execute — then Epic 10 timers and Phase 7 conformance.
