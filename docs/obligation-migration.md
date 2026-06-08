<!-- Non-normative migration guide for the Activation Criteria + Durable Obligations feature (ADR 0096; gate WOS-MIG-2602). Normative contract: specs/governance/workflow-governance.md §16; shapes in schemas/wos-workflow.schema.json. This is migration guidance, not a new contract: the hand-rolled timer + guard + milestone topology REMAINS valid. -->

# Migrating to durable obligation policies (ADR 0096)

Before ADR 0096, a recurring shape — *"after X, Y must happen by deadline T by role R, else apply action A"* — had no WOS-native primitive. Authors hand-rolled it out of a kernel **timer** (the deadline), a transition **guard** (the "has Y happened?" gate), a **milestone** (the "X happened" trigger), and bespoke provenance. An `ObligationPolicy` (under `governance.obligationPolicies[]`) now expresses the whole duty as one durable, future-tense object with a governed lifecycle (`pending → satisfied/violated/cancelled/expired/bypassed`) and first-class provenance.

This guide is **non-normative**. The contract is [Governance §16](../specs/governance/workflow-governance.md); the shapes live in [`schemas/wos-workflow.schema.json`](../schemas/wos-workflow.schema.json). For the concepts, read the [authoring guide](activation-and-obligations.md) first.

> **The explicit topology remains valid.** Nothing about ADR 0096 deprecates hand-rolled timer + guard + milestone constructions. They are still conformant kernel/governance authoring and are still the right tool when you need fine-grained, transition-local control (see [When *not* to migrate](#when-not-to-migrate)). An obligation policy is a *higher-level* expression of one common pattern, not a replacement for the underlying primitives.

## Before — hand-rolled timer + guard + milestone

The duty *"once income is reported after submission, an independent underwriting review must complete within 2 business days, else block final approval"* spread across four places:

- a **milestone** firing when `caseFile.income` changed after submission (the "X happened" signal);
- a kernel **timer** armed off that milestone for the 2-business-day window (the deadline);
- a transition **guard** on `finalApprovalRequested` asserting `reviewCompleted == true` (the "has Y happened?" gate);
- **bespoke provenance** records the author had to define and emit by hand to record activation, the block, satisfaction, and timer expiry — with no shared vocabulary, so each workflow named these events differently.

Costs of the explicit form for *this* pattern:

- **No durable pending state.** "An obligation is outstanding" was implicit in the conjunction of timer-armed + guard-false; nothing named it, so nothing could report or audit it as a single thing.
- **Separation-of-duties was manual.** "The reviewer must not be the person who reported the change" had to be re-encoded in the guard with custom actor bookkeeping.
- **Provenance fragmented.** Activation, violation, satisfaction, cancellation, and expiry had no canonical record kinds; downstream audit/export could not recognize "an obligation lifecycle" generically.
- **Three primitives to keep in sync.** Editing the deadline meant touching the timer *and* the guard *and* the milestone consistently.

## After — one obligation policy

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

The four hand-rolled pieces collapse:

| Hand-rolled piece | Obligation-policy clause |
|---|---|
| milestone "X happened" trigger | `activateWhen` |
| guard "has Y happened?" gate | `satisfyWhen` (+ `notSameAsTriggerActor` for separation of duties) |
| kernel timer for the window | `deadline.within` (+ `calendarRef`) |
| guard on the downstream transition | `violateWhen` + `onViolation: "block"` (checked before the kernel applies the event) |
| bespoke provenance records | canonical `ObligationActivated` / `Satisfied` / `Violated` / `Cancelled` / `Expired` / `Bypassed` / `Warning` kinds, with PROV-O/XES/OCEL export |

The duty now has a durable pending lifecycle the runtime monitor owns, recognizable provenance, and built-in separation-of-duties — without the author manually coordinating three primitives.

## When *not* to migrate

Prefer the narrower primitive when it fits; do not rewrite working topologies preemptively.

- **A guard alone suffices.** If you only need to gate one transition on a local boolean (no future-tense duty, no deadline, no cross-event tracking), keep the transition guard. Wrapping it in a policy adds lifecycle machinery you do not need.
- **A milestone alone suffices.** A one-shot data-driven checkpoint that fires and is done is a milestone, not an obligation. There is no "must happen by then" follow-on.
- **A task SLA covers it.** A single task that must complete within a window is an `SlaDefinition` with deadline actions — that is the task-scoped tool. (Note: SLA/Hold `ActivationCriteria` *runtime* wiring is a deferred follow-up; see [`TODO.md`](../TODO.md) and the conformance summary's deferred-claims note.)
- **It is an agent pre-commit check.** "This agent's output must satisfy a requirement *now*, before commit" is the deontic `Obligation` under `agents[].deontic.obligations`, not a durable cross-event policy. (The bare noun `Obligation` is reserved for the deontic concept; the durable concept is always the two-word `ObligationPolicy` / `PendingObligation`.)
- **The construction works and is stable.** Migration is opt-in. A correct, well-tested timer + guard + milestone implementation does not need rewriting to "modernize" it. Migrate when you are *already* editing that area, when you want the durable pending state surfaced for audit/reporting, or when separation-of-duties / cross-actor independence is load-bearing.

Avoid **premature migration**: do not bulk-convert existing workflows into obligation policies as a refactor. Convert opportunistically, fixture the converted workflow, and verify the event → provenance trace matches the prior behavior before retiring the hand-rolled pieces.

## Migration checklist

1. Identify the pattern — confirm it is genuinely *"after X, Y by T by R, else A"* and not one of the narrower primitives above.
2. Map each hand-rolled piece to a policy clause using the table above; pick `onViolation` (`warn` < `escalate` < `fail` < `block`, or the `createTask` / `emitEvent` object form).
3. Add `notSameAsTriggerActor` to `satisfyWhen.actor` if separation of duties was previously encoded in the guard.
4. Run the lint (`ACT-001..010`) — `where` must parse as boolean-shaped FEL, referenced events/tasks/calendars must resolve. See [`LINT-MATRIX.md`](../LINT-MATRIX.md) Tier 2.
5. Author a conformance fixture (`OBL-*` family) capturing the event → provenance trace and assert it matches the pre-migration behavior. See the [conformance summary](obligation-conformance.md).
6. Retire the now-redundant timer, guard, and milestone — only after the fixture is green.

## See also

- Concepts + "which primitive when": [`docs/activation-and-obligations.md`](activation-and-obligations.md)
- Worked event → provenance traces: [`docs/obligation-examples.md`](obligation-examples.md)
- LLM authoring snippets (plain-language → policy JSON): [`docs/obligation-authoring-prompts.md`](obligation-authoring-prompts.md)
- Conformance / fixture mapping: [`docs/obligation-conformance.md`](obligation-conformance.md)
- Normative contract: [`specs/governance/workflow-governance.md` §16](../specs/governance/workflow-governance.md)
- Decision record: [`thoughts/adr/0096-shared-activation-criteria-and-durable-obligations.md`](../thoughts/adr/0096-shared-activation-criteria-and-durable-obligations.md)
