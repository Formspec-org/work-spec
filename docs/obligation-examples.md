<!-- Non-normative worked examples for the Activation Criteria + Durable Obligations feature (ADR 0096). Normative contract: specs/governance/workflow-governance.md §16. Concepts + the canonical income-change example + "which primitive when" live in docs/activation-and-obligations.md; this file adds two further worked scenarios. -->

# Durable obligation examples

Worked, non-normative companions to the [authoring guide](activation-and-obligations.md). Each shows an `obligationPolicy` JSON fragment (authored under `governance.obligationPolicies[]`) and the event → provenance trace it produces. The contract is [Governance §16](../specs/governance/workflow-governance.md); the shapes are in [`schemas/wos-workflow.schema.json`](../schemas/wos-workflow.schema.json).

For the foundational concepts (the five `ActivationCriteria` clauses, the obligation lifecycle, `duplicatePolicy`, the `warn < escalate < fail < block` ladder, and "which primitive when"), read the [authoring guide](activation-and-obligations.md) first — this file does not repeat them.

Provenance kinds referenced below: `ObligationActivated`, `ObligationSatisfied`, `ObligationViolated`, `ObligationCancelled`, `ObligationExpired`, `ObligationBypassed`, `ObligationWarning`.

## 1. AI-review obligation — supervised independent review of an agent's output

> When a supervisory agent emits an assessment, an independent human review is required before that assessment can drive a final decision; the reviewer must not be the actor that produced the assessment, and the review must complete within the review window — otherwise escalate.

This pairs Claim B (agents as runtime actors) with a durable cross-actor duty. The agent's output *activates* the obligation; an **independent** reviewer (`notSameAsTriggerActor`) *satisfies* it; review-window expiry *escalates* via the deadline path.

```json
{
  "governance": {
    "obligationPolicies": [
      {
        "id": "agent-assessment-human-review-required",
        "description": "An agent's supervisory assessment requires independent human review before it drives a final decision.",
        "activateWhen": {
          "on": { "event": "agentAssessmentEmitted" },
          "actor": { "actorType": "agent" }
        },
        "satisfyWhen": {
          "on": { "event": "humanReviewCompleted" },
          "actor": { "actorType": "human", "role": "reviewer", "notSameAsTriggerActor": true }
        },
        "deadline": {
          "within": "P1D",
          "calendarRef": "urn:wos:calendar:federal-fy2026",
          "warningThresholds": [
            { "beforeBreach": "PT4H", "notify": ["reviewer"], "templateKey": "reviewDueSoon" }
          ]
        },
        "responsibleRole": "reviewer",
        "duplicatePolicy": "createEachTime",
        "onViolation": { "action": "escalate", "escalateTo": "supervisor", "reason": "Independent review of agent assessment not completed within the review window." }
      }
    ]
  }
}
```

Notes:

- `actor.actorType: "agent"` on `activateWhen` scopes activation to agent-produced events only (a human producing the same event does not activate the duty).
- `notSameAsTriggerActor: true` on `satisfyWhen` enforces independence: the actor that produced the assessment cannot also discharge the review (separation of duties). Because the trigger actor is an agent and the satisfier must be `human`, this is doubly enforced here.
- `duplicatePolicy: createEachTime` — each fresh agent assessment gets its own pending review rather than coalescing into one.
- The deadline drives escalation: there is no `violateWhen` event, so the only violation path is expiry of the `P1D` window. The `warningThresholds` entry fires an `ObligationWarning` four hours before breach.

**Event sequence → provenance:**

| Event | Effect | Provenance |
|---|---|---|
| `agentAssessmentEmitted` (by `eligibilityAgent`, an `agent`) | pending obligation created | `ObligationActivated` |
| *(4h before deadline, still pending)* | pre-breach warning to `reviewer` | `ObligationWarning` |
| `humanReviewCompleted` (by a `reviewer` who is **not** the agent) | discharged | `ObligationSatisfied` |
| *(alternative: deadline passes with no review)* | timer fires → escalated to `supervisor` | `ObligationExpired` (`effectiveAction: escalate`) |

## 2. Due-process notice obligation — block the adverse action until notice is sent

> When an adverse decision is prepared, a notice to the affected person is required; the effective adverse action must be blocked while the notice is still pending, and is unblocked once `noticeSent` discharges the obligation.

This expresses a due-process invariant structurally: *no adverse action takes effect before notice*. The `violateWhen` clause names the event that would violate the duty while pending, and `onViolation: block` prevents the kernel from applying it.

```json
{
  "governance": {
    "obligationPolicies": [
      {
        "id": "adverse-action-requires-notice",
        "description": "An adverse decision requires notice to the affected person before it takes effect.",
        "activateWhen": { "on": { "event": "adverseDecisionPrepared" } },
        "satisfyWhen": {
          "on": { "event": "noticeSent" },
          "actor": { "role": "caseworker" }
        },
        "violateWhen": { "on": { "event": "adverseActionEffective" } },
        "deadline": { "within": "P3BD", "calendarRef": "urn:wos:calendar:federal-fy2026" },
        "responsibleRole": "caseworker",
        "duplicatePolicy": "ignoreWhilePending",
        "onViolation": "block"
      }
    ]
  }
}
```

Notes:

- The `violateWhen` event (`adverseActionEffective`) is checked **before** the kernel applies it (§16.2.3). With `onViolation: block`, the kernel rejects that event while the obligation is pending — the adverse action cannot take effect until `noticeSent` discharges the duty. This is the load-bearing difference from a `deadline`-only policy: a deadline bounds *time*, `violateWhen` + `block` bounds the *ordering* of events.
- The deadline (`P3BD` — three business days, resolved against the named calendar) still applies as an independent violation path: if notice is not sent within three business days the obligation expires. Because `onViolation` is `block`, expiry records an `ObligationExpired` with `effectiveAction: block`; the blocked event simply stays blocked rather than failing the workflow.

