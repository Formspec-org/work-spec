# WOS Release Streams

> **Updated 2026-04-28 per [ADR 0076](../thoughts/adr/0076-product-tier-consolidation.md) D-7:** the schema family collapses from 27 → 6 schemas with a single top-level marker `$wosWorkflow`. Stream identity (governance, agents, signature, custody, advanced) is implicit in the workflow envelope version. Compliance claims now compose as `$wosWorkflow@X.Y` plus the **claims map** below. The four-stream cadence and source-path framing below is preserved for repository-engineering hygiene (Changesets groupings, per-tier cadence) but is no longer the authoring or compliance-claim surface.

## Claims map (post-ADR-0076)

A vendor implementing `$wosWorkflow@1.0` declares which embedded blocks they exercise:

| Embedded block | Conformance suite | Old stream framing (reference) |
|---|---|---|
| (none — bare core) | Kernel-Basic conformance | `wos-kernel@1.0` |
| `governance` | Governance fixtures + due-process pipeline tests | `wos-governance@1.0` |
| `governance.obligationPolicies[]` (activation criteria + durable obligations, [ADR 0096](thoughts/adr/0096-shared-activation-criteria-and-durable-obligations.md)) | `OBL-001..013` runtime fixtures + `ACT-001..010` lint | within `wos-governance@1.0` (governance subset) |
| `agents` + `aiOversight` | AI deontic + autonomy + drift fixtures | `wos-ai@1.0` |
| `signature` | T4 SIG-* conformance suite (signature.md ⇒ embedded `signature` block) | T4 (separate per Q11) |
| `custody` | Trellis custody-anchor cross-stack fixtures | (cross-stack, not stream-scoped) |
| `advanced` | Advanced equity + DCR + verifiable-constraints fixtures | `wos-advanced@1.0` |
| `assurance` | Assurance attestation fixtures | (typically paired with governance) |

A vendor's claim string: `$wosWorkflow@1.0 [governance, agents, aiOversight, signature]`. A vendor exercising durable obligation policies notes the governance subset, e.g. `$wosWorkflow@1.0 [governance(obligations)]` — the obligation/activation block composes under the same envelope version ([ADR 0096](thoughts/adr/0096-shared-activation-criteria-and-durable-obligations.md); it adds no new top-level marker and no new stream).

WOS ships as **four independent release streams** inside one repository. Each stream
owns a set of source paths, a publishing cadence, and a stability commitment. Vendor
compliance claims reference a *pair* (or more) of stream versions — for example,
`wos-kernel@1.0 + wos-ai@0.5` — rather than a single monolithic `wos@X.Y` number.

See [`COMPATIBILITY-MATRIX.md`](COMPATIBILITY-MATRIX.md) for known-good pairings across
streams, and [`thoughts/plans/2026-04-16-wos-release-trains.md`](thoughts/plans/2026-04-16-wos-release-trains.md)
for the full rollout plan (this file covers the scope + mapping; the plan covers
Changesets tooling and the per-stream release workflow).

---

## Why four streams?

The four layers of WOS evolve on fundamentally different clocks. Pinning them
together forces every kernel consumer to re-certify against AI-layer churn they do
not use, and forces every AI-layer experiment to wait on kernel semver discipline
it does not need. Splitting the streams lets each layer ship at its natural cadence
while keeping the repository (and cross-layer invariants) unified.

The streams mirror parent
[formspec ADR 0063](../thoughts/adr/0063-release-trains-by-tier.md) — kernel /
foundation / integration / AI — adapted to WOS's four tiers.

## Stream → path mapping

Every file in the repository belongs to exactly one stream. When a change touches
paths from multiple streams, it produces one Changeset entry per affected stream.

