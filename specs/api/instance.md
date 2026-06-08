# WOS Public API Workflow Process

**Status:** Draft
**ADR:** [`thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md`](../../../thoughts/adr/0082-stack-public-api-contract-and-schema-discipline.md) (D-15 step 4)
**Schema:** [`api/instance.schema.json`](../../schemas/api/instance.schema.json)
**Schema ID:** `https://schemas.formspec.io/wos-api/instance/v1`

## Purpose

`WorkflowProcess` is the public projection of a running WOS workflow instance — the case-file state, lifecycle posture, and identity that drive the case-portal and SDK consumers. `CaseLedgerProjection` is the durable case-level view keyed by the governed case ledger; it can exist before any workflow process starts and aggregates bound `WorkflowProcess` process projections. `CaseLedgerEventSummary` summarizes the latest direct case-ledger provenance event when one exists and exposes the D26 event literal rather than the redundant inner `recordKind` discriminator. Greenfield per ADR 0082 D-15: the kernel runtime artifact (`wos-process.schema.json`) and the prior `case-portal/src/ports/types.ts` `WorkflowProcessView` are prior art, not this contract. Per ADR 0082 D-3, process resources carry lifecycle and `caseState` only; governance, tasks, timers, holds, compensation, and cross-case relationships are subresources and the `?include=` aggregation seam is closed. `CustodyReceipt` exposes the Trellis custody-anchoring receipt projection; `CompensationLogEntry` and `CompensationLogEntryPage` supply the kernel compensation log (Kernel §9.5).

## Resource Shape

`WorkflowProcess` carries identity, workflow binding, lifecycle posture, current configuration, and the case-file value:

- `id`: `urn:wos:<typeid>` URN per ADR 0092 D-1. The namespace-specific string IS the TypeID. The TypeID prefix carries the record family (e.g., `case`).
- `workflowUrl`, `workflowVersion`: the governing Workflow Document reference (Kernel S9.6).
- `lifecycleState`: closed posture taxonomy `active | suspended | migrating | completed | terminated | stalled` mirroring the kernel runtime `status` enum at `wos-process.schema.json#/properties/status` (`x-wos.mirror` annotation enables Gate 6 parity).
- `impactLevel`: REQUIRED closed-with-vendor-extension kernel impact level (Kernel S6; kernel:826) cross-`$ref`d from `task.schema.json#/$defs/ImpactLevel`. Surfaces the workflow's proportionality index at instance scope so clients can branch on `rights-impacting` / `safety-impacting` cases without re-fetching the workflow.
- `configuration`: ordered active leaf-state identifiers (Kernel S4.2). Empty for terminal instances.
- `stalledSince`: REQUIRED when `lifecycleState == stalled` (ADR 0070 D-5).
- `outcomeCode`: optional machine-readable terminal-outcome marker (Kernel S6 final-state `outcomeCode`). Populated only when `lifecycleState == completed`; absent on every other lifecycle state. Identifier-shaped.
- `milestonesFired`: optional sorted set of milestone identifiers fired on this case (Kernel S4 milestones; kernel:661-672). Companion: Agent B's `milestoneFired` literal on `FactsRecordKind` carries the per-event detail.
- `continuationOfServicesActive`, `continuationOfServicesEndsAt`: optional flag and absolute deadline for the active continuation-of-services window (governance §3.6 — `AppealMechanism.continuationOfServices`; workflow-governance.md:154-158). Surfaces the existence of the policy without leaking the underlying rationale; staff caseworkers use it to avoid premature termination during an active appeal.
- `dcrZones`: optional per-instance state of declared DCR constraint zones (advanced/advanced-governance.md §1.2). Cross-cite Agent B's `dcrZoneViolation` Facts record kind for the per-event detail in provenance.
- `caseState`: opaque-shaped case-file value; per-workflow validation is the reference server's responsibility against the resolved Workflow Document.
- `createdAt`, `updatedAt`: RFC 3339 UTC timestamps per ADR 0082 D-10.
- `tenant`: optional tenant scope per ADR 0068 D-1.1.
- `correlationKey`: optional cross-case correlation identifier (Kernel S9.4).

`WorkflowProcess` does NOT carry `activeTasks`, `timers`, `holds`, `delegations`, or `relatedCases`. Those are subresources.

## Subresources and `?include=`

