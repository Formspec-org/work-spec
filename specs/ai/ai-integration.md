---
title: WOS AI Integration Specification
version: 1.0.0-draft.1
date: 2026-04-09
status: draft
---

# WOS AI Integration Specification v1.0

**Version:** 1.0.0-draft.1
**Date:** 2026-04-09
**Editors:** Formspec Working Group
**Companion to:** WOS Kernel Specification v1.0

---

## Abstract

The WOS AI Integration Specification defines Layer 2 of the Workflow Orchestration Standard: the governance structures for AI agent participation in WOS workflows. The `agents[]` and `aiOversight` embedded blocks of a `$wosWorkflow` document declare agent registration, deontic constraints on agent behavior, autonomy levels with impact-level caps, the Formspec-as-validator pattern, a confidence framework with decay and calibration, fallback chains guaranteeing graceful degradation, decision drift detection, AI-specific oversight extensions, volume constraints, agent-specific review sampling, agent disclosure for due process, the Narrative provenance tier, and the Assist Governance Proxy.

**ADR 0063 framing.** Per ADR 0063 §2.1, `agents[]` and `aiOversight` are *part of* the enclosing `$wosWorkflow` envelope; they are no longer standalone AI Integration Documents. The blocks govern the enclosing workflow and MUST NOT declare `targetWorkflow`, `url`, or `version`. Pre-merge prose in this spec that referenced "the AI Integration Document" or "targets a WOS Kernel Document via `targetWorkflow`" should be read in that light: the same content now lives under `$wosWorkflow.agents[]` + `$wosWorkflow.aiOversight`, joined to lifecycle, actors, governance, and case file in a single envelope.

**ADR 0064 framing.** Per ADR 0064, agents are first-class actors (`ActorKind::Agent` in the Rust runtime) and invocation goes through a substrate-neutral `AgentInvoker` port. The `agents[].invoker` discriminator selects which adapter (`wos-agent-anthropic`, `wos-agent-claude-sdk`, `wos-agent-mcp`, `wos-agent-a2a`, `wos-agent-http`, `wos-agent-stub`) handles a given agent's invocations. WOS specifies the *governance contract* the runtime enforces — deontic constraints, confidence floor, autonomy caps, fallback chain — and treats the substrate as adapter-tier. Multi-agent orchestrators are *one kind* of `AgentInvoker`, not a special spec concept. See `crates/wos-core/src/agent/mod.rs` for the port definition.

AI governance is not a separate track. Every Layer 2 concept extends a Layer 1 (Workflow Governance) concept through the kernel's named seams. Review protocols gain AI-specific suppression. Data validation pipelines gain Formspec-as-validator. Due process gains agent disclosure. Structured audit gains the Narrative tier. Quality controls gain agent-specific sampling.

WOS MUST NOT alter core Formspec processing semantics. WOS processors MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).

---

## Status of This Document

This document is a **draft specification**. It is a companion to the WOS Kernel Specification v1.0 and does not modify the kernel's processing model. It defines AI governance structures that attach to kernel workflows through the `actorExtension`, `contractHook`, `provenanceLayer`, and `lifecycleHook` seams. Implementors are encouraged to experiment with this specification and provide feedback, but MUST NOT treat it as stable for production use until a 1.0.0 release is published.

---

## 1. Introduction

### 1.1 Background

AI agents increasingly participate in high-stakes workflows. Empirical research constrains the design of any standard governing this participation:

- Naive human-AI combinations degrade decision quality compared to either humans or AI alone (Vaccaro et al., Nature Human Behaviour, 2024; meta-analysis of 106 experiments).
- Model-generated explanations are systematically post-hoc rationalizations, not faithful reasoning traces (Turpin et al., NeurIPS 2023; Lanham et al., Anthropic, 2023).
- Behavioral drift between model versions can cause near-total performance collapse on specific tasks (Chen, Zaharia, and Zou, 2023).
- No single defense against prompt injection has proven robust against adaptive adversaries.

This specification encodes these findings as structural requirements. The agent is outside the trust boundary. The WOS Processor -- not the agent -- enforces constraints, validates outputs, and controls workflow progression.

### 1.2 Design Goals

1. **AI plugs into human governance, not alongside it.** Every AI governance structure extends a Layer 1 structure.
2. **Constraints are external to the agent.** The WOS Processor enforces governance. The agent cannot weaken its own constraints.
3. **Graceful degradation is mandatory.** Every workflow MUST function without any agent participation.
4. **Formspec-as-validator.** Agent output is untrusted input validated against the same Formspec contract a human would submit against.
5. **Every normative claim is testable** (inherited from Kernel §1.2 Design Goal 6). Every AI-specific obligation in this spec — deontic constraint enforcement, autonomy-cap enforcement, confidence-decay semantics, fallback-chain execution, drift detection, assurance composition — MUST reduce to a conformance test or lint rule. An AI behavior that no test can falsify is not a governance guarantee. Authors adding a new AI obligation MUST also add the corresponding test artifact.

### 1.3 Scope

**Within scope:** agent registration via `actorExtension`; deontic constraints (permission, prohibition, obligation, right); autonomy levels with impact-level caps; Formspec-as-validator; confidence framework with decay; fallback chains; decision drift detection; AI-specific oversight extensions; volume constraints; agent-specific review sampling; agent disclosure; Narrative provenance tier; Assist Governance Proxy; conformance profiles.

