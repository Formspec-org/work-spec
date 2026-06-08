<!-- Non-normative authoring guide for the Activation Criteria + Durable Obligations feature (ADR 0096). Normative contract lives in specs/governance/workflow-governance.md §16; structural truth in schemas/wos-workflow.schema.json. This doc serves the LLM/human authoring loop (Claim A). -->

# Activation Criteria & Durable Obligations (authoring guide)

WOS workflows often need to say:

> **After X happens, Y must happen — before Z, or within T — by actor/role R; otherwise apply action A and record why.**

Before ADR 0096 this was hand-rolled with a timer + a guard + a milestone + bespoke provenance. Two WOS-native primitives now express it directly. This guide is non-normative; the contract is [Governance §16](../specs/governance/workflow-governance.md), the shapes are in `schemas/wos-workflow.schema.json`.

> **Not temporal logic.** FEL gains no `G`/`F`/`U` or other temporal operators (ADR 0096 D-1). FEL stays a *local boolean predicate* inside `where`; WOS owns the event/time/actor/provenance semantics.

## 1. `ActivationCriteria` — "when does this become active?"

A reusable predicate composing up to five optional clauses (at least one required):

| Clause | Meaning |
|---|---|
| `on` | event/transition trigger — `event` (exact name), `eventTag`, `eventKind`, or `transitionTag` |
| `where` | FEL **boolean** guard over `caseFile` / `event` / `actor` / `context` (non-boolean fails activation — no truthy coercion) |
| `actor` | `actorId` / `role` / `actorType` (`human`\|`system`\|`agent`) / `notSameAsTriggerActor` |
| `requiredData` | dotted paths that MUST be present and non-null (presence check, not FEL) |
| `within` (+ `calendarRef`) | a deadline window (ISO 8601; `P<N>BD` = business days) |

Evaluation is deterministic and short-circuits: **trigger → actor → requiredData → `where` → deadline hint**.

```json
{ "on": { "event": "caseFileUpdated" }, "where": "event.field = 'income'" }
```

### Activation criteria vs. a transition guard

A **transition guard** decides whether *one specific transition* fires, evaluated only when that transition is considered. `ActivationCriteria` is a *reusable* "is this relevant now?" predicate consumed by many surfaces (obligation policies today; optionally milestones/SLAs/holds/DCR/agent preconditions). Use a guard to gate a transition; use activation criteria to describe a condition several features share.

## 2. Obligation policies — "once active, what must happen, by when, else what?"

An `ObligationPolicy` (authored under `governance.obligationPolicies[]`) is a **durable, future-tense, cross-event duty**. Each of its clauses is an `ActivationCriteria`:

- `activateWhen` (required) → creates a `PendingObligation`
- `satisfyWhen` (required) → discharges it (honors `notSameAsTriggerActor`)
- `cancelWhen` (optional) → cancels it
- `violateWhen` (optional) → an event that violates it *while pending* (checked before the kernel applies the event, so `block` can prevent it)
- `deadline` (optional) → time-bounds it
- `onViolation` (required) → `warn` \| `escalate` \| `fail` \| `block`, or an object form for `createTask` / `emitEvent`

### Lifecycle

```text
inactive ─activateWhen→ pending ─satisfyWhen→ satisfied (terminal)
                          ├─cancelWhen──────→ cancelled (terminal)
                          ├─violateWhen─────→ violated  (terminal)
                          ├─deadline expiry─→ expired   (terminal)
                          └─authorized bypass→ bypassed (terminal)
```

When multiple policies are violated by one event, **all** violations are recorded but the strictest action wins: `warn < escalate < fail < block`. `duplicatePolicy` controls re-activation while one is pending (`ignoreWhilePending` default, `createEachTime`, `replaceExisting`, `coalesceByKey`).

### Three things named "obligation" — keep them distinct

| Concept | Where | Temporality |
|---|---|---|
| deontic `Obligation` | `agents[].deontic.obligations` | immediate, pre-commit check on agent output |
| policy-engine obligation | `PolicyDecision.obligations[]` (integration) | per-decision directive |
| **`ObligationPolicy` / `PendingObligation`** | `governance.obligationPolicies[]` | **durable, future-tense workflow duty** |

The bare noun `Obligation` is reserved for the deontic concept; the durable concept is always the two-word form.

## 3. Canonical example — income-change review

> When reported income changes after submission, an independent underwriting review is required before final approval (within 2 business days); otherwise block the approval and record why.

```json
{
  "governance": {
    "obligationPolicies": [
      {
        "id": "income-change-review-required",
        "activateWhen": { "on": { "event": "caseFileUpdated" }, "where": "event.field = 'income'" },
        "satisfyWhen": {
          "on": { "event": "underwritingReviewCompleted" },
          "actor": { "role": "underwriter", "notSameAsTriggerActor": true }
        },
        "violateWhen": { "on": { "event": "finalApprovalRequested" } },
        "deadline": { "within": "P2D", "calendarRef": "urn:wos:calendar:federal-fy2026" },
        "responsibleRole": "underwriter",
        "duplicatePolicy": "ignoreWhilePending",
        "onViolation": "block"
      }
    ]
  }
}
```

**Event sequence → provenance:**

| Event | Effect | Provenance |
|---|---|---|
| `caseFileUpdated` (`field=income`) | pending obligation created | `ObligationActivated` |
| `finalApprovalRequested` *(before review)* | **blocked** — kernel event not applied | `ObligationViolated` (`effectiveAction: block`) |
| `underwritingReviewCompleted` (by a different underwriter) | discharged | `ObligationSatisfied` |
| *(deadline passes with no review)* | timer fires → expired | `ObligationExpired` |

The same actor who triggered the change cannot satisfy the review (`notSameAsTriggerActor`) — separation of duties.

## 4. Which primitive when?

| Use… | When |
|---|---|
| **transition guard** | gate whether one transition fires |
| **milestone** | a data-driven checkpoint fires once when a condition holds |
| **task SLA** | a single task must complete within a window |
| **typed hold** | a case is suspended pending one external event |
| **DCR constraint zone** | *zone-local* flexible-activity sequencing (condition/response/include/exclude) |
| **deontic obligation** | an agent's output must satisfy a requirement *now* |
| **obligation policy** | a **cross-event / cross-actor / cross-task** duty: "after X, Y must happen by T, else A" |

Prefer the narrower primitive when it fits; reach for an obligation policy when the duty spans events/actors/tasks or needs a durable pending lifecycle with violation handling.

## See also

- More worked examples (AI-review, due-process notice) with event → provenance traces: [`docs/obligation-examples.md`](obligation-examples.md)
- LLM authoring snippets (plain-language → policy JSON, and when *not* to use a policy): [`docs/obligation-authoring-prompts.md`](obligation-authoring-prompts.md)
- Migrating from hand-rolled timer + guard + milestone: [`docs/obligation-migration.md`](obligation-migration.md)
- Conformance / fixture mapping: [`docs/obligation-conformance.md`](obligation-conformance.md)
- Normative contract: [`specs/governance/workflow-governance.md` §16](../specs/governance/workflow-governance.md)
- Decision record: [`thoughts/adr/0096-shared-activation-criteria-and-durable-obligations.md`](../thoughts/adr/0096-shared-activation-criteria-and-durable-obligations.md)
- Lint: `ACT-001`/`ACT-002` (and catalogued `ACT-003..007`) in [`LINT-MATRIX.md`](../LINT-MATRIX.md)