Per ADR 0082 D-3, subresource endpoints scale pagination per resource and prevent the "first 50 active tasks always come along" pathology. The closed `?include=` taxonomy (D-3) supports single-call UI views without GraphQL-style open composition.

| Subresource | Endpoint | Schema | `include=` literal | Pagination |
|---|---|---|---|---|---|
| Compensation | `GET /api/v1/cases/{case_id}/processes/{process_id}/compensation` | `CompensationLogEntryPage` | `compensation` | cursor (D-7) |
| Governance | `GET /api/v1/cases/{case_id}/processes/{process_id}/governance` | `WorkflowProcessGovernance` | `governance` | not paginated; small bounded set |
| Tasks | `GET /api/v1/cases/{case_id}/processes/{process_id}/tasks` | `TaskPage` (from `task.schema.json`) | `tasks` | cursor (D-7); query filters per `TaskListOptions` |
| Timers | `GET /api/v1/cases/{case_id}/processes/{process_id}/timers` | `WorkflowProcessTimerList` | `timers` | not paginated |
| Holds | `GET /api/v1/cases/{case_id}/processes/{process_id}/holds` | `WorkflowProcessHoldList` | `holds` | not paginated |
| Related cases | `GET /api/v1/cases/{case_id}/processes/{process_id}/related` | `WorkflowProcessRelatedList` | `related` | not paginated |
| Provenance | `GET /api/v1/cases/{case_id}/processes/{process_id}/provenance` | `ProvenanceRecordPage` (from `provenance.schema.json`) | NOT in `?include=` | cursor (D-7); query filters per `ProvenanceListOptions` |
| Correspondence | `GET /api/v1/cases/{case_id}/processes/{process_id}/correspondence` | `CorrespondenceMessagePage` (from `correspondence.schema.json`) | `correspondence` | cursor (D-7); query filters per `CorrespondenceListOptions` |

Provenance is intentionally NOT in the `?include=` taxonomy because the cardinality is unbounded and provenance domain-ratification (ADR 0082 D-15 step 3) already supplies a dedicated paginated endpoint.

### `GET /api/v1/cases/{case_id}/processes/{process_id}/provenance` — query filters

`ProvenanceListOptions` (from `provenance.schema.json`) carries the closed query-parameter set. With 57 reserved Facts-tier `recordKind` literals and hot cases potentially accruing thousands of records, narrow filters are required for query planners to keep page latency bounded. The shape mirrors `AuditQueryRequest` from `audit.schema.json` so cross-case audit and per-case provenance share the same query patterns:

- `tier?`: closed enum `facts | reasoning | counterfactual | narrative` — the existing single-tier filter.
- `recordKindFilter?: FactsRecordKind[]`: closed-with-vendor-extension `recordKind` array. Cross-`$ref`s the `FactsRecordKind` taxonomy declared in `provenance.schema.json` (ADR 0082 D-14 — never redefine inline). Empty array is rejected (`WOS-1422`); omit the field to span every record kind.
- `timeRange?: { since, until }`: inclusive UTC RFC 3339 window against the record `timestamp` field. Same shape as `AuditQueryRequest.timeRange`.
- `actorRefFilter?: ActorRef[]`: closed `actor:(human|service-account|workload|support):...` URN array; cross-`$ref`s `_common.schema.json#/$defs/ActorRef`. Empty array is rejected; omit to span every actor.
- `cursor?`, `limit?`: standard pagination per ADR 0082 D-7.

**Composition.** Filters compose with AND semantics — a record matches when it satisfies every supplied filter independently. `recordKindFilter` is implicitly tier-scoped to Facts (the kernel `recordKind` is a Facts-tier field); supplying `recordKindFilter` alongside `tier=reasoning` is rejected with `WOS-1422` because the cross-tier intersection is empty by construction. Cursor pagination per D-7 still applies on top of the filter set; cursors are opaque, single-use within the issuing deploy, and `WOS-1410` on expiry restarts pagination from the top.

`?include=` is a closed enum (`IncludeKind`): `compensation | governance | tasks | timers | holds | related`. Per-section limits (for example `?include=tasks(limit=10)`) are enforced server-side; the inline `tasks` projection uses the lighter `TaskListItem` shape so list views inside the aggregation match the standalone task list. No vendor extensions on `IncludeKind` — closed by design (D-3 framing).

