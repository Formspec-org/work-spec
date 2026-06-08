# WOS Feature & Requirements Matrix

**Last updated:** 2026-05-09
**Spec version:** 1.0.0-draft.1
**Purpose:** Non-technical reference for evaluating WOS capabilities against competing platforms.

---

## How to Read This Document

**WOS Capability Status:**

| Icon | Meaning |
|------|---------|
| ✅ | Fully Specified and Implemented in Reference Stack |
| 🟦 | Specified and partially implemented |
| 🟡 | Specified in prose and schema; implementation pending |
| ⚪ | Referenced or planned; specification pending |

**Current implementation caveat:** `wos-runtime` owns reference companion policy, event identity, and runtime/core provenance emission; `wos-conformance` observes those behaviors. `wos-formspec-binding` implements the full S15 protocol with binding-backed conformance fixtures; `ConformanceBinding` has been deleted (S15.3 complete); `StubValidator` is retained for service-invocation contract validation.

**Competitor Support:**

| Icon | Meaning |
|------|---------|
| ✔ | Full native support |
| ~ | Partial or limited support |
| ✘ | Not supported |
| -- | Not applicable |

**Competitors Evaluated (16):**

| Abbrev | Platform | Type |
|--------|----------|------|
| SNow | ServiceNow | Commercial. 100+ US federal agencies. |
| Pega | Pegasystems | Commercial. CMS, SSA, IRS, USCIS. |
| Appn | Appian | Commercial. FedRAMP. DoD + civilian. |
| SFGov | Salesforce Government Cloud | Commercial. Agentforce AI. FedRAMP. |
| Cam | Camunda 8 | Open source (Apache 2.0 / SSPL). Zeebe. |
| KIE | Apache KIE | Open source (Apache 2.0). BPMN+DMN+Drools+OptaPlanner. |
| Flow | Flowable | Open source (Apache 2.0). BPMN+CMMN+DMN. |
| Temp | Temporal | Open source (MIT). Deterministic replay. |
| Palnt | Palantir AIP | Commercial. DoD, IC, HHS, VA. |
| MSPow | Microsoft Power Platform | Commercial. Power Automate + Copilot Studio. FedRAMP. |
| LGrph | LangGraph / LangChain | Open source. AI agent orchestration. |
| StepFn | AWS Step Functions | Commercial. Amazon States Language. GovCloud. |
| Tyler | Tyler Technologies | Commercial. Courts, finance, permitting, public safety. |
| Bonit | Bonita | Open source (GPL v2). European government. |
| PMkr | ProcessMaker | Open source (AGPL v3). LATAM government. |

Full 16-competitor ratings reside in the companion spreadsheet (`wos-formspec-competitive-feature-matrix.xlsx`). Technical implementation details are tracked in `WOS-IMPLEMENTATION-STATUS.md`.

---

## 1. Process Orchestration

