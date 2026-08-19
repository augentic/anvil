# ADR-0006: Remediation shape — walking-skeleton rebuild vs six-cut refactor

> Status: **Accepted** (operator decision, 2026-08-18), narrowed by [ADR-0008](0008-spec-generator-programme.md). Not waiting on the ADR-0001 / ADR-0002 spikes as originally proposed.
> Date: 2026-08-17

## Context

The corrective programme in [architecture-review.md](../architecture-review.md) is a six-cut refactor across ~130k lines (engine ~101k Rust, ~27k adapter prose, ~12k adapter Rust), each cut preserving behavior nobody depends on yet — the product is pre-production. The [addendum](../architecture-review-addendum.md)'s second pass found ~65 further findings across authoring, definition, the executor, the seam, and the adapters, which is direct evidence the refactor cost was underestimated on the first pass: finding density did not fall with a second look.

Omnia — the hand-written comparison point — is ~30k lines for an entire runtime platform. Its coherence came from being built to a settled, bounded architecture. Repair does not produce that property; building to one does.

[ADR-0008](0008-spec-generator-programme.md) further narrows what is built: extract + synthesise, not a four-verb spine with an executor.

**Kept kernels for this programme** (ported, not rewritten): artifact parsers collapsed to one fail-closed spec AST (addendum A17); adapter source operations trait + prose embedder; synthesis / authority prose; the WIT component seam + Omnia hosting (ADR-0002); `error` / `diagnostics`. Snapshot/CID identity ports only if sources must be pinned directory trees. The RFC-90 phase machine, refinement-as-separate-stage, and target operations traits wait in the annex.

## Options

- **A. Walking-skeleton rebuild.** Write [target-architecture.md](../target-architecture.md) v1, then build the spine new — store (ADR-0001), one loop (ADR-0003), executor, four verbs — reusing the kept kernels, porting adapters behind the traits, deleting the rest. The old tree remains readable reference until parity.
- **B. Six-cut refactor** as written in the standing review, greenfield-in-place for Cuts 1–2 only.
- **C. Hybrid**: rebuild the spine (state, authoring, executor — the regions where both reviews found structural defects), refactor the periphery (transport grammar, artifacts, adapters) in place.
- **D. Narrowed A (this programme).** New crates for extract + synthesise only. The archive is tag `v1` + worktree, not an in-tree quarantine. The executor, four verbs, and store spike are not in this rebuild.

## Decision

**Option D.** The regions where finding density is highest are exactly the regions a refactor would rewrite; rewriting them *against the target document* is the difference between building to an architecture and repairing toward one. ADR-0008 deletes most of that region from *this* programme's scope, which makes a full-spine hybrid (C) the wrong amount of rebuild.

The original "decide after the two spikes" gate is withdrawn: ADR-0002 is already accepted (direction, not cost), and ADR-0001's merge-commit spike is a build-programme cost ([0001](0001-state-model.md) Option C).

The [capability conservation ledger](../capability-conservation.md) is the deletion gate for *live* generator capabilities. Deferred-annex capabilities are not Preserve-for-this-skeleton; they return with the build programme or an ADR that deletes them.

## Deletions

The six-cut refactor as the execution sequence; in-tree `crates-v1/` quarantine; coexistence of new spine and old crates on the live branch. Replaced, not migrated, when Phase 3 deletes them from the live tree: journal-authority reducers, the authoring mode machine, the execute loop, `crates/system` as a product (ADR-0003), the native provider and adapter resolution matrix (ADR-0002). Pre-1.0 hard reset; no compatibility layer, no migration framework (already repo policy).

## Consequences

The walking-skeleton CI journey ([remediation-plan.md](../remediation-plan.md) Phase 3) is the only definition of progress. Feature work stays frozen until the skeleton is green. Work happens in this repository; a clone is refused (ADR-0008).

## Revisit trigger

Opening the build programme reopens how much of the annex is rebuilt vs ported — a new ADR, not a silent widening of this skeleton. Independently: if extract + synthesise can be reached by deleting 80% of `crates/change` + `crates/slice` in place with the journey green in a week, Option B becomes cheaper for *that remaining 20%* and should be re-decided before Phase 3 spends a month on empty new crates.
