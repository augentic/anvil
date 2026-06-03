# multi-project-workspace — `/spec:finalize` archives a workspace-driven change across two projects

End-to-end variant of acceptance scenario #10. A workspace plan named `dark-mode` has two slices, one routed to `project-a`, one to `project-b`. Both per-entry statuses are `done` after `/spec:execute` drained the loop. `specify workspace push` reports both projects as `up-to-date` (the PRs were opened on a prior `/spec:finalize` run that halted at step 4 for operator merge). `gh pr view` reports both PRs as `MERGED`. `specify plan archive` archives the change cleanly. This is the success terminator for the workspace-driven path.

The PR-observation mock used by this fixture returns canned `state: MERGED` for `https://github.com/org/project-a/pull/57` and `https://github.com/org/project-b/pull/29`. No live `gh` invocation is performed.

## Transcript

```text
$ /spec:finalize dark-mode

Step 1 — Pre-flight
  name:         dark-mode (kebab-case ok)
  workspace:    /…/shop-platform/.specify/project.yaml
  plan.yaml:    present (workspace)
  specify:      2.0.x on PATH
  gh:           v2.x.y on PATH (read-only PR observation)

---

Step 2 — Drained check

  $ specify plan next --format json
  {
    "active":  null,
    "next":    null,
    "project": null,
    "reason":  "drained",
    "sources": null,
    "target":  null
  }

  | # | Entry              | Project    | Status |
  |---|--------------------|------------|--------|
  | 1 | dark-mode-backend  | project-a  | done   |
  | 2 | dark-mode-mobile   | project-b  | done   |

ok: plan drained (2 entries done).

---

Step 3 — Push

  $ specify workspace push
  specify: workspace push — dark-mode

    project-a   up-to-date
    project-b   up-to-date

  0 pushed, 0 created, 2 up-to-date. 0 failed.

---

Step 4 — PR observation loop

  $ gh pr view https://github.com/org/project-a/pull/57 \
      --json state,url,number
  project-a   PR #57    state=MERGED  url=https://github.com/org/project-a/pull/57

  $ gh pr view https://github.com/org/project-b/pull/29 \
      --json state,url,number
  project-b   PR #29    state=MERGED  url=https://github.com/org/project-b/pull/29

ok: every PR MERGED (2/2).

---

Step 5 — Archive

  $ specify plan archive
  Archived plan to /…/shop-platform/.specify/archive/plans/dark-mode-20260521.yaml. Working directory moved to /…/shop-platform/.specify/archive/plans/dark-mode-20260521.

---

Step 6 — Wrap-up summary

  Brief:    .specify/archive/plans/dark-mode-20260521/change.md
  Plan:     .specify/archive/plans/dark-mode-20260521.yaml
  PRs:      project-a#57 (merged), project-b#29 (merged)
  Post-merge tidy-ups (from change.md): none recorded.

  Change dark-mode finalized. Plan archived at .specify/archive/plans/dark-mode-20260521/.

  Re-running /spec:finalize dark-mode will find no active plan and report the change already archived.

Exit 0
```

## Invariants pinned

1. **Workspace is the working directory throughout.** `/spec:finalize` is invoked from the workspace; `specify workspace push` and `specify plan archive` both operate against the workspace `plan.yaml` (single-`plan.yaml` invariant preserved at the workspace).
2. **Every step runs in order across both projects.** Pre-flight → drained → push (one verb, per-project status table) → PR observation (per-project `gh pr view`, all `MERGED`) → archive → wrap-up. No project is dropped.
3. **`specify workspace push` is the sole push verb.** The skill does not loop `gh push` itself; per-project routing is owned by the CLI verb.
4. **PR observation is per-project but the halt classification is plan-wide.** Both PRs must reach `MERGED` before step 5 runs; one open PR halts the entire finalize.
5. **The skill never merges PRs.** Both PRs were merged externally by the operator between runs.
6. **`specify plan archive` is the sole archive writer.** No hand-`mv` into `.specify/archive/`; PR state is observed by `/spec:finalize` through `gh pr view`, and archive paths come from the CLI verb.
7. **Closing message matches the canonical wording.** The skill prints `Change dark-mode finalized. Plan archived at <path>` regardless of project count.
8. **Re-running after a successful finalize exits zero after confirming the archive.** Absence of an active plan is treated as already closed only after matching the archive path or prior transcript.
9. **Exit 0.** Successful workspace-driven finalize.