| # | Requirement | Description | WOS | SNow | Pega | Cam | KIE | Flow | Temp | Palnt | LGrph |
|---|------------|-------------|-----|------|------|-----|-----|------|------|-------|-------|
| 1.1 | **Sequential/parallel/choice composition** | Defines workflows as sequences, branches, and parallel tracks | ✅ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ~ |
| 1.2 | **Hierarchical states (nested, compound)** | Nests sub-states within phases (e.g., internal steps for a "Review" phase) | ✅ | ~ | ~ | ~ | ✔ | ~ | ~ | ✘ | ✘ |
| 1.3 | **History states** | Resumes previously exited phases at their exact exit point | ✅ | ~ | ~ | ✘ | ~ | ~ | ✔ | ✘ | ✘ |
| 1.4 | **Parallel regions (orthogonal)** | Progresses concurrent, independent state aspects simultaneously | ✅ | ~ | ~ | ✔ | ✔ | ✔ | ~ | ✘ | ✘ |
| 1.5 | **Parallel completion policies** | Enforces wait-all, cancel-siblings, or fail-fast policies upon track completion | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 1.6 | **Declarative constraint zones (DCR-style)** | Manages adaptive cases via condition/response/include/exclude relations | 🟡 ^8 | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 1.7 | **CMMN case management** | Executes discretionary items, sentries, and ad-hoc work within stages | 🟡 ^9 | ~ | ✔ | ✘ | ✘ | ✔ | ✘ | ~ | ✘ |
| 1.8 | **Milestones (data-driven checkpoints)** | Fires named conditions when data reaches thresholds, independent of workflow state | ✅ | ~ | ✔ | ✘ | ~ | ~ | ✘ | ~ | ✘ |
| 1.9 | **Cancellation regions** | Cancels activity sets upon specific completion or failure events | ✅ | ~ | ~ | ✔ | ✔ | ✔ | ~ | ✘ | ✘ |
| 1.10 | **Process definition as declarative data** | Stores workflow as a JSON document rather than imperative code | ✅ | ✘ | ~ | ✔ | ✔ | ✔ | ✘ | ✘ | ✘ |
| 1.11 | **Shared activation criteria** | Reusable "when does this become active?" shape (event/transition trigger + actor + required-data + FEL predicate + deadline window) referenced across milestones, SLAs, holds, DCR, and agent preconditions | ⚪ ^12 | ✘ | ~ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 1.12 | **Durable pending obligations** | Tracks "after X, Y must happen by deadline by role R, else action A" as a governed lifecycle (pending → satisfied/violated/cancelled/expired) with provenance | ⚪ ^12 | ~ | ~ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 1.13 | **Cross-event obligation policies** | Author-time obligation policies whose activate/satisfy/cancel/violate conditions span events, actors, and tasks | ⚪ ^12 | ~ | ~ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 1.14 | **Policy-engine obligation materialization** | Materializes external policy-engine (OPA/Cedar) decision obligations into WOS pending obligations when configured | ⚪ ^12 | ✘ | ~ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |

---

## 2. Lifecycle & Durable Execution

| # | Requirement | Description | WOS | SNow | Pega | Cam | Temp | StepFn | KIE | Flow |
|---|------------|-------------|-----|------|------|-----|------|--------|-----|------|
| 2.1 | **Crash recovery** | Resumes running workflows from the last saved point after system failure | ✅ | ✔ | ✔ | ✔ | ✔ | ✔ | ~ | ~ |
| 2.2 | **Deterministic replay** | Produces identical outcomes during replay by caching external service results | ✅ | ✘ | ✘ | ✘ | ✔ | ✘ | ✘ | ✘ |
| 2.3 | **Saga/compensation transactions** | Reverses previously completed steps upon task failure | ✅ | ~ | ~ | ✔ | ✔ | ✔ | ~ | ~ |
| 2.4 | **Timer management** | Guarantees absolute, relative, and recurring timers with precision tolerances | ✅ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |
| 2.5 | **Business-calendar-aware deadlines** | Computes SLA deadlines in business days, excluding holidays and non-working hours | ✅ | ✔ | ✔ | ~ | ✘ | ✘ | ~ | ~ |
| 2.6 | **Statutory deadline chains** | Automates legal consequences when interdependent government deadlines pass | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 2.7 | **Instance migration across versions** | Validates state and maps fields when moving cases to new workflow versions | ✅ | ~ | ~ | ✔ | ✔ | ✘ | ~ | ~ |
| 2.8 | **Idempotent execution** | Prevents duplicate service calls using deduplication keys | ✅ | ~ | ~ | ✔ | ✔ | ✔ | ✘ | ✘ |
| 2.9 | **Schema upgrade as named lifecycle operation** | Records explicit migrations with version provenance and migration mechanism; preserves historical verifiability | ✅ | ~ | ~ | ~ | ~ | ✘ | ~ | ~ |

---

## 3. Decision & Rules