| Stream | Paths | Cadence | Stability |
|--------|-------|---------|-----------|
| `wos-kernel` | `specs/kernel/**`, `specs/companions/**`, `schemas/kernel/**`, `schemas/companions/**`, `crates/wos-core/**`, `crates/wos-runtime/**` | 6–12 months | semver-strict after 1.0 |
| `wos-governance` | `specs/governance/**`, `schemas/governance/**`, `fixtures/governance/**` | 3–6 months | semver-strict at 1.0+ |
| `wos-ai` | `specs/ai/**`, `schemas/ai/**`, `fixtures/ai/**` | monthly / quarterly | pre-1.0, no stability guarantee |
| `wos-advanced` | `specs/advanced/**`, `schemas/advanced/**`, `fixtures/advanced/**`, `specs/assurance/**`, `schemas/assurance/**` | research | no GA commitment |

## Cross-cutting crates and sidecars

Several artifacts are cross-cutting — they operate on documents from every stream
and cannot live cleanly in one. The rule is **the checking surface follows
kernel**: if a crate or sidecar exists to validate or report conformance against
the spec, it releases with `wos-kernel`. This keeps the "what is a conformant WOS
document" question answerable against a single version number.

| Artifact | Owning stream | Rationale |
|----------|---------------|-----------|
| `crates/wos-lint/**` | `wos-kernel` | Defines the lint surface that all streams' fixtures are checked against. |
| `crates/wos-conformance/**` | `wos-kernel` | Runs the rule-coverage suite that produces per-tier conformance numbers. |
| `LINT-MATRIX.md` (generated) | `wos-kernel` | Derived from `wos-lint`; moves in lockstep with it. |
| Rule-coverage reports | `wos-kernel` | Same; the reports *reference* per-stream tiers but the reporting machinery is kernel-owned. |
| Profile sidecars (authoring) | Stream of the profile's scope | A profile scoped to governance releases with `wos-governance`; an AI-layer profile releases with `wos-ai`. Default when ambiguous: kernel. |

Non-crate sidecars (e.g. authoring tools, MCP glue, synthesis stack) follow their
primary consumer. At time of writing: `wos-authoring`, `wos-mcp`, `wos-synth-*`, and
`wos-bench` are AI-stream by consumption. See the Task 4 Changesets `fixed` group
configuration in the [release-trains plan](thoughts/plans/2026-04-16-wos-release-trains.md#task-4-changesets--tagging--release-workflow-mirror-adr-0063-steps-14)
for the precise package groupings.

## Tag convention

Per-stream git tags follow `<stream>-v<X.Y.Z>` — for example `kernel-v1.0.0`,
`governance-v1.0.0`, `ai-v0.5.0`, `advanced-v0.1.0`. Vendor compliance claims and
`COMPATIBILITY-MATRIX.md` cells reference these tags. Matches parent ADR 0063 step 4.

## Changesets

Version bumps are computed by [Changesets](https://github.com/changesets/changesets)
with one `fixed` group per stream. This is **not yet wired up** — it lands in Task
4 of the release-trains plan. Until then, contributors do not need to produce
Changesets entries; once the tooling lands, the flow becomes "one `.changeset/*.md`
per stream touched by a PR."

## How to pick a stream when editing

1. Determine which stream path(s) your diff lands under, using the table above.
2. If exactly one stream: that's the stream. Your CHANGELOG entry goes in
   `changelogs/<stream>.md` under `## [Unreleased]`.
3. If multiple streams: one entry per stream. This is the normal case for changes
   that span spec prose and companion fixtures.
4. If neither (repo-level docs, CI, tooling): no CHANGELOG entry required; note
   the change in the PR description.

## References

- [Release-trains implementation plan](thoughts/plans/2026-04-16-wos-release-trains.md)
- [Compatibility matrix](COMPATIBILITY-MATRIX.md)
- [Parent formspec ADR 0063 — Release trains by tier](../thoughts/adr/0063-release-trains-by-tier.md)
- [Architecture review §4.4](thoughts/archive/reviews/2026-04-16-architecture-review-handoff.md)
