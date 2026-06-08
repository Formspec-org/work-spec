# ARG-1 — Architecture Review Gate: Activation + Obligations framing & scope lock

**Date:** 2026-06-08
**Gate:** ARG-1 (after Phase 0 of the Activation Criteria + Durable Obligations program)
**Reviews:** [ADR 0096](../adr/0096-shared-activation-criteria-and-durable-obligations.md); feature-matrix rows 1.11–1.14 + footnote ^12 (`WOS-FEATURE-MATRIX.md`); roadmap §5 entry + `WOS-IMPLEMENTATION-STATUS.md`; Backlog epic link (`TODO.md`).
**Verdict:** **PASS** — proceed to Phase 1 (schema + spec-prose foundations).

## Review mechanism

Plan specifies skill-driven review via `formspec-specs:wos-expert` / `formspec-specs:wos-spec-author`. **Those plugin skills are not available in this session**, so this gate uses the plan's documented fallback: the manual invariant checklist below. When the skills are available at a later gate, re-run this gate's checks through them.

## Standing invariants

| Invariant | Status | Note |
|---|---|---|
| FEL-only; no grammar drift / no symbolic operators | ✅ | ADR D-1 + Non-goals explicitly reject STL/LTL `G`/`F`/`U` and FEL conformance-profile dialects; reinforces Kernel §7.4 / ADR 0075 #8. `where` stays a local boolean predicate. |
| Six-seam invariant (no new seam) | ✅ | ADR D-3: obligations live in `governance` block; attach via existing `lifecycleHook` + `provenanceLayer` seams; `ActivationCriteria` is a `$def`, not a seam. |
| Trellis boundary (no WOS-side proof substrate) | ✅ | ADR D-5: WOS emits obligation provenance records only; anchoring/export/sealing stays Trellis-side via `custodyHook`. |
| Naming distinction from deontic + policy-engine obligations | ✅ | ADR D-2: bare `Obligation` reserved; new concept always `ObligationPolicy`/`PendingObligation`; disambiguation table is normative for prose + code. Confirmed both existing `struct Obligation` (ai.rs:290, policy_decision.rs:34) are not type-name clashes. |
| Backward compatibility | ✅ (claim) | ADR D-5/Consequences: `GovernanceState`/`governance` changes are additive (default-empty/optional). Not yet executable — re-verified at ARG-3 (model) once schema/model land. |
| Three-way agreement feasibility | ✅ (posture set) | ADR Consequences commit every obligation MUST to in-memory reference fixtures + Restate-adapter implementability. Tested at ARG-4. |
| Deterministic ordering & replay safety | ✅ (posture set) | Deferred to runtime (ARG-4); ADR records the requirement; OBL-009 replay fixture planned. |

## Gate-specific focus (ARG-1)

1. **Is the framing genuinely non-STL?** Yes. The decomposition is structural (`ActivationCriteria` trigger+predicate; obligation lifecycle owns temporality), not syntactic. No "eventually/until" leaks into the expression layer. ADR §1.3 argues this explicitly.
2. **Is the naming decision defensible & the seam correct?** Yes. Keep-not-rename is justified (no Rust clash, correct domain noun, avoids divergence from backlog IDs and auditor vocabulary) with a normative disambiguation table. Governance-embedded placement on `lifecycleHook`/`provenanceLayer` matches the ADR 0095 D-5 precedent (sub-layer execution runs *inside* the workflow; statechart stays canonical).
3. **Does scope avoid Q2/Q3 violations?** Yes. Q3 (named-seams): no new seam. Q2 (first-engagement/managed-hosting): obligations directly serve rights-impacting adjudication/permit/fraud workflows in scope. Non-goals fence off DCR replacement and forced SLA/hold/milestone migration.

## Findings / notes carried to later phases

- **F-1 (Phase 1):** `$defs/ObligationPolicy` MUST `$ref` `$defs/ActivationCriteria` for all four condition fields (`activateWhen`/`satisfyWhen`/`cancelWhen`/`violateWhen`) so the "shared primitive" claim (D-4) is structurally true, not just prose. ARG-2 verifies.
- **F-2 (Phase 1 prose):** WOS-OBL-SPEC-0706/0707 must reproduce the D-2 disambiguation table verbatim so the three "obligation" meanings are separated at the spec surface, not only in the ADR.
- **F-3 (Phase 2):** Re-verify the additive/backward-compatible claim with executable evidence once `GovernanceState.pendingObligations` lands (existing process fixtures must deserialize unchanged).
- **F-4 (docs):** No repo-local markdown/link checker found; `scripts/check-canonical-seams.py` scans `specs/**` only and does not cover `thoughts/adr/`. ADR cross-stack links to `../../../thoughts/adr/0075-*` follow the established ADR 0095 form (sibling `formspec-stack/thoughts/` not checked out in this isolated container — expected per topology note).

## Next

Phase 1 — Epics 1, 2, 6, 7: `$defs/ActivationCriteria` (+ `ActivationTrigger`/`ActorConstraint`/`RequiredDataPath`), `$defs/ObligationPolicy`/`ObligationDeadline`/violation-action/duplicate-policy, `governance.obligationPolicies[]`, and the activation + obligation spec prose. Validate examples via `python3 -m pytest tests/schemas -q`. Stop at ARG-2 before Rust.