| # | Requirement | Description | WOS | Pega | Cam | KIE | Flow | Temp | Palnt |
|---|------------|-------------|-----|------|-----|-----|------|------|-------|
| 3.1 | **Decision tables (DMN)** | Executes tabular decision logic with hit policies | ✅ ^1 | ✔ | ✔ | ✔ | ✔ | ✘ | ~ |
| 3.2 | **Decision requirement graphs** | Visualizes composition of dependent decisions | 🟡 ^1 | ✔ | ✔ | ✔ | ~ | ✘ | ✘ |
| 3.3 | **FEEL expression language** | Supports the OMG standard expression language for decisions | ✘ | ~ | ✔ | ✔ | ✔ | ✘ | ✘ |
| 3.4 | **FEL expression language** | Employs deterministic, side-effect-free expressions with DAG dependency tracking | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 3.5 | **Business rules engine (DRL/Rete)** | Evaluates forward-chaining production rules | ✘ | ✔ | ✘ | ✔ | ✘ | ✘ | ✘ |
| 3.6 | **Defeasible rules** | Applies general rules with priority-based exceptions | 🟡 ^2 | ~ | ✘ | ✘ | ✘ | ✘ | ~ |
| 3.7 | **Temporal parameters (date-effective values)** | Applies policy values indexed by effective date based on filing timestamps | ✅ | ✔ | ✘ | ~ | ✘ | ✘ | ✘ |
| 3.8 | **Regulatory version bindings** | Binds documents to specific versions by date; preserves original versions for old cases | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 3.9 | **Policy change propagation** ^7 | Migrates, reviews, or grandfathers in-flight cases when regulations change | 🟦 | ~ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 3.10 | **Constraint solver / optimization** | Allocates resources using constraint solving (e.g., OptaPlanner) | ✘ | ✘ | ✘ | ✔ | ✘ | ✘ | ✘ |

> ^1 **Decision tables / DRGs:** WOS embeds a FEL-native decision table construct (not OMG DMN/FEEL) with full schema, typed Rust model, runtime evaluator, and lint validation. Decision requirement graphs are achievable through table composition but have no explicit DRG construct.
>
> ^2 **Defeasible rules:** `sourceAuthority` ranking (statute > regulation > policy > guideline) orders explanations. Runtime priority-based override (Catala-style default logic) is pending — see TODO #24b/#25. The `defeasible` schema field is declarable with no runtime behavioral effect yet.
>
> ^7 **Policy change propagation:** Migration is specified (Runtime §11, Governance §2.9) via a manual `migrate` operation with implicit grandfather semantics. A declarative `migrationPolicy` enum with `grandfather | migrateAll | migrateByState | expression` options is **not** specified; operators compose the three modes ad-hoc. See `IDEA_SCRATCH.md` #3 for the proposed declarative enum.

---

## 4. Human Task Management

| # | Requirement | Description | WOS | SNow | Pega | Cam | KIE | Flow | Temp | MSPow |
|---|------------|-------------|-----|------|------|-----|-----|------|------|-------|
| 4.1 | **WS-HumanTask lifecycle** | Implements the 8-state lifecycle: created, assigned, claimed, completed, failed, delegated, escalated, skipped | ✅ | ✔ | ✔ | ✔ | ✔ | ✔ | ✘ | ✔ |
| 4.2 | **Role-based assignment** | Declares owners, nominees, potential owner pools, administrators, and excluded actors | ✅ | ✔ | ✔ | ✔ | ✔ | ✔ | ✘ | ✔ |
| 4.3 | **Capability-based routing** | Routes tasks based on actor skills or capabilities rather than roles | ⚪ ^3 | ~ | ✔ | ✘ | ~ | ~ | ✘ | ~ |
| 4.4 | **Separation of duties (four-eyes)** | Ensures decision-maker and reviewer are different people | ✅ | ~ | ✔ | ✘ | ~ | ~ | ✘ | ✘ |
| 4.5 | **Delegation with accountability chain** | Tracks formal authority chains with legal instrument references and expiration dates | ✅ | ✔ | ✔ | ✘ | ~ | ~ | ✘ | ~ |
| 4.6 | **Escalation (time + condition based)** | Escalates tasks automatically when deadlines pass or conditions are met | ✅ | ✔ | ✔ | ~ | ✔ | ✔ | ✘ | ✔ |
| 4.7 | **SLA tracking with deadline actions** | Executes auto-actions (escalate, reassign, notify, extend) on SLA breach | 🟦 | ✔ | ✔ | ~ | ~ | ~ | ~ | ✔ |
| 4.8 | **Override with structured rationale** | Requires mandatory rationale, authority verification, and immutable audit for decision overrides | ✅ | ~ | ~ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 4.9 | **Quorum-based delegation (N-of-M authorization)** | Requires authorization from N of M distinct authorities for high-stakes operations | ✅ | ~ | ~ | ✘ | ✘ | ✘ | ✘ | ~ |