**Out of scope:** SMT verification, equity guardrails, constraint zones, multi-step sessions, tool use governance, agent lifecycle state machine (Layer 3: Advanced Governance). Lifecycle topology, case state (Layer 0: Kernel). Due process, review protocols, data validation pipelines (Layer 1: Workflow Governance -- this layer extends them).

### 1.4 Relationship to Lower Layers

The `agents[]` and `aiOversight` embedded blocks govern the enclosing `$wosWorkflow` envelope's kernel surface. Per ADR 0063 §2.1, the blocks have no independent identity — the envelope's `url` and `version` are the sole identity boundary. Per ADR 0064, agents are first-class actors (`actors[]` entries with `type: "agent"`) joined to `agents[]` runtime declarations by `id` (lint `WOS-AGENT-XREF-001`). AI governance extends kernel and governance structures through four seams:

| Seam | What Layer 2 Attaches |
|------|----------------------|
| `actorExtension` | Agent taxonomy (deterministic/statistical/generative) on `agents[]` declarations; `ActorKind::Agent` is first-class per ADR 0064. |
| `contractHook` | Formspec-as-validator: agent output as untrusted input. |
| `provenanceLayer` | Narrative tier: non-authoritative model-generated explanation. |
| `lifecycleHook` | Deontic enforcement, autonomy enforcement, AI-specific oversight suppression, agent sampling, agent disclosure. |

**Substrate portability.** The `agents[].invoker` discriminator selects an `AgentInvoker` adapter (Anthropic SDK / Claude Agent SDK / MCP / A2A orchestrator / HTTP service / deterministic stub) at deploy time. The spec is portable across substrates because the runtime never names a concrete adapter; deployment binds the discriminator. See `crates/wos-core/src/agent/mod.rs` for the port and `crates/wos-agent-stub` for the canonical deterministic implementation used by conformance fixtures.

### 1.5 How AI Extends Human Governance

| Layer 1 Concept (Human) | Seam | Layer 2 Extension (AI) |
|---|---|---|
| Review protocols | `lifecycleHook` on `review` tags | AI-specific suppression: hide agent output until independent assessment. |
| Data validation pipelines | `contractHook` | Formspec-as-validator: agent output validated through same gates. |
| Due process (notice, appeal) | `lifecycleHook` on `adverse-decision` tags | Agent disclosure requirement added to notice. |
| Structured audit (Reasoning + Counterfactual tiers) | `provenanceLayer` | Narrative tier (non-authoritative, AI-specific). |
| Quality controls (review sampling) | `lifecycleHook` on `quality-check` tags | Agent-specific sampling: stratified, adversarial methods. |
| Separation of duties | `lifecycleHook` on `review` tags | Agent MUST NOT review its own output. |
| Override authority | `lifecycleHook` on `determination` tags | Agent output MUST NOT override human override. |
| Screener intake routing | `contractHook` | Agent-assisted screening: agent classification feeds Screener Determination Records via contractHook. |

### 1.6 Notational Conventions

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [BCP 14][RFC 2119] [RFC 8174] when, and only when, they appear in ALL CAPITALS, as shown here.

---

## 2. Conformance

### 2.1 Conformance Profiles

Three profiles are defined:

**AI Basic.** Agent registration (S3) and Formspec-as-validator (S6) are enforced. The processor correctly registers agents via the `actorExtension` seam and validates agent output against Formspec contracts.

**AI Governed.** Basic conformance plus: deontic constraints (S4), autonomy levels (S5), confidence framework (S7), fallback chains (S8), and agent disclosure (S12).

**AI Complete.** Governed conformance plus: decision drift detection (S9), AI-specific oversight extensions (S10), volume constraints (S11), agent-specific review sampling (S11), Narrative tier provenance (S13), and Assist Governance Proxy (S14).

### 2.2 Conformance Requirements

1. An AI Integration Processor MUST implement agent registration (S3).
2. An AI Integration Processor MUST implement the confidence framework (S7), including ConfidenceReport validation.
3. An AI Integration Processor MUST validate fallback chains at document load time, rejecting chains that cycle or lack a terminal action (S8).
4. An AI Integration Processor MUST delegate Formspec Definition evaluation to a Formspec-conformant processor (Core S1.4).

---

## 3. Agent Registration

This section is normative.

### 3.1 Agent as Actor Type

Layer 2 registers `agent` as an actor type via the kernel's `actorExtension` seam (Kernel S10.1). An agent actor has additional identity and provenance requirements beyond the kernel's `human` and `system` types.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique agent identifier within the workflow. |
| `type` | string | REQUIRED | MUST be `"agent"`. |
| `agentType` | enum | REQUIRED | `deterministic`, `statistical`, or `generative`. |
| `modelIdentifier` | string | REQUIRED | Identifier of the model powering this agent. |
| `modelVersion` | string | REQUIRED | Version of the model. |
| `description` | string | OPTIONAL | Human-readable description of the agent's role. |
| `capabilities` | array of Capability | OPTIONAL | Declared capabilities with I/O contracts. |
| `modelVersionPolicy` | enum | OPTIONAL | `pinned`, `approved`, or `latest`. Default: `pinned`. |

