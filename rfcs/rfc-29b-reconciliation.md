# RFC-29b: Plan-Time Lead Reconciliation

> Status: Draft — Milestone **M2a** of [RFC-29](rfc-29-fan-in-fan-out.md) — Depends: [RFC-29a](rfc-29a-source.md) (surveyed `discovery.md`) — Unblocks: the M2b plan rows in [RFC-29c](rfc-29c-synthesis.md)

This milestone closes plan-time fan-in. It turns the surveyed `Lead[]` from multiple sources into the `plan.yaml.slices[]` rows that `/spec:execute` will later run.

The important split is simple:

- The **agent** decides which leads describe the same work, which targets each group should build for, and, in workspace mode, which registry project owns each row.
- The **CLI** supplies a deterministic floor, validates the agent's response, derives slice names, emits journal events, and writes the plan. The agent never hand-edits `plan.yaml`.

The shared wire contracts this milestone extends are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This document is the source of truth for D2.

## Decision


| ID                         | Decision                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **D2 Lead reconciliation** | `specrun plan propose` wraps agent-led lead reconciliation in a CLI-owned projection kernel.The agent groups surveyed leads, binds each `(group-id, target)` row to one target and, in workspace mode, one registry project. The kernel computes the structural floor, validates the response schema, enforces the global lead partition, prevents floor groups from being split, validates project bindings, derives slice names, emits journal events, and writes `plan.yaml.slices[]`. |


## Operator Surface

```bash
specrun plan propose --dry-run --format json
specrun plan propose --from <response.json> [--format json]
```

`--dry-run` writes nothing. It reads `plan.yaml.sources`, the surveyed `discovery.md` lead inventory, operator-authored aliases, bound targets, and, in workspace mode, `registry.yaml`. It returns a request envelope for the agent.

`--from` is the only writer. It validates the agent response, enforces the invariants below, derives slice names, emits reconciliation events, and writes slices through the existing `crates/workflow/src/change/plan/` writers.

## Reconciliation Flow

1. `/spec:plan` runs `specrun plan propose --dry-run --format json`.
2. The CLI returns the lead inventory, the deterministic structural floor, ungrouped leads, bound targets, and optional workspace `projects[]`.
3. The agent returns final groups. Each group row represents one future slice: one concept, one target, and optionally one registry project.
4. `/spec:plan` submits that response with `specrun plan propose --from <response.json>`.
5. The CLI validates and writes `plan.yaml.slices[]`.
6. The agent renders semantic and tentative matches into `change.md` so Gate 1 can review them.

The agent owns judgment. The CLI owns projection and persistence.

## Structural Floor

Before the agent sees the inventory, the kernel computes conservative groups that may not be split:

1. Exact canonical `id` match across source keys.
2. Exact alias match, recorded under the canonical id.
3. Transitive cross-reference through `Lead.sources[]`.
4. All remaining leads stay ungrouped.

The floor is a pure function of `discovery.md`. The agent may extend a floor group with semantic members, but `propose --from` rejects any response that splits a floor group with `plan-reconcile-structural-floor-violated`.

Every surveyed lead must appear exactly once across the whole response. Missing or duplicate members fail with `plan-reconcile-partition`; references to unknown leads fail with `plan-reconcile-lead-orphan`.

## Envelope

The request and response are both validated by `schemas/discovery/proposal.schema.json`, embedded as `PROPOSAL_JSON_SCHEMA`, with a closed `kind: request | response` discriminator.

Example request:

```yaml
version: 1
kind: request
sources: [docs, legacy]
lead-inventory:
  docs: [identity-api, password-reset]
  legacy: [identity-api, reset-password]
structural-floor:
  - group-id: identity-api
    rule: exact-id
    members:
      - { source-key: docs, lead-id: identity-api }
      - { source-key: legacy, lead-id: identity-api }
ungrouped:
  - { source-key: docs, lead-id: password-reset }
  - { source-key: legacy, lead-id: reset-password }
bound-targets: [contracts@v1, omnia@v1]
projects:
  - { name: identity-contracts, target: contracts@v1, description: "Versioned API contracts crate for the identity domain." }
  - { name: identity-service, target: omnia@v1, description: "Omnia identity service implementing auth and password flows." }
```

`projects[]` is present only in workspace mode.

Example response:

```yaml
version: 1
kind: response
groups:
  - group-id: identity-api
    slice-name: identity-contracts
    members:
      - { source-key: docs, lead-id: identity-api, match-basis: exact-id }
      - { source-key: legacy, lead-id: identity-api, match-basis: exact-id }
    target: contracts@v1
    project: identity-contracts
  - group-id: identity-api
    slice-name: identity-service
    members:
      - { source-key: docs, lead-id: identity-api, match-basis: exact-id }
      - { source-key: legacy, lead-id: identity-api, match-basis: exact-id }
      - { source-key: docs, lead-id: password-reset, match-basis: semantic }
      - { source-key: legacy, lead-id: reset-password, match-basis: semantic, tentative: true }
    rationale: "identity-api floor plus semantic merge of password reset leads"
    target: omnia@v1
    project: identity-service
    depends-on: [identity-contracts]
```

`group-id` is a concept id, not necessarily the slice name. It may repeat when the same concept fans out to multiple targets. `depends-on` names derived slice names, not group ids.

## Slice Names

Each response row becomes one `plan.yaml.slices[]` entry. The kernel assigns its name as follows:

1. If the row provides `slice-name`, validate and use it.
2. Else if `group-id` is not already used as a slice name in this response, use `group-id`.
3. Else use `<group-id>-<adapter-slug>`, where `<adapter-slug>` is the segment before `@v` in `target` (`contracts@v1` -> `contracts`).

After deriving all names, the kernel validates every `depends-on` entry against that name set.

## Project Binding

Workspace mode adds one more agent decision: which registry project owns each row.

In workspace mode, the dry-run request includes `projects[]` from the validated registry as `{ name, target, description }`. The agent must choose one project for every response row, using the row's domain and target plus the registry descriptions. The CLI does not choose a project, even when only one registry project matches the target.

Before writing, the kernel enforces:

1. A workspace request requires `project` on every row; a single-repo request forbids it. Failure: `plan-reconcile-project-binding-required`.
2. The named project must exist in the registry. Failure: `plan-reconcile-project-orphan`.
3. The project's registered `target` must equal the row's `target`. Failure: `plan-reconcile-project-target-mismatch`.

The validated value is written verbatim to `plan.yaml.slices[].project`. Build-time workspace routing resolves it against `registry.yaml` as described by [RFC-29c §"Per-slice fan-out (D5)"](rfc-29c-synthesis.md).

## Match Basis And Review

Each member carries a `match-basis`:

- `exact-id`
- `exact-alias`
- `cross-reference`
- `semantic`

`semantic` and `tentative: true` are review signals. The agent renders them into `change.md` for Gate 1 so the operator can accept the grouping, split it, or promote recurring semantic matches into aliases with `specrun plan amend --add-alias`.

The kernel does not decide whether a semantic match is correct. It only proves that the response is well-formed, partitions the lead set, preserves the structural floor, and binds valid targets/projects.

## Agent Responsibilities

During `/spec:plan`, the agent:

1. Calls `specrun plan propose --dry-run --format json`.
2. Groups ungrouped leads semantically and may extend, but not split, structural-floor groups.
3. Binds each group to one or more targets, producing one response row per `(group-id, target)` pair.
4. Adds `rationale`, `tentative` flags, `depends-on`, and optional `slice-name`.
5. In workspace mode, binds every row to a compatible `project`; in single-repo mode, omits `project`.
6. Calls `specrun plan propose --from <response.json>`.
7. Renders semantic and tentative matches into `change.md` for Gate 1 review.

The agent never writes `plan.yaml`, never writes `discovery.md`, and never decides authority.

## Wire Contracts

The canonical closed tables live in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This milestone appends:

- **Journal events:** `plan.reconcile.agent`, `plan.reconcile.completed`.
- **Operational validation codes:** `plan-reconcile-lead-orphan`, `plan-reconcile-partition`, `plan-reconcile-structural-floor-violated`, `plan-reconcile-project-binding-required`, `plan-reconcile-project-orphan`, `plan-reconcile-project-target-mismatch`, `plan-propose-missing-grouping`. These are `Error::Validation` outcomes and abort with exit 2.
- **Schema:** `schemas/discovery/proposal.schema.json` (`PROPOSAL_JSON_SCHEMA`), covering request and response envelopes.

## Resolved Question

Optional lead target-axis hints are deferred to a follow-on RFC. M2a ships pure agent target binding wrapped by the deterministic kernel above.