# Transcript — `/change:plan --orchestrate polish-pass --shape update-existing`

This transcript pins the dialogue and shell-outs the umbrella runs against a populated multi-project hub when sources are unused. The discovery brief falls back to baseline accumulation in `.specify/workspace/<peer>/specs/` to surface polish opportunities; the operator approves two slices, the umbrella pushes PRs, stops for operator merge, and finalizes on re-entry. Every shell-out uses post-1F+1G v1 verbs.

```text
$ /change:plan --orchestrate polish-pass --shape update-existing

Pre-flight
  shape:        update-existing
  hub:          shop-platform/.specify/project.yaml (hub: true; capability omitted)
  specify:      0.x.y on PATH
  gh:           v2.x.y on PATH (read-only PR observation)
  change:       polish-pass (kebab-case ok)

  Shape `update-existing` forbids --from, --against, and --source.
  None supplied. Continuing.

---

## Step 1 — Brief

change.md is absent.

  $ specify change create polish-pass
  ok: wrote change.md

Shape `update-existing` carries no sources. Suggested default body:

  ---
  name: polish-pass
  inputs: []
  ---

  A polish pass extending existing capabilities on both registered
  projects. No new APIs, no new screens — only tightening rough edges
  on flows that already shipped.

Accept the default body? [Y/n] y
ok: wrote change.md (operator can $EDITOR to refine before step 3)

---

## Step 2 — Registry

  $ specify registry validate
  ok: 2 projects valid (omnia-backend, vectis-mobile)
  ok: every entry has a description

Multi-project registry; descriptions complete. Continuing.

---

## Step 3 — Plan

  $ /change:plan polish-pass

  ── /change:plan polish-pass ──

  $ specify change plan create polish-pass
  ok: scaffolded plan.yaml

  Step 3(a) — Discovery
    No --from / --against / --source supplied.
    Reading change.md:inputs → empty.
    Falling back to baseline accumulation across workspace clones.
    Wrote .specify/plans/polish-pass/discovery.md

    No `## Proposed registry topology` section in discovery.md →
    skipping greenfield bootstrap (registry already populated).

  Step 3(b) — Sync peers
    $ specify workspace sync
    ok: refreshed .specify/workspace/omnia-backend/
    ok: refreshed .specify/workspace/vectis-mobile/
    Wrote .specify/plans/polish-pass/workspace.md

  Step 3(c) — Propose
    Slice 1: tighten-auth-error-messages  (depends-on: —)              [accept] [edit] [reject] [abort]
    > accept
    $ specify change plan add tighten-auth-error-messages --description "Polish the auth flow's error messages to match the platform style guide. Delta-targets the `user-auth` baseline spec on omnia-backend."
    ok: appended `tighten-auth-error-messages` to plan.yaml

    Slice 2: theme-picker-a11y-label      (depends-on: —)
    > accept
    $ specify change plan add theme-picker-a11y-label --description "Add the missing accessibility label on the theme-picker control. Delta-targets the `settings-screen` baseline spec on vectis-mobile."
    ok: appended `theme-picker-a11y-label` to plan.yaml

  Step 3(d) — Assignment

  | # | Entry                          | Project        | Rationale                                                  |
  |---|--------------------------------|----------------|------------------------------------------------------------|
  | 1 | tighten-auth-error-messages    | omnia-backend  | Baseline spec affinity: `user-auth` exists on omnia-backend. |
  | 2 | theme-picker-a11y-label        | vectis-mobile  | Baseline spec affinity: `settings-screen` on vectis-mobile.  |

    $ specify change plan amend tighten-auth-error-messages --project omnia-backend
    ok: tighten-auth-error-messages.project = omnia-backend
    $ specify change plan amend theme-picker-a11y-label --project vectis-mobile
    ok: theme-picker-a11y-label.project = vectis-mobile

  Wrote .specify/plans/polish-pass/proposal.md

  Step 4 — Validate
    $ specify change plan validate
    PASS

  Done. Next steps:
    - specify change plan status
    - /change:execute --loop

---

## Step 4 — Execute

  $ /change:execute --loop

  ## /change:execute — polish-pass

  ### Change: polish-pass
  Progress: done 0, in-progress 0, pending 2, blocked 0, failed 0, skipped 0 (total 2)

  ---

  Self-heal: no in-progress entries found.

  # specify change plan next → { "next": "tighten-auth-error-messages", "project": "omnia-backend", ... }

  Routing: tighten-auth-error-messages → omnia-backend (.specify/workspace/omnia-backend/)

  ### Processing: tighten-auth-error-messages

  Step 1/3: define ✓
  Step 2/3: build ✓
  Step 3/3: merge
    Auto-commit: git commit -m "specify: merge tighten-auth-error-messages"
    Status: done

  ---

  # specify change plan next → { "next": "theme-picker-a11y-label", "project": "vectis-mobile", ... }

  Routing: theme-picker-a11y-label → vectis-mobile (.specify/workspace/vectis-mobile/)

  ### Processing: theme-picker-a11y-label

  Step 1/3: define ✓
  Step 2/3: build ✓
  Step 3/3: merge ✓
    Status: done

  ---

  ## /change:execute — polish-pass — terminated

  ### Final state
  Progress: done 2, in-progress 0, pending 0, blocked 0, failed 0, skipped 0 (total 2)

  Completion: all-done

