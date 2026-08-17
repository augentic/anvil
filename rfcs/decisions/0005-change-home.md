# ADR-0005: Change-home shape — detached-only

> Status: Proposed — pending operator acceptance
> Date: 2026-08-17

## Context

In-place vs detached mode is encoded five ways (path equality, an explicit bool, ambient env vars via `unsafe set_var`, a WIT field, duplicated strings) and running `emery` in the wrong directory silently scaffolds state ([architecture-review.md](../architecture-review.md) D2). The [addendum](../architecture-review-addendum.md) adds P15: `init` / `--upgrade` and `Ctx` are in-place-only — init in a detached home nests a `.emery/` project inside the change home, and `EMERY_DETACHED` then makes `Ctx::load` return a fabricated empty `ProjectConfig` that ignores the file just written (no version floor, no adapter, no platforms).

## Options

- **A. Detached-only.** Every change home is detached-shaped; the product checkout is an explicit binding, never ambient anchoring.
- **B. Keep both modes**, fixing each encoding.

## Decision (proposed)

**Option A** (the standing D2 recommendation). One shape kills the null-object `ProjectConfig`, path-equality `is_detached`, the env-var channel, and silent scaffold-in-any-directory. `init` becomes a change-home scaffold with typed re-entry (addendum P15 recommendation).

## Deletions

The in-place mode, all five mode encodings, `EMERY_DETACHED` / `EMERY_CHANGE_ROOT` / `EMERY_PROJECT_ROOT`, the `allow-in-place` WIT field, the null-object config branch. Concept-count effect: removes the in-place/detached distinction from the operator surface entirely — one fewer thing to know.

## Consequences

Single-repo projects gain one explicit binding step where in-place was previously implicit. Publication's in-place fast path (single-member set on a clean product repo) is re-derived as a binding property, not a home shape.

## Revisit trigger

None anticipated; a UX measurement showing the explicit binding step is a dominant friction source would reopen the ergonomics (not the single shape).