### 3.2 Agent Type Taxonomy

The kernel treats agents uniformly. Layer 2 differentiates governance by type:

| Type | Characteristics | Governance Implications |
|------|----------------|------------------------|
| `deterministic` | Rules engines, decision tables, lookup systems. Output is a pure function of input. | Confidence reporting is trivial (always 1.0). Drift detection focuses on input distribution shift. |
| `statistical` | Classical ML. Fixed model, bounded output space, calibratable confidence. | Confidence calibration is meaningful and required. Drift detection uses standard statistical methods. |
| `generative` | LLMs. Unbounded output space, non-deterministic, injection-vulnerable. | Confidence is difficult to calibrate. Formspec-as-validator is essential. Drift detection must account for prompt sensitivity. |

### 3.3 Capability Declaration

Each capability declares what the agent can do, with Formspec Definition references for I/O contracts:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Capability identifier. |
| `description` | string | OPTIONAL | What this capability does. |
| `inputContractRef` | string (URI) | OPTIONAL | Formspec Definition or JSON Schema for input. Uses headless contract pattern (Core S2.3). |
| `outputContractRef` | string (URI) | OPTIONAL | Formspec Definition or JSON Schema for output. |
| `preconditions` | array of string (FEL) | OPTIONAL | Boolean FEL expressions evaluated before invocation; see §3.3.1. |

### 3.3.1 Capability Preconditions

A capability MAY declare a `preconditions` array of FEL expressions (Kernel §7). Before the processor invokes the capability, every expression is evaluated against the current evaluation context -- `caseFile`, `@event`, `@agent`, and any other context a guard may reference (Kernel §7.4). Semantics:

1. All entries MUST evaluate to boolean `true` for the capability to be invocable.
2. If any entry evaluates to `false`, or fails to evaluate to a boolean, the capability is **skipped**: the processor does NOT invoke the agent and instead falls through to the fallback chain defined for this capability (§8).
3. Absent or empty `preconditions` (the default) means the capability is always invocable.
4. Every precondition evaluation MUST produce a provenance record with `recordKind: "capabilityInvocation"`. The record's `data.invocationBlocked` flag MUST be `true` when a precondition caused the processor to skip the agent, and the record's `outcome` MUST then be `preconditionNotSatisfied` (Kernel §8.2.2 reserved outcome literal) so that audit tooling can distinguish a declarative gate from an agent failure. This shape is enforced at schema-validation time by `$defs/CapabilityInvocationRecord` in `schemas/wos-workflow.schema.json` (promoted Facts-tier $defs per ADR 0076 step 5); persisted append-only logs validate against `schemas/wos-provenance-log.schema.json`, where `FactsTierRecord` composes `CapabilityInvocationRecord` via `allOf` so every conformant provenance log participates in the MUST.
5. Preconditions are evaluated **before** the agent input contract is rendered and before any guardrails run. They are the cheapest gate in the capability pipeline; use them to keep agents from being called against cases that are structurally wrong for them.

Preconditions are declarative: they say "only invoke this capability when the world looks like this." They do not relax deontic constraints (§4) -- a capability that passes its preconditions is still bound by every permission, prohibition, obligation, and right declared against it.

### 3.4 Model Version Policy

| Policy | Behavior |
|--------|----------|
| `pinned` | Agent uses exactly the declared `modelVersion`. Version changes require document update. |
| `approved` | Agent may use any version from an approved list. Version changes emit provenance but do not require document update. |
| `latest` | Agent uses the latest available version. Every invocation records the actual version used. |

When a version change is detected (for `approved` and `latest` policies), the processor MUST emit a provenance record of type `agentVersionChange`.

### 3.5 Trust Boundary

The agent is outside the trust boundary of the governance envelope. The WOS Processor -- not the agent -- enforces constraints, validates outputs, and controls workflow progression. This separation ensures governance survives agent changes, prompt injection, and behavioral drift.

When capability declarations reference domain-specific concepts, agents SHOULD use Formspec Ontology terms (Ontology S3) to establish semantic contracts. Ontology terms provide stable, machine-readable identifiers for domain concepts that survive agent replacement -- when one agent is swapped for another, the ontology contract remains the same even if the underlying model changes.

### 3.6 Security Patterns (informative)

The trust boundary model is compatible with the CaMeL (Capability-Limited Model) dual-LLM security architecture: a privileged controller validates whether an untrusted capability model's output should be committed. In WOS terms, the WOS Processor plays the controller role, and the agent plays the capability model role. Deontic constraints (S4) and Formspec-as-validator (S6) provide the validation mechanism. Implementations MAY adopt the CaMeL pattern as an implementation strategy for the trust boundary defined in S3.5.

### 3.7 Normative Constraints

1. Agents MUST NOT override human decisions.
2. Provenance records for `agent` actors MUST include model identifier, model version, confidence, and input summary.
3. Cascading autonomous agents (an agent at `autonomous` level invoking another agent at `autonomous` level) MUST be explicitly declared in the AI Integration Document via the `cascadingInvocations` property on the invoking agent's declaration. Each entry names a target agent that this agent may invoke at `autonomous` level.
4. An actor's type is immutable for a given action. A human using an AI tool remains a `human` actor if the human reviews and commits the output.

---

