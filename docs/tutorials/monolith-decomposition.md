# Monolith Decomposition

Decompose a single legacy monolith into slice-sized migration candidates using `/change:survey`. This tutorial walks you through a `legacy-code` source that is too large to migrate in one pass, showing how the survey stage scans surfaces, sizes code footprints, clusters overlapping handlers, and produces a candidate inventory for `propose`.

**Prerequisites:**

- [`specify` CLI](../orientation/prerequisites.md) installed and on `PATH`.
- A project initialised with `specify init` (single-repo) or `specify init --hub` (platform hub).
- Familiarity with [multi-slice changes](single-repo-change.md) and [cross-repo planning](cross-repo-change.md).
- A local path to the legacy monolith you want to decompose (or a materialized clone).

## Contents

- [Scenario](#scenario)
- [1. Record the source](#1-record-the-source)
- [2. Survey scans surfaces](#2-survey-scans-surfaces)
- [3. Candidate inventory in survey.md](#3-candidate-inventory-in-surveymd)
- [4. Candidates flow into discovery.md](#4-candidates-flow-into-discoverymd)
- [5. Propose pause point](#5-propose-pause-point)
- [When a candidate is too large](#when-a-candidate-is-too-large)
- [What you learned](#what-you-learned)
- [Next steps](#next-steps)

## Scenario

You have a TypeScript monolith at `./legacy/monolith` — approximately 2400 production LOC across 12 modules. It exposes three HTTP routes: `GET /users/:id`, `POST /users`, and `POST /orders`. You want to migrate these capabilities into a Specify-managed project, but the monolith is too large to be a single slice (anything ≥ 1000 LOC triggers decomposition). You need Specify to break it into reviewable, slice-sized candidates before planning begins.

The `/change:draft` pipeline handles this automatically. When it sees a `legacy-code` source, it inserts a `/change:survey` step between workspace sync and propose. Survey mechanically scans the source for surfaces, sizes each candidate, clusters overlapping handlers, and writes a candidate inventory that `propose` consumes.

## 1. Record the source

Start by drafting a change that points at the monolith:

```text
/change:draft migrate-monolith source legacy-monolith=./legacy/monolith
```

The `source` keyword tells `/change:draft` that this is a `legacy-code` input. The `legacy-monolith` key names the source for traceability — it appears in every surface id and candidate block throughout the pipeline.

Behind the scenes, `/change:draft` writes a sources file for the CLI:

```yaml
version: 1
sources:
  - key: legacy-monolith
    path: ./legacy/monolith
```

This is the batch input that `specify change survey` consumes. For a single source the batch file has one row; the same format scales to many sources (see [Legacy Fleet Decomposition](legacy-fleet-decomposition.md)).

## 2. Survey scans surfaces

`/change:survey` invokes the CLI scanner once:

```bash
specify change survey --sources .specify/plans/migrate-monolith/survey/sources.yaml \
    --out .specify/plans/migrate-monolith/survey/
```

The scanner runs every registered detector against `./legacy/monolith`. For this TypeScript monolith, the Express detector fires and enumerates three HTTP routes. It writes two sidecar files per source-key:

**`metadata.json`** — coarse source facts:

```json
{
  "version": 1,
  "source-key": "legacy-monolith",
  "language": "typescript",
  "loc": 2400,
  "module_count": 12,
  "top_level_modules": [
    "src/orders",
    "src/server.ts",
    "src/users"
  ]
}
```

**`surfaces.json`** — externally observable surfaces and their code footprints:

```json
{
  "version": 1,
  "source-key": "legacy-monolith",
  "language": "typescript",
  "surfaces": [
    {
      "id": "http-get-users-id",
      "kind": "http-route",
      "identifier": "GET /users/:id",
      "handler": "src/users/get.ts:getUser",
      "touches": [
        "src/users/get.ts",
        "src/users/repository.ts"
      ],
      "declared-at": ["src/server.ts:18"]
    },
    {
      "id": "http-post-orders",
      "kind": "http-route",
      "identifier": "POST /orders",
      "handler": "src/orders/create.ts:createOrder",
      "touches": [
        "src/orders/create.ts",
        "src/orders/pricing.ts",
        "src/orders/repository.ts",
        "src/orders/validate.ts"
      ],
      "declared-at": ["src/server.ts:22"]
    },
    {
      "id": "http-post-users",
      "kind": "http-route",
      "identifier": "POST /users",
      "handler": "src/users/register.ts:registerUser",
      "touches": [
        "src/users/register.ts",
        "src/users/repository.ts",
        "src/users/validate.ts"
      ],
      "declared-at": ["src/server.ts:14"]
    }
  ]
}
```

> These outputs match [`plugins/change/skills/survey/fixtures/single-source-monolith/inputs/`](../../plugins/change/skills/survey/fixtures/single-source-monolith/inputs/). If the fixture changes, this tutorial must also change.

### What just happened

The scanner found three HTTP routes by reading Express route registrations in `src/server.ts`. For each surface it recorded:

- The **identifier** — the legacy spelling of the route (`GET /users/:id`, `POST /users`, `POST /orders`).
- The **handler** — the function that implements the route.
- The **touches** — every source file reachable from the handler (via import-graph walking, excluding `node_modules`).
- The **declared-at** — the line in `src/server.ts` where the route is mounted (the proof the surface exists).

The scanner is mechanical. It does not call an LLM, infer business meaning, or write plan entries.

## 3. Candidate inventory in survey.md

After the CLI finishes, `/change:survey` reads the sidecars and runs the candidate algorithm:

1. **Size check.** The union of all `touches` across every surface is 1320 LOC — above the 1000 LOC threshold. The monolith cannot be a single candidate, so the algorithm descends to surface-level candidates.

2. **Surface candidates.** Each surface becomes a default candidate. But two of them — `GET /users/:id` and `POST /users` — share `src/users/repository.ts` in their `touches`.

3. **Minimal clustering.** The overlap between those two surfaces is 50% (1 shared file out of the smaller set's 2 files). Their combined `touches` total 700 LOC — still below the 1000 LOC threshold. Survey merges them into one `user-management` candidate. The third surface (`POST /orders`) has no overlap with either and remains standalone as `order-creation`.

The result is `.specify/plans/migrate-monolith/survey.md`:

<details>
<summary>Expected <code>survey.md</code></summary>

````markdown
# migrate-monolith survey

## Summary

Sources: 1 | Surfaces: 3 | Candidates: 2 | Unresolved: 0

## Source inventory

| Source | Path | Language | LOC | Surfaces |
|---|---|---|---|---|
| legacy-monolith | ./legacy/monolith | typescript | 2400 | 3 |

## Candidate inventory

### user-management [acceptable, 700 LOC]

```yaml
kind: candidate
sources: [legacy-monolith]
touches:
  - src/users/get.ts
  - src/users/register.ts
  - src/users/repository.ts
  - src/users/validate.ts
surfaces:
  - legacy-monolith:http-get-users-id
  - legacy-monolith:http-post-users
declared-at:
  - legacy-monolith:src/server.ts:14
  - legacy-monolith:src/server.ts:18
```

### order-creation [acceptable, 620 LOC]

```yaml
kind: candidate
sources: [legacy-monolith]
handler: src/orders/create.ts:createOrder
touches:
  - src/orders/create.ts
  - src/orders/pricing.ts
  - src/orders/repository.ts
  - src/orders/validate.ts
surfaces:
  - legacy-monolith:http-post-orders
declared-at:
  - legacy-monolith:src/server.ts:22
```
````

</details>

> This output matches [`plugins/change/skills/survey/fixtures/single-source-monolith/expected/survey.md`](../../plugins/change/skills/survey/fixtures/single-source-monolith/expected/survey.md).

### What just happened

Survey decomposed one 2400 LOC monolith into two `acceptable` candidates. The `user-management` candidate clusters two surfaces that share handler infrastructure. The `order-creation` candidate stands alone. Both are small enough for a single slice.

Every surface id is namespaced as `<source-key>:<surface-id>` (e.g. `legacy-monolith:http-post-users`) so identifiers remain distinguishable when multiple sources appear in the same inventory.

## 4. Candidates flow into discovery.md

Survey also appends candidate blocks to the `## Candidate inventory` heading in `discovery.md`. The discovery brief wrote this heading before survey ran; both `/change:analyze` (for documentation inputs) and `/change:survey` (for legacy-code inputs) append under it.

<details>
<summary>Expected <code>discovery.md</code> after survey</summary>

````markdown
# Discovery — migrate-monolith

## Candidate inventory

<!-- source-key: legacy-monolith -->
### user-management [acceptable, 700 LOC]

```yaml
kind: candidate
sources: [legacy-monolith]
touches:
  - src/users/get.ts
  - src/users/register.ts
  - src/users/repository.ts
  - src/users/validate.ts
surfaces:
  - legacy-monolith:http-get-users-id
  - legacy-monolith:http-post-users
declared-at:
  - legacy-monolith:src/server.ts:14
  - legacy-monolith:src/server.ts:18
```

<!-- source-key: legacy-monolith -->
### order-creation [acceptable, 620 LOC]

```yaml
kind: candidate
sources: [legacy-monolith]
handler: src/orders/create.ts:createOrder
touches:
  - src/orders/create.ts
  - src/orders/pricing.ts
  - src/orders/repository.ts
  - src/orders/validate.ts
surfaces:
  - legacy-monolith:http-post-orders
declared-at:
  - legacy-monolith:src/server.ts:22
```
````

</details>

> This output matches [`plugins/change/skills/survey/fixtures/single-source-monolith/expected/discovery.md`](../../plugins/change/skills/survey/fixtures/single-source-monolith/expected/discovery.md).

`propose` reads these candidate blocks and presents each one for your review.

## 5. Propose pause point

After survey, the pipeline continues to `propose`. For each candidate, you can:

- **Accept** — add it to the plan as a slice.
- **Edit** — rename it, adjust the scope, or change dependencies.
- **Reject** — exclude it from the plan.

```text
Proposed: user-management
  Sources: [legacy-monolith]
  Surfaces: http-get-users-id, http-post-users
  Touches: 4 files, 700 LOC
  Accept / Edit / Reject?
```

Every accepted candidate becomes a plan entry via `specify plan add`. Survey produces candidates; propose produces `plan.yaml`.

After you accept both candidates:

```bash
specify plan status
```

```text
migrate-monolith
  pending  user-management    (depends-on: [])
  pending  order-creation     (depends-on: [])

Summary: 2 pending, 0 in-progress, 0 done
```

`/change:draft` stops here — the operator review seam between draft and execute is by design. See [Reviewing the plan](reviewing-a-plan.md) for the full review checklist.

## When a candidate is too large

Not every source decomposes cleanly. When minimal clustering produces a candidate that is still ≥ 1000 LOC, survey marks it `unresolved: true` rather than silently merging unrelated surfaces or inventing a split:

```yaml
kind: candidate
sources: [legacy-billing]
handler: src/billing/invoices.ts:syncInvoices
touches:
  - src/billing/core.ts
  - src/billing/invoices.ts
surfaces:
  - legacy-billing:scheduled-job-invoice-sync
declared-at:
  - legacy-billing:src/billing/scheduler.ts:24
unresolved: true
```

An `unresolved` candidate appears in the survey summary (`Unresolved: 1`) and in the candidate inventory. `propose` refuses to draft a plan entry from an unresolved candidate until you resolve it — either by editing the candidate to narrow its scope, splitting it manually, or rescoping the change.

> This pattern is exercised by the [`too-large-unresolved` fixture](../../plugins/change/skills/survey/fixtures/too-large-unresolved/).

## What you learned

- `/change:draft` automatically inserts a `/change:survey` step for `legacy-code` sources. You do not invoke survey separately.
- The CLI scanner (`specify change survey`) is mechanical — it detects framework surfaces, records code footprints, and writes byte-stable JSON. No LLM involved.
- Survey sizes candidates by production LOC. Sources under 1000 LOC become one candidate; larger sources are decomposed into surface-level candidates with minimal same-source clustering.
- Clustering merges surfaces that share ≥ 50% `touches` overlap, as long as the combined candidate remains under 1000 LOC.
- Candidates that cannot be reduced below 1000 LOC are marked `unresolved: true` for operator review during `propose`.
- Surface ids are namespaced `<source-key>:<surface-id>` for traceability across multi-source inventories.
- Survey writes `survey.md` and appends candidate blocks to `discovery.md`. Propose reads the blocks and drives the accept/edit/reject loop.

## Next steps

- [Legacy Fleet Decomposition](legacy-fleet-decomposition.md) — the same mechanics with multiple legacy sources producing one combined inventory.
- [Reviewing the plan](reviewing-a-plan.md) — the operator review seam between `/change:draft` and `/change:execute`.
- [Legacy Migration at Scale](legacy-migration-at-scale.md) — the end-to-end migration workflow including execution and landing.
- [Legacy migration at scale (explanation)](../explanation/legacy-migration-at-scale.md) — where cross-change scale concerns (source catalogues, migration ledgers, reconciliation) will live.
