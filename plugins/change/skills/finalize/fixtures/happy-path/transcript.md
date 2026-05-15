# happy-path — `/change:finalize` runs every step to a clean archive

Every plan entry is `done`, `specify workspace push` reports every project as `up-to-date` (PRs were pushed on a prior `/change:finalize` run that halted at step 4 for operator merge), `gh pr list` reports every PR as `MERGED`, and `specify change finalize` archives the change cleanly. This is the success terminator for the change lifecycle.

The transcript is lifted from the post-execute tail (steps 5–7) of the umbrella's `migrate-legacy/` fixture, re-headed to the new `/change:finalize` invocation surface and trimmed to the post-execute scope (steps 1–4 of the old umbrella, which authored and executed the plan, are owned by `/change:draft` and `/change:execute` and are out of scope here).

## Transcript

```text
$ /change:finalize migrate-foo

Pre-flight
  change:       migrate-foo (kebab-case ok)
  project root: /…/shop-platform/.specify/project.yaml
  plan.yaml:    present
  specify:      0.x.y on PATH
  gh:           v2.x.y on PATH (read-only PR observation)

---

## Step 2 — Plan terminality

  | # | Entry                | Project      | Status |
  |---|----------------------|--------------|--------|
  | 1 | migrate-foo-contract | —            | done   |
  | 2 | migrate-foo-backend  | foo-backend  | done   |
  | 3 | migrate-foo-mobile   | foo-mobile   | done   |

ok: every entry terminal (3 done).

---

## Step 3 — Push

  $ specify workspace push
  specify: workspace push — migrate-foo

    foo-backend   up-to-date
    foo-mobile    up-to-date

  0 pushed, 0 created, 2 up-to-date. 0 failed.

---

## Step 4 — PR observation

  $ gh pr list --head specify/migrate-foo --state all \
      --json number,state,merged,headRefName,url
  foo-backend   PR #41    state=MERGED  url=https://github.com/org/foo-backend/pull/41
  foo-mobile    PR #18    state=MERGED  url=https://github.com/org/foo-mobile/pull/18

ok: every PR MERGED.

---

## Step 5 — Finalize

  $ specify change finalize
  specify: change finalize — migrate-foo (specify/migrate-foo)

    foo-backend         merged                   PR #41   https://github.com/org/foo-backend/pull/41
    foo-mobile          merged                   PR #18   https://github.com/org/foo-mobile/pull/18

  2 merged, 0 unmerged, 0 closed, 0 no-branch, 0 branch-pattern-mismatch, 0 dirty, 0 failed.

  Change `migrate-foo` finalized.
    archived plan: /…/shop-platform/.specify/archive/plans/migrate-foo-20260428.yaml
    archived dir:  /…/shop-platform/.specify/archive/plans/migrate-foo-20260428

---

## Step 6 — Wrap-up summary

## /change:finalize — migrate-foo — landed

  Brief:    .specify/archive/plans/migrate-foo-20260428/change.md
  Plan:     .specify/archive/plans/migrate-foo-20260428.yaml
  PRs:      foo-backend#41 (merged), foo-mobile#18 (merged)
  Post-merge tidy-ups (from change.md): none recorded.

  Re-running /change:finalize migrate-foo will report plan-not-found
  from `specify change finalize` and exit 0.

Exit 0
```

## Invariants pinned

1. **Every step runs in order.** Pre-flight → terminality → push (idempotent: `up-to-date`) → PR observation (`MERGED` × 2) → finalize → wrap-up summary. No step is skipped.
2. **`specify workspace push` is idempotent on re-entry.** This run is the second invocation of the skill — the first opened the PRs and halted at step 4 for operator merge. The second push reports `up-to-date` for both projects.
3. **The skill never merges PRs.** Both PRs were merged externally by the operator between the two runs. The skill only observed the `MERGED` state on the second invocation.
4. **`specify change finalize` is the canonical archive.** The skill surfaces the CLI's per-project status table verbatim plus the archive paths. The archive layout and naming are owned by the verb.
5. **Re-running after a successful finalize exits zero with `plan-not-found`.** The verb's "already finalized" signal is forwarded by the skill.
6. **Exit 0.** Successful finalize.