The aggregated response shape is `WorkflowProcessWithIncludes`: a required `instance` envelope (the same `WorkflowProcess` shape returned with no `?include=`) plus optional subresource fields. Subresource fields are present only when their literal was requested. This keeps the aggregation seam discoverable and structurally explicit.

## Governance subresource fields

`WorkflowProcessGovernance` (the `governance` `include=` literal) carries the enriched governance projection for a workflow process. Beyond the required `processId`, `delegations` (governance S11), and `reviewState` (closed-with-vendor-extension `none | pending | in-review | cleared | flagged` plus `^x-[a-z]+-` extensions), the following optional fields project governance posture at instance scope:

- `adverseDecisionPolicyActive?: boolean` — when true, the workflow's adverse-decision policy is active. The case portal SHOULD surface notices about appeal rights, deadlines, and continuation-of-services windows (governance §3.6).
- `reviewProtocolActive?: ReviewProtocolKind` — the review protocol currently active on this instance. Projected from the workflow's review-protocol declaration. Present only when a review-protocol-governed transition is in the active configuration. Cross-`$ref`d from `task.schema.json#/$defs/ReviewProtocolKind`.
- `activeEscalation?: { level, escalatedTo, escalatedAt, reason }` — current escalation posture. Present only when an escalation chain is actively walking; absent otherwise. `level` (integer >= 1) is the current 1-based escalation level; `escalatedTo` carries the `ActorRef` URN the case was reassigned to; `escalatedAt` is the RFC 3339 UTC timestamp; `reason` carries the `EscalationReason` cause cross-`$ref`d from `governance.schema.json`.
- `activeHoldsCount?: integer (>= 0)` — count of currently active holds. The holds subresource carries the per-hold detail (governance S12).
- `activeGovernanceRules?: [ { ruleId, ruleKind, triggerTag?, activatedAt } ]` — governance rules currently active on this instance's state configuration. `ruleKind` is the closed taxonomy `lifecycle-hook | contract-hook | review-protocol | due-process-notice | assertion-gate`. `triggerTag` is an optional identifier for the originating trigger; `activatedAt` is the RFC 3339 timestamp when the rule became active.
- `pendingObligations?: PendingObligation[]` — durable obligations currently outstanding on this instance (WOS-API-2201, ADR 0096 D-2/D-5). Backward compatible: optional and default-absent on instances that declare no obligation policies. Projects the runtime `WorkflowProcess.governanceState.pendingObligations[]` state (Rust `wos_core::instance::GovernanceState.pending_obligations`); ordered by `activatedAt` ascending.

## Durable obligations

A durable obligation (ADR 0096) is a future-tense, cross-event duty: "after X happens, Y must happen before Z or within T, by actor/role R, otherwise apply action A." The author-time policy is `ObligationPolicy` at `wos-workflow.schema.json#/$defs/ObligationPolicy`; this API surface projects the runtime obligation state and exposes governance commands over it.

`PendingObligation` (WOS-API-2201) is the read-only runtime projection of a single tracked obligation, mirroring kernel `wos_core::instance::PendingObligation`. Fields: `obligationId` (runtime instance id, unique within the process), `policyId` (originating `ObligationPolicy.id`), closed `status` (`pending | satisfied | violated | cancelled | expired | bypassed`), optional `triggerEvent` and `triggerActorId` (the latter load-bearing for `satisfyWhen` separation-of-duties checks), `activatedAt`, optional `deadline` (computed `activatedAt + ObligationDeadline.within`), optional `responsibleActor`/`responsibleRole`, optional `correlationKey`, and the `x-`-keyed `extensions` vendor bag. It appears as the array element of `WorkflowProcessGovernance.pendingObligations`.

### Obligation-status projection

`ObligationStatusList` (WOS-API-2202) is the obligation-status query response for a workflow process: `{ processId, items: ObligationStatusItem[] }`, not cursor-paginated because the set is small and bounded. Each `ObligationStatusItem` wraps the full `obligation: PendingObligation` plus list-level `policyId`, `status`, optional `deadline`, and an `explanation` — a human-readable statement of why the obligation holds its current status (policy duty, breach/satisfaction cause, or bypass rationale). `ObligationStatusQuery` is the reserved query-parameter set: `status[]`, `responsibleActor`, `responsibleRole`, `deadlineBefore`, `deadlineAfter`. Filters compose with AND semantics; an empty `status[]` is rejected with `WOS-1422` and a missing filter spans every value. Obligations without a deadline are excluded when either deadline-window filter is present. Items are ordered by `deadline` ascending.

