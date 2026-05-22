---
name: specify-build
description: Build the active in-progress slice by loading its target adapter's build brief and running it. Use when `/spec:execute` parks on a build failure, when running build standalone after `/spec:refine`, or to retry the brief after fixing a failing task; not when the slice has yet to be refined (use `/spec:refine`) or has already merged.
argument-hint: "[slice-name]"
---

# Build skill

Build the active `in-progress` slice. The skill body is shared with the `/spec:execute` loop — when the loop runs build, it loads this same body. Both the loop and standalone breakouts resolve the active slice from `specify plan next`, hold the same plan lock, and walk the same target build brief. Deterministic work — slice resolution, lifecycle reads, target manifest lookup, brief resolution, slice transitions — flows through the `specify` CLI; this body drives the agent-side reading and execution of the build brief.

The skill refuses only on slice lifecycle. Synthesis review tags (`[unknown]`, `[conflict]`, `[divergence]`) carried in `spec.md` are review signals, not build blockers — operators may have hand-edited the spec after `/spec:refine` surfaced the tags, and the brief proceeds against whatever spec is on disk.

## Bindings

```text
$SLICE                  = active in-progress plan entry's slice name (from `specify plan next`)
$TARGET                 = active slice's target adapter (from `specify plan next`)
$PROJECT                = active slice's workspace project (workspace mode only)
$SLICE_DIR              = .specify/slices/$SLICE/
$LOG_PATH               = brief-captured stdout/stderr on failure (target-specific path)
$SPECIFY_PLAN_LOCK_HELD = "1" when invoked from /spec:execute (parent already holds the plan lock)
```

`$SLICE` defaults to the active `in-progress` entry. When `[slice-name]` is supplied it MUST equal that active entry; mismatches refuse to preserve the single-active-slice invariant.

## Critical Path

1. **Resolve the active slice.** Run `specify plan next --format json`. If `[slice-name]` was passed, validate it matches the returned `in-progress` entry; refuse on mismatch. Read `$TARGET` (and `$PROJECT` in workspace mode) from the same response.
2. **Acquire the plan lock when invoked standalone.** When `$SPECIFY_PLAN_LOCK_HELD = 1` the parent loop holds it — do not re-acquire. Otherwise acquire `.specify/plan.lock` (workspace root in workspace mode); see [plan-lock.md](../../references/plan-lock.md).
3. **Workspace routing.** When `.specify/project.yaml` carries `workspace: true`, run `specify workspace sync $PROJECT` and `chdir` into `.specify/workspace/$PROJECT/` before continuing. Single-repo mode is a no-op.
4. **Refuse on slice lifecycle.** Read `$SLICE_DIR/.metadata.yaml`. Proceed only when `status: refined`. Pre-`refined` (e.g. `refining`) → halt with hint pointing at `/spec:refine`. Post-`refined` (`built`, `merged`, `dropped`) → halt with "no rebuild needed" / "already merged".
5. **Load the target build brief.** Run `specify target resolve $TARGET --format json` and read `adapters/targets/$TARGET/briefs/build.md` from the resolved path. The brief carries the orchestration (omnia: crate / test / guest / review; vectis: core / iOS / Android / `composition.yaml` regen; contracts: format-dispatched author-import-verify). Follow it linearly; do not invoke retired writer or reviewer skills directly.
6. **Stop on failure.** A non-zero exit anywhere in the brief's verify-repair loop or post-build gate emits a structured stop hint (see § Stop hint contract). The slice stays at `refined`; do not transition forward. The plan lock releases on process exit.
7. **Transition on success.** Run `specify slice transition $SLICE built --format json`. The CLI stamps `.metadata.yaml`. Return control to the caller; `/spec:execute` (when present) advances to merge, otherwise the operator runs `/spec:merge $SLICE`.

## Stop hint contract

A build failure surfaces a stop hint as the body's final output — a single structured message the operator (or the parent loop) can act on without re-deriving context:

- `slice` — slice name from `specify plan next`.
- `phase` — `build`.
- `failing-task` — the `tasks.md` checkbox (or sub-step) that exited non-zero (e.g. `cargo test`, `clippy`, `cargo build --target wasm32-wasip2`).
- `log-path` — absolute path to the captured stdout/stderr (`$SLICE_DIR/.build-log` is the default; targets MAY override).
- `next-action` — typically `re-run /spec:build $SLICE after fix`, optionally a target-specific suggestion (e.g. `inspect $LOG_PATH for clippy lints`).

Render the hint as the final visible output of the run. Do not call `specify slice transition` on the failure path — the slice stays `refined` so the loop (or a re-invocation) re-enters cleanly. Do not write to `plan.yaml`; the per-entry status stays `in-progress`, which is the v1 wire signal that execution is parked rather than drained.

## Plan-lock semantics

This body shares the plan lock with `/spec:execute`. Detection is the env var `SPECIFY_PLAN_LOCK_HELD=1`: when set, the parent (almost always `/spec:execute`) holds the lock and this body MUST NOT re-acquire — re-entrant `flock(LOCK_EX | LOCK_NB)` would error and abort the loop. When unset (the standalone-breakout path) this body acquires the lock at the workspace root in workspace mode or at `.specify/plan.lock` in single-repo mode, releases on process exit, and exposes `SPECIFY_PLAN_LOCK_HELD=1` to any sub-invocations it spawns. Full detail and the acquisition snippet live in [plan-lock.md](../../references/plan-lock.md).

## Guardrails

- **Refuse only on slice lifecycle, never on synthesis tags.** `[unknown]` / `[conflict]` / `[divergence]` in `spec.md` are review signals; the build proceeds against whatever spec is on disk.
- **Never write `plan.yaml` from this body.** Per-entry transitions are owned by `specify plan next` (writes `in-progress`) and `specify slice merge` (writes `done`). `/spec:build` only writes the slice's `.metadata.yaml` via `specify slice transition`.
- **Never hand-edit `.metadata.yaml` or archive paths.** See [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state).
- **Never re-acquire the plan lock when `SPECIFY_PLAN_LOCK_HELD=1`.** Re-entrant acquisition aborts the loop.
- **Never invoke retired writer or reviewer skills directly.** The target build brief carries those bodies inline; calling them out-of-band bypasses brief orchestration and breaks shape-injection guarantees.

## References

- [plan-lock.md](../../references/plan-lock.md) — env-var detection and the `flock` snippet shared with `/spec:execute` and `/spec:merge`.
- [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state) — single-writer rules for `.metadata.yaml`, `plan.yaml`, archive paths.
- `adapters/targets/<target>/briefs/build.md` — the orchestration this skill loads and executes (omnia, vectis, contracts).
