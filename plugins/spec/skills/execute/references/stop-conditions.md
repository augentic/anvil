# Stop conditions

`/spec:execute` halts the loop on exactly three conditions. Two of them leave the active entry `in-progress` for re-entry; the third is the only successful exit. Every other return from a phase skill (success on `refine` / `build` / `merge`) falls through to the next iteration.

The structured hints below are the exact strings the skill body prints on stderr (one per stop). Re-running `/spec:execute` after a stop resumes from the same active entry — no flags, no resume tokens.

## 1. Build non-zero exit

`/spec:build` returned non-zero (compiler error, failing test, exhausted repair budget). The slice stays `refined`; the plan entry stays `in-progress`.

```text
stop: build-failed
  slice: <slice-name>
  project: <project-or-"-">
  task: <task-id-from-build-outcome>
  log: <path-to-build-log-or-"-">
hint: Fix the failure, then re-run /spec:execute (or /spec:build to retry the
      failing task in isolation). The plan entry stays in-progress; the slice
      lifecycle stays where /spec:build left it.
```

Re-entry contract: the next `/spec:execute` calls `specify plan next`, sees the entry still `in-progress`, runs the snippet from [`plan-lock.md`](plan-lock.md), dispatches phase work based on the slice lifecycle. If the slice is `built` it skips straight to `/spec:merge`; if it is `refined`, it re-runs `/spec:build`.

## 2. Merge baseline conflict

`/spec:merge` reported a baseline conflict — typically the slice's delta touches the same `.specify/specs/<adapter>/spec.md` lines another change already merged. The plan entry stays `in-progress`; the slice lifecycle stays `built`.

```text
stop: merge-conflict
  slice: <slice-name>
  project: <project-or-"-">
  paths:
    - <conflicting-baseline-path>
    - ...
hint: Resolve the baseline conflict by editing the slice's delta specs (or
      amending the plan to drop the conflicting source), then re-run
      /spec:execute. The plan entry stays in-progress until the merge lands.
```

Re-entry contract: the next `/spec:execute` resumes at `/spec:merge` (slice lifecycle is still `built`). If the operator chose to drop the slice instead, they run `/spec:drop <slice> reason "<rationale>"`, amend the plan entry via `specify plan amend <plan> <slice> ...` as needed to unblock the queue, then re-run `specify plan next` (or `/spec:execute`) — not `specify plan transition <slice> done`, which is reserved for successful merges.

## 3. Drained

No `pending` or `in-progress` entries remain. This is the only clean exit. The closing hint is the literal string referenced in the SKILL.md:

```text
drained — run /spec:finalize <name>
```

`<name>` is the plan name from the drained envelope. `/spec:finalize` is the next operator step: it re-validates every per-entry `done`, pushes branches, observes PRs to `MERGED`, then runs `specify plan finalize` to archive. `/spec:execute` itself never pushes, never opens a PR, and never archives — those are finalize's responsibility.

## What is NOT a stop

The following return cleanly into the next iteration:

| Phase return | Loop behaviour |
|---|---|
| `/spec:refine` success | Continue to `/spec:build` for the same entry. |
| `/spec:build` success | Continue to `/spec:merge` for the same entry. |
| `/spec:merge` success | `specify plan transition <slice> done` (by `/spec:merge`), then continue to the next `specify plan next`. |
| `/spec:refine` surfaces `[unknown]` / `[conflict]` / `[divergence]` tags | Tags are review signals; lifecycle still reaches `refined`; loop continues to `/spec:build`. RFC-25 D2/D4. |
| Workspace residue commit succeeded | Continue. |

A `/spec:refine` hard failure (extract failed, schema rejection) leaves the slice `refining` and the plan entry `in-progress`; the skill body treats this the same as a build failure for stop-condition purposes — print `stop: refine-failed` with the same shape as build-failed and exit.

## Lock release

Every stop path releases the plan lock by virtue of the snippet's trailing edge — the bash session that holds `flock` exits, or the `python3` interpreter on the macOS fallback exits. The skill body never calls `flock -u` explicitly; relying on process-exit semantics is the contract.
