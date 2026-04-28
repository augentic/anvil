# Transcript — `/spec:initiative create dark-mode --shape new-feature --from ./docs/dark-mode-spec.md`

This transcript pins the dialogue and shell-outs across **two runs** of the umbrella: the first run drives steps 1–6 and halts at step 6 (no `--auto-merge`), the second run resumes at step 7 once the operator has merged both PRs by hand. Every shell-out uses post-1F+1G v1 verbs.

## Run 1 — drive through to step 6, halt at step 6

```text
$ /spec:initiative create dark-mode \
    --shape new-feature \
    --from ./docs/dark-mode-spec.md

Pre-flight
  shape:        new-feature
  hub:          shop-platform/.specify/project.yaml (schema: hub, hub: true)
  specify:      0.x.y on PATH
  gh:           v2.x.y on PATH (advisory only — not required without --auto-merge)
  initiative:   dark-mode (kebab-case ok)

---

## Step 1 — Brief

.specify/initiative.md is absent.

  $ specify initiative create dark-mode
  ok: wrote .specify/initiative.md

Shape `new-feature` with one --from. Suggested default body:

  ---
  name: dark-mode
  inputs:
    - path: ./docs/dark-mode-spec.md
      kind: documentation
  ---

  Add a dark-mode theme to the storefront, spanning the backend
  (omnia-backend) and the mobile shells (vectis-mobile). ...

Accept the default body? [Y/n] y
ok: wrote .specify/initiative.md

---

## Step 2 — Registry

  $ specify registry validate
  ok: 2 projects valid (omnia-backend, vectis-mobile)
  ok: every entry has a description

Multi-project registry; descriptions complete. Continuing.

---

## Step 3 — Plan

  $ /spec:plan dark-mode --from ./docs/dark-mode-spec.md

  ── /spec:plan dark-mode ──

  $ specify plan create dark-mode
  ok: scaffolded .specify/plan.yaml

  Step 3(a) — Discovery
    Reading ./docs/dark-mode-spec.md (kind: documentation)…
    Wrote .specify/plans/dark-mode/discovery.md

    No `## Proposed registry topology` section in discovery.md →
    skipping greenfield bootstrap (registry already populated).

  Step 3(b) — Sync peers
    $ specify workspace sync
    ok: refreshed .specify/workspace/omnia-backend/
    ok: refreshed .specify/workspace/vectis-mobile/
    Wrote .specify/plans/dark-mode/workspace.md

  Step 3(c) — Propose
    Slice 1: dark-mode-contract  (sources: —; depends-on: —)              [accept] [edit] [reject] [abort]
    > accept
    $ specify plan add dark-mode-contract --description "Define the cross-project HTTP contract for the per-user theme preference: GET/PUT /v1/users/me/theme with body { theme: light | dark | system }."
    ok: appended `dark-mode-contract` to plan.yaml

    Slice 2: dark-mode-backend   (depends-on: dark-mode-contract)
    > accept
    $ specify plan add dark-mode-backend --depends-on dark-mode-contract --description "Persist a per-user `theme` setting. Implement GET and PUT endpoints from the contract. Default `system` for users who have never set the preference."
    ok: appended `dark-mode-backend` to plan.yaml

    Slice 3: dark-mode-mobile    (depends-on: dark-mode-contract)
    > accept
    $ specify plan add dark-mode-mobile --depends-on dark-mode-contract --description "Settings screen exposes a three-way picker bound to the theme-preference API. Every screen honours the active theme via design system tokens. Cache the preference locally for cold-launch."
    ok: appended `dark-mode-mobile` to plan.yaml

  Step 3(d) — Assignment

  | # | Entry                | Project        | Rationale                                                |
  |---|----------------------|----------------|----------------------------------------------------------|
  | 1 | dark-mode-contract   | —              | Cross-project contract; runs against the hub.            |
  | 2 | dark-mode-backend    | omnia-backend  | Description overlap: theme-preference API, persistence.  |
  | 3 | dark-mode-mobile     | vectis-mobile  | Description overlap: settings screens, theme-aware UI.   |

    $ specify plan amend dark-mode-backend --project omnia-backend
    ok: dark-mode-backend.project = omnia-backend
    $ specify plan amend dark-mode-mobile --project vectis-mobile
    ok: dark-mode-mobile.project = vectis-mobile

  Wrote .specify/plans/dark-mode/proposal.md

  Step 4 — Validate
    $ specify plan validate
    PASS

  Done. Next steps:
    - specify plan status
    - /spec:execute --loop

---

## Step 4 — Execute

  $ /spec:execute --loop

  ## /spec:execute — dark-mode

  ### Initiative: dark-mode
  Progress: done 0, in-progress 0, pending 3, blocked 0, failed 0, skipped 0 (total 3)

  ---

  Self-heal: no in-progress entries found.

  # specify plan next --format json → { "next": "dark-mode-contract", "project": null, ... }

  ### Processing: dark-mode-contract (greenfield)

  Step 1/3: define ✓
  Step 2/3: build ✓
  Step 3/3: merge
    Baseline updated: .specify/contracts/http/dark-mode.yaml ✓
    Status: done

  ---

  # specify plan next → { "next": "dark-mode-backend", "project": "omnia-backend", ... }

  Routing: dark-mode-backend → omnia-backend (.specify/workspace/omnia-backend/)

  ### Processing: dark-mode-backend (greenfield)

  Step 1/3: define ✓
  Step 2/3: build ✓
  Step 3/3: merge
    Auto-commit: git commit -m "specify: merge dark-mode-backend"
    Status: done

  ---

  # specify plan next → { "next": "dark-mode-mobile", "project": "vectis-mobile", ... }

  Routing: dark-mode-mobile → vectis-mobile (.specify/workspace/vectis-mobile/)

  ### Processing: dark-mode-mobile (greenfield)

  Step 1/3: define ✓
  Step 2/3: build ✓
  Step 3/3: merge ✓
    Status: done

  ---

  ## /spec:execute — dark-mode — terminated

  ### Final state
  Progress: done 3, in-progress 0, pending 0, blocked 0, failed 0, skipped 0 (total 3)

  Completion: all-done

