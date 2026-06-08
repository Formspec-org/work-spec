<!-- Non-normative LLM-authoring snippets for the Activation Criteria + Durable Obligations feature (ADR 0096; WOS-TOOL-2505). Serves Claim A (the spec → schema → lint → conformance authoring loop). Normative contract: specs/governance/workflow-governance.md §16. Concepts: docs/activation-and-obligations.md. Worked traces: docs/obligation-examples.md. -->

# Authoring obligation policies from plain language (LLM snippets)

Few-shot material for the LLM authoring loop (Claim A): a plain-language requirement on the left, the `governance.obligationPolicies[]` entry it should produce on the right. Non-normative; the contract is [Governance §16](../specs/governance/workflow-governance.md) and the shapes are in [`schemas/wos-workflow.schema.json`](../schemas/wos-workflow.schema.json). For concepts read the [authoring guide](activation-and-obligations.md); for full event → provenance traces read the [worked examples](obligation-examples.md).

## When NOT to author an obligation policy

Reach for a narrower primitive first. **Do not use an obligation policy when an SLA, DCR constraint zone, or hold fits better:**

- A single task must finish within a window → **task SLA** (`governance.taskCatalog[].slaPerInstance`), not an obligation policy.
- A case is suspended pending one external event → **typed hold** (`governance.holds`), not an obligation policy.
- Zone-local flexible-activity sequencing (condition/response/include/exclude) → **DCR constraint zone** (`advanced`), not an obligation policy.
- One transition should be gated by a predicate → **transition guard**, not an obligation policy.
- An agent's output must satisfy a requirement *now*, pre-commit → **deontic `obligation`** (`agents[].deonticConstraints`), not an obligation policy.

Author an **obligation policy** only when the duty is durable and **cross-event / cross-actor / cross-task**: "after X happens, Y must happen by T (by actor/role R), else action A — and record why." See the full table in the [authoring guide §4](activation-and-obligations.md#4-which-primitive-when).

## Translation checklist

When turning a requirement into a policy, fill these in order:

1. **Trigger** (`activateWhen.on`) — what event creates the duty?
2. **Discharge** (`satisfyWhen.on` + optional `actor`) — what event clears it, by whom? Add `notSameAsTriggerActor: true` if the requirement implies independence/separation of duties.
3. **Bound it** — a `deadline.within` (time) and/or a `violateWhen.on` event (ordering).
4. **Consequence** (`onViolation`) — `warn` | `escalate` | `fail` | `block`, or the object form for `createTask` / `emitEvent`. Use `block` only when a `violateWhen` event should be *prevented*.
5. **Duplicates** (`duplicatePolicy`) — default `ignoreWhilePending`; `createEachTime` when each trigger is its own duty; `coalesceByKey` (+ `correlationKey`) to group.

## Examples

### A. "After income changes, an independent underwriting review is required before final approval, within 2 business days; otherwise block the approval."

```json
{
  "id": "income-change-review-required",
  "activateWhen": { "on": { "event": "caseFileUpdated" }, "where": "event.field = 'income'" },
  "satisfyWhen": {
    "on": { "event": "underwritingReviewCompleted" },
    "actor": { "role": "underwriter", "notSameAsTriggerActor": true }
  },
  "violateWhen": { "on": { "event": "finalApprovalRequested" } },
  "deadline": { "within": "P2BD", "calendarRef": "urn:wos:calendar:federal-fy2026" },
  "responsibleRole": "underwriter",
  "onViolation": "block"
}
```

Reasoning surfaced: "independent" → `notSameAsTriggerActor: true`; "before final approval" → `violateWhen` on `finalApprovalRequested` + `onViolation: block` (ordering, not just time); "within 2 business days" → `deadline.within: "P2BD"`.

### B. "When an agent emits an assessment, a different human reviewer must sign off within a day, else escalate to the supervisor."

```json
{
  "id": "agent-assessment-human-review-required",
  "activateWhen": { "on": { "event": "agentAssessmentEmitted" }, "actor": { "actorType": "agent" } },
  "satisfyWhen": {
    "on": { "event": "humanReviewCompleted" },
    "actor": { "actorType": "human", "role": "reviewer", "notSameAsTriggerActor": true }
  },
  "deadline": { "within": "P1D" },
  "responsibleRole": "reviewer",
  "duplicatePolicy": "createEachTime",
  "onViolation": { "action": "escalate", "escalateTo": "supervisor" }
}
```

Reasoning surfaced: no ordering event named, so the only violation path is the deadline; "a different human reviewer" → `actorType: human` + `notSameAsTriggerActor: true`; "escalate to the supervisor" → object-form `escalate` with `escalateTo`.

### C. "An adverse decision can't take effect until notice is sent to the affected person."

```json
{
  "id": "adverse-action-requires-notice",
  "activateWhen": { "on": { "event": "adverseDecisionPrepared" } },
  "satisfyWhen": { "on": { "event": "noticeSent" }, "actor": { "role": "caseworker" } },
  "violateWhen": { "on": { "event": "adverseActionEffective" } },
  "deadline": { "within": "P3BD", "calendarRef": "urn:wos:calendar:federal-fy2026" },
  "responsibleRole": "caseworker",
  "onViolation": "block"
}
```

Reasoning surfaced: "can't take effect until" → `violateWhen` on the effective-action event + `onViolation: block`; no independence requirement (accountability act, not separation of duties) → no `notSameAsTriggerActor`.

### D. Counter-example — requirement that should NOT be an obligation policy

> "Primary review must complete within 10 days."

This is a single-task time bound — author it as a **task SLA**, not an obligation policy:

```json
{
  "taskCatalog": [
    { "id": "primaryReview", "actorRole": "primaryReview", "slaPerInstance": "P10BD" }
  ]
}
```

Use an obligation policy only if the duty spans events/actors/tasks or needs a durable pending lifecycle with violation handling.

## See also

- Concepts + "which primitive when": [`docs/activation-and-obligations.md`](activation-and-obligations.md)
- Worked event → provenance traces: [`docs/obligation-examples.md`](obligation-examples.md)
- Migrating from hand-rolled timer + guard + milestone: [`docs/obligation-migration.md`](obligation-migration.md)
- Conformance / fixture mapping: [`docs/obligation-conformance.md`](obligation-conformance.md)
- Normative contract: [`specs/governance/workflow-governance.md` §16](../specs/governance/workflow-governance.md)
- Lint: `ACT-001..010` (all `draft`) in [`LINT-MATRIX.md`](../LINT-MATRIX.md)