## 4. Deontic Constraints

This section is normative.

### 4.1 Overview

Agent constraints use four deontic types adopted from OASIS LegalRuleML. Deontic constraints are evaluated after interface contract validation and before output is committed to workflow state. The WOS Processor is the Policy Enforcement Point.

### 4.2 Permission

A Permission bounds what the agent is allowed to produce. Agent outputs within permission bounds are accepted; outputs outside are violations.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique permission identifier. |
| `allowedFields` | array of string | OPTIONAL | Fields the agent is permitted to produce. |
| `field` | string | OPTIONAL | Specific field this permission constrains. |
| `bounds` | string (FEL) | OPTIONAL | FEL expression defining value bounds. |
| `onViolation` | enum | REQUIRED | `reject`, `escalateToHuman`, `switchToAssistive`, or `flag`. |
| `nullBehavior` | enum | OPTIONAL | Override for null propagation: `pass`, `deny`, or `escalate`. |

### 4.3 Prohibition

A Prohibition forbids specified agent outputs or actions regardless of confidence.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique prohibition identifier. |
| `condition` | string (FEL) | REQUIRED | FEL expression defining the forbidden condition. When true, the prohibition is triggered. |
| `reason` | string | OPTIONAL | Human-readable explanation. Included in provenance when triggered. |
| `onViolation` | enum | REQUIRED | Enforcement action. |
| `nullBehavior` | enum | OPTIONAL | Override for null propagation. |

### 4.4 Obligation

An Obligation requires the agent to perform a specified action or include specified content. Checked after output, before commit.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique obligation identifier. |
| `requirement` | string (FEL) | REQUIRED | FEL expression defining the required condition. When false, the obligation is unmet. |
| `reason` | string | OPTIONAL | Human-readable explanation. |
| `onViolation` | enum | REQUIRED | Enforcement action. |
| `nullBehavior` | enum | OPTIONAL | Override for null propagation. |

### 4.5 Right

A Right specifies what the agent is entitled to receive as input context. The WOS Processor has an Obligation to provide data specified in agent Rights. A Rights violation MUST NOT be attributed to the agent and MUST trigger a system-level error.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | REQUIRED | Unique right identifier. |
| `entitlement` | string | REQUIRED | Path or reference specifying the entitled data. |
| `description` | string | OPTIONAL | Human-readable description. |

### 4.6 Enforcement Ordering

Deontic constraints MUST be evaluated by the WOS Processor (the Policy Enforcement Point, S4.1) in the following order. This is the runtime enforcement contract: a conformant processor that activates the AI Integration layer MUST execute these stages in this sequence for every agent invocation.

1. **Permissions** -- Is the agent permitted to perform this action? Structural bounds on allowed outputs (S4.2).
2. **Prohibitions** -- Is the agent prohibited from this action? Forbidden output patterns (S4.3).
3. **Obligations** -- Has the agent fulfilled its obligations? Required output elements (S4.4).
4. **Confidence floor** -- Does the agent's confidence meet the floor? (S7.4).
5. **Volume constraints** -- Has the agent exceeded volume constraints? Rate limits on autonomous actions (S11.1).
6. **Human review sampling** -- Is this action selected for quality review? Quality-assurance selection (S11.2; Governance S7).

When multiple constraints are violated simultaneously, the processor MUST apply the most restrictive enforcement action. Restriction ordering, from least to most restrictive: `flag` < `switchToAssistive` < `escalateToHuman` < `reject`. The `requireReview` and `log` actions exposed by external policy-engine bridges (see below) compose at the same precedence as `escalateToHuman` and `flag` respectively.

> **Editorial note (ADR 0076 absorption, 2026-04-28):** prior to this absorption, the Runtime Companion's §8.3 specified `log < flag < requireReview < reject < escalateToHuman` (with `escalateToHuman` most restrictive). The two documents disagreed on whether `reject` or `escalateToHuman` was the strictest action. Reconciliation here adopts the AI Integration spec's local ordering (`reject` strictest), bridges `requireReview`→`escalateToHuman` and `log`→`flag` for cross-doc compatibility, and treats the resulting four-action ordering as canonical. The decision rationale: `reject` is a final answer that terminates the agent invocation; `escalateToHuman` is recoverable (the human may approve), so it cannot logically dominate `reject`.

**Composition with external policy engines (deny-overrides-permit).** When a workflow integrates an external policy engine (XACML, OPA, Cedar, or equivalent) via the integration binding mechanism (Workflow §3, §8), the engine's decision composes with this deontic pipeline as a strict restrictor: a `deny` decision from a policy engine overrides any `permit` produced by S4.2-S4.4 evaluation. **External policy engines are more restrictive, never more permissive.** A policy-engine `permit` does not override a deontic `prohibition`, an unmet `obligation`, a confidence-floor failure, a volume-constraint violation, or a sampling selection. The engine's decision is recorded as a Facts-tier provenance record (Kernel §8) and made available under the binding's `outputBinding` path so guard expressions on subsequent transitions (Kernel §4.5) can reference it.

### 4.7 Constraint Composition

Deontic constraints compose across three levels. All applicable constraints at all levels are evaluated for every agent invocation:

