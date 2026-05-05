# Transcript — `/spec:plan --orchestrate polish-pass --shape update-existing --auto-merge`

This transcript pins the dialogue and shell-outs the umbrella runs against a populated multi-project hub when sources are unused. The discovery brief falls back to baseline accumulation in `.specify/workspace/<peer>/specs/` to surface polish opportunities; the operator approves two slices and the umbrella runs all seven steps in one pass. Every shell-out uses post-1F+1G v1 verbs.

```text
$ /spec:plan --orchestrate polish-pass --shape update-existing --auto-merge

Pre-flight
  shape:        update-existing
  hub:          shop-platform/.specify/project.yaml (hub: true; capability omitted)
  specify:      0.x.y on PATH
  gh:           v2.x.y on PATH
  initiative:   polish-pass (kebab-case ok)

  Shape `update-existing` forbids --from, --against, and --source.
  None supplied. Continuing.

---

## Step 1 — Brief

initiative.md is absent.

  $ specify initiative create polish-pass
  ok: wrote initiative.md

Shape `update-existing` carries no sources. Suggested default body:

  ---
  name: polish-pass
  inputs: []
  ---

  A polish pass extending existing capabilities on both registered
  projects. No new APIs, no new screens — only tightening rough edges
  on flows that already shipped.

Accept the default body? [Y/n] y
ok: wrote initiative.md (operator can $EDITOR to refine before step 3)

---

## Step 2 — Registry

  $ specify registry validate
  ok: 2 projects valid (omnia-backend, vectis-mobile)
  ok: every entry has a description

Multi-project registry; descriptions complete. Continuing.

---

## Step 3 — Plan

  $ /spec:plan polish-pass

  ── /spec:plan polish-pass ──

  $ specify plan create polish-pass
  ok: scaffolded plan.yaml

  Step 3(a) — Discovery
    No --from / --against / --source supplied.
    Reading initiative.md:inputs → empty.
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
    $ specify plan add tighten-auth-error-messages --description "Polish the auth flow's error messages to match the platform style guide. Delta-targets the `user-auth` baseline spec on omnia-backend."
    ok: appended `tighten-auth-error-messages` to plan.yaml

    Slice 2: theme-picker-a11y-label      (depends-on: —)
    > accept
    $ specify plan add theme-picker-a11y-label --description "Add the missing accessibility label on the theme-picker control. Delta-targets the `settings-screen` baseline spec on vectis-mobile."
    ok: appended `theme-picker-a11y-label` to plan.yaml

  Step 3(d) — Assignment

  | # | Entry                          | Project        | Rationale                                                  |
  |---|--------------------------------|----------------|------------------------------------------------------------|
  | 1 | tighten-auth-error-messages    | omnia-backend  | Baseline spec affinity: `user-auth` exists on omnia-backend. |
  | 2 | theme-picker-a11y-label        | vectis-mobile  | Baseline spec affinity: `settings-screen` on vectis-mobile.  |

    $ specify plan amend tighten-auth-error-messages --project omnia-backend
    ok: tighten-auth-error-messages.project = omnia-backend
    $ specify plan amend theme-picker-a11y-label --project vectis-mobile
    ok: theme-picker-a11y-label.project = vectis-mobile

  Wrote .specify/plans/polish-pass/proposal.md

  Step 4 — Validate
    $ specify plan validate
    PASS

  Done. Next steps:
    - specify plan status
    - /spec:execute --loop

---

## Step 4 — Execute

  $ /spec:execute --loop

  ## /spec:execute — polish-pass

  ### Initiative: polish-pass
  Progress: done 0, in-progress 0, pending 2, blocked 0, failed 0, skipped 0 (total 2)

  ---

  Self-heal: no in-progress entries found.

  # specify plan next → { "next": "tighten-auth-error-messages", "project": "omnia-backend", ... }

  Routing: tighten-auth-error-messages → omnia-backend (.specify/workspace/omnia-backend/)

  ### Processing: tighten-auth-error-messages

  Step 1/3: define ✓
  Step 2/3: build ✓
  Step 3/3: merge
    Auto-commit: git commit -m "specify: merge tighten-auth-error-messages"
    Status: done

  ---

  # specify plan next → { "next": "theme-picker-a11y-label", "project": "vectis-mobile", ... }

  Routing: theme-picker-a11y-label → vectis-mobile (.specify/workspace/vectis-mobile/)

  ### Processing: theme-picker-a11y-label

  Step 1/3: define ✓
  Step 2/3: build ✓
  Step 3/3: merge ✓
    Status: done

  ---

  ## /spec:execute — polish-pass — terminated

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

## Step 6 — Land (--auto-merge)

  $ specify workspace merge
  specify: workspace merge — polish-pass (specify/polish-pass)

    omnia-backend   merged                    PR #62   https://github.com/org/omnia-backend/pull/62
    vectis-mobile   merged                    PR #34   https://github.com/org/vectis-mobile/pull/34

  2 merged, 0 would-merge, 0 pending-checks, 0 failed-checks, 0 closed, 0 no-branch, 0 branch-pattern-mismatch, 0 failed.

---

## Step 7 — Finalize

  $ specify initiative finalize
  specify: initiative finalize — polish-pass (specify/polish-pass)

    omnia-backend         merged                   PR #62   https://github.com/org/omnia-backend/pull/62
    vectis-mobile         merged                   PR #34   https://github.com/org/vectis-mobile/pull/34

  2 merged, 0 unmerged, 0 closed, 0 no-branch, 0 branch-pattern-mismatch, 0 dirty, 0 failed.

  Initiative `polish-pass` finalized.
    archived plan: /…/shop-platform/.specify/archive/plans/polish-pass-20260428.yaml
    archived dir:  /…/shop-platform/.specify/archive/plans/polish-pass-20260428

---

## /spec:plan --orchestrate — polish-pass — landed

  Brief:    .specify/archive/plans/polish-pass-20260428/initiative.md
  Plan:     .specify/archive/plans/polish-pass-20260428.yaml
  Registry: omnia-backend, vectis-mobile (unchanged)
  PRs:      omnia-backend#62 (merged), vectis-mobile#34 (merged)

  Re-running /spec:plan --orchestrate polish-pass will report
  plan-not-found from `specify initiative finalize` and exit 0.
```

## Invariants pinned by this transcript

- **Verb hygiene.** Every shell-out is a post-1F+1G v1 verb: `specify initiative {create, finalize}`, `specify plan {create, add, amend, validate}`, `specify registry validate`, `specify workspace {sync, push, merge}`. No retired verbs.
- **No `--from` / `--source` / `--against`.** Pre-flight enforces this; the dispatch is unambiguous when shape is `update-existing`.
- **`inputs:` is empty in the brief.** Discovery falls back to baseline accumulation; the `.specify/workspace/<peer>/specs/` trees are the only signal.
- **No registry mutation.** `registry.yaml` is byte-identical between input and output. The 2B registry-proposal sub-step does not fire.
- **No contract change.** The polish pass does not cross the API boundary, so propose surfaces only two slices — one per project. The two implementation entries each carry `project:` written by the assignment step.
- **Idempotent re-entry.** Re-running the umbrella with the same flags after a successful finalize exits zero with `plan-not-found`.