> ^3 **Capability-based routing:** WOS defines assignment roles; capability-specific routing remains implementation-defined.

---

## 5. AI Agent Governance

| # | Requirement | Description | WOS | Palnt | LGrph | SNow | Pega | Cam |
|---|------------|-------------|-----|-------|-------|------|------|-----|
| 5.1 | **Agent registration with type taxonomy** | Registers agents as deterministic, statistical, or generative with model ID and version | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.2 | **Agent as untrusted actor** | Enforces all system constraints by treating agent outputs as untrusted input | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.3 | **Deontic constraints (POPR)** | Governs agent behavior using the Permission/Prohibition/Obligation/Right framework | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.4 | **SMT-verifiable governance constraints** | Proves constraints hold for all inputs before deployment using SMT | 🟡 ^10 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.5 | **Four autonomy levels** | Applies autonomous, supervisory, assistive, or manual levels per action | ✅ | ~ | ✘ | ✘ | ~ | ✘ |
| 5.6 | **Impact-level caps on autonomy** | Caps agent autonomy based on consequence level (rights, safety, operational, info) | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.7 | **Dynamic autonomy (escalation/demotion)** | Adjusts autonomy levels based on human approval or performance triggers | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.8 | **Confidence framework with decay** | Flags decaying confidence over time or as underlying data changes | ✅ | ~ | ~ | ✘ | ~ | ✘ |
| 5.9 | **Cumulative confidence tracking** | Pauses sessions for human review when compounding multi-step error exceeds floors | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.10 | **Mandatory fallback chains** | Validates at load time that workflows function when agents are unavailable | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.11 | **Kill-switch / circuit-breaker** | Triggers automatic fallback when agent error rates exceed thresholds | ✅ | ~ | ✘ | ~ | ~ | ✘ |
| 5.12 | **Drift monitoring with auto demotion** | Reduces agent autonomy automatically upon detecting statistical drift | ✅ | ~ | ✘ | ✘ | ~ | ✘ |
| 5.13 | **Volume rate limits** | Prevents runaway automation by capping autonomous actions per hour or day | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.14 | **Agent lifecycle state machine** | Governs agent states (active, degraded, suspended, retired) via performance transitions | ✅ | ~ | ✘ | ✘ | ✘ | ✘ |
| 5.15 | **Model version pinning and policy** | Pins versions or uses approved lists; emits provenance for version changes | ✅ | ~ | ✘ | ✘ | ~ | ✘ |
| 5.16 | **Tool use governance** | Restricts tool invocation, rate limits, and data mutation for agents | ✅ | ~ | ~ | ✘ | ✘ | ✘ |
| 5.17 | **Formspec-as-Validator** | Validates agent output against the same form contract as human data | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.18 | **Agent disclosure** | Discloses AI participation to affected individuals; mandatory for rights-impacting cases | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.19 | **Assist Governance Proxy** | Wraps assistant tools with deontic constraints and provenance | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.20 | **CaMeL-compatible trust boundary** | Employs a trust boundary model compatible with dual-LLM security architectures | 🟡 ^4 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 5.21 | **Equity guardrails (bias monitoring)** | Monitors fairness statistically for human and AI decisions by demographic group | 🟦 | ~ | ✘ | ✘ | ~ | ✘ |
| 5.22 | **Shadow / canary deployment** | Executes model versions in shadow then canary modes before production | 🟡 ^11 | ✘ | ✘ | ✘ | ✘ | ✘ |


---

## 6. Structured Human Oversight

