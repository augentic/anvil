---
name: specify-merge
description: Merge the active built slice via `specify slice merge` — apply delta specs to the baseline, archive the slice, and stamp the plan entry `done`. Use when `/spec:execute` reaches the merge phase or for a manual breakout after `/spec:build` succeeds; not when the slice is still `refining` / `refined` (use `/spec:build`) or has already merged.
argument-hint: "[slice-name]"
---

# Merge skill

Merge the active `in-progress` slice. The skill body is shared with the `/spec:execute` loop — when the loop runs merge, it loads this same body. Both the loop and standalone breakouts resolve the active slice from `specify plan next`, hold the same plan lock, and walk the same target merge brief. Deterministic work — preview, baseline conflict detection, delta merge, lifecycle transition, archive move, and per-entry `done` stamping — runs through `specify slice merge`. This body drives the agent-side coordination: pre-merge gate from the target brief, the AskQuestion confirmation when interactive, and post-merge result rendering.

`specify slice merge` is the **sole writer of per-entry `done`**. No other plan or slice verb produces that value. `/spec:merge` is therefore the only place a plan entry can advance through merge — every other path leaves the entry `in-progress`.

## Bindings

```text
$SLICE                  = active in-progress plan entry's slice name (from `specify plan next`)
$TARGET                 = active slice's target adapter (from `specify plan next`)
$PROJECT                = active slice's workspace project (workspace mode only)
$SLICE_DIR              = .specify/slices/$SLICE/
$LOG_PATH               = brief-captured stdout/stderr on pre-merge or post-merge failure
$PROJECT_ROOT           = repo root (single-repo) or active workspace project slot (workspace mode)
$SPECIFY_PLAN_LOCK_HELD = "1" when invoked from /spec:execute (parent already holds the plan lock)
```

`$SLICE` defaults to the active `in-progress` entry. When `[slice-name]` is supplied it MUST equal that active entry; mismatches refuse to preserve the single-active-slice invariant.

## Critical Path

