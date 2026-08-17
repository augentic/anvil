# ADR-0006: Remediation shape — walking-skeleton rebuild vs six-cut refactor

> Status: Proposed — decide after the ADR-0001 and ADR-0002 spikes
> Date: 2026-08-17

## Context

The corrective programme in [architecture-review.md](../architecture-review.md) is a six-cut refactor across ~130k lines (engine ~101k Rust, ~27k adapter prose, ~12k adapter Rust), each cut preserving behavior nobody depends on yet — the product is pre-production. The [addendum](../architecture-review-addendum.md)'s second pass found ~65 further findings across authoring, definition, the executor, the seam, and the adapters, which is direct evidence the refactor cost was underestimated on the first pass: finding density did not fall with a second look.

Omnia — the hand-written comparison point — is ~30k lines for an entire runtime platform. Its coherence came from being built to a settled, bounded architecture. Repair does not produce that property; building to one does.

**Kept kernels** (sound on both reviews, ported not rewritten): the content-addressed snapshot store and CID identity (`project::snapshot`, `project::workspace`), the RFC-90 engine-owned build phase machine, the artifact parsers (collapsed to one fail-closed AST per addendum A17), the adapter operations traits, the adapter prose corpus (~27k lines, ports nearly intact), the refinement-stage boundary, ultrathin skills.

## Options

- **A. Walking-skeleton rebuild.** Write [target-architecture.md](../target-architecture.md) v1, then build the spine new — store (ADR-0001), one loop (ADR-0003), executor, four verbs — reusing the kept kernels, porting adapters behind the traits, deleting the rest. The old tree remains readable reference until parity.
- **B. Six-cut refactor** as written in the standing review, greenfield-in-place for Cuts 1–2 only.
- **C. Hybrid**: rebuild the spine (state, authoring, executor — the regions where both reviews found structural defects), refactor the periphery (transport grammar, artifacts, adapters) in place.

## Decision (proposed)

**Option C, biased toward A.** The regions where finding density is highest (authoring generation machine, executor/wave/merge transaction, definition persistence, journal authority) are exactly the regions Cut 1–2 would rewrite anyway; rewriting them *against the target document* instead of *against the finding list* is the difference between building to an architecture and repairing toward one. The periphery (clap grammar, diagnostics, artifact types, adapter crates) is healthy enough to move.

Decide finally after the two spikes: ADR-0001's store spike measures how much executor code survives contact with the new authority model; ADR-0002's component-seam spike measures what the Wasm-primary journey costs in CI and how much of the resolution/dual-provider layer simply deletes.

The [capability conservation ledger](../capability-conservation.md) is the deletion gate. A legacy implementation may leave only after its replacement capability passes the ledger's acceptance evidence, or an accepted ADR explicitly deletes the capability and records the consequence. Walking-skeleton greenness alone is not parity for deferred capabilities.

## Deletions

Under C: the journal-authority reducers, the authoring mode machine, the current execute loop, `crates/system` as a product (per ADR-0003), and — per the accepted ADR-0002 (Wasm-primary) — the **native provider and the adapter resolution matrix** (the Wasm seam is kept and hardened, not deleted) — replaced, not migrated. Pre-1.0 hard reset; no compatibility layer, no migration framework (already repo policy).

## Consequences

A period where the new spine and old tree coexist; the walking-skeleton CI journey (see [remediation-plan.md](../remediation-plan.md) Phase 2) is the only definition of progress during it. Feature work stays frozen until the skeleton is green.

The work happens **in place, in this repository** — new spine crates, legacy crates quarantined under a decrease-only ratchet, deleted as parity lands (see the plan's "Working in place" discipline). The one new-repo trigger considered (consolidating `emery-adapters` as compiled-in crates) dissolved with ADR-0002's Wasm-primary acceptance.

## Revisit trigger

If the ADR-0001 spike shows the existing execute loop adapts to the store with <20% structural change, Option B becomes cheaper and this ADR should be re-decided before the spine rebuild starts.