**Event sequence → provenance:**

| Event | Effect | Provenance |
|---|---|---|
| `adverseDecisionPrepared` | pending obligation created | `ObligationActivated` |
| `adverseActionEffective` *(before notice)* | **blocked** — kernel event not applied | `ObligationViolated` (`effectiveAction: block`) |
| `noticeSent` (by the responsible `caseworker`) | discharged | `ObligationSatisfied` |
| `adverseActionEffective` *(after notice)* | applied normally — no pending obligation to violate | *(no obligation provenance; ordinary transition provenance)* |

The same actor *may* send the notice here (no `notSameAsTriggerActor`), because due-process notice is an accountability act by the deciding office, not a separation-of-duties check — contrast §1, where independence is the point.

## 3. Signature-before-release obligation — block payment until signed (WOS-INTEG-SIG-2001/2002)

> When a payment is prepared, an approver's signature is required before the payment takes effect; the payment-effective event is blocked while the signature is pending. For multiple approvers, signing is sequential — each approver's signature activates the next approver's obligation.

This composes the Signature Profile with a durable duty: a WOS `SignatureAffirmation` *satisfies* the obligation; the release event is gated by `violateWhen` + `block`. WOS does not invent a second meaning of "signed" — the obligation observes the same `SignatureAffirmation` + custody/export pipeline. Contract: [Governance §16.3 (Signature-before-release)](../specs/governance/workflow-governance.md).

```json
{
  "governance": {
    "obligationPolicies": [
      {
        "id": "payment-requires-approver-signature",
        "description": "A prepared payment requires the approver's signature before it takes effect.",
        "activateWhen": { "on": { "event": "paymentPrepared" } },
        "satisfyWhen": {
          "on": { "event": "signatureAffirmed" },
          "actor": { "role": "approver" }
        },
        "violateWhen": { "on": { "event": "paymentEffective" } },
        "deadline": { "within": "P2BD", "calendarRef": "urn:wos:calendar:federal-fy2026" },
        "responsibleRole": "approver",
        "duplicatePolicy": "ignoreWhilePending",
        "onViolation": "block"
      }
    ]
  }
}
```

**Sequential signing (WOS-INTEG-SIG-2002):** to require approver-A then approver-B, chain a second policy whose `activateWhen` matches A's `signatureAffirmed` (`actor.role: approver-a`) and whose `satisfyWhen` matches B's (`actor.role: approver-b`, optionally `notSameAsTriggerActor: true`). Each `SignatureAffirmation` is the satisfaction of one obligation and the activation of the next, reproducing the Signature Profile signing order without duplicating signing semantics.

**Event sequence → provenance:**

| Event | Effect | Provenance |
|---|---|---|
| `paymentPrepared` | pending obligation created | `ObligationActivated` |
| `paymentEffective` *(before signature)* | **blocked** | `ObligationViolated` (`effectiveAction: block`) |
| `signatureAffirmed` (by `approver`) | discharged | `ObligationSatisfied` (+ `SignatureAffirmation` provenance, anchored Trellis-side) |
| `paymentEffective` *(after signature)* | applied | *(ordinary transition provenance)* |

## 4. Related-case dependency obligation — wait on a related case to resolve (WOS-INTEG-REL-2102)

> A consolidation case cannot close until its related parent case is resolved; if the parent does not resolve within the window, escalate.

The obligation activates on *this* case but is satisfied by an event surfaced from a *related* case (`on.eventScope: related`). A `deadline` bounds the wait so a never-resolving parent escalates rather than hanging. The obligation reads the related case's surfaced *event*, not its state (no related-case FEL — that stays forbidden by K-017).

```json
{
  "governance": {
    "obligationPolicies": [
      {
        "id": "await-parent-case-resolution",
        "description": "This case cannot close until its related parent case resolves.",
        "activateWhen": { "on": { "event": "closeRequested" } },
        "satisfyWhen": { "on": { "event": "relatedCaseResolved", "eventScope": "related" } },
        "deadline": { "within": "P10BD", "calendarRef": "urn:wos:calendar:federal-fy2026" },
        "duplicatePolicy": "ignoreWhilePending",
        "onViolation": "escalate"
      }
    ]
  }
}
```

A multi-agent handoff (WOS-INTEG-AI-1707) uses the same shape with no related scope: an orchestrator step activates a child-agent-completion obligation (`activateWhen` on `childAgentDispatched`), satisfied by the child's `childAgentCompleted` event (`actor.actorType: agent`, `notSameAsTriggerActor: true` so the dispatching parent cannot self-satisfy); the `deadline` routes non-completion to the fallback chain. No multi-agent engine is needed — the orchestrator is one kind of `AgentInvoker` (ADR 0064).

## See also

- Concepts + canonical income-change example + "which primitive when": [`docs/activation-and-obligations.md`](activation-and-obligations.md)
- Normative contract: [`specs/governance/workflow-governance.md` §16](../specs/governance/workflow-governance.md)
- LLM authoring snippets (plain-language → policy JSON): [`docs/obligation-authoring-prompts.md`](obligation-authoring-prompts.md)
- Decision record: [`thoughts/adr/0096-shared-activation-criteria-and-durable-obligations.md`](../thoughts/adr/0096-shared-activation-criteria-and-durable-obligations.md)
