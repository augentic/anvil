# Transcript — `/spec:plan --orchestrate migrate-foo --shape migrate-legacy --source monolith=… --auto-merge`

This transcript pins the dialogue and the shell-outs the umbrella runs against an empty hub when migrating the legacy `mono-repo-foo` monolith. Every shell-out uses post-1F+1G v1 verbs verbatim — none of the retired hyphenated, nested, or `init`-named forms appears (see the v1 [migration map](../../../../../../docs/explanation/migrating-cli-v1.md)).

The transcript starts at the operator's invocation; pre-flight, all seven steps, and the terminal summary follow.

```text
$ /spec:plan --orchestrate migrate-foo \
    --shape migrate-legacy \
    --source monolith=git@github.com:org/legacy-foo.git \
    --auto-merge

Pre-flight
  shape:        migrate-legacy
  hub:          shop-platform/.specify/project.yaml (schema: hub, hub: true)
  specify:      0.x.y on PATH
  gh:           v2.x.y on PATH
  initiative:   migrate-foo (kebab-case ok)

---

## Step 1 — Brief

initiative.md is absent.

  $ specify initiative create migrate-foo
  ok: wrote initiative.md

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
ok: wrote initiative.md (operator can $EDITOR to refine before step 3)

---

## Step 2 — Registry

  $ specify registry validate
  ok: registry is empty (0 projects)

Shape is `migrate-legacy` and the registry is empty.
Handing off to /spec:plan's greenfield path (RFC-9 §2B).

---

## Step 3 — Plan

  $ /spec:plan migrate-foo --source monolith=git@github.com:org/legacy-foo.git

  ── /spec:plan migrate-foo ──

  $ specify plan create migrate-foo --source monolith=git@github.com:org/legacy-foo.git
  ok: scaffolded plan.yaml

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
      $ specify registry add foo-backend --url git@github.com:org/foo-backend.git --schema omnia@v1 --description "Backend service migrated from the legacy mono-repo-foo TypeScript monolith. Owns user accounts, order processing, and the HTTP API that the mobile app calls into."
      ok: appended `foo-backend` to registry.yaml (1 project)

      $ specify registry add foo-mobile --url git@github.com:org/foo-mobile.git --schema vectis@v1 --description "iOS and Android mobile clients migrated from the legacy mono-repo-foo monolith's mobile shells. Owns the storefront, checkout, and account-management flows."
      ok: appended `foo-mobile` to registry.yaml (2 projects)

      $ specify workspace sync
      ok: materialised .specify/workspace/foo-backend/
      ok: materialised .specify/workspace/foo-mobile/

  Step 3(b) — Sync peers
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
    - /spec:execute --loop

---

## Step 4 — Execute

  $ /spec:execute --loop

  ## /spec:execute — migrate-foo

  ### Initiative: migrate-foo
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

  ## /spec:execute — migrate-foo — terminated

  ### Final state
  Progress: done 3, in-progress 0, pending 0, blocked 0, failed 0, skipped 0 (total 3)

  Completion: all-done

  Next action: Initiative complete. Land remote PRs (specify workspace
    merge or merge them by hand on the forge), then close out via
    specify initiative finalize.

---

## Step 5 — Push

  $ specify workspace push
  specify: workspace push — migrate-foo

    foo-backend   pushed       specify/migrate-foo   PR #41
    foo-mobile    pushed       specify/migrate-foo   PR #18

  2 pushed, 0 created, 0 up-to-date. 0 failed.

---

## Step 6 — Land (--auto-merge)

  $ specify workspace merge
  specify: workspace merge — migrate-foo (specify/migrate-foo)

    foo-backend   merged                    PR #41   https://github.com/org/foo-backend/pull/41
    foo-mobile    merged                    PR #18   https://github.com/org/foo-mobile/pull/18

  2 merged, 0 would-merge, 0 pending-checks, 0 failed-checks, 0 closed, 0 no-branch, 0 branch-pattern-mismatch, 0 failed.

---

## Step 7 — Finalize

  $ specify initiative finalize
  specify: initiative finalize — migrate-foo (specify/migrate-foo)

    foo-backend         merged                   PR #41   https://github.com/org/foo-backend/pull/41
    foo-mobile          merged                   PR #18   https://github.com/org/foo-mobile/pull/18

  2 merged, 0 unmerged, 0 closed, 0 no-branch, 0 branch-pattern-mismatch, 0 dirty, 0 failed.

  Initiative `migrate-foo` finalized.
    archived plan: /…/shop-platform/.specify/archive/plans/migrate-foo-20260428.yaml
    archived dir:  /…/shop-platform/.specify/archive/plans/migrate-foo-20260428

---

## /spec:plan --orchestrate — migrate-foo — landed

  Brief:    .specify/archive/plans/migrate-foo-20260428/initiative.md
  Plan:     .specify/archive/plans/migrate-foo-20260428.yaml
  Registry: foo-backend, foo-mobile (added in step 3)
  PRs:      foo-backend#41 (merged), foo-mobile#18 (merged)

  Re-running /spec:plan --orchestrate migrate-foo will report
  plan-not-found from `specify initiative finalize` and exit 0.
```

## Invariants pinned by this transcript

- **Verb hygiene.** Every shell-out is a post-1F+1G v1 verb: `specify initiative create`, `specify plan {create, add, amend, validate}`, `specify registry {add, validate}`, `specify workspace {sync, push, merge}`, `specify change journal append` (inside the executor's self-heal — not visible here because the run is clean), `specify initiative finalize`. No retired verbs appear anywhere in the trace.
- **Greenfield ordering.** `specify registry add` (×2) precedes `specify workspace sync`, which precedes any `specify plan add` for entries routed to the new projects. This is the 2B invariant.
- **Cross-project contract change has no `project`.** `migrate-foo-contract` runs against the hub itself (no `Routing:` diagnostic from `/spec:execute`), exactly as the cross-repo tutorial pins.
- **`--auto-merge` does not bypass safety guards.** `specify workspace merge` inherits the branch-pattern guard, the no-`--admin` rule, and the no-CI-override rule. The transcript shows both PRs landing on `merged` (CI green); a `pending-checks` or `failed-checks` finding would have halted the umbrella before step 7.
- **Idempotent re-entry.** The umbrella's terminal summary points the operator at re-running `/spec:plan --orchestrate migrate-foo`, which exits zero with `plan-not-found` after the archive completes.
