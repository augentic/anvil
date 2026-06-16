# Stop conditions

`/spec:execute` halts the loop when `specify plan status` says so. The CLI owns stop classification — it projects plan entries, the active slice's `metadata.yaml` lifecycle, and the journal tail into a `next-action` of `refine|build|merge <slice>`, `stop <reason>`, or `drained` — and the skill renders that output verbatim. Failure stops leave the active entry `in-progress` for re-entry; `drained` is the only successful exit. Every other phase return (success on `refine` / `build` / `merge`) falls through to the next iteration.

The blocks below are the exact text rendering of `specify plan status` (one per stop). Re-running `/spec:execute` after a stop asks `plan status` again and resumes from the same active entry — no flags, no resume tokens.

## Stop shape

Every failure stop renders the same shape — a `stop:` line with the closed reason, the entry context, the failure detail from the journal event payload when one exists, a one-line operator hint, and (when a single command makes progress) the literal re-entry command:

```text
stop: <reason>
  slice: <slice-name>
  project: <project-or-"-">
  detail: <reason-from-the-failing-phase's-journal-event>
hint: <one-line re-entry hint>
resume: <literal-command-or-skill-invocation>
```

The closed reason set: `plan-not-approved`, `refine-failed`, `build-failed`, `merge-conflict`, `slice-dropped`, `merge-incomplete`, `stuck`. The three loop stops are below; the rest are pre-loop or repair conditions rendered the same way (`plan-not-approved` hints the literal Gate 1 command; `slice-dropped` and `stuck` hint a plan amendment; `merge-incomplete` flags a merged slice whose entry never got its `done` stamp). The `resume:` line is the RM-15 re-entry field — `plan status --format json` carries it as `resume` beside `current-step` / `last-completed`; it is omitted for the repair-shaped stops (`stuck`, `slice-dropped`) where no single command makes progress.

## 1. Refine or build failure

`/spec:refine` hard-failed (extract failed, schema rejection — the newest journal terminal is `slice.synthesize.failed`) or `/spec:build` returned non-zero (compiler error, failing test, exhausted repair budget — `slice.build.failed`). The slice stays `refining` / `refined`; the plan entry stays `in-progress`.

```text
stop: build-failed
  slice: <slice-name>
  project: <project-or-"-">
  detail: <reason-from-slice.build.failed>
hint: Fix the failure, then retry /spec:build for the slice. The plan entry stays in-progress.
resume: /spec:build <slice-name>
```

(`refine-failed` renders identically with `/spec:refine` in the hint.) The phase skill's own stop-hint contract (failing task, log path) remains the place to look for the full failure context; `plan status` carries the journal reason as `detail`.

Re-entry contract: the stop is sticky while the failure is the newest journal terminal for the awaited phase — re-running `/spec:execute` re-renders it. Retry the failed phase through its breakout as the hint directs (`/spec:build <slice>` after the fix); a successful completion journals `slice.build.succeeded`, clearing the stop, and the next `/spec:execute` dispatches forward (`built` → merge).

## 2. Merge baseline conflict

`/spec:merge` reported a baseline conflict (`slice.merge.failed`) — typically the slice's delta touches the same `.specify/specs/<adapter>/spec.md` lines another change already merged. The plan entry stays `in-progress`; the slice lifecycle stays `built`.

```text
stop: merge-conflict
  slice: <slice-name>
  project: <project-or-"-">
  detail: <reason-from-slice.merge.failed>
hint: Resolve the baseline conflict (or drop the slice), then retry /spec:merge. The plan entry stays in-progress until the merge lands.
resume: /spec:merge <slice-name>
```

Re-entry contract: the stop is sticky while `slice.merge.failed` is the newest journal terminal for the slice — resolve the conflict, then retry through the `/spec:merge` breakout as the hint directs; a successful merge stamps `done` and the next `/spec:execute` moves on. If the operator chose to drop the slice instead, they run `/spec:drop <slice> reason "<rationale>"`, amend the plan entry via `specify plan amend <entry> ...` as needed to unblock the queue, then re-run `/spec:execute` — not `specify plan transition <slice> done`, which is reserved for successful merges. (After a drop, `plan status` renders `stop: slice-dropped` until the entry is amended.)

## 3. Drained

No `pending` or `in-progress` entries remain. This is the only clean exit. The CLI renders the literal closing line:

```text
drained — run /spec:finalize <name>
```

`<name>` is the plan name. `/spec:finalize` is the next operator step: it re-validates every per-entry `done`, pushes branches with `specify workspace push`, then runs `specify plan archive` to archive. `/spec:execute` itself never pushes and never archives — those are finalize's responsibility. Opening and merging the pull requests is operator-owned and happens outside Specify.

## What is NOT a stop

The following return cleanly into the next iteration:

| Phase return | Loop behaviour |
|---|---|
| `/spec:refine` success | `plan status` dispatches `build` for the same entry. |
| `/spec:build` success | `plan status` dispatches `merge` for the same entry. |
| `/spec:merge` success | The entry is `done` (stamped by `specify slice merge`); `plan status` moves to the next eligible entry. |
| `/spec:refine` surfaces `[unknown]` / `[conflict]` / `[divergence]` tags | Tags are review signals; lifecycle still reaches `refined`; loop continues to `/spec:build`. |
| Workspace residue commit succeeded | Continue. |

A phase failure only stops the loop while it is the *newest* journal terminal for the awaited phase — once the operator moves the slice past it (e.g. a successful re-build after `build-failed`), `plan status` dispatches forward again.

## Lock release

Every stop path releases the plan lock when the `specify plan lock -- <cmd>` child exits — the wrapper holds the lock only for its child's lifetime, so the lock is released the moment the driver returns. Relying on process-exit semantics is the contract; nothing unlocks explicitly. (The CLI's `plan-lock-not-held` refusal — see [`plan-lock.md`](plan-lock.md) — is the runtime guard that no driver re-enters without re-acquiring.)
