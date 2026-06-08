# ARG-3 — Architecture Review Gate: Rust model + activation evaluator

**Date:** 2026-06-08
**Gate:** ARG-3 (after Phase 2 — Epics 3, 4, 8)
**Reviews:** `crates/wos-core/src/model/activation.rs`, `crates/wos-core/src/model/obligation.rs`, `crates/wos-core/src/activation.rs`, `crates/wos-core/src/instance.rs` (`GovernanceState`), module wiring in `model/mod.rs` + `lib.rs`.
**Verdict:** **PASS (with CI-gated compilation caveat)** — proceed to Phase 3 (lint).

Skills unavailable; fallback invariant checklist used.

## ⚠️ Environment limitation (load-bearing for this gate)

**Rust cannot be compiled or tested in this container.** The workspace `Cargo.toml` declares `fel-core = { path = "../fel-core" }`; the sibling `formspec-stack` repos (`formspec`/`fel-core`) are not checked out in this isolated session, so `cargo check`/`cargo nextest` fail at manifest load. This is the documented WOS-GATE-2904 topology condition. Consequences:

- Phase 2 Rust is **correct-by-construction**, not executable-verified here. CI (full topology) is authoritative for `cargo nextest run -p wos-core`.
- Mitigations applied: FEL-env construction copied verbatim from the proven `crates/wos-core/src/deontic.rs:661-693` pattern (`parse`/`json_to_fel`/`MapEnvironment::with_fields`/`evaluate(...).value`); `ActorKind` reused from `wos_events` (exact `human`/`system`/`agent` serde match); `Eq` deliberately **not** derived on `PendingObligation` (holds `serde_json::Value`, which is `PartialEq` only); deferred-init borrow in `required_data_present` is a standard valid pattern; rustfmt applied then import order re-aligned to house style (capital-first, matching `deontic.rs`) — note the repo has **no** fmt CI gate / rustfmt.toml, so import ordering is cosmetic.

## Standing invariants

| Invariant | Status | Note |
|---|---|---|
| FEL-only, no temporal operators | ✅ | `eval_guard` evaluates `where` as a plain FEL boolean; non-boolean/null/error → `GuardNonBoolean`/`GuardError` (fails activation, no coercion). No temporal constructs. |
| Six-seam invariant | ✅ | Pure model + evaluator; no seam introduced. |
| Trellis boundary | ✅ | No provenance/anchoring here (Phase 5). |
| Naming distinction | ✅ | `model/obligation.rs` doc reserves bare `Obligation` for the deontic type; uses `ObligationPolicy`/`PendingObligation`. |
| Backward compatibility | ✅ (test-encoded) | `GovernanceState.pending_obligations` is `#[serde(default, skip_serializing_if = "Vec::is_empty")]`; tests `governance_state_backward_compat_without_pending_obligations` + `..._round_trips_with_pending_obligation` encode the contract (run in CI). Resolves ARG-1 F-3 / ARG-2 F-6. |
| Three-way agreement | ✅ (center built) | Evaluator lives in `wos-core` center; reused by runtime (Phase 4) + lint (Phase 3). |
| Deterministic ordering | ✅ | `evaluate_activation_criteria` short-circuits in spec order (trigger→actor→requiredData→guard→deadline) returning a deterministic `ActivationDecisionReason`. |

## Gate-specific focus

1. **Model ⇄ schema fidelity (ARG-2 F-5)** — every `$defs` field has a typed counterpart in camelCase: `ActivationCriteria` (`where`→`where_fel` with `#[serde(rename="where")]`), `ActivationTrigger`, `ActorConstraint` (`actorType`→`ActorKind`), `ObligationPolicy` (all four clauses typed as `ActivationCriteria`), `ObligationViolationAction` (untagged shorthand|object), `DuplicatePolicy` (default `IgnoreWhilePending`). Tests deserialize the schema examples verbatim and round-trip. ✅
2. **Evaluator determinism & FEL semantics** — 14 unit tests cover event mismatch/match, missing/null/present(+nested) required-data, FEL true/false/non-boolean/parse-error, actor id/role/type, `notSameAsTriggerActor` self-satisfaction block, and deadline-hint return. ✅ (CI-run)
3. **`GovernanceState` additive/backward-compatible** — see invariants row; no existing field changed. ✅
4. **No spec behavior leaked outside the Rust center** — evaluator is in `wos-core`; runtime/lint will consume it, not reimplement. ✅

## Findings carried forward

- **F-8 (Phase 3 lint):** lint `ACT-002` (boolean shape) should reuse the same boolean-result discipline as the evaluator's `GuardNonBoolean`; keep them consistent with existing `AI-058`/`fel_analysis.rs`.
- **F-9 (Phase 4 runtime):** the runtime obligation monitor must construct `ActivationContext` from the drain event path, populating `trigger_actor_id` on satisfaction/violation evaluation so `notSameAsTriggerActor` works end-to-end.
- **F-10 (CI):** first CI run on this branch is the real gate for Phase 2 — `cargo nextest run -p wos-core` (incl. the 14 activation + 9 obligation/model tests). If `fel_core` API has drifted from the `deontic.rs` signatures, it surfaces there.

## Next

Phase 3 — Epic 5: register `ACT-001..ACT-007` in `crates/wos-lint/src/rules/registry.rs` + `LINT-MATRIX.md`, implement the FEL-parse/boolean-shape/event/requiredData/duration/calendarRef/ref-resolution rules, with fixtures to graduate them past Draft. Stop at ARG-4 (after Phase 4 runtime).
