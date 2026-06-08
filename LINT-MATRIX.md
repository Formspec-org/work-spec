# WOS Verification Matrix

> **Studio-tier rules** (S1–S6, ~65 readiness rules) live in [`studio/STUDIO-LINT-MATRIX.md`](studio/STUDIO-LINT-MATRIX.md). This matrix covers the kernel-envelope tiers (T1 / T2 / T3) only. Studio's lint engine is implemented in `studio/crates/wos-studio-lint/`.

> **Updated 2026-06-08 (ADR 0096 — activation criteria)** — registered T2 rules `ACT-001` (activation criteria `where` MUST be valid FEL) and `ACT-002` (boolean-shaped, mirrors AI-058), implemented in `fel_analysis.rs::check_obligation_activation_fel` over `governance.obligationPolicies[*].{activateWhen,satisfyWhen,cancelWhen,violateWhen}.where`. Both start `draft` (fixtures land with the OBL-* conformance batch, Phase 7). `ACT-003`..`ACT-007` are listed as the normative catalog (`planned`) and not yet registry-backed. fel-core sibling absent in sandbox; CI is authoritative for compilation.
>
> **Updated 2026-06-08 (ADR 0096 — durable-obligations integrations)** — added T2 rules `ACT-008` (WOS-INTEG-MILE-1302: milestone `activationCriteria.where` parse + boolean-shape via `fel_analysis.rs::check_milestone_activation_fel`, deliberately disjoint from K-013's `condition`), `ACT-009` (WOS-TOOL-2502: unreachable obligation satisfaction) and `ACT-010` (WOS-TOOL-2503: impossible violation action — `createTask.taskRef`/`emitEvent.event` resolution), both in `fel_analysis.rs::check_obligation_authoring`. All three start `draft`; unit tests added; fixtures land with the OBL-* batch. fel-core sibling absent in sandbox; CI is authoritative for compilation.
>
> **Updated 2026-05-03 (rebase reconcile)** — WS-094 graduations (`WOS-VER-LEVEL-001` → `tested`, `K-031`/`K-032` typed walks under `tier1.rs::transition_actor_and_initial_state_tests`) merged on top of the Studio Stages 3-7 base. K-051/K-053 stay `tested`, K-052 stays `draft`. T1 covers both K-016 (root + compound + region key resolution) and K-032 (root resolution; matrix description carries the broader compound coverage even though the typed function is root-only — gap tracked).
>
> **Updated 2026-05-02 (Stage-4 Rust impl)** — `K-051`/`K-052`/`K-053` Rust impl landed via the corrected Stage-4 plan at `thoughts/plans/2026-05-01-stage-4-decision-table-lint-rules.md`. **Wave 0** ships `wos_core::model::decision_table::{DecisionTable, DecisionTableGuard, HitPolicy, Guard}` plus the `Transition.guard` polymorphism refactor + `KernelDocument.decision_tables` field. **Wave 1'** ships `crates/wos-lint/tests/decision_table_fixtures.rs` loading the 10 K-05X conformance fixtures. **Wave 2'** ships `crates/wos-lint/src/rules/decision_table.rs` with K-051/K-052/K-053 implementations + per-rule unit tests. JSONPath shape standardized at `crates/wos-lint/src/diagnostic.rs:104` (slash format, RFC-6901-shaped). All three rules graduated to `tested`. Cargo build + tests SHOULD pass locally where parent `fel-core` is mounted; sandbox cannot verify.
>
> **Updated 2026-05-02 (Wave-4 review remediation)** — `K-051`/`K-052`/`K-053` fixtures landed at `crates/wos-conformance/fixtures/K-05[123]-*.json` (10 files: 4+3+3 across the three rules). Status remains `draft` because the Rust impl is not yet shipped; graduation to `tested` requires the corrected Stage-4 plan at `thoughts/plans/2026-05-01-stage-4-decision-table-lint-rules.md` to land its three waves (wos-core typed surface area, new lint-test target, lint-rule impl).
>
> **Updated 2026-05-01** — three decision-table rules registered (`K-051`/`K-052`/`K-053` per Kernel §4.5.1) backing the `decisionTable` first-class kernel construct that lands `requiresSpecExtension` queue item from Studio's mapping spec (Studio→`mapsToWos`). Tier 1 also adds `K-031` (`transition.actor` pattern + declared `actors[].id`), `K-032` (root and compound `initialState` resolution), and `SCHEMA-OPEN-001` (`x-wos.openStringKind` for honest-open string leaves).
>
> **Updated 2026-04-28** for ADR 0076 step 12 — three new rules registered (`WOS-AGENT-XREF-001`, `WOS-SIG-COVER-001`, `WOS-VER-LEVEL-001`); I-001 reanchored to kernel/spec.md §9.2 (was Integration Profile §3.3.1, absorbed per ADR 0076 D-8).
>
> **Aggregate (post-rebase 2026-05-03):** **122** rules across **39** T1 / 74 T2 / 9 T3 (1 LoadBearing, 0 Stable, ≥4 Tested, balance Draft). Per-rule counts derive from the table below.

```text
┌─────────────────────────────────────────────────────────────────┐
│  Tier 1: wos-lint (static)            39 rules                  │
│  Single-document structural checks. Pattern matching and graph  │
│  walks over the JSON document tree. No parsing, no cross-doc.   │
├─────────────────────────────────────────────────────────────────┤
│  Tier 2: wos-lint --project (cross)   75 rules                  │
│  Multi-document resolution + FEL AST analysis. Loads a project  │
│  directory, resolves cross-references, parses FEL expressions.  │
├─────────────────────────────────────────────────────────────────┤
│  Tier 3: wos-conformance (dynamic)     9 rules                  │
│  Event-driven test fixtures. Feeds event sequences through      │
│  WosRuntime, asserts on observed state transitions, provenance  │
│  records, timer behavior, compensation ordering, and autonomy.  │
└─────────────────────────────────────────────────────────────────┘
```

**Graduation ladder:**

| Tier | Meaning |
|------|---------|
| `draft` | No executable fixture linked yet |
| `tested` | ≥1 executable fixture exercises this rule |
| `stable` | Tested + unchanged across ≥3 consecutive releases |
| `load_bearing` | Removing breaks ≥2 executable fixtures (spec_ref + suggested_fix required) |

## Tier 1 — Static Single-Document Rules (`wos-lint`)

| ID | Category | Graduation | Summary | Fixture evidence |
|-----|----------|------------|---------|------------------|
| `AI-041` | AI | tested | Fallback chain MUST terminate in `escalateToHuman` or `fail`; MUST NOT cycle. | AI-041-negative-fallback-cycle.json |
| `AI-049` | AI | draft | Narrative records MUST have `authoritative: false`. | — |
| `CM-001` | CM | draft | Entry template `id` values MUST be unique within the sidecar. | — |
| `G-037` | G | draft | Assertion `id` values MUST be unique within the library. | — |
| `G-038` | G | draft | Assertions of type `arithmetic`/`range`/`temporal` SHOULD include `expression`. | — |
| `G-039` | G | draft | Assertions of type `source-grounded`/`consistency` SHOULD include `fields`. | — |
| `G-044` | G | draft | Delegation `expirationDate` MUST be strictly after `effectiveDate`. | — |
| `G-045` | G | draft | Delegation `revokedDate` MUST be on or after `effectiveDate`. | — |
| `G-047` | G | draft | Parameter `values` entries MUST be in ascending `effectiveDate` order. | — |
| `G-048` | G | draft | Binding `id` MUST match the key under which it appears in the `bindings` map. | — |
| `G-050` | G | draft | Resolved parameter value MUST be type-consistent with declared `type`. | — |
| `G-055` | G | draft | `expectedDuration` MUST be an ISO 8601 duration or the literal `"indefinite"`. | — |
| `G-057` | G | draft | Binding `values` entries MUST be in ascending `effectiveDate` order. | — |
| `G-058` | G | draft | Each Holiday entry MUST specify exactly one of `date` or `rule`. | — |
| `G-059` | G | draft | Operating hours `end` MUST be strictly after `start`. | — |
| `G-062` | G | draft | Adverse-decision templates MUST cover determination, reasons, rights, and instructions. | — |
| `G-065` | G | draft | Notification template section `id` values MUST be unique within a template. | — |
| `I-001` | I | draft | `outputBinding` JSONPath MUST NOT use filter expressions or recursive descent. | — |
| `K-001` | K | tested | Final states MUST NOT have outgoing transitions. | K-001-negative-final-transitions.json |
| `K-002` | K | draft | Compound states MUST have `initialState` and `states`. | — |
| `K-003` | K | draft | Parallel states MUST have `regions`. | — |
| `K-004` | K | draft | `cancellationPolicy` MUST only appear on `parallel` states. | — |
| `K-005` | K | draft | `historyState` MUST only appear on `compound` states. | — |
| `K-006` | K | draft | Transition `target` MUST reference an existing state. | — |
| `K-007` | K | draft | Typed `message` event names MUST NOT start with `$`; typed `signal` allows `$join` and `$compensation.complete` only (JSON Schema + Tier 1 on parsed model). | tier1_rules.rs |
| `K-008` | K | draft | Parallel state outgoing transitions MUST use `$join` as event. | — |
| `K-009` | K | draft | Actor identifiers MUST be unique. | — |
| `K-014` | K | draft | Milestone `id` values MUST be unique. | — |
| `K-015` | K | draft | `setData` path MUST reference a declared `caseFile.fields` entry. | — |
| `K-021` | K | draft | Provenance `actorId` MUST reference a declared actor. | — |
| `K-022` | K | draft | Digest present implies algorithm recorded in extensions. | — |
| `K-029` | K | draft | `startTimer` MUST specify exactly one of `duration` or `deadline`. | — |
| `K-030` | K | draft | Extension keys MUST be `x-` prefixed. | — |
| `K-031` | K | draft | `transition.actor` when present MUST use the `actors[].id` lexical pattern and MUST name a declared kernel `actors[].id` (membership, not pattern alone). | inline (`tier1.rs::transition_actor_and_initial_state_tests`) |
| `K-032` | K | draft | Root `lifecycle.initialState` MUST be a key of `lifecycle.states`; compound `initialState` MUST be a key of that compound's `states` map (same rule id — diagnostic `path` distinguishes root vs compound). | inline (`tier1.rs::transition_actor_and_initial_state_tests`) |
| `K-048` | K | draft | Non-standard case relationship `type` values MUST use `x-` prefix. | — |
| `K-051` | K | tested | DecisionTableGuard `ref` MUST resolve to a top-level `decisionTables[]` entry; `outputColumn` MUST exist on the referenced table; every declared input MUST have an `inputBindings` entry (Kernel §4.5.1). | K-051-resolved-decision-table-guard.json, K-051-negative-unresolved-table-ref.json, K-051-negative-unresolved-output-column.json, K-051-negative-missing-input-binding.json (`crates/wos-lint/tests/decision_table_fixtures.rs`) |
| `K-053` | K | tested | DecisionTable input cells MUST evaluate to boolean; transition-guard `outputColumn` MUST be `boolean`-typed; `collect` hit policy is rejected for transition-guard usage (Kernel §4.5.1.4). | K-053-boolean-output-column.json, K-053-negative-non-boolean-output.json, K-053-negative-collect-hit-policy-on-guard.json (`crates/wos-lint/tests/decision_table_fixtures.rs`) |
| `SCHEMA-DOC-001` | SCHEMA-DOC | draft | Schema leaf properties MUST carry sufficient `description` and `examples`. | — |
| `SCHEMA-OPEN-001` | SCHEMA-OPEN | tested | Open string leaves MUST declare `x-wos.openStringKind` unless `enum`/`const`/`pattern` already close the leaf. | inline (`tests/schema_open_string_kind.rs`) |
| `WOS-VER-LEVEL-001` | WOS | tested | Agents declaring `fallbackChain` SHOULD have at least one `verificationLevel` declared on output bindings (ADR 0076 step 12, Q6). | inline (`tier1.rs::ver_level_tests`) |

**T1 total: 39** (0 LoadBearing, 0 Stable, 4 Tested, 35 Draft)

---

## Tier 2 — Cross-Document + FEL AST Rules (`wos-lint --project`)

| ID | Category | Graduation | Summary | Fixture evidence |
|-----|----------|------------|---------|------------------|
| `ACT-001` | ACT | draft | Obligation-policy activation criteria `where` MUST be valid FEL. | — |
| `ACT-002` | ACT | draft | Obligation-policy activation criteria `where` AST root MUST be boolean-shaped (no truthy coercion). | — |
| `ACT-003` | ACT | draft | Activation trigger `on.event` SHOULD resolve to a known workflow event. | — |
| `ACT-004` | ACT | draft | Activation `requiredData` paths SHOULD resolve to known case-file fields. | — |
| `ACT-005` | ACT | draft | Activation/deadline `within` MUST be a valid ISO 8601 / `P<N>BD` duration. | — |
| `ACT-006` | ACT | draft | Business-day `within` SHOULD declare a resolvable `calendarRef`. | — |
| `ACT-007` | ACT | draft | `activationCriteriaRef` MUST resolve to a named criteria (no duplicate ids). | — |
| `ACT-008` | ACT | draft | Milestone `activationCriteria.where` MUST be valid, boolean-shaped FEL (WOS-INTEG-MILE-1302; legacy `condition` stays K-013). | — |
| `ACT-009` | ACT | draft | Obligation `satisfyWhen.on.event` SHOULD be reachable in the static event graph — unreachable satisfaction (WOS-TOOL-2502). | — |
| `ACT-010` | ACT | draft | Obligation `onViolation.createTask.taskRef`/`emitEvent.event` MUST resolve to a known task/event (WOS-TOOL-2503). | — |
| `AG-008` | AG | draft | Side-effect tools at `autonomous` autonomy MUST declare a `sideEffectPolicy`. | — |
| `AG-010` | AG | draft | Verifiable constraints MUST satisfy all SMT subset restrictions (parse failures). | — |
| `AG-011` | AG | draft | `let` bindings in verifiable expressions MUST NOT be recursive. | — |
| `AG-012` | AG | draft | Quantifiers MUST quantify over finite domains (non-standard every/some arity). | — |
| `AG-013` | AG | draft | Verifiable arithmetic MUST be linear (no variable*variable products). | — |
| `AG-014` | AG | draft | Verifiable subset MUST NOT include extension function calls. | — |
| `AG-017` | AG | draft | Shadow mode is RECOMMENDED for rights-impacting workflows. | — |
| `AI-007` | AI | draft | Cascading autonomous agents MUST be declared via `cascadingInvocations`. | — |
| `AI-018` | AI | draft | `autonomous` actions MUST have associated deontic constraints. | — |
| `AI-020` | AI | draft | `supervisory` actions MUST define `reviewWindow`. | — |
| `AI-023` | AI | draft | Every agent invocation MUST have a reachable path to completion without any agent. | — |
| `AI-024` | AI | draft | Escalation conditions MUST be valid FEL referencing `@agent` context. | — |
| `AI-026` | AI | draft | Escalation MUST have `escalationExpiry`; agent reverts when expired. | — |
| `AI-031` | AI | draft | Agent output contract MUST apply same rules as human-facing form. | — |
| `AI-042` | AI | draft | Agent config MUST disclose training data characteristics. | — |
| `AI-043` | AI | draft | Agent config MUST disclose optimization objective. | — |
| `AI-046` | AI | draft | `rights-impacting` workflows MUST have `discloseThatAgentAssisted: true`. | — |
| `AI-056` | AI | draft | Autonomy is an action-site property, not an agent property. | — |
| `AI-057` | AI | draft | Capability `preconditions` entries MUST be valid FEL. | — |
| `AI-058` | AI | tested | Capability `preconditions` AST root MUST be boolean-shaped (no truthy coercion). | — |
| `SIG-001` | SIG | draft | Signature Profile `targetWorkflow.url` MUST match the loaded kernel URL. | — |
| `SIG-002` | SIG | draft | Signature Profile roles MUST reference declared kernel actors. | — |
| `SIG-003` | SIG | draft | Signature Profile roles MUST bind to human kernel actors. | — |
| `SIG-004` | SIG | draft | Signature role authenticationPolicyKey values MUST resolve. | — |
| `SIG-005` | SIG | draft | Signature signing-step roleId values MUST resolve. | — |
| `SIG-006` | SIG | draft | Signature signing-step documentId values MUST resolve. | — |
| `SIG-007` | SIG | draft | Signature signing-step dependencies MUST resolve and MUST NOT cycle. | — |
| `SIG-008` | SIG | draft | Routed signing guards MUST parse as valid FEL. | — |
| `SIG-009` | SIG | draft | Signature lifecycle tags SHOULD appear in the target kernel. | — |
| `SIG-010` | SIG | draft | Signature reminder and expiry events MUST map to kernel events. | — |
| `SIG-011` | SIG | draft | SignatureAffirmation evidence inputs MUST be satisfiable. | — |
| `SIG-012` | SIG | draft | Signature Profile fields MUST follow Ref/Key/Id naming conventions. | — |
| `DM-002` | DM | draft | Rights/safety workflows SHOULD follow shadow/canary/production sequence. | — |
| `G-001` | G | draft | Due process MUST be enforced for `rights-impacting` or `safety-impacting` kernels. | — |
| `G-003` | G | draft | Notice MUST include specific determination, reason codes, and appeal instructions. | — |
| `G-004` | G | draft | Explanation level MUST be `individualized` when kernel impact is `rights-impacting`. | — |
| `G-005` | G | draft | Adverse decisions MUST include positive and negative counterfactuals when rights-impacting. | — |
| `G-008` | G | draft | `continuationOfServices: true` requires kernel topology to freeze adverse impacts. | — |
| `G-009` | G | draft | Transitions tagged `adverse-decision` MUST trigger due process policy enforcement. | — |
| `G-011` | G | draft | Review protocol tags MUST match tags declared in the target kernel. | — |
| `G-014` | G | draft | Reasoning tier MUST be present for `determination`-tagged transitions. | — |
| `G-015` | G | draft | Counterfactual tier MUST be present for `adverse-decision` transitions in rights-impacting workflows. | — |
| `G-022` | G | draft | `excludedOwner` MUST override `potentialOwner` when actor appears in both. | — |
| `G-023` | G | draft | SLA evaluation SHOULD use business calendar when BC sidecar is present. | — |
| `G-024` | G | draft | Determination-tagged transitions MUST verify the actor has valid delegation. | — |
| `G-027` | G | draft | Sub-delegation MUST respect `maxDelegationDepth`. | — |
| `G-028` | G | draft | Hold policies MUST attach to kernel states tagged `hold`. | — |
| `G-029` | G | draft | `resumeTrigger` event name MUST reference an event in the target kernel. | — |
| `G-031` | G | draft | `resolutionDateRef` MUST refer to a field path in the kernel's case state. | — |
| `G-033` | G | draft | Parameter `values` SHOULD cover every resolution date (no coverage gap). | — |
| `G-034` | G | draft | `targetWorkflow` MUST match the `url` of the target kernel document. | — |
| `G-035` | G | draft | `targetGovernance` MUST reference a valid governance document. | — |
| `G-036` | G | draft | `independenceConstraint` MUST describe a mechanism preventing self-review. | — |
| `G-040` | G | draft | `consistency` assertions `referenceStage` MUST refer to an earlier pipeline stage. | — |
| `G-041` | G | draft | Pipeline-stage assertion ids MUST exist in the targeted assertion library. | — |
| `G-042` | G | draft | FEL expressions in assertion `expression` fields MUST be syntactically valid. | — |
| `G-043` | G | draft | FEL expressions in delegation `conditions` MUST be syntactically valid. | — |
| `G-046` | G | draft | Delegation `delegator`/`delegate` MUST reference declared kernel actors. | — |
| `G-053` | G | draft | Sub-delegation is only permitted if the original delegation explicitly allows it. | — |
| `G-056` | G | draft | Binding `resolutionDateRef` MUST reference a field path in kernel case state. | — |
| `G-060` | G | draft | Business Calendar target requires SLA evaluation in business days. | — |
| `G-063` | G | draft | Notification template keys MUST resolve to a template in a targeting sidecar. | — |
| `G-066` | G | draft | BreachPolicy escalationStepId MUST resolve within the task pattern. | — |
| `K-010` | K | draft | createTask `assignTo` MUST reference a declared kernel actor. | — |
| `K-012` | K | draft | Guards MUST be valid FEL expressions. | — |
| `K-013` | K | draft | Milestone conditions MUST be valid FEL expressions. | — |
| `K-017` | K | draft | FEL guards MUST NOT reference related case state. | — |
| `K-019` | K | draft | FEL functions MUST be declared built-ins or registered extensions. | — |
| `K-037` | K | draft | Fail-fast `$join` fires only on an error final state. | — |
| `K-049` | K | load_bearing | Continuous-mode kernels MUST NOT contain `setData` → guard dependency cycles. | k-049-load-bearing-self-loop.json, k-049-load-bearing-two-node-cycle.json |
| `K-052` | K | tested | DecisionTable rows for `hitPolicy = unique` MUST be pairwise disjoint over the declared input domain; `hitPolicy = priority` rows MUST have unique `priority` values among rows that overlap (Kernel §4.5.1.4). Cross-document because resolution depends on the table's declared input types and FEL AST analysis. | K-052-disjoint-unique-rows.json, K-052-negative-overlapping-unique-rows.json, K-052-negative-priority-tie.json (`crates/wos-lint/tests/decision_table_fixtures.rs`) |
| `K-EXT-002` | K-EXT | tested | `x-wos-*` namespace is reserved for future normative WOS use. | x-wos-reserved-warn.json, x-vendor-custom-ok.json |
| `VR-003` | VR | draft | `counterexample` MUST be present when result is `proven-unsafe`. | — |
| `WOS-AGENT-XREF-001` | WOS | tested | Every actor with `type=='agent'` MUST have a matching `agents[].id` (ADR 0076 D-2 cross-reference). | inline (`tier2.rs::tests::wos_agent_xref_001_*`) |
| `WOS-SIG-COVER-001` | WOS | tested | Signature-gated transitions MUST be covered by `signature.signers[]` (ADR 0076 D-2 cross-reference). | inline (`tier2.rs::tests::wos_sig_cover_001_*`) |

**T2 total: 80** (1 LoadBearing, 0 Stable, 4 Tested, 75 Draft)

---

## Tier 3 — Dynamic Runtime Rules (`wos-conformance`)

| ID | Category | Graduation | Summary | Fixture evidence |
|-----|----------|------------|---------|------------------|
| `AI-001` | AI | tested | Processor MUST implement agent registration (AI S3). | ai-005-no-override-human.json, ai-009-permission-bounds.json, ai-034-confidence-report-required.json |
| `AI-002` | AI | tested | Processor MUST implement the confidence framework (AI S7). | ai-034-confidence-report-required.json, ai-035-calibrated-confidence.json, ai-036-confidence-below-floor.json |
| `AI-004` | AI | draft | Processor MUST delegate Formspec evaluation to a conformant processor. | — |
| `AI-050` | AI | draft | Assist Governance Proxy MUST NOT modify conformance requirements. | — |
| `AI-AUTO-001` | AI-AUTO | tested | Escalation expiry MUST revoke elevated autonomy and emit an autonomyDemotion record (AI S5.5). | AI-AUTO-001-escalation-expiry-revocation.json |
| `AI-AUTO-002` | AI-AUTO | tested | Drift-alert thresholds with action=demoteToAssistive MUST emit autonomyDemotion + driftReclassification and reroute the event through escalation (AI S5.5). | AI-AUTO-002-drift-alert-demotion.json |
| `G-051` | G | tested | Governance Basic processor MUST enforce due process and review protocols. | g-002-notice-before-adverse.json, g-006-appeal-independent-reviewer.json, g-007-appeal-provenance.json, g-010-independent-first.json, g-016-review-sampling.json, g-017-reviewer-separation.json, g-018-override-rationale.json |
| `G-052` | G | tested | Governance Complete processor MUST enforce all normative sections. | g-002-notice-before-adverse.json, g-006-appeal-independent-reviewer.json, g-007-appeal-provenance.json, g-010-independent-first.json, g-012-pipeline-stage-provenance.json, g-013-weakest-link-risk.json, g-016-review-sampling.json, g-017-reviewer-separation.json, g-018-override-rationale.json, g-019-override-immutable.json, g-020-rejection-detail.json, g-021-task-provenance.json, g-025-delegation-required.json, g-026-delegation-in-provenance.json, g-030-hold-timer-start.json, g-032-temporal-resolution.json, g-049-binding-type-neutral.json, g-054-resume-cancels-hold-timer.json, g-061-expired-calendar-ignored.json, g-064-notification-missing-variables.json |
| `K-DET-001` | K-DET | tested | Determination-tagged transitions MUST emit the pre-transition case-file snapshot in Facts-tier provenance. | k-det-001-determination-snapshot.json |

**T3 total: 9** (0 LoadBearing, 0 Stable, 7 Tested, 2 Draft)

---

## Summary

| Tier | Total | LoadBearing | Stable | Tested | Draft |
|------|-------|-------------|--------|--------|-------|
| T1 | 39 | 0 | 0 | 4 | 35 |
| T2 | 74 | 1 | 0 | 4 | 69 |
| T3 | 9 | 0 | 0 | 7 | 2 |
| **Total** | **122** | **1** | **0** | **15** | **106** |
