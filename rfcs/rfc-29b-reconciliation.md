# RFC-29b: Plan-Time Lead Reconciliation

> Status: Draft — Milestone **M2a** of [RFC-29](rfc-29-fan-in-fan-out.md) — Depends: [RFC-29a M1 (shipped)](rfc-29-fan-in-fan-out.md#sub-rfcs-and-milestone-ordering) (surveyed `discovery.md`) — Unblocks: the M2b plan rows in [RFC-29c](rfc-29c-synthesis.md)

This milestone closes plan-time fan-in. It turns the surveyed `Lead[]` from multiple sources into the `plan.yaml.slices[]` rows that `/spec:execute` will later run.

The important split is simple:

- The **agent** decides which leads describe the same work, which target slice candidates to emit, and, in workspace mode, which registry project owns each slice candidate.
- The **CLI** supplies deterministic locked blocks and advisory blocks, validates the agent's response, derives slice names, emits journal events, and writes the plan. The agent never hand-edits `plan.yaml`.

The shared wire contracts this milestone extends are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This document is the source of truth for D2.

## Decision


| ID                         | Decision                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **D2 Lead reconciliation** | `specrun plan propose` wraps agent-led lead reconciliation in a CLI-owned projection kernel. The agent partitions surveyed leads into `lead-groups[]`, then emits one or more `slice-candidates[]` rows per lead group by binding each `(concept-id, target)` pair to one target and, in workspace mode, one registry project. The kernel computes locked blocks and advisory blocks, tags each lead with at most one block, records block adjacency in `blocks[]`, validates the response schema, enforces the global lead partition, prevents locked blocks from being split, validates project bindings, derives slice names, emits journal events, and writes `plan.yaml.slices[]`. |


## Operator Surface

```bash
specrun plan propose --dry-run --format json
specrun plan propose --from <response.json> [--format json]
```

`--dry-run` writes nothing. It reads `plan.yaml.sources`, the surveyed `discovery.md` lead inventory, operator-authored aliases, targets, and, in workspace mode, `registry.yaml`. It returns a request envelope for the agent.

`--from` is the only writer. It validates the agent response, enforces the invariants below, derives slice names, emits reconciliation events, and writes slices through the existing `crates/workflow/src/change/plan/` writers.

## Reconciliation Flow

1. `/spec:plan` runs `specrun plan propose --dry-run --format json`.
2. The CLI returns a single lead-centric `leads[]` catalog — each lead tagged with at most one reconciliation `block` (untagged leads being unmatched) — plus the `blocks[]` adjacency index, `targets[]`, and, in workspace mode, `projects[]`.
3. The agent works block-by-block but returns one global `lead-groups[]` partition plus `slice-candidates[]`. Lead groups partition the surveyed leads; slice candidates fan those concepts out to targets and projects.
4. `/spec:plan` submits that response with `specrun plan propose --from <response.json>`.
5. The CLI validates and writes `plan.yaml.slices[]`.
6. The agent renders semantic and tentative matches into `change.md` so Gate 1 can review them.

The agent owns judgment. The CLI owns projection and persistence.

## Reconciliation Blocks

Before the agent sees the inventory, the kernel computes conservative locked blocks that may not be split. It then tags each member lead with a `block` object:

1. Exact canonical `id` match across source keys.
2. Exact alias match, recorded under the canonical id.
3. Transitive cross-reference through `Lead.sources[]`.

Locked blocks are a pure function of `discovery.md`. The agent may extend a locked block with semantic members, but `propose --from` rejects any response that splits a locked block with `plan-reconcile-required-group-split`.

The remaining open leads are not returned as one flat matching problem. The kernel partitions them into advisory blocks and tags each open lead with its block id so large surveys stay reviewable. A lead is only ever a member of one block:

1. Treat every locked block and every open lead as a node.
2. Attach deterministic blocking keys from surveyed lead metadata (`blocking-keys[]`) when present.
3. Add edges for shared blocking keys. When an open lead shares a key with a locked-block member, the kernel does not pull the locked block into the advisory block; instead the advisory block records that locked block under `blocks[].adjacent-blocks[]`. This is an invitation to consider extending the locked block, not a match decision.
4. For leads without blocking keys, use a fixed lexical fallback over normalized `id`, `aliases[]`, and `summary` tokens.
5. Each connected component becomes one advisory block id stamped onto its open-lead members only; any locked block in the component is recorded as an `adjacent-blocks[]` entry rather than tagged. Open leads with no useful edge are left without a `block`, marking them unmatched.
6. If a component exceeds the implementation limit, split it deterministically by strongest available key in this order: `domain:*`, `module:*`, `entity:*`, `route:*`, then lexical token.

Advisory blocks reduce the agent's search space; they do not decide semantic matching, do not constrain the final response, and do not relax the global partition check. The request exposes the block-level key union as `blocks[].keys[]`; per-lead `blocking-keys[]` remain surveyed `discovery.md` metadata and are not repeated on every request lead.

Every surveyed lead must appear exactly once across `lead-groups[].members[]`. Missing or duplicate members fail with `plan-reconcile-partition`; member `key`s that name no request lead fail with `plan-reconcile-lead-orphan`. Every `slice-candidates[].concept-id` must name a declared lead group (`plan-reconcile-concept-orphan`), and every lead group must have at least one slice candidate (`plan-reconcile-concept-unbound`).

## Envelope

The request and response are both validated by `schemas/discovery/proposal.schema.json`, embedded as `PROPOSAL_JSON_SCHEMA`, with a closed `kind: request | response` discriminator.

The request is lead-centric: a single `leads[]` list carries the surveyed catalog, and each lead names at most one reconciliation `block`. A small top-level `blocks[]` index records whether a block is locked or advisory, the blocking keys that explain the block, and the one cross-cutting fact a single lead tag cannot: which locked blocks an advisory block sits adjacent to.

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
blocks:
  - id: identity-api
    constraint: locked
    keys: [domain:identity, module:auth]
  - id: identity
    constraint: advisory
    keys: [domain:identity, module:auth, capability:password-reset]
    adjacent-blocks: [identity-api]
leads:
  - key: docs:identity-api
    source: docs
    id: identity-api
    summary: "Identity API contract for authentication and account access."
    block:
      id: identity-api
      basis: exact-id
  - key: legacy:identity-api
    source: legacy
    id: identity-api
    summary: "Legacy identity endpoints."
    block:
      id: identity-api
      basis: exact-id
  - key: docs:password-reset
    source: docs
    id: password-reset
    summary: "Users can request a password reset email."
    block:
      id: identity
  - key: legacy:reset-password
    source: legacy
    id: reset-password
    summary: "Legacy reset-password flow."
    block:
      id: identity
```

Each lead carries a stable `key` (`<source>:<id>`), its `source` and `id`, the surveyed `summary`, and at most one `block`:

- `block.id` names the reconciliation block this lead belongs to.
- `block.basis` is present when the kernel proved the lead belongs to a locked block. It records this member's match basis (`exact-id`, `exact-alias`, or `cross-reference`; basis is per-lead, so one locked block may mix bases).

A lead carries `block` or no `block` — the three states being **settled** (locked block), **candidate** (advisory block), and **unmatched** (no block). The unmatched residue replaces the old explicit `unmatched-leads[]` array.

The top-level `blocks[]` index is the bridge between settled and candidate states: locked blocks carry `constraint: locked`; advisory blocks carry `constraint: advisory`. `keys[]` is the block-level union of blocking keys that explain why the kernel formed the block; lexical fallback keys use the `token:<term>` namespace. When an advisory block sits adjacent to one or more locked blocks (an open-lead member shares a blocking key with a locked-block member), it lists them under `adjacent-blocks[]`. That is the kernel's invitation to consider extending those locked blocks with the advisory block's open leads. `mode` declares the reconciliation mode; `projects[]` is present only in workspace mode.

Example response:

```yaml
version: 1
kind: response
lead-groups:
  - concept-id: identity-api
    block-id: identity
    members:
      - { key: docs:identity-api, match-basis: exact-id }
      - { key: legacy:identity-api, match-basis: exact-id }
      - { key: docs:password-reset, match-basis: semantic }
      - { key: legacy:reset-password, match-basis: semantic, tentative: true }
    rationale: "identity API plus semantic password reset merge"
slice-candidates:
  - concept-id: identity-api
    slice-name: identity-contracts
    target: contracts@v1
    project: identity-contracts
  - concept-id: identity-api
    slice-name: identity-service
    target: omnia@v1
    project: identity-service
    depends-on: [identity-contracts]
```

`lead-groups[]` is the partition of the surveyed leads. Each member references a lead by its request `key` and carries the agent's per-member `match-basis` (and optional `tentative`). `block-id` is optional review traceability back to the request's advisory block; it carries no validation authority over membership. `slice-candidates[]` is the plan-row projection surface: each candidate names a `concept-id`, target, optional project, optional explicit `slice-name`, and optional dependencies. A `concept-id` may appear in multiple slice candidates when the same concept fans out to multiple targets, but the members are declared once under `lead-groups[]`. `depends-on` names derived slice names, not concept ids.

## Slice Names

Each `slice-candidates[]` entry becomes one `plan.yaml.slices[]` entry. The kernel assigns its name as follows:

1. If the candidate provides `slice-name`, validate and use it.
2. Else if `concept-id` is not already used as a slice name in this response, use `concept-id`.
3. Else use `<concept-id>-<adapter-slug>`, where `<adapter-slug>` is the segment before `@v` in `target` (`contracts@v1` -> `contracts`).

After deriving all names, the kernel validates every `depends-on` entry against that name set.

## Project Binding

Workspace mode adds one more agent decision: which registry project owns each slice candidate.

In workspace mode, the dry-run request includes `projects[]` from the validated registry as `{ name, target, description }`. The agent must choose one project for every slice candidate, using the concept, target, and registry descriptions. The CLI does not choose a project, even when only one registry project matches the target.

Before writing, the kernel enforces:

1. A workspace request requires `project` on every slice candidate; a single-repo request forbids it. Failure: `plan-reconcile-project-binding-required`.
2. The named project must exist in the registry. Failure: `plan-reconcile-project-orphan`.
3. The project's registered `target` must equal the candidate's `target`. Failure: `plan-reconcile-project-target-mismatch`.

The validated value is written verbatim to `plan.yaml.slices[].project`. Build-time workspace routing resolves it against `registry.yaml` as described by [RFC-29c §"Per-slice fan-out (D5)"](rfc-29c-synthesis.md).

## Match Basis And Review

Each `lead-groups[].members[]` entry carries a `match-basis`:

- `exact-id`
- `exact-alias`
- `cross-reference`
- `semantic`

`semantic` and `tentative: true` are review signals. The agent renders them into `change.md` for Gate 1 so the operator can accept the grouping, split it, or promote recurring semantic matches into aliases with `specrun plan amend --add-alias`.

The kernel does not decide whether a semantic match is correct. It only proves that the response is well-formed, partitions the lead set, preserves locked blocks, and binds valid targets/projects.

## Agent Responsibilities

During `/spec:plan`, the agent:

1. Calls `specrun plan propose --dry-run --format json`.
2. Walks the open leads by advisory `block`, grouping them semantically and, where a block names `adjacent-blocks[]`, possibly extending those locked blocks without splitting them; untagged leads are the unmatched residue to place.
3. Binds each lead group to one or more targets, producing one slice candidate per `(concept-id, target)` pair.
4. Adds `rationale`, `tentative` flags, `depends-on`, and optional `slice-name`.
5. In workspace mode, binds every slice candidate to a compatible `project`; in single-repo mode, omits `project`.
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