| # | Requirement | Description | WOS | Any Competitor? |
|---|------------|-------------|-----|-----------------|
| 6.1 | **Independent-first protocol** | Suppresses recommendations until the reviewer records an independent assessment | ✅ | ✘ |
| 6.2 | **Consider-opposite protocol** | Requires reviewers to articulate counter-arguments before confirming recommendations | ✅ | ✘ |
| 6.3 | **Calibrated confidence display** | Highlights low-confidence items and displays scores alongside recommendations | ✅ | Palnt ~ |
| 6.4 | **Dual-blind review** | Reconciles results from two independent reviewers who cannot see each other's work | ✅ | Pega ~ |
| 6.5 | **Rejection-rate monitoring** | Tracks reviewer agreement and modification rates as quality signals | 🟡 | Pega ~ |
| 6.6 | **Dynamic sampling (risk-based review allocation)** | Selects a configurable percentage of decisions for stratified quality review | ✅ | SNow ~ |
| 6.7 | **Rubber-stamp detection** | Detects blind acceptance of recommendations by monitoring review times and agreement patterns | 🟦 | ✘ |

---

## 7. Due Process & Legal Compliance

| # | Requirement | Description | WOS | SNow | Pega | Palnt |
|---|------------|-------------|-----|------|------|-------|
| 7.1 | **Impact level classification** | Workflow declares impact: rights-impacting, safety, operational, or informational | ✅ | ✘ | ✘ | ✘ |
| 7.2 | **Mandatory notice before adverse decisions** | Issues notice containing reasons, appeal rights, and deadlines before decisions take effect | ✅ | ✘ | ✘ | ✘ |
| 7.3 | **Individualized explanation** | Adverse decisions must include reasons specific to the individual case | ✅ | ✘ | ~ | ✘ |
| 7.4 | **Counterfactual explanation** | Details both positive factors (what could change) and negative factors (what was irrelevant) | ✅ | ✘ | ✘ | ✘ |
| 7.5 | **Deterministic adverse-decision notice (dual-form)** ^6 | Specified deterministic algorithm (not model-generated) that derives two co-synchronized outputs from Facts + Reasoning provenance: a machine-readable artifact and a human-prose artifact. Identical inputs MUST produce identical outputs in both forms. | ✅ | ✘ | ✘ | ✘ |
| 7.6 | **Appeal with independent adjudicator** | Ensures appeals are reviewed by someone independent of the original decision-maker | ✅ | ✘ | ~ | ✘ |
| 7.7 | **Continuation of service during appeal** | Freezes adverse impacts and maintains services during the appeal period | ✅ | ✘ | ✘ | ✘ |
| 7.8 | **Agent disclosure requirement** | Mandates disclosure of AI participation for all rights-impacting workflows | ✅ | ✘ | ✘ | ✘ |
| 7.9 | **Respondent ledger** | Tracks delivery, receipt confirmation, and appeal deadlines per individual | 🟡 | ✘ | ✘ | ✘ |
| 7.10 | **Typed hold policies** | Enforces suspension reasons, expected durations, resume triggers, and timeout actions | ✅ | ✘ | ✘ | ✘ |
| 7.11 | **Tag-based governance attachment** | Attaches governance rules to semantic categories (e.g., "all determination steps") | ✅ | ✘ | ✘ | ✘ |
| 7.12 | **Scoped governance rules** | Applies rules only when FEL conditions are met (e.g., "claims over $10,000") | ✅ | ✘ | ✘ | ✘ |
| 7.13 | **EU AI Act high-risk alignment** | Aligning specification with EU AI Act Art. 13-14 requirements | 🟡 | ~ | ~ | ✘ |
| 7.14 | **OMB M-24-10 compliance support** | Consistent with federal AI guidance on agent disclosure and governance structures | 🟡 | ✘ | ✘ | ~ |
| 7.15 | **Legal hold (distinct from workflow hold)** | Statutory-override hold that survives terminal state and blocks destruction with explicit release semantics | ✅ | ~ | ~ | ✘ |

> ^6 **Deterministic adverse-decision notice:** Specified at Governance §3.2 with deterministic assembly contract. Implemented in `wos-runtime` companion with `noticeSent` provenance record (machine-readable artifact + human-readable prose). Conformance fixture G-002.

---

## 8. Provenance & Audit

