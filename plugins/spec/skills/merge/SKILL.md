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
```

`$SLICE` defaults to the active `in-progress` entry. When `[slice-name]` is supplied it MUST equal that active entry; mismatches refuse to preserve the single-active-slice invariant.

## Critical Path

1. **Hold the plan lock.** Drive this phase under `specify plan lock -- <cmd>` (the workspace lock in workspace mode, via `--plan-dir`) *before* any plan verb; see [plan-lock.md](../../references/plan-lock.md). When env var `SPECIFY_PLAN_LOCK_HELD=1` the parent loop already holds it and the CLI skips re-acquisition automatically. `specify plan next` and `specify slice merge run` are CLI-gated and refuse an unlocked driver with `plan-lock-not-held`.
2. **Resolve the active slice.** Run `specify plan next --format json`. If `[slice-name]` was passed, validate it matches the returned `in-progress` entry; refuse on mismatch. Read `$TARGET` (and `$PROJECT` in workspace mode) from the same response.
3. **Workspace routing.** When `.specify/project.yaml` carries `workspace: true`, route per [`../execute/references/workspace-routing.md`](../execute/references/workspace-routing.md) (sync, `chdir` into the slot, `SPECIFY_PLAN_DIR` export; restore on exit) — the export is how step 6's `done` stamp reaches the workspace plan. Single-repo mode is a no-op.
4. **Refuse if lifecycle is not `built`.** Read `$SLICE_DIR/metadata.yaml`. Halt on `refining` / `refined` with a hint pointing at `/spec:build`; halt on `merged` / `dropped` with "already finalised". Only `built` proceeds.
5. **Load and run the target merge brief.** Resolve `specify target resolve $TARGET --format json`; read `adapters/targets/$TARGET/briefs/merge.md`. The brief covers target-specific pre-merge gates (omnia: cargo + clippy + test + `cargo build --target wasm32-wasip2`; vectis: cap-matrix re-run; contracts: WASI tool against the slice). Pre-merge gate failure → emit a stop hint (§ Stop hint contract); slice stays `built`.
6. **Apply the merge through `specify slice merge run $SLICE --format json`.** The CLI runs the deterministic delta merge against `.specify/specs/`, promotes any slice-authored Decision Records under `$SLICE_DIR/decisions/` into the append-only catalogue at `.specify/decisions/DEC-NNNN-<slug>.md` (core, runs for every target; engine-assigned `DEC-NNNN` ids, supersede flips applied to named targets), transitions the slice to `merged`, archives `$SLICE_DIR` into `.specify/archive/YYYY-MM-DD-$SLICE/`, appends a `slice.archive.created` entry to the append-only outcome ledger in `.specify/journal.jsonl` (slice, touched-specs, outcome summary, merge SHA, and the promoted `decisions[]` ids), and stamps the plan entry's per-entry status to `done`. A non-zero exit on baseline conflict — or a `decision-supersede-orphan` whose target moved out of the baseline since refine — surfaces the offending paths; the slice stays `built` and the plan entry stays `in-progress`. Use `specify slice merge preview $SLICE` first when the operator asks to preview without writing; use `specify slice merge conflict-check $SLICE` for a baseline-conflict probe. The archived slice folder is a prunable cache (`specify archive prune`); the ledger plus git history of `.specify/specs/` and `.specify/decisions/` is the durable record.
7. **Run the post-merge target hook.** Some target merge briefs (notably `contracts`) re-run a validator against the promoted baseline (e.g. `specify extension run contract -- "$PROJECT_ROOT/contracts" --format json`). A failure here is a post-merge signal, not a rollback — the slice is already `merged` and the plan entry is already `done`. Surface the failure so the operator can queue a repair slice; do not attempt to revert.
8. **Surface the replay summary in the closing message** when `$SLICE_DIR/metadata.yaml` carried a `replay:` block (written by the target's optional build-time replay hook — see [`adapters/shared/target-hooks/replay/hook-contract.md`](../../../../adapters/shared/target-hooks/replay/hook-contract.md)). Capture the block before step 6 — `specify slice merge` archives the slice dir — and render one line in the close, e.g. `replay: 47 passed, 0 failed, 2 skipped`. `merge` does **not** auto-refuse on `failed > 0`; the operator decides whether to land. Missing block → omit the line; omission is not an error.

## Stop hint contract

When the pre-merge gate, the CLI delta merge, or the post-merge hook fails, emit a structured stop hint as the body's final output:

- `slice` — slice name from `specify plan next`.
- `phase` — `merge`.
- `failure-kind` — one of `pre-merge-gate`, `baseline-conflict`, `lifecycle-refused`, `post-merge-validator`.
- `paths` — for `baseline-conflict`: the conflicting baseline files reported by `specify slice merge`. For `pre-merge-gate` / `post-merge-validator`: the captured `$LOG_PATH`.
- `next-action` — `resolve and re-run /spec:merge $SLICE` for conflicts; `re-run /spec:build $SLICE` for gate failures classified as build regressions; `queue repair slice` for `post-merge-validator` drift.

Lifecycle invariants: `pre-merge-gate` and `baseline-conflict` leave the slice at `built` and the plan entry at `in-progress`. `post-merge-validator` runs after `specify slice merge` succeeded, so the slice is already `merged` and the plan entry is already `done` — the hint is observability, not a park.

## Guardrails

- **`specify slice merge` is the sole writer of per-entry `done`** and the only writer that transitions a slice to `merged` or moves a slice into `.specify/archive/`.
- **Lifecycle single-writer:** [shared guardrails](../../references/guardrails.md#single-writer-for-lifecycle-state).
- **Never auto-revert on a `post-merge-validator` failure.** The merge already landed; surface the failure and let the operator queue a repair slice. Reverting an archived slice is operator-only.
- **Never treat `specify slice merge conflict-check` success as a green light to skip the target merge brief's pre-merge gate.** `conflict-check` probes baseline drift; the brief gate covers target-specific build, lint, and validation.
- **Run the AskQuestion confirmation when invoked interactively** (i.e. `SPECIFY_PLAN_LOCK_HELD` unset). When invoked from `/spec:execute` the loop is its own confirmation seam; skip the prompt.

## References

- [shared guardrails](../../references/guardrails.md#single-writer-for-lifecycle-state) — single-writer rules for `metadata.yaml`, `plan.yaml`, archive paths.
- `adapters/targets/<target>/briefs/merge.md` — pre-merge gate and post-merge hook this skill drives (omnia, vectis, contracts).
