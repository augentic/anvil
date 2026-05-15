# pr-not-merged — `/change:finalize` halts when at least one PR is still open

A two-entry `dark-mode` plan, every entry `done`. `specify workspace push` succeeds for both projects. `gh pr list` shows `omnia-backend` PR #57 already `MERGED` (the operator merged it earlier) but `vectis-mobile` PR #29 is still `OPEN`. The skill halts at step 4 with the `pr-not-merged` classification, naming each open PR with its URL.

This fixture pins the `pr-not-merged` halt at step 4.

## Transcript

```text
$ /change:finalize dark-mode

Pre-flight
  change:       dark-mode (kebab-case ok)
  project root: /…/shop-platform/.specify/project.yaml
  plan.yaml:    present

---

## Step 2 — Plan terminality

  | # | Entry              | Project        | Status |
  |---|--------------------|----------------|--------|
  | 1 | dark-mode-backend  | omnia-backend  | done   |
  | 2 | dark-mode-mobile   | vectis-mobile  | done   |

ok: every entry terminal.

---

## Step 3 — Push

  $ specify workspace push
  specify: workspace push — dark-mode

    omnia-backend   up-to-date
    vectis-mobile   up-to-date

  0 pushed, 0 created, 2 up-to-date. 0 failed.

---

## Step 4 — PR observation

  $ gh pr list --head specify/dark-mode --state all \
      --json number,state,merged,headRefName,url
  omnia-backend   PR #57    state=MERGED  url=https://github.com/org/omnia-backend/pull/57
  vectis-mobile   PR #29    state=OPEN    url=https://github.com/org/vectis-mobile/pull/29

Halt: pr-not-merged.

  - vectis-mobile  PR #29  OPEN   https://github.com/org/vectis-mobile/pull/29

Next action: the operator merges each named PR through the forge UI
  or a hand-run `gh pr merge`, then re-runs /change:finalize dark-mode.
  This skill never invokes gh pr merge itself.

Exit 1
```

## Invariants pinned

1. **The skill never merges PRs.** Step 4 is observe-only; the operator merges through the forge UI or an explicit `gh pr merge` invocation outside this skill.
2. **The halt diagnostic names every non-`MERGED` PR with its URL.** A pasteable URL is the operator's primary cue — clicking it lands on the forge UI's merge button.
3. **`MERGED` PRs are listed but not flagged as halts.** `omnia-backend` PR #57 appears in the table; only the open PR triggers the halt.
4. **`specify change finalize` is not invoked.** The CLI guard (`unmerged PR`) would catch this anyway; the skill halts upstream so the operator-actionable cue (the URL list) appears earlier in the output.
5. **Re-entry is idempotent.** After the operator merges PR #29 externally, re-running `/change:finalize dark-mode` re-runs `gh pr list`, observes both PRs as `MERGED`, and continues to step 5.