| # | Requirement | Description | WOS | SNow | Pega | Cam | Temp | Palnt |
|---|------------|-------------|-----|------|------|-----|------|-------|
| 8.1 | **Facts tier (immutable action records)** | Records every state transition and action: who, what, when, inputs, and outputs | ✅ | ✔ | ✔ | ✔ | ✔ | ✔ |
| 8.2 | **Reasoning tier** | Records which rules, evidence, and criteria drove every determination | 🟡 ^13 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 8.3 | **Counterfactual tier** | Records what would have changed the outcome and what did not affect it | 🟡 ^12 | ✘ | ✘ | ✘ | ✘ | ✘ |
| 8.4 | **Narrative tier (non-authoritative)** | Records AI-generated explanations separately and marks them non-authoritative | ✅ | ✘ | ✘ | ✘ | ✘ | ~ |
| 8.5 | **Epistemic status tagging** | Tags assertions as verified facts, system records, agent-generated, or human judgment | ✅ | ✘ | ✘ | ✘ | ✘ | ~ |
| 8.6 | **Authority-ranked explanations** | Ranks reasoning rules by authority: statute > regulation > policy > guideline | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 8.7 | **Tamper detection** | Verifies input/output integrity using cryptographic digests | ✅ ^5 | ✘ | ✘ | ✘ | ✔ | ~ |
| 8.8 | **W3C PROV-O export** | Exports provenance records to the standard W3C PROV Ontology format | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 8.9 | **OCEL 2.0 event logging** | Records events in the object-centric event log format for process mining | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 8.10 | **XES process mining export** | Exports provenance to IEEE 1849 XES format for process mining tools | ✅ | ~ | ~ | ✔ | ✘ | ~ |
| 8.11 | **Custody seam (custodyHook)** | Named extension seam for custody posture declaration; enables both trust-the-host monolith and distributed-trust bindings | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 8.12 | **Invariant 6 (disclosure ≠ assurance)** | Structurally prevents conflation of identity-revelation level with identity-binding strength | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ |

> ^5 **Tamper detection:** The reference implementation provides per-record digests. Full Merkle tree hash-chaining is a future roadmap item.
>
> ^8 **DCR constraint zones:** Spec and schema are complete (Advanced §4: activities, five relation types, evaluation algorithm). Runtime DCR evaluator is not yet implemented in the reference Rust stack.
>
> ^9 **CMMN case management:** WOS achieves adaptive case management through DCR constraint zones and statecharts rather than CMMN execution semantics. CMMN-specific constructs (discretionary items, sentries) are not directly specified or implemented.
>
> ^10 **SMT-verifiable constraints:** Advanced §8 specifies the verifiable constraint subset (decidable fragment, verification interface, relationship to runtime enforcement). No SMT solver integration or conformance test yet exists in the reference stack; the spec defines the interface for external SMT verification.
>
> ^11 **Shadow/canary deployment:** Shadow mode has schema support in the Advanced block. Canary phase deployment sequencing is described in the drift-monitor spec prose but not yet formalized as a schema $def.

> ^12 **Activation criteria & durable obligations (1.11–1.14):** Introduced by [ADR 0096](thoughts/adr/0096-shared-activation-criteria-and-durable-obligations.md); currently framing/ADR only — schema, runtime monitor, and conformance are planned (see WOS-IMPLEMENTATION-STATUS.md §5). These are **distinct** from DCR constraint zones (1.6, zone-local flexible-activity sequencing), milestones (1.8, data-driven checkpoints), task SLAs (§2.4–2.5), the deontic `Obligation` (§5, immediate pre-commit agent-output check), and the policy-engine bridge (§12, which is implemented; 1.14 adds optional materialization of its decision obligations into durable WOS obligations). FEL gains no temporal operators — it stays the local boolean predicate inside `where`. Competitor marks are preliminary; ServiceNow/Pega carry SLA/escalation engines that partially approximate durable duties (`~`), but none expose a reusable cross-event obligation primitive.
>
> ^12 **Counterfactual tier:** Specified in the type system and schema (`AuditLayer::Counterfactual`). Runtime emission is pending Layer 1 injection wiring; no `ProvenanceKind` variant or conformance test exercises counterfactual content. Counterfactual *requirements* for adverse decisions are enforced via Governance §3.4.
>
> ^13 **Reasoning tier:** Structural support exists (`AuditLayer::Reasoning`, config, lint G-014, `explain.rs` consumer). `ProvenanceAuditTier` only has Facts + Narrative variants; no runtime path stamps `audit_layer = "reasoning"`. Reasoning *content* (deontic evaluations, policy decisions) is recorded in Facts-tier records. Explicit tier label reserved per `audit_tier.rs` comment.