1. **Workflow-level** -- constraints in the AI Integration Document's `deonticConstraints`. Apply to all agent invocations in the workflow.
2. **Agent-level** -- constraints declared on a specific `AgentDeclaration`. Apply to all invocations of that agent.
3. **Action-level** -- constraints on a specific action override in the Agent Config sidecar. Apply to a single invocation point.

When constraints at different levels conflict, the most restrictive enforcement action wins (same restriction ordering as S4.6).

**Consistency constraints** detect contradictions between agent output and case data or prior outputs (e.g., `not (output.eligible = true and output.riskScore > 90)`).

**Scope constraints** ensure output stays within declared capability boundaries by restricting which fields the agent may produce (Permission with `allowedFields`).

**Guardrail bypass:** An authorized actor with the required role MAY bypass a constraint marked `bypassable: true` with a structured rationale. Bypass applies to a single invocation only and produces a provenance record including the rationale and the bypassed constraint id.

**Durable-obligation independence and bypass (WOS-INTEG-AI-1705 / -1706).** The same separation-of-duties and no-self-bypass guarantees that govern deontic constraints here also govern durable obligation policies, and the obligation contract is normative for both: an agent that *activated* a durable obligation MUST NOT satisfy it when `satisfyWhen.actor.notSameAsTriggerActor` is set (Governance §16.2.5, same-agent independence), and **agents MUST NOT bypass a pending obligation by default** — an agent bypass attempt is rejected and recorded as a violation/tamper provenance record, never as a bypass; only an authorized human/admin actor with a structured rationale may bypass. See [Governance §16.2.5](../governance/workflow-governance.md).

### 4.8 Examples (non-normative)

**Permission** -- bound agent output to allowed fields and value ranges:

```
bounds: "output.eligibilityScore >= 0 and output.eligibilityScore <= 100"
```

**Prohibition** -- forbid agent from issuing final denial on rights-impacting case:

```
condition: "output.eligible = false and instance.impactLevel = 'rights-impacting'"
```

**Obligation** -- require agent to provide justification for every extracted value:

```
requirement: "every(output.extractedFields, $.sourceReference != null)"
```

(Formspec Core §3.5.1: the second argument is a FEL predicate expression; `$` is rebound to each element of `extractedFields`, so `$.sourceReference` reads the property on each record.)

**Cross-layer constraint** -- prohibition referencing kernel case state, Layer 1 temporal parameters, and Layer 2 agent output simultaneously:

```
condition: "output.eligible = true and caseFile.verification.totalIncome > parameters.eligibilityThreshold * 1.5"
```

This last example demonstrates the evaluation context enrichment model (Kernel S7.3): `output` is injected by Layer 2, `caseFile` is kernel base context, and `parameters` is injected by Layer 1 via temporal resolution.

### 4.9 Null Propagation

When a deontic constraint expression evaluates to `null` (due to missing data or unresolvable references), the behavior is determined by the kernel's `impactLevel`:

| Impact Level | Null Behavior | Rationale |
|---|---|---|
| `rights-impacting` | `escalateToHuman` | Unknown constraint state on a rights-affecting decision requires human review. |
| `safety-impacting` | `escalateToHuman` | Safety constraints that cannot be evaluated must not silently pass. |
| `operational` | `true` (pass) | Operational constraints failing to evaluate should not block workflow. |
| `informational` | `true` (pass) | Low stakes; passing is reasonable. |

A constraint MAY override the default with an explicit `nullBehavior` property.

---

## 5. Autonomy Levels

This section is normative.

### 5.1 Overview

Every agent action operates at a declared autonomy level. Autonomy is a property of the **action site** -- the point in the workflow where an agent is invoked -- not a property of the agent itself. The same agent MAY operate at different autonomy levels in different contexts.

### 5.2 Level Definitions

| Level | Semantics | Fallback Requirement |
|-------|-----------|---------------------|
| `autonomous` | Agent output committed without human review. REQUIRES deontic constraints. | MUST define fallback to human. |
| `supervisory` | Agent output provisionally committed. Human reviews within a `reviewWindow`. If the window expires without intervention, the output becomes final with a provenance record noting implicit confirmation. | MUST define fallback to human. |
| `assistive` | Agent produces a recommendation. Human reviews, may modify, and explicitly confirms. The human owns confirmed output. | MUST define fallback to human. |
| `manual` | Action performed entirely by a human. Agent MAY provide contextual assistance on demand. | N/A. |

### 5.3 Autonomy Constraints

1. An action with `autonomous` autonomy MUST have associated deontic constraints (S4). Autonomous actions without deontic constraints are a structural error.
2. An action with `assistive` autonomy MUST create a human task for confirmation.
3. An action with `supervisory` autonomy MUST define a `reviewWindow` (ISO 8601 duration).
4. The effective autonomy MUST NOT exceed the workflow's impact-level cap. For `rights-impacting` and `safety-impacting` workflows, the default cap is `assistive`.
5. The effective autonomy for any agent action is the **minimum** of: the AI Integration Document's `defaultAutonomy`, the Agent Config sidecar's `maxAutonomy` (if present), the per-action override's autonomy (if present), and the impact-level cap from constraint 4.
6. Every agent invocation MUST have a reachable path to workflow completion that does not require any agent to succeed.

### 5.4 Autonomy Escalation

