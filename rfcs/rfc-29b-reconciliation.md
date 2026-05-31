# RFC-29b: Plan-Time Lead Reconciliation

> Status: Draft — Milestone **M2a** of [RFC-29](rfc-29-fan-in-fan-out.md) — Depends: [RFC-29a M1 (shipped)](rfc-29-fan-in-fan-out.md#sub-rfcs-and-milestone-ordering) (surveyed `discovery.md`) — Unblocks: the M2b plan rows in [RFC-29c](rfc-29c-synthesis.md)

This milestone closes plan-time fan-in. It turns the surveyed `Lead[]` from multiple sources into the `plan.yaml.slices[]` rows that `/spec:execute` will later run.

The important split is simple:

- The **agent** decides which leads describe the same work, which projects own each slice, and how scopes fan out across the project topology.
- The **CLI** supplies deterministic locked groups, validates the agent's response, derives slice names, emits journal events, and writes the plan. The agent never hand-edits `plan.yaml`.

Three nouns carry the milestone; keep them distinct:

- **group** — a kernel-computed set of `(source-key, lead-id)` rows proved to be the same work by exact id or alias. Deterministic, enforced, never split.
- **scope** — the agent's unit of work. `scopes[]` partition the surveyed leads exactly once; a scope may absorb several locked groups plus open leads merged by judgment.
- **slice** — one `plan.yaml.slices[]` row, produced by binding a `(scope, project)` pair. One scope fans out to multiple slices across projects (each project's `target` is written to `plan.yaml` at projection time).

The shared wire contracts this milestone extends are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This document is the source of truth for D2.

## Decision


| ID                         | Decision                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **D2 Lead reconciliation** | `specrun plan propose` wraps agent-led lead reconciliation in a CLI-owned projection kernel. **The agent** partitions surveyed leads into `scopes[]`, then emits one or more `slices[]` rows per scope by binding each `(scope-id, project)` pair to one project from the request's `projects[]`. **The kernel** computes deterministic locked groups (exact id / alias), validates the response schema, enforces the global lead partition, prevents locked groups from being split, carries each locked member's proven `match-basis` forward, resolves each slice's `target` from the bound project's entry in `projects[]`, derives slice names, emits journal events, and writes `plan.yaml.slices[]`. |


## Operator Surface

```bash
specrun plan propose --dry-run --format json
specrun plan propose --from <response.json> [--format json]
```

`--dry-run` writes nothing. It reads `plan.yaml.sources`, the surveyed `discovery.md` lead inventory, operator-authored aliases, and the project topology (`registry.yaml` for a hub, or the sole project synthesized from `project.yaml`). It returns a request envelope for the agent.

`--from` is the only writer. On every invocation it **re-reads** `discovery.md`, **recomputes** locked groups and the lead catalog (it does not trust a prior `--dry-run` snapshot), validates the agent response against that fresh catalog, **replaces** all `plan.yaml.slices[]` rows on a pending plan, derives slice names, emits reconciliation events, and writes slices through the existing `crates/workflow/src/change/plan/` writers. Manual `specrun plan add` remains available for headless authoring; the default `/spec:plan` flow uses `propose --from` projecting through the same writers.

## Reconciliation Flow

1. `/spec:plan` runs `specrun plan propose --dry-run --format json`.
2. The CLI returns a flat `leads[]` catalog — one row per raw `(source-key, lead-id)` lead read 1:1 from `discovery.md`, each optionally carrying a `group` block when the kernel proved it is the same work across sources (rows without one being open leads) — plus `projects[]` (each entry carries the target adapter the agent may bind).
3. The agent groups the open leads by judgment but returns one global `scopes[]` partition plus `slices[]`. Scopes partition the surveyed leads; slices fan those scopes out to projects (the kernel derives each row's `target` from the bound project).
4. `/spec:plan` submits that response with `specrun plan propose --from <response.json>`.
5. The CLI re-reads `discovery.md`, recomputes the catalog and locked groups, validates the response against that fresh state, and **replaces** all `plan.yaml.slices[]` rows (idempotent re-propose on a pending plan).
6. The agent renders match review signals into `change.md` so Gate 1 can review them.

The agent owns judgment. The CLI owns projection and persistence.

## Locked Groups

Before the agent sees the inventory, the kernel computes conservative locked groups that may not be split. Membership uses two proof bases recorded per row:

1. **`exact-id`** — this row's `lead-id` equals a token shared with a lead from a different `source-key`.
2. **`exact-alias`** — an entry in this row's `aliases[]` equals a token shared with a lead from a different `source-key` (including when the other lead's `lead-id` is that token).

### Locked-group computation

The algorithm is deterministic and runs over the flat lead catalog read 1:1 from `discovery.md`:

1. For each catalog row, build a **token set** `{lead-id} ∪ aliases[]`.
2. Connect two rows when they have **different** `source-key` values and their token sets intersect. Union-find merges connected components transitively (A↔B and B↔C ⇒ one group).
3. A component **locks** only when it spans two or more distinct `source-key` values; single-source rows carry no `group` block.
4. Set **`group-id`** to the lexicographically smallest `lead-id` among the component's members (always a member's canonical id, never a bare alias).
5. For each locked row, set **`match-basis`**: `exact-id` when this row's `lead-id` is in the intersection token that linked it to a different source; otherwise `exact-alias`.

Alias example (same request envelope as below, with one extra locked pair):

```yaml
  - source-key: docs
    lead-id: user-registration
    summary: "Registration endpoint from design notes."
    group: { group-id: user-registration, match-basis: exact-id }
  - source-key: legacy
    lead-id: account-registration
    summary: "POST /users registration handler."
    aliases: [user-registration]
    group: { group-id: user-registration, match-basis: exact-alias }
```

Here `docs:user-registration` and `legacy:account-registration` share token `user-registration`; `group-id` is `user-registration` (smallest member `lead-id`); the legacy row carries `exact-alias` because the cross-source link is through its alias list.

A locked group is a pure function of `discovery.md` and is defined implicitly by the catalog rows that share a `group.group-id`; there is no separate group index and no kernel-maintained reverse membership list. The agent may extend a locked group with semantically matched open leads inside a single scope, but `propose --from` rejects any response that splits a locked group across scopes with `plan-reconcile-locked-group-split`.

Open leads — those the kernel could not prove — carry no `group` block. They are returned as a flat catalog and grouped entirely by the agent's judgment from their per-source `summary`; the kernel does no heuristic clustering of open leads, surfaces no facets, and never auto-merges them. Semantic groupings the agent makes are surfaced for operator review at Gate 1 (`match-basis: semantic`).

> Deferred (YAGNI): kernel-side advisory clustering of open leads (facet edges, lexical fallback, connected-component bucketing, deterministic component splitting). It is a large-survey search-space optimisation with no current consumer, and its headline input — per-lead `blocking-keys[]` survey metadata — is not produced by the shipped M1 `survey` (`lead.schema.json`). Revisit only if agents demonstrably choke on a large flat catalog; reintroducing it requires no response-side change, only additive request fields.

Every surveyed lead must appear exactly once across `scopes[].members[]`. Missing or duplicate members fail with `plan-reconcile-partition`; a `(source-key, lead-id)` pair that names no **current** catalog row fails with `plan-reconcile-lead-orphan`. Every `slices[].scope-id` must name a declared scope (`plan-reconcile-scope-orphan`), and every scope must have at least one slice (`plan-reconcile-scope-unbound`). After project auto-bind, duplicate `(scope-id, project)` pairs fail with `plan-reconcile-slice-duplicate`. After slice-name derivation, colliding derived names fail with `plan-reconcile-slice-name-collision`. A cyclic `depends-on` graph fails with `plan-reconcile-depends-on-cycle` (same detection as `specrun plan validate`'s `cycle-in-depends-on`, raised as a single propose abort).

## Envelope

The request and response are both validated by `schemas/discovery/proposal.schema.json`, embedded as `PROPOSAL_JSON_SCHEMA`, with a closed `kind: request | response` discriminator.

The request is lead-centric: a flat `leads[]` catalog carries one row per raw `(source-key, lead-id)` lead, and each row that the kernel proved is the same work across sources names its locked group via an optional `group` block. The flat `leads[]` list is canonical and the sole source of group membership: per-row `group` records both the group assignment (`group.group-id`) and the locked-match proof (`group.match-basis`), and a group's members are read by filtering `leads[]` on `group.group-id`. Rows without `group` are open leads for the agent to place.

The kernel reads each raw, unmerged lead from `discovery.md` 1:1 into one catalog row — there is no expansion or cross-source merge at survey or request time, so each source's per-source summary and match basis survives into reconciliation intact. The `lead-id` field is the discovery lead id; it is **not** globally unique across `leads[]` when multiple sources surface the same id (for example `identity-api` from both `docs` and `legacy`). Identity is the `(source-key, lead-id)` pair. The envelope and `plan.yaml` slice bindings name the same `{ source-key, lead-id }` shape.

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

`projects[]` lists every project the agent may bind a slice to and always carries at least one entry (a single regular project is synthesized from `project.yaml`). Each project entry carries its `target` adapter; there is no separate request-level `targets[]` — available targets are read from `projects[].target`. Each catalog row carries a `source-key` binding, a discovery `lead-id`, the per-source surveyed `summary`, an optional `aliases[]` list, and optionally a `group` block:

- `group.group-id` is the canonical lead id of the locked group this row belongs to. The kernel reads a group's members by filtering `leads[]` on this value.
- `group.match-basis` records this row's proof basis (`exact-id` or `exact-alias`; per-row, so one locked group may mix bases).

A row carries `group` or it does not — the two states being **locked** (kernel-proven, must not be split) and **open** (the agent places it by judgment). The open residue replaces the old explicit `unmatched-leads[]` array.

### N=1 degenerate example

Pure-intent changes use the same envelope; locked groups are absent and the kernel auto-binds the sole project:

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
      - { source-key: docs, lead-id: password-reset, match-basis: semantic }
      - { source-key: legacy, lead-id: reset-password, match-basis: semantic }
    rationale: "identity API plus semantic password reset merge"
slices:
  - scope-id: identity-api
    name: identity-contracts
    project: identity-contracts
  - scope-id: identity-api
    name: identity-service
    project: identity-service
    depends-on: [identity-contracts]
```

`scopes[]` is the partition of the surveyed leads. Each member references a catalog row by `{ source-key, lead-id }`. A member of a locked group omits `match-basis` — the kernel carries its proven `exact-id` / `exact-alias` basis forward during projection, so the agent can neither forge nor downgrade a locked match; the agent sets `match-basis: semantic` only on open leads it merged by judgment. `slices[]` is the plan-row projection surface: each slice names a `scope-id`, optional `project`, optional explicit `name`, and optional dependencies. There is no response-level `target` — the kernel resolves each slice's `target` from the bound project's `projects[].target` entry. A `scope-id` may appear in multiple slices when the same scope fans out to multiple projects, but the members are declared once under `scopes[]`. `depends-on` names derived slice names, not scope ids. The kernel projects each scope's members into `plan.yaml.slices[].sources[]` as `{ source-key: <source-key>, lead-id: <lead-id> }`, writes each slice's `project`, and writes each slice's `target` from the matching `projects[]` entry. Target-adapter resolvability is enforced by the existing plan writers; the reconciliation kernel adds no separate target-membership code.

## Slice Names

Each `slices[]` entry becomes one `plan.yaml.slices[]` entry. The kernel assigns its name as follows:

1. If the agent provides `name`, validate and use it.
2. Else if `scope-id` is not already used as a slice name in this response, use `scope-id`.
3. Else use `<scope-id>-<adapter-slug>`, where `<adapter-slug>` is the segment before `@v` in the slice's resolved `target` (derived from the bound project's `projects[].target`; `contracts@v1` -> `contracts`).

After deriving all names, the kernel validates every `depends-on` entry against that name set and rejects cyclic graphs with `plan-reconcile-depends-on-cycle`.

Because `depends-on` resolves against names derived only after submission, set an explicit `name` on any slice another slice depends on. An explicit name pins the reference regardless of derivation order and avoids a dangling `depends-on` when rule 3 derives a `<scope-id>-<adapter-slug>` fallback the agent did not anticipate.

## Project Binding

Every slice resolves to exactly one project. The dry-run request always carries `projects[]` as `{ name, target, description }` — for a hub it is the validated registry's projects; for a single regular project the CLI synthesizes one entry from `project.yaml` (`name`, the project's target adapter, and `domain` as the description). There is no separate single-repo path: a lone project is just a `projects[]` of length one.

The agent binds a `project` on each slice by matching the scope against each project's `target` and `description`. As a convenience, when `projects[]` has exactly one entry the agent may omit `project` and the kernel auto-binds the sole project — binding the only candidate is not a judgment. When `projects[]` offers more than one project the agent must name one explicitly; the CLI never chooses among multiple candidates.

Before writing, the kernel enforces:

1. A slice may omit `project` only when exactly one project exists (the kernel then auto-binds it); when more than one project is offered, every slice must name one. Failure: `plan-reconcile-project-binding-required`.
2. The named (or auto-bound) project must exist in `projects[]`. Failure: `plan-reconcile-project-orphan`.

The kernel writes the resolved `project` verbatim to `plan.yaml.slices[].project` and writes `plan.yaml.slices[].target` from that project's `projects[].target` entry. Build-time routing resolves a hub project against `registry.yaml` as described by [RFC-29c §"Per-slice fan-out (D5)"](rfc-29c-synthesis.md); a synthesized single project names the repo itself.

## Match Basis And Review

A reconciled member resolves to one of three match bases, split by who authors it:

- `exact-id`, `exact-alias` — kernel-computed locked-group proof. The agent omits `match-basis` on these members; the kernel carries the proven basis forward from the request `group` block during projection.
- `semantic` — the agent's cross-source judgment over open leads, authored explicitly on the member.

**Persistence:** `match-basis` is **not** written to `plan.yaml`. It is a Gate 1 review signal only — the agent renders locked and semantic pairings into `change.md`; the kernel may optionally echo summary counts in the `plan.reconcile.completed` journal payload. `plan.yaml.slices[].sources[]` names `{ source-key, lead-id }` only.

`semantic` matches are review signals. The agent renders them into `change.md` for Gate 1 so the operator can accept the grouping, split it, or promote recurring semantic matches into aliases with `specrun plan amend --add-alias` (which makes them locked on the next survey).

The kernel does not decide whether a semantic match is correct. It only proves that the response is well-formed, partitions the lead set, preserves locked groups, and binds valid projects.

## Out Of Kernel Scope

The following plan-time signals stay in the **skill layer** and existing CLI amend paths; the reconciliation kernel does not author or persist them:

- **`## Tentative merges` in `change.md`** — when the agent is uncertain a semantic grouping is correct, it records reasoning there. The agent never edits `discovery.md` (no `tentative: true` bullets on lead blocks).
- **`## Likely divergences` in `change.md`** — when merged leads' per-source `summary` strings materially disagree, the agent records side-by-side summaries there after `propose --from` succeeds.
- **`plan.yaml.slices[].divergence: likely`** — written only by `specrun plan amend <plan> <slice> --divergence likely` (fires `plan.amend.divergence`); the agent invokes amend after propose when summaries disagree materially.

## Agent Responsibilities

During `/spec:plan`, the agent:

1. Calls `specrun plan propose --dry-run --format json`.
2. Groups the open leads (rows without `group`) by judgment from their `summary`, optionally merging them into the scope that holds a related locked group — without splitting that locked group across scopes.
3. Binds each scope to one or more projects, producing one slice per `(scope-id, project)` pair (or omits `project` when exactly one exists, letting the kernel auto-bind).
4. Adds `rationale`, `depends-on`, and optional `name`.
5. Ensures every slice resolves to a project — explicitly named, or auto-bound when only one project exists.
6. Calls `specrun plan propose --from <response.json>`.
7. Renders match review signals into `change.md` for Gate 1 (`## Tentative merges` for uncertain semantic groupings; `## Likely divergences` when per-source summaries materially disagree).
8. When summaries materially disagree on a merged slice, invokes `specrun plan amend <plan> <slice> --divergence likely` so the CLI stamps `slices[].divergence` (see §"Out of kernel scope").

The agent never writes `plan.yaml`, never writes `discovery.md`, and never decides authority.

## Wire Contracts

The canonical closed tables live in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This milestone appends:

- **Journal events:** `plan.reconcile.agent`, `plan.reconcile.completed`.
- **Operational validation codes:** `plan-reconcile-lead-orphan`, `plan-reconcile-partition`, `plan-reconcile-locked-group-split`, `plan-reconcile-scope-orphan`, `plan-reconcile-scope-unbound`, `plan-reconcile-slice-duplicate`, `plan-reconcile-slice-name-collision`, `plan-reconcile-depends-on-cycle`, `plan-reconcile-project-binding-required`, `plan-reconcile-project-orphan`, `plan-propose-missing-grouping`. These are `Error::Validation` outcomes and abort with exit 2. The `plan-reconcile-*` codes name response-invariant failures; `plan-propose-missing-grouping` carries the `plan-propose-` prefix because it guards command-argument selection (neither `--dry-run` nor `--from`), not a reconciliation invariant.
- **Schema:** `schemas/discovery/proposal.schema.json` (`PROPOSAL_JSON_SCHEMA`), covering request and response envelopes.

## Resolved Question

Optional lead target-axis hints are deferred to a follow-on RFC. M2a ships pure agent project binding wrapped by the deterministic kernel above; `target` is always kernel-derived from the bound project.