1. **Resolve the active slice.** Run `specify plan next --format json`. If `[slice-name]` was passed, validate it matches the returned `in-progress` entry; refuse on mismatch. Read `$TARGET` (and `$PROJECT` in workspace mode) from the same response.
2. **Acquire the plan lock when invoked standalone.** When `$SPECIFY_PLAN_LOCK_HELD = 1` the parent loop holds it — do not re-acquire. Otherwise acquire `.specify/plan.lock` (workspace root in workspace mode); see [plan-lock.md](../../references/plan-lock.md).
3. **Workspace routing.** When `.specify/project.yaml` carries `workspace: true`, run `specify workspace sync $PROJECT` and `chdir` into `.specify/workspace/$PROJECT/` before continuing. Single-repo mode is a no-op.
4. **Refuse if lifecycle is not `built`.** Read `$SLICE_DIR/.metadata.yaml`. Halt on `refining` / `refined` with a hint pointing at `/spec:build`; halt on `merged` / `dropped` with "already finalised". Only `built` proceeds.
5. **Load and run the target merge brief.** Resolve `specify target resolve $TARGET --format json`; read `targets/$TARGET/briefs/merge.md`. The brief covers target-specific pre-merge gates (omnia: cargo + clippy + test + `cargo build --target wasm32-wasip2`; vectis: cap-matrix re-run; contracts: WASI tool against the slice). Pre-merge gate failure → emit a stop hint (§ Stop hint contract); slice stays `built`.
6. **Apply the merge through `specify slice merge $SLICE --format json`.** The CLI runs the deterministic delta merge against `.specify/specs/`, transitions the slice to `merged`, archives `$SLICE_DIR` into `.specify/archive/YYYY-MM-DD-$SLICE/`, and stamps the plan entry's per-entry status to `done`. A non-zero exit on baseline conflict surfaces the conflict paths; the slice stays `built` and the plan entry stays `in-progress`. Use `--dry-run` first when the operator asks to preview without writing; use `--check-only` for a baseline-conflict probe.
7. **Run the post-merge target hook.** Some target merge briefs (notably `contracts`) re-run a validator against the promoted baseline (e.g. `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json`). A failure here is a post-merge signal, not a rollback — the slice is already `merged` and the plan entry is already `done`. Surface the failure so the operator can queue a repair slice; do not attempt to revert.
8. **Surface the fixture-replay summary in the closing message** when `$SLICE_DIR/.metadata.yaml` carried a `fixture-replay:` block (written by the target's optional build-time replay hook — see [`targets/omnia/briefs/build.md`](../../../../targets/omnia/briefs/build.md) § Fixture replay). Capture the block before step 6 — `specify slice merge` archives the slice dir — and render one line in the close, e.g. `fixture-replay: 47 passed, 0 failed, 2 skipped`. `merge` does **not** auto-refuse on `failed > 0`; the operator decides whether to land. Missing block → omit the line; omission is not an error.

## Stop hint contract

When the pre-merge gate, the CLI delta merge, or the post-merge hook fails, emit a structured stop hint as the body's final output:

- `slice` — slice name from `specify plan next`.
- `phase` — `merge`.
- `failure-kind` — one of `pre-merge-gate`, `baseline-conflict`, `lifecycle-refused`, `post-merge-validator`.
- `paths` — for `baseline-conflict`: the conflicting baseline files reported by `specify slice merge`. For `pre-merge-gate` / `post-merge-validator`: the captured `$LOG_PATH`.
- `next-action` — `resolve and re-run /spec:merge $SLICE` for conflicts; `re-run /spec:build $SLICE` for gate failures classified as build regressions; `queue repair slice` for `post-merge-validator` drift.

Lifecycle invariants: `pre-merge-gate` and `baseline-conflict` leave the slice at `built` and the plan entry at `in-progress`. `post-merge-validator` runs after `specify slice merge` succeeded, so the slice is already `merged` and the plan entry is already `done` — the hint is observability, not a park.

## Plan-lock semantics

This body shares the plan lock with `/spec:execute`. Detection is the env var `SPECIFY_PLAN_LOCK_HELD=1`: when set, the parent loop holds it and this body MUST NOT re-acquire — re-entrant `flock(LOCK_EX | LOCK_NB)` would error and abort the loop. When unset (the standalone-breakout path) this body acquires the lock at the workspace root in workspace mode or at `.specify/plan.lock` in single-repo mode, releases on process exit, and exposes `SPECIFY_PLAN_LOCK_HELD=1` to any sub-invocations. Full detail and the acquisition snippet live in [plan-lock.md](../../references/plan-lock.md).

## Guardrails

- **`specify slice merge` is the sole writer of per-entry `done`** and the only writer that transitions a slice to `merged` or moves a slice into `.specify/archive/`. Never hand-edit `plan.yaml`, `.metadata.yaml`, or archive paths. See [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state).
- **Never re-acquire the plan lock when `SPECIFY_PLAN_LOCK_HELD=1`.** Re-entrant acquisition aborts the loop.
- **Never auto-revert on a `post-merge-validator` failure.** The merge already landed; surface the failure and let the operator queue a repair slice. Reverting an archived slice is operator-only.
- **Never treat `--check-only` success as a green light to skip the target merge brief's pre-merge gate.** `--check-only` probes baseline drift; the brief gate covers target-specific build, lint, and validation.
- **Run the AskQuestion confirmation when invoked interactively** (i.e. `SPECIFY_PLAN_LOCK_HELD` unset). When invoked from `/spec:execute` the loop is its own confirmation seam; skip the prompt.

## References

- [plan-lock.md](../../references/plan-lock.md) — env-var detection and the `flock` snippet shared with `/spec:execute` and `/spec:build`.
- [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state) — single-writer rules for `.metadata.yaml`, `plan.yaml`, archive paths.
- `targets/<target>/briefs/merge.md` — pre-merge gate and post-merge hook this skill drives (omnia, vectis, contracts).
