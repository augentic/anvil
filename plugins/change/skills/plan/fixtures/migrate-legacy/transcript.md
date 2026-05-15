# Transcript — `/change:plan <name> orchestrate migrate-foo shape migrate-legacy source monolith=…`

This transcript pins the dialogue and the shell-outs the umbrella runs against an empty hub when migrating the legacy `mono-repo-foo` monolith. Every shell-out uses post-1F+1G v1 verbs verbatim — none of the retired hyphenated, nested, or `init`-named forms appears.

The transcript starts at the operator's invocation. Run 1 drives through PR creation and stops for operator merge; run 2 resumes after the PRs are merged and finalizes.

```text
$ /change:plan <name> orchestrate migrate-foo \
    shape migrate-legacy \
    source monolith=git@github.com:org/legacy-foo.git

Pre-flight
  shape:        migrate-legacy
  hub:          shop-platform/.specify/project.yaml (hub: true; capability omitted)
  specify:      0.x.y on PATH
  gh:           v2.x.y on PATH (read-only PR observation)
  change:       migrate-foo (kebab-case ok)

---

## Step 1 — Brief

change.md is absent.

  $ specify change create migrate-foo --source monolith=git@github.com:org/legacy-foo.git
  ok: wrote change.md
  ok: scaffolded plan.yaml (sources: monolith)

Shape `migrate-legacy` carries one --source. Suggested default body:

  ---
  name: migrate-foo
  inputs:
    - path: git@github.com:org/legacy-foo.git
      kind: legacy-code
  ---

  Migrate the legacy `mono-repo-foo` TypeScript monolith onto the
  Augentic platform.

Accept the default body? [Y/n] y
ok: wrote change.md (operator can $EDITOR to refine before step 3)

---

## Step 2 — Registry

  $ specify registry validate
  ok: registry is empty (0 projects)

Shape is `migrate-legacy` and the registry is empty.
Handing off to /change:plan's greenfield path (RFC-9 §2B).

---

## Step 3 — Plan

  $ /change:plan migrate-foo

  ── /change:plan migrate-foo ──

  Step 2 — Scaffold (specify change create) skipped: change.md and
    plan.yaml are already present from step 1; running under
    --extend so the existing scaffold is reused.

  Step 3(a) — Discovery
    Cloned monolith → .specify/plans/migrate-foo/analyze/monolith/
    Inferring capability inventory…
    Wrote .specify/plans/migrate-foo/discovery.md

    The discovery brief proposes a two-project topology:

    ## Proposed registry topology

    | # | Name        | URL                                  | Schema    | Description                              |
    |---|-------------|--------------------------------------|-----------|------------------------------------------|
    | 1 | foo-backend | git@github.com:org/foo-backend.git   | omnia@v1  | Backend service migrated from the legacy |
    |   |             |                                      |           | mono-repo-foo TypeScript monolith. ...   |
    | 2 | foo-mobile  | git@github.com:org/foo-mobile.git    | vectis@v1 | iOS and Android mobile clients migrated  |
    |   |             |                                      |           | from the legacy mono-repo-foo monolith.  |

    Approve each row? [Y/n/edit] Y

    Greenfield bootstrap — running:
      $ specify registry add foo-backend --url git@github.com:org/foo-backend.git --capability omnia@v1 --description "Backend service migrated from the legacy mono-repo-foo TypeScript monolith. Owns user accounts, order processing, and the HTTP API that the mobile app calls into."
      ok: appended `foo-backend` to registry.yaml (1 project)

      $ specify registry add foo-mobile --url git@github.com:org/foo-mobile.git --capability vectis@v1 --description "iOS and Android mobile clients migrated from the legacy mono-repo-foo monolith's mobile shells. Owns the storefront, checkout, and account-management flows."
      ok: appended `foo-mobile` to registry.yaml (2 projects)

      $ specify workspace sync
      ok: materialised .specify/workspace/foo-backend/
      ok: materialised .specify/workspace/foo-mobile/

  Step 3(b) — Sync workspace
    Wrote .specify/plans/migrate-foo/workspace.md

  Step 3(c) — Propose
    Slice 1: migrate-foo-contract  (sources: monolith; depends-on: —)            [accept] [edit] [reject] [abort]
    > accept
    $ specify plan add migrate-foo-contract --sources monolith --description "Define the cross-project HTTP contract between foo-backend and foo-mobile. Inferred from the legacy monolith's internal call sites (src/api/)."
    ok: appended `migrate-foo-contract` to plan.yaml

    Slice 2: migrate-foo-backend   (sources: monolith; depends-on: migrate-foo-contract)
    > accept
    $ specify plan add migrate-foo-backend --sources monolith --depends-on migrate-foo-contract --description "Migrate the backend layer of mono-repo-foo onto Omnia: user accounts, orders, and the HTTP surface the contract pins."
    ok: appended `migrate-foo-backend` to plan.yaml

    Slice 3: migrate-foo-mobile    (sources: monolith; depends-on: migrate-foo-contract)
    > accept
    $ specify plan add migrate-foo-mobile --sources monolith --depends-on migrate-foo-contract --description "Migrate the mobile shells onto Vectis Crux: storefront, checkout, and account-management screens. Consumes the migrate-foo-contract HTTP API."
    ok: appended `migrate-foo-mobile` to plan.yaml

  Step 3(d) — Assignment

  | # | Entry                  | Project        | Rationale                                          |
  |---|------------------------|----------------|----------------------------------------------------|
  | 1 | migrate-foo-contract   | —              | Cross-project contract; runs against the hub.      |
  | 2 | migrate-foo-backend    | foo-backend    | Description overlap: backend, HTTP, accounts.     |
  | 3 | migrate-foo-mobile     | foo-mobile     | Description overlap: mobile, storefront, checkout. |

    $ specify plan amend migrate-foo-backend --project foo-backend
    ok: migrate-foo-backend.project = foo-backend
    $ specify plan amend migrate-foo-mobile --project foo-mobile
    ok: migrate-foo-mobile.project = foo-mobile

  Wrote .specify/plans/migrate-foo/proposal.md

  Step 4 — Validate
    $ specify plan validate
    PASS

  Done. Next steps:
    - specify plan status
    - /change:execute loop

---

## Step 4 — Execute

  $ /change:execute loop

  ## /change:execute — migrate-foo

  ### Change: migrate-foo
  Progress: done 0, in-progress 0, pending 3, blocked 0, failed 0, skipped 0 (total 3)

  ---

  Self-heal: no in-progress entries found.

  # specify plan next --format json → { "next": "migrate-foo-contract", "project": null, ... }
  # specify plan transition migrate-foo-contract in-progress

  ### Processing: migrate-foo-contract (sources: [monolith])

  Step 1/3: define ✓
  Step 2/3: build ✓
  Step 3/3: merge
    Baseline updated: contracts/http/migrate-foo.yaml ✓
    Status: done

  ---

  # specify plan next --format json → { "next": "migrate-foo-backend", "project": "foo-backend", ... }

  Routing: migrate-foo-backend → foo-backend (.specify/workspace/foo-backend/)

  ### Processing: migrate-foo-backend (sources: [monolith])

  Step 1/3: define ✓
  Step 2/3: build ✓
  Step 3/3: merge
    Auto-commit: git commit -m "specify: merge migrate-foo-backend"
    Baseline updated: .specify/specs/foo-backend/spec.md ✓
    Status: done

  ---

  # specify plan next --format json → { "next": "migrate-foo-mobile", "project": "foo-mobile", ... }

  Routing: migrate-foo-mobile → foo-mobile (.specify/workspace/foo-mobile/)

  ### Processing: migrate-foo-mobile (sources: [monolith])

  Step 1/3: define ✓
  Step 2/3: build ✓
  Step 3/3: merge ✓
    Status: done

  ---

  ## /change:execute — migrate-foo — terminated

  ### Final state
  Progress: done 3, in-progress 0, pending 0, blocked 0, failed 0, skipped 0 (total 3)

  Completion: all-done

  Next action: Change complete. Push remote PRs with specify workspace
    push, merge them through the forge UI or hand-run gh pr merge,
    then re-run the umbrella so specify change finalize can verify.

---

## Step 5 — Push

  $ specify workspace push
  specify: workspace push — migrate-foo

    foo-backend   pushed       specify/migrate-foo   PR #41
    foo-mobile    pushed       specify/migrate-foo   PR #18

  2 pushed, 0 created, 0 up-to-date. 0 failed.

---

## Step 6 — PR handoff

Open PRs on `specify/migrate-foo`:

  foo-backend   specify/migrate-foo    PR #41    https://github.com/org/foo-backend/pull/41
  foo-mobile    specify/migrate-foo    PR #18    https://github.com/org/foo-mobile/pull/18

Merge these PRs through the forge UI or an explicit hand-run `gh pr merge`,
then re-run /change:plan <name> orchestrate migrate-foo to finalize.

---

## /change:plan <name> orchestrate — migrate-foo — paused

  Brief:    change.md
  Plan:     plan.yaml (3 changes, all `done`)
  PRs:      foo-backend#41 (open), foo-mobile#18 (open)

  Next action: merge PRs, then re-run /change:plan <name> orchestrate
  migrate-foo shape migrate-legacy source monolith=...
```

