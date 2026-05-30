# RFC-29b: Plan-Time Lead Reconciliation

> Status: Draft — Milestone **M2a** of [RFC-29](rfc-29-fan-in-fan-out.md) — Depends: [RFC-29a M1 (shipped)](rfc-29-fan-in-fan-out.md#sub-rfcs-and-milestone-ordering) (surveyed `discovery.md`) — Unblocks: the M2b plan rows in [RFC-29c](rfc-29c-synthesis.md)

This milestone closes plan-time fan-in. It turns the surveyed `Lead[]` from multiple sources into the `plan.yaml.slices[]` rows that `/spec:execute` will later run.

The important split is simple:

- The **agent** decides which leads describe the same work, which target slices to emit, and which project owns each slice.
- The **CLI** supplies deterministic locked groups, validates the agent's response, derives slice names, emits journal events, and writes the plan. The agent never hand-edits `plan.yaml`.

Three nouns carry the milestone; keep them distinct:

- **group** — a kernel-computed set of `(source-key, lead-id)` rows proved to be the same work by exact id or alias. Deterministic, enforced, never split.
- **concept** — the agent's unit of work. `concepts[]` partition the surveyed leads exactly once; a concept may absorb several locked groups plus open leads merged by judgment.
- **slice** — one `plan.yaml.slices[]` row, produced by binding a `(concept, target)` pair. One concept fans out to multiple slices across targets.

The shared wire contracts this milestone extends are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This document is the source of truth for D2.

## Decision


| ID                         | Decision                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **D2 Lead reconciliation** | `specrun plan propose` wraps agent-led lead reconciliation in a CLI-owned projection kernel. **The agent** partitions surveyed leads into `concepts[]`, then emits one or more `slices[]` rows per concept by binding each `(concept-id, target)` pair to one target and one project. **The kernel** computes deterministic locked groups (exact id / alias), validates the response schema, enforces the global lead partition, prevents locked groups from being split, carries each locked member's proven `match-basis` forward, validates project bindings, derives slice names, emits journal events, and writes `plan.yaml.slices[]`. |


## Operator Surface

```bash
specrun plan propose --dry-run --format json
specrun plan propose --from <response.json> [--format json]
```

`--dry-run` writes nothing. It reads `plan.yaml.sources`, the surveyed `discovery.md` lead inventory, operator-authored aliases, targets, and the project topology (`registry.yaml` for a hub, or the sole project synthesized from `project.yaml`). It returns a request envelope for the agent.

`--from` is the only writer. It validates the agent response, enforces the invariants below, derives slice names, emits reconciliation events, and writes slices through the existing `crates/workflow/src/change/plan/` writers.

## Reconciliation Flow

1. `/spec:plan` runs `specrun plan propose --dry-run --format json`.
2. The CLI returns a flat `leads[]` catalog — one row per raw `(source-key, lead-id)` lead read 1:1 from `discovery.md`, each optionally carrying a `group` block when the kernel proved it is the same work across sources (rows without one being open leads) — plus `targets[]` and `projects[]`.
3. The agent groups the open leads by judgment but returns one global `concepts[]` partition plus `slices[]`. Concepts partition the surveyed leads; slices fan those concepts out to targets and projects.
4. `/spec:plan` submits that response with `specrun plan propose --from <response.json>`.
5. The CLI validates and writes `plan.yaml.slices[]`.
6. The agent renders semantic matches into `change.md` so Gate 1 can review them.

The agent owns judgment. The CLI owns projection and persistence.

## Locked Groups

Before the agent sees the inventory, the kernel computes conservative locked groups that may not be split. It binds each member lead with a `group` block proving membership on one of two deterministic bases:

1. Exact `lead-id` match across source keys — the same `lead-id` surfaced by more than one source binding (`match-basis: exact-id`).
2. Exact alias match across source keys, recorded under the canonical `lead-id` (`match-basis: exact-alias`).

A locked group is a pure function of `discovery.md` and is defined implicitly by the catalog rows that share a `group.group-id`; there is no separate group index and no kernel-maintained reverse membership list. The agent may extend a locked group with semantically matched open leads inside a single concept, but `propose --from` rejects any response that splits a locked group across concepts with `plan-reconcile-locked-group-split`.

Open leads — those the kernel could not prove — carry no `group` block. They are returned as a flat catalog and grouped entirely by the agent's judgment from their per-source `summary`; the kernel does no heuristic clustering of open leads, surfaces no facets, and never auto-merges them. Semantic groupings the agent makes are surfaced for operator review at Gate 1 (`match-basis: semantic`).

> Deferred (YAGNI): kernel-side advisory clustering of open leads (facet edges, lexical fallback, connected-component bucketing, deterministic component splitting). It is a large-survey search-space optimisation with no current consumer, and its headline input — per-lead `blocking-keys[]` survey metadata — is not produced by the shipped M1 `survey` (`lead.schema.json`). Revisit only if agents demonstrably choke on a large flat catalog; reintroducing it requires no response-side change, only additive request fields.

Every surveyed lead must appear exactly once across `concepts[].members[]`. Missing or duplicate members fail with `plan-reconcile-partition`; a `(source-key, lead-id)` pair that names no request catalog row fails with `plan-reconcile-lead-orphan`. Every `slices[].concept-id` must name a declared concept (`plan-reconcile-concept-orphan`), and every concept must have at least one slice (`plan-reconcile-concept-unbound`).

## Envelope

The request and response are both validated by `schemas/discovery/proposal.schema.json`, embedded as `PROPOSAL_JSON_SCHEMA`, with a closed `kind: request | response` discriminator.

The request is lead-centric: a flat `leads[]` catalog carries one row per raw `(source-key, lead-id)` lead, and each row that the kernel proved is the same work across sources names its locked group via an optional `group` block. The flat `leads[]` list is canonical and the sole source of group membership: per-row `group` records both the group assignment (`group.group-id`) and the locked-match proof (`group.match-basis`), and a group's members are read by filtering `leads[]` on `group.group-id`. Rows without `group` are open leads for the agent to place.

The kernel reads each raw, unmerged lead from `discovery.md` 1:1 into one catalog row — there is no expansion or cross-source merge at survey or request time, so each source's per-source summary and match basis survives into reconciliation intact. The `lead-id` field is the discovery lead id; it is **not** globally unique across `leads[]` when multiple sources surface the same id (for example `identity-api` from both `docs` and `legacy`). Identity is the `(source-key, lead-id)` pair. The envelope and `plan.yaml` slice bindings name the same `{ source-key, lead-id }` shape.

Example request:

```yaml
version: 1
kind: request
targets: [contracts@v1, omnia@v1]
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
    group: { group-id: identity-api, match-basis: exact-id }
  - source-key: legacy
    lead-id: identity-api
    summary: "Legacy identity endpoints."
    group: { group-id: identity-api, match-basis: exact-id }
  - source-key: docs
    lead-id: password-reset
    summary: "Users can request a password reset email."
  - source-key: legacy
    lead-id: reset-password
    summary: "Legacy reset-password flow."
```

`projects[]` lists every project the agent may bind a slice to and always carries at least one entry (a single regular project is synthesized from `project.yaml`). Each catalog row carries a `source-key` binding, a discovery `lead-id`, the per-source surveyed `summary`, an optional `aliases[]` list, and optionally a `group` block:

- `group.group-id` is the canonical lead id of the locked group this row belongs to. The kernel reads a group's members by filtering `leads[]` on this value.
- `group.match-basis` records this row's proof basis (`exact-id` or `exact-alias`; per-row, so one locked group may mix bases).

A row carries `group` or it does not — the two states being **locked** (kernel-proven, must not be split) and **open** (the agent places it by judgment). The open residue replaces the old explicit `unmatched-leads[]` array.

Example response:

```yaml
version: 1
kind: response
concepts:
  - concept-id: identity-api
    members:
      - { source-key: docs, lead-id: identity-api }
      - { source-key: legacy, lead-id: identity-api }
      - { source-key: docs, lead-id: password-reset, match-basis: semantic }
      - { source-key: legacy, lead-id: reset-password, match-basis: semantic }
    rationale: "identity API plus semantic password reset merge"
slices:
  - concept-id: identity-api
    name: identity-contracts
    target: contracts@v1
    project: identity-contracts
  - concept-id: identity-api
    name: identity-service
    target: omnia@v1
    project: identity-service
    depends-on: [identity-contracts]
```

`concepts[]` is the partition of the surveyed leads. Each member references a catalog row by `{ source-key, lead-id }`. A member of a locked group omits `match-basis` — the kernel carries its proven `exact-id` / `exact-alias` basis forward during projection, so the agent can neither forge nor downgrade a locked match; the agent sets `match-basis: semantic` only on open leads it merged by judgment. `slices[]` is the plan-row projection surface: each slice names a `concept-id`, target, optional project, optional explicit `name`, and optional dependencies. A `concept-id` may appear in multiple slices when the same concept fans out to multiple targets, but the members are declared once under `concepts[]`. `depends-on` names derived slice names, not concept ids. The kernel projects each concept's members into `plan.yaml.slices[].sources[]` as `{ source-key: <source-key>, lead-id: <lead-id> }`, and writes each slice's `target` verbatim. Target-adapter resolvability is enforced by the existing plan writers (and by the project↔target equality check below); the reconciliation kernel adds no separate target-membership code.

## Slice Names

Each `slices[]` entry becomes one `plan.yaml.slices[]` entry. The kernel assigns its name as follows:

1. If the agent provides `name`, validate and use it.
2. Else if `concept-id` is not already used as a slice name in this response, use `concept-id`.
3. Else use `<concept-id>-<adapter-slug>`, where `<adapter-slug>` is the segment before `@v` in `target` (`contracts@v1` -> `contracts`).

After deriving all names, the kernel validates every `depends-on` entry against that name set.

Because `depends-on` resolves against names derived only after submission, set an explicit `name` on any slice another slice depends on. An explicit name pins the reference regardless of derivation order and avoids a dangling `depends-on` when rule 3 derives a `<concept-id>-<adapter-slug>` fallback the agent did not anticipate.

## Project Binding

Every slice resolves to exactly one project. The dry-run request always carries `projects[]` as `{ name, target, description }` — for a hub it is the validated registry's projects; for a single regular project the CLI synthesizes one entry from `project.yaml` (`name`, the project's target adapter, and `domain` as the description). There is no separate single-repo path: a lone project is just a `projects[]` of length one.

