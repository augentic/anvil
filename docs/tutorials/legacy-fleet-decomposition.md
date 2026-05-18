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
- [Combining candidates across sources during propose](#combining-candidates-across-sources-during-propose)
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

`/change:survey` enumerates each source through the same two-stage pipeline as a monolith — the skill drives an LLM with the per-language enumeration brief, the CLI validates and canonicalises — but applies it once per row in the batch. Both sources here are TypeScript, so both resolve to [`plugins/change/skills/survey/briefs/enumerate/typescript.md`](../../plugins/change/skills/survey/briefs/enumerate/typescript.md). For the single-source mechanics in detail, see [Monolith Decomposition § Survey drives the per-language brief](monolith-decomposition.md#2-survey-drives-the-per-language-brief).

### Staged candidates per source

The skill produces one staged candidate per source-key under the plan's `survey/staged/` directory:

```text
.specify/plans/migrate-fleet/survey/staged/
├── legacy-api.json
└── legacy-billing.json
```

Both candidates target the same closed schema; only their `source-key`, `surfaces[]`, and underlying source root differ.

<details>
<summary>Staged <code>legacy-api.json</code> (first attempt)</summary>

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
      "touches": ["src/users/list.ts", "src/users/repository.ts"],
      "declared-at": ["src/server.ts:9"]
    },
    {
      "id": "http-post-orders",
      "kind": "http-route",
      "identifier": "POST /orders",
      "handler": "src/orders/create.ts:createOrder",
      "touches": [
        "../shared/dto/order.ts",
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
<summary>Staged <code>legacy-billing.json</code></summary>

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
      "touches": ["src/invoices/list.ts", "src/invoices/repository.ts"],
      "declared-at": ["src/server.ts:8"]
    },
    {
      "id": "http-post-payments",
      "kind": "http-route",
      "identifier": "POST /payments",
      "handler": "src/payments/create.ts:createPayment",
      "touches": ["src/payments/create.ts", "src/payments/repository.ts"],
      "declared-at": ["src/server.ts:12"]
    }
  ]
}
```

</details>

Note the `legacy-api` candidate's `POST /orders` surface includes `../shared/dto/order.ts` — a relative import the LLM followed outside the source root. That entry violates the path-under-source-root rule. The `legacy-billing` candidate is clean.

### Batch invocation

The skill writes the matching `sources.yaml` and invokes the CLI once for the whole batch:

```bash
specify change survey \
    --sources .specify/plans/migrate-fleet/survey/sources.yaml \
    --staged  .specify/plans/migrate-fleet/survey/staged/ \
    --out     .specify/plans/migrate-fleet/survey/
```

The CLI processes each row independently and atomically: a row's `surfaces.json` and `metadata.json` are written iff that row's candidate validates, and a row failure leaves that row's existing files untouched. On this first run:

- `legacy-billing` validates cleanly. The CLI canonicalises the candidate and writes `.specify/plans/migrate-fleet/survey/legacy-billing/surfaces.json` + `metadata.json`.
- `legacy-api` fails with `surfaces-touches-out-of-tree` on `surfaces[1].touches[0]` (the `../shared/dto/order.ts` entry). No sidecars are written for `legacy-api`.

The CLI exits non-zero overall, but the partial write is intentional: re-runs only re-do the failed work.

### Repair loop on one row, the other untouched

`/change:survey` enters the bounded repair loop only for `legacy-api` — the row that failed. The repair contract is the same one the monolith tutorial walked through: a JSON envelope carrying the CLI's `code` and `detail` is fed back to the LLM together with the brief and the failed candidate. See [`references/repair-loop.md`](../../plugins/change/skills/survey/references/repair-loop.md) for the full contract.

```json
{
  "failure": {
    "code": "surfaces-touches-out-of-tree",
    "detail": "surfaces[1].touches[0]: ../shared/dto/order.ts"
  },
  "instruction": "Fix only the rule cited above. Re-emit the full surfaces.json with the offending entry corrected; do not alter unrelated surfaces."
}
```

The LLM re-emits the `legacy-api` candidate with that single entry removed (the TypeScript brief tells it to treat paths outside the source root as a module boundary). The skill re-validates with `--validate-only` against just the corrected staged file, then drops `--validate-only` and re-runs the batch to write the canonical sidecars. Because `legacy-billing` already wrote successfully on the first batch, the second batch is idempotent for that row — the canonical content is byte-identical — and the only effective work is the canonical write for `legacy-api`.

After both rows succeed, the output directory looks like:

```text
.specify/plans/migrate-fleet/survey/
├── legacy-api/
│   ├── metadata.json
│   └── surfaces.json
└── legacy-billing/
    ├── metadata.json
    └── surfaces.json