### Obligation commands

`ObligationBypassCommand` (WOS-API-2203) records that a `pending` obligation is intentionally not satisfied: `{ obligationId, actorRef, rationale }`. The runtime sets `status` to `bypassed` and emits a Facts-tier provenance record carrying the actor and rationale; bypassing a terminal obligation is rejected with `WOS-1409`. `actorRef` and `rationale` are REQUIRED so the bypass is never silent.

`ObligationDeadlineExtensionCommand` (WOS-API-2203) pushes a `pending` obligation's deadline out: `{ obligationId, actorRef, rationale }` plus exactly one of `newDeadline` (absolute RFC 3339 instant) or `newWithin` (relative ISO 8601 duration with the `P<N>BD` business-day extension, mirroring `ObligationDeadline.within`). The `oneOf` makes supplying both, or neither, structurally inexpressible. `newDeadline` earlier than the current deadline is rejected with `WOS-1422`; extending a terminal obligation is rejected with `WOS-1409`. The runtime reschedules the deadline timer and emits a Facts-tier provenance record.

## DCR constraint zone state

`DcrConstraintZoneState` (projected on `WorkflowProcess.dcrZones`) carries per-zone DCR constraint posture for lifecycle states that declare advanced-governance constraint zones (advanced-governance.md §1.2). In addition to the existing `zoneId`, `currentLevel` (closed `none | caution | breach`), and `lastTriggered` (RFC 3339 timestamp of last transition out of `none`), the enriched projection adds:

- `pendingActivities?: string[]` — activity identifiers currently in the 'pending' marking within this DCR zone. Each entry matches an activity name declared in the workflow's constraint-zone declaration.
- `violatedRelations?: [ { relationType, source, target } ]` — DCR relations currently in violation within this zone. `relationType` is the closed DCR relation taxonomy `condition | response | include | exclude | milestone` per advanced-governance.md. `source` and `target` are identifier-shaped strings naming the activities whose relation is in violation. Empty or absent when no relations are in violation.

## Endpoints

```
GET   /api/v1/cases/{case_id}/processes                       -> WorkflowProcessList
POST  /api/v1/cases/{case_id}/processes                       -> WorkflowProcess
GET   /api/v1/cases/{case_id}/processes/{process_id}                  -> WorkflowProcess | WorkflowProcessWithIncludes
GET   /api/v1/cases/{case_id}/processes/{process_id}/governance       -> WorkflowProcessGovernance
GET   /api/v1/cases/{case_id}/processes/{process_id}/compensation      -> CompensationLogEntryPage     (cursor-paginated)
GET   /api/v1/cases/{case_id}/processes/{process_id}/tasks            -> TaskPage                    (cursor-paginated)
GET   /api/v1/cases/{case_id}/processes/{process_id}/timers           -> WorkflowProcessTimerList
GET   /api/v1/cases/{case_id}/processes/{process_id}/holds            -> WorkflowProcessHoldList
GET   /api/v1/cases/{case_id}/processes/{process_id}/related          -> WorkflowProcessRelatedList
GET   /api/v1/cases/{case_id}/processes/{process_id}/provenance       -> ProvenanceRecordPage        (cursor-paginated)
GET   /api/v1/cases/{case_id}/processes/{process_id}/custody           -> CustodyReceipt
POST  /api/v1/cases/{case_id}/processes/{process_id}/inputs           -> EventSubmissionResponse     (Idempotency-Key REQUIRED)
POST  /api/v1/cases/{case_id}/processes/{process_id}/drain            -> RuntimeDrainStepList
POST  /api/v1/cases/{case_id}/processes/{process_id}/suspend          -> WorkflowProcess                (Idempotency-Key REQUIRED)
POST  /api/v1/cases/{case_id}/processes/{process_id}/resume           -> WorkflowProcess                (Idempotency-Key REQUIRED)
POST  /api/v1/cases/{case_id}/processes/{process_id}/terminate        -> WorkflowProcess                (Idempotency-Key REQUIRED)
POST  /api/v1/cases/{case_id}/processes/{process_id}/migrate          -> MigrationResult             (Idempotency-Key REQUIRED)
```