The agent binds a `project` on each slice by matching the concept, target, and project descriptions. As a convenience, when `projects[]` has exactly one entry the agent may omit `project` and the kernel auto-binds the sole project — binding the only candidate is not a judgment. When `projects[]` offers more than one project the agent must name one explicitly; the CLI never chooses among multiple candidates.

Before writing, the kernel enforces:

1. A slice may omit `project` only when exactly one project exists (the kernel then auto-binds it); when more than one project is offered, every slice must name one. Failure: `plan-reconcile-project-binding-required`.
2. The named (or auto-bound) project must exist in `projects[]`. Failure: `plan-reconcile-project-orphan`.
3. The project's `target` must equal the slice's `target`. Failure: `plan-reconcile-project-target-mismatch`.

The resolved value is written verbatim to `plan.yaml.slices[].project`. Build-time routing resolves a hub project against `registry.yaml` as described by [RFC-29c §"Per-slice fan-out (D5)"](rfc-29c-synthesis.md); a synthesized single project names the repo itself.

## Match Basis And Review

A reconciled member resolves to one of three match bases, split by who authors it:

- `exact-id`, `exact-alias` — kernel-computed locked-group proof. The agent omits `match-basis` on these members; the kernel carries the proven basis forward from the request `group` block during projection.
- `semantic` — the agent's cross-source judgment over open leads, authored explicitly on the member.

