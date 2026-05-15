# finalize-guard-refusal — `specify change finalize` refuses on a dirty workspace

Every plan entry is `done`. `specify workspace push` succeeds. `gh pr list` reports every PR as `MERGED`. The skill reaches step 5 and shells out to `specify change finalize` — which refuses because `git status --porcelain` is non-empty (the operator left an uncommitted edit in the hub root). The skill surfaces the CLI's guard diagnostic verbatim and halts.

This fixture pins the finalize-CLI-guard refusal — specifically the `dirty workspace` guard. The same surface applies to the other three guard refusals (`plan-not-found`, `non-terminal-entries`, `unmerged PR`); only the diagnostic body differs.

## Transcript

```text
$ /change:finalize dark-mode

Pre-flight
  change:       dark-mode (kebab-case ok)
  project root: /…/shop-platform/.specify/project.yaml
  plan.yaml:    present

---

## Step 2 — Plan terminality

ok: every entry terminal (2 done).

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
  vectis-mobile   PR #29    state=MERGED  url=https://github.com/org/vectis-mobile/pull/29

ok: every PR MERGED.

---

## Step 5 — Finalize

  $ specify change finalize
  Error: change-finalize-blocked

    guard: dirty workspace
    detail: git status --porcelain reports uncommitted changes in the
            hub root:
              M  README.md
              ?? notes/scratch.txt

    The change cannot be archived while the workspace has uncommitted
    residue. Commit, stash, or discard the dirty paths and re-run
    specify change finalize.

  exit 1

Halt: finalize CLI guard refusal (dirty workspace).

Next action: commit, stash, or discard the dirty paths listed in the
  diagnostic, then re-run /change:finalize dark-mode.

Exit 1
```

## Invariants pinned

1. **The CLI's guard diagnostic is surfaced byte-for-byte.** `Error: change-finalize-blocked`, the `guard:` line, and the `detail:` body appear verbatim in the operator's output. The skill never paraphrases.
2. **Guard refusals are the canonical safety net.** Plan-terminality (step 2) and PR observation (step 4) caught nothing here — the skill only learns the workspace is dirty when `specify change finalize` runs the guard. The redundancy at step 5 is intentional.
3. **The same shape applies to the other three guard refusals.** Swap `dirty workspace` for `plan-not-found`, `non-terminal-entries`, or `unmerged PR` and the transcript shape is unchanged — the diagnostic body changes, but the halt-handling rule (surface verbatim, point at the operator-actionable fix) is constant.
4. **No archive happens.** `plan.yaml`, `change.md`, and `.specify/plans/dark-mode/` stay where they are. The archive runs only when every guard passes.
5. **Re-entry is idempotent.** The operator commits or discards the residue, then re-runs `/change:finalize dark-mode`. The skill re-reads plan and PR state, re-runs the CLI guards, and finalizes when they pass.
