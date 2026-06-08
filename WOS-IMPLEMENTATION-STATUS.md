# WOS Implementation Status & Roadmap

**Last updated:** 2026-06-08
**Status:** Certified Reference Implementation (Draft 1)

This document tracks crate maturity, test coverage, and the technical roadmap. For a high-level feature comparison, see `WOS-FEATURE-MATRIX.md`.

---

## 1. Crate Maturity Matrix

| Component | Status | Detail |
|-----------|--------|--------|
| **wos-core** | ✅ | Implements the typed evaluation kernel, deontic/autonomy modules, and explanation assembly. |
| **wos-lint** | ✅ | Executes 116 registered rules (35 T1 + 72 T2 + 9 T3) against typed models and project graphs, including SIG-001..SIG-012 for Signature Profile consistency. |
| **wos-conformance** | ✅ | Manages 162 fixtures, including 13 SIG-* Signature Profile runtime fixtures, and handles Batch 16 processor reporting. |
| **wos-runtime** | ✅ | Orchestrates generic persistence, durable execution, event queues, Signature Profile task evidence validation, and `SignatureAffirmation` emission. |
| **wos-formspec-binding** | ✅ | Implements the S15 protocol, task prefill, and response validation. |
| **wos-export** | ✅ | Serializes provenance to PROV-O JSON-LD (§5.3–5.6), XES XML (§6.3), OCEL 2.0 JSON (§6.4); 3 SP-EXPORT-* conformance fixtures green. |
| **wos-assurance** | 🟡 | Spec complete; reference implementation pending. Attaches via provenanceLayer and custodyHook seams. |

---

## 2. Verification Progress (LINT-MATRIX)

WOS verifies 116 registered constraints across three tiers.

| Tier | Type | Rules | Verified | Gap |
|------|------|-------|----------|-----|
| **Tier 1** | Single-Doc | 35 | 35 | 0 |
| **Tier 2** | Cross-Doc / AST | 72 | 72 | 0 |
| **Tier 3** | Runtime | 9 | 9 | 0 |
| **Total** | | **116** | **116** | **0** |

**NB:** Counts reflect the current `wos-lint` registry and `LINT-MATRIX.md`, including the Signature Profile SIG-* rule family. Assurance-layer rule additions remain future registry work.

---

## 3. Reference Implementation Details

### Formspec Coprocessor (S15)
Runtime Companion S15 specifies the handoff between WOS tasks and Formspec forms.
*   **Handoff Protocol:** Orchestrated by `wos-runtime`.
*   **Mapping DSL:** Synchronizes `caseFile` data via `wos-formspec-binding`.
*   **Validation Ordering:** Prioritizes contract validation before assertion gates in `wos-core`.

### Durable Execution
`wos-core` implements lifecycle behavior while `wos-runtime` manages persistence, profile attachment, and evidence-producing task execution.
*   **Instance Loading:** Resolves kernel versions strictly.
*   **Atomic Saves:** Guarantees state consistency via the `RuntimeStore` interface.
*   **Timer Materialization:** Checks tolerances during simulated time advancement.
*   **Signature Profile Runtime:** Loads Signature Profile documents, validates signing evidence from Formspec task responses, emits `SignatureAffirmation`, gates lifecycle completion across sequential/parallel/routed/free-for-all flows, and records decline, void, reassignment, and timer-driven expiry evidence.

---

## 4. Technical Architecture & Standards Alignment

WOS employs a linked-data architecture to ensure interoperability and AI-safety.

### 4.1 Semantic Interoperability
*   **JSON-LD Native:** Every WOS document functions as a valid RDF graph.
*   **SHACL Governance Shapes:** Validates semantic constraints beyond JSON Schema limits.
*   **SPARQL-Queryable:** Supports cross-workflow analysis via standard RDF queries.

### 4.2 Architecture Principles
*   **Layered Opt-in:** Kernel, Governance, AI, and Advanced layers remain independently adoptable.
*   **Sidecar Document Pattern:** Isolates configuration into separate, updatable documents.
*   **Extension Seams:** Provides named attachment points for actors, contracts, and lifecycle events.
*   **Separation of Concerns:** Segregates lifecycle, case state, and audit logs.
*   **Conformance Profiles:** Offers incremental tiers for engine adoption.

### 4.3 AI-Native Patching
*   **Typed Patch Operations:** AI proposes edits as statically analyzable AST operations.
*   **Four-stage Validation:** Checks every AI edit for schema, SHACL, soundness, and provenance.

---

## 5. Engineering Roadmap