---

## Step 5 — Push

  $ specify workspace push
  specify: workspace push — polish-pass

    omnia-backend   pushed       specify/polish-pass   PR #62
    vectis-mobile   pushed       specify/polish-pass   PR #34

  2 pushed, 0 created, 0 up-to-date. 0 failed.

---

## Step 6 — PR handoff

Open PRs on `specify/polish-pass`:

  omnia-backend   specify/polish-pass    PR #62    https://github.com/org/omnia-backend/pull/62
  vectis-mobile   specify/polish-pass    PR #34    https://github.com/org/vectis-mobile/pull/34

Merge these PRs through the forge UI or an explicit hand-run `gh pr merge`,
then re-run /change:plan --orchestrate polish-pass to finalize.

---

## /change:plan --orchestrate — polish-pass — paused

  Brief:    change.md
  Plan:     plan.yaml (2 changes, all `done`)
  PRs:      omnia-backend#62 (open), vectis-mobile#34 (open)

  Next action: merge PRs, then re-run /change:plan --orchestrate
  polish-pass --shape update-existing.
```

## Operator merges PRs

The operator merges both PRs on github.com (squash, conventional commit titles) or with explicit `gh pr merge` commands they run themselves. The umbrella only observes this state on the next run.

## Run 2 — re-entry, runs step 7 only

```text
$ /change:plan --orchestrate polish-pass --shape update-existing

Pre-flight
  shape:        update-existing
  hub:          shop-platform/.specify/project.yaml (hub: true; capability omitted)
  specify:      0.x.y on PATH
  gh:           v2.x.y on PATH (read-only PR observation)
  change:       polish-pass (kebab-case ok)

  Shape `update-existing` forbids --from, --against, and --source.
  None supplied. Continuing.

---

## Step 1 — Brief

change.md is present. Skipping.

## Step 2 — Registry

  $ specify registry validate
  ok: 2 projects valid (no changes)

## Step 3 — Plan

plan.yaml is present and every entry is in a terminal state
(done × 2). Skipping /change:plan.

## Step 4 — Execute

Plan is fully terminal. Skipping /change:execute.

## Step 5 — Push

  $ specify workspace push
  specify: workspace push — polish-pass

    omnia-backend   up-to-date
    vectis-mobile   up-to-date

  0 pushed, 0 created, 2 up-to-date. 0 failed.

## Step 6 — PR handoff

Querying PRs on `specify/polish-pass`:

  $ gh pr list --head specify/polish-pass --state all --json number,state,merged,headRefName
  omnia-backend   PR #62    state=MERGED, merged=true
  vectis-mobile   PR #34    state=MERGED, merged=true

Every PR is `MERGED` on remote. Continuing to step 7.

---

## Step 7 — Finalize

  $ specify change finalize
  specify: change finalize — polish-pass (specify/polish-pass)

    omnia-backend         merged                   PR #62   https://github.com/org/omnia-backend/pull/62
    vectis-mobile         merged                   PR #34   https://github.com/org/vectis-mobile/pull/34

  2 merged, 0 unmerged, 0 closed, 0 no-branch, 0 branch-pattern-mismatch, 0 dirty, 0 failed.

  Change `polish-pass` finalized.
    archived plan: /…/shop-platform/.specify/archive/plans/polish-pass-20260428.yaml
    archived dir:  /…/shop-platform/.specify/archive/plans/polish-pass-20260428

---

## /change:plan --orchestrate — polish-pass — landed

  Brief:    .specify/archive/plans/polish-pass-20260428/change.md
  Plan:     .specify/archive/plans/polish-pass-20260428.yaml
  Registry: omnia-backend, vectis-mobile (unchanged)
  PRs:      omnia-backend#62 (merged), vectis-mobile#34 (merged)

  Re-running /change:plan --orchestrate polish-pass will report
  plan-not-found from `specify change finalize` and exit 0.
```

## Invariants pinned by this transcript

- **Verb hygiene.** Every shell-out is a post-Phase-3 verb: `specify change {create, finalize}`, `specify change plan {create, add, amend, validate}`, `specify registry validate`, `specify workspace {sync, push}`, `gh pr list`. No retired verbs.
- **No `--from` / `--source` / `--against`.** Pre-flight enforces this; the dispatch is unambiguous when shape is `update-existing`.
- **`inputs:` is empty in the brief.** Discovery falls back to baseline accumulation; the `.specify/workspace/<peer>/specs/` trees are the only signal.
- **No registry mutation.** `registry.yaml` is byte-identical between input and output. The 2B registry-proposal sub-step does not fire.
- **No contract change.** The polish pass does not cross the API boundary, so propose surfaces only two slices — one per project. The two implementation entries each carry `project:` written by the assignment step.
- **PR merge is outside orchestration.** The transcript shows the umbrella stopping with open PRs in run 1, then observing both PRs as `MERGED` in run 2. It never calls `specify workspace merge` or `gh pr merge`.
- **Idempotent re-entry.** Re-running the umbrella with the same flags after a successful finalize exits zero with `plan-not-found`.
