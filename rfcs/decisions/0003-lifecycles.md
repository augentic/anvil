# ADR-0003: Lifecycles — one spec-mining loop vs definition + delivery

> Status: Proposed — pending operator acceptance
> Date: 2026-08-17

## Context

RFC-104 implemented a second product: `crates/system` (~6.7k lines) with its own layout, schemas, survey/extract orchestration, judgments, facts, status projection, handoffs, and renderers ([architecture-review.md](../architecture-review.md) P1). The seams between the two lifecycles are broken in both directions: review does not freeze delivery inputs (P2, P5, P7), and — per the [addendum](../architecture-review-addendum.md) — delivery holds a live lease on the definition tree, so later archaeology revokes in-flight waves (P12); review attests a bag of independently current files, not one observation (P11); definition status can declare completion vacuously (P13); and the definition loop's own persistence has no generation identity (S41–S45). A one-wave intent-only engagement requires hand-authored `scope.yaml` / `coverage.yaml`, failing the product's "extremely simple" test outright.

## Options

- **A. One lifecycle.** Legacy code, documentation, contracts, captures, and designs are ordinary *sources* feeding one spec-mining loop. "Archaeology" is running the loop with code/doc sources. The architecture model (as-is, target, migration) becomes an optional *projection* of the same evidence corpus.
- **B. Keep two lifecycles**, closing every seam finding (P2, P5, P6, P7, P11, P12, P13, S41–S45) and making definition optional (standing P1 recommendation).

## Decision (proposed)

**Option A.** The volume of seam findings is the argument: two lifecycles require the review-to-delivery authorization chain (standing Cut 3) *plus* the inverse fence (P12) *plus* a second generation system for definition state (S41–S45) — machinery that exists only because there are two homes. One lifecycle makes the mutual-revocation class unrepresentable. The paid archaeology deliverable survives as a projection (report + diagrams) over the mined evidence, not as a separate product.

## Deletions

`crates/system` as a parallel workflow engine (survey kernel merges with the delivery source axis, per standing P1's "one survey/extract kernel"); the definition home layout, its events root, `system status`; hand-authored `scope.yaml` / `coverage.yaml` as preconditions; the sixteen-category `HandoffWave`; the `current_definition` fence. Concept-count effect: removes the definition-home noun family (scope, coverage, handoff, wave review) from the operator surface — the largest single simplification available.

## Consequences

The reviewed-wave gesture (`system review`) is replaced by specification review in the one loop; engagements that want a formal architecture sign-off get it as a review of the projected architecture document, recorded as an ordinary decision fact. Migration-wave planning becomes plan topology, not a separate artifact family.

## Revisit trigger

A paid engagement whose deliverable is the architecture model alone, at a scale where the projection approach measurably fails (e.g. coverage accounting across >50 sources needs its own workflow).
