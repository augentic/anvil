# single-repo — `/spec:finalize` archives after publication

The plan is drained and the operator has already completed the repository's publication workflow. Finalize confirms those preconditions and archives the change without performing Git or forge operations.

## Transcript

```text
$ /spec:finalize fix-typo

Step 1 — Drained check
  $ specify plan status --format json
  { "plan": "fix-typo", "action": "drained", "counts": { "pending": 0, "in-progress": 0, "done": 1 } }

Step 2 — Publication confirmation
  Operator confirmed the completed repository changes are published.

Step 3 — Archive
  $ specify plan archive
  Archived plan to /…/user-svc/.specify/archive/plans/fix-typo-20260521.yaml.

Change fix-typo finalized. Plan archived at .specify/archive/plans/fix-typo-20260521/.
Exit 0
```

## Invariants pinned

1. `specify plan status` is the drainage gate.
2. Publication is operator-owned and complete before archive.
3. Finalize performs no Git or forge operation.
4. `specify plan archive` is the sole archive writer.
5. Re-entry after archive reports no active plan without recreating state.