---

## Step 5 — Push

  $ specify workspace push
  specify: workspace push — dark-mode

    omnia-backend   pushed       specify/dark-mode   PR #57
    vectis-mobile   pushed       specify/dark-mode   PR #29

  2 pushed, 0 created, 0 up-to-date. 0 failed.

---

## Step 6 — Land (--auto-merge not set)

Open PRs on `specify/dark-mode`:

  omnia-backend   specify/dark-mode    PR #57    https://github.com/org/omnia-backend/pull/57
  vectis-mobile   specify/dark-mode    PR #29    https://github.com/org/vectis-mobile/pull/29

--auto-merge not set; merge by hand on the forge (or run
`specify workspace merge`) and re-run /spec:initiative create dark-mode
to finalize.

---

## /spec:initiative — dark-mode — paused

  Brief:    .specify/initiative.md
  Plan:     .specify/plan.yaml (3 changes, all `done`)
  PRs:      omnia-backend#57 (open), vectis-mobile#29 (open)

  Next action: merge PRs (forge UI or `specify workspace merge`),
  then re-run /spec:initiative create dark-mode --shape new-feature
  --from ./docs/dark-mode-spec.md to finalize.
```

## Operator merges PRs by hand

The operator merges both PRs on github.com (squash, conventional commit titles). The umbrella does not see this happen — it only observes the result on the second run.

## Run 2 — re-entry, runs step 7 only

```text
$ /spec:initiative create dark-mode \
    --shape new-feature \
    --from ./docs/dark-mode-spec.md

Pre-flight
  shape:        new-feature
  hub:          shop-platform/.specify/project.yaml (schema: hub, hub: true)
  specify:      0.x.y on PATH
  gh:           v2.x.y on PATH
  initiative:   dark-mode (kebab-case ok)

---

## Step 1 — Brief

.specify/initiative.md is present. Skipping.

## Step 2 — Registry

  $ specify registry validate
  ok: 2 projects valid (no changes)

## Step 3 — Plan

.specify/plan.yaml is present and every entry is in a terminal state
(done × 3). Skipping /spec:plan.

## Step 4 — Execute

Plan is fully terminal. Skipping /spec:execute.

## Step 5 — Push

  $ specify workspace push
  specify: workspace push — dark-mode

    omnia-backend   up-to-date
    vectis-mobile   up-to-date

  0 pushed, 0 created, 2 up-to-date. 0 failed.

## Step 6 — Land (--auto-merge not set)

Querying open PRs on `specify/dark-mode`:

  $ gh pr list --head specify/dark-mode --state all --json number,state,merged,headRefName
  omnia-backend   PR #57    state=MERGED, merged=true
  vectis-mobile   PR #29    state=MERGED, merged=true

Every PR is `MERGED` on remote. Continuing to step 7.

---

## Step 7 — Finalize

  $ specify initiative finalize
  specify: initiative finalize — dark-mode (specify/dark-mode)

    omnia-backend         merged                   PR #57   https://github.com/org/omnia-backend/pull/57
    vectis-mobile         merged                   PR #29   https://github.com/org/vectis-mobile/pull/29

  2 merged, 0 unmerged, 0 closed, 0 no-branch, 0 branch-pattern-mismatch, 0 dirty, 0 failed.

  Initiative `dark-mode` finalized.
    archived plan: /…/shop-platform/.specify/archive/plans/dark-mode-20260428.yaml
    archived dir:  /…/shop-platform/.specify/archive/plans/dark-mode-20260428

---

## /spec:initiative — dark-mode — landed

  Brief:    .specify/archive/plans/dark-mode-20260428/initiative.md
  Plan:     .specify/archive/plans/dark-mode-20260428.yaml
  Registry: omnia-backend, vectis-mobile (unchanged)
  PRs:      omnia-backend#57 (merged), vectis-mobile#29 (merged)

  Re-running /spec:initiative create dark-mode will report
  plan-not-found from `specify initiative finalize` and exit 0.
```

## Invariants pinned by this transcript

- **Verb hygiene.** Every shell-out is a post-1F+1G v1 verb: `specify initiative {create, finalize}`, `specify plan {create, add, amend, validate}`, `specify registry validate`, `specify workspace {sync, push}`, `gh pr list`. No retired verbs.
- **No registry mutation under `new-feature`.** `registry.yaml` is byte-identical between run-1 input and run-2 output. The 2B registry-proposal sub-step does not fire because every assignment routes to an existing project.
- **Step 6 stops without `--auto-merge`.** The umbrella never invokes `gh pr merge` or `specify workspace merge` in run 1. It surfaces the open-PR list and exits zero. The operator's manual merges happen off-fixture.
- **Re-entry is idempotent.** Run 2 traverses every step but does **work** only at step 7. Each earlier step short-circuits because its on-disk state is already terminal: brief present, registry unchanged, plan terminal, workspace clones up-to-date, every PR `MERGED` on remote.
- **Cross-project contract change has no `project`.** `dark-mode-contract` runs against the hub itself; the per-project routing applies only to `dark-mode-backend` and `dark-mode-mobile`.
