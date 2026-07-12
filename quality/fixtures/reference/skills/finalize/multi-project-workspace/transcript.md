# multi-project workspace — `/spec:finalize` archives after publication

A workspace plan named `dark-mode` has two completed slices routed to `backend` and `mobile`. The operator has published both repositories through ordinary repository tooling before invoking finalize.

## Transcript

```text
$ /spec:finalize dark-mode

Step 1 — Drained check
  $ specify plan status --format json
  { "plan": "dark-mode", "action": "drained", "counts": { "pending": 0, "in-progress": 0, "done": 2 } }

Step 2 — Publication confirmation
  Operator confirmed all affected repositories are published and their required review workflow is complete.

Step 3 — Archive
  $ specify plan archive
  Archived plan to /…/platform/.specify/archive/plans/dark-mode-20260521.yaml.

Change dark-mode finalized. Plan archived at .specify/archive/plans/dark-mode-20260521/.
Exit 0
```

## Invariants pinned

1. The workspace remains the plan root throughout finalize.
2. Publication is operator-owned and complete before archive.
3. Finalize performs no Git or forge operation.
4. `specify plan archive` is the sole archive writer.
5. The closing message is independent of project count.
