# ADR-0005: Change-home shape — detached-only

> Status: Proposed — re-scoped 2026-08-18 for the spec-generator programme ([ADR-0008](0008-spec-generator-programme.md)); pending operator acceptance of the narrowed decision
> Date: 2026-08-17

## Context

In-place vs detached mode is encoded five ways (path equality, an explicit bool, ambient env vars via `unsafe set_var`, a WIT field, duplicated strings) and running `emery` in the wrong directory silently scaffolds state ([architecture-review.md](../architecture-review.md) D2). The [addendum](../architecture-review-addendum.md) adds P15: `init` / `--upgrade` and `Ctx` are in-place-only — init in a detached home nests a `.emery/` project inside the change home, and `EMERY_DETACHED` then makes `Ctx::load` return a fabricated empty `ProjectConfig` that ignores the file just written (no version floor, no adapter, no platforms).

[ADR-0008](0008-spec-generator-programme.md) writes specifications, not a delivery change home. Publication's in-place fast path, product-checkout bindings, and `allow-in-place` WIT fields belong to the frozen build programme. The live question is: where do `spec.md` / `design.md` live, and can `init` silently scaffold in the wrong directory.

## Options

- **A. Detached-only.** Every change home is detached-shaped; the product checkout is an explicit binding, never ambient anchoring.
- **B. Keep both modes**, fixing each encoding.
- **C. One output home (this programme).** `emery init` scaffolds one directory for source bindings and the spec set. There is no in-place/detached distinction because there is no product checkout to be in-place *in*. Running in an unrelated directory does not silently create state. Option A remains the standing preference when the build programme needs a change home again.

## Decision (proposed)

**Option C for this programme.** One shape, one scaffold, typed re-entry. Do not spend this programme encoding five ways to tell in-place from detached.

Option A is the preferred answer for the deferred build programme (it still kills the null-object `ProjectConfig` and the env-var channel). It is not a week of work that gates the spec skeleton.

## Deletions

The in-place/detached distinction from the *live* operator surface; `EMERY_DETACHED` / `EMERY_CHANGE_ROOT` / `EMERY_PROJECT_ROOT` as live policy; the `allow-in-place` WIT field (the target world is gone in this programme anyway). Concept-count effect: removes the in-place/detached distinction — one fewer thing to know.

## Consequences

Single-repo projects that later build will gain an explicit binding step (Option A's cost) when that programme opens. Publication's in-place fast path is re-derived then, as a binding property, not a home shape.

## Revisit trigger

Opening the build programme reopens Option A vs C for *delivery* state. Independently: a UX measurement showing the output-home location is a dominant friction source would reopen the ergonomics (not the single-shape rule).