## Operator merges PRs

The operator merges both PRs on github.com (squash, conventional commit titles) or with explicit `gh pr merge` commands they run themselves. The umbrella only observes this state on the next run.

## Run 2 — re-entry, runs step 7 only

```text
$ /change:plan <name> orchestrate migrate-foo \
    shape migrate-legacy \
    source monolith=git@github.com:org/legacy-foo.git

Pre-flight
  shape:        migrate-legacy
  hub:          shop-platform/.specify/project.yaml (hub: true; capability omitted)
  specify:      0.x.y on PATH
  gh:           v2.x.y on PATH (read-only PR observation)
  change:       migrate-foo (kebab-case ok)

---

## Step 1 — Brief

change.md is present. Skipping.

## Step 2 — Registry

  $ specify registry validate
  ok: 2 projects valid (no changes)

## Step 3 — Plan

plan.yaml is present and every entry is in a terminal state
(done × 3). Skipping /change:plan.

## Step 4 — Execute

Plan is fully terminal. Skipping /change:execute.

## Step 5 — Push

  $ specify workspace push
  specify: workspace push — migrate-foo

    foo-backend   up-to-date
    foo-mobile    up-to-date

  0 pushed, 0 created, 2 up-to-date. 0 failed.

## Step 6 — PR handoff

Querying PRs on `specify/migrate-foo`:

  $ gh pr list --head specify/migrate-foo --state all --json number,state,merged,headRefName
  foo-backend   PR #41    state=MERGED, merged=true
  foo-mobile    PR #18    state=MERGED, merged=true

Every PR is `MERGED` on remote. Continuing to step 7.

---

## Step 7 — Finalize

  $ specify change finalize
  specify: change finalize — migrate-foo (specify/migrate-foo)

    foo-backend         merged                   PR #41   https://github.com/org/foo-backend/pull/41
    foo-mobile          merged                   PR #18   https://github.com/org/foo-mobile/pull/18

  2 merged, 0 unmerged, 0 closed, 0 no-branch, 0 branch-pattern-mismatch, 0 dirty, 0 failed.

  Change `migrate-foo` finalized.
    archived plan: /…/shop-platform/.specify/archive/plans/migrate-foo-20260428.yaml
    archived dir:  /…/shop-platform/.specify/archive/plans/migrate-foo-20260428

---

## /change:plan <name> orchestrate — migrate-foo — landed

  Brief:    .specify/archive/plans/migrate-foo-20260428/change.md
  Plan:     .specify/archive/plans/migrate-foo-20260428.yaml
  Registry: foo-backend, foo-mobile (added in step 3)
  PRs:      foo-backend#41 (merged), foo-mobile#18 (merged)

  Re-running /change:plan <name> orchestrate migrate-foo will report
  plan-not-found from `specify change finalize` and exit 0.
```

## Invariants pinned by this transcript

- **Verb hygiene.** Every shell-out is a current verb: `specify change create`, `specify plan {create, add, amend, validate}`, `specify registry {add, validate}`, `specify workspace {sync, push}`, `gh pr list`, `specify slice journal append` (inside the executor's self-heal — not visible here because the run is clean), `specify change finalize`. No retired verbs appear anywhere in the trace.
- **Greenfield ordering.** `specify registry add` (×2) precedes `specify workspace sync`, which precedes any `specify plan add` for entries routed to the new projects. This is the 2B invariant.
- **Cross-project contract change has no `project`.** `migrate-foo-contract` runs against the hub itself (no `Routing:` diagnostic from `/change:execute`), exactly as the cross-repo tutorial pins.
- **PR merge is outside orchestration.** The transcript shows the umbrella stopping with open PRs in run 1, then observing both PRs as `MERGED` in run 2. The merge itself happens outside the umbrella.
- **Idempotent re-entry.** The umbrella's terminal summary points the operator at re-running `/change:plan <name> orchestrate migrate-foo`, which exits zero with `plan-not-found` after the archive completes.
