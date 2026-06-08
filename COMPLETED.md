# WOS Completed

Archive of closed-out work items extracted from `TODO.md`. Active backlog and in-flight work continue to live in `TODO.md`; this file is append-only and is not read during planning.

---

## Session 2026-06-08 — Activation Criteria + Durable Obligations (ADR 0096, PR #4)

- [x] **Shared activation criteria + durable obligation policies** — full feature landed on branch across Phases 0–9. A reusable `ActivationCriteria` shape ("when does this become active?") and durable pending obligations (`ObligationPolicy` → `PendingObligation`; lifecycle `pending → satisfied/violated/cancelled/expired/bypassed`) with deadline timers, violation actions (`warn < escalate < fail < block`), and first-class provenance. Distinct from DCR zones, task SLAs, milestones, the deontic `Obligation`, and the policy-engine obligation; no FEL grammar change (FEL stays the local boolean predicate inside `where`, ADR 0096 D-1).
  - **Schema:** `$defs/ActivationCriteria`, `ObligationPolicy`, `ObligationDeadline`, `ObligationViolationAction`, `PolicyObligationHandling`; `governance.obligationPolicies[]` on `wos-workflow.schema.json`. Normative contract: Governance §16. Required for `rightsImpacting`/`safetyImpacting` workflows declaring obligation policies (processors fail closed otherwise).
  - **`wos-core`:** activation/obligation model + deterministic activation evaluator (trigger → actor → requiredData → `where` → deadline; short-circuit; no truthy coercion).
  - **`wos-lint`:** `ACT-001..010` registered (Tier 2, all `draft` — unit-test evidence). LINT-MATRIX T2 total reconciled to 80, grand total 128.
  - **`wos-events`:** seven obligation `ProvenanceKind` variants + witness + PROV-O/XES/OCEL export.
  - **`wos-runtime`:** obligation monitor wired into `drain_once` (activation/satisfaction/cancellation, pre-event violate-block, lazy deadline expiry, strictest-action gating, replay dedupe, count cap, fail-closed posture, bypass/extension authorizer).
  - **`wos-conformance`:** 13 `OBL-001..013` fixtures (incl. Phase-6 integrations: milestone/SLA/hold criteria, policy-engine materialization, AI bypass/independence). Authored, not yet executed — compilation + exact conformance traces are CI-gated (`rust-tests.yml` + `STACK_REPOS_TOKEN`); the standalone sandbox cannot compile Rust (fel-core + private siblings absent).
  - **Closing docs/gates:** `docs/activation-and-obligations.md` (authoring), `docs/obligation-examples.md` (worked traces), `docs/obligation-authoring-prompts.md` (LLM snippets), `docs/obligation-conformance.md` (WOS-GATE-2902 fixture↔§16 + rule↔check map), `docs/obligation-migration.md` (WOS-MIG-2602; explicit timer+guard+milestone topology remains valid). Changelog entries (governance + kernel), COMPATIBILITY-MATRIX + RELEASE-STREAMS claims-map note (composes under `$wosWorkflow@1.0 [governance(obligations)]`, no new stream/marker), WOS-IMPLEMENTATION-STATUS §5 rewritten planned → landed-on-branch, WOS-FEATURE-MATRIX rows 1.11–1.14 ⚪ → 🟦.
  - **Deferred (tracked in TODO.md):** CI verification (WOS-GATE-2903/2904), SLA/Hold `ActivationCriteria` runtime wiring, true business-day deadline expiry (WOS-OBL-TIME-1002 / TIME-1008), DCR per-activity activation gating (WOS-INTEG-DCR-1602), deadline-index perf (2801).

## Session 2026-05-06 — Scout-swarm validation + T4-TODO merge + ADR checklist refresh

