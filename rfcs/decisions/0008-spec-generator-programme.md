# ADR-0008: Specification-generator programme

> Status: **Accepted** (operator decision, 2026-08-18).
> Date: 2026-08-18

## Context

The architecture review and the first remediation plan still described a four-verb product (spec / build / status / fix) with slice topology, a refine loop, collation, a SQLite-or-store spike, and platform hardening (D7/D8) *before* a walking skeleton. That is the destination in [product.md](../product.md), not a shippable increment.

The operator decision: ship a specification generator first. Mine sources (documentation, code, intent) into a coherent spec set describing the current system and how it could be rebuilt. That set is the product for Propellerhead. Build planning, building, and merging wait until the generator is reliable.

The v1 implementation splits source work into `survey` (leads catalog) and `extract` (per-lead evidence). The generator does not need a lead catalog or a topology compiler. Survey as a distinct operation is surplus.

Conservation of the current tree was still open: clone a second repo, quarantine crates in-tree as `crates-v1/`, or treat git history as the archive. Cloning recreates the two-product failure mode. In-tree `crates-v1/` leaves ~101k lines of exemplar on the branch agents work on.

## Options

- **A. Spec generator as the live programme.** Collapse survey into extract; CLI is `init` + `specify`; first artifacts `spec.md` / `design.md`; archive is tag `v1` + worktree; build/merge frozen.
- **B. Keep the four-verb walking skeleton** (spec → build → status → fix) as the definition of done, and treat the generator as a slice of that skeleton.
- **C. Generator plus slice topology and collation on day one** (the 2026-08-18 morning plan): extract → spec IR → synthesise → per-slice specs → refine loop → collation.

## Decision

**Option A.**

1. **Live journey.** Sources in, reviewable specification out. CONSTITUTION.md invariant 1 matches this journey until a later ADR restores build-and-merge as CI's definition of done.
2. **One source operation.** `wit/emery.wit` exports `extract` + `metadata` on the source world. No `survey`, no `lead` / `survey-result`, no target world in this programme. Extract returns specifications (or a claim set that *is* the spec IR).
3. **Operator surface.** `emery init` and `emery specify`. Other routes are deleted from the grammar, not hidden. The four-verb budget in product.md is the destination, not the live route budget.
4. **First artifacts.** `spec.md` and `design.md`. `composition.yaml` and `tasks.md` wait with the build programme.
5. **Conservation.** Tag `v1` (already created on this repo and `augentic/emery-adapters`) is the archive. Retrieve with `git worktree add ../emery-v1 v1`. No second repository. No `crates-v1/` on the live branch. Ports copy from the worktree after review; never `path =` depend on archive crates.
6. **Sequencing vs ADR-0002.** Wasm-primary stands. The original 0002 spike (survey/extract + build phase report) is not a gate on this skeleton. The CI rung is extract across the component seam. Capability profiles (D7) and dispatch budgets (D8) defer with the build programme.

## Deletions

Survey as a distinct WIT operation and operator/debug verb; live CLI verbs other than `init` / `specify`; `composition.yaml` / `tasks.md` as first-wave artifacts; in-tree `crates-v1/` quarantine; a second-repo archive. Concept-count effect: the live operator surface shrinks to two verbs and the nouns *source* and *specification* (gap and conflict remain visible *in* the specification). Slice, target, correction, and baseline stay destination nouns.

## Consequences

A period where the live product cannot build. That is the point. Agents implementing against [target-architecture.md](../target-architecture.md) must cite the spec-generator sections, not the deferred annex. A later programme ADR re-derives build planning / build / merge from the annex and tag `v1`.

## Revisit trigger

A Propellerhead engagement that cannot review or act on a spec set without slice topology, collation, or a build — which would reopen Option C or B, via a new ADR, not by silently widening the skeleton.
