# Legacy Fleet Decomposition

Decompose multiple legacy repositories into one combined candidate inventory using `/change:survey`. This tutorial walks you through a two-source change where each source is surveyed independently, candidates remain source-local, and the operator reviews the combined inventory during `propose`.

For single-source mechanics (sizing, clustering, the candidate algorithm), see [Monolith Decomposition](monolith-decomposition.md). This tutorial builds on that foundation and focuses on what changes when the input is a fleet of repositories rather than one monolith.

**Prerequisites:**

- [`specify` CLI](../orientation/prerequisites.md) installed and on `PATH`.
- A project initialised with `specify init` (single-repo) or `specify init --hub` (platform hub).
- Familiarity with [multi-slice changes](single-repo-change.md), [cross-repo planning](cross-repo-change.md), and [Monolith Decomposition](monolith-decomposition.md).
- Local paths to two or more legacy sources (or materialized clones).

## Contents

- [Scenario](#scenario)
- [1. Record the sources](#1-record-the-sources)
- [2. Survey runs each source independently](#2-survey-runs-each-source-independently)
- [3. One combined inventory](#3-one-combined-inventory)
- [4. Discovery carries source-tagged candidates](#4-discovery-carries-source-tagged-candidates)
- [5. Operator review during propose](#5-operator-review-during-propose)
- [What v1 does not do](#what-v1-does-not-do)
- [What you learned](#what-you-learned)
- [Next steps](#next-steps)

## Scenario

You are migrating two legacy TypeScript services:

| Source | Path | LOC | Surfaces |
|---|---|---|---|
| `legacy-api` | `./legacy/api` | 1200 | 2 HTTP routes (`GET /users`, `POST /orders`) |
| `legacy-billing` | `./legacy/billing` | 780 | 2 HTTP routes (`GET /invoices`, `POST /payments`) |

The API service is large enough to decompose (≥ 1000 LOC). The billing service is small enough to be a single candidate (< 1000 LOC). Both need to reach the plan as reviewable candidates.

## 1. Record the sources

Draft a change that points at both sources:

```text
/change:draft migrate-fleet source legacy-api=./legacy/api source legacy-billing=./legacy/billing
```

Each `source` keyword records one `legacy-code` input with a named key. `/change:draft` writes a batch sources file with one row per source:

```yaml
version: 1
sources:
  - key: legacy-api
    path: ./legacy/api
  - key: legacy-billing
    path: ./legacy/billing
```

> This matches [`plugins/change/skills/survey/fixtures/multi-source-fleet/inputs/sources.yaml`](../../plugins/change/skills/survey/fixtures/multi-source-fleet/inputs/sources.yaml).

The source count changes the breadth of the inventory, not the planning model. The candidate algorithm is identical for each source.

## 2. Survey runs each source independently

`/change:survey` invokes the CLI scanner once with the batch file:

```bash
specify change survey --sources .specify/plans/migrate-fleet/survey/sources.yaml \
    --out .specify/plans/migrate-fleet/survey/
```

The scanner processes each row independently. It writes per-source-key sidecars under the output directory:

```text
.specify/plans/migrate-fleet/survey/
├── legacy-api/
│   ├── metadata.json
│   └── surfaces.json
└── legacy-billing/
    ├── metadata.json
    └── surfaces.json
```

Each source gets its own detectors run. Row failure leaves that row's files untouched and does not affect other rows — the writes are independent and atomic per row.

<details>
<summary><code>legacy-api/surfaces.json</code></summary>

```json
{
  "version": 1,
  "source-key": "legacy-api",
  "language": "typescript",
  "surfaces": [
    {
      "id": "http-get-users",
      "kind": "http-route",
      "identifier": "GET /users",
      "handler": "src/users/list.ts:listUsers",
      "touches": [
        "src/users/list.ts",
        "src/users/repository.ts"
      ],
      "declared-at": ["src/server.ts:9"]
    },
    {
      "id": "http-post-orders",
      "kind": "http-route",
      "identifier": "POST /orders",
      "handler": "src/orders/create.ts:createOrder",
      "touches": [
        "src/orders/create.ts",
        "src/orders/repository.ts",
        "src/orders/validate.ts"
      ],
      "declared-at": ["src/server.ts:14"]
    }
  ]
}
```

</details>

<details>
<summary><code>legacy-billing/surfaces.json</code></summary>

```json
{
  "version": 1,
  "source-key": "legacy-billing",
  "language": "typescript",
  "surfaces": [
    {
      "id": "http-get-invoices",
      "kind": "http-route",
      "identifier": "GET /invoices",
      "handler": "src/invoices/list.ts:listInvoices",
      "touches": [
        "src/invoices/list.ts",
        "src/invoices/repository.ts"
      ],
      "declared-at": ["src/server.ts:8"]
    },
    {
      "id": "http-post-payments",
      "kind": "http-route",
      "identifier": "POST /payments",
      "handler": "src/payments/create.ts:createPayment",
      "touches": [
        "src/payments/create.ts",
        "src/payments/repository.ts"
      ],
      "declared-at": ["src/server.ts:12"]
    }
  ]
}
```

</details>

> These outputs match [`plugins/change/skills/survey/fixtures/multi-source-fleet/inputs/`](../../plugins/change/skills/survey/fixtures/multi-source-fleet/inputs/). If the fixture changes, this tutorial must also change.

### What just happened

The scanner processed two sources independently. `legacy-api` produced two surfaces; `legacy-billing` produced two surfaces. Each source's `surfaces.json` is self-contained — there is no cross-source coordination at scan time.

## 3. One combined inventory

After the CLI finishes, `/change:survey` reads all sidecars and runs the candidate algorithm on each source:

- **`legacy-api`** (1200 LOC): union-of-`touches` ≥ 1000, so the algorithm descends to surface candidates. The two surfaces have no `touches` overlap, so no clustering — two standalone candidates.
- **`legacy-billing`** (780 LOC): union-of-`touches` < 1000, so the entire source is emitted as one terminal candidate covering both surfaces.

The result is one combined `survey.md`:

<details>
<summary>Expected <code>survey.md</code></summary>

````markdown
# migrate-fleet survey

## Summary

Sources: 2 | Surfaces: 4 | Candidates: 3 | Unresolved: 0

## Source inventory

| Source | Path | Language | LOC | Surfaces |
|---|---|---|---|---|
| legacy-api | ./legacy/api | typescript | 1200 | 2 |
| legacy-billing | ./legacy/billing | typescript | 780 | 2 |

## Candidate inventory

### user-list [acceptable, 540 LOC]

```yaml
kind: candidate
sources: [legacy-api]
handler: src/users/list.ts:listUsers
touches:
  - src/users/list.ts
  - src/users/repository.ts
surfaces:
  - legacy-api:http-get-users
declared-at:
  - legacy-api:src/server.ts:9
```

### order-creation [acceptable, 660 LOC]

```yaml
kind: candidate
sources: [legacy-api]
handler: src/orders/create.ts:createOrder
touches:
  - src/orders/create.ts
  - src/orders/repository.ts
  - src/orders/validate.ts
surfaces:
  - legacy-api:http-post-orders
declared-at:
  - legacy-api:src/server.ts:14
```

### legacy-billing [acceptable, 780 LOC]

```yaml
kind: candidate
sources: [legacy-billing]
touches:
  - src/invoices/list.ts
  - src/invoices/repository.ts
  - src/payments/create.ts
  - src/payments/repository.ts
surfaces:
  - legacy-billing:http-get-invoices
  - legacy-billing:http-post-payments
declared-at:
  - legacy-billing:src/server.ts:12
  - legacy-billing:src/server.ts:8
```
````

</details>

> This output matches [`plugins/change/skills/survey/fixtures/multi-source-fleet/expected/survey.md`](../../plugins/change/skills/survey/fixtures/multi-source-fleet/expected/survey.md).

The inventory is combined but the candidates are source-local. Every surface id is namespaced `<source-key>:<surface-id>` so `legacy-api:http-post-orders` and a hypothetical `legacy-billing:http-post-orders` remain distinguishable.

Ordering is alphabetical by source-key, then by first surface id within each source.

## 4. Discovery carries source-tagged candidates

Survey appends the same candidate blocks to `discovery.md`, each tagged with a `<!-- source-key: ... -->` comment:

<details>
<summary>Expected <code>discovery.md</code> after survey</summary>

````markdown
# Discovery — migrate-fleet

## Candidate inventory

<!-- source-key: legacy-api -->
### user-list [acceptable, 540 LOC]

```yaml
kind: candidate
sources: [legacy-api]
handler: src/users/list.ts:listUsers
touches:
  - src/users/list.ts
  - src/users/repository.ts
surfaces:
  - legacy-api:http-get-users
declared-at:
  - legacy-api:src/server.ts:9
```

<!-- source-key: legacy-api -->
### order-creation [acceptable, 660 LOC]

```yaml
kind: candidate
sources: [legacy-api]
handler: src/orders/create.ts:createOrder
touches:
  - src/orders/create.ts
  - src/orders/repository.ts
  - src/orders/validate.ts
surfaces:
  - legacy-api:http-post-orders
declared-at:
  - legacy-api:src/server.ts:14
```

<!-- source-key: legacy-billing -->
### legacy-billing [acceptable, 780 LOC]

```yaml
kind: candidate
sources: [legacy-billing]
touches:
  - src/invoices/list.ts
  - src/invoices/repository.ts
  - src/payments/create.ts
  - src/payments/repository.ts
surfaces:
  - legacy-billing:http-get-invoices
  - legacy-billing:http-post-payments
declared-at:
  - legacy-billing:src/server.ts:12
  - legacy-billing:src/server.ts:8
```
````

</details>

> This output matches [`plugins/change/skills/survey/fixtures/multi-source-fleet/expected/discovery.md`](../../plugins/change/skills/survey/fixtures/multi-source-fleet/expected/discovery.md).

## 5. Operator review during propose

`propose` presents all three candidates for your review — the combined inventory from both sources:

```text
Proposed: user-list
  Sources: [legacy-api]
  Surfaces: http-get-users
  Touches: 2 files, 540 LOC
  Accept / Edit / Reject?

Proposed: order-creation
  Sources: [legacy-api]
  Surfaces: http-post-orders
  Touches: 3 files, 660 LOC
  Accept / Edit / Reject?

Proposed: legacy-billing
  Sources: [legacy-billing]
  Surfaces: http-get-invoices, http-post-payments
  Touches: 4 files, 780 LOC
  Accept / Edit / Reject?
```

This is the operator review point. You see candidates from both sources in one inventory and can:

- **Accept** each candidate as-is — one plan entry per candidate.
- **Combine** related candidates by editing them during propose. For example, if `order-creation` from `legacy-api` and `legacy-billing` are tightly coupled, you can merge them into one plan entry.
- **Reorder** candidates by editing `depends-on` relationships.
- **Reject** candidates you want to defer.

After accepting all three:

```bash
specify plan status
```

```text
migrate-fleet
  pending  user-list          (depends-on: [])
  pending  order-creation     (depends-on: [])
  pending  legacy-billing     (depends-on: [])

Summary: 3 pending, 0 in-progress, 0 done
```

## What v1 does not do

Survey in v1 is deliberately conservative about multi-source changes. Understanding these boundaries helps you know what to expect during `propose`:

- **No cross-source pairing.** Survey does not automatically merge candidates from different sources, even when surfaces look related (e.g. an outbound HTTP call in `legacy-api` that matches a route in `legacy-billing`). Evidence of cross-source communication is preserved in the surfaces, but pairing is an operator decision during `propose`.
- **No `depends-on` inference.** Survey does not infer dependency ordering from contract edges. Candidates are emitted in source order, then surface order. Dependency ordering is a `propose` and operator concern.
- **No automated routing.** Survey-derived candidates carry no `target-project`. Assignment uses today's signals: description match, baseline spec affinity, and capability compatibility.
- **No cross-source identifier normalization.** Surface identifiers preserve the legacy spelling and are not canonicalized for matching. The same route from two repos (`/users`) remains two distinct surface ids (`legacy-api:http-get-users` and `legacy-billing:http-get-users` if both existed).

These are deliberate deferrals, not missing features. The evidence for cross-source relationships is captured in the surfaces — what v1 omits is the automated inference and merging that would act on that evidence. See [Legacy migration at scale (explanation)](../explanation/legacy-migration-at-scale.md) for the full list of deferrals and where the solutions will live.

## What you learned

- Multi-source changes produce one combined candidate inventory. The algorithm is identical per source; the source count changes the breadth, not the model.
- Each source gets independent detector runs and independent sidecar files. Row failure does not affect other rows.
- Small sources (< 1000 LOC) emit as one source-level candidate. Large sources decompose into surface-level candidates with minimal clustering.
- Surface ids are namespaced `<source-key>:<surface-id>` so identifiers from different sources remain distinguishable.
- v1 does not pair candidates across sources, infer `depends-on` from contract edges, or route survey-derived candidates to target projects. These are operator decisions during `propose`.
- The operator review point during `propose` is where you combine, reorder, or split candidates as needed.

## Next steps

- [Monolith Decomposition](monolith-decomposition.md) — single-source mechanics (sizing, clustering, the unresolved path).
- [Reviewing the plan](reviewing-a-plan.md) — the operator review seam between `/change:draft` and `/change:execute`.
- [Legacy Migration at Scale](legacy-migration-at-scale.md) — the end-to-end migration workflow including execution and landing.
- [Legacy migration at scale (explanation)](../explanation/legacy-migration-at-scale.md) — where cross-change scale concerns (source catalogues, migration ledgers, reconciliation) will live.
