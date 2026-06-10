# #8 — Step-through breakout mid-execute

Pins the contract that `/spec:execute` re-entry is implicit. The operator cancels the loop on slice 2, runs `/spec:build` directly as a breakout, then re-invokes `/spec:execute`. No `--continue`, no `--one`, no `--until`; the only state mutation between runs is whatever `/spec:build` itself wrote.

## Starting state

- `plan.yaml.lifecycle == approved` (Gate 1 stamped).
- Slice 1 `user-list-search-filter` already `done`.
- Slice 2 `group-list-search-filter` is `in-progress`; its slice lifecycle is `refined` (refine landed before the operator cancelled).
- Slice 3 `audit-list-search-filter` is still `pending`.

## Trace

1. **Operator runs `/spec:execute`.**
   - Acquires `.specify/plan.lock` via the snippet in [`../../../../../plugins/spec/references/plan-lock.md`](../../../../../plugins/spec/references/plan-lock.md).
   - `specify plan next` returns slice 2 (already `in-progress`).
   - Slice lifecycle is `refined`; loop skips `/spec:refine` and dispatches `/spec:build`.
   - Operator interrupts mid-build with Ctrl-C. The shell holding `flock` exits; the lock auto-releases. The plan entry stays `in-progress`; the slice lifecycle stays `refined`.

2. **Operator runs `/spec:build group-list-search-filter` standalone.**
   - The breakout body re-acquires the plan lock via the same snippet.
   - Resolves the active slice via `specify plan next`; confirms slice 2 is `in-progress`.
   - Runs the build phase to completion against the slice; `/spec:build` transitions the slice lifecycle from `refined` to `built` on success.
   - Releases the lock on shell exit.

3. **Operator re-runs `/spec:execute`.**
   - Acquires the plan lock.
   - `specify plan next` returns slice 2 (still `in-progress`).
   - Slice lifecycle is `built`; loop skips both `/spec:refine` and `/spec:build` and dispatches `/spec:merge` directly.
   - On merge success, `/spec:merge` runs `specify plan transition group-list-search-filter done`.
   - Next iteration: `specify plan next` returns slice 3; loop runs `/spec:refine` → `/spec:build` → `/spec:merge` end-to-end.
   - Next iteration: drained.

## Terminal state

- Every per-entry `status: done`.
- Closing hint printed: `drained — run /spec:finalize search-filter-rollout`.
- `.specify/plan.lock` removed.

## Stress test

- Re-entry without `--continue` reads `plan.yaml` and slice `metadata.yaml` only; no resume token; no skill-side state file.
- Breakout reuses the exact lock snippet `/spec:execute` uses — no second-tier "breakout lock" exists.
- The skill body never advances slice 2's per-entry status; only `/spec:merge` writes `done`.
