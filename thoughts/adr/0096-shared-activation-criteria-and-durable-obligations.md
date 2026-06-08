# ADR 0096 — Shared Activation Criteria and Durable Pending Obligations

**Status:** Accepted
**Date:** 2026-06-08
**Scope:** WOS Layer 1 (Workflow Governance) — introduction of two WOS-native primitives: a reusable `ActivationCriteria` shape ("when does this become active?") and durable pending obligations (`ObligationPolicy` → `PendingObligation`, "once active, what must happen, by when, by whom, and what if it does not?"). Establishes framing, naming, kernel-seam placement, lifecycle ownership, and non-replacement of existing governance surfaces. Locks the rejection of Signal/Linear Temporal Logic as a FEL extension. Does not, by itself, ship schema, runtime, or conformance — it is the framing/scope-lock decision that gates the implementation program (Epics 1–29 of the Activation + Obligations backlog).

**Related:**
[ADR 0075 (rejection register)](../../../thoughts/adr/0075-rejection-register.md) #8 *FEL Conformance Profiles* and Kernel §7.4 (no FEL grammar extensions — the invariant this ADR is consistent with);
[ADR 0095 (governance pipeline DAG execution semantics)](./0095-governance-pipeline-dag-execution-semantics.md) D-5 (sub-layer execution topology runs *inside* a transition; the kernel statechart stays canonical — the same categorical separation applies to obligations);
[ADR 0064 (agent as first-class `ActorKind`; `AgentInvoker` port)](./0064-agent-actor-kind-and-invoker-port.md) (agent actors; `notSameAsTriggerActor` independence depends on actor identity);
the six canonical kernel extension seams (Kernel §10; reproduced in [`work-spec/CLAUDE.md`](../../CLAUDE.md) Decision heuristics §3);
[`work-spec/specs/governance/workflow-governance.md`](../../specs/governance/workflow-governance.md) (§5 pipelines, §8.1 rejection policy — neighboring governance surfaces);
[`work-spec/crates/wos-core/src/model/ai.rs`](../../crates/wos-core/src/model/ai.rs):287-311 (deontic `Obligation` — AI S4.4, *immediate pre-commit* constraint; first existing meaning of "obligation");
[`work-spec/crates/wos-core/src/deontic.rs`](../../crates/wos-core/src/deontic.rs) (deontic evaluation; the FEL-env reuse pattern an activation evaluator follows);
[`work-spec/crates/wos-runtime/src/policy_decision.rs`](../../crates/wos-runtime/src/policy_decision.rs):32-52 (policy-engine `Obligation` carried on `PolicyDecision.obligations[]`; second existing meaning of "obligation");
[`work-spec/crates/wos-runtime/src/companion.rs`](../../crates/wos-runtime/src/companion.rs) (companion-policy monitor — the runtime pattern the obligation monitor mirrors);
[`work-spec/crates/wos-runtime/src/milestones.rs`](../../crates/wos-runtime/src/milestones.rs) (once-fired evaluation pattern);
[`work-spec/crates/wos-core/src/instance.rs`](../../crates/wos-core/src/instance.rs):452-465 (`GovernanceState` — where pending obligations live);
[`work-spec/schemas/wos-workflow.schema.json`](../../schemas/wos-workflow.schema.json) `$defs/HoldPolicy`, `$defs/SlaDefinition` (existing activation-shaped surfaces this generalizes);
[`work-spec/TODO.md`](../../TODO.md) `Rejected` register and Backlog.

---

## 1. Context

### 1.1 WOS has many "when does this become active?" surfaces and no shared shape

The notion "this becomes relevant when an event/condition holds" is already encoded, separately, in at least seven places:

- transition **guards** (Kernel §15.5 FEL predicates on transitions);
- milestone **`condition`** (lifecycle milestones, lint K-013);
- task SLA **`startAt`** (`$defs/SlaDefinition`, enum `assignment | activation | custom-event`);
- hold **`resumeTrigger`** (`$defs/HoldPolicy` — an event name that unblocks a hold);
- DCR constraint zones (advanced — zone-local flexible-activity availability);
- agent capability **`preconditions: string[]`** (AI integration);
- the trigger half of deontic and policy-engine obligations.

Each surface re-expresses a subset of {event match, transition match, actor constraint, required-data presence, FEL predicate, deadline window} in its own ad-hoc shape. There is no reusable primitive an author or an LLM can learn once and apply everywhere. This is a Q-rubric "named-seams" smell: the same concept, scattered.

### 1.2 WOS has no durable "future duty" primitive

The complementary question — *once* something is active, **what must happen, by when, by whom, and what if it does not** — has no first-class home. Authors hand-roll it with an explicit timer plus a guard plus a milestone plus bespoke provenance. The pattern recurs constantly in the target domains:

> After income changes, an underwriting review must happen before final approval (or within 2 business days), by someone other than the triggering actor; otherwise block the approval and record why.

There is no structure that captures "pending → satisfied / violated / cancelled / expired" as a governed, replayable, provenanced lifecycle.

