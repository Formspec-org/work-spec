# WOS Stream Compatibility Matrix

This matrix lists the **known-good pairings** across WOS's four release streams.
It is hand-authored — a row is added when a cross-stream combination has been
tested together and recorded as a supported pairing. A combination not listed
here is not automatically broken, but it also has not been validated as a whole.

For the scope of each stream and the tag convention, see
[`RELEASE-STREAMS.md`](RELEASE-STREAMS.md).

## Matrix

| Kernel | Governance | AI    | Advanced | Notes |
|--------|------------|-------|----------|-------|
| 1.0.x  | 1.0.x      | 0.5.x | 0.1.x    | Current. Treat as the sole supported combination until a second row lands. The activation-criteria + durable-obligation block ([ADR 0096](thoughts/adr/0096-shared-activation-criteria-and-durable-obligations.md)) composes within this row: it lands inside `governance` and adds no new stream or envelope marker, so it claims under the existing `wos-governance@1.0` cell (`$wosWorkflow@1.0 [governance(obligations)]`). See the [claims map](RELEASE-STREAMS.md#claims-map-post-adr-0076). |

## How to read a row

Each cell is a SemVer x-range (e.g. `1.0.x` means "any 1.0 patch release").
A row is a **claim of compatibility** — all streams at the listed versions have
been exercised together and the combination is known to work.

A vendor who implements WOS declares compliance by pinning the streams they
consume:

> "This system is conformant with `wos-kernel@1.0 + wos-ai@0.5` under the
> 2026-04-20 compatibility matrix row."

A vendor does **not** need to pin every stream — only the ones they consume.
A processor that is purely kernel-level claims only `wos-kernel@X.Y`.

## Known-broken pairings

When a combination is discovered to be incompatible, a new row is added with an
`x-` prefix on the offending stream range — for example `x-0.6.x` in the AI
column would mean "this governance/kernel pair is known to break when paired
with any 0.6.x AI release." An `x-`-prefixed row is a **negative claim**: do
not ship this combination.

(No such rows exist yet.)

## Lifecycle rules

1. **Additive only.** Rows are added; existing rows are not rewritten. If a
   supported pairing later turns out to be broken, append a new `x-` row rather
   than editing history.
2. **CI enforcement lands with Task 4** of the [release-trains
   plan](thoughts/plans/2026-04-16-wos-release-trains.md). Until then, this
   matrix is hand-maintained and staleness is caught by review.
3. **Breaking-change criteria per stream** are documented in each stream's
   CHANGELOG header — see [`changelogs/`](changelogs/).

## References

- [`RELEASE-STREAMS.md`](RELEASE-STREAMS.md) — stream → path mapping.
- [`changelogs/`](changelogs/) — per-stream CHANGELOGs with stability commitments.
- [Release-trains implementation plan](thoughts/plans/2026-04-16-wos-release-trains.md)
  — full rollout, including the CI staleness checker (Task 3.2, still open).