- [x] **Scout-swarm validation** — 7 parallel wos-scout agents validated all uncompleted TODO.md items against HEAD. 4 stale paths corrected, 3 stale counts fixed, 2 stale gate descriptions updated, 1 factual error corrected (#70 `String` → `RuntimeError`), 3 duplicate items (Backlog #3/#4) removed, 1 new blocker surfaced (ADR 0064 orchestrator gap for #2), 3 material flaws in #7 plan documented. ADR 0066/0067 statuses confirmed as Proposed-but-de-facto-accepted (cluster-ratification gate satisfied; status flips pending).
- [x] **T4-TODO.md merged into TODO.md item #1** — all active cross-repo tracking consolidated into TODO.md. T4-TODO.md deleted (content merged 2026-05-06).
- [x] **ADR 0066 checklist refreshed** — items 1 (provenance) and 5 (export, 2/3 paths) marked `[x]`; "six" corrected to "seven" variants (+`Reinstated`); stale `schemas/kernel/wos-provenance-record.schema.json` path corrected to `wos-workflow.schema.json`; Trellis item reference fixed (`17` → `7`).
- [x] **ADR 0067 checklist refreshed** — item 1 (provenance + schema $defs) and §3 authoring surface marked `[x]`; Trellis item reference fixed (`18` → `8`); §5 export note added (all three export paths still empty for clock kinds).
- [x] **Duplicate items removed** — `AuthorizationAttestation` and `ADR 0066/0067 implementation` umbrella items removed from Backlog (tracked in Do-next #3-#5).
- [x] **#70 stale claim fixed** — `Result<_, String>` corrected to `Result<_, RuntimeError>` (the real surface is already typed; the gap is the subset classification, not the wrapper type).
- [x] **#7 plan revised** — fixture validation gap (`additionalProperties: false` on Advanced block), endpoint placement (`instance.md` not `governance.md`), scope expansion decision documented.

## Moved from TODO.md 2026-05-06 — Completed items cleanup

- [x] **PLN-0406 — `ProvenanceKind` Rust enum extension (14 variants) + match-arm fixes** — landed 2026-05-05. 14 new `ProvenanceKind` variants (`AutonomyEscalation`, `DriftAlert`, `LegalHoldPlaced`/`Released`/`DestructionRejected`, `ContinuationOfServicesActivated`, `InstanceSuspended`/`Resumed`/`Terminated`, `CircuitBreakerTripped`/`Reset`, `ShadowModeDivergence`, `DcrZoneViolation`, `ReportTimedOut`), match-arm fixes at `audit_tier.rs`/`mod.rs`/`tests.rs`, schema enum extended 43→~56 literals, typed payload sub-shapes in `provenance.schema.json`. Final residual from ADR 0082 closure.
- [x] **WOS-T2** — ADR-0060 cross-reference taxonomy revisit. Workflow Governance now uses `templateKey`, `noticeTemplateKey`, `notificationTemplateKey`, `escalationStepId`; G-063 enforces Notification Template keys; G-066 enforces `BreachPolicy.escalationStepId` resolution; Studio WOS types regenerated.
- [x] **WOS-T3** — `DurableRuntime` extraction + Temporal/Restate spike. Public backend-neutral trait, in-memory `WosRuntime` adapter, runtime module split, Restate selected as first production backend, Temporal deferred, tenant-scope contract recorded.
- [x] **ADR 0073 — case initiation and intake handoff** — Landed. Typed `IntakeHandoff` parser/classifier in `wos-formspec-binding`; runtime `accept_intake_handoff` seam in `wos-runtime`; default intake policies; durable replay/persistence; canonical runtime provenance emission; Runtime Companion normative algorithm; Trellis-backed verification vectors. Remaining: one parent-owned shared fixture bundle (convenience-only).
- [x] **#65d `crates/wos-mcp/NOTES.md`** — decision record for hand-rolled JSON-RPC vs `rust-mcp-sdk`. Landed Session 16.
- [x] **#67 `ProvenanceKind::ConfigurationWarning`** — variant + audit-tier=Facts + constructor + 4 unit tests. 4 MUST sites follow-up tracked separately. Landed Session 16.
- [x] **#68 Schema↔enum drift lint** — `check-recordkind-parity.py` walks all schemas, finds recordKind bindings, asserts ProvenanceKind parity. Wired into CI. Landed Session 16.
- [x] **#69 `wos-export` exhaustive variant test** — `prov_o.rs` and `xes.rs` smoke tests rewritten to enumerate all 101 ProvenanceKind variants. Landed Session 16.

---

## Session 2026-05-01 — Closed-vocabulary hardening review follow-ups

- [x] **`assurance_rank` IAL/AAL recognition** — `assurance_rank` now recognizes NIST SP 800-63 IAL/AAL labels (`ial1`/`aal1`=1, `ial2`/`aal2`=2, `ial3`/`aal3`=3); previously these returned 0 (unknown). Aligns runtime with Signature Profile §2.7 ("Identity binding is provider-neutral … records authentication method, provider reference, assurance strength"). Behavior change for any prior fixture that relied on IAL labels being unrecognized; SIG-008 (notary, very-high) and SIG-013 (email-otp, low vs standard) cover the new path.

- [x] **WS-042 / ADR 0083 reference-server migration slice** — `kernels` table keyed by `(url, version)`; bundle resolution returns the requested definition version; `POST /api/instances/:id/migrate` honors `Idempotency-Key` for successful replays; integration test `migrate_instance_via_http_cross_version_idempotency_key_replays_outcome` covers `1.0.0 → 1.1.0`.

- [x] **Delivery sidecar rename (`actorType` → `correspondenceRole`) — release-note hook** — Normative closure lives in `specs/sidecars/delivery.md` §4 / schema; consumers migrating from pre-rename payloads should search `correspondenceRole` in `wos-delivery.schema.json` and the parity plan **D1** checklist.

---

## Specs and schemas

- [x] Kernel spec (S4.2, S4.10, S9.2) — concurrency, cascade depth, async actions.
- [x] Governance spec (S6.2) — source authority ranking.
- [x] Runtime companion (S5.3, S10, S12, S14) — parallel provenance, convergence cap, EventQueue interface.
- [x] Formspec integration gaps — version pinning, changelog migration, semantic contracts.
- [x] LINT-MATRIX rule count reconciled (197 total; I-001 added in NB.2).
- [x] Kernel schema — `evaluationMode`, `maxRelationshipEventDepth`.
- [x] Governance schema — `scope`, `sourceAuthority`, `ruleId`.
- [x] Workflow Process schema — `pendingEvents`, `governanceState`, `volumeCounters`.

## Normative features (from IDEA_SCRATCH Shipped)

- [x] **Null behavior on deontic constraints** (formerly IDEA #11) — `nullBehavior` on Permission/Prohibition/Obligation with impact-level defaults. `ai-integration.md §4.2-4.5 + §5`; `NullBehavior` `$def`.
- [x] **Arazzo integration sequences** (formerly IDEA #14) — Multi-step API orchestration via Arazzo references. `integration.md §3.5`; fixtures `INT-ARAZZO-001..003`. (See NB.4.)
- [x] **Non-HTTP tool invocation** (formerly IDEA #15) — `tool` binding kind (`command-line`, `batch-file`, `database-procedure`, `graph-query`). `integration.md §3.6`; fixtures `INT-TOOL-001..002`. (See NB.4.)
- [x] **Assist Governance Proxy** (formerly IDEA #16) — Deontic constraint enforcement on Formspec Assist tool calls. `ai-integration.md §14`; schema `AssistGovernanceProxy`. Stabilizes with Assist layer upstream.

## wos-core and runtime capabilities

- [x] Typed deserialization — Kernel, Governance, AI fixtures round-trip.
- [x] Evaluator — deterministic algorithm from S2.
- [x] Host traits — nine interfaces in `traits/mod.rs`.
- [x] `instance.rs`, `explain.rs`.
- [x] Conformance harness wired to runtime (`WosRuntime` / evaluator path as landed).
- [x] WOS-T2 — ADR-0060 cross-reference taxonomy revisit: Workflow Governance now uses `templateKey`, `noticeTemplateKey`, `notificationTemplateKey`, and `escalationStepId`; stale `noticeTemplateRef` governance fixtures/runtime surfaces were removed; G-063 enforces Notification Template keys; G-066 enforces `BreachPolicy.escalationStepId` resolution within the same `TaskPattern`; Studio WOS types regenerated from schemas.
- [x] WOS-T3 — `DurableRuntime` extraction + Temporal/Restate spike: public backend-neutral trait, in-memory `WosRuntime` adapter, runtime module split (`tasks`, `actions`, `timers`, `provenance`, `support`, `drain`, `instance`, `durable_impl`), Restate selected as first production backend, Temporal deferred pending Rust workflow API stability, and tenant-scope contract recorded in `thoughts/reviews/2026-04-21-wos-t3-durable-runtime-temporal-restate-spike.md`.
- [x] T3 fixtures batches 1–17 (102) and batch 16 processor meta-rules.
- [x] Inline conformance documents — `run_fixture` and fixture parse checks support `documents.* = "inline"`.
- [x] Timer region scoping and tolerance validation.
- [x] `deontic.rs`, `autonomy.rs`, `confidence.rs`, `event_handler.rs`, `eval_mode.rs`, `explain.rs` behavior.

## wos-lint

- [x] T1/T2 on typed models (`KernelDocument`, `KernelCollections`).
- [x] Typed state-tree walks (replaced manual tag/event collection).
- [x] G-027 sub-delegation depth via typed models.
- [x] T1-TESTS (G-058, G-059, G-062, G-065), T1-K009, CM-001, T2-GAPS (G-060, G-063).
- [x] LINT-COVERAGE — 197 of 197 rules covered (see LINT-MATRIX.md; I-001 added in NB.2).

## Conformance harness hygiene

- [x] **CONF-META-MOVE** — Move `observe_proxy_behavior` / `observe_assist_governance_proxy` into `wos-core/src/proxy.rs`.
- [x] **CONF-AI050-DIFF** — `differential_check_passed` computed from actual severity + violation-id comparison instead of hard-coded `true`.
- [x] **CONF-AI004-EVIDENCE** — `observe_delegated_formspec_evaluation` sets `full_response_envelope_validated` from `validation_result.valid`.
- [x] **CONF-PROFILE-DEDUP** — `tests/profile_conformance.rs` now delegates to `run_profile_against_fixtures` in `meta.rs`.
- [x] **CONF-RUNTIME-POLICY** — Move deontic, autonomy, confidence, event-handler, and DCR fixture policy into `wos_runtime::ReferenceCompanionPolicy`; conformance only selects/configures it.
- [x] **CONF-RUNTIME-PROVENANCE** — Emit compensation, lifecycle/case separation, and history-cleared provenance from `wos-runtime` / `wos-core`; conformance asserts observed provenance instead of synthesizing it.
- [x] **CONF-EVENT-IDENTITY** — Runtime drain results report the processed event token; fixture draining no longer stops on event name alone.
- [x] **CONF-IDEMPOTENCY-SCOPE** — Scope reference companion idempotency tracking per instance.
- [x] **CONF-STORE-API** — Remove `InMemoryStore` from the conformance public API; engine uses `wos_runtime::InMemoryStore`.
- [x] **CONF-STUB-TESTS** — Document inline stub tests as harness verification, not spec behavior.
- [x] **CONF-BINDING-DOC** — Document `ConformanceBinding`: intentionally permissive, `compute_case_mutation` returns `None`.

## Documentation

- [x] `work-spec/README.md`, root `context.md` WOS section, `wos-core/README.md`, `WOS-IMPLEMENTATION-STATUS.md`.

## Conformance profiles

- [x] Governance Basic/Complete aggregate tests.
- [x] Agent Registration / Confidence Framework aggregate tests.

## SMT / static analysis

- [x] AG010 finite-domain AST analysis, `finiteDomainDeclarations` in schema/linter, FEL filter rejection.

## Formspec coprocessor

- [x] FEL `every`/`some` in Formspec core.
- [x] Runtime Companion S15 interface and reference in-memory runtime path.
- [x] `wos-formspec-binding` — adapter surface plus prefill, validation, and mapping tests.
- [x] S15.3 pin re-validation on replay paths — `wos-formspec-binding::FormspecBinding::revalidate_submission` recomputes pin equality fresh on every replay/audit/review call.

## Coprocessor version discipline (S15)

- [x] S15.1 — register `FormspecBinding` alongside `ConformanceBinding`; real binding path exercised in conformance (61132c1).
- [x] S15.2 — author S15 validation fixtures through real `wos-formspec-binding` path; all 6 fixtures green (b0f3306).
- [x] S15.3 — delete `ConformanceBinding`; pin re-validation enforced on replay paths (0283740 + 0a3c369). `StubValidator` retained for service-invocation contract validation (`contract_outcomes` fixture field), which is a separate code path from the task-binding adapter.

## Kernel/runtime semantics (KS)

- [x] KS.1 — DeepHistory + ShallowHistory state semantics with conformance fixtures (D1 depth-1, D2 depth-2 + parallel-exit, D3 depth-3); `wos-core` capture/restore (c78848c).
- [x] KS.2 — Milestone firing with pinned ordering (data write durable → `MilestoneFired` → reactive transitions evaluated); 5 conformance fixtures K-M-001 through K-M-005 (521bd54).

## Business calendar (BC)

- [x] BC.1 — Business Calendar SLA runtime integration: lazy deadline evaluation at check time, `calendarVersion` snapshot, `DidNotConverge` error on convergence failure; 4 fixtures G-S10-001 through G-S10-004 green (c93052f).

## Provenance export (PE)

- [x] PE.1 — `wos-export` crate: PROV-O JSON-LD (§5.3–5.6), XES XML (§6.3), OCEL 2.0 JSON (§6.4); `timestamp` added to `ProvenanceRecord`; 3 SP-EXPORT-* conformance fixtures green (9daf447, 7cedfae, d8fbcf0, 7cd3cd3, 3ed010e, bd4e52f, b55b67e). Known limitations: higher-tier PROV-O bundles (§5.4) not emitted; OCEL events link to instance object only (per-case-file-item E2O links deferred); SHACL validation out of scope.
- [x] PE.2 — `ProvenanceRecord` schema extension + full SP §5.3/§5.5/§6.3 emission (2026-04-16, branch `feat/provenance-export` at `0fb895d` — unmerged). Eight optional SP-mandated fields added to `ProvenanceRecord`: `audit_layer`, `actor_type`, `lifecycle_state`, `definition_version`, `inputs`, `outputs`, `input_digest`, `output_digest`. Runtime populates all eight at stamp time via new `populate_provenance_record_fields` helper (wired at all 9 append sites; 1:1 with `provenance_log.push`/`.extend` invariant verified). Exporters emit the full §5.3/§5.5/§6.3 mappings: PROV-O `prov:used`/`prov:wasGeneratedBy` Entity nodes, `wos:atLifecycleState`, `wos:definitionVersion`, §5.5 actor-type subclass pairs (`[prov:Person, wos:HumanAgent]` / `[prov:SoftwareAgent, wos:SystemAgent]` / `[prov:SoftwareAgent, wos:AIAgent]`); XES `org:group`, repeated-key `wos:input`/`wos:output`, trace-level `wos:definitionVersion`, `wos:lifecycleState`, per-event digests; OCEL uniform `eventTypes` schema + indexed `inputs.{i}`/`outputs.{i}` scalar attrs (OCEL 2.0 compliance — no array-valued attributes). §6.5 Facts-tier filter applied uniformly via shared `is_facts_tier` helper; exhaustive `audit_layer_for_kind` match (93/93 variants) compile-gates future tier additions. New SP-EXPORT-004 fixture locks the filter. SHA-256 digests via new `sha2` crate dep. 407 tests passing, zero TODO(spec-upstream) markers remaining. Four rounds of semi-formal code review; all findings addressed (da20e80, d33b3ef, 32e453f, d86709b + 10 findings-fix commits: 8f3583a, 8cf6802, 0357b26, 1c86299, 418c0f9, 5ee7291, 2809393, 0f2a4a0, b735923, 0fb895d). Known limitations remaining: higher-tier PROV-O bundle wrapping (§5.4 — requires export API redesign to accept tier-discriminated output); OCEL case-file-item objects + per-item E2O/O2O links (§6.4 — requires case state snapshot protocol); SHACL validation (needs RDF library dependency); `ActorKind::Agent` mapping (`actor_type = "agent"`) pending AI Integration agent-registry threading through runtime context. Follow-up plan at `thoughts/plans/2026-04-16-wos-provenance-record-schema-extension.md`.

## Integration Profile binding kinds (NB)

- [x] NB.1 — typed `IntegrationBindingKind` enum + `IntegrationBindingHandler` trait; replaced stringly-typed dispatch (f017910).
- [x] NB.2 — outputBinding RFC 9535 profile pinned (wildcard + slice; filter/recursive-descent rejected); lint rule I-001; spec §3.3.1 (e6e916d).
- [x] NB.3 — CloudEvents bindings (`event-emit`, `event-consume`, `callback`) with subject correlation `{processId}:{bindingId}:{invocationId}`; full envelope captured in provenance; 6 fixtures INT-EMIT/CONSUME/CALLBACK-001–003 (75c8b21).
- [x] NB.4 — Arazzo, tool, and policy-engine bindings; `PolicyDecision` normalized to `{decision, reasons, obligations}`; 7 fixtures INT-ARAZZO/TOOL/POLICY-001–004 (d79c02b).

## Security / architecture docs

- [x] Runtime S13 isolation conformance guidance.
- [x] AI-004 / AI-050 behavioral verification strategy (ARCH-AI004).

## Session 4 (2026-04-18) — wos-synth scaffold + §4.1 chain unblocking

- [x] **§5.4 wos-synth Tasks 1-6 scaffold** (`6409006` + review fixes `b824927`) — four-crate split: `wos-synth-core` (loop + `Prompter` trait + `ToolContext` trait + prompt templates + `DirectToolContext` stopgap), `wos-synth-mock` (deterministic test prompter), `wos-synth-anthropic` (streaming-callback Anthropic provider), `wos-synth-cli` (binary `wos-synth` with `generate` / `dry-run` / `explain`). DIP invariant verified empty `cargo tree -p wos-synth-core --edges normal | grep -E 'reqwest|tokio|anthropic'`. CLI `dry-run` produces a kernel doc that lints clean without touching the network. Plan Task 7 (synth-trace JSON Schema + drift test) deferred to follow-up. Review fixes: AnthropicPrompter `Arc::try_unwrap` → `mem::take` (no more discarded paid completions); `strip_fences` no-newline regression; `LintFinding.suggested_fix` + `related_docs` plumbing into repair prompt; ScriptedPrompter/Tools converted to VecDeque + pop_front; trace explain prints "unknown" instead of misleading 0/0/0 token totals; OverrideRecord orphan-`$def` annotated; `anyhow_lite` rationale documented.
- [x] **§4.1 NoticeTemplate reconciliation** (`dfd9189`) — dropped thin `NoticeTemplate` `$def` from `wos-due-process.schema.json`; rich `TemplateSection`-based shape in `wos-notification-template.schema.json` is canonical. Zero in-tree consumers. `noticeTemplateRef` (Governance §3.1) and `notificationTemplateRef` (Governance §12.2) both already routed through the Notification Template sidecar via lint rule G-063.
- [x] **§4.1 #23 OverrideRecord schema** (`62b1561` + pytest contract `b824927`) — typed `OverrideRecord` + `EvidenceReference` `$def`s in `wos-workflow-governance.schema.json` with 1:1 mapping to OverrideAuthority policy switches (`requireStructuredRationale` ↔ `rationale`, `requireAuthorityVerification` ↔ `authorityVerification`, `requireSupportingEvidence` ↔ `supportingEvidence`). Authority-verification typed via 4-variant `method` enum (`roleAssignment | delegationGrant | supervisorAttestation | externalAuthority`) + `actorId` + `verifiedAt`. Spec §7.3 prose links to typed shape. EvidenceReference enforces "MUST be locatable" structurally via `required: ["kind"]` + `anyOf: [{required: ["caseFieldPath"]}, {required: ["uri"]}]`. Pytest contract `tests/schemas/test_override_record_shape.py` (12 cases: 6 EvidenceReference + 6 OverrideRecord with parameterized missing-field rejection + empty-supporting-evidence rejection) added in the review-fix commit. OverrideRecord is intentionally orphan (shape catalog for runtime provenance) — annotated via `$comment`.
- [x] **§4.1 #31 Jurisdiction-aware business calendar selection** (`44ac44c`) — replaced "implementation-defined" §7.1 selection with deterministic 6-step algorithm via optional `appliesWhen` FEL on each Business Calendar (matches `DueProcess.scope` pattern). Multi-jurisdiction rights-impacting workflows (e.g., national benefits with one calendar per US state) now have a declarative selection mechanism. Timezone disagreement among applicable calendars is a configuration error — surfaces modelling mistakes at evaluation time instead of silently picking one timezone. Spec §7 gained 7.1 (selection algorithm), 7.2 (composition + timezone-error rule), 7.3 (worked multi-state example).
- [x] **§4.2 #29a Milestone trigger-mode spec-lag closure** (`64b03a5`) — `Milestone.triggerMode: writeSettled` (default-only enum, extensible) names the runtime KS.2 behavior in authoring-visible form. Spec §4.13 gained "Trigger semantics" paragraph naming the three runtime invariants: fire-after-settled-write, at-most-once-per-instance, lexicographic id ordering for deterministic provenance. Wos-core `Milestone` struct picked up the optional field with `serde(skip_serializing_if = "Option::is_none")` so existing fixtures roundtrip byte-identically. Unblocks IDEA #29b reactive milestone firing — which can now extend `triggerMode` cleanly.

## Session 5 (2026-04-19) — §4.2 #37 / #46 closeout

- [x] **§4.2 #37 Drift Monitor demotion policy binding** — `AlertThreshold.policyRef` binds Drift Monitor alerts to named Agent Config `DemotionRule.id` semantics. Added executable T3 fixtures and expected traces for `AI-AUTO-001-escalation-expiry-revocation` and `AI-AUTO-002-drift-alert-demotion`; registered both as Tested conformance rules; parity + runtime-engagement tests prove escalation-expiry emits `autonomyDemotion`, while drift-alert demotion emits `autonomyDemotion` + `driftReclassification` and reroutes through `escalated` to human review. `LINT-MATRIX.md` regenerated to 99 rules / 8 T3.
- [x] **§4.2 #46 Schema-prose enum alignment batch** — closed enum/prose drift in `wos-kernel.schema.json` and `wos-workflow-governance.schema.json`: `CaseRelationship.type` and `HoldPolicy.holdType` now accept standard values or `x-` vendor extensions; `AppealMechanism.reviewerConstraint` is required and uses the due-process independence vocabulary; `AppealMechanism.continuationScope` uses the due-process continuation vocabulary; duration fields are constrained to the runtime-supported ISO 8601 grammar; `DelegationScope.conditions` cites the shared FEL evaluation contract. Drift Monitor `AlertThreshold.policyRef` prose/schema binding is covered by the #37 conformance slice.

## Session 5 (2026-04-19) — §4.1 #24a Facts-tier input snapshot

- [x] **§4.1 #24a Mandatory Facts-Tier input snapshot** — Kernel §8.2.1 now requires `transitionTags` plus `caseFileSnapshot` on Facts-tier state-transition provenance for determination-tagged transitions. `FactsTierRecord` / `CaseFileSnapshot` schema `$defs` and pytest contracts lock the shape.
- [x] **Runtime support** — `wos-core` snapshots use RFC 8785 JCS canonicalization plus SHA-256. The lifecycle evaluator captures snapshots at the exact transition firing point and persists transition tags on the provenance record, so recursive `$join` determinations receive the case-file state current to that transition rather than a stale per-drain snapshot.
- [x] **Executable conformance coverage** — registered T3 rule `K-DET-001` and added `k-det-001-determination-snapshot.json`, asserting transition tags, snapshot value, canonical JSON, and digest. `LINT-MATRIX.md` regenerated to 100 rules / 9 T3.

## Session 6 (2026-04-20) — active closeout

- [x] **§5.4 Task 7 synth-trace schema + drift test** — `schemas/synth/wos-synth-trace.schema.json` now publishes the `SynthTrace` and `SynthOutcome` artifact contract for `wos-synth explain`. `crates/wos-synth-core/tests/trace_schema_drift.rs` validates representative `SynthTrace`, converged `SynthOutcome`, and unconverged `SynthOutcome` serde output against the published schema, including optional `conformance`, `path`, `suggested_fix`, and `related_docs` fields. Local verification: `cargo test -p wos-synth-core --test trace_schema_drift -- --nocapture` passed 3/3; provider-DIP invariant remains clean (`cargo tree -p wos-synth-core --edges normal | rg 'reqwest|tokio|anthropic'` returned no matches).
- [x] **§5.4 synth review follow-up** — semi-formal review found two adjacent behavioral gaps: `wos-synth explain` omitted per-iteration conformance verdicts, and `strip_fences` did not honor its own non-JSON language-tag contract. Fixed both with tests: `cargo test -p wos-synth-core strip_fences -- --nocapture` (7 passed) and `cargo test -p wos-synth-cli render_trace_includes_iteration_conformance -- --nocapture` (1 passed).
- [x] **§4.1 #2 Deterministic adverse-decision notice (dual-form)** — `ReferenceCompanionPolicy` now detects active `adverse-decision` transitions with `noticeRequired`, captures the pre-transition Facts-tier case-file snapshot, resolves the Notification Template sidecar by `noticeTemplateRef`, renders deterministic human-readable prose, and emits a `noticeSent` record with `data.source = "deterministic"`, `machineReadable.kind = "adverseDecisionNotice"`, `snapshotSha256`, transition metadata, appeal configuration, and template reference. Governance §3.2 and schema prose now state the deterministic assembly contract. G-002 uses inline governance + notification-template documents and asserts the deterministic artifact plus snapshot digest. Verification: `cargo test -p wos-conformance g002_notice_before_adverse -- --nocapture`; `python3 -m pytest tests/schemas/test_fixture_validity.py tests/schemas/test_meta_validity.py -q`.
- [x] **§4.2 #21 Extension registry (seams-only MVP)** (`3550fad`) — `schemas/registry/wos-extension-registry.schema.json` + `specs/registry/extension-registry.md` catalog the six kernel seams (§10.1 actor-extension, §10.2 contract-hook, §10.3 provenance-layer, §10.4 lifecycle-hook, §10.5 custody-hook, §10.6 vendor-extension) plus the Trellis custody shape. `RegistryEntry` `$def` carries lifecycle (draft/stable/deprecated/retired), composition (merge/replace/augment), `since` / `replacedBy` / `vendorPrefix`. Descriptive, not enforcement; closes the `custodyHook` prose-only escape.
- [x] **§4.2 #39 ContinuationPolicy normative linkage** (`eaa678d`) — `AppealMechanism.continuationPolicyRef` (optional, `x-lm.critical`) resolves to `ContinuationPolicy.id` (now REQUIRED). `continuationOfServices: true` with neither ref nor scope resolving is a configuration error. Governance §3.6 prose added; misconfiguration-vs-error distinction spelled out.
- [x] **§4.2 #37 Drift Monitor demotion policy binding** (`b077613`) — `AlertThreshold.policyRef` (optional, `x-lm.critical`) resolves to `DemotionRule.id` (now REQUIRED) in the Agent Config sidecar. Named rule's structured semantics take precedence over the `action` enum; unresolvable ref falls back with a provenance warning. Drift-monitor §1.4.1 prose added. Combined with session-5 AI-AUTO-001/002 fixtures, this closes the full §4.2 #37 slice.
- [x] **§4.3 #13 Verifiability test principle** (`31a0e21`) — Kernel §1.2 design-goal bullet + cross-refs in Governance §6.1 and AI Integration §1.2. Doc-only.
- [x] **§4.3 #57 Assurance schema `x-lm.critical` coverage** (`a1100fe`) — Annotations on `assuranceLevel`, `subjectContinuity.{reference,scope}`, `disclosurePosture`, `attestation.{subject,predicate,basis}`. The only schema in the suite with zero annotations now has them; `schema_doc_zero_regression` stays green.

## Session 7 (2026-04-20) — DRAFTS triage + §4.3 close + v0 spike + review pass (8 commits)

- [x] **§4.1 DRAFTS triage** (`0d17f9f`) — 13 historical kernel drafts (v0.x through v7 plus tier-spec ancestors and a schema snapshot) moved from `DRAFTS/` to `thoughts/archive/drafts/` with a README classifying each file (superseded kernel iterations / v7 reframe ancestors / tier-spec ancestors / schema snapshot). MD-INVENTORY §6 rewritten to point at the archive; IDEA_SCRATCH reference updated. Unblocks §4.1 #20 typed event meta-vocabulary.
- [x] **§4.3 #56 K-049 continuous-mode cycle detection** (`4fd32e3` + review Finding 1 fix `2c6a2e2`) — new module `crates/wos-lint/src/rules/continuous_mode.rs`: parses each transition guard via `fel-core`, collects `setData` write-paths from transition actions plus source-state `onExit` plus target-state `onEntry` (Kernel §4.7 execution sequence), builds a directed write→read graph keyed by a per-path writer index (O(writes × reads)), runs iterative-DFS cycle detection, emits a T2 warning when `evaluationMode: continuous`. Exports `simple_access_path_string` + `walk_expr` as `pub(super)` from `fel_analysis.rs`. Registered `Tested` with spec_ref `specs/companions/runtime.md#s10-3`. 7 unit tests (self-loop, 2-node cycle, compound-nested cycle, entry/exit cycle, event-driven skip, acyclic control, unparseable guard).
- [x] **§4.3 #12 Capability preconditions + AI-057** (`19ad643`) — added `Capability.preconditions: array of FEL strings` (with `x-lm.critical`) to `schemas/ai/wos-ai-integration.schema.json`; normative semantics in new spec §3.3.1 (all entries MUST evaluate to boolean `true`; unsatisfied → skip to fallback chain §8; provenance `outcome: preconditionNotSatisfied`; preconditions do not relax deontic constraints). Wos-core `Capability` struct picked up `preconditions: Vec<String>` with `serde(default)`. New AI-057 T2 error lint enforces FEL parse validity per entry; 3 unit tests. LINT-MATRIX regenerated to 102 rules / 11 Tested / 58 T2.
- [x] **v0 spike Tasks 4–5** (`f6320c2` + `a80e37d`) — Task 4 conformance smoke-test gate: after lint passes, wraps the synthesized kernel in a minimal inline `ConformanceFixture` (empty `event_sequence`, empty `expected_transitions`) and calls `wos_conformance::run_fixture`; one repair round granted; budget-aware; `SpikeError::ConformanceFailure` isolates conformance-phase failures. Task 5 retrospective at `thoughts/research/2026-04-20-wos-synth-v0-spike-findings.md` with plan propagations appended inline to `wos-synth-crate`, `wos-synthesis-benchmark`, `wos-mcp-crate` plans. Key findings: `wos-conformance` has no `run(&doc)` entry point (fixture wrapper required); `ToolContext` shipped without spike counter-example → provisional; structured repair-prompt with `rule_id` + `suggested_fix` + `spec_ref` recommended before `wos-bench` measures convergence; live Anthropic iteration counts (Q-V0-1..4) flagged as follow-up. Spike disposition: keep-with-deletion-horizon (2–3 months). 17 unit tests green.
- [x] **§4.3a K-049 / AI-057 review follow-ups filed + refined** (`64962ea` + `4ceddb7`) — background `semi-formal-code-review` agent ran over `0d17f9f` + `4fd32e3` + `19ad643`. Verdict APPROVE with 9 findings; Finding 1 (K-049 missing entry/exit actions) fixed in `2c6a2e2`; Findings 6/8/9 OBSERVATION-only. Remaining 4 filed as §4.3a items in TODO, then refined via parallel spec-expert + wos-expert consultations into six concrete work items: **#F2** structured `Vec<Segment>` path comparison under Core §3.6.4 reachability; **#F3a** K-049 message reword + `$continuous` fixture; **#F3b** ADR + rewrite `eval.rs:412-421` post-mutation re-scan to match Runtime §10.3; **#F4** AI-058 boolean-AST-root allowlist lint + upstream Formspec §3.8.1 normativity gap filing; **#F5a** kernel `$defs/ProvenanceOutcome` enum (closes both `preconditionNotSatisfied` and `convergenceCapReached` MUSTs in one schema change); **#F5b** AI schema `if/then` enforcement. Cross-cutting drift surfaced: `ProvenanceKind::ConvergenceCapReached` missing from `crates/wos-core/src/provenance.rs:44` despite being named as a `recordKind` in `runtime.md:517`.
- [x] **Validation at close** — `cargo test --workspace` (63 test binaries, 0 failures). SCHEMA-DOC-001 zero-regression gate passes. Python `python3 -m pytest tests/ -q` 121 passed / 11 skipped / 1 xfailed.

## Session 8 (2026-04-20) — 8-agent parallel dispatch (~23 commits)

Largest parallel dispatch to date. Three batches: (1) uncommitted session-6 work committed + review-finding fixes; (2) eight concurrent agents on disjoint file sets; (3) four concurrent semi-formal code reviews.

### §4.1 #2 Deterministic adverse-decision notice — commit-split of uncommitted session-6 work (4 commits)

- [x] **`02ca0c1` style(runtime): rustfmt import-sort + assert! macro wrap** — split rustfmt churn out of the semantic commit per review Finding 6.
- [x] **`a041433` feat(runtime): thread current_time_ms + now_iso through drain context** — adds `now_ms: u64` + `now_iso: String` to `RuntimeEventContext`, populated once per drain from `self.clock.now_ms()`. No silent-zero path; missing populates surface at compile time. Prerequisite for the adverse-decision emitter's deterministic timestamps.
- [x] **`25026dd` feat(runtime): deterministic adverse-decision notice emission (§4.1 #2)** — `ReferenceCompanionPolicy::deterministic_adverse_decision_notice_input` + `AdverseDecisionNoticeInput`. Digest `7c6c9f04…f8a749` verified via both Rust + Python JCS implementations. Schema `if/then` requires `noticeTemplateRef` when `noticeRequired: true` (closes F8). Resolver returns typed `NoticeTemplateResolution` enum; audit signal surfaces as `resolvedTemplateKey` / `templateResolution` on the emitted record (closes F4). Spec §3.2 enumerates "transition-firing-timestamp" as a determining input (closes F3). Fixture `initial_case_state` cleaned up to realistic pre-transition state (closes F7). Two new unit tests pin `humanReadable` byte-identity under a fixed clock (closes F2).
- [x] **`abe3c76` fix(synth): strip non-JSON fence language tags; render per-iteration conformance in explain** — §5.4 synth review follow-up: `strip_fence_language` heuristic + `render_trace` pure function.

### §4.3a K-049 / AI-057 review follow-ups — 5 of 6 closed (8 commits)

- [x] **#F3a K-049 message reword + `$continuous` fixture** (`e15bd80`) — diagnostic now spec-faithful; `$continuous`-event fixture added.
- [x] **#F4 AI-058 boolean-AST-root lint** (`8855591`) — `is_boolean_shaped(&Expr)` pub(super) in `fel_analysis.rs`; 3 unit tests.
- [x] **#F5a Kernel `$defs/ProvenanceOutcome`** (`2d890d3`) — open-enum with `preconditionNotSatisfied` + `convergenceCapReached` reserved, `^x-` vendor pattern; optional `outcome` on `FactsTierRecord`; Rust `ProvenanceKind::ConvergenceCapReached` variant. Closes both §3.3.1 and §10.3 MUSTs in one schema change.
- [x] **#F2 K-049 structured-path reachability** (`ee05cec`) — `Vec<Segment>` + `reaches()` per Core §3.6.4; 2 regression fixtures; 10 new tests (`normalize_setdata_path` helpers + cycle regressions).
- [x] **#F5b AI schema `if/then` preconditionNotSatisfied** (`ae3589f`) — `CapabilityInvocationRecord` $def enforces `outcome = "preconditionNotSatisfied"` when `data.invocationBlocked: true`.
- [x] **LINT-MATRIX regen** (`d46d172`) — 102 → 103 rules; T2 Tested 2 → 3 (AI-058 added); K-049 later promoted LoadBearing in `f03ca40` after F3b.
- **#F3b ADR 0059 drafted** (`fcd2c19`) — Runtime §10.3 conformance plan; 5 tasks, ~3-5 engineer-days; preconditions satisfied by F5a. Implementation deferred.

### §4.4 Release trains Tasks 1-3 (4 commits)

- [x] **`78283ae` docs(release-trains): stream path mapping (§4.4 Task 1)** — `RELEASE-STREAMS.md`: kernel / governance / ai / advanced with paths, cadence, stability; sidecar attribution (lint/conformance/rule-coverage follow kernel); tag convention.
- [x] **`2c53f62` docs(changelogs): four per-stream changelog files (§4.4 Task 2)** — seeded with stability commitments per stream (kernel/governance semver-strict, ai pre-1.0, advanced research).
- [x] **`49de6c0` docs(release-trains): COMPATIBILITY-MATRIX + README pointer (§4.4 Task 3)** — `COMPATIBILITY-MATRIX.md` with `1.0.x / 1.0.x / 0.5.x / 0.1.x` row, `x-` known-broken convention, vendor-claim pattern.
- [x] **`9aee9be` docs(todo): mark §4.4 as partial after Tasks 1-3** — TODO state updated.

### §4.4 #40 Task SLA authoring surface (3 commits)

- [x] **`8b466fa` feat(governance): Task SLA authoring schema** — four OPTIONAL properties on `TaskPattern` (`slaDefinitions`, `warningThresholds`, `breachPolicy`, `escalationChain`) + four supporting `$def`s.
- [x] **`bc5de5f` docs(governance): Task SLA authoring spec subsection** — §10.4 + §10.4.5 future-work lint deferrals.
- [x] **`130a51e` test(schemas): contract tests for Task SLA shape** — 27 parametrized tests + happy-path fixture.

### §4.4 #38 Assertion Library cross-document reference protocol (3 commits)

- [x] **`77695eb` feat(governance): Assertion Library cross-document reference shape** — `AssertionReference` / `AssertionInlineUse` / `AssertionUse` three-$def `oneOf` split.
- [x] **`f862d1f` docs(governance): Assertion Library cross-reference protocol** — new spec §2.3/§2.4 with resolution semantics + G-064 design sketch.
- [x] **`21e9195` test(schemas): AssertionReference shape contract** — 12 tests covering hybrid-mix rejection + URI validation + `assertionId` pattern.

### §4.6 #45 Sidecar normative-contract audit (1 commit)

- [x] **`9900e39` docs(reviews): sidecar normative-contract audit** — 9 sidecars audited against CONVENTIONS.md (Step 0 + Structure / Semantics / Composition rubric). Verdict: 3 KEEP / 3 MERGE / 3 RESHAPE / 0 RETIRE. Ratifies the §4.5 three-merge plan. Six open questions filed for user verdict.

### Plans + ADR (2 commits)

- [x] **`6cad36e` docs(plans): draft implementation plan for #20 typed event meta-vocabulary** — 9 sections, 10 ordered tasks, grep-verified fixture counts (185 files / 844 occurrences), four open questions (OQ1 `$join` + OQ4 vendor kinds are load-bearing blockers).
- [x] **`fcd2c19` docs(adr): continuous-mode post-mutation re-scan driver (F3b)** — ADR 0059. All preconditions satisfied; 5-task implementation plan; READY TO EXECUTE.

### Semi-formal review pass (4 parallel reviews)

- [x] **Review A — wos-lint cluster** (F3a + F4 + F2): APPROVE WITH FOLLOW-UPS. 1 WARNING (AI-058 allowlist drift — missing `every`/`some`/`boolean`, bogus `isBoolean`) + 1 NIT (guard-walker short-circuit regression test) + observations. Filed in TODO §4.3b as #F4a + #F2a.
- [x] **Review B — schema cluster** (F5a + F5b): APPROVE WITH FOLLOW-UPS. 3 WARNINGs: F5b's `CapabilityInvocationRecord` is orphan `$def` with no composer (#F5d); F5a Rust emission not wired (`ProvenanceRecord` lacks `outcome` field; runtime still emits `CaseStateMutation`) (#F5c, rolls into F3b Task 3); vendor-extension regex drift from lowercase-kebab convention (#F5e).
- [x] **Review D — #40 Task SLA**: APPROVE WITH FOLLOW-UPS. 2 WARNINGs + 2 NITs: `expectedDuration` `indefinite` semantics (#40a); `startEvent` pattern allows `$continuous` (#40b); `EscalationStep.id` drift (#40c); enum-rejection test gaps.
- [x] **Review H — #38 Assertion Library**: APPROVE WITH FOLLOW-UPS. 3 WARNINGs: stale `.llm.md` regen (#38a); TODO #38 text stale (fixed inline); "one-line $ref" adoption claim understated — adoption requires cross-schema `$ref` plumbing or duplicate $defs (#38b; G+H concur the §4.5 merge is the natural landing).

### Validation at close

Final state: `cargo test --workspace` 1002+ passed / 0 failed · SCHEMA-DOC-001 zero-regression gate green · `pytest tests/schemas/ -q` 171 passed / 11 skipped / 1 xfailed (+50 vs session 7). 103 LINT-MATRIX rules (AI-058 added). All eight parallel agents + all four parallel reviews landed on disjoint file sets without conflict — validates the parallel-agent dispatch discipline from `thoughts/practices/2026-04-17-parallel-agent-dispatch.md`.

## Session 11 (2026-04-22) — WOS-T4 Signature Profile WOS-side closeout

WOS-side completion contract archived from the earlier `T4-TODO.md` execution file:

1. WOS freezes the Signature Profile acceptance criteria and DocuSign common-case parity bar.
2. WOS lands ADR-0062 and the Signature Profile spec/schema surfaces.
3. WOS ships schema fixtures, generated Studio type bindings, and SIG-001..SIG-012 lint rules.
4. WOS emits `SignatureAffirmation` provenance with schema-constrained payload and custody append inclusion.
5. WOS runtime enforces sequential, parallel, routed, free-for-all, decline, void, expiry, reassignment, witness/counter-signature, and notary/in-person semantics.
6. WOS conformance proves the common signing patterns and missing-consent rejection.
7. WOS-side verification gates pass; remaining work is cross-repo Formspec, Trellis, and Studio closeout.

- [x] **Acceptance surface and ADR landed** — `TODO.md` now carries the exact `WOS-T4 -COMPLETE-` criteria; [ADR-0062](thoughts/adr/0062-signature-profile-workflow-semantics.md) locked the center-vs-adapter split, signer-role attachment through `actorExtension`, ADR-0060 naming, and the rejection list for kernel enum widening / opaque-vendor-ceremony encoding.
- [x] **Spec and schema landed** — [specs/profiles/signature.md](specs/profiles/signature.md) and [schemas/profiles/wos-signature-profile.schema.json](schemas/profiles/wos-signature-profile.schema.json) define the Signature Profile document, signer roles, signing flows, consent and identity-binding evidence, reminders, expiry, decline, void, reassignment, and `SignatureAffirmation` shape.
- [x] **Fixture and type surfaces landed** — schema fixtures `fixtures/profiles/signature-benefits-attestation.json`, `signature-parallel-countersignature.json`, and `signature-routed-notary.json`, Python contract tests, and generated Studio type bindings for `signature-profile` all landed.
- [x] **Lint slice landed** — SIG-001..SIG-012 now enforce profile-to-kernel/governance consistency, typed timer mapping, satisfiable `SignatureAffirmation` evidence inputs, and ADR-0060 naming discipline; `LINT-MATRIX.md` was updated accordingly.
- [x] **Runtime provenance landed** — `ProvenanceKind::SignatureAffirmation`, schema-constrained payload validation, Rust helper constructors, stable `recordKind: "signatureAffirmation"`, and custody append inclusion all landed in the WOS runtime surface.
- [x] **Runtime signing semantics landed** — WOS now enforces sequential, parallel, routed, and free-for-all signing plus decline, void, expiry, reassignment, witness/counter-signature, and notary/in-person flows.
- [x] **Conformance slice landed** — `SIG-001` through `SIG-012` cover sequential, parallel, routed, expiry, decline, reassignment, witness/counter-signature, notary/in-person authentication, missing-consent rejection, custody append inclusion, free-for-all completion, and void-path behavior.
- [x] **Planning surfaces updated to handoff state** — `T4-TODO.md` now carries only the remaining cross-repo Formspec, Trellis, Studio, and verification work; WOS-side execution detail moved into this archive.

### Validation at close

- [x] `cargo fmt --all`
- [x] `cargo check --workspace`
- [x] `cargo test -p wos-lint`
- [x] `cargo test -p wos-runtime --lib`
- [x] `cargo test -p wos-conformance --test signature_profile -- --nocapture`
- [x] `../.venv/bin/pytest tests/schemas -q`
- [x] `npm run types:check` in `studio/` *(now `case-portal/`; renamed 2026-05-02)*
- [x] `git diff --check`

## Session 16 (2026-04-28) — Low-hanging cluster execution

Six tickets cleared in scout-recommended order after Session 15's validation pass. Net change: +1 `ProvenanceKind` variant (`ConfigurationWarning`) bringing HEAD count to 101; 4 new wos-core unit tests; 2 wos-export smoke tests rewritten exhaustively; 1 new Python lint + CI wire; 2 schemas gain `^x-` `patternProperties` blocks; 1 new `NOTES.md`; 3 doc edits normalizing `CaseInitiationRequest` → `IntakeHandoff`.

- [x] **#69 — `wos-export` `camel_cases_all_record_kinds` → all variants.** Both `crates/wos-export/src/prov_o.rs:569-722` and `crates/wos-export/src/xes.rs:575-685` rewritten from hand-list of 3 to exhaustive enumeration of all 101 `ProvenanceKind` variants. Tests assert (a) every record produces one `prov:Activity` / XES event, (b) the export's `wos:actionType` / `concept:name` matches the variant's serde-camelCase round-trip verbatim, (c) every camelCase translation begins with a lowercase letter (prov_o.rs only). Mirrors the `audit_layer_for_kind_covers_every_variant` Finding-3 enumeration discipline. No exporter bugs surfaced; generic dispatch at `prov_o.rs:136` is correct for all variants. 44 wos-export tests pass.
- [x] **#68 — Schema↔enum drift lint.** New `work-spec/scripts/check-recordkind-parity.py` walks all `schemas/**/*.json`, finds every `$def` whose `properties.recordKind.const` (or single-element `enum`) pins a literal string, and asserts each literal maps to a `ProvenanceKind` variant under `serde(rename_all = "camelCase")`. Also walks `allOf[].if.properties.recordKind.const` to catch the `CapabilityInvocationRecord`-style if/then guard pattern. Reverse-direction (variants without schema binding) reports as informational; `--strict` upgrades to error. Wired into `work-spec/.github/workflows/schema-regression.yml` as a separate CI step alongside `check-canonical-seams.py`. Current state: 15 schema bindings against 101 variants; forward parity holds; 94 of 101 variants are runtime-emitted without a schema $def (informational, expected). Reverse-direction enforcement is an intentional follow-up if/when per-kind schema $defs proliferate.
- [x] **§4.1 — `x-` extension seam, 2 schemas.** `schemas/conformance/conformance-trace.schema.json` and `schemas/mcp/wos-mcp-tools.schema.json` gained top-level `patternProperties: { "^x-": { "$comment": "Vendor extensions per Kernel spec §10.6 (extensions escape hatch)" } }` blocks matching the precedent in `schemas/kernel/wos-kernel.schema.json:11`. Note: the mcp-tools schema already had an `x-tool-catalog` key at top level paired with `additionalProperties: false`, which would have rejected its own data — adding `patternProperties` aligns the schema with its own usage. JSON validity verified.
- [x] **#67 — `ProvenanceKind::ConfigurationWarning`.** Variant + audit-tier=Facts + `ConfigurationWarningInput<'a>` + `ProvenanceRecord::configuration_warning(input)` constructor + 4 unit tests (`unresolved_ref_subject_serializes_required_fields`, `render_failure_subject_omits_unresolved_ref`, `drops_context_keys_that_collide_with_required_fields`, `classifies_as_facts`). Carrier for the four spec MUSTs at `drift-monitor.md:77`, `workflow-governance.md:154`, `notification-template.md:199, 222`. `data.subject` discriminator selects the failure site (reserved literals: `drift-monitor.policyRef` | `governance.continuationPolicyRef` | `notification-template.key` | `notification-template.render`; vendor extensions via `x-` prefix). `unresolvedRef` optional (omitted for render-failure subject). Same shape as the Session 14 `CapabilityInvocation` closure. Module export updated. Test enumeration in `provenance/tests.rs` and the wos-export hand-lists (prov_o + xes) extended; assertion counts bumped 100 → 101. Per-variant `wos-runtime` emission wiring is its own follow-up — this slice closes the typed Rust path so future runtime sites can construct schema-conformant records.
- [x] **#65d — `crates/wos-mcp/NOTES.md` decision record.** New file extracts the hand-rolled JSON-RPC vs `rust-mcp-sdk` rationale from existing code comments at `Cargo.toml:13-21` (feature-analysis retraction, 2026-04-18) and `src/server.rs:1-5` (header retraction). Records: original rationale, retraction context, current shape, why hand-rolled is still in place, when to revisit (3 trigger conditions), and pointers to ADR 0065 production seam discipline (D-3) plus open follow-ups (#65e SDK migration, #65f real MCP client validation).
- [x] **ADR 0073 terminology normalization.** Three files edited; `CaseInitiationRequest` retired in favor of `IntakeHandoff`. Each occurrence preserved with parenthetical "(formerly `CaseInitiationRequest`; renamed via ADR 0073)" so historical references resolve. Files: `thoughts/adr/0059-unified-ledger-as-canonical-event-store.md:634` (parent), `thoughts/adr/0073-stack-case-initiation-and-intake-handoff.md:47`, `trellis/thoughts/formspec/adrs/0059-unified-ledger-as-canonical-event-store.md:635` (Trellis submodule). Trellis-side change requires submodule commit + parent submodule-pointer bump on commit.

**Validation:**

- [x] `cargo check --workspace` — green.
- [x] `cargo test -p wos-core --lib` — 70 passed (4 new for ConfigurationWarning).
- [x] `cargo test -p wos-export --lib` — 44 passed (2 exhaustive smoke tests rewritten).
- [x] `python3 scripts/check-recordkind-parity.py` — OK (15 bindings / 101 variants; forward parity holds).
- [x] `python3 scripts/check-canonical-seams.py` — OK (72 files; canonical six seams hold).

**Net change to backlog:**

- Hygiene #68 + #69 closed; ADR 0064 §4.1 closed; Behavioral / governance #67 closed; ADR 0065 #65d closed; TODO-STACK.md "ADR 0073 terminology normalization" closed.
- `K-EXT-001` lint-rule gap remains separately filed (visible in `LINT-MATRIX.md` jumps from K-005 → K-EXT-002; out of scope for §4.1's schema patches).
- Reverse-direction parity-lint enforcement (`check-recordkind-parity.py --strict`) remains an intentional follow-up; runtime ConfigurationWarning emission wiring at the four MUST sites also follows independently.

## Session 15 (2026-04-28) — Cross-stack-scout validation of 8 low-hanging candidates

After Session 13's audit + Session 14's typed-path closure, the user asked for a list of low-hanging tickets to chip away at. Listed 8 candidates and dispatched `formspec-specs:cross-stack-scout` to validate each against HEAD before execution. The scout's report surfaced material drift between TODO claims and reality.

**Validation findings:**

- [x] **Variant-count drift** — TODO Snapshot, audit doc, and COMPLETED.md Session 13 all cited "95 `ProvenanceKind` variants" — actual count at HEAD is **100** (`awk '/^pub enum ProvenanceKind/,/^}/' kind.rs | grep -c "^    [A-Z]"` confirms). Six sites corrected to "99 at audit time, 100 at HEAD" framing. The audit's *script* enumerated all 100 (hand-list verified by `comm` diff against the actual enum), so the structural finding (1 zero-emission variant, 1 missing schema-driven kind) holds — only the *prose count* was wrong.
- [x] **#68 rescoped (all schemas + bidirectional).** Original framing scanned only `wos-workflow.schema.json`; scout pointed out `wos-provenance-log.schema.json` and `wos-process.schema.json` also carry `recordKind` references and the lint should be bidirectional (every literal → variant AND every variant → literal binding) so dead variants like `TaskSkipped` get caught mechanically. Description updated.
- [x] **§4.1 rescoped (3 schemas → 2).** Third schema (`schemas/kernel/wos-custody-hook-encoding.schema.json`) was deleted in ADR 0076 promotion — confirmed by `ls schemas/kernel/`. Rescoped to `schemas/conformance/conformance-trace.schema.json` + `schemas/mcp/wos-mcp-tools.schema.json`. Adjacent gap surfaced: `K-EXT-001` (T1 unknown-non-`x-` lint) is referenced in `LINT-MATRIX.md` jumps but not landed (`K-005 → K-EXT-002` with K-EXT-001 missing) — flagged in the entry as a separate lint-rule gap, not part of #2's scope.
- [x] **Kernel-Basic LoadBearing declaration retired.** Scout found "Kernel-Basic" appears only as a name in `RELEASE-STREAMS.md:11` claims-map row — no profile definition file, no fixtures bound, no kernel-block selection. Real Cx is 5+ (write the profile artifact + fixture binding + conformance crate wiring before the LoadBearing flag, not "one-line declaration"). Verifiability-closure entry rewritten as `RESCOPED 2026-04-28 (LARGER-THAN-CLAIMED)`; re-file when the profile artifact actually exists.
- [x] **Stack-level ADR cross-check lint retired** (TODO-STACK.md). No ADR carries a structured `Cross-references` section; refs are inline narrative. The lint itself is trivial; the **work** is the convention — adding a structured `## Cross-references` block (or YAML frontmatter) to the ADR template, which is a stack-wide convention change requiring owner sign-off, not single-session. Re-file as a two-step (template revision → lint) once the convention is decided.
- [x] **#69 filed** — `wos-export` `camel_cases_all_record_kinds` test extension split out of Do-next #2's grouping. Scout confirmed it's independent of AI-runtime wiring (generic dispatch at `prov_o.rs:136` already correct; the smoke test at `prov_o.rs:569-583` just under-covers — hand-iterates 3 of 100 variants). Land before #68 so the exporter test catches any bugs that #68 would otherwise lock down.

**Net change:** zero code, six+ doc edits across TODO.md / COMPLETED.md / audit doc / TODO-STACK.md. Validated execution order for survivors: **#69** (exporter test extension, `[3/1/2]` 6) → **#68** (schema↔enum drift lint, rescoped, `[5/2/4]` 20) → **§4.1** (`x-` extension, rescoped to 2 schemas, `[6/3/3]` 18) → **#67** (ConfigurationWarning kind, `[4/3/3]` 12) → **#65d** (`wos-mcp/NOTES.md`, doc-only) → **ADR 0073 terminology** (3 files; Trellis-submodule cross-boundary commit required).

## Session 14 (2026-04-28) — Gap 1 typed-path closure (`ProvenanceKind::CapabilityInvocation`)

Closes the typed Rust slice of Do-next #2 from Session 13's audit. AI §3.3.1 + Kernel §8.2.2 require emission of `recordKind: "capabilityInvocation"` with `outcome: "preconditionNotSatisfied"` when a precondition blocks an invocation; `schemas/wos-workflow.schema.json` `$defs/CapabilityInvocationRecord` enforces the shape via `FactsTierRecord.allOf`, but the Rust enum had no matching variant — the typed Rust path could not fulfill the schema MUST.

- [x] **Variant** — `ProvenanceKind::CapabilityInvocation` added to `crates/wos-core/src/provenance/kind.rs:50` with full doc comment citing AI §3.3.1 + Kernel §8.2.2.
- [x] **Audit-tier classification** — `K::CapabilityInvocation => Self::Facts` arm added to the exhaustive `From<K>` match in `crates/wos-core/src/provenance/audit_tier.rs:60` (preserves the discipline that adding a variant forces a conscious tier decision).
- [x] **Input struct + constructor** — `CapabilityInvocationInput<'a>` and `ProvenanceRecord::capability_invocation(input)` in `crates/wos-core/src/provenance/record.rs:11-32` and `:399-432`. Constructor sets `outcome: "preconditionNotSatisfied"` when `invocation_blocked == true`, omits outcome otherwise. Required keys (`capabilityId`, `invocationBlocked`) protected against context override (constructor's args win).
- [x] **Module export** — `CapabilityInvocationInput` added to `crates/wos-core/src/provenance/mod.rs:27`.
- [x] **Test enumeration** — variant added to the exhaustive `all: &[ProvenanceKind]` list in `crates/wos-core/src/provenance/tests.rs:329` so the Finding-3 regression test covers it.
- [x] **Unit tests** — five in `crates/wos-core/src/provenance/tests.rs`: `capability_invocation_blocked_sets_precondition_outcome`, `capability_invocation_permitted_omits_outcome`, `capability_invocation_drops_context_keys_that_collide_with_required_fields`, `capability_invocation_classifies_as_facts`, `capability_invocation_round_trips_through_serde` (last one added per review F3 item 1 — serializes a blocked record then deserializes and asserts `record_kind`, `outcome`, `actor_id` survive).
- [x] **Schema-validation coverage** — already provided by the Python suite at `tests/schemas/test_capability_invocation_record.py` (six if/then cases including blocked-with-correct-outcome, blocked-missing-outcome rejected, blocked-wrong-outcome rejected, permitted-without-outcome accepted, absent `invocationBlocked` not required to carry outcome, non-`capabilityInvocation` kind not required to carry outcome). Combined with the Rust unit tests, this proves: typed Rust constructor → JSON shape → schema validation round-trip is sound.

**Validation:**

- [x] `cargo check --workspace` — green.
- [x] `cargo test -p wos-core --lib` — 66 passed (5 new, including the review F3 item 1 round-trip test).
- [x] `python3 -m pytest tests/schemas/test_capability_invocation_record.py tests/schemas/test_provenance_outcome_literal_agreement.py tests/schemas/test_intake_provenance_records.py -q` — 16 passed.
- [x] `git diff --check` — clean.

**Code review (`formspec-specs:wos-scout`, semi-formal-code-review, 2026-04-28):** verdict **Land it**. Drift survey clean — every `recordKind` `const` literal across `wos-workflow.schema.json` and `schemas/kernel/wos-provenance-record.schema.json` has a matching `ProvenanceKind` variant. Confirmed correct: constructor invariant (blocked → outcome="preconditionNotSatisfied", unblocked → outcome omitted via `skip_serializing_if`); audit-tier=Facts placement; naming consistency; doc-comment correctness. Two review-driven additions landed in-session: **F1** doc grammar fix on `CapabilityInvocationInput.context` plus collision-policy rationale ("agent declaration is the source of truth; context is untrusted scratch and MUST NOT overwrite the schema-required discriminators that drive the if/then guard"); **F3 item 1** serde round-trip test (`capability_invocation_round_trips_through_serde`).

**Still open (carried into Do-next #2):** (a) AI-runtime invocation seam wiring — AI §3.3.1 steps 1-3 specify precondition evaluation; no runtime path actually evaluates `Capability.preconditions` today (the field exists at `crates/wos-core/src/model/ai.rs:189` but is never read by event_handler / runtime). Wiring requires AI-runtime architecture decisions. (b) `wos-export` `camel_cases_all_record_kinds` smoke-test extension (review F3 item 2). (c) JSON conformance fixture pair, co-lands with (a) (review F3 item 3). (d) Ergonomic constructor variant if call-site count justifies (review F2). All four grouped under `TODO.md` Do-next **#2** with `[6 / 5 / 3]` (18); gated on AI-runtime invocation seam design. Score dropped vs the original Gap-1 entry (24 → 18) because the cross-stack typed-path debt is gone — the remaining work is local AI-runtime wiring, not architectural drift.

## Session 13 (2026-04-28) — Provenance emission completeness audit

`TODO.md` Do-next item *Provenance emission completeness audit* closed. Audit doc at [`thoughts/audit-2026-04-28-provenance-emission-completeness.md`](thoughts/audit-2026-04-28-provenance-emission-completeness.md). Method: enumerated all 99 `ProvenanceKind` variants from `crates/wos-core/src/provenance/kind.rs` (pre-Session-14 — count is 100 at HEAD post-`CapabilityInvocation` addition); counted live emission sites in `crates/wos-{runtime,formspec-binding}/src` plus `crates/wos-core/src` (excluding the provenance module's own definitions, audit-tier mapping, and tests); cross-checked every `recordKind: "..."` mention from `specs/**/*.md` against the enum; spot-checked `MUST.*(emit|produce|record|append).*provenance` clauses against emission paths.

**Findings (3 gaps total, all filed):**

- [x] **Gap 1 — `CapabilityInvocation` recordKind has no Rust variant or emission path (HIGH).** Filed as Do-next item **#2** (`[6 / 3 / 4]` — 24). Schema enforces shape via `$defs/CapabilityInvocationRecord` + `FactsTierRecord.allOf`; spec MUST at AI §3.3.1 + Kernel §8.2.2 + outcome literal `preconditionNotSatisfied`; Rust enum has no matching variant. Typed Rust path cannot fulfill the schema MUST.
- [x] **Gap 2 — `TaskSkipped` variant defined but never emitted (MEDIUM).** Already tracked at backlog **#66e** (skip-path lifecycle in Runtime Companion §15 / Phase 11). No new TODO entry; this audit confirms the gap. Spec MUST at Runtime Companion `:863` + outcome row `:929` + Workflow Governance `:496`.
- [x] **Gap 3 — Configuration-warning emission discipline (LOW).** Filed as Behavioral / governance item **#67** (`[4 / 3 / 3]` — 12). Four spec MUSTs require provenance for unresolvable configuration references but bind no `recordKind` and the codebase emits nothing: `drift-monitor.md:77`, `workflow-governance.md:154`, `notification-template.md:199`, `notification-template.md:222`. Lean generic `ConfigurationWarning` with `data.subject` enum.

**Scope confirmation:**

- 99 `ProvenanceKind` variants at audit time (100 at HEAD); **98 with at least one live emission site**. Truly-zero count = 1 (`TaskSkipped`).
- 0 spec `recordKind:` literals without an enum variant EXCEPT `capabilityInvocation` (Gap 1).
- Variants showing `R+B = 0` but `wos-core ≥ 1` (e.g., `DeonticViolation`, `AppealFiled`, `OverrideRecorded`, `PipelineStageCompleted`, `DcrActivityExecuted`, `EquityAlert`, `VerificationReportProduced`) all resolve through `wos-core/src/event_handler.rs` plus `deontic.rs` / `proxy.rs` — not gaps; the runtime composes the core emission rather than re-emitting at a separate site.

**Meta-finding:** Gap 1 was structurally avoidable — the ADR 0076 promotion pass (2026-04-26) added `$defs/CapabilityInvocationRecord` with a fixed `recordKind: "capabilityInvocation"` literal but no CI gate ensured the matching `ProvenanceKind` variant landed. Same failure mode could recur for any future record-shape $def. Filed Hygiene **#68** to add a schema↔enum parity gate (walk `wos-workflow.schema.json` $defs whose `recordKind` is pinned to a literal; assert each maps to a `ProvenanceKind` variant under `serde(rename_all = "camelCase")`). Same pattern as `scripts/check-canonical-seams.py` for ADR 0077.

**Net change:** `TODO.md` Do-next item retired; three new TODO items filed (Gap 1 in Do-next #2; Gap 3 in Behavioral / governance #67; Meta-finding in Hygiene #68). Existing backlog **#66e** cross-referenced. Verifiability-closure section's duplicate audit entry collapsed to a closed reference. Snapshot last-audited line at top of TODO.md extended with the audit summary. No code change.

## Session 12 (2026-04-28) — Custody / Assurance governance prose gap closeout

`TODO.md` Do-next item #2 retired. The 2026-04-24 audit ([`thoughts/audit-2026-04-24-wos-spec-thoughts-plans.md`](thoughts/audit-2026-04-24-wos-spec-thoughts-plans.md) verdict #17) flagged Governance §2.9 / §4.9 / §7.15 prose + schema as missing while the feature matrix showed ✅ — drift between matrix and normative artifacts. Verification at HEAD shows all four sub-tasks already landed on 2026-04-15:

- [x] **§2.9 Schema upgrade** prose at `specs/governance/workflow-governance.md:88-100` (commit `2f50812`); schema `schemaUpgrade` block at `schemas/governance/wos-workflow-governance.schema.json:200-210` declares `priorVersion`, `newVersion`, `migrationMechanism` (`formspec-changelog` | `custom-map` | `declared-equivalence`), and `scope` (`instance` | `workflow` | `tenant`) — all four required.
- [x] **§4.9 Quorum-based delegation** prose at `workflow-governance.md:663-678` (commit `2a5d89b`); schema `quorumCount` + `quorumPool` at `wos-workflow-governance.schema.json:1427-1436` with `quorumCount → ["quorumPool"]` `dependentRequired` at `:1337`.
- [x] **§7.15 Legal hold** prose at `workflow-governance.md:734-749` (commit `5d86839`); schema `holdType: "legal-hold"` enum at `wos-workflow-governance.schema.json:1545,1557` with conditional carve-out (`legal-hold` does NOT require `resumeTrigger` / `timeoutAction`) at `:1525-1530`.
- [x] **Legal-sufficiency cross-ref to Assurance §6** at `workflow-governance.md:46` — front-matter MUST cites `WOS Assurance Layer §6`.
- [x] **Invariant 6 dedup** completed in Plan 3 (2026-04-15). `specs/assurance/assurance.md` §4.4 declares the normative home and explicitly forbids restatement: *"Other specifications in the WOS family, and bindings such as Trellis, MUST reference this section rather than restating the invariant."* Active Trellis specs (`trellis/specs/cross-reference-map.md`, `trellis/specs/trellis-requirements-matrix.md`) only cross-reference; the legacy ULCR-112 row was removed from the Trellis matrix in Plan 3 with explicit upstream-owned notes; remaining `Invariant 6` hits are appropriate citations (PARITY.md, server `assurance_service.rs`, studio types) not redefinitions.

The audit was correct on the date it ran against an older HEAD, but the §2.9/§4.9/§7.15 work landed nine days before the audit was written. Net change to `TODO.md`: Do-next item #2 removed; subsequent items renumbered #3..#6 → #2..#5. No code change needed.

## Session 10 (2026-04-21) — WOS-T1 custodyHook execution closeout

Completion contract archived from the former `T1-TODO.md` execution file:

1. WOS publishes the reserved TypeID family prefixes and `wos.*` event-type ownership in the extension-registry surface.
2. WOS workflow processes and authored records mint TypeIDs at authoring time rather than deriving them from log position or storage order.
3. Record-family schemas reject malformed `caseId` / `id` values at authoring time.
4. The binding mechanically converts schema-valid WOS JSON records into dCBOR using the ADR-0061 encoding table and rejection list.
5. The WOS runtime emits the narrow four-field append input: `caseId`, `recordId`, `eventType`, `record`.
6. The WOS-side idempotency source tuple is exactly `(caseId, recordId)` with domain tag `trellis-wos-idempotency-v1`.
7. The runtime receipt surface is narrowed to `CustodyAppendReceipt { canonical_event_hash }`, and WOS stamps that hash into the first downstream consumer path.
8. Trellis fixture `append/010-wos-custody-hook-state-transition` and Trellis Operational Companion §24.9 match the final emitted shape.
9. Round-trip fixture corpora byte-match in Rust and Python for every WOS record family crossing `custodyHook`.

- [x] **TypeID minting landed in code** — added stack-local [typeid.rs](crates/wos-core/src/typeid.rs) with UUIDv7/Crockford `{tenant}_{type}_{uuidv7_base32}` minting + validation; `ProvenanceRecord` now mints `prov` IDs at authoring time; `WorkflowProcess` now mints `case` IDs and preserves legacy request aliases for runtime compatibility.
- [x] **Kernel provenance records gained durable custody citation** — `ProvenanceRecord` now carries `canonicalEventHash`; runtime added `apply_custody_receipt(...)` and stamps `CustodyAppendReceipt { canonical_event_hash }` onto persisted provenance by `recordId`.
- [x] **`wos-runtime::custody` rewritten to ADR-0061** — removed the superseded JCS/wide-shape append surface; live runtime now emits the narrow four-field append input (`caseId`, `recordId`, `eventType`, `record`) with dCBOR-authored bytes, base64 JSON host serialization, canonical CBOR map ordering, oversize rejection, and 2-tuple idempotency `(caseId, recordId)`.
- [x] **Spec / schema / registry surfaces aligned** — [specs/kernel/custody-hook-encoding.md](specs/kernel/custody-hook-encoding.md), [schemas/kernel/wos-custody-hook-encoding.schema.json](schemas/kernel/wos-custody-hook-encoding.schema.json), registry ownership metadata, case/provenance/governance TypeID patterns, and the Trellis Operational Companion §24.9 now all point at the accepted ADR-0061 contract.
- [x] **Planning surfaces updated to closure state** — `TODO.md` now dropped the live WOS-T1 row; this session archive carries the former T1 closeout contract and verification log.

### Validation at close

- [x] `cargo test -p wos-core --lib`
- [x] `cargo test -p wos-runtime --lib`
- [x] `cargo test -p wos-export --lib`
- [x] `cargo test -p wos-conformance --lib`
- [x] `pytest tests/schemas/test_custody_hook_encoding.py tests/schemas/test_extension_registry.py tests/schemas/test_facts_tier_snapshot.py tests/schemas/test_facts_tier_outcome.py tests/schemas/test_capability_invocation_record.py tests/schemas/test_override_record_shape.py tests/schemas/test_case_instance_typeid.py tests/schemas/test_meta_validity.py`
- [x] `npm run docs:check`

## Session 9 (2026-04-20) — 4-agent parallel sweep of review follow-ups (19 commits)

All §4.3b review follow-ups closed in a single 4-agent parallel dispatch. Disjoint file scopes kept conflict surface minimal despite shared-crate touches on `wos-core/src/provenance.rs`.

### Review A follow-ups — wos-lint cluster (6 commits)

- [x] **#F4a AI-058 allowlist drift** (`2d3132f` + `b0ec6e0`) — `is_boolean_shaped`'s boolean-returning builtin allowlist now derives from `fel_core::builtin_function_catalog()` via `std::sync::LazyLock<HashSet<&'static str>>`, filtering on signatures ending `→ boolean`. Adds `every`, `some`, `boolean` (three real builtins previously missing → false positives on valid FEL); removes bogus `isBoolean` (was never a registered builtin). Four new tests pin each branch.
- [x] **#F2a Guard-walker short-circuit regression test** (`196346c`) — direct test `k049_guard_walker_short_circuit_prevents_spurious_cycle` with inline rationale naming the `PostfixAccess(FieldRef("caseFile", []), [Dot("input")])` parse shape that motivated the short-circuit. Previously only indirect-tested via `k049_ignores_acyclic_continuous_kernel`.
- [x] **Review A Finding 4 — `NullCoalesce` admission** (`10bd3af`) — `is_boolean_shaped` now recurses into `Expr::NullCoalesce { left, right }` (both sides must be boolean-shaped). Closes a false-positive class for `$flag ?? true` precondition patterns.
- [x] **Review A Finding 5 — adversarial `normalize_setdata_path` coverage** (`6b448df`) — new test `normalize_adversarial_inputs_degrade_to_single_dot` covers 7 edge cases (`""`, `"."`, `"foo[]"`, `"foo[-1]"`, `"foo[[0]]"`, `"foo[a]"`, `"foo[ 1 ]"`). `[*]` deliberately excluded since the normalizer handles it as `[Wildcard]` (documented inline).
- [x] **Review A Findings 3/6/8 — narrative cleanup** (`45a97f3`) — `extract_read_paths` docstring names the PostfixAccess parse shape; `reaches()` gains a symmetry comment + regression test; module docstring normalizes "100-cycle cap" / "convergence cap" phrasing to match the emitted diagnostic. Zero behavior change; diagnostic test still passes.
- wos-lint unit tests: 88 → 97 (+9).

### Review B follow-ups — schema cluster (6 commits)

- [x] **Review B Finding 4 — `ProvenanceOutcome` shape simplification** (`3f4bce9`) — rework to match sibling open-enum convention at `wos-kernel.schema.json:803-818`: top-level `type: string` + bare `oneOf: [{enum}, {pattern}]`. No leaf-level duplication. Commit bundled the F5e vendor-regex change to avoid a transient-invalid intermediate shape.
- [x] **#F5d F5b composition story** (`504a48b` + `2e853b7`) — `CapabilityInvocationRecord` $def moved from `schemas/ai/wos-ai-integration.schema.json` to `schemas/kernel/wos-provenance-record.schema.json`. Kernel provenance schema is now the single validation point for the §3.3.1 MUST. AI schema retains only a `$comment` pointer. Spec prose (AI §3.3.1 + Kernel §8.2.2) updated to describe the moved enforcement accurately.
- [x] **#F5e Vendor-extension regex normalization** (`37347a5`, regression test only — regex flip itself landed in `3f4bce9`) — `^x-[a-zA-Z][a-zA-Z0-9-]*$` → `^x-[a-z][a-z0-9-]*$`, matching the established lowercase-kebab convention elsewhere. `x-Acme-Foo` now correctly rejected.
- [x] **#F5c F5a runtime-emission wiring / F3b Task 3** (`a683c03`) — `ProvenanceRecord` gained `pub outcome: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]` (roundtrip-safe on existing fixtures). `eval_mode.rs` convergence-cap emission flipped from `ProvenanceKind::CaseStateMutation` + `data.convergenceCapReached: true` to the dedicated `ProvenanceKind::ConvergenceCapReached` variant with `outcome: Some("convergenceCapReached")` and clean `data` payload. **ADR 0059 Task 3 is complete** — F3b remaining scope shrinks to 4 tasks / ~2-3 engineer-days. Crossed the `wos-runtime` fence with mechanical `outcome: None` additions at ~29 literal-constructor sites plus spillover in `wos-core/{explain,event_handler,deontic,autonomy,confidence}` and `wos-conformance`. New regression test `convergence_cap_emits_dedicated_kind_and_outcome_field`.
- [x] **Review B Findings 5 + 6 — edge-case coverage + literal agreement** (`0eb14b2`) — 4 new Python contract tests: `test_outcome_rejects_bare_x_prefix`, `test_absent_invocation_blocked_not_required_outcome`, `test_non_capability_record_kind_with_blocked_flag_not_required_outcome`, plus a cross-schema grep-based smoke test that `preconditionNotSatisfied` agrees across the (now post-move) kernel $def and its `if/then` const. Finding 6 discharge: `const` retained for simplicity; agreement pinned by test.
- `cargo test --workspace`: 1006 → 1012 (+6 net across wos-core + the new regression).

### Review D follow-ups — #40 Task SLA (4 commits)

- [x] **#40a `expectedDuration` rejects `"indefinite"`** (`8b32330`) — drop `indefinite|` branch from `SlaDefinition.expectedDuration` regex; now matches sibling `WarningThreshold.beforeBreach` + `EscalationStep.gracePeriod` duration-only regex. Prose + examples updated; one new negative test. Semantic justification: "indefinite SLA" is an oxymoron since `warningThresholds` + `breachPolicy` have nothing to fire against.
- [x] **#40b `startEvent` kernel event-name pattern** (`d22038c`) — `"pattern": "^[a-zA-Z][a-zA-Z0-9_-]*$"` added. Rejects `$`-prefixed reserved names (`$continuous`, `$join`, `$timeout.*`), empty strings, whitespace. Two new negative tests.
- [x] **#40c `EscalationStep.id` OPTIONAL + `escalationChainRef` contract** (`dea7786`) — added OPTIONAL `id: string` with kernel identifier pattern; `BreachPolicy.escalationChainRef` description now concretely references how level-based vs id-based resolution works. Fixture gained `id: "supervisor"` on step 2.
- [x] **Review D Findings 3 + 4 — calendarRef convention comments + enum negatives** (`62c43cc`) — confirmed `HoldPolicy.notificationTemplateRef` precedent (plain `type: string` for in-document keys, `format: uri` for sidecar URI). Added one-line convention comments to `calendarRef` / `WarningThreshold.templateRef` / `BreachPolicy.templateRef` / `escalationChainRef`. 4 new enum negative tests (`calendarType`, `startAt`, `onExhaustion`, `timeoutPolicy.onRepeatedBreach`).
- Task SLA tests: 27 → 35 (+8).

### Review H follow-ups — #38 Assertion Library (3 commits)

- [x] **#38b + Review H F4/F5/F9 — adoption path + dual-role clarifications** (`c746e9c`) — `specs/governance/assertion-library.md` §2 rewritten with honest adoption story: adopting `AssertionUse` from a consumer schema requires either cross-schema URI `$ref` plumbing (untested territory) OR duplicating the three $defs OR the §4.5 merge which dissolves the choice. New paragraph §2.1 disambiguates `assertionId`'s dual role (inline-standalone vs. library-mirrored). G-064 check (c) tightened to "When an `assertionRef` resolves to a library body that carries its own `assertionId`, that `assertionId` MUST match the library `id`." §2 gained a forward-looking sentence on the §4.5 merge interaction.
- [x] **Review H Finding 7 — "Configuration error" glossary** (`2020c48`) — one-paragraph gloss at top of §2.2 defining "configuration error" as a load-time reject condition. Cross-linkable from any future sidecar spec.
- [x] **Review H Finding 8 — Edge-case negative tests** (`4b0e575`) — 3 new tests in `test_assertion_reference_shape.py`: `assertionRef: ""` rejected via `minLength: 1`; `assertionRef: null` rejected via type mismatch; `assertionRef: "#localFrag"` rejected via `format: uri` requiring absolute URI.
- **Review H Finding 1 (#38a)** — regen no-op (`npm run docs:generate` reported 3 updated artifacts but git saw no diff; `docs:check` was already exit-0 at session start because the 3 stale `.llm.md` files had been regenerated content-identically). No commit needed.
- AssertionReference tests: 12 → 15 (+3).

### Cross-agent coordination notes

- **Transient git churn** between Agents A and B on shared wos-core crate: one agent's `git reset` / `git stash` operations briefly touched the other's uncommitted work. Recovered cleanly via `git stash pop` + `git checkout HEAD --`; no scope-overlap damage. Parallel-agent dispatch on shared crates carries real friction — future sessions should sequence provenance.rs-touching work or introduce a coordination mutex.
- **F3b Task 3 landed ahead of F3b Tasks 1-2** — Agent B completed the emission wiring opportunistically while adding the `outcome` field. Order departed from the ADR's sequential plan but delivered the same end-state; ADR 0059 commit-message cross-reference notes Task 3 closed out-of-band.

### ADR 0059 F3b + Task 5 — Runtime §10.3 + K-049 LoadBearing (`bdf7063`, `f03ca40`)

- [x] **F3b continuous-mode post-mutation guard re-scan** (`bdf7063`) — `Evaluator::rescan_on_mutation`; guard-only transitions participate per Runtime Companion §10.3; `Transition::event` optional with trim-to-absent deserialization; kernel schema + spec alignment.
- [x] **ADR 0059 Task 5 — K-049 LoadBearing + greenfield cleanup** (`f03ca40`) — drop authored `"$continuous"` from `participates_in_continuous_rescan`; synthetic trace/provenance dispatch label `$postMutationRescan`; remove deprecated `try_fire_guardless_transition`; K-049 warning cites §10.3 + `CONVERGENCE_CAP`; rule promoted **LoadBearing** with two `fixtures/validation/k-049-load-bearing-*.json` + `tier2_rules` harness; governance/kernel schema descriptions updated.

### #22a — ProvenanceKind tier-typing (`1240745`, `916d6db`)

- [x] **`wos-core` provenance module split** — `provenance.rs` replaced by `provenance/{mod,snapshot,kind,audit_tier,record,log,tests}.rs`; `ProvenanceAuditTier` (`Facts` | `Narrative`) with `From<ProvenanceKind>`; `audit_layer_for_kind` retained as a string bridge; crate-root re-export.
- [x] **`wos-runtime` stamp path** — `populate_provenance_record_fields` sets `audit_layer` via `ProvenanceAuditTier::from(record.record_kind).as_str()` (typed tier at emission site).

### #20 — Typed event meta-vocabulary (`TransitionEvent`)

- [x] **Kernel JSON Schema** — `$defs/TransitionEvent` + five branch shapes; `Transition.event` and `Action.event` (`startTimer`) reference the union; `signal.name` pattern allows `$join` and `$compensation.complete`.
- [x] **`wos-core`** — `TransitionEvent` with lowercase `kind` tag, camelCase JSON field renames on variants, `from_legacy_string` / `runtime_dispatch_label` / `matches_runtime_dispatch`; optional `Transition.event`; legacy string deserialization on transitions and actions.
- [x] **Eval / runtime / lint / authoring / MCP** — dispatch and lint rules updated; K-007 retained on typed model for `$` misuse on `message` / disallowed `$` signals; K-008 join signal check.
- [x] **Fixtures + migration script** — `scripts/migrate-transition-events.py`; kernel fixtures including `$compensation.complete` as `signal` name with `$` prefix (matches `process_event`).
- [x] **Spec prose** — `specs/kernel/spec.md` §4.5–§4.6, §4.8, §4.10, §9.2; governance `startEvent` reserved-name note.
- [x] **Plan doc** — `thoughts/plans/2026-04-20-wos-typed-event-meta-vocabulary.md` aligned with shipped serde and compensation signal spelling.

### Validation at close

`cargo test --workspace`: **1012 passed / 0 failed**. SCHEMA-DOC-001 zero-regression gate green. `pytest tests/schemas/ -q`: **188 passed / 11 skipped / 1 xfailed** (+17 vs session 8). `npm run docs:check`: exit 0. `git status`: clean.

---

## Session — D-wave + E-wave (Studio production polish, 2026-05-03)

Two consecutive waves on the Studio (Authoring) layer reducing
deferred-work surface from 343 → 268 markers (22% burndown) and
closing 4 of 5 pre-D-wave open IDs.

### D-wave (16 commits including ADR-0083 + parallel-burndown sub-commits D8-A/B/C/D9)

- [x] **STUDIO-DEFER-001 closed** — `.raw` access ratchet 47 → 8
  residual (`crates/wos-studio-lint/tests/raw_access_ratchet.rs`).
  Added `WorkspaceDocument` typed accessors + `StudioDocument::body()`
  dispatch; migrated 47 of 47 lint-rule sites.
- [x] **STUDIO-DEFER-002 closed** — Lint fixture suite externalization;
  37 of 43 fixtures externalized to `crates/wos-studio-lint/fixtures/{s1..s6,cross_cutting}/`;
  inventory ratchet at `tests/fixture_inventory_ratchet.rs`; 6 date-arithmetic
  / sentinel tests intentionally inline.
- [x] **STUDIO-DEFER-003 Tranches A/B/C closed** — boon format-assertion mode
  enabled; parent lint K-016 added (`initialState ∈ states`); actor-id
  uniqueness lint-covered via K-009 with no schema reshape.
- [x] **STUDIO-DEFER-004 split** — single 343-marker entry split into 6
  per-kind sub-IDs (RUNTIME / LINT / SCHEMA / FIXTURE / COORDINATION /
  WORKFLOW) with per-kind ratchet baselines.
- [x] **DEFER-004 D-wave burndown** — 343 → 273 markers (20%) across
  SCHEMA / LINT / FIXTURE; 4 parallel agents (A:SCHEMA, B:LINT, C:FIXTURE,
  D:investigation).
- [x] **ADR-0083 (Proposed)** — `thoughts/adr/0083-studio-retention-policy-shape.md`
  pins the typed `RetentionPolicy` shape that would unblock STUDIO-DEFER-005.
  Awaiting Studio team review.
- [x] **K-016 lint rule (parent)** — added to `crates/wos-lint/src/rules/tier1.rs`
  - registry; 4 sentinel tests in `tier1_rules.rs`; Studio cross-pass
  test in `lint_pass_xref.rs`.

### E-wave (10 commits, 273 → 268; renumbered as E0-E10 with E5 + E11 skipped, plus E3 split into E3.1/E3.2)

- [x] **E0** — bookkeeping accuracy (CLAUDE.md totals, DEFERRED.md narrative
  fixes, DEFER-005 ADR back-link, retention_policy() line anchor).
- [x] **E1** — placeholder `specs/kernel/spec.llm.md` (force-add) unblocks
  parent `cargo check --workspace`. The synth crates' include_str! resolves;
  prompts are empty until authored.
- [x] **E2** — README false-positive fix; ratchet `count_pending` now skips
  `README.md` (convention prose with literal marker tokens). Drops 4 markers
  from baselines.
- [x] **E3** — schema burndown 20 → 10: encoded 4 markers via `if/then` +
  `contains` (ws-002, rv-030, bind-040, id-010); fixture migration in
  snap-redetermination examples; reclassified 6 cross-doc markers
  (prov-010, pom-021, scn-002, scn-006) to lint, plus bind-031 enum
  encode + ra-001 sweep.
- [x] **E10** — RUNTIME re-triage: 4 mis-categorized markers reclassified
  to lint (prov-005, ra-012, pom-035, cmp-052 — all readiness-tier
  checks, not Phase-4 runtime).
- [x] **E4** — re-evidence 9 promoted parent K-/WOS-* rules; added
  per-rule inline-evidence comments to `crates/wos-lint/src/rules/registry.rs`
  (K-FOREACH-001..004, WOS-EMBED-IDENTITY-001, WOS-EMBED-TARGET-001,
  WOS-SIDECAR-TARGET-001, WOS-SIG-COVER-001, WOS-VER-LEVEL-001).
  `cargo test -p wos-lint --test rule_registry` now 5 passed / 0 failed.
- [x] **E5 skipped** — STUDIO-LINT-MATRIX already explicit on parent vs
  Studio rule split; audit agent had conflated separate registries.
- [x] **E6** — Companion-PRD reconciliation: 3 net-new items landed
  (VISION.md "Capabilities-first product rule"; binding-and-integration.md
  "Binding Inspector" surface; source-vault.md "Akoma Ntoso ingest path");
  8 of 9 capability modules from the addendum rejected as redundant
  with existing specs (documented in studio/specs/README.md).
- [x] **E7** — DEFER-004-WORKFLOW closed: pinned 0.5 ExtractedClaim
  confidence threshold as default-with-override per parent
  ai-integration.md §S7. Marker reclassified to schema-pending +
  runtime-pending.
- [x] **E11 skipped** — 4 new POM-LINT rules deferred (closer to feature
  work than tightening; new BIND-LINT family also deferred for spec-side
  design).
- [x] **E9** — removed unused_import warning in phase8_package.rs;
  appended this entry to COMPLETED.md.

### Open after E-wave

- **STUDIO-DEFER-004-{RUNTIME (183), LINT (71), SCHEMA (11), FIXTURE (1),
  COORDINATION (2)}** — 268 markers across 5 sub-IDs.
- **STUDIO-DEFER-005** — ADR-0083 Proposed; 4-step implementation queued
  pending Studio team acceptance.

### Validation at close

`python3 studio/tests/pending_ratchet.py`: total 268, all kinds within
baseline. `cd studio && cargo test --workspace`: green (210+ tests).
`cargo check --workspace` from repo root: clean. `python3 -m pytest
studio/tests/schemas`: 39 passed, 1 skipped. `cargo test -p wos-lint
--test rule_registry`: 5 passed. Schema regressions: zero. New parent
warnings: zero.

### F-wave (review-driven fixups, 2026-05-03 same-day)

After D+E-wave shipped, ran a swarm of 5 semi-formal code-review
agents (3 sonnet: Rust / schemas / bookkeeping; 2 opus: spec
authoring / ADR-0083). Aggregate findings: 2 Critical (both
ADR-0083), 12 Major (4 schemas, 4 specs, 4 bookkeeping), ~10 Minor.
F-wave addresses all of them.

- [x] **F1** — ADR-0083 r2: revised in place (Status still Proposed)
  per opus deep-review: `disposalAction` default removed (force
  explicit pick — government-benefits substrate where defaults
  matter); §2.4 clock-resume promise dropped (workflow-governance
  §7.15 doesn't actually back it); renamed `legalHoldOverride` →
  `respectsLegalHold` (prior name read backwards); dropped
  `transfer` from v1 enum (footgun without destination shape);
  pinned regulatoryBasis[] merge semantics; acknowledged
  workspace-bag breaking change + preserved `^x-` patternProperties;
  added 4 open questions (versioning at republish, disposal audit
  sink, pseudonymization-vs-deletion, DSAR×legal-hold).
- [x] **F2** — Schema correctness: bind-040 enums fixed
  (`hitPolicy: first-match|priority|unique|output-merge`,
  `completenessRequirement: all-inputs-covered|partial-allowed-with-default`
  — schema had used wrong values vs spec); id-010 `"indefinite"`
  sentinel made explicit via oneOf; prov-071 aiLineage required
  fields now conditional on `eventSubtype = "ai-assisted"` via
  outer if/then.
- [x] **F3** — Spec internal contradictions: source-vault.md
  ingestFormat enum gains `akoma-ntoso` (E6.3 had claimed it did
  but didn't); VISION's "five binding kinds" → "four binding
  kinds" with explicit list; VISION MUST NOT → SHOULD NOT (no
  enforcement seam yet); VISION PROV-O reclassified as
  user-visible export (not backing profile); Binding Inspector
  drops DecisionRule (not a binding kind) + loosens
  projection-target column (per-kind not hardcoded);
  SOURCE-LINT-008 reference dropped (rule-id minted when
  registry implements); pom-035 restored runtime-pending half
  (re-validation cascade is genuinely runtime).
- [x] **F4** — Bookkeeping accuracy round 2: DEFERRED.md RUNTIME
  header/anchor 182 → 184; SCHEMA header/anchor 10 → 11; SCHEMA
  rationale "remaining 20" → "remaining 11"; LINT narrative
  extended to document 63 → 71 path (E2/E3.2/E6.3/E10/F3 deltas);
  FIXTURE narrative cleanup post-E2 SKIP_FILES; COMPLETED.md
  D-wave commit count 10 → 16 + E-wave 8 → 10.
- [x] **F5** — Rust polish: K-016 graduation Draft → Tested + 4
  inline tests promoted with severity/path assertions + 2 new
  deep-nested tests; E4 evidence comment paths disambiguated
  (src/rules/tier2.rs vs tests/tier2_rules.rs); schema_validator.rs
  stripped-pattern count comment clarified; phase8_package.rs raw
  accesses migrated to typed accessors where possible (id,
  wos_version_pin) with the rest documented as legitimate
  compiler-tier `.raw` use.

### Final state at F-wave close

- **Commits:** 31 total over D+E+F waves on the branch
  (16 D + 10 E + 5 F), plus this COMPLETED entry = 32.
- **Markers:** 343 → 269 (22% burndown), distributed:
  runtime 184 / lint 71 / schema 11 / fixture 1 / coordination 2.
- **DEFER status:** DEFER-001/002/003 all closed (incl. all
  three Tranches A/B/C); DEFER-004 split + per-kind ratcheted
  - WORKFLOW closed; DEFER-005 awaiting Studio team review of
  ADR-0083 r2.
- **Pre-existing parent failures:** all addressed
  (rule_registry now 5 passed; synth crates compile via
  placeholder spec.llm.md).

## G-wave (2026-05-03) — production polish, multi-agent review fixups

7 commits closing every finding from the 5-agent semi-formal code
review of the post-F state:

- **G1** — K-016 lint rule covers two reachable subtrees A1's
  review identified as gaps: `Region.initial_state` vs
  `Region.states` (parallel-state regions) and `State.body` (forEach
  iteration body). Both let a workflow pass schema-pass + lint-pass
  and blow up at runtime; one-line additions to the recursion plus
  two sentinel tests with severity + path assertions. Tightened the
  parallel-region test to gold-standard asserts (NIT-1). Fixed
  dangling `spec_ref` (NIT-3): pointed at non-existent
  `studio/specs/lifecycle.md`; now references `specs/kernel/spec.md
  §4.1/§4.3/§4.8`. Documented empty-string `initialState` handling
  in registry comment (NIT-2).
- **G2** — Three schema-conditional bypasses A3's review surfaced:
  workspace `reviewerRoles` not top-level required (paired
  `contains` only fired when present, silently bypassed
  SA-MUST-ws-002 on omission); collection-form PolicyObject didn't
  validate per-kind body if/then (`$defs.PolicyObjectKindRules`
  extracted; both single-form and collection-form items reference
  it via `$ref`); embedded `provenance[]` arrays didn't `$ref`
  AuthoringProvenanceRecord (PolicyObject + Binding +
  WorkflowIntent now use `oneOf [string-ref, $ref]`). Migrated 3
  snap fixtures to satisfy the tightened embedded-provenance shape
  (`subjectRef` + `recordedAt` + `inputContextHash` added).
- **G3** — 11 negative tests in `test_studio_negative.py` covering
  the E3.1 / E3.2 / F2 / G2 conditionals (A3 F5):
  ws-002, rv-030 waiver shape, bind-040 enums (single + collection
  form), id-010 sentinel, prov-071 outer if/then, ra-022 empty
  conditions. Total Studio-tier negative tests: 32 (was 21).
- **G4-G6** — Bookkeeping accuracy: studio/CLAUDE.md totals
  corrected from `183/71/11/1/2 post-E-wave` to `184/71/11/1/2 at
  HEAD` (A5 F-MAJOR-1); COMPLETED.md commit count corrected from
  35 to 31+1=32 (A5 F-MAJOR-2); four `.raw` accesses in
  phase8_package.rs migrated to typed accessors (workspaceId,
  version → `.body().get(...)`; sourceVersions → `.source_versions()`;
  id → `.id()`; policyObjects → `.body().get(...)`) — A2 F4.
- **G7-G9** — ADR-0083 §2.2 worked-example (b) prose clarified
  to foreclose a bug-class where the implementer might strip
  workspace `regulatoryBasis[]` on `respectsLegalHold=true` upgrade
  (A5 F-MAJOR-3); STUDIO-DEFER-006 added for the kernel-spec
  amendment surfaced by ADR-0083 §2.4 as "blocking E8.4 lint
  promotion" — first cross-repo DEFER (A5 F-MAJOR-5);
  studio/VISION.md SHOULD-NOT→MUST-NOT trigger sharpened with two
  named anchors (A5 F-MAJOR-6).
- **G10-G12** — Minor follow-ups: `body()` 14-arm dispatch test
  added in `wos-studio-model::docs::tests` (A2 F5); raw_access_ratchet
  doc-comment rewritten to remove stale STUDIO-DEFER-001 reference
  and clarify scope (A2 F6); fixture_inventory_ratchet now walks
  both `src/` and `tests/` trees and enforces a `FOUND_FLOOR=36`
  to surface partial reverts (A4 F3 + F6).

### Final state at G-wave close

- **Commits:** 38 total (16 D + 10 E + 5 F + 7 G), plus this
  COMPLETED entry.
- **Markers:** 269 unchanged (G-wave is correctness-tightening,
  no marker burndown).
- **Negative tests:** 32 (was 21).
- **K-016 sentinel tests:** 8 (was 6); covers top-level, compound
  (1-level + 3-level), parallel-region (compound substate +
  bare region.initialState), forEach body.
- **DEFER status:** unchanged from F-wave close +
  STUDIO-DEFER-006 added (kernel-spec amendment for legal-hold
  clock-resume semantics).
- **Verified:** Studio tests green; Python schema regression
  green (51 passed, 1 skipped); pending-annotation ratchet
  green (184/71/11/1/2 = 269); raw_access_ratchet green
  (≤8); fixture_inventory_ratchet green (≥36); rule_registry
  green; api_surface boundary guard green; determinism green.
  Pre-existing schema_doc_zero_regression failure on
  `wos-workflow.schema.json` (parent kernel surface, not Studio)
  remains for parent-team work.

## H-wave (2026-05-03) — E8 (DEFER-005 close) + E11 (4 POM-LINT rules)

8 commits completing the user-gated E8 implementation per ADR-0083 r2
plus the optional E11 rule-authoring chip from the original E-plan:

### E8 — DEFER-005 implementation (RetentionPolicy typed promotion)

- **E8.0** — ADR-0083 Status: Proposed → Accepted (2026-05-03).
  G7 had clarified the §2.2 worked-example (b) prose; design
  production-ready per A5's review.
- **E8.1** — `studio/specs/policy-object-model.md`:
  EvidenceRequirement.body.retentionPeriod? → retentionPolicy?:
  RetentionPolicy. Added full § "RetentionPolicy" data-model block
  (6-row field table + composition note on workspace defaults +
  override resolution semantics + migration narrative). Sharpened
  SA-MUST-pom-037 to enumerate the closed-shape requirements.
- **E8.2** — schema:
  `wos-studio-policy-object.schema.json::$defs.RetentionPolicy`
  added (closed shape, additionalProperties: false, three if/then
  guards in allOf for mode/duration/respectsLegalHold);
  EvidenceRequirement body declares `retentionPolicy` as `$ref`;
  `wos-studio-workspace.schema.json::retentionPolicies` value-side
  tightened to `additionalProperties: $ref →
  studio-policy-object/1.0#/$defs/RetentionPolicy`. Snap workspace
  fixture migrated from singular ISO-duration strings to typed
  policies (HIPAA carries cited regulatoryBasis).
- **E8.3** — Rust:
  New `studio/crates/wos-studio-model/src/policy.rs` with
  `RetentionPolicy` struct + `DisposalAction` /
  `RetentionMode` / `TriggerEvent` enums + `effective_mode()` /
  `effective_respects_legal_hold()` defaulters +
  `shape_violations()` validator + 6 unit tests covering
  minimal-bounded, indefinite, each violation kind, and round-trip
  with x- extension. `retention_policy()` accessor promoted from
  `Option<&Value>` to `Option<Result<RetentionPolicy,
  serde_json::Error>>`; companion `retention_policy_raw()` and
  `legacy_retention_period()` accessors added.
- **E8.4** — WF-LINT-006 promoted from presence-only to
  shape-aware: resolves per-EvidenceRequirement override OR
  workspace default keyed by DPV sensitivity, parses via
  RetentionPolicy, runs shape_violations(). New
  `Workspace::workspace_document()` accessor. New
  `SA-WARN-pom-MIGRATE-RETENTION` advisory rule fires when the
  legacy `retentionPeriod` field is present (independent of
  WF-LINT-006). 3 new fixtures + 3 new tests (workspace-default
  resolves, inline-policy malformed, legacy-field advisory).
  Registry count 70 → 71.
- **E8.5** — STUDIO-DEFER-005 moved from Open to Closed in
  DEFERRED.md with full resolution narrative; CLAUDE.md preamble
  updated; STUDIO-DEFER-006 narrative updated to clarify it is no
  longer blocking E8.4 (lint shipped without needing the kernel
  amendment, which becomes a forward-looking improvement).

### E11 — 4 new POM-LINT rules (the optional E-plan chip)

- **E11.1** — Implementations + fixtures + tests:
  - `POM-LINT-020` (S2, Error) — PolicyObject *past* approved
    (mapped/validated/published/superseded/deprecated/demoted)
    requires matching ApprovalDecision (SA-MUST-pom-020). The
    `approved` state itself is the gate being crossed (snap-shorthand
    pattern accommodated). Reads both
    body.decision.subjectRef and body.subjectRef serialization
    shapes.
  - `POM-LINT-033` (S4, Error) — AppealRight.outcomeRef MUST equal
    linked Notice's outcomeRef on explicit mismatch
    (SA-MUST-pom-033). Implicit inheritance (no AppealRight
    outcomeRef) permitted as authoring shorthand. Waiver path:
    body.waiverScope='separate-procedure' + body.waivedAt silences.
  - `POM-LINT-040` (S2, Error) — two approved Deadlines on the
    same body.trigger with different body.calendarDaysFromTrigger
    require a Conflict naming both subjects to be filed
    (SA-MUST-pom-040). Tractable lint-time slice; the general
    non-Deadline algorithm stays runtime-pending.
  - `POM-LINT-051` (S2, Warning) — two deontic constraints
    (Permission/Prohibition/Obligation) sharing (subject, action)
    flagged as composition candidates unless one carries
    body.compositionAttestation='reviewed' (SA-MUST-pom-051).
    Warning severity per spec wording. Effectiveness intersection
    not modeled at lint time.
  - 8 fixtures (one firing + one silent per rule) + 8 tests.
    Registry count 71 → 75. `BTreeSet` import added to
    workspace_rules.rs.
- **E11.2** — Marker sweep:
  4 lint-pending markers closed in policy-object-model.md (each
  cited the corresponding new rule); pom-040 sharpening preserved
  one runtime-pending for the general non-Deadline algorithm.
  STUDIO-LINT-MATRIX.md authoritative count 70 → 75 with 4 new
  rows. Ratchet baselines: lint 71 → 67 (-4); runtime 184 → 185
  (+1); net total 269 → 266 (-3). DEFERRED.md anchors updated;
  CLAUDE.md preamble totals 184/71/11/1/2 → 185/67/11/1/2.

### Final state at H-wave close

- **Commits:** 46 total (16 D + 10 E + 5 F + 7 G + 8 H), plus this
  COMPLETED entry.
- **Markers:** 343 → 266 (22% → 22.5% burndown), distributed:
  runtime 185 / lint 67 / schema 11 / fixture 1 / coordination 2.
- **Open Studio DEFERs:**
  STUDIO-DEFER-004-{RUNTIME 185, LINT 67, SCHEMA 11, FIXTURE 1,
  COORDINATION 2} per-kind pending-annotation burndown;
  STUDIO-DEFER-006 (kernel-spec amendment for legal-hold
  clock-resume; forward-looking, not blocking).

## I-wave (2026-05-03) — finish DEFER-004 in parallel (closure + reclassify)

Per the user's "tractable + reclassify residual" directive: close
every closable DEFER-004 marker, then reclassify the irreducible
residual to a new STUDIO-DEFER-007 (Stage-7/8 substrate dependency
taxonomy) so DEFER-004's RUNTIME / FIXTURE / COORDINATION sub-IDs
all drain to 0.

8 commits across four phases (A: parallel lint authoring;
B: schema sweep; C: skipped — Phase D5 reclassify supplants;
D: ADRs + reclassify + bookkeeping):

### Phase A — Lint cluster authoring (4 commits, 35 new rules, 36 markers)

- **I-A1** — SV-LINT-007..014 (8 new rules, 9 markers).
  Source-vault extension family: workspace-tier sv_lint_007
  (versionless source cited) + 7 doc-local rules covering temporal
  progression slice (sv-008), parsingResult.status enforcement
  (sv-009), at-most-one-current per SourceDocument (sv-010),
  pageable→pageRange (sv-011), JSON-LD context drift (sv-012),
  akoma-ntoso FRBRdate (sv-013), multilingual authoritative locale
  (sv-014).
- **I-A2** — BIND-LINT-001..006 (6 new rules, 6 markers).
  Brand-new BIND-LINT family: extension registry lookup
  (bind-001), closed seam set per ADR-0077 (bind-002),
  caseFilePath resolution (bind-003), output target resolution
  with ignoredRationale escape (bind-004), sensitive-input
  handling (bind-005), errorHandling.onError enum (bind-006).
  Two private helpers (`binding_kind_of` / `binding_body`) route
  through StudioDocument body() typed dispatch.
- **I-A3+I-A4+I-A5** — BIND-LINT-010..072 (8 new rules, 8 markers).
  EventBinding (consumed-source / emitted-recipient / sensitive-
  payload-redaction); PolicyEngineBinding (caseFilePaths
  declaration / engineReasonCodes mapping); binding-scenario
  coverage (≥1 happy-path scenario / ≥2 error-path scenarios for
  retry / both permit+deny for PolicyEngine).
- **I-A6+I-A7+I-A8** — WFI/MAP/RA/PROV cross-ref (13 rules, 13 markers).
  WF-LINT-009..013 (element id uniqueness, position references,
  notice/appeal/system-check refs); MAP-LINT-009..011 (workflow
  advancement gates with mappingState; ExtensionRecord motivation);
  RA-LINT-001..002 (reviewerRole + comment subject resolution);
  PROV-LINT-005..007 (parentRecordIds resolve, originClass on
  approved elements, approved-interpretation evidence chain).

### Phase B — Schema sweep (1 commit, 4 markers encoded, 7 residual)

- **I-B1** — pom-001 retire (already encoded), pom-010 add
  ExtractedClaim.body.confidenceFloor optional field, map-003
  retire (already encoded via mapping schema's four-way if/then),
  source-051 add canonicalSourceRef.referencedUri via if/then on
  ingestFormat=json-ld + snap fixture migration. 7 schema markers
  remain residual (need fixture migration deferred to follow-up).

### Phase D — ADRs + reclassify + bookkeeping (3 commits)

- **I-D1+I-D2** — ADR-0084 (PLN-0381 identity attestation primitive
  Studio anchor) + ADR-0085 (PLN-0384 event-types taxonomy Studio
  anchor). Both at Status: Proposed. Each pins a Studio-side
  placeholder shape that is a strict subset of the parent's
  expected primitive — so parent ratification is a `$ref` swap
  with no breaking change. Coordination markers id-004 + prov-082
  retire to point at these ADRs.
- **I-D4** — cmp-051 (compiler version-bump semantic equality)
  sharpened from `(fixture-pending)` to `(substrate-pending)`:
  the cross-version comparison harness lands when v2 compiler
  exists. Single FIXTURE marker rolls into DEFER-007.
- **I-D5** — Reclassify all 190 remaining `(runtime-pending)`
  markers to `(substrate-pending)` via mass sed across
  `studio/specs/*.md` (excluding README.md). The reclassification
  reflects that these markers need actual Stage-7/8 substrate
  work (audit log, change-detection engine, scenario simulator
  emission, runtime-observation adapter, Trellis identity seam,
  kernel clock-resume amendment) that Studio cannot unblock alone.
- **I-D6-8** — pending_ratchet.py BASELINES updated: runtime/
  fixture/coordination = 0; substrate = 191 (new kind);
  lint = 31, schema = 7 (residual). DEFERRED.md restructured:
  DEFER-004-RUNTIME / DEFER-004-FIXTURE / DEFER-004-COORDINATION
  moved to Closed; DEFER-007 added under Open with substrate-
  dependency taxonomy. CLAUDE.md preamble updated.

### Final state at I-wave close

- **Commits:** 54 total (16 D + 10 E + 5 F + 7 G + 8 H + 8 I),
  plus this COMPLETED entry.
- **Markers:** 266 → 229 (22.5% → 33% burndown via tractable
  closure; the substrate residual stays 191 but moves to a
  separate sub-ID with sharpened narrative). Distribution:
  substrate 191 / lint 31 / schema 7 / fixture 0 / coordination 0
  / runtime 0.
- **Open Studio DEFERs:**
  - STUDIO-DEFER-004-LINT (31 markers — spec-side blockers + fixture
    migration debt).
  - STUDIO-DEFER-004-SCHEMA (7 markers — pom-032 / ra-021 / rtos-001 /
    scn-001 / scn-043 / wfi-010 / wfi-040; need fixture vetting +
    Phase-4 RuntimeObservation schema artifact).
  - STUDIO-DEFER-006 (kernel-spec amendment; forward-looking).
  - STUDIO-DEFER-007 (191 substrate markers; Stage-7/8 dependency).
- **Closed in I-wave:** STUDIO-DEFER-004-RUNTIME (CLOSED via
  reclassify); STUDIO-DEFER-004-FIXTURE (CLOSED via cmp-051
  sharpen); STUDIO-DEFER-004-COORDINATION (CLOSED via ADR-0084 +
  ADR-0085).
- **Rules in registry:** 110 (was 75 pre-I-wave; +35 across Phase A).
- **New ADRs:** 0084 (PLN-0381 identity attestation), 0085
  (PLN-0384 event-types taxonomy). Both Status: Proposed.
- **Verified at I-wave close:**
  - cd studio && cargo test --workspace: 0 failures.
  - python3 -m pytest studio/tests/schemas: 51 passed, 1 skipped.
  - python3 studio/tests/pending_ratchet.py: substrate 191 /
    lint 31 / schema 7 / fixture 0 / coordination 0 = 229.
  - raw_access_ratchet, fixture_inventory_ratchet, rule_registry,
    api_surface boundary, determinism: all green.
- **Rules in registry:** 75 (was 70 pre-E8.4).
- **Verified at H-wave close:**
  - Studio cargo test --workspace: clean (lib 121, was 113;
    +8 from E11.1 + 4 from E8.4 + 6 from E8.3 - 2 dup counts).
  - python3 -m pytest studio/tests/schemas: 51 passed, 1 skipped.
  - python3 studio/tests/pending_ratchet.py: 185/67/11/1/2 = 266.
  - cargo test -p wos-lint --test rule_registry: 5 passed.
  - cargo test -p wos-lint --test tier1_rules: 105 passed
    (includes the 8 K-016 tests from G1).
  - raw_access_ratchet, fixture_inventory_ratchet, api_surface
    boundary, determinism: all green.
  - Pre-existing schema_doc_zero_regression on
    wos-workflow.schema.json (parent kernel surface) remains for
    parent-team work.

## J-wave (2026-05-03) — multi-agent review fixups, all 12 I-wave findings addressed

Single commit closing every finding from the 5-agent semi-formal code
review of the I-wave (3 BLOCKER/CRITICAL + 3 HIGH + 4 MEDIUM + 2 MINOR).
All 3 critical claims verified against the source before fixing.

### Critical / blocker fixes (3)

- **J1** — `Scenario.expectedDecision` schema field added (was reading
  a field BIND-LINT-072 invented by convention; fixture passed for
  the wrong reason).
- **J2** — MAP-LINT-010 + MAP-LINT-011 rewritten to query Mapping
  documents instead of `kind=ExtensionRecord` PolicyObjects (which
  isn't a kind in the enum). Both rules previously dead code.
- **J3** — ID-LINT-004 added (cardinality + temporal-validity check
  per ADR-0084 §2.2). ID-LINT-003 was already taken by an
  attestationLevel-sufficiency check; minted new id to avoid
  collision; ADR-0084 §2.2 + §6 updated.

### Major fixes (3)

- **J4** — 13 cross-ref fixtures + 13 firing-case tests authored.
  I-A6+A7+A8 had landed 13 rules with no test coverage; J4 closes
  the gap.
- **J5** — SV-LINT-012 lifecycle-aware comparison: only compare
  `current|superseded` json-ld versions; non-json-ld breaks chain.
  2 regression tests added.
- **J6/J10** — ADR-0084 §4 contingency narrative replaces
  "analogous to ADR-0083" (which understated the cost given
  ADR-0084 references ~12 spec locations).

### Moderate fixes (3)

- **J7** — BIND-LINT-070 doc-comment names the structural limit
  (no Scenario.scenarioType discriminator → can only enforce
  existence; sharpen when discriminator lands).
- **J8** — MAP-LINT-009 ref field list extended with `deadlineRef`
  - `serviceBindingRef`; comment names policy.
- **J9** — ADR-0085 demoted-vs-deprecated resolution: distinct
  events at distinct lifecycle phases; cluster grouping
  downgraded to ADVISORY-ONLY.

### Minor fixes (2)

- **J11** — SV-LINT-010 message id-list fidelity: uses "?"
  placeholder for missing ids, never silently truncates.
- **J12** — DEFERRED.md:75 title fix ("11 markers" → "7 markers
  residual").

### Final state at J-wave close

- **Commits:** 55 total (16 D + 10 E + 5 F + 7 G + 8 H + 8 I + 1 J),
  plus this COMPLETED entry.
- **Rules in registry:** 111 (was 110 pre-J3; +1 ID-LINT-004).
- **Tests added:** 13 cross-ref firing tests + 4 ID-LINT-004 +
  2 SV-LINT-012 regression = 19 new tests.
- **Findings closed:** 12 / 12.
- **Marker baselines unchanged:** substrate 191 / lint 31 /
  schema 7 / fixture 0 / coordination 0 = 229 markers.
- **Verified at J-wave close:**
  - cd studio && cargo test --workspace: 0 failures.
  - python3 -m pytest studio/tests/schemas: 51 passed, 1 skipped.
  - python3 studio/tests/pending_ratchet.py: substrate 191 /
    lint 31 / schema 7 / fixture 0 / coordination 0 = 229.
  - cargo test -p wos-lint --test rule_registry: 5 passed.
  - cargo test -p wos-lint --test tier1_rules: 105 passed.
