# push-failed — `/change:finalize` halts when `specify workspace push` reports a `failed` project

A two-entry `dark-mode` plan where every entry is `done`. Step 3's `specify workspace push` succeeds for `omnia-backend` but fails for `vectis-mobile` because the remote rejected the push (no permissions on `specify/dark-mode` in that repository). The skill halts the run with the `failed` halt classification before reaching PR observation.

This fixture pins the `failed` halt classification at step 3.

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

    omnia-backend   pushed       specify/dark-mode   PR #57
    vectis-mobile   failed       specify/dark-mode   permission denied to push to git@github.com:org/vectis-mobile.git

  1 pushed, 0 created, 0 up-to-date. 1 failed.

Halt: failed.

  - vectis-mobile: permission denied to push to git@github.com:org/vectis-mobile.git

Next action: resolve the upstream issue (auth, network, missing
  remote, branch protection), then re-run /change:finalize dark-mode.
  specify workspace push is idempotent — already-pushed projects
  re-report as `up-to-date` on the next run.

Exit 1
```

## Invariants pinned

1. **`specify workspace push` is best-effort across projects, but a single `failed` halts the finalize run.** `omnia-backend` was pushed successfully and PR #57 exists on remote; the run still halts because `vectis-mobile` failed.
2. **The halt classification is `failed`, verbatim.** It mirrors the per-project status the verb reports.
3. **The diagnostic surfaces the verb's per-project status table verbatim.** No paraphrase; the operator pastes `permission denied to push to git@github.com:org/vectis-mobile.git` into their issue tracker if they need to.
4. **PR observation (step 4) is skipped.** The skill never reaches `gh pr list` — there is nothing to observe yet for `vectis-mobile`, and no point reporting `omnia-backend` PR #57 in isolation.
5. **`specify change finalize` is not invoked.** The CLI guard would refuse anyway (the `vectis-mobile` PR is missing), but the skill stops at step 3 to surface the actionable upstream cause rather than the downstream guard.
6. **Re-entry is idempotent.** After the operator fixes auth on `vectis-mobile`, re-running `/change:finalize dark-mode` re-runs `specify workspace push`; `omnia-backend` reports `up-to-date` and `vectis-mobile` pushes cleanly, then the run continues to step 4.