Escalation (raising an agent's effective autonomy level) requires human approval:

1. The Agent Config sidecar's `autonomyPolicy` MUST define escalation conditions as FEL expressions evaluated against the `agent` context variable (e.g., `agent.calibration.accuracy >= 0.97 and agent.recentViolations('P30D') = 0`).
2. A human with the required role MUST review and approve the escalation.
3. The approval MUST have a defined expiration period (`escalationExpiry`, ISO 8601 duration). The agent reverts to its prior autonomy level when the period expires unless re-approved.
4. Escalation produces a provenance record of type `autonomyEscalation` including the approving actor, the escalation conditions that were met, and the expiry.
5. Escalation does NOT bypass deontic constraints (S4) or any other governance mechanism. An agent escalated to `autonomous` still has all its constraints evaluated.

### 5.5 Autonomy Demotion

Demotion (lowering an agent's effective autonomy level) MAY be automatic:

1. The Agent Config sidecar's `autonomyPolicy` MUST define demotion triggers. Standard triggers:
   - Calibration accuracy falls below threshold.
   - Guardrail violation rate exceeds threshold within evaluation window.
   - Model version change (pending recalibration).
   - Drift detection alert (S9).
2. When a demotion trigger fires, the demotion takes effect for the next invocation. In-flight sessions are not retroactively affected but are annotated with the demotion event in provenance.
3. When `pendingRecalibration` is true, the agent operates at the demoted level until recalibration meets the escalation conditions defined in S5.4.
4. Demotion produces a provenance record of type `autonomyDemotion` including the trigger that fired and the new effective level.

### 5.6 Dynamic Autonomy Selection

A workflow MAY compute the effective autonomy level from case data and agent state using a FEL expression evaluated pre-invocation. The expression is evaluated in the standard evaluation context (Kernel S7.2) enriched with the `agent` variable (operational state, calibration metrics). The computed level MUST NOT exceed the effective cap defined in constraint 5 of S5.3.

---

## 6. Formspec-as-Validator

This section is normative.

### 6.1 Principle

**Agent output is untrusted input validated against the same Formspec contract a human would submit against.**

This extends Layer 1's data validation pipelines (Governance S5) via the `contractHook` seam (Kernel S10.2). The Formspec Definition serves as a validation cage around agent output, eliminating the maintenance burden of custom validation while preserving structural verifiability.

### 6.2 Requirements

1. A Formspec Definition used as an agent output contract MUST apply the same validation rules, Bind expressions, and constraint Shapes as the human-facing form.
2. Validation failures on agent output MUST be recorded as provenance and trigger fallback (S8), not silent acceptance.
3. Agent-touched fields MUST be annotated in the Formspec instance with `agentProvenance` metadata: which agent, which model version, what confidence, what source material.
4. WOS processors MUST delegate contract evaluation to a Formspec-conformant processor (Core S1.4). Deontic constraint evaluation (S4) occurs outside the Formspec processing model -- the WOS Processor evaluates deontic constraints against the contract processor's output. The Formspec processing model (Core S2.4) is a closed system; WOS governance wraps it.

### 6.3 Agent Provenance Metadata

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `agentId` | string | REQUIRED | Which agent produced this value. |
| `modelVersion` | string | REQUIRED | Model version at time of production. |
| `confidence` | number (0-1) | OPTIONAL | Confidence for this specific field. |
| `sourceRef` | string | OPTIONAL | Reference to the source material used. |

---

## 7. Confidence Framework

This section is normative.

### 7.1 Confidence Report

Every agent output MUST be accompanied by a ConfidenceReport.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `overall` | number (0.0-1.0) | REQUIRED | Estimated probability that the output is correct. |
| `method` | enum | REQUIRED | How confidence was derived. |
| `explanation` | string | OPTIONAL | Human-readable explanation of confidence factors. |
| `fieldLevel` | object | OPTIONAL | Per-output-field confidence values, keyed by field name. |

The `overall` value represents the agent's estimated probability that its output would be accepted without modification by a competent human reviewer performing the same task.

### 7.2 Confidence Methods

| Method | Definition | Calibration Requirement |
|--------|-----------|------------------------|
| `modelNative` | Derived from the model's own probability estimates. | MUST be calibrated. |
| `calibrated` | Post-hoc calibration applied to model-native scores using historical accuracy data. | Calibration is inherent. |
| `heuristic` | Derived from structural properties of the output (consistency checks, output stability). | SHOULD be calibrated. |
| `conformal` | Conformal prediction sets with guaranteed coverage. | Calibration is inherent. |
| `declared` | Manually assigned by the agent developer. | MAY be calibrated. |

### 7.3 Per-Field Confidence

When per-field confidence is available, guard expressions and routing logic MAY reference individual fields. This enables targeted routing: a classification output with high confidence on document type but low confidence on an extracted amount routes only the low-confidence field to human review.

### 7.4 Confidence Floor

The confidence floor is a workflow-level minimum confidence threshold. When an agent's output confidence falls below the floor, the output is invalidated.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `threshold` | number (0.0-1.0) | REQUIRED | Minimum acceptable confidence. |
| `onViolation` | enum | REQUIRED | Action when confidence is below threshold: `reject` (discard output and trigger fallback) or `escalateToHuman` (route to human review with full context). |

The confidence floor participates in the enforcement ordering (S4.6) at step 4 -- after permissions, prohibitions, and obligations, but before volume constraints.

### 7.5 Confidence Decay

Agent outputs become less reliable as underlying case data changes. Confidence decay models this degradation.

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `enabled` | boolean | REQUIRED | Whether confidence decay is active. |
| `halfLife` | string (duration) | OPTIONAL | Duration after which effective confidence halves, absent triggering events. |
| `triggers` | array of DecayTrigger | OPTIONAL | Events causing immediate confidence reduction. |

A DecayTrigger binds an event to a multiplicative `decayFactor` (0-1). When the trigger fires, the effective confidence of all outputs produced by that agent for the current case is multiplied by the factor. When confidence falls below the confidence floor, the output is invalidated and the action is escalated.

### 7.6 Temporal Confidence Thresholds

Confidence thresholds used in autonomy decisions and routing SHOULD be modeled as temporal parameters (Governance S13, Policy Parameter Config sidecar) so they can be adjusted based on operational experience.

### 7.7 Cumulative Confidence

In multi-step agent interactions, errors compound. The cumulative confidence after step *n* is the product of individual step confidences by default. A four-step process where each step has 0.9 confidence yields cumulative confidence of approximately 0.66.

A conformant processor MUST track cumulative confidence. When cumulative confidence falls below the confidence floor, the interaction MUST pause for human review.

---

## 8. Fallback Chains

This section is normative.

### 8.1 Principle

Every workflow that uses agents MUST function correctly when agents are unavailable (Kernel S1.2). Agent unavailability is a regular operating condition, not an edge case.

### 8.2 Fallback Chain Execution

When an agent invocation fails (error, timeout, guardrail rejection, or unavailability), the fallback chain is executed:

1. Each level is attempted in order.
2. A successful level stops the chain.
3. A failed level advances to the next.
4. Every attempt MUST produce a provenance record.
5. The terminal level MUST produce a result or transition to a human task.

### 8.3 Fallback Options

| Option | Description | Key Properties |
|--------|-------------|----------------|
| `escalateToHuman` | Create a human task. | `taskRef` |
| `retry` | Retry with backoff. | `maxRetries`, `backoff`, `initialInterval` |
| `alternateAgent` | Try a different agent. | `alternateAgentRef` |
| `fail` | Fail the action. | -- |

### 8.4 Chain Constraints

A fallback chain MUST terminate in either `escalateToHuman` or `fail`. A chain MUST NOT cycle. A conformant processor MUST validate chains at document load time and reject chains that cycle or lack a terminal action.

---

## 9. Decision Drift Detection

This section is normative.

### 9.1 Overview

Structural guards against patterns where agent behavior silently drifts from its intended purpose.

### 9.2 Drift Indicators

| Indicator | Description | Detection Method |
|-----------|-------------|-----------------|
| Training data contamination | Agent was trained on human determination outcomes (approvals, denials) rather than task-specific ground truth. The agent optimizes for outcome prediction, not task accuracy. | Declaration: the agent configuration MUST disclose training data characteristics. |
| Optimization objective misalignment | Agent optimizes for "maximize agreement with reviewer decisions" rather than "maximize extraction accuracy against ground truth." | Declaration: the agent configuration MUST disclose its optimization objective. |
| Rubber-stamp detection | Human reviewers are ratifying agent output without genuine cognitive engagement at increasing rates. | Monitoring: track reviewer engagement metrics (time spent, modifications made, disagreement rate). |

### 9.3 Governance Implications

When training data contamination is detected, the agent's task MUST be reclassified from preparation to determination, and appropriate governance (deontic constraints, autonomy caps) MUST be applied.

---

## 10. AI-Specific Oversight Extensions

This section is normative.

### 10.1 Overview

Layer 2 extends Layer 1's review protocols (Governance S4) with AI-specific presentation and suppression controls. These attach via `lifecycleHook` on `review`-tagged transitions.

### 10.2 Independent-First Suppression

When the `independentFirst` protocol (Governance S4.1) is active on a transition where an agent has produced output, the agent's output MUST be hidden until the reviewer's independent assessment is recorded. The interface MUST enforce this ordering.

### 10.3 Presentation Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `showConfidence` | boolean | `false` | Display the agent's confidence score alongside the recommendation. |
| `showAlternatives` | boolean | `false` | Display alternative recommendations the agent considered. |
| `highlightLowConfidenceFields` | boolean | `false` | Visually highlight fields where agent confidence is below threshold. |
| `showDiffFromIndependent` | boolean | `false` | After independent assessment is recorded, highlight differences between reviewer and agent. Only meaningful with `independentFirst`. |

---

## 11. Volume Constraints and Review Sampling

This section is normative.

### 11.1 Volume Constraints

Rate limits on autonomous agent actions prevent runaway automation:

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `maxAutonomousPerHour` | integer | OPTIONAL | Maximum autonomous actions per hour. |
| `maxAutonomousPerDay` | integer | OPTIONAL | Maximum autonomous actions per day. |

When volume limits are reached, subsequent agent invocations are escalated to human review regardless of confidence.

### 11.2 Agent-Specific Review Sampling

Extends Layer 1's review sampling (Governance S7.1) with agent-specific methods:

| Method | Description |
|--------|-------------|
| `random` | Standard random sampling (same as Layer 1). |
| `stratified` | Stratified by agent type, confidence level, or case characteristics. |
| `adversarial` | Targeted sampling of cases likely to reveal agent weaknesses. |

Actions selected for sampling are routed to human review regardless of the autonomy level. The review outcome feeds the calibration pipeline.

---

## 12. Agent Disclosure

This section is normative.

### 12.1 Overview

Layer 2 extends Layer 1's due process notice requirements (Governance S3) with agent disclosure. This attaches via `lifecycleHook` on `adverse-decision`-tagged transitions.

### 12.2 Disclosure Requirements

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `discloseThatAgentAssisted` | boolean | REQUIRED | Disclose that an AI system assisted in the determination. |
| `discloseModelIdentity` | boolean | OPTIONAL | Disclose the model identifier. Default: `false`. |
| `discloseConfidence` | boolean | OPTIONAL | Disclose the agent's confidence score. Default: `false`. |

For `rights-impacting` workflows, `discloseThatAgentAssisted` MUST be `true`. This requirement is consistent with OMB M-24-10 and EU AI Act Art. 13.

---

## 13. Narrative Provenance Tier

This section is normative.

### 13.1 Overview

Layer 2 adds the **Narrative tier** to the kernel's provenance model via the `provenanceLayer` seam (Kernel S10.3). This is the only AI-specific provenance tier.

### 13.2 Epistemic Status

The Narrative tier is **NON-AUTHORITATIVE**. Every provenance record that includes Narrative tier content MUST label it non-authoritative. This requirement exists because model-generated explanations are systematically unfaithful to actual reasoning processes (Turpin et al., NeurIPS 2023; Lanham et al., Anthropic, 2023).

A conformant implementation MUST NOT treat Narrative tier content as dispositive evidence in any adjudicative, audit, or due process context.

### 13.3 Narrative Tier Record

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `narrative` | string | REQUIRED | The model's natural language explanation. |
| `authoritative` | boolean | REQUIRED | MUST be `false`. Explicitly marks this content as non-authoritative. |
| `modelIdentifier` | string | REQUIRED | Model that generated this narrative. |
| `modelVersion` | string | REQUIRED | Model version. |

### 13.4 Relationship to Layer 1 Tiers

Layer 1 provides the Reasoning tier (rules applied, evidence consulted) and Counterfactual tier (what would change the outcome). Layer 2 extends these existing tiers with agent-specific metadata (model version, confidence scores, input summaries) but does not add new tiers beyond the Narrative tier. The Reasoning and Counterfactual tiers remain authoritative; the Narrative tier is informational only.

---

## 14. Assist Governance Proxy

This section is normative.

### 14.1 Overview

The WOS Assist Governance Proxy is a **WOS-defined governance construct** that consumes the Formspec Assist protocol (Assist S2.1). It sits between the Assist Consumer and the Assist Provider (Assist S2.1), intercepting tool invocations and applying the WOS deontic constraint framework.

The proxy is NOT a Formspec concept. It is a WOS construct that wraps Assist protocol interactions with governance.

### 14.2 Requirements

1. The proxy MUST NOT modify either role's Assist conformance requirements.
2. The proxy MUST apply deontic constraints (S4) to tool invocation results.
3. The proxy MUST produce provenance records for each governed tool invocation.
4. Per-tool-category governance is supported: different tool categories may have different constraint sets.
5. Transport bindings follow Assist S7 (Transport Bindings).

---

## 15. AI Governance in the Workflow Envelope

AI governance is declared as embedded blocks (`agents`, `aiOversight`) within a `$wosWorkflow` document. The same `url` that a Workflow Governance document targets via `targetWorkflow` is the `url` of the `$wosWorkflow` envelope.

```json
{
  "$wosWorkflow": "1.0",
  "url": "https://agency.gov/workflows/benefits-adjudication",
  "...": "..."
}
```

When a `$wosWorkflow` document carries both `governance` and `agents`/`aiOversight` blocks, AI governance extends the governance structures within the same envelope. Each embedded block is independently optional; adding `agents` does not require `governance` to be present, and vice versa.

---

## References

### Normative References

- [WOS Kernel] Formspec Working Group, "WOS Kernel Specification v1.0".
- [WOS Workflow Governance] Formspec Working Group, "WOS Workflow Governance Specification v1.0".
- [Formspec Core] Formspec Working Group, "Formspec Core Specification v1.0".
- [Assist] Formspec Working Group, "Formspec Assist Specification v1.0".

### Informative References

- Vaccaro, M. et al., "When combinations of humans and AI are useful: A systematic review and meta-analysis", Nature Human Behaviour, 2024.
- Turpin, M. et al., "Language Models Don't Always Say What They Think", NeurIPS 2023.
- Lanham, T. et al., "Measuring Faithfulness in Chain-of-Thought Reasoning", Anthropic, 2023.
- Chen, L., Zaharia, M., and Zou, J., "How is ChatGPT's behavior changing over time?", 2023.
- OMB Memorandum M-24-10, "Advancing Governance, Innovation, and Risk Management for Agency Use of Artificial Intelligence", March 2024.
- EU AI Act, Regulation (EU) 2024/1689.
- OASIS LegalRuleML TC, "LegalRuleML Core Specification v1.0", 2021.