### 1.3 The temporal-logic temptation, and why it is rejected

The recurring discussion around "STL-style workflow logic" proposes adding Signal/Linear Temporal Logic operators (`G` globally, `F` eventually, `U` until) to FEL so that "Y must eventually happen after X" becomes an expression. This is rejected. Kernel §7.4 and [ADR 0075](../../../thoughts/adr/0075-rejection-register.md) #8 already reject FEL grammar extensions; FEL is a *local, deterministic, side-effect-free boolean predicate* over a single evaluation context. Temporal "eventually/until" semantics are not local predicates — they are statements about an event *history* and a *future*, which is precisely what WOS's event/time/actor/provenance machinery already owns. Encoding them as expression-language operators would (a) break FEL's local-determinism contract, (b) duplicate runtime semantics into the predicate layer, and (c) create a second, weaker model of time alongside timers and the kernel clock.

The correct decomposition is structural, not syntactic:

```
ActivationCriteria  →  "when does this become active?"   (FEL stays the local predicate inside `where`)
PendingObligation   →  "once active: what, by when, by whom, else what?"  (WOS owns the temporal/lifecycle semantics)
```

### 1.4 The word "obligation" is already overloaded — twice

Two distinct `struct Obligation` already exist in the codebase:

1. **Deontic `Obligation`** ([`ai.rs`](../../crates/wos-core/src/model/ai.rs):287-311, AI S4.4): `{ id, requirement (FEL), on_violation, reason, null_behavior, bypassable }`. This is an **immediate, pre-commit** check on an agent's output: at the moment an agent produces output, the requirement FEL must hold or the configured violation action fires. It does not persist; it has no deadline; it is not a future duty.
2. **Policy-engine `Obligation`** ([`policy_decision.rs`](../../crates/wos-runtime/src/policy_decision.rs):32-38): `{ id, data }`, carried on `PolicyDecision.obligations[]`. This is an **XACML-style directive** attached to an external allow/deny decision — "you may proceed, but you must also do Z."

The new concept is a **third** thing: a *durable, future-tense, cross-event workflow duty* with its own lifecycle. Introducing it without resolving the naming is a recipe for author/LLM confusion and Rust import ambiguity. D-2 resolves this.

### 1.5 Decision posture

Pre-release window, greenfield discipline per [`work-spec/CLAUDE.md`](../../CLAUDE.md): additive schema/model changes, no compatibility shim required, but existing fixtures and authoring forms (guards, milestones, SLAs, holds, DCR) MUST keep working. The cost of fixing the framing now — before any schema lands — is one ADR; the cost of fixing it after a primitive ships under a colliding name or with STL baked into FEL is a renormalization migration plus a grammar rollback.

---

## 2. Decision

### D-1. The feature is Activation Criteria + Durable Obligations, not temporal logic

WOS introduces two structural primitives. FEL remains the **only** expression language and remains a local boolean predicate; it gains **no** temporal operators, no `G`/`F`/`U`, no LTL/STL grammar, no conformance-profile dialects. "Eventually / until / by deadline" semantics are owned by WOS's event, timer, actor, and provenance machinery — expressed through `ActivationCriteria` (trigger + predicate) and obligation lifecycle (deadline + satisfaction events), never through expression syntax. This decision is consistent with and reinforces Kernel §7.4 / ADR 0075 #8.

### D-2. Naming: keep `ObligationPolicy` / `PendingObligation`; reserve the bare noun `Obligation`

The durable concept is named with the two-word forms **`ObligationPolicy`** (the author-time declaration) and **`PendingObligation`** (the runtime state instance). The bare noun `Obligation` is **reserved** and is *not* reused for the new concept. Disambiguation rule, normative for spec prose and code:

| Concept | Canonical name | Layer | Temporality |
|---|---|---|---|
| AI deontic constraint | **deontic `Obligation`** (`model::ai::Obligation`) | author-time AI block / runtime deontic eval | immediate, pre-commit |
| Policy-engine directive | **policy-engine obligation** (`policy_decision::Obligation`) | runtime integration boundary | per-decision directive |
| Durable workflow duty (new) | **`ObligationPolicy`** / **`PendingObligation`** | author-time governance block / runtime governance state | durable, future-tense |

Rationale for keep-not-rename: the two-word forms do **not** collide as Rust type names with either existing `Obligation` (no import ambiguity), the noun is semantically correct (it *is* a duty), and renaming to a coined term (`WorkflowDuty`, `PendingDuty`) would diverge from the backlog's IDs and from the domain vocabulary auditors use, for no structural gain. The conceptual collision is dissolved by always using the qualified two-word form for the new concept and by the spec carrying the table above (WOS-OBL-SPEC-0706, -0707). New code MUST NOT introduce a bare `Obligation` type for the durable concept.

### D-3. Kernel-seam placement: governance-embedded; lifecycle + provenance seams; no new seam