`semantic` matches are review signals. The agent renders them into `change.md` for Gate 1 so the operator can accept the grouping, split it, or promote recurring semantic matches into aliases with `specrun plan amend --add-alias` (which makes them locked on the next survey).

The kernel does not decide whether a semantic match is correct. It only proves that the response is well-formed, partitions the lead set, preserves locked groups, and binds valid targets/projects.

## Agent Responsibilities

During `/spec:plan`, the agent:

1. Calls `specrun plan propose --dry-run --format json`.
2. Groups the open leads (rows without `group`) by judgment from their `summary`, optionally merging them into the concept that holds a related locked group — without splitting that locked group across concepts.
3. Binds each concept to one or more targets, producing one slice per `(concept-id, target)` pair.
4. Adds `rationale`, `depends-on`, and optional `name`.
5. Binds every slice to a compatible `project` — or omits it when exactly one project exists, letting the kernel auto-bind.
6. Calls `specrun plan propose --from <response.json>`.
7. Renders semantic matches into `change.md` for Gate 1 review.

The agent never writes `plan.yaml`, never writes `discovery.md`, and never decides authority.

## Wire Contracts

The canonical closed tables live in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This milestone appends:

- **Journal events:** `plan.reconcile.agent`, `plan.reconcile.completed`.
- **Operational validation codes:** `plan-reconcile-lead-orphan`, `plan-reconcile-partition`, `plan-reconcile-locked-group-split`, `plan-reconcile-concept-orphan`, `plan-reconcile-concept-unbound`, `plan-reconcile-project-binding-required`, `plan-reconcile-project-orphan`, `plan-reconcile-project-target-mismatch`, `plan-propose-missing-grouping`. These are `Error::Validation` outcomes and abort with exit 2. The `plan-reconcile-*` codes name response-invariant failures; `plan-propose-missing-grouping` carries the `plan-propose-` prefix because it guards command-argument selection (neither `--dry-run` nor `--from`), not a reconciliation invariant.
- **Schema:** `schemas/discovery/proposal.schema.json` (`PROPOSAL_JSON_SCHEMA`), covering request and response envelopes.

## Resolved Question

Optional lead target-axis hints are deferred to a follow-on RFC. M2a ships pure agent target binding wrapped by the deterministic kernel above.
