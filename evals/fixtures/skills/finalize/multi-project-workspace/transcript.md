# multi-project-workspace — `/spec:finalize` archives a workspace-driven change across two projects

End-to-end variant of eval scenario #10. A workspace plan named `dark-mode` has two slices, one routed to `backend`, one to `mobile`. Both per-entry statuses are `done` after `/spec:execute` drained the loop. `specify workspace push` publishes both projects' `specify/dark-mode` branches to their respective `origin`, and `specify plan archive` archives the change cleanly in the same run. This is the success terminator for the workspace-driven path. Opening and merging the pull requests is an operator step done by hand outside Specify.

No forge client (`gh`) is involved; `specify workspace push` shells out to Git only.

## Transcript

```text
$ /spec:finalize dark-mode

Step 1 — Pre-flight
  name:         dark-mode (kebab-case ok)
  workspace:    /…/platform/.specify/project.yaml
  plan.yaml:    present (workspace)
  specify:      2.0.x on PATH

---

Step 2 — Drained check

  $ specify plan status --format json
  {
    "plan":        "dark-mode",
    "lifecycle":   "approved",
    "counts":      { "pending": 0, "in-progress": 0, "done": 2 },
    "active":      null,
    "next-action": "drained",
    "action":      "drained",
    "slice":       null,
    "project":     null
  }

  | # | Entry              | Project    | Status |
  |---|--------------------|------------|--------|
  | 1 | dark-mode-backend  | backend  | done   |
  | 2 | dark-mode-mobile   | mobile  | done   |

ok: plan drained (2 entries done).

---

Step 3 — Push

  $ specify workspace push
  specify: workspace push — dark-mode

    backend   pushed         specify/dark-mode
    mobile    pushed         specify/dark-mode

  2 pushed, 0 up-to-date, 0 local-only, 0 no-branch. 0 failed.

---

Step 4 — Archive

  $ specify plan archive
  Archived plan to /…/platform/.specify/archive/plans/dark-mode-20260521.yaml. Working directory moved to /…/platform/.specify/archive/plans/dark-mode-20260521.

---

Step 5 — Wrap-up summary

  Brief:    .specify/archive/plans/dark-mode-20260521/change.md
  Plan:     .specify/archive/plans/dark-mode-20260521.yaml
  Pushed:   backend → specify/dark-mode (origin), mobile → specify/dark-mode (origin)
  Next:     open a pull request for each pushed branch by hand (forge UI or `gh pr create`); Specify does not open or merge PRs.
  Post-merge tidy-ups (from change.md): none recorded.

  Change dark-mode finalized. Plan archived at .specify/archive/plans/dark-mode-20260521/.

  Re-running /spec:finalize dark-mode will find no active plan and report the change already archived.

Exit 0
```

## Invariants pinned

1. **Workspace is the working directory throughout.** `/spec:finalize` is invoked from the workspace; `specify workspace push` and `specify plan archive` both operate against the workspace `plan.yaml` (single-`plan.yaml` invariant preserved at the workspace).
2. **Every step runs in order across both projects.** Pre-flight → drained → push (one verb, per-project status table) → archive → wrap-up. No project is dropped.
3. **`specify workspace push` is the sole push verb.** The skill does not loop `git push` itself; per-project routing is owned by the CLI verb.
4. **Push and archive happen in one invocation.** Both branches push, then `specify plan archive` runs; there is no PR-merge gate.
5. **The skill never creates, observes, or merges PRs.** Opening and merging both pull requests is an operator action outside Specify.
6. **`specify plan archive` is the sole archive writer.** No hand-`mv` into `.specify/archive/`; archive paths come from the CLI verb.
7. **Closing message matches the canonical wording.** The skill prints `Change dark-mode finalized. Plan archived at <path>` regardless of project count.
8. **Re-running after a successful finalize exits zero after confirming the archive.** Absence of an active plan is treated as already closed only after matching the archive path or prior transcript.
9. **Exit 0.** Successful workspace-driven finalize.