ADR 0093 dual-identity case/process bindings: the public OpenAPI snapshot documents `ProcessLifecycleRequest` (optional `reason` only; binding validation is authoritative before the lifecycle mutation) and `ProcessLifecycleResponse` `{ processId, caseLedgerId, lifecycleState }` for the suspend, resume, and terminate routes on that surface. Canonical full actor-bearing bodies remain `SuspendInstanceRequest`, `ResumeInstanceRequest`, and `TerminateInstanceRequest` as exported below.

`GET /api/v1/cases/{case_id}/processes` returns `WorkflowProcessList`, an unpaginated array of `WorkflowProcess` projections bound to the supplied durable case ledger. Case-scoped process listing is intentionally scoped by `{case_id}` rather than exposed as a directory-scale process browser; cursor pagination remains on unbounded subresources such as tasks, provenance, correspondence, and compensation.

`WorkflowProcessListOptions` and `WorkflowProcessPage` remain reserved directory-style models for a future process-directory route; the reference case-scoped list route does not consume them.

`POST /api/v1/cases/{case_id}/processes` accepts `WorkflowProcessCreateRequest { definitionUrl, definitionVersion?, processId?, tenant?, initialCaseState? }` and returns the created `WorkflowProcess` projection directly. `processId` MAY be omitted, in which case the server allocates a process TypeID bound to the supplied case ledger. The intake-handoff policy outcome envelope is not the wire shape of this reference-server route.

`WorkflowProcessCreateResponse` remains a reserved intake-handoff outcome envelope for adapters that expose `acceptIntakeHandoff` as a separate creation contract; the reference case-scoped process-create route does not return it.

`GET /api/v1/cases/{case_id}/processes/{process_id}` returns `WorkflowProcess` when no `?include=` is supplied and `WorkflowProcessWithIncludes` when one or more subresource literals are requested. The schemas are distinct shapes; clients discriminate on the presence of the `instance` envelope key.

### Lifecycle-control mutations

The five mutations below close the kernel S11.3 / S4.9 instance-operations gap identified in [`thoughts/specs/2026-05-05-api-coverage-kernel.md`](../../../thoughts/specs/2026-05-05-api-coverage-kernel.md) Top-3 #1 and #2. Each request body carries a REQUIRED `actorRef` per ADR 0082 D-9 — every mutation has a recorded actor — and a REQUIRED `Idempotency-Key` HTTP header per ADR 0082 D-16. Each fires the corresponding kernel Facts-tier provenance record (Kernel S11.3 column 4); URNs are surfaced on the response (or via `GET /api/v1/cases/{case_id}/processes/{process_id}/provenance`) so clients can dereference reasoning/counterfactual/narrative tier data via ADR 0082 D-5.

`POST /api/v1/cases/{case_id}/processes/{process_id}/inputs` accepts `EventSubmissionRequest { eventName, payload?, actorRef, occurredAt?, correlationKey? }` and returns `EventSubmissionResponse { evaluationResult, newLifecycleState, evaluatedAt, provenanceRecordId?, correlationGroupResult? }`. The kernel transition algorithm (Kernel S4.7) runs against the supplied event; the response carries `EvaluationResult` with the typed `mutations: CaseStateMutation[]` array (ADR 0082 D-6) — the kernel anti-design `caseStateMutations: Record<string, unknown>` is structurally inexpressible. Events whose name uses the kernel-reserved `$`-prefix family (Kernel S4.10) are rejected with `WOS-1422`; submissions to a non-`active` instance are rejected with `WOS-1409`. Fires a `stateTransition` (or `unmatchedEvent`) Facts-tier record.

`POST /api/v1/cases/{case_id}/processes/{process_id}/drain` drains queued workflow inputs for the bound process and returns `RuntimeDrainStepList`, an ordered array of `RuntimeDrainStep` summaries naming transition counts, provenance counts, created task identifiers, and emitted workflow events for each processed input.