---

## 9. Data Collection & Interface Contracts

| # | Requirement | Description | WOS | SNow | Pega | Appn | MSPow |
|---|------------|-------------|-----|------|------|------|-------|
| 9.1 | **Universal interface contract model** | Employs one contract spec for human forms, agent I/O, decision services, and integrations | ✅ | ✘ | ✘ | ✘ | ✘ |
| 9.2 | **Headless contract pattern** | Uses Formspec Definitions as pure typed data contracts with no presentation layer | ✅ | ✘ | ✘ | ✘ | ✘ |
| 9.3 | **Reactive form behavior** | Supports computed fields, conditional visibility, and cross-field dependencies | ✅ | ~ | ✔ | ✔ | ✔ |
| 9.4 | **Cross-field validation (Shapes)** | Validates rules spanning multiple fields | ✅ | ~ | ✔ | ~ | ~ |
| 9.5 | **Structured validation results** | Provides severity, field path, message, and constraint kind for every error | ✅ | ~ | ~ | ~ | ✘ |
| 9.6 | **Mapping DSL for bidirectional data flow** | Transforms data between case files and external formats via versioned, auditable mappings | ✅ | ✘ | ✘ | ✘ | ✘ |
| 9.7 | **Ontology-driven semantic field identity** | Identifies fields by semantic meaning rather than name | 🟡 | ✘ | ✘ | ✘ | ✘ |
| 9.8 | **Version-pinned responses** | Binds contracts immutably at submission time | ✅ | ✘ | ~ | ✘ | ✘ |
| 9.9 | **Changelog with impact classification** | Provides structured change records with migration guidance and severity levels | ✅ | ✘ | ✘ | ✘ | ✘ |

---

## 10. Data Validation Pipelines

| # | Requirement | Description | WOS | Any Competitor? |
|---|------------|-------------|-----|-----------------|
| 10.1 | **Four pipeline stage types** | Sequences contract validation, assertion gates, data transformation, and human review | ✅ | ✘ |
| 10.2 | **Seven assertion gate types** | Applies source-grounded, arithmetic, range, consistency, format, cross-document, and temporal gates | ✅ | ✘ |
| 10.3 | **Reusable assertion libraries** | Shares named assertions across pipelines and governance documents | ✅ | ✘ |
| 10.4 | **Pipeline risk profile** | Determines overall reliability by the weakest gate rather than the strongest stage | ✅ | ✘ |
| 10.5 | **Four rejection policies** | Policies: retry with corrections, escalate, hold pending data, or fail with explanation | ✅ | ✘ |
| 10.6 | **Rejection provenance** | Records the specific gate failure, input, threshold, and the value required to pass | ✅ | ✘ |

---

## 11. Case State & Evidence

| # | Requirement | Description | WOS | SNow | Pega | Cam | Palnt |
|---|------------|-------------|-----|------|------|-----|-------|
| 11.1 | **Typed case file** | Structured data container with named, typed fields | ✅ | ✔ | ✔ | ✘ | ✔ |
| 11.2 | **Immutable mutation history** | Records every change: who, what, from, to, when, and in what state | ✅ | ~ | ~ | ✘ | ~ |
| 11.3 | **Case relationships** | Typed links (parent/child, sibling, related, supersedes) with full provenance | ✅ | ✘ | ✘ | ✘ | ✘ |
| 11.4 | **Cross-case isolation** | Related cases react to status changes but cannot access each other's data | ✅ | ✘ | ✘ | ✘ | ✘ |
| 11.5 | **Claim check pattern** | References evidence documents via content hash and URI (not stored in case state) | ⚪ | ✘ | ✘ | ✘ | ~ |
| 11.6 | **Role-based field-level visibility** | Enforces access control per field based on actor roles | ⚪ | ✔ | ✔ | ✘ | ✔ |

---

## 12. Integration & Eventing

