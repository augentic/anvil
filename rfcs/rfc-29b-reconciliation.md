# RFC-29b: Plan-Time Lead Reconciliation

> Status: Draft — Milestone **M2a** of [RFC-29](rfc-29-fan-in-fan-out.md) — Depends: [RFC-29a M1 (shipped)](rfc-29-fan-in-fan-out.md#sub-rfcs-and-milestone-ordering) (surveyed `discovery.md`) — Unblocks: the M2b plan rows in [RFC-29c](rfc-29c-synthesis.md)

This milestone closes plan-time fan-in. It turns the surveyed `Lead[]` from multiple sources into the `plan.yaml.slices[]` rows that `/spec:execute` will later run.

The important split is simple:

- The **agent** decides which leads describe the same work, which target slices to emit, and, in workspace mode, which registry project owns each slice.
- The **CLI** supplies deterministic locked clusters and advisory clusters, validates the agent's response, derives slice names, emits journal events, and writes the plan. The agent never hand-edits `plan.yaml`.

The shared wire contracts this milestone extends are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This document is the source of truth for D2.

## Decision


| ID                         | Decision                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **D2 Lead reconciliation** | `specrun plan propose` wraps agent-led lead reconciliation in a CLI-owned projection kernel. The agent partitions surveyed leads into `concepts[]`, then emits one or more `slices[]` rows per concept by binding each `(concept-id, target)` pair to one target and, in workspace mode, one registry project. The kernel computes locked clusters and advisory clusters, tags each lead with at most one cluster, records cluster adjacency in `clusters[]`, validates the response schema, enforces the global lead partition, prevents locked clusters from being split, validates project bindings, derives slice names, emits journal events, and writes `plan.yaml.slices[]`. |


## Operator Surface

```bash
specrun plan propose --dry-run --format json
specrun plan propose --from <response.json> [--format json]
```

`--dry-run` writes nothing. It reads `plan.yaml.sources`, the surveyed `discovery.md` lead inventory, operator-authored aliases, targets, and, in workspace mode, `registry.yaml`. It returns a request envelope for the agent.

`--from` is the only writer. It validates the agent response, enforces the invariants below, derives slice names, emits reconciliation events, and writes slices through the existing `crates/workflow/src/change/plan/` writers.

## Reconciliation Flow

1. `/spec:plan` runs `specrun plan propose --dry-run --format json`.
2. The CLI returns a flat `leads[]` catalog — one row per raw `(source-key, lead-id)` lead read 1:1 from `discovery.md`, each optionally carrying a `cluster` block (rows without one being unmatched) — plus the `clusters[]` adjacency index, `targets[]`, and, in workspace mode, `projects[]`.
3. The agent works cluster-by-cluster but returns one global `concepts[]` partition plus `slices[]`. Concepts partition the surveyed leads; slices fan those concepts out to targets and projects.
4. `/spec:plan` submits that response with `specrun plan propose --from <response.json>`.
5. The CLI validates and writes `plan.yaml.slices[]`.
6. The agent renders semantic and tentative matches into `change.md` so Gate 1 can review them.

The agent owns judgment. The CLI owns projection and persistence.

## Reconciliation Clusters

Before the agent sees the inventory, the kernel computes conservative locked clusters that may not be split. It then binds each member lead with a `cluster` block:

1. Exact `lead-id` match across source keys — the same `lead-id` surfaced by more than one source binding.
2. Exact alias match across source keys, recorded under the canonical `lead-id`.

(A former third basis, "transitive cross-reference through `Lead.sources[]`", is dropped: leads are now raw and per-source, so a lead carries a single `source-key` rather than a `sources[]` list, and cross-source identity is subsumed by basis 1.)

Locked clusters are a pure function of `discovery.md`. The agent may extend a locked cluster with semantic members, but `propose --from` rejects any response that splits a locked cluster with `plan-reconcile-required-group-split`.

The remaining open leads are not returned as one flat matching problem. The kernel partitions them into advisory clusters and tags each open lead with its cluster id so large surveys stay reviewable. A lead is only ever a member of one cluster:

1. Treat every locked cluster and every open lead as a node.
2. Attach deterministic grouping facets from surveyed lead metadata (`blocking-keys[]`) when present.
3. Add edges for shared facets. When an open lead shares a facet with a locked-cluster member, the kernel does not pull the locked cluster into the advisory cluster; instead the advisory cluster records that locked cluster under `clusters[].adjacent-clusters[]`. This is an invitation to consider extending the locked cluster, not a match decision.
4. For leads without survey facets, use a fixed lexical fallback over normalized `id`, `aliases[]`, and `summary` tokens (`token:<term>` facets).
5. Each connected component becomes one advisory cluster id stamped onto its open-lead members only; any locked cluster in the component is recorded as an `adjacent-clusters[]` entry rather than tagged. Open leads with no useful edge are left without `cluster`, marking them unmatched.
6. If a component exceeds the implementation limit, split it deterministically by strongest available facet in this order: `domain:*`, `module:*`, `entity:*`, `route:*`, then lexical `token:*`.

Advisory clusters reduce the agent's search space; they do not decide semantic matching, do not constrain the final response, and do not relax the global partition check. The request exposes the cluster-level facet union as `clusters[].facets[]`; per-lead `blocking-keys[]` remain surveyed `discovery.md` metadata and are not repeated on every request lead.

Every surveyed lead must appear exactly once across `concepts[].members[]`. Missing or duplicate members fail with `plan-reconcile-partition`; a `(source-key, lead-id)` pair that names no request catalog row fails with `plan-reconcile-lead-orphan`. Every `slices[].concept-id` must name a declared concept (`plan-reconcile-concept-orphan`), and every concept must have at least one slice (`plan-reconcile-concept-unbound`).

## Envelope

The request and response are both validated by `schemas/discovery/proposal.schema.json`, embedded as `PROPOSAL_JSON_SCHEMA`, with a closed `kind: request | response` discriminator.

The request is lead-centric: a flat `leads[]` catalog carries one row per raw `(source-key, lead-id)` lead, and each row names at most one reconciliation cluster via optional `cluster`. A small top-level `clusters[]` index records whether a cluster is locked or advisory, the `facets[]` union across bound catalog rows, and the one cross-cutting fact a single row binding cannot: which locked clusters an advisory cluster sits adjacent to. The flat `leads[]` list stays canonical and is the sole source of cluster membership: per-row `cluster` records both cluster assignment and locked-match proof, and the agent reads a cluster's members by filtering `leads[]` on `cluster.cluster-id`. The kernel emits no reverse membership index.

The kernel reads each raw, unmerged lead from `discovery.md` 1:1 into one catalog row — there is no expansion or cross-source merge at survey or request time, so each source's per-source summary and match basis survives into reconciliation intact. The `lead-id` field is the discovery lead id; it is **not** globally unique across `leads[]` when multiple sources surface the same id (for example `identity-api` from both `docs` and `legacy`). Identity is the `(source-key, lead-id)` pair. The envelope and `plan.yaml` slice bindings name the same `{ source-key, lead-id }` shape.

Example request:

```yaml
version: 1
kind: request
mode: workspace                       # workspace | single-repo
targets: [contracts@v1, omnia@v1]
projects:
  - name: identity-contracts
    target: contracts@v1
    description: "Versioned API contracts crate for the identity domain."
  - name: identity-service
    target: omnia@v1
    description: "Omnia identity service implementing auth and password flows."
clusters:
  - cluster-id: identity-api
    enforcement: locked
    facets: [domain:identity, module:auth]
  - cluster-id: identity
    enforcement: advisory
    facets: [domain:identity, module:auth, capability:password-reset]
    adjacent-clusters: [identity-api]
leads:
  - source-key: docs
    lead-id: identity-api
    summary: "Identity API contract for authentication and account access."
    cluster: { cluster-id: identity-api, match-basis: exact-id }
  - source-key: legacy
    lead-id: identity-api
    summary: "Legacy identity endpoints."
    cluster: { cluster-id: identity-api, match-basis: exact-id }
  - source-key: docs
    lead-id: password-reset
    summary: "Users can request a password reset email."
    cluster: { cluster-id: identity }
  - source-key: legacy
    lead-id: reset-password
    summary: "Legacy reset-password flow."
    cluster: { cluster-id: identity }
```

Each catalog row carries a `source-key` binding, a discovery `lead-id`, the per-source surveyed `summary`, and optionally a `cluster` block:

- `cluster.cluster-id` names the reconciliation cluster this catalog row is bound to.
- `cluster.match-basis` is present when the kernel proved the row belongs to a locked cluster. It records this row's match basis (`exact-id` or `exact-alias`; the basis is per-row, so one locked cluster may mix bases).

`projects[]` is present only in workspace mode.

### Cluster

A catalog row carries `cluster` or no `cluster` — the three states being **settled** (locked cluster), **candidate** (advisory cluster), and **unmatched** (no cluster). The unmatched residue replaces the old explicit `unmatched-leads[]` array.

The top-level `clusters[]` index is the bridge between settled and candidate states: locked clusters carry `enforcement: locked`; advisory clusters carry `enforcement: advisory`.

`facets[]` is the cluster-level union of grouping facets (`domain:*`, `module:*`, …) shared by member leads. On advisory clusters, facets explain how the kernel bucketed open leads; on locked clusters, facets are annotative (the union of member survey metadata, not how locking worked). Lexical fallback facets use the `token:<term>` namespace.

A cluster carries no membership list of its own. Cluster membership lives solely on per-row `cluster.cluster-id`; the agent reads a cluster's catalog rows by filtering `leads[]` on that field. The kernel emits no reverse index.

When an advisory cluster sits adjacent to one or more locked clusters (an open-lead member shares a facet with a locked-cluster member), it lists them under `adjacent-clusters[]`. That is the kernel's invitation to consider extending those locked clusters with the advisory cluster's open leads. `mode` declares the reconciliation mode.

Example response:

```yaml
version: 1
kind: response
concepts:
  - concept-id: identity-api
    cluster-id: identity
    members:
      - { source-key: docs, lead-id: identity-api, match-basis: exact-id }
      - { source-key: legacy, lead-id: identity-api, match-basis: exact-id }
      - { source-key: docs, lead-id: password-reset, match-basis: semantic }
      - { source-key: legacy, lead-id: reset-password, match-basis: semantic, tentative: true }
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

`concepts[]` is the partition of the surveyed leads. Each member references a catalog row by `{ source-key, lead-id }` and carries the agent's per-member `match-basis` (and optional `tentative`). `cluster-id` is optional review traceability back to the request's advisory cluster; it carries no validation authority over membership. `slices[]` is the plan-row projection surface: each slice names a `concept-id`, target, optional project, optional explicit `name`, and optional dependencies. A `concept-id` may appear in multiple slices when the same concept fans out to multiple targets, but the members are declared once under `concepts[]`. `depends-on` names derived slice names, not concept ids. The kernel projects each concept's members into `plan.yaml.slices[].sources[]` as `{ source-key: <source-key>, lead-id: <lead-id> }`.

## Slice Names

Each `slices[]` entry becomes one `plan.yaml.slices[]` entry. The kernel assigns its name as follows:

1. If the agent provides `name`, validate and use it.
2. Else if `concept-id` is not already used as a slice name in this response, use `concept-id`.
3. Else use `<concept-id>-<adapter-slug>`, where `<adapter-slug>` is the segment before `@v` in `target` (`contracts@v1` -> `contracts`).

After deriving all names, the kernel validates every `depends-on` entry against that name set.

## Project Binding

Workspace mode adds one more agent decision: which registry project owns each slice.

In workspace mode, the dry-run request includes `projects[]` from the validated registry as `{ name, target, description }`. The agent must choose one project for every slice, using the concept, target, and registry descriptions. The CLI does not choose a project, even when only one registry project matches the target.

Before writing, the kernel enforces:

1. A workspace request requires `project` on every slice; a single-repo request forbids it. Failure: `plan-reconcile-project-binding-required`.
2. The named project must exist in the registry. Failure: `plan-reconcile-project-orphan`.
3. The project's registered `target` must equal the slice's `target`. Failure: `plan-reconcile-project-target-mismatch`.

The validated value is written verbatim to `plan.yaml.slices[].project`. Build-time workspace routing resolves it against `registry.yaml` as described by [RFC-29c §"Per-slice fan-out (D5)"](rfc-29c-synthesis.md).

## Match Basis And Review

Each `concepts[].members[]` entry carries a `match-basis`:

- `exact-id`
- `exact-alias`
- `semantic`

`semantic` and `tentative: true` are review signals. The agent renders them into `change.md` for Gate 1 so the operator can accept the grouping, split it, or promote recurring semantic matches into aliases with `specrun plan amend --add-alias`.

The kernel does not decide whether a semantic match is correct. It only proves that the response is well-formed, partitions the lead set, preserves locked clusters, and binds valid targets/projects.

## Agent Responsibilities

During `/spec:plan`, the agent:

1. Calls `specrun plan propose --dry-run --format json`.
2. Walks the open leads by advisory cluster, grouping them semantically and, where a cluster names `adjacent-clusters[]`, possibly extending those locked clusters without splitting them; rows without `cluster` are the unmatched residue to place.
3. Binds each concept to one or more targets, producing one slice per `(concept-id, target)` pair.
4. Adds `rationale`, `tentative` flags, `depends-on`, and optional `name`.
5. In workspace mode, binds every slice to a compatible `project`; in single-repo mode, omits `project`.
6. Calls `specrun plan propose --from <response.json>`.
7. Renders semantic and tentative matches into `change.md` for Gate 1 review.

The agent never writes `plan.yaml`, never writes `discovery.md`, and never decides authority.

## Wire Contracts

The canonical closed tables live in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This milestone appends:

- **Journal events:** `plan.reconcile.agent`, `plan.reconcile.completed`.
- **Operational validation codes:** `plan-reconcile-lead-orphan`, `plan-reconcile-partition`, `plan-reconcile-required-group-split`, `plan-reconcile-concept-orphan`, `plan-reconcile-concept-unbound`, `plan-reconcile-project-binding-required`, `plan-reconcile-project-orphan`, `plan-reconcile-project-target-mismatch`, `plan-propose-missing-grouping`. These are `Error::Validation` outcomes and abort with exit 2.
- **Schema:** `schemas/discovery/proposal.schema.json` (`PROPOSAL_JSON_SCHEMA`), covering request and response envelopes.

## Resolved Question

Optional lead target-axis hints are deferred to a follow-on RFC. M2a ships pure agent target binding wrapped by the deterministic kernel above.