**Correlation-group fan-out atomicity.** When the submitted event triggered a kernel correlation-group fan-out (Kernel S9.4 — `correlationKey` resolves a multi-instance group), the server runs the kernel transition algorithm against each related instance independently and reports the typed per-instance outcome on `correlationGroupResult: CorrelationGroupResult`. The contract is **best-effort-with-typed-failure**, NOT all-or-nothing: when the server hits 3 of 5 related instances and the 4th refuses (lifecycle-posture mismatch, payload validation against the related instance's workflow event schema, scope filter), the response carries `allSucceeded: false` and the per-instance failure surface on `perInstanceResults: PerInstanceCorrelationResult[]`. Each entry carries `processId`, closed-no-extension `status: succeeded | failed | skipped`, REQUIRED `failureCode: WosErrorCode` when `status == failed`, and optional `evaluatedAt`. Callers MUST inspect `perInstanceResults` to determine compensation when `allSucceeded == false` — silent partial-failure is structurally impossible because every related instance the server attempted is named, and a missing entry is itself a contract bug. (Cross-cite ADR 0070 cross-layer failure and compensation when ratified.) `correlationGroupResult` is absent when no fan-out occurred (single-instance evaluation only); the originating instance the request targeted is reported via `evaluationResult` and MAY also appear in `perInstanceResults` at server discretion — callers MUST NOT depend on its presence in the per-instance array.

`POST /api/v1/cases/{case_id}/processes/{process_id}/suspend` accepts `SuspendInstanceRequest { reason, actorRef, holdUntil? }` and returns the updated `WorkflowProcess` (now `lifecycleState == suspended`). `holdUntil` accepts an `ExpirableTimestamp` (RFC 3339 timestamp or the `never` sentinel per ADR 0082 D-10); the server uses it to surface an expected-resume timestamp on `WorkflowProcessHoldList` but does NOT auto-resume — `POST /resume` is still required (Kernel S11.3). Submissions to a non-`active` instance are rejected with `WOS-1409`. Fires an `instanceSuspended` Facts-tier record (see "Companion provenance kinds" below).

`POST /api/v1/cases/{case_id}/processes/{process_id}/resume` accepts `ResumeInstanceRequest { actorRef, justification? }` and returns the updated `WorkflowProcess` (now `lifecycleState == active`). Pending events queued during the suspension are evaluated in delivery order after the status flip (Kernel S11.5). Submissions to a non-`suspended` instance are rejected with `WOS-1409`. Fires an `instanceResumed` Facts-tier record.

`POST /api/v1/cases/{case_id}/processes/{process_id}/terminate` accepts `TerminateInstanceRequest { reason, terminationKind, actorRef }` and returns the updated `WorkflowProcess` (now `lifecycleState == terminated`, irreversible per Kernel S11.5). `TerminationKind` is closed-with-vendor-extension per ADR 0082 D-12 — reserved literals `policy-violation | applicant-withdrawn | duplicate | error | administrative` plus `^x-[a-z]+-` extensions. Submissions to a `completed` or already-`terminated` instance are rejected with `WOS-1409`. Fires an `instanceTerminated` Facts-tier record.

`POST /api/v1/cases/{case_id}/processes/{process_id}/migrate` accepts `MigrateInstanceRequest { targetDefinitionUrl?, targetDefinitionVersion, migrationMap, actorRef, justification }` and returns `MigrationResult { instance, migratedAt, instanceMigratedRecordId, migrationPinChangedRecordId, previousWorkflowVersion, newWorkflowVersion, previousWorkflowUrl?, newWorkflowUrl? }`. The route validates that `{process_id}` belongs to `{case_id}` before invoking migration; mismatch returns 404. `MigrationMap` mirrors the kernel four-key bag (Kernel S11.2 step 2) but exposes each operation as a typed array of `{path, ...}` objects (`FieldRename`, `FieldDefault`, `FieldCoercion`, plus a bare `FieldPath` array for removals) instead of an open key-value map so D-12 closed-taxonomy discipline holds at the API boundary. Migration is atomic: any step failure leaves the instance on its prior version (Kernel S11.2). Submissions to a `completed` or `terminated` instance are rejected with `WOS-1409`; a target definition lacking a state currently in the configuration is rejected with `WOS-1422` (kernel `stateNotFound`). Fires both `instanceMigrated` (Kernel S11.3) AND `migrationPinChanged` (ADR 0071 D-4 — the cross-layer provenance record kind anchoring the chain transition for offline verifier reconstruction) Facts-tier records; the response carries both URNs.

### Companion provenance kinds

Each lifecycle-control mutation produces at least one Facts-tier provenance record (Kernel S11.3 column 4). The required `FactsRecordKind` reserved literals below are inner `recordKind` names, not F-13 event-type literals:

| Endpoint | Record kind(s) | Status on `provenance.schema.json` |
|---|---|---|
| `POST /events` | `stateTransition` (or `unmatchedEvent`) | `stateTransition` already in the closed reserved set |
| `POST /suspend` | `instanceSuspended` | Emitted when the suspend mutation commits. |
| `POST /resume` | `instanceResumed` | Emitted when the resume mutation commits. |
| `POST /terminate` | `instanceTerminated` | Emitted when the terminate mutation commits. |
| `POST /migrate` | `instanceMigrated` + `migrationPinChanged` | `migrationPinChanged` already in the reserved set per ADR 0071 D-4; `instanceMigrated` already present |

The kernel-named operation results map to API-side response fields: `instanceMigrated` and `migrationPinChanged` URNs are first-class on `MigrationResult`; the others are recoverable via `GET /api/v1/cases/{case_id}/processes/{process_id}/provenance`. The `EventSubmissionResponse.provenanceRecordId` field surfaces the Facts-tier URN inline so callers running an event-driven UI loop don't have to round-trip the provenance subresource.

## Identifiers

- `WorkflowProcess.id`: `urn:wos:<typeid>` URN per ADR 0092 D-1. The namespace-specific string IS the TypeID. Strip `urn:wos:` to extract the canonical TypeID.
- `actorRef` fields use the canonical `ActorRef` URN (`actor:(human|service-account|workload|support):...`) hoisted to `_common.schema.json`.

## Pagination

`GET /api/v1/cases/{case_id}/processes/{process_id}/tasks` and `GET /api/v1/cases/{case_id}/processes/{process_id}/provenance` use cursor pagination per `api/pagination.schema.json`. Cursors are opaque, single-use within the issuing deploy. Cursor expiry returns `410 Gone` with `WOS-1410` (ADR 0082 D-7). No `total`, `page`, or `pageSize` echo.

The smaller subresources (`governance`, `timers`, `holds`, `related`, `custody`) are not paginated because the cardinality is bounded by definition (typically single-digit per case). `compensation` is cursor-paginated (D-7) because compensation logs may grow unbounded.

## Idempotency

Process-scoped lifecycle mutations require an `Idempotency-Key` HTTP header per ADR 0082 D-16: `POST /inputs`, `POST /suspend`, `POST /resume`, `POST /terminate`, and `POST /migrate`. Server retains the request/response pair for at least 24 hours keyed by `(idempotency-key, route, scope)`. Repeated identical requests within the retention window return the original response unchanged. A repeat with the same key but a different request body for the same route/scope is rejected with `WOS-1409` (idempotency-key conflict).

`GET` endpoints are idempotent by construction.

## Errors

All non-2xx responses use `application/problem+json` per ADR 0082 D-8 and `api/error.schema.json`. Domain-relevant codes from the registry:

- `WOS-1404`: workflow process does not exist or is not in the caller's scope.
- `WOS-1409`: state-transition mismatch — covers four lifecycle-control families: (a) `?include=` request when the instance is `terminated` and the subresource is unavailable; (b) event submission, suspend/resume/terminate/migrate against an instance whose `lifecycleState` does not admit the operation per Kernel S11.5 (e.g. `POST /resume` on an `active` instance, `POST /events` on a `completed` instance); (c) `Idempotency-Key` conflict — same key, different body, same route/scope; (d) `migrate` against an instance currently mid-migration.
- `WOS-1410`: cursor expired.
- `WOS-1422`: request failed schema validation. Lifecycle-control families: (a) invalid `definitionUrl`, unknown `initialCaseState` field, or `tenant` mismatch with `X-WOS-Tenant`; (b) `eventName` uses the kernel-reserved `$`-prefix family (Kernel S4.10) — those are processor-emitted only; (c) migration `targetDefinitionVersion` lacks a state currently in the configuration (kernel `stateNotFound` per Kernel S11.2); (d) `payload` fails the workflow's declared event-payload schema; (e) `terminationKind` carries a vendor extension whose prefix is unregistered for the tenant.
- `WOS-1503`: durable runtime backend unavailable.

The lifecycle-control surface introduces no new RFC 7807 codes — the existing registry at `error-registry.md` covers the failure space. If a follow-up spec discovers an irreducible new failure mode (for instance a kernel `stateNotFound` migration error that warrants its own code separate from `WOS-1422`), the registry edit is deferred and flagged in the closing report rather than landed inline.

## State-mutation events

`EvaluationResult` is the wire shape returned by `POST /api/v1/cases/{case_id}/processes/{process_id}/inputs` (wrapped in `EventSubmissionResponse`). Per ADR 0082 D-6, mutations are projected as a typed array `mutations: CaseStateMutation[]` with closed `mutationKind` and `verificationLevel` taxonomies. The kernel anti-design `caseStateMutations: Record<string, unknown>` from `case-portal/src/ports/types.ts` is structurally inexpressible: every mutation requires `fieldPath`, `newValue`, `mutationKind`, and `verificationLevel`, so an open-taxonomy mutation bag has no construction path through this contract.

`MutationKind` reserved literals (`agent-extracted | system-fetched | human-entered | human-corrected | computed | self-attested`) mirror the kernel `MutationSource` open enum at `wos-workflow.schema.json#/$defs/MutationSource` (`x-wos.mirror` annotation enables Gate 6 parity). `VerificationLevel` reserved literals (`independent | attested | corroborated | authoritative`) mirror `wos-workflow.schema.json#/$defs/VerificationLevel`. Both accept `^x-[a-z]+-` vendor extensions per ADR 0082 D-12.

## Kernel mirrors

The contract projects (does not redefine) the kernel runtime model. `x-wos.mirror` annotations enable the Gate 6 mirror-parity check:

| API definition | Kernel source | Mirror annotation |
|---|---|---|
| `LifecycleState` | `wos-workflow.schema.json#/$defs/InstanceStatus` (logical name; the kernel value lives at `wos-process.schema.json#/properties/status`) | `x-wos.mirror = "wos-workflow.schema.json#/$defs/InstanceStatus"` |
| `MutationKind` | `wos-workflow.schema.json#/$defs/MutationSource` | `x-wos.mirror = "wos-workflow.schema.json#/$defs/MutationSource"` |
| `VerificationLevel` | `wos-workflow.schema.json#/$defs/VerificationLevel` | `x-wos.mirror = "wos-workflow.schema.json#/$defs/VerificationLevel"` |
| `TimerEntry` | `wos-process.schema.json#/$defs/TimerState` | `x-wos.mirror = "wos-process.schema.json#/$defs/TimerState"` |
| `ImpactLevel` (cross-`$ref` to `task.schema.json#/$defs/ImpactLevel`) | `wos-workflow.schema.json#/$defs/ImpactLevel` | mirror annotation lives on `task.schema.json#/$defs/ImpactLevel`; per ADR 0082 D-14, the kernel/governance type is `$ref`d from its existing schema rather than redefined here |

`HoldEntry`, `DelegationEntry`, `ReviewState`, and `RelatedCaseLink` mirror governance / kernel concepts by judgment without an exact kernel `$def` (governance §11, §12, §S5.5; kernel S5.5) and so do not carry the annotation.

## Related-case links

`RelatedCaseLink` (Kernel S5.5; kernel:775) reconciles the previously-divergent kernel and API enums for `relationship`. The merged closed-with-vendor-extension taxonomy is `parent | child | sibling | predecessor | successor | appeals | appealed-by | related | supersedes` plus the `^x-[a-z]+-` vendor-extension seam. The `related` and `supersedes` literals were reserved on the kernel side and are now first-class on the API side.

Two new optional fields close the kernel gap:

- `bidirectional?: boolean` (default `false`): when true, a corresponding `RelatedCaseLink` exists on the peer case pointing back at this one. Symmetric kinds (`sibling`, `related`) typically set this `true`; directional kinds (`predecessor → successor`, `appeals → appealed-by`) typically leave it absent.
- `semanticLabel?: prose`: free-form prose label disambiguating the relationship — useful when several `related` links coexist on the same case and the operator needs to tell them apart in a UI.

## Non-Goals

- Provenance read API — covered by `provenance.schema.json` (D-15 step 3, landed).
- Task lifecycle — covered by `task.schema.json` (this same step 4).
- Governance authoring — `delegations` and `reviewState` are the case-projection slice; full delegation/review records live on the dedicated `governance` domain (D-15 step 5).
- Streaming or push notifications — out of scope per ADR 0082 D-16.
- `advanceTime` operation — Kernel S11.3 marks it testing-only; correctly absent from the public surface (per `2026-05-05-api-coverage-kernel.md` §11 verdict).