| # | Requirement | Description | WOS | SNow | Pega | Cam | KIE | Temp | StepFn | LGrph |
|---|------------|-------------|-----|------|------|-----|-----|------|--------|-------|
| 12.1 | **HTTP/REST (OpenAPI)** | Invokes services synchronously via OpenAPI bindings | ✅ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ | ✔ |
| 12.2 | **CloudEvents 1.0 native** | Uses standard event envelopes with extensions for routing and causal chains | ✅ | ~ | ~ | ✔ | ✔ | ✔ | ✔ | ✘ |
| 12.3 | **Multi-step API orchestration (Arazzo)** | Orchestrates multi-step API sequences via Arazzo document references | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 12.4 | **Non-HTTP tool invocation** | Invokes CLI, batch, database, or graph query tools | ✅ | ✘ | ✘ | ✘ | ✘ | ✔ | ✔ | ✔ |
| 12.5 | **Policy engine bridge** | Integrates directly with XACML, OPA, and Cedar authorization engines | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 12.6 | **Form-level agent interaction (Assist)** | Governs AI assistant tool invocations at the form interaction level | ✅ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| 12.7 | **SCXML interoperability mapping** | Translates between WOS and W3C SCXML documents bidirectionally | 🟡 | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |

---

## 13. AI-Ready Authoring

| # | Requirement | Description | WOS | Any Competitor? |
|---|------------|-------------|-----|-----------------|
| 13.1 | **Compositional authoring** | Reuses components by URI; composes documents without copy-pasting | ✅ | Cam ~ |
| 13.2 | **Workflow definition as data** | Uses declarative JSON rather than imperative code—ideal for AI generation | ✅ | Cam ✔, KIE ✔, Flow ✔ |
| 13.3 | **Open specification** | Prevents vendor lock-in; any compliant engine can implement the standard | ✅ | ✔ | ✔ | ✔ | ~ |

---

## 14. Identity & Assurance

| # | Requirement | Description | WOS | SNow | Pega | Palnt |
|---|------------|-------------|-----|------|------|-------|
| 14.1 | **Assurance-level taxonomy** | Four-level ordered declaration of identity-binding strength (L1–L4), independent of disclosure posture | ✅ | ✘ | ✘ | ~ |
| 14.2 | **Subject continuity primitive** | Links related activity across time without requiring full legal-identity disclosure | ✅ | ✘ | ✘ | ✘ |
| 14.3 | **Provider-neutral attestation representation** | Attestation meaning representable independently of identity-provider bindings | ✅ | ✘ | ✘ | ✘ |
| 14.4 | **Assurance-upgrade facts** | Forward-only, non-rewriting facts for strengthening identity bindings over time | ✅ | ✘ | ✘ | ✘ |
| 14.5 | **Legal-sufficiency disclaimer (normative)** | Implementations MUST NOT imply cryptographic controls alone guarantee legal admissibility | ✅ | ✘ | ✘ | ✘ |

---

## 15. Strategic Position

### WOS Governs Existing Engines

Existing engines define **how** work flows. WOS defines **how work is governed**.

#### WOS inherits engine capabilities through bindings:

1. BPMN visual notation and mature tooling.
2. FEEL/DMN decision tables with production implementations.
3. WS-HumanTask lifecycle and task management UIs.
4. Process persistence and crash recovery.
5. Existing government deployments and developer communities.

#### Unique WOS capabilities:

1. **Deontic governance:** Structurally enforces Permission/Prohibition/Obligation/Right.
2. **Four-layer audit with epistemic status:** Separates verified facts, AI narratives, and counterfactuals.
3. **Structured oversight protocols:** Builds interface protocols (like independent-first) into the task lifecycle.
4. **Due process as a structural requirement:** Enforces appeal topology, continuation-of-service, and counterfactual explanation constraints.
5. **Universal interface contract:** Employs one spec for human forms, agent I/O, decision services, and integrations.
6. **Constraint zones:** Verifies satisfiability of declarative case management within statecharts.
7. **Formally verifiable constraints:** Provable governance properties via SMT.
8. **JSON-LD capable via sidecar:** Every WOS document can be projected as a valid RDF graph through the ontology-alignment sidecar.
