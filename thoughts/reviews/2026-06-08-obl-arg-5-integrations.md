# ARG-5 — Architecture Review Gate: Phase-6 integrations

**Date:** 2026-06-08
**Gate:** ARG-5 (after Phase 6 — Epics 13–22, integrations into existing surfaces)
**Reviews:** schema+model foundations (SLA `startWhen`, hold `resumeWhen`, milestone `activationCriteria`, agent `preconditionCriteria`, `PolicyObligationHandling`+`obligationHandling`, `ActivationTrigger.eventScope`); runtime wiring (milestone criteria firing, policy-engine `materialize`, AI same-agent independence + agent-bypass guard, related-case `event_scope`); API schema (`pendingObligations`/status/bypass-extension); specs+lint (ACT-008/009/010, DCR/multi-agent/signature/related-case prose).
**Verdict:** **PASS (with deferrals noted)** — proceed to conformance + hardening.

Skills unavailable; fallback invariant checklist used. Rust not compiled here (fel-core + private siblings absent); CI-gated via `rust-tests.yml`.

## Standing invariants

| Invariant | Status | Note |
|---|---|---|
| FEL-only, no temporal operators | ✅ | Every integration consumes `ActivationCriteria` whose `where` is the same local boolean FEL; no new expression surface. |
| Six-seam invariant | ✅ | All additions are optional fields on existing governance/kernel/AI shapes or the policy binding; no new kernel seam. |
| Trellis boundary | ✅ | Signature-before-release reuses the `SignatureAffirmation` pipeline (no second meaning of "signed"); obligations still emit WOS provenance only. |
| Naming distinction | ✅ | `PolicyObligationHandling` bridges the *policy-engine* obligation directive into a durable `PendingObligation` explicitly; the three "obligation" meanings stay separate. |
| Backward compatibility | ✅ (verified) | Every schema/model add is optional/additive; `pytest tests/schemas` 488 pass; API-discipline 16/16 pass; canonical-seams OK; existing fixtures unaffected. |
| Three-way agreement | ◐ | Reference runtime wires milestones/policy-materialization/AI-independence/bypass; conformance fixtures land in Phase 7; Restate-adapter implementability unaffected (additive). |
| Deterministic ordering / replay | ✅ | Unchanged from ARG-4; integrations don't alter the drain ordering. |

## Gate-specific focus

1. **Each integration additive & backward-compatible.** ✅ — `startWhen`/`resumeWhen`/`activationCriteria`/`preconditionCriteria` sit alongside the existing `startAt`/`resumeTrigger`/`condition`/`preconditions`; no migration forced (matches ADR 0096 D-6 and §16.3).
2. **DCR / SLA / hold / deontic boundaries respected.** ✅ — §16.3 makes the DCR-vs-obligation line normative; deontic vs durable-obligation vs policy-engine kept distinct; SLA/hold *runtime* wiring deferred (no SLA-clock/hold state machine exists in `wos-runtime` yet) — schema+model surfaces are in place, flagged as the cleanest follow-up.
3. **`notSameAsTriggerActor` independence enforced.** ✅ — verified end-to-end (trigger actor recorded on activation; satisfaction compares; agent self-satisfy blocked) with a runtime test (AI-1705).
4. **Agent bypass blocked by default.** ✅ — `bypass_obligation` refuses agent actors with tamper/`ObligationViolated` provenance (AI-1706); spec §16.2.5 + ai-integration §4.7 state it.
5. **Related-case reads within boundary.** ✅ (evaluator) — `eventScope: related` matches related-case *events* (not state); the related-case event *source* plumbing is a documented runtime follow-up (drain passes `is_related_event=false` today).

## Deferrals carried to backlog (not blockers)

- **F-15:** SLA `startWhen` / hold `resumeWhen` *runtime* — needs an SLA-clock / hold-resume state machine that `wos-runtime` does not yet have. Schema+model ready.
- **F-16:** DCR activity-level activation gating (1602) — `constraintZones` are an untyped advanced array; no typed activity shape to attach to.
- **F-17:** AI-1702/1703/1704 (supervisory-review / sampling / drift→recalibration) realized as **obligation-policy patterns** (documented), not new engine code — correct per the "no special multi-agent engine" decision.
- **F-18:** business-calendar obligation deadlines (TIME-1002) + deadline extension scheduling (TIME-1008 timer side) remain on the calendar-aware path.

## Next

Phase 7 conformance fixtures (done in the next round) + Phase 9 hardening → ARG-6 release sign-off.
