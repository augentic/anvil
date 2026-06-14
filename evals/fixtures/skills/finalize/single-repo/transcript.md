# single-repo — `/spec:finalize` runs every step to a clean archive

End-to-end variant of eval scenario #1. The plan is drained (`fix-typo` is N=1, the slice landed on the first `/spec:execute` pass), `specify workspace push` publishes the lone project's `specify/fix-typo` branch to its `origin`, and `specify plan archive` archives the change cleanly in the same run. This is the success terminator for the change lifecycle in the single-repo / N=1 mode. Opening the pull request and merging it is an operator step done by hand outside Specify.

No forge client (`gh`) is involved; `specify workspace push` shells out to Git only.

## Transcript

```text
$ /spec:finalize fix-typo

Step 1 — Pre-flight
  name:         fix-typo (kebab-case ok)
  project root: /…/user-svc/.specify/project.yaml
  plan.yaml:    present
  specify:      2.0.x on PATH

---

Step 2 — Drained check

  $ specify plan status --format json
  {
    "plan":        "fix-typo-rollout",
    "lifecycle":   "approved",
    "counts":      { "pending": 0, "in-progress": 0, "done": 1 },
    "active":      null,
    "next-action": "drained",
    "action":      "drained",
    "slice":       null,
    "project":     null
  }

  | # | Entry    | Project   | Status |
  |---|----------|-----------|--------|
  | 1 | fix-typo | user-svc  | done   |

ok: plan drained (1 entry done).

---

Step 3 — Push

  $ specify workspace push
  specify: workspace push — fix-typo

    user-svc   pushed         specify/fix-typo

  1 pushed, 0 up-to-date, 0 local-only, 0 no-branch. 0 failed.

---

Step 4 — Archive

  $ specify plan archive
  Archived plan to /…/user-svc/.specify/archive/plans/fix-typo-20260521.yaml. Working directory moved to /…/user-svc/.specify/archive/plans/fix-typo-20260521.

---

Step 5 — Wrap-up summary

  Brief:    .specify/archive/plans/fix-typo-20260521/change.md
  Plan:     .specify/archive/plans/fix-typo-20260521.yaml
  Pushed:   user-svc → specify/fix-typo (origin)
  Next:     open a pull request for specify/fix-typo by hand (forge UI or `gh pr create`); Specify does not open or merge PRs.
  Post-merge tidy-ups (from change.md): none recorded.

  Change fix-typo finalized. Plan archived at .specify/archive/plans/fix-typo-20260521/.

  Re-running /spec:finalize fix-typo will find no active plan and report the change already archived.

Exit 0
```

## Invariants pinned

1. **Every step runs in order.** Pre-flight → drained (`specify plan status` returns `action: drained`) → push → archive → wrap-up. No step is skipped.
2. **Drainage is computed by the CLI.** The skill never reads `plan.yaml`; it routes through the read-only `specify plan status` and matches on `action: drained` (never `plan next`, which is a lock-gated writer).
3. **`specify workspace push` publishes the branch only.** It pushes `specify/fix-typo` to `origin` and stops; it creates no remote repository and no pull request.
4. **The skill never creates, observes, or merges PRs.** Opening and merging the pull request is an operator action outside Specify, before or after this run.
5. **Push and archive happen in one invocation.** `specify plan archive` runs immediately after a successful push; there is no PR-merge gate.
6. **Closing message matches the canonical wording.** The skill prints `Change fix-typo finalized. Plan archived at <path>` so peer skills (notably `/spec:execute`) can match on the literal text.
7. **Re-running after a successful finalize exits zero after confirming the archive.** Absence of an active plan is treated as already closed only after matching the archive path or prior transcript.
8. **Exit 0.** Successful finalize.