```

> The canonical sidecar shapes match [`plugins/change/skills/survey/fixtures/multi-source-fleet/`](../../plugins/change/skills/survey/fixtures/multi-source-fleet/). If the fixture changes, this tutorial must also change.

### What just happened

Two sources were enumerated independently. The LLM over-reached on one surface in `legacy-api`; the CLI's path-under-source-root invariant caught it row-locally; the skill's repair loop got that one row through without touching `legacy-billing`. Per-row independence is the property that lets the batch form survive partial failure — the row that validated stays on disk, and re-runs only redo the work that needs redoing.

## 3. One combined inventory

After the CLI finishes, `/change:survey` reads all sidecars and runs the candidate algorithm on each source (see [`references/candidate-algorithm.md`](../../plugins/change/skills/survey/references/candidate-algorithm.md) for the full algorithm):

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

## Combining candidates across sources during propose

Survey does not pair candidates across sources mechanically — even when the surfaces look related. Cross-source combination is an explicit operator decision during `propose`, and it is a small ergonomic step on top of the candidate inventory.

A concrete example: imagine the `legacy-api` survey had also picked up an `external-call-out` surface where `POST /orders` calls `POST https://billing.internal/payments`, and that target lines up with `legacy-billing`'s `http-post-payments` route. The two surfaces describe two ends of the same flow and the operator wants them landing as a single slice.

Survey leaves both candidates separate. You combine them by editing the inventory entry before accepting it — either by hand-writing the merged candidate block in your draft `proposal.md` …

```yaml
kind: candidate
sources: [legacy-api, legacy-billing]
touches:
  - legacy-api/src/orders/create.ts
  - legacy-api/src/orders/repository.ts
  - legacy-api/src/orders/validate.ts
  - legacy-billing/src/payments/create.ts
  - legacy-billing/src/payments/repository.ts
surfaces:
  - legacy-api:external-call-out-billing-payments
  - legacy-api:http-post-orders
  - legacy-billing:http-post-payments
declared-at:
  - legacy-api:src/orders/create.ts:31
  - legacy-api:src/server.ts:14
  - legacy-billing:src/server.ts:12
```

… or by accepting the combined entry directly through the single-writer CLI seam:

```bash
specify plan add migrate-fleet \
    --name order-payment-flow \
    --source legacy-api \
    --source legacy-billing \
    --surface legacy-api:http-post-orders \
    --surface legacy-api:external-call-out-billing-payments \
    --surface legacy-billing:http-post-payments
```

`specify plan add` is the only path that writes `plan.yaml`; propose still owns the operator interaction. The surface ids stay namespaced so the plan entry preserves traceability to both source surveys. If you later regret the merge, drop the entry with `specify plan amend` and accept the two source-local candidates separately on the next pass.

## What v1 does not do

Survey in v1 is deliberately conservative about multi-source changes. Understanding these boundaries helps you know what to expect during `propose`:

- **No cross-source pairing.** Survey does not automatically merge candidates from different sources, even when surfaces look related (e.g. an outbound HTTP call in `legacy-api` that matches a route in `legacy-billing`). Evidence of cross-source communication is preserved in the surfaces, but pairing is an operator decision during `propose`.
- **No `depends-on` inference.** Survey does not infer dependency ordering from contract edges. Candidates are emitted in source order, then surface order. Dependency ordering is a `propose` and operator concern.
- **No automated routing.** Survey-derived candidates carry no `target-project`. Assignment uses today's signals: description match, baseline spec affinity, and adapter compatibility.
- **No cross-source identifier normalization.** Surface identifiers preserve the legacy spelling and are not canonicalized for matching. The same route from two repos (`/users`) remains two distinct surface ids (`legacy-api:http-get-users` and `legacy-billing:http-get-users` if both existed).

These are deliberate deferrals, not missing features. The evidence for cross-source relationships is captured in the surfaces — what v1 omits is the automated inference and merging that would act on that evidence. The [Combining candidates across sources during propose](#combining-candidates-across-sources-during-propose) section above shows the operator-driven path for the cases v1 leaves on the table. See [Legacy migration at scale (explanation)](../explanation/legacy-migration-at-scale.md) for the full list of deferrals and where the solutions will live.

## What you learned

- Multi-source changes produce one combined candidate inventory. The same skill-drives-LLM, CLI-validates pipeline runs once per row; the source count changes the breadth, not the model.
- Each source gets an independent staged candidate at `.specify/plans/<change>/survey/staged/<source-key>.json`, and the CLI writes per-source-key sidecars under the output directory. Row failure leaves that row's files untouched and does not affect other rows.
- The bounded repair loop runs per row: only the source that failed validation is re-prompted, only that row's canonical sidecar gets re-written, and validated rows from the same batch stay on disk across retries.
- Small sources (< 1000 LOC) emit as one source-level candidate. Large sources decompose into surface-level candidates with minimal clustering.
- Surface ids are namespaced `<source-key>:<surface-id>` so identifiers from different sources remain distinguishable.
- v1 does not pair candidates across sources, infer `depends-on` from contract edges, or route survey-derived candidates to target projects. These are operator decisions during `propose` — combine cross-source candidates by editing the inventory entry or by invoking `specify plan add` with multiple `--source` / `--surface` flags.
- The operator review point during `propose` is where you combine, reorder, or split candidates as needed.

## Next steps

- [Monolith Decomposition](monolith-decomposition.md) — single-source mechanics (sizing, clustering, the unresolved path).
- [Reviewing the plan](reviewing-a-plan.md) — the operator review seam between `/change:draft` and `/change:execute`.
- [Legacy Migration at Scale](legacy-migration-at-scale.md) — the end-to-end migration workflow including execution and landing.
- [Legacy migration at scale (explanation)](../explanation/legacy-migration-at-scale.md) — where cross-change scale concerns (source catalogues, migration ledgers, reconciliation) will live.