Obligation policies are authored inside the **`governance`** embedded block as `governance.obligationPolicies[]`. They attach to runtime behavior through existing canonical seams only: the **`lifecycleHook`** seam (the obligation monitor observes events and may gate them within the drain loop, exactly as the kernel statechart fires inside a transition) and the **`provenanceLayer`** seam (obligation lifecycle events emit WOS provenance records). `ActivationCriteria` is a reusable `$def` referenced by consuming surfaces; it introduces no new extension point. **No new kernel seam is created** (Q3 / named-seams invariant, [`CLAUDE.md`](../../CLAUDE.md) heuristic 3).

### D-4. `ActivationCriteria` is the shared, reusable activation primitive

`ActivationCriteria` is a single `$def` capturing `{ on (event/transition trigger), where (FEL boolean), actor (constraint), requiredData (presence paths), within (deadline window), calendarRef, id, description }`. It is evaluated deterministically (trigger → actor → requiredData → FEL → deadline-hint) in `wos-core` and is reusable by `ObligationPolicy` (`activateWhen`/`satisfyWhen`/`cancelWhen`/`violateWhen`) and, additively and optionally, by milestones, SLAs, holds, DCR activities, and agent preconditions. Existing surfaces keep their current shapes; `ActivationCriteria` is offered alongside them, never as a forced migration. `where` MUST evaluate to boolean; a non-boolean result fails activation (truthiness MUST NOT decide governance).

### D-5. Durable obligations own a governed lifecycle; WOS emits provenance, Trellis anchors

A `PendingObligation` has the lifecycle `pending → {satisfied | violated | cancelled | expired | bypassed}` with `satisfied`/`cancelled`/`violated`/`expired`/`bypassed` terminal. State lives on `GovernanceState` ([`instance.rs`](../../crates/wos-core/src/instance.rs):452) as `pendingObligations`, default-empty so existing process JSON deserializes unchanged. Every lifecycle event emits a WOS provenance record (`ObligationActivated/Satisfied/Violated/Cancelled/Expired/Bypassed/Warning`). Per the Trellis-boundary heuristic, WOS emits these records only; anchoring/export/sealing stays Trellis-side via `custodyHook`. No WOS-side proof substrate is created.

### D-6. Obligations run inside the workflow; they do not replace existing surfaces

Following the same categorical separation as [ADR 0095](./0095-governance-pipeline-dag-execution-semantics.md) D-5: the obligation monitor runs *inside* the runtime drain loop alongside the companion policy and milestone evaluation; it does not replace the kernel statechart, DCR zones, SLAs, holds, deontic constraints, or policy-engine integration. Boundaries:

- **DCR zones** govern zone-local flexible-activity sequencing; **obligation policies** govern cross-state / cross-actor / cross-task durable duties. (WOS-OBL-SPEC-0705.)
- **Deontic `Obligation`** stays the immediate pre-commit check on agent output; **`ObligationPolicy`** is the durable pending duty. (WOS-OBL-SPEC-0706.)
- **Policy-engine obligations** may be record-only or, when explicitly configured with a mapping/template, *materialize* into `PendingObligation`s; indeterminate decisions are never coerced. (WOS-OBL-SPEC-0707.)
- **Milestones / SLAs / holds** keep their current trigger fields; `ActivationCriteria` is an optional alternative, additive only.

---

## 3. Non-goals

This ADR explicitly does **not**:

- add STL/LTL syntax, symbolic `G`/`F`/`U` operators, or any temporal operators to FEL;
- add FEL conformance-profile dialects (already rejected, ADR 0075 #8);
- replace or deprecate DCR constraint zones;
- force migration of existing SLAs, holds, milestones, guards, or agent preconditions to `ActivationCriteria`;
- introduce a new kernel extension seam;
- create a WOS-side proof/anchoring substrate (that is Trellis via `custodyHook`);
- ship the schema, Rust model, runtime monitor, or conformance fixtures — those are the downstream epics this ADR gates.

---

## 4. Consequences

- **Enables** the implementation program: `ActivationCriteria` `$def` (Epic 1), obligation schema (Epic 6), typed model + evaluator (Epics 3–4, 8), runtime monitor in the drain loop (Epics 9–10), provenance + exports (Epics 11–12), integrations (Epics 13–22), conformance/docs/hardening (Epics 23–29).
- **Constrains** all downstream tickets to the names in D-2, the seam in D-3, and the boundaries in D-6.
- **Backward compatibility:** the `GovernanceState` and `governance` block changes are additive (default-empty / optional); existing workflow and process fixtures deserialize and pass unchanged. This is an acceptance criterion on every downstream ticket and is gated at each architecture-review checkpoint.
- **Three-way agreement** posture applies: every obligation runtime MUST is exercised against the in-memory reference adapter via conformance fixtures and must remain implementable in the Restate production adapter.
- **Verification posture for downstream work:** unsupported-processor behavior fails closed for `rightsImpacting` / `safetyImpacting` workflows (WOS-MIG-2604, WOS-SEC-2704); obligation policy is immutable by runtime actors and agents cannot self-bypass by default (WOS-SEC-2702, WOS-INTEG-AI-1706).
