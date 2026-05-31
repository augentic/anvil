# RFC-29b: Plan-Time Lead Reconciliation

> Status: Draft — Milestone **M2a** of [RFC-29](rfc-29-fan-in-fan-out.md) — Depends: [RFC-29a](rfc-29-fan-in-fan-out.md#sub-rfcs-and-milestone-ordering) (surveyed `discovery.md`) — Unblocks: [RFC-29c](rfc-29c-synthesis.md) plan rows

This milestone closes plan-time fan-in: surveyed `Lead[]` rows from multiple sources become the `plan.yaml.slices[]` rows that `/spec:execute` runs later.

- The **agent** decides which leads describe the same work, which projects own each slice, and how scopes fan out across the project topology.
- The **CLI** validates the agent's response, derives slice names, emits journal events, and writes the plan. The agent never hand-edits `plan.yaml`.
- The **operator** curates at **Gate 1** — after propose completes and before `approved` — via `change.md` review and `specrun plan amend`.

Two nouns:

- **scope** — the agent's unit of work. `scopes[]` partition surveyed leads exactly once; cross-source merging is agent judgment from per-source `summary`, optional `aliases[]` hints, and shared slugs — never kernel-enforced.
- **slice** — one `plan.yaml.slices[]` row from a `(scope, project)` pair. One scope may fan out to multiple slices (each project's `target` is written at projection time).

Shared wire contracts are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This document is the source of truth for D2.

## Decision

| ID | Decision |
| -- | -------- |
| **D2 Lead reconciliation** | `specrun plan propose` wraps agent-led lead reconciliation in a CLI-owned projection kernel. **The agent** partitions surveyed leads into `scopes[]`, then emits one or more `slices[]` rows per scope by binding each `(scope-id, project)` pair to a project from `projects[]`. **The kernel** validates the response schema, enforces the global lead partition, resolves each slice's `target` from the bound project, derives slice names, emits journal events, and writes `plan.yaml.slices[]`. Cross-source matching is agent judgment; human curation happens at Gate 1 after propose. |

## Operator Surface

```bash
specrun plan propose --dry-run --format json
specrun plan propose --from <response.json> [--format json]
```

`--dry-run` writes nothing. It reads `plan.yaml.sources`, the surveyed `discovery.md` lead inventory, and the project topology (`registry.yaml` for a hub, or the sole project synthesized from `project.yaml`), then returns a request envelope for the agent.

`--from` is the only writer. On every invocation it **re-reads** `discovery.md`, **rebuilds** the lead catalog (it does not trust a prior `--dry-run` snapshot), validates the agent response against that fresh catalog, **replaces** all `plan.yaml.slices[]` rows on a replaceable pending plan, derives slice names, emits reconciliation events, and writes slices through the existing `crates/workflow/src/change/plan/` writers. Manual `specrun plan add` remains available for headless authoring; the default `/spec:plan` flow uses `propose --from` through the same writers.

A plan is replaceable only while `plan.lifecycle` is `pending` and every existing slice entry is still `pending`. If the plan is `approved`, or any entry is `in-progress` / `done`, `propose --from` fails with `plan-reconcile-plan-not-replaceable`.

## Reconciliation Flow

1. `/spec:plan` runs `specrun plan propose --dry-run --format json`.
2. The CLI returns a flat `leads[]` catalog — one row per `(source-key, lead-id)` from `discovery.md`, plus `projects[]` (each entry carries the target adapter the agent may bind).
3. The agent returns a global `scopes[]` partition and `slices[]`. Scopes partition leads; slices fan scopes out to projects (the kernel derives each row's `target` from the bound project).
4. `/spec:plan` submits the response with `specrun plan propose --from <response.json>`.
5. The CLI re-reads `discovery.md`, rebuilds the catalog, validates, and **replaces** all `plan.yaml.slices[]` rows.
6. The agent renders cross-source merge review prose into `change.md`.
7. `/spec:plan` exits at `pending`. The operator reviews at **Gate 1**, amends if needed, then runs `specrun plan transition <name> approved`.

The agent owns judgment during propose. The CLI owns projection and persistence. The operator owns curation before execution.

## Cross-Source Matching

Sources survey independently — there is no cross-source coordination step. Each catalog row is one raw `(source-key, lead-id)` lead with its per-source `summary` and optional `aliases[]` (operator-authored hints from `discovery.md`; the kernel does not interpret them as locks).

The agent decides which rows belong in the same scope:

- **Shared slug** — two sources may emit the same `lead-id` (e.g. both surface `identity-api`). That overlap is a hint, not a kernel lock. The agent may merge or keep them separate; accidental slug collision is resolved at Gate 1, not forced at propose time.
- **Alias hints** — `aliases[]` on a row may bridge to another source's `lead-id`. The agent may use these when grouping; `specrun plan amend --add-alias` records operator knowledge on `discovery.md` for future replans.
- **Summary judgment** — when ids and aliases do not suggest a link (e.g. `password-reset` and `reset-password`), the agent merges or splits from per-source summaries.

The kernel enforces shape only: every surveyed lead appears exactly once across `scopes[].members[]`. It does not auto-merge, cluster, or forbid splits.

### Partition invariants

Missing or duplicate members fail with `plan-reconcile-partition`; a `(source-key, lead-id)` pair naming no current catalog row fails with `plan-reconcile-lead-orphan`. `scopes[].scope-id` values must be unique (`plan-reconcile-scope-duplicate`). Every `slices[].scope-id` must name a declared scope (`plan-reconcile-scope-orphan`), and every scope must have at least one slice (`plan-reconcile-scope-unbound`). Duplicate `(scope-id, project)` pairs fail with `plan-reconcile-slice-duplicate`. Colliding derived slice names fail with `plan-reconcile-slice-name-collision`. A cyclic `depends-on` graph fails with `plan-reconcile-depends-on-cycle`.

## Envelope

Request and response validate against `schemas/discovery/proposal.schema.json` (`PROPOSAL_JSON_SCHEMA`), with a closed `kind: request | response` discriminator.

The request is lead-centric: flat `leads[]` carries one row per raw `(source-key, lead-id)` lead. Each catalog row carries `source-key`, `lead-id`, per-source `summary`, and optional `aliases[]`.

Each raw lead from `discovery.md` becomes one catalog row — no expansion or cross-source merge at survey or request time. `lead-id` is unique only within a `source-key`; the same slug under different sources is legal. Catalog identity is the `(source-key, lead-id)` pair. Envelope and `plan.yaml` slice bindings use the same shape.

Example request:

```yaml
version: 1
kind: request
projects:
  - name: identity-contracts
    target: contracts@v1
    description: "Versioned API contracts crate for the identity domain."
  - name: identity-service
    target: omnia@v1
    description: "Omnia identity service implementing auth and password flows."
leads:
  - source-key: docs
    lead-id: identity-api
    summary: "Identity API contract for authentication and account access."
  - source-key: legacy
    lead-id: identity-api
    summary: "Legacy identity endpoints."
  - source-key: docs
    lead-id: password-reset
    summary: "Users can request a password reset email."
  - source-key: legacy
    lead-id: reset-password
    summary: "Legacy reset-password flow."
```

The agent may merge `docs:identity-api` with `legacy:identity-api` by shared slug, and `password-reset` with `reset-password` by summary — both are judgment calls surfaced for Gate 1 review.

`projects[]` lists every project the agent may bind and always carries at least one entry (a single regular project is synthesized from `project.yaml`). Available targets come from `projects[].target` — there is no separate request-level `targets[]`.

The dry-run envelope normalizes project topology into `{ name, target, description }`: hub projects read the registry target adapter field; a single regular project resolves `.specify/project.yaml.adapter` through the target adapter resolver into a `name@vN` ref. The normalized `projects[].target` is envelope-local and is written to `plan.yaml.slices[].target`.

### N=1 degenerate example

Pure-intent changes use the same envelope; the kernel auto-binds the sole project:

Request:

```yaml
version: 1
kind: request
projects:
  - name: my-app
    target: omnia@v1
    description: "Single Omnia service for this repository."
leads:
  - source-key: intent
    lead-id: fix-typo
    summary: "fix typo in user.rs"
```

Response:

```yaml
version: 1
kind: response
scopes:
  - scope-id: fix-typo
    members:
      - { source-key: intent, lead-id: fix-typo }
slices:
  - scope-id: fix-typo
```

The kernel auto-binds `my-app`, derives `target: omnia@v1`, and writes one slice whose `sources[]` uses the bare `[intent]` shorthand (normalised to `{ source-key: intent, lead-id: fix-typo }`).

Example response (multi-source):

```yaml
version: 1
kind: response
scopes:
  - scope-id: identity-api
    members:
      - { source-key: docs, lead-id: identity-api }
      - { source-key: legacy, lead-id: identity-api }
      - { source-key: docs, lead-id: password-reset }
      - { source-key: legacy, lead-id: reset-password }
    rationale: "identity API by shared slug; password reset merged from per-source summaries"
slices:
  - scope-id: identity-api
    name: identity-contracts
    project: identity-contracts
  - scope-id: identity-api
    name: identity-service
    project: identity-service
    depends-on: [identity-contracts]
```

`scopes[]` partitions surveyed leads. Each member references a catalog row by `{ source-key, lead-id }`. Multi-member scopes SHOULD carry `rationale` when the grouping is not obvious. `slices[]` is the plan-row projection: each slice names a `scope-id`, optional `project`, optional explicit `name`, and optional dependencies. There is no response-level `target` — the kernel resolves it from the bound project's `projects[].target`. A `scope-id` may appear in multiple slices when fanning out to projects; members are declared once under `scopes[]`. `depends-on` names derived slice names, not scope ids. The kernel projects members into `plan.yaml.slices[].sources[]` as `{ source-key, lead-id }`, writes `project`, and writes `target` from the matching `projects[]` entry.

## Slice Names

Each `slices[]` entry becomes one `plan.yaml.slices[]` entry. Name assignment:

1. If the agent provides `name`, validate and use it.
2. Else if `scope-id` is not already used as a slice name in this response, use `scope-id`.
3. Else use `<scope-id>-<adapter-slug>`, where `<adapter-slug>` is the segment before `@v` in the slice's resolved `target` (`contracts@v1` → `contracts`).

After deriving all names, the kernel validates every `depends-on` entry against that name set and rejects cyclic graphs with `plan-reconcile-depends-on-cycle`.

Set an explicit `name` on any slice another slice depends on — `depends-on` resolves against names derived only after submission.

## Project Binding

Every slice resolves to exactly one project. The dry-run request carries `projects[]` as `{ name, target, description }` — for a hub, the validated registry; for a single regular project, one entry synthesized from `project.yaml` (`name`, target adapter, `domain` as description).

The agent binds `project` on each slice by matching the scope against each project's `target` and `description`. When `projects[]` has exactly one entry the agent may omit `project` and the kernel auto-binds it. When more than one project exists the agent must name one explicitly; the CLI never chooses among candidates.

Before writing, the kernel enforces:

1. A slice may omit `project` only when exactly one project exists; otherwise every slice must name one (`plan-reconcile-project-binding-required`).
2. The named (or auto-bound) project must exist in `projects[]` (`plan-reconcile-project-orphan`).

The kernel writes `project` verbatim to `plan.yaml.slices[].project` and `target` from that project's `projects[].target` entry. Build-time routing resolves a hub project against `registry.yaml` as described in [RFC-29c §"Per-slice fan-out (D5)"](rfc-29c-synthesis.md).

## Gate 1 Review

Human curation happens **after** propose and **before** execution. `/spec:plan` exits at `plan.lifecycle: pending`; the operator reads `change.md`, amends the plan if the agent's grouping is wrong, then stamps `approved`.

The agent renders cross-source merges into `change.md` so Gate 1 can review them. The kernel does not judge grouping correctness — it validates partition shape, derives names, and writes slices.

Operator override paths at Gate 1 (see [decision log §"Automated propose"](../docs/explanation/decision-log.md)):

- **`specrun plan amend`** — split, merge, relabel, or rebind slices; accept or reject divergence
- **`specrun plan amend --add-alias`** — record a cross-source bridge on `discovery.md` for the next replan (not a propose-time lock)
- **Re-propose** — re-run `propose --from` on a still-pending plan after fixing `discovery.md` or adjusting the agent response

Recurring cross-source pairings the operator accepts may be promoted to aliases with `--add-alias` so future surveys surface the hint on disk.

## Out Of Kernel Scope

These plan-time signals stay in the skill layer and existing CLI amend paths:

- **`## Tentative merges` in `change.md`** — uncertain groupings; the agent never edits `discovery.md`.
- **`## Likely divergences` in `change.md`** — materially disagreeing per-source summaries after `propose --from` succeeds.
- **`plan.yaml.slices[].divergence: likely`** — written only by `specrun plan amend <plan> <slice> --divergence likely`.

## Agent Responsibilities

During `/spec:plan`, the agent:

1. Calls `specrun plan propose --dry-run --format json`.
2. Partitions all catalog rows into `scopes[]` by judgment from `summary`, shared slugs, and optional `aliases[]` hints.
3. Binds each scope to one or more projects, producing one slice per `(scope-id, project)` pair (or omits `project` when exactly one exists).
4. Adds `rationale` on non-obvious multi-member scopes, plus `depends-on` and optional `name` on slices.
5. Calls `specrun plan propose --from <response.json>`.
6. Renders cross-source merge review prose into `change.md` for Gate 1.
7. When summaries materially disagree on a merged slice, invokes `specrun plan amend <plan> <slice> --divergence likely`.

The agent never writes `plan.yaml`, never writes `discovery.md`, and never decides authority.

## Wire Contracts

Canonical closed tables live in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This milestone appends:

- **Journal events:** `plan.reconcile.agent`, `plan.reconcile.completed`.
- **Operational validation codes:** `plan-reconcile-lead-orphan`, `plan-reconcile-partition`, `plan-reconcile-scope-duplicate`, `plan-reconcile-scope-orphan`, `plan-reconcile-scope-unbound`, `plan-reconcile-slice-duplicate`, `plan-reconcile-slice-name-collision`, `plan-reconcile-depends-on-cycle`, `plan-reconcile-project-binding-required`, `plan-reconcile-project-orphan`, `plan-reconcile-plan-not-replaceable`, `plan-propose-missing-grouping`. These are `Error::Validation` outcomes and abort with exit 2. The `plan-reconcile-*` codes name response-invariant failures; `plan-propose-missing-grouping` guards command-argument selection (neither `--dry-run` nor `--from`).
- **Schema:** `schemas/discovery/proposal.schema.json` (`PROPOSAL_JSON_SCHEMA`).

## Appendix: Deferred Work

Items intentionally out of scope for this milestone:

1. **Kernel-side token-intersection locks** — auto-merging rows when `{lead-id} ∪ aliases[]` intersects across source keys. Rejected for D2: shared slugs are unattested (collision risk), and Gate 1 is the human curation step after agent propose.
2. **Kernel-side advisory clustering of open leads** — facet edges, lexical fallback, connected-component bucketing. Would require per-lead `blocking-keys[]` survey metadata not produced by current `lead.schema.json`.
3. **Optional lead target-axis hints** — deferred to a follow-on RFC. `target` is always kernel-derived from the bound project.
