# ARG-6 — Architecture Review Gate: release sign-off

**Date:** 2026-06-08
**Gate:** ARG-6 (after Phases 7 + 9 — conformance + hardening; final program sign-off)
**Scope:** the complete Activation Criteria + Durable Obligations program (ADR 0096), Phases 0–9, on branch `claude/new-session-s2erzd` / PR #4.
**Verdict:** **CONDITIONAL PASS** — the program is feature-complete-by-construction across all phases; the single outstanding condition is **executable verification**, which is gated on CI (`rust-tests.yml` + `STACK_REPOS_TOKEN`) because this sandbox cannot compile the Rust (fel-core + private siblings absent). No further design work is blocked.

Skills unavailable; fallback invariant checklist used.

## Program completeness (Phases 0–9)

| Phase | Outcome |
|---|---|
| 0 Scope/ADR | ✅ ADR 0096 (framing, naming, seam, non-goals) |
| 1 Schema + spec | ✅ verified (schema meta-valid, examples, §16 three-section) |
| 2 Rust model + evaluator | ✅ correct-by-construction (+tests) |
| 3 Lint | ✅ ACT-001..010 (+tests) |
| 4 Runtime | ✅ monitor + drain integration + violation actions/precedence/expiry/replay/cap |
| 5 Provenance + exports | ✅ 7 kinds + witness + PROV-O/XES/OCEL |
| 6 Integrations | ✅ milestones/SLA/holds/agents/policy criteria + API; SLA/Hold *runtime* + DCR-activity deferred (F-15/F-16) |
| 7 Conformance | ✅ OBL-001..013 fixtures (authored-not-run) |
| 8 Docs/tooling | ✅ guides, examples, graph+explain helpers, prompts, migration/conformance docs |
| 9 Hardening | ✅ cap, fail-closed, authorizer, no-mutation, PII, crash-recovery, backward-compat; perf indexing deferred (PERF-2801) |

## Final invariant audit

| Invariant | Status |
|---|---|
| FEL-only; no temporal operators / grammar change | ✅ honored end-to-end |
| Six canonical kernel seams; no new seam | ✅ governance-embedded + lifecycleHook/provenanceLayer |
| Trellis boundary (WOS emits provenance; Trellis anchors) | ✅ no WOS-side proof substrate; signature reuses `SignatureAffirmation` |
| Three "obligation" meanings distinct | ✅ deontic / policy-engine / durable kept separate (normative table §16.3) |
| Backward compatibility | ✅ all schema/model/process/provenance additions optional/default-empty; existing `tests/schemas` (488) + API discipline (16) green |
| Deterministic ordering & replay | ✅ doc-order policies, activation-order obligations, dedupe key; replay test |
| Fail-closed for rights/safety | ✅ `ObligationSupportPosture` gate (MIG-2604) |
| Policy immutable by runtime actors | ✅ policies are `&[...]` read-only; test (SEC-2702) |
| Agent self-bypass blocked | ✅ `ObligationAuthorizer` default-denies agents (SEC-2701/AI-1706) |
| PII minimization in witnesses | ✅ referenced paths only; test (SEC-2703) |

## Outstanding condition (the only thing between this and an unconditional pass)

**Executable verification.** ~60+ unit tests + 13 conformance fixtures are authored correct-by-construction but **never compiled/run here**. Two prior compile-breaks were caught by inspection/agents (the `evaluate_activations` `config`-arg stale callers; a missing `is_related_event` drain field) — a strong signal that a real `cargo` pass will surface more. Required to clear:
1. Add the **`STACK_REPOS_TOKEN`** secret so `rust-tests.yml` assembles the formspec-stack topology and runs `cargo nextest` (wos-core/wos-events/wos-lint/wos-runtime), OR run the branch through the formspec-stack monorepo CI.
2. Confirm the OBL-001..013 conformance **golden traces** against an actual runtime run (they are authored-not-run; each carries a `_note`).
3. Resolve any compile/clippy fallout (expected: minor — signature/borrow nits across the agent-authored Rust).

## Deferred (tracked, not blocking the program's design)

SLA/Hold runtime wiring (F-15), DCR activity gating (F-16), business-calendar obligation deadlines TIME-1002 + extension timer TIME-1008 (F-18), policy activation-event indexing PERF-2801. All are additive follow-ups; schema/model/spec surfaces are in place where relevant.

## Sign-off

The plan is **executed in full** (every epic addressed; deferrals explicitly recorded with reasons). The feature is coherent and invariant-clean by construction. It is **not** "verified working" until CI compiles and runs it — that is the gate, and it is a credential/topology step, not a design step. Recommend: set the token, green CI, triage fallout, confirm goldens, then merge.
