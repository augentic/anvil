# RFC-29b: Plan-Time Lead Reconciliation

> Status: Draft — Milestone **M2a** of [RFC-29](rfc-29-fan-in-fan-out.md) — Depends: [RFC-29a](rfc-29-fan-in-fan-out.md#sub-rfcs-and-milestone-ordering) (surveyed `discovery.md`) — Unblocks: [RFC-29c](rfc-29c-synthesis.md) plan rows

This milestone closes plan-time fan-in: surveyed `Lead[]` rows from multiple sources become the `plan.yaml.slices[]` rows that `/spec:execute` runs later.

- The **agent** decides which leads describe the same work, which projects own each slice, and how scopes fan out across the project topology.
- The **CLI** validates the agent's response, derives slice names, emits journal events, and writes the plan. The agent never hand-edits `plan.yaml`.
- The **operator** curates at **Gate 1** — after propose completes and before `approved` — via `change.md` review and `specrun plan amend`.

Two nouns:

- **scope** — the reconciled unit of work: the set of leads the agent judges to be the same piece of work, **at most one lead per source**. A scope is expressed by a shared `scope` id across one or more `slices[]` rows that carry identical `sources[]`. Cross-source matching is agent judgment from per-source `summary`, optional `aliases[]` hints, and shared slugs — never kernel-enforced. The agent never fuses two leads from the *same* source: each source's lead is its own candidate slice, sized by the source adapter, and same-source re-sizing is an operator action at Gate 1.
- **slice** — one `plan.yaml.slices[]` row: a `(scope, project)` pair carrying its own `sources[]` inline. A 1:1 scope is one slice; a scope may fan out to multiple slices (each project's `target` is written at projection time).

Shared wire contracts are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This document is the source of truth for D2.

## Decision

| ID | Decision |
| -- | -------- |
| **D2 Lead reconciliation** | `specrun plan propose` wraps agent-led lead reconciliation in a CLI-owned projection kernel. **The agent** emits `slices[]`, each carrying a `scope` id, its matched `sources[]` (at most one lead per source), and a bound `project` from `projects[]`. **The kernel** validates the response schema, enforces the global lead partition over scopes, rejects same-source fusion, resolves each slice's `target` from the bound project, derives slice names, emits journal events, and writes `plan.yaml.slices[]`. Cross-source matching is agent judgment; human curation happens at Gate 1 after propose. |

## Operator Surface

```bash
specrun plan propose --dry-run --format json
specrun plan propose --from <response.json> [--format json]
specrun plan remove <entry>
```

`--dry-run` writes nothing. It reads `plan.yaml.sources`, the surveyed `discovery.md` lead inventory, and the project topology (`registry.yaml` for a hub, or the sole project synthesized from `project.yaml`), then returns a request envelope for the agent. When `discovery.md` carries no leads, `--dry-run` aborts with `plan-reconcile-empty-catalog` (exit 2).

`--from` is the only writer. On every invocation it **re-reads** `discovery.md`, **rebuilds** the lead catalog (it does not trust a prior `--dry-run` snapshot), validates the agent response against that fresh catalog, **replaces** all `plan.yaml.slices[]` rows on a replaceable pending plan, derives slice names, emits reconciliation events, and writes slices through the existing `crates/workflow/src/change/plan/` writers. Slices are written in the agent's `slices[]` response order, so `plan.yaml.slices[]` ordering is a deterministic function of the response; `plan next` then applies `depends-on` eligibility over that order. Because `--from` replaces all rows wholesale, any per-slice operator edits made on a prior pending plan (a relabel, or a `--divergence` stamp) are discarded on re-propose — re-propose is a fresh projection, not a merge. Manual `specrun plan add` remains available for headless authoring; the default `/spec:plan` flow uses `propose --from` through the same writers.

Exactly one mode is required: `propose` with neither `--dry-run` nor `--from` fails with `plan-propose-mode-required`, and the argument parser rejects passing both at once.

A plan is replaceable only while `plan.lifecycle` is `pending` and every existing slice entry is still `pending`. If the plan is `approved`, or any entry is `in-progress` / `done`, `propose --from` fails with `plan-reconcile-plan-not-replaceable`. The same replaceable gate applies to `specrun plan remove <entry>` — Gate 1 deferral drops a pending entry without re-surveying `discovery.md`; the lead remains in `discovery.md` and resurfaces on the next `propose --from` unless the inventory is re-surveyed without it.

## Reconciliation Flow

1. `/spec:plan` runs `specrun plan propose --dry-run --format json`.
2. The CLI returns a flat `leads[]` catalog — one row per `(source-key, lead-id)` from `discovery.md`, plus `projects[]` (each entry carries the target adapter the agent may bind).
3. The agent returns `slices[]`. Each slice carries a `scope` id, its matched `sources[]` (at most one lead per source), and a bound `project`. Scopes partition the surveyed leads exactly once; fan-out is multiple slices sharing one `scope` (the kernel derives each row's `target` from the bound project).
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

**At most one lead per source.** A scope matches leads *across* sources; it never fuses two leads from the *same* source. Each surveyed lead is that source adapter's candidate slice — a sizing judgment made with full visibility of the legacy code, documentation, or capture the agent does not have. Merging two same-source leads would override that sizing and risk a slice too large to execute, so the propose kernel rejects it (`plan-reconcile-slice-source-collision`). When a source genuinely over-fragments, the fix is a better source adapter or an operator Gate 1 merge via `specrun plan amend --sources` — where a human owns the sizing risk — not an agent propose-time fusion.

The kernel enforces shape only: collapsing `slices[]` by `scope` id, every surveyed lead appears exactly once across the resulting scopes, and no scope names two leads from the same source. It does not auto-merge, cluster, or forbid cross-source splits.

The partition is **total**: propose must place every surveyed lead into some scope's `sources[]` (every scope necessarily has at least one slice, since scopes exist only as `slices[]` rows). Deferring a lead — choosing not to plan it in this change — is an operator action at Gate 1 (`specrun plan remove <entry>`), not an agent propose-time choice. There is no "unscoped" or "deferred" bucket in the response, and a removed lead resurfaces on the next `propose --from` unless `discovery.md` is re-surveyed without it. A future RFC may add an explicit agent-side defer bucket if total partition proves too strict; until then this is a deliberate "account for every lead" invariant at propose time only.

The survey-time `tentative` flag a source adapter may set on its own lead (`lead.schema.json`) is **not** surfaced in the request catalog. Grouping uncertainty is the agent's to express through `## Tentative merges` prose in `change.md`, not a per-lead input signal; keeping it off the wire avoids conflating source-side and grouping-side uncertainty.

### Partition invariants

Collapsing `slices[]` by `scope` yields the scope partition: a missing or double-counted lead across scopes fails with `plan-reconcile-partition`; a `(source-key, lead-id)` pair naming no current catalog row fails with `plan-reconcile-lead-orphan`. A scope that names two leads from the same source fails with `plan-reconcile-slice-source-collision`. Slices that share a `scope` id but carry differing `sources[]` fail with `plan-reconcile-fanout-source-mismatch`. Duplicate `(scope, project)` pairs fail with `plan-reconcile-slice-duplicate`. Colliding agent-supplied explicit slice names fail with `plan-reconcile-slice-name-collision`. A cyclic `depends-on` graph fails with `plan-reconcile-depends-on-cycle`.

### Why `scope` is an explicit field

The kernel could derive scope without an agent-supplied id by grouping slices that carry an identical `sources[]` set — under the total partition, two slices with identical `sources[]` reference identical leads and can therefore only be the same scope. That derivation would drop the `scope` field from the response and make `plan-reconcile-fanout-source-mismatch` impossible by construction (the shared `sources[]` set *is* the group). It is deliberately **rejected**: an explicit `scope` id is the only collision-free **name stem** when matched leads carry different slugs. `docs:password-reset` and `legacy:reset-password` reconcile to one slice, but there is no non-arbitrary derived name for it unless the agent names the scope (`password-reset`) — deriving from either source's slug would privilege one source over the other. The explicit id additionally anchors the scope-level `rationale` and the deduped `plan.reconcile.agent` journal payload. The cost of keeping it is exactly one extra invariant — `plan-reconcile-fanout-source-mismatch`, that slices sharing a `scope` carry identical `sources[]` — plus the agent's obligation to mint a stable id and reuse it verbatim across a fan-out group. `scope` is propose-time only; it is never written to `plan.yaml` (slice identity on disk is the derived `name`, and fan-out membership is recoverable from identical `sources[]`).

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
slices:
  - name: fix-typo
    scope: fix-typo
    sources:
      - { source-key: intent, lead-id: fix-typo }
```

The kernel auto-binds `my-app`, derives `target: omnia@v1`, and writes one slice whose `sources[]` carries the explicit `{ source-key: intent, lead-id: fix-typo }` binding. The kernel always emits the structured `{ source-key, lead-id }` form; the bare `[intent]` shorthand stays available for hand-authoring via `specrun plan add`, but the projection kernel never depends on the slice name matching a lead-id.

Example response (multi-source):

```yaml
version: 1
kind: response
slices:
  - name: identity-contracts
    scope: identity-api
    sources:
      - { source-key: docs, lead-id: identity-api }
      - { source-key: legacy, lead-id: identity-api }
    project: identity-contracts
    rationale: "identity API surface matched by shared slug across docs + legacy"
  - name: identity-service
    scope: identity-api
    sources:
      - { source-key: docs, lead-id: identity-api }
      - { source-key: legacy, lead-id: identity-api }
    project: identity-service
    depends-on: [identity-contracts]
  - name: password-reset
    scope: password-reset
    sources:
      - { source-key: docs, lead-id: password-reset }
      - { source-key: legacy, lead-id: reset-password }
    project: identity-service
    rationale: "password-reset (docs) and reset-password (legacy) are the same flow by summary judgment"
```

The two same-source leads `docs:password-reset` and `docs:identity-api` stay in **separate** scopes — the agent matches across sources, never fusing one source's candidate slices. `identity-api` fans out (one body of work → a contracts crate and an omnia service), so two slices share that `scope` id and carry identical `sources[]`; `password-reset` is a 1:1 scope bound to a single project.

`slices[]` is the only list. Each slice carries a `scope` id, its matched `sources[]` (each a catalog row referenced by `{ source-key, lead-id }`, at most one per source), an optional `project`, optional explicit `name`, optional `rationale`, and optional dependencies. Every slice sharing a `scope` id MUST carry an identical `sources[]` set — that shared set is the reconciled scope, and the per-scope sets partition the surveyed leads exactly once. **`rationale` is scope-level**: attach it on any one slice in a fan-out group (not on every row). The kernel dedupes by `scope` when echoing into the `plan.reconcile.agent` journal payload and when the agent renders `change.md`. A scope SHOULD carry `rationale` when the cross-source match is not obvious. There is no response-level `target` — the kernel resolves it from the bound project's `projects[].target`. A `scope` id appears in multiple slices only when fanning out to projects; `depends-on` names derived slice names, not scope ids. The kernel writes each slice's `sources[]` to `plan.yaml.slices[].sources[]` as the structured `{ source-key, lead-id }` form, writes `project`, and writes `target` from the matching `projects[]` entry.

### Persisted `plan.yaml`

Projecting that multi-source response writes the following `plan.yaml` at the repo root. `scope` and `rationale` are propose-time only — neither reaches disk; the kernel derives each slice's `target` from its bound project and stamps every entry `pending`:

```yaml
name: identity-platform
lifecycle: pending
sources:
  docs:
    adapter: documentation
    path: docs/identity/
  legacy:
    adapter: code-typescript
    path: src/
slices:
  - name: identity-contracts
    status: pending
    target: contracts@v1
    project: identity-contracts
    sources:
      - { source-key: docs, lead-id: identity-api }
      - { source-key: legacy, lead-id: identity-api }
  - name: identity-service
    status: pending
    target: omnia@v1
    project: identity-service
    depends-on: [identity-contracts]
    sources:
      - { source-key: docs, lead-id: identity-api }
      - { source-key: legacy, lead-id: identity-api }
  - name: password-reset
    status: pending
    target: omnia@v1
    project: identity-service
    sources:
      - { source-key: docs, lead-id: password-reset }
      - { source-key: legacy, lead-id: reset-password }
```

The top-level `name` is the change name (kebab-case; the archive filename prefix). `lifecycle` carries the two stored gate states (`pending | approved`) — `currently executing` and `drained` are computed from per-entry `status` at read time. Top-level `sources` is the named-source map each slice's `sources[]` resolves against; each value is the structured `{ adapter, path? | value? }` object with exactly one of `path` or `value`. Per slice, `status` walks the collapsed `pending → in-progress → done` enum, and each entry declares at least one of `target` (`name@vN`) or `project`. Optional per-slice fields not shown here: `divergence` (`likely | accepted | rejected`), the per-slice `authority-override` map keyed by claim kind ([RFC-29c §"Authority resolution"](rfc-29c-synthesis.md)), `context`, and `description`. The normative shape is `schemas/plan/plan.schema.json`.

## Slice Names

Each `slices[]` entry becomes one `plan.yaml.slices[]` entry. The derivation is keyed on the `(scope, project)` pair, which the partition kernel already proves unique (`plan-reconcile-slice-duplicate`), so every derived name is collision-free by construction:

1. If the agent provides `name`, validate and use it.
2. Else if the slice's `scope` projects to exactly one slice (a 1:1 scope, no fan-out), use `scope`.
3. Else (a scope fanning out to more than one project) use `<scope>-<project>` for **every** slice in that fan-out. Deriving from the bound `project` — not the target adapter — keeps fan-out names unique even when two projects share one adapter, and makes the result symmetric and independent of response order.

After deriving all names, the kernel validates every `depends-on` entry against that name set and rejects cyclic graphs with `plan-reconcile-depends-on-cycle`.

Because derived names are collision-free, `plan-reconcile-slice-name-collision` fires only when the agent supplies two explicit `name` values that clash. Set an explicit `name` on any slice another slice depends on — `depends-on` resolves against names derived only after submission, so a fanned-out slice you depend on is easier to reference by an explicit name than by its `<scope>-<project>` derivation.

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

Operator override paths at Gate 1 (see [decision log §"Automated propose"](../docs/explanation/decision-log.md)). **Grouping and deferral** use re-propose, `plan add`, `plan remove`, or the recipes below. **`plan amend`** is the scalpel for divergence, authority overrides, and single-field / single-source fixes:

- **`specrun plan propose --from`** — re-run agent grouping (replaces all slices on a replaceable plan)
- **`specrun plan add` / `specrun plan remove`** — append or drop pending entries (structural edits)
- **`specrun plan amend <entry>`** — relabel, rebind sources/project/depends-on, accept or reject divergence; compose with `plan add` / `plan remove` for split and merge
- **`specrun plan amend --add-alias`** — record a cross-source bridge on `discovery.md` for the next replan (not a propose-time lock; requires an existing `<entry>` positional even though the write targets `discovery.md`)
- **Re-propose** — re-run `propose --from` on a still-pending plan after fixing `discovery.md` or adjusting the agent response

### Gate 1 recipes

D2 partition invariants apply only to `propose --from`. `plan add`, `plan amend`, and `plan remove` do not re-check them — the operator owns sizing and grouping risk at Gate 1.

| Goal | Commands |
| --- | --- |
| **Relabel / rebind** | `specrun plan amend <entry> --sources <key>=<lead-id> ...` (plus `--project`, `--depends-on` as needed) |
| **Split** | `specrun plan add <new-entry> --sources ...` then `specrun plan amend <original> --sources ...` (narrow bindings); `specrun plan remove <original>` when the original entry is empty |
| **Merge (cross-source)** | `specrun plan amend <keep> --sources ...` (union of bindings) then `specrun plan remove <drop>` |
| **Merge (same-source sizing override)** | Same as merge — allowed at Gate 1 only; propose kernel forbids this |
| **Defer a lead** | `specrun plan remove <entry>` — lead stays in `discovery.md` until re-survey or the next `propose --from` |

Recurring cross-source pairings the operator accepts may be promoted to aliases with `--add-alias` so future surveys surface the hint on disk.

## Out Of Kernel Scope

These plan-time signals stay in the skill layer and existing CLI amend/remove paths:

- **`## Tentative merges` in `change.md`** — uncertain groupings; the agent never edits `discovery.md`.
- **`## Likely divergences` in `change.md`** — materially disagreeing per-source summaries after `propose --from` succeeds.
- **`plan.yaml.slices[].divergence: likely`** — staged after `propose --from` by `specrun plan amend <entry> --divergence likely`, because `propose --from` is the slice writer and slices do not exist until it runs. This is the only writer of the `divergence` field; `plan create` scaffolds an empty plan and never stamps divergence.

D2 partition invariants (total lead coverage, at-most-one-lead-per-source per scope, fan-out `sources[]` consistency) are enforced only by `propose --from`. Gate 1 edits through `plan add`, `plan amend`, and `plan remove` may violate those invariants deliberately — including same-source fusion via `plan amend --sources` — because the operator owns that risk before stamping `approved`.

## Agent Responsibilities

During `/spec:plan`, the agent:

1. Calls `specrun plan propose --dry-run --format json`.
2. Matches catalog rows across sources by judgment from `summary`, shared slugs, and optional `aliases[]` hints — at most one lead per source per scope, never fusing two leads from the same source.
3. Emits one `slices[]` row per `(scope, project)` pair: assigns a `scope` id, lists the matched `sources[]`, and binds a `project` (or omits it when exactly one exists). Fan-out repeats the `scope` id and identical `sources[]` across the rows.
4. Adds `rationale` on non-obvious cross-source matches, plus `depends-on` and optional `name` on slices.
5. Calls `specrun plan propose --from <response.json>`.
6. Renders cross-source match review prose into `change.md` for Gate 1.
7. When summaries materially disagree on a matched slice, invokes `specrun plan amend <entry> --divergence likely`.

The agent never writes `plan.yaml`, never writes `discovery.md`, and never decides authority.

## Wire Contracts

Canonical closed tables live in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This milestone appends:

- **Journal events:** `plan.reconcile.agent`, `plan.reconcile.completed`. Both are emitted atomically by `propose --from` on success — the skill does not call `specrun journal emit` for D2. Payload shapes are pinned in RFC-29 §"Journal events".
- **Operational validation codes:** `plan-reconcile-empty-catalog`, `plan-reconcile-lead-orphan`, `plan-reconcile-partition`, `plan-reconcile-slice-source-collision`, `plan-reconcile-fanout-source-mismatch`, `plan-reconcile-slice-duplicate`, `plan-reconcile-slice-name-collision`, `plan-reconcile-depends-on-cycle`, `plan-reconcile-project-binding-required`, `plan-reconcile-project-orphan`, `plan-reconcile-plan-not-replaceable`, `plan-propose-mode-required`, `plan-remove-plan-not-replaceable`, `plan-remove-entry-referenced`. These are `Error::Validation` outcomes (or `Error::Diag` for `plan-entry-not-found` on remove) and abort with exit 2. The `plan-reconcile-*` codes name response-invariant failures; `plan-propose-mode-required` guards command-mode selection (neither `--dry-run` nor `--from`); `plan-reconcile-empty-catalog` fires when `discovery.md` has no leads; `plan-reconcile-slice-source-collision` fires when one scope names two leads from the same source; `plan-reconcile-fanout-source-mismatch` when slices sharing a `scope` carry differing `sources[]`; `plan-reconcile-slice-name-collision` fires only on clashing agent-supplied explicit `name` values — derived names are unique by construction; `plan-remove-plan-not-replaceable` and `plan-remove-entry-referenced` guard Gate 1 `plan remove`.
- **Schema:** `schemas/discovery/proposal.schema.json` (`PROPOSAL_JSON_SCHEMA`).

## Appendix: Deferred Work

Items intentionally out of scope for this milestone:

1. **Kernel-side token-intersection locks** — auto-merging rows when `{lead-id} ∪ aliases[]` intersects across source keys. Rejected for D2: shared slugs are unattested (collision risk), and Gate 1 is the human curation step after agent propose.
2. **Kernel-side advisory clustering of open leads** — facet edges, lexical fallback, connected-component bucketing. Would require per-lead `blocking-keys[]` survey metadata not produced by current `lead.schema.json`.
3. **Optional lead target-axis hints** — deferred to a follow-on RFC. `target` is always kernel-derived from the bound project.