### Phase 1: Engine Bindings (§1 Reference Blockers Complete)
§1 reference implementation blockers are complete as of 2026-04-14. WOS-T3 selected Restate as the first production durable backend; remaining Phase 1 work is adapter implementation:
*   [ ] **Camunda 8 Worker:** Delegates BPMN task execution to WOS governance.
*   [ ] **Restate Adapter:** Implements `DurableRuntime` over tenant-qualified Restate workflows/virtual objects.
*   [ ] **Temporal Workflow:** Deferred until the official Rust workflow API stabilizes; maps WOS evaluation steps to deterministic replay when revisited.
*   [ ] **AWS Step Functions:** Bridges ASL states to WOS transitions.
*   [x] **Integration Profile Processor:** CloudEvents 1.0 (`event-emit`, `event-consume`, `callback`), Arazzo multi-step sequences, tool invocations, and policy engine bridges all implemented in `wos-runtime`. 13 INT-* conformance fixtures green. (NB.3 + NB.4 complete)
*   [x] **Business Calendar SLA Evaluation:** `wos-business-calendar` sidecar consumed for Governance S10.3 SLA deadline computation; lazy evaluation at check time; `calendarVersion` snapshot; 4 G-S10-* fixtures green. (BC.1 complete)

### Phase 2: Advanced Provenance (Future)
*   [x] **History State Semantics:** DeepHistory (full state snapshot) and ShallowHistory (exit point only) implemented in `wos-core`. 9 K-H-* conformance fixtures covering depth-1, depth-2, depth-3, and parallel-exit re-entry. (KS.1 complete)
*   [x] **Milestone Firing:** Data-driven milestone firing independent of workflow state implemented in `wos-runtime`. Ordering pinned: data write durable → `MilestoneFired` → reactive transitions evaluated. 5 K-M-* conformance fixtures. (KS.2 complete)
*   [ ] **Merkle Provenance Chains:** Adds cryptographic hash-chaining for tamper-proof logs.
*   [x] **Provenance Export Formats:** `wos-export` crate serializes internal provenance to W3C PROV-O (JSON-LD), IEEE 1849 XES (XML), and OCEL 2.0 (JSON) per Semantic Profile §§5.3–5.6 and §§6.3–6.4; `timestamp` added to `ProvenanceRecord` as export prerequisite; 3 SP-EXPORT-* conformance fixtures green (`sp-export-prov-o`, `sp-export-xes`, `sp-export-ocel`). Known limitations: higher-tier PROV-O bundles (§5.4 Reasoning/Counterfactual/Narrative) not emitted; OCEL case-file-item object tracking linked to instance object only (per-item E2O links future); SHACL validation of PROV-O output out of scope; agent actor-type currently falls back to plain `prov:Agent` pending `ProvenanceRecord` actor-type extension.
*   [ ] **Simulation Trace Format:** Standardizes formats for replaying simulation runs.
*   [ ] **Federation Profile:** Enables cross-processor migration and signal routing.
*   [x] **Shared Activation Criteria + Durable Obligations** (ADR 0096; **landed on branch, PR #4 — compilation and exact conformance traces are CI-gated**): a reusable `ActivationCriteria` shape ("when does this become active?") and durable pending obligations (`ObligationPolicy` → `PendingObligation`, lifecycle `pending → satisfied/violated/cancelled/expired/bypassed`) with deadline timers, violation actions, and provenance. Distinct from DCR constraint zones (zone-local flexible-activity sequencing), task SLAs (`SlaDefinition`), milestones (data-driven checkpoints), the deontic `Obligation` (immediate pre-commit agent-output check), and the policy-engine `Obligation` (per-decision directive). No FEL grammar change (FEL stays the local boolean predicate inside `where`).
    *   **Spec + schema:** Governance §16 normative contract; `$defs/ActivationCriteria`, `ObligationPolicy`, `ObligationDeadline`, `ObligationViolationAction`, `PolicyObligationHandling`, and `governance.obligationPolicies[]` on `wos-workflow.schema.json`.
    *   **`wos-core`:** activation/obligation model + deterministic activation evaluator (trigger → actor → requiredData → `where` → deadline, short-circuit, no truthy coercion).
    *   **`wos-lint`:** `ACT-001..010` registered (Tier 2, all `draft` — unit-test evidence; FEL-backed fixtures land with the OBL-* batch). See `LINT-MATRIX.md`.
    *   **`wos-events`:** seven obligation `ProvenanceKind` variants (`ObligationActivated/Satisfied/Violated/Cancelled/Expired/Bypassed/Warning`) + witness + PROV-O/XES/OCEL export.
    *   **`wos-runtime`:** obligation monitor wired into `drain_once` — activation/satisfaction/cancellation, pre-event violate-block, lazy deadline expiry, violation-action effects with strictest-action (`warn < escalate < fail < block`) gating, replay dedupe, count cap, fail-closed posture for `rightsImpacting`/`safetyImpacting`, bypass/extension authorizer.
    *   **`wos-conformance`:** 13 `OBL-001..013` fixtures (Phase-6 integrations include milestone/SLA/hold criteria, policy-engine materialization, AI bypass/independence).
    *   **Verification posture:** the standalone WOS checkout **cannot compile Rust** — `fel-core` and the private sibling repos are absent from the sandbox, so `cargo nextest` / `make ci` do not run here. The authoritative gate is the formspec-stack monorepo CI (`.github/workflows/rust-tests.yml`) once the `STACK_REPOS_TOKEN` secret is added; the build step self-skips until then. Conformance fixtures are **authored but not yet executed** in this repository (WOS-GATE-2903 / WOS-GATE-2904). Full fixture↔§16-claim and rule↔check traceability: [`docs/obligation-conformance.md`](docs/obligation-conformance.md). Deferred follow-ups: SLA/Hold `ActivationCriteria` runtime wiring, true business-day deadline expiry (WOS-OBL-TIME-1002 / TIME-1008), DCR per-activity activation gating (WOS-INTEG-DCR-1602), deadline-index performance work (2801).

### Phase 3: Adoption Artifacts
*   [ ] Kubernetes and Serverless reference deployment patterns.
*   [ ] Processor certification narratives for procurement.
*   [ ] Sector-specific competitive analysis (Health, Defense, Benefits).

---

## Appendix A: Audit Corrections

| Feature | xlsx Rating | Corrected To | Reason |
|---------|------------|--------------|--------|
| Decision tables (DMN) | ■ | 🟡 (integration) | Kernel requires no embedded decision engine. |
| Decision requirement graphs | ■ | 🟡 (integration) | Defined by external integration only. |
| Merkle tree logging | ■ | 🟡 (per-record) | Published spec uses per-record digests; Merkle chains deferred. |
| RO-Crate packaging | ■ | ⚪ | Draft only; omitted from published specs. |
| MCP agent-tool protocol | ■ | -- (Formspec) | Specified as a Formspec package, not a WOS feature. |
| CaMeL dual-LLM | ■ | 🟡 (informative) | Specified as optional guidance in S3.6. |
| Capability routing | ■ | ⚪ | Remains implementation-defined in the kernel. |
| Defeasible rules | ■ | ✅ | Implemented via authority-ranked assembly. |
| Business calendar SLA | 🟡 | ✅ | Lazy evaluation at check time; `calendarVersion` snapshot; 4 G-S10-* fixtures. (BC.1) |
| CloudEvents binding | 🟡 | ✅ | `event-emit`, `event-consume`, `callback` with subject correlation and full envelope provenance; 6 INT-* fixtures. (NB.3) |
| Arazzo orchestration | 🟡 | ✅ | Per-step `invokeService` invocations with step-level provenance; pause/resume across sequence; 3 INT-ARAZZO-* fixtures. (NB.4) |
| Policy engine bridge | 🟡 | ✅ | `PolicyDecision` normalized to `{decision, reasons, obligations}` at binding boundary; OPA adapter; 4 INT-POLICY-* fixtures. (NB.4) |
| History states | 🟡 | ✅ | DeepHistory + ShallowHistory implemented; 9 K-H-* fixtures covering depth-1, depth-2, parallel-exit, depth-3. (KS.1) |
| Milestone firing | 🟡 | ✅ | Milestone firing with pinned ordering (write → MilestoneFired → transitions); 5 K-M-* fixtures. (KS.2) |
| PROV-O / OCEL / XES export | 🟡 | ✅ | `wos-export` crate emits PROV-O JSON-LD (§5.3–5.6), XES XML (§6.3), OCEL 2.0 JSON (§6.4); 3 SP-EXPORT-* conformance fixtures green. Bundles, per-item OCEL links, and SHACL validation deferred. |

---

## Appendix B: Standards Lineage

*   **Adopted Intact:** WS-HumanTask, CMMN Case File, DMN (integration), CloudEvents, W3C PROV, JSON Schema.
*   **Adapted:** BPMN events (taxonomy), SCXML (statechart semantics), XACML (PEP/PDP), Catala (default logic), OpenFisca (temporal parameters), GSM (milestones), DCR (constraint zones).
*   **Evaluated but Rejected:** WS-BPEL, XPDL, YAWL, Azure Durable Functions, Netflix Conductor, Google Zanzibar.
