# RFC-29b: Plan-Time Lead Reconciliation

> Status: Draft — Milestone **M2a** of [RFC-29](rfc-29-fan-in-fan-out.md) — Depends: [RFC-29a](rfc-29-fan-in-fan-out.md#sub-rfcs-and-milestone-ordering) (surveyed `discovery.md`) — Unblocks: [RFC-29c](rfc-29c-synthesis.md) plan rows

This milestone closes plan-time fan-in: surveyed `Lead[]` rows from multiple sources become the `plan.yaml.slices[]` rows that `/spec:execute` runs later.

- The **agent** decides which leads describe the same work, which projects own each slice, names each slice, and joins related slices with `depends-on`.
- The **CLI** validates the agent's response, emits one journal event, and writes the plan. The agent never hand-edits `plan.yaml`.
- The **operator** curates at **Gate 1** — after propose completes and before `approved` — via `change.md` review and `specrun plan amend`.

One noun:

- **slice** — one `plan.yaml.slices[]` row: an explicit kebab-case `name`, a bound `project`, and its matched `sources[]` (at most one lead per source) inline. Cross-source matching is agent judgment from per-source `synopsis` and shared slugs — never kernel-enforced. The agent never fuses two leads from the *same* source: each source's lead is its own candidate slice, sized by the source adapter, and same-source re-sizing is an operator action at Gate 1. A body of work that targets more than one project is expressed as **multiple slices** that may reference the same lead, joined by `depends-on` — there is no `scope` grouping noun (RFC-29 review F3 removed it). Each slice binds a project; its target is resolved on demand, not written to disk.

Shared wire contracts are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This document is the source of truth for D2.

## Decision

| ID | Decision |
| -- | -------- |
| **D2 Lead reconciliation** | `specrun plan propose` wraps agent-led lead reconciliation in a CLI-owned projection kernel. **The agent** emits `slices[]`, each carrying an explicit kebab-case `name`, its matched `sources[]` (at most one lead per source), and a bound `project` from `projects[]`. **The kernel** validates the response schema, enforces total lead coverage (every surveyed lead referenced by ≥1 slice), rejects same-source fusion per slice, enforces unique slice names, resolves each slice's `target` from the bound project, emits one journal event, and writes `plan.yaml.slices[]`. Cross-source matching is agent judgment; human curation happens at Gate 1 after propose. There is no `scope` grouping noun (RFC-29 review F3): cross-target fan-out is expressed as multiple ordinary slices joined by `depends-on`. |

## Operator Surface

```bash
specrun plan propose --dry-run --format json
specrun plan propose --from <response.json> [--format json]
specrun plan remove <entry>
```

`--dry-run` writes nothing. It reads `plan.yaml.sources`, the surveyed `discovery.md` lead inventory, and the project topology (`registry.yaml` for a hub, or the sole project synthesized from `project.yaml`), then returns a request envelope for the agent. When `discovery.md` carries no leads, `--dry-run` aborts with `plan-reconcile-empty-catalog` (exit 2).

`--from` is the only writer. On every invocation it **re-reads** `discovery.md`, **rebuilds** the lead catalog (it does not trust a prior `--dry-run` snapshot), validates the agent response against that fresh catalog, **replaces** all `plan.yaml.slices[]` rows on a replaceable pending plan, validates the explicit slice names, emits the reconciliation event, and writes slices through the existing `crates/workflow/src/change/plan/` writers. Slices are written in the agent's `slices[]` response order, so `plan.yaml.slices[]` ordering is a deterministic function of the response; `plan next` then applies `depends-on` eligibility over that order. Because `--from` replaces all rows wholesale, any per-slice operator edits made on a prior pending plan (a relabel, or a `--divergence` stamp) are discarded on re-propose — re-propose is a fresh projection, not a merge. Manual `specrun plan add` remains available for headless authoring; the default `/spec:plan` flow uses `propose --from` through the same writers.

Exactly one mode is required: `propose` with neither `--dry-run` nor `--from` fails with `plan-propose-mode-required`, and the argument parser rejects passing both at once.

A plan is replaceable only while `plan.lifecycle` is `pending` and every existing slice entry is still `pending`. If the plan is `approved`, or any entry is `in-progress` / `done`, `propose --from` fails with `plan-reconcile-plan-not-replaceable`. The same replaceable gate applies to `specrun plan remove <entry>` — Gate 1 deferral drops a pending entry without re-surveying `discovery.md`; the lead remains in `discovery.md` and resurfaces on the next `propose --from` unless the inventory is re-surveyed without it.

## Reconciliation Flow

1. `/spec:plan` runs `specrun plan propose --dry-run --format json`.
2. The CLI returns a flat `leads[]` catalog — one row per `(source, lead)` from `discovery.md`, plus `projects[]` (each entry carries the target adapter the agent may bind).
3. The agent returns `slices[]`. Each slice carries an explicit `name`, its matched `sources[]` (at most one lead per source), and a bound `project`. Every surveyed lead must be referenced by at least one slice; cross-target fan-out is multiple slices that reference the same lead, joined by `depends-on` (the kernel derives each row's `target` from the bound project).
4. `/spec:plan` submits the response with `specrun plan propose --from <response.json>`.
5. The CLI re-reads `discovery.md`, rebuilds the catalog, validates, and **replaces** all `plan.yaml.slices[]` rows.
6. The agent renders cross-source merge review prose into `change.md`.
7. `/spec:plan` exits at `pending`. The operator reviews at **Gate 1**, amends if needed, then runs `specrun plan transition <name> approved`.

The agent owns judgment during propose. The CLI owns projection and persistence. The operator owns curation before execution.

## Cross-Source Matching

Sources survey independently — there is no cross-source coordination step. Each catalog row is one raw `(source, lead)` lead with its per-source `synopsis`.

The agent decides which rows belong in the same slice:

- **Shared slug** — two sources may emit the same `lead` (e.g. both surface `identity-api`). That overlap is a hint, not a kernel lock. The agent may merge or keep them separate; accidental slug collision is resolved at Gate 1, not forced at propose time.
- **Synopsis judgment** — when ids do not suggest a link (e.g. `password-reset` and `reset-password`), the agent merges or splits from per-source synopses.

**At most one lead per source.** A slice matches leads *across* sources; it never fuses two leads from the *same* source. Each surveyed lead is that source adapter's candidate slice — a sizing judgment made with full visibility of the legacy code, documentation, or capture the agent does not have. Merging two same-source leads would override that sizing and risk a slice too large to execute, so the propose kernel rejects it (`plan-reconcile-slice-source-collision`). When a source genuinely over-fragments, the fix is a better source adapter or an operator Gate 1 merge via `specrun plan amend --sources` — where a human owns the sizing risk — not an agent propose-time fusion.

The kernel enforces shape only: every surveyed lead is referenced by at least one slice, and no slice names two leads from the same source. It does not auto-merge, cluster, or forbid cross-source splits. A lead referenced by more than one slice is legal **fan-out**, not a double-count: the same body of work targeting two projects becomes two slices that share a lead and are joined by `depends-on`.

The coverage invariant is **total**: propose must reference every surveyed lead from at least one slice's `sources[]`. Deferring a lead — choosing not to plan it in this change — is an operator action at Gate 1 (`specrun plan remove <entry>`), not an agent propose-time choice. There is no "deferred" bucket in the response, and a removed lead resurfaces on the next `propose --from` unless `discovery.md` is re-surveyed without it. A future RFC may add an explicit agent-side defer bucket if total coverage proves too strict; until then this is a deliberate "account for every lead" invariant at propose time only.

The survey-time `tentative` flag a source adapter may set on its own lead (`lead.schema.json`) is **not** surfaced in the request catalog. Grouping uncertainty is the agent's to express through `## Tentative merges` prose in `change.md`, not a per-lead input signal; keeping it off the wire avoids conflating source-side and grouping-side uncertainty.

### Coverage invariants

A surveyed lead referenced by no slice fails with `plan-reconcile-partition`; a `(source, lead)` pair naming no current catalog row fails with `plan-reconcile-lead-orphan`. A slice that names two leads from the same source fails with `plan-reconcile-slice-source-collision`. A non-kebab-case slice name fails with `plan-reconcile-slice-name-invalid` (rejected as `proposal-schema` at the wire gate before the kernel sees it). Two slices resolving to the same name fail with `plan-reconcile-slice-name-collision`. A cyclic `depends-on` graph fails with `plan-reconcile-depends-on-cycle`.

### Why fan-out is implicit (no `scope` noun)

An earlier draft carried a `scope` grouping id and forced every slice sharing it to carry an identical `sources[]` set, with the kernel partitioning leads *exactly once* across scopes and deriving slice names from `scope` (or `scope`-plus-`project` on fan-out). RFC-29 review F3 removed it: the grouping noun, the `plan-reconcile-fanout-source-mismatch` and `plan-reconcile-slice-duplicate` invariants it needed, and all kernel-side name derivation. Fan-out is now **implicit** — two slices that target different projects may simply reference the same lead, ordered by `depends-on`. The agent already names slices in practice, so it names every slice directly (the review's "explicit names disambiguate the different-slug case," taken to its clean conclusion: `docs:password-reset` and `legacy:reset-password` reconcile into one slice the agent names `password-reset`, with no source's slug privileged). The exactly-once partition relaxes to **total coverage** — every surveyed lead referenced by at least one slice — because cross-slice lead reuse is now legal fan-out rather than a double-count.

## Envelope

Request and response validate against `schemas/discovery/proposal.schema.json` (`PROPOSAL_JSON_SCHEMA`), with a closed `kind: request | response` discriminator.

The request is lead-centric: flat `leads[]` carries one row per raw `(source, lead)` lead. Each catalog row carries `source`, `lead`, and per-source `synopsis`.

Each raw lead from `discovery.md` becomes one catalog row — no expansion or cross-source merge at survey or request time. `lead` is unique only within a `source`; the same slug under different sources is legal. Catalog identity is the `(source, lead)` pair. Envelope and `plan.yaml` slice bindings use the same shape.

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
  - source: docs
    lead: identity-api
    synopsis: "Identity API contract for authentication and account access."
  - source: legacy
    lead: identity-api
    synopsis: "Legacy identity endpoints."
  - source: docs
    lead: password-reset
    synopsis: "Users can request a password reset email."
  - source: legacy
    lead: reset-password
    synopsis: "Legacy reset-password flow."
```

The agent may merge `docs:identity-api` with `legacy:identity-api` by shared slug, and `password-reset` with `reset-password` by synopsis — both are judgment calls surfaced for Gate 1 review.

`projects[]` lists every project the agent may bind and always carries at least one entry (a single regular project is synthesized from `project.yaml`). Available targets come from `projects[].target` — there is no separate request-level `targets[]`. Each entry may also carry the derived-identity surfaces `surface[]` / `decisions[]` / `recent[]` ([RFC-36](rfc-36-project-identity.md)) so the agent can bind on actual owned behaviour and architectural commitment, not description prose alone.

The dry-run envelope normalizes project topology into `{ name, target, description, surface, decisions, recent }`: hub projects are projected from the committed `.specify/topology.lock` (regenerated by `specrun workspace sync` from each member project's `project.yaml` plus its baseline per [RFC-36](rfc-36-project-identity.md)); a single regular project resolves `.specify/project.yaml.adapter` through the target adapter resolver into a `name@vN` ref. The normalized `projects[].target` is envelope-local; the kernel uses it only to resolve a bound slice's target on demand and never writes it to `plan.yaml` — a slice persists only its `project`.

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
  - source: intent
    lead: fix-typo
    synopsis: "fix typo in user.rs"
```

Response:

```yaml
version: 1
kind: response
slices:
  - name: fix-typo
    sources:
      - { source: intent, lead: fix-typo }
```

The kernel auto-binds `my-app` and writes one slice whose `sources[]` carries the explicit `{ source: intent, lead: fix-typo }` binding; the target (`omnia@v1`) is resolved on demand from the bound project, not persisted. The kernel always emits the structured `{ source, lead }` form; the bare `[intent]` shorthand stays available for hand-authoring via `specrun plan add`, but the projection kernel never depends on the slice name matching a lead.

Example response (multi-source):

```yaml
version: 1
kind: response
slices:
  - name: identity-contracts
    sources:
      - { source: docs, lead: identity-api }
      - { source: legacy, lead: identity-api }
    project: identity-contracts
    rationale: "identity API surface matched by shared slug across docs + legacy"
  - name: identity-service
    sources:
      - { source: docs, lead: identity-api }
      - { source: legacy, lead: identity-api }
    project: identity-service
    depends-on: [identity-contracts]
  - name: password-reset
    sources:
      - { source: docs, lead: password-reset }
      - { source: legacy, lead: reset-password }
    project: identity-service
    rationale: "password-reset (docs) and reset-password (legacy) are the same flow by synopsis judgment"
```

The two same-source leads `docs:password-reset` and `docs:identity-api` stay in **separate** slices — the agent matches across sources, never fusing one source's candidate slices. The `identity-api` lead fans out (one body of work → a contracts crate and an omnia service), so two slices (`identity-contracts`, `identity-service`) reference it and are ordered by `depends-on`; `password-reset` is a single slice bound to one project.

`slices[]` is the only list. Each slice carries an explicit `name`, its matched `sources[]` (each a catalog row referenced by `{ source, lead }`, at most one per source), an optional `project`, an optional `rationale`, and optional dependencies. A lead may be referenced by more than one slice — that is fan-out, ordered by `depends-on`. **`rationale`** is agent-authored prose the agent renders into `change.md` for Gate 1 review; the kernel ignores it and does not echo it into the journal. A slice SHOULD carry `rationale` when the cross-source match is not obvious. There is no response-level `target` — the kernel resolves it on demand from the bound project's `projects[].target`. `depends-on` names other slice names. The kernel writes each slice's `sources[]` to `plan.yaml.slices[].sources[]` as the structured `{ source, lead }` form and writes `project`; the target is never written to disk.

### Persisted `plan.yaml`

Projecting that multi-source response writes the following `plan.yaml` at the repo root. `rationale` is propose-time only — it does not reach disk; the kernel writes only each slice's `name`, bound `project`, `sources[]`, and `depends-on`, and stamps every entry `pending`. The target adapter is resolved on demand from that project and is not persisted:

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
    project: identity-contracts
    sources:
      - { source: docs, lead: identity-api }
      - { source: legacy, lead: identity-api }
  - name: identity-service
    status: pending
    project: identity-service
    depends-on: [identity-contracts]
    sources:
      - { source: docs, lead: identity-api }
      - { source: legacy, lead: identity-api }
  - name: password-reset
    status: pending
    project: identity-service
    sources:
      - { source: docs, lead: password-reset }
      - { source: legacy, lead: reset-password }
```

The top-level `name` is the change name (kebab-case; the archive filename prefix). `lifecycle` carries the two stored gate states (`pending | approved`) — `currently executing` and `drained` are computed from per-entry `status` at read time. Top-level `sources` is the named-source map each slice's `sources[]` resolves against; each value is the structured `{ adapter, path? | value? }` object with exactly one of `path` or `value`. Per slice, `status` walks the collapsed `pending → in-progress → done` enum, and each entry binds a `project` (optional on disk — an omitted value resolves to the sole topology project; the target adapter is resolved on demand from that project, not stored per slice). Optional per-slice fields not shown here: `divergence` (`likely | accepted | rejected`), the per-slice `authority-override` map keyed by claim kind ([RFC-29c §"Authority resolution"](rfc-29c-synthesis.md)), `context`, and `description`. The normative shape is `schemas/plan/plan.schema.json`.

## Slice Names

Each `slices[]` entry becomes one `plan.yaml.slices[]` entry, and every slice carries an explicit kebab-case `name` (RFC-29 review F3 removed kernel name derivation along with the `scope` noun). The kernel:

1. Validates each `name` is kebab-case (`plan-reconcile-slice-name-invalid`; the wire schema's `kebabName` pattern catches a malformed name as `proposal-schema` first).
2. Writes the name verbatim to `plan.yaml.slices[].name`.
3. Validates every `depends-on` entry against the name set and rejects cyclic graphs with `plan-reconcile-depends-on-cycle`.

Name uniqueness is the sole duplicate gate: two slices resolving to the same name fail `plan-reconcile-slice-name-collision` (this subsumes the former `(scope, project)` duplicate check). `depends-on` resolves against these explicit names directly — there is no longer a derived `<scope>-<project>` form to reference, so a fanned-out slice you depend on is named directly.

## Project Binding

Every slice resolves to exactly one project. The dry-run request carries `projects[]` as `{ name, target, description, surface, decisions, recent }` — for a hub, projected from the committed `.specify/topology.lock` ([RFC-36](rfc-36-project-identity.md)); for a single regular project, one entry synthesized from `project.yaml` (`name`, target adapter, `description`) plus the live baseline projection (`surface`, `decisions`, `recent`).

The agent binds `project` on each slice by matching the slice against each project's `target`, `description`, and derived `surface[]` / `decisions[]`. When `projects[]` has exactly one entry the agent may omit `project` and the kernel auto-binds it. When more than one project exists the agent must name one explicitly; the CLI never chooses among candidates.

Before writing, the kernel enforces:

1. A slice may omit `project` only when exactly one project exists; otherwise every slice must name one (`plan-reconcile-project-binding-required`).
2. The named (or auto-bound) project must exist in `projects[]` (`plan-reconcile-project-orphan`).

The kernel writes `project` verbatim to `plan.yaml.slices[].project`; the target adapter is resolved on demand from that project's `projects[].target` entry and is not persisted. Build-time routing resolves a hub project against `registry.yaml` as described in [RFC-29c §"Per-slice fan-out (D5)"](rfc-29c-synthesis.md).

## Gate 1 Review

Human curation happens **after** propose and **before** execution. `/spec:plan` exits at `plan.lifecycle: pending`; the operator reads `change.md`, amends the plan if the agent's grouping is wrong, then stamps `approved`.

The agent renders cross-source merges into `change.md` so Gate 1 can review them. The kernel does not judge grouping correctness — it validates partition shape, derives names, and writes slices.

Operator override paths at Gate 1 (see [decision log §"Automated propose"](../docs/explanation/decision-log.md)). **Grouping and deferral** use re-propose, `plan add`, `plan remove`, or the recipes below. **`plan amend`** is the scalpel for divergence, authority overrides, and single-field / single-source fixes:

- **`specrun plan propose --from`** — re-run agent grouping (replaces all slices on a replaceable plan)
- **`specrun plan add` / `specrun plan remove`** — append or drop pending entries (structural edits)
- **`specrun plan amend <entry>`** — relabel, rebind sources/project/depends-on, accept or reject divergence; compose with `plan add` / `plan remove` for split and merge
- **Re-propose** — re-run `propose --from` on a still-pending plan after fixing `discovery.md` or adjusting the agent response

### Gate 1 recipes

D2 partition invariants apply only to `propose --from`. `plan add`, `plan amend`, and `plan remove` do not re-check them — the operator owns sizing and grouping risk at Gate 1.

| Goal | Commands |
| --- | --- |
| **Relabel / rebind** | `specrun plan amend <entry> --sources <key>=<lead> ...` (plus `--project`, `--depends-on` as needed) |
| **Split** | `specrun plan add <new-entry> --sources ...` then `specrun plan amend <original> --sources ...` (narrow bindings); `specrun plan remove <original>` when the original entry is empty |
| **Merge (cross-source)** | `specrun plan amend <keep> --sources ...` (union of bindings) then `specrun plan remove <drop>` |
| **Merge (same-source sizing override)** | Same as merge — allowed at Gate 1 only; propose kernel forbids this |
| **Defer a lead** | `specrun plan remove <entry>` — lead stays in `discovery.md` until re-survey or the next `propose --from` |

## Out Of Kernel Scope

These plan-time signals stay in the skill layer and existing CLI amend/remove paths:

- **`## Tentative merges` in `change.md`** — uncertain groupings; the agent never edits `discovery.md`.
- **`## Likely divergences` in `change.md`** — materially disagreeing per-source synopses after `propose --from` succeeds.
- **`plan.yaml.slices[].divergence: likely`** — staged after `propose --from` by `specrun plan amend <entry> --divergence likely`, because `propose --from` is the slice writer and slices do not exist until it runs. This is the only writer of the `divergence` field; `plan create` scaffolds an empty plan and never stamps divergence.

D2 coverage invariants (total lead coverage, at-most-one-lead-per-source per slice) are enforced only by `propose --from`. Gate 1 edits through `plan add`, `plan amend`, and `plan remove` may violate those invariants deliberately — including same-source fusion via `plan amend --sources` — because the operator owns that risk before stamping `approved`.

## Agent Responsibilities

During `/spec:plan`, the agent:

1. Calls `specrun plan propose --dry-run --format json`.
2. Matches catalog rows across sources by judgment from `synopsis` and shared slugs — at most one lead per source per slice, never fusing two leads from the same source.
3. Emits one `slices[]` row per slice: names it, lists the matched `sources[]`, and binds a `project` (or omits it when exactly one exists). Cross-target fan-out is multiple slices that reference the same lead, ordered by `depends-on`.
4. Adds `rationale` on non-obvious cross-source matches, plus `depends-on` on slices.
5. Calls `specrun plan propose --from <response.json>`.
6. Renders cross-source match review prose into `change.md` for Gate 1.
7. When synopses materially disagree on a matched slice, invokes `specrun plan amend <entry> --divergence likely`.

The agent never writes `plan.yaml`, never writes `discovery.md`, and never decides authority.

## Wire Contracts

Canonical closed tables live in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This milestone appends:

- **Journal event:** `plan.reconcile.completed` — one event emitted by `propose --from` on success (RFC-29 review F8 folded the former `plan.reconcile.agent` + `plan.reconcile.completed` pair into it). The skill does not call `specrun journal emit` for D2. Payload shape is pinned in RFC-29 §"Journal events".
- **Operational validation codes:** `plan-reconcile-empty-catalog`, `plan-reconcile-lead-orphan`, `plan-reconcile-partition`, `plan-reconcile-slice-source-collision`, `plan-reconcile-slice-name-invalid`, `plan-reconcile-slice-name-collision`, `plan-reconcile-depends-on-cycle`, `plan-reconcile-project-binding-required`, `plan-reconcile-project-orphan`, `plan-reconcile-plan-not-replaceable`, `plan-propose-mode-required`, `plan-remove-plan-not-replaceable`, `plan-remove-entry-referenced`. These are `Error::Validation` outcomes (or `Error::Diag` for `plan-entry-not-found` on remove) and abort with exit 2. The `plan-reconcile-*` codes name response-invariant failures; `plan-propose-mode-required` guards command-mode selection (neither `--dry-run` nor `--from`); `plan-reconcile-empty-catalog` fires when `discovery.md` has no leads; `plan-reconcile-slice-source-collision` fires when one slice names two leads from the same source; `plan-reconcile-slice-name-collision` fires on two slices resolving to one name; `plan-remove-plan-not-replaceable` and `plan-remove-entry-referenced` guard Gate 1 `plan remove`. (RFC-29 review F3 removed `plan-reconcile-fanout-source-mismatch` and `plan-reconcile-slice-duplicate` with the `scope` grouping they policed.)
- **Schema:** `schemas/discovery/proposal.schema.json` (`PROPOSAL_JSON_SCHEMA`).

## Appendix: Deferred Work

Items intentionally out of scope for this milestone:

1. **Kernel-side token-intersection locks** — auto-merging rows when `lead` slugs intersect across source keys. Rejected for D2: shared slugs are unattested (collision risk), and Gate 1 is the human curation step after agent propose.
2. **Kernel-side advisory clustering of open leads** — facet edges, lexical fallback, connected-component bucketing. Would require per-lead `blocking-keys[]` survey metadata not produced by current `lead.schema.json`.
3. **Optional lead target-axis hints** — deferred to a follow-on RFC. `target` is always kernel-derived from the bound project.
