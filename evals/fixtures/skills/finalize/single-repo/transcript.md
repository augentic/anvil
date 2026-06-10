# single-repo — `/spec:finalize` runs every step to a clean archive

End-to-end variant of eval scenario #1. The plan is drained (`fix-typo` is N=1, the slice landed on the first `/spec:execute` pass), `specify workspace push` reports the lone project as `up-to-date` (the PR was opened on a prior `/spec:finalize` run that halted at step 4 for operator merge), `gh pr view` reports the PR as `MERGED`, and `specify plan archive` archives the change cleanly. This is the success terminator for the change lifecycle in the single-repo / N=1 mode.

The PR-observation mock used by this fixture returns canned `state: MERGED` for `https://github.com/org/user-svc/pull/14`. No live `gh` invocation is performed.

## Transcript

```text
$ /spec:finalize fix-typo

Step 1 — Pre-flight
  name:         fix-typo (kebab-case ok)
  project root: /…/user-svc/.specify/project.yaml
  plan.yaml:    present
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

  | # | Entry    | Project   | Status |
  |---|----------|-----------|--------|
  | 1 | fix-typo | user-svc  | done   |

ok: plan drained (1 entry done).

---

Step 3 — Push

  $ specify workspace push
  specify: workspace push — fix-typo

    user-svc   up-to-date

  0 pushed, 0 created, 1 up-to-date. 0 failed.

---

Step 4 — PR observation loop

  $ gh pr view https://github.com/org/user-svc/pull/14 \
      --json state,url,number
  user-svc   PR #14    state=MERGED  url=https://github.com/org/user-svc/pull/14

ok: every PR MERGED (1/1).

---

Step 5 — Archive

  $ specify plan archive
  Archived plan to /…/user-svc/.specify/archive/plans/fix-typo-20260521.yaml. Working directory moved to /…/user-svc/.specify/archive/plans/fix-typo-20260521.

---

Step 6 — Wrap-up summary

  Brief:    .specify/archive/plans/fix-typo-20260521/change.md
  Plan:     .specify/archive/plans/fix-typo-20260521.yaml
  PRs:      user-svc#14 (merged)
  Post-merge tidy-ups (from change.md): none recorded.

  Change fix-typo finalized. Plan archived at .specify/archive/plans/fix-typo-20260521/.

  Re-running /spec:finalize fix-typo will find no active plan and report the change already archived.

Exit 0
```

## Invariants pinned

1. **Every step runs in order.** Pre-flight → drained (`specify plan next` returns `reason: drained`) → push (idempotent: `up-to-date`) → PR observation (`MERGED`) → finalize → wrap-up. No step is skipped.
2. **Drainage is computed by the CLI.** The skill never reads `plan.yaml`; it routes through `specify plan next` and matches on `reason: drained`.
3. **`specify workspace push` is idempotent on re-entry.** This is the second invocation of the skill — the first opened the PR and halted at step 4 for operator merge. The second push reports `up-to-date`.
4. **The skill never merges PRs.** PR #14 was merged externally by the operator between the two runs. The skill only observed the `MERGED` state on the second invocation.
5. **`specify plan archive` is the canonical archive.** PR state is observed by `/spec:finalize` through `gh pr view`; the archive layout and naming are owned by the verb.
6. **Closing message matches the canonical wording.** The skill prints `Change fix-typo finalized. Plan archived at <path>` so peer skills (notably `/spec:execute`) can match on the literal text.
7. **Re-running after a successful finalize exits zero after confirming the archive.** Absence of an active plan is treated as already closed only after matching the archive path or prior transcript.
8. **Exit 0.** Successful finalize.
