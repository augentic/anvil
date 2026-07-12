# #8 — Step-through breakout mid-execute

Pins the contract that `/spec:execute` re-entry is implicit. The operator cancels the loop on slice 2, runs `/spec:build` directly as a breakout, then re-invokes `/spec:execute`. No `--continue`, no `--one`, no `--until`; the only state mutation between runs is whatever `/spec:build` itself wrote.

## Starting state

- `plan.yaml.lifecycle == approved` (Gate 1 stamped).
- Slice 1 `user-list-search-filter` already `done`.
- Slice 2 `group-list-search-filter` is `in-progress`; its slice lifecycle is `refined` (refine landed before the operator cancelled).
- Slice 3 `audit-list-search-filter` is still `pending`.

## Trace

1. **Operator runs `/spec:execute`.**
   - Acquires the execution marker described by [`specify plan execute`](../../../../../../docs/reference/cli/plan.md#specify-plan-execute).
   - `specify plan status` returns `next-action: build group-list-search-filter` (slice 2 is `in-progress`, lifecycle `refined`); `specify plan next` confirms the active entry; the loop dispatches `/spec:build`.
   - Operator interrupts mid-build with Ctrl-C. The `specify plan lock` process holding the lock exits; the lock auto-releases. The plan entry stays `in-progress`; the slice lifecycle stays `refined`.

2. **Operator runs `/spec:build group-list-search-filter` standalone.**
   - The breakout body's first action is to re-acquire the plan lock via the same snippet — `specify plan next` is CLI-gated and would refuse an unlocked session with `plan-lock-not-held`.
   - Resolves the active slice via `specify plan next`; confirms slice 2 is `in-progress`.
   - Runs the build phase to completion against the slice; `specify slice build --phase finalize` transitions the slice lifecycle from `refined` to `built` on success.
   - Releases the lock on shell exit.

3. **Operator re-runs `/spec:execute`.**
   - Acquires the plan lock.
   - `specify plan status` returns `next-action: merge group-list-search-filter` (slice 2 still `in-progress`, lifecycle `built`); the loop dispatches `/spec:merge` directly — no skill-side lifecycle re-derivation.
   - On merge success, `specify slice merge run` stamps the entry `done`.
   - Next iteration: `specify plan status` names slice 3 (`refine audit-list-search-filter`); `specify plan next` claims it; loop runs `/spec:refine` → `/spec:build` → `/spec:merge` end-to-end.
   - Next iteration: `specify plan status` reports `drained`.

## Terminal state

- Every per-entry `status: done`.
- Closing hint printed: `drained — run /spec:finalize search-filter-rollout`.
- `.specify/plan.lock` removed.

## Stress test

- Re-entry without `--continue` is one `specify plan status` call — the CLI reads `plan.yaml`, slice `metadata.yaml`, and the journal tail; no resume token; no skill-side state file.
- Breakout reuses the exact lock snippet `/spec:execute` uses — no second-tier "breakout lock" exists; the CLI's `plan-lock-not-held` refusal backstops a snippet-skipping session.
- The skill body never advances slice 2's per-entry status; only `/spec:merge` writes `done`.
