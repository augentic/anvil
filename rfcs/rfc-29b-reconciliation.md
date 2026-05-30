# RFC-29b: Plan-Time Lead Reconciliation

> Status: Draft — Milestone **M2a** of [RFC-29](rfc-29-fan-in-fan-out.md) — Depends: [RFC-29a](rfc-29a-source-operations.md) (consumes its surveyed `discovery.md`) — Unblocks: the plan-time fan-in contract and the M2b plan rows ([RFC-29c](rfc-29c-synthesis-typed-model.md))

This is the second independently shippable milestone of [RFC-29](rfc-29-fan-in-fan-out.md). It closes plan-time fan-in: an agent-led cross-source matching step that groups each source's `Lead[]` into unified slice candidates (including semantic matches that exact id / alias cannot catch), binds each `(group-id, target)` row to a target, and — in workspace mode — to a registry project. `specrun plan propose` closes this without synthesis or `model.yaml`.

The cross-milestone wire contracts this milestone appends to are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This document is the source of truth for D2.

## Decision owned by this milestone

| ID | Decision |
| -- | -------- |
| **D2 Lead reconciliation engine** | Agent-led cross-source matching of `Lead[]` into lead groups — which leads describe the same unit of work (including semantic matches beyond exact id / alias / cross-reference), each group's `(source-key, lead-id)` members and per-member `match-basis`, each `(group-id, target)` row's target, and — workspace mode — its registry project. The CLI owns the projection kernel: the deterministic structural floor the agent may extend but never split, the registry pre-pass, schema validation, the global lead-partition invariant, project-binding validation, slice-name derivation, journal events, and the plan writers. Carries `execution: agent \| tool`; `agent` is the default and designed centre. |

## Two layers

There is no deterministic function from `Lead[]` across heterogeneous sources to a coherent set of slice candidates — deciding that the documentation lead `password-reset` and the legacy-code lead `reset-password` describe the same unit of work is exactly the cross-source judgment call the framework exists to make. So D2 mirrors D3: the judgment layer is agent-led and first, the projection layer is CLI-owned and deterministic.

1. **Lead-matching step (judgment, agent-led — the heart).** Cross-source reconciliation of `Lead[]` into lead groups: deciding which leads across sources describe the same unit of work, including **semantic** matches that exact id / alias / cross-reference cannot catch, declaring each group's `(source-key, lead-id)` members with a `match-basis` per member (`exact-id` | `exact-alias` | `cross-reference` | `semantic`), binding each group to exactly one target (one slice per `(group, target)` pair, per D5), **binding each row to a registry project** (workspace mode — §"Project selection"), authoring the per-group rationale and any `tentative` low-confidence flags the operator should eyeball, and rendering the "Lead inventory" / "Tentative merges" prose into `change.md`. This is the load-bearing judgment of plan-time fan-in and stays with the agent.
2. **Projection kernel (deterministic projection, CLI-owned).** A structural **floor** — exact id, exact alias, transitive cross-reference (rules 1–3 below) — computed deterministically and handed to the agent in the request envelope; the registry pre-pass that surfaces `projects[]` (name + target + description) into the request in workspace mode; schema validation of the returned grouping; the **global lead-partition invariant** (every surveyed lead lands in exactly one group across the whole response — no orphan or duplicate members, every cited `(source, lead-id)` exists in `discovery.md`); structural-floor preservation (the agent may *extend* a floor group with a semantic member but may not *split* a floor match); **project-binding validation** (§"Project selection"); **slice-name derivation** (§"Slice-name derivation"); journal events; and the write of `plan.yaml.slices[]` through the existing `crates/workflow/src/change/plan/` writers. The kernel projects over the structure the agent returns; it never invents, drops, or re-groups leads on its own heuristic, never merges on textual similarity by itself, never picks a project the registry does not declare, and never overrides a structural-floor match.

## Command

```bash
specrun plan propose --dry-run --format json          # returns the reconciliation request envelope (floor + inventory + registry projects); writes nothing
specrun plan propose --from <response.json> [--format json]   # kernel: validate → partition/floor + project-binding invariants → slice-name derivation → journal → plan writers
```

`propose --dry-run` reads `plan.yaml.sources`, the `discovery.md` lead inventory (via the in-place `crates/model/src/discovery/` model — `Discovery::parse` + `Discovery::resolve_lead` already cover the join surface), optional operator-authored aliases, and — in workspace mode — the parsed `registry.yaml` topology (it validates the registry first, exactly as the `/spec:plan` row of [docs/reference/registry.md](../docs/reference/registry.md#verbs) requires). It writes **nothing** to disk and returns the request envelope, including `projects[]` when the registry declares projects. `propose --from` consumes the agent's grouping response and is the only writer; the agent never hand-edits `plan.yaml`.

## Structural floor (kernel)

The kernel's deterministic pre-pass is intentionally conservative — it is a *floor*, not the final grouping:

1. Exact canonical `id` match across source keys -> one floor group.
2. Exact alias match -> one floor group, recorded under the canonical id.
3. One lead's `sources` list transitively names another source's lead id (the existing `Lead.sources[]` cross-reference field) -> one floor group.
4. Otherwise each lead starts ungrouped.

The floor is a pure function of the parsed discovery document. The agent receives it pre-computed so it never has to re-derive the trivial matches and can spend its judgment on the semantic joins (rule 4 leftovers). The kernel later refuses any response that *splits* a floor group (`plan-reconcile-structural-floor-violated`); the agent may only add semantic members on top.

## Reconciliation envelope

The matching step receives a fixed-shape request and returns a fixed-shape response, dispatched to the operator's agent under `execution: agent` (the default and designed centre) or to a declared WASI tool under `execution: tool` (the D10-style mirror; see [RFC-29c §"Synthesis execution mode (D10)"](rfc-29c-synthesis-typed-model.md) for the agent-first rationale, which applies identically here). The request:

```yaml
version: 1
kind: request
sources: [docs, legacy]
lead-inventory:
  docs:    [identity-api, password-reset]
  legacy:  [identity-api, reset-password]
structural-floor:
  - group-id: identity-api
    rule: exact-id
    members:
      - { source-key: docs,   lead-id: identity-api }
      - { source-key: legacy, lead-id: identity-api }
ungrouped:
  - { source-key: docs,   lead-id: password-reset }
  - { source-key: legacy, lead-id: reset-password }
bound-targets: [contracts@v1, omnia@v1]
projects:                                            # workspace mode only — absent in single-repo mode
  - { name: identity-contracts, target: contracts@v1, description: "Versioned API contracts crate for the identity domain." }
  - { name: identity-service,   target: omnia@v1,     description: "Omnia identity service implementing auth and password flows." }
```

The response declares the final grouping and target binding. Each row is one `(group-id, target)` pair (one plan slice per row, per D5):

```yaml
version: 1
kind: response
groups:
  - group-id: identity-api
    slice-name: identity-contracts
    members:
      - { source-key: docs,   lead-id: identity-api,   match-basis: exact-id }
      - { source-key: legacy, lead-id: identity-api,   match-basis: exact-id }
    target: contracts@v1
    project: identity-contracts                       # only project whose registered target is contracts@v1
  - group-id: identity-api
    slice-name: identity-service
    members:
      - { source-key: docs,   lead-id: identity-api,     match-basis: exact-id }
      - { source-key: legacy, lead-id: identity-api,     match-basis: exact-id }
      - { source-key: docs,   lead-id: password-reset,   match-basis: semantic }
      - { source-key: legacy, lead-id: reset-password,     match-basis: semantic, tentative: true }
    rationale: "identity-api floor plus semantic merge of docs 'password-reset' and legacy 'reset-password' into one omnia slice"
    target: omnia@v1
    project: identity-service                          # omnia@v1 row routed to the omnia-targeted project
    depends-on: [identity-contracts]
```

`group-id` is a **concept id** for related work — it may repeat when the same concept fans out to multiple targets. It is not the plan slice name unless the derived name happens to equal it. The kernel derives the unique `plan.yaml.slices[]` name from optional `slice-name` or the rule in §"Slice-name derivation". `depends-on` lists **derived slice names**, not group-ids. Full request/response shape: [`rfc-29/schemas/discovery/proposal.schema.json`](rfc-29/schemas/discovery/proposal.schema.json) (`kind: request | response` discriminator), embedded as `PROPOSAL_JSON_SCHEMA`. `propose --dry-run` validates its own request output before returning; `propose --from` validates the response before projecting.

## Slice-name derivation

Each response row binds one `(group-id, target)` pair to exactly one `plan.yaml.slices[]` entry. The kernel assigns the slice name deterministically:

1. If the row carries optional `slice-name`, validate it against the slice-name grammar and use it.
2. Else if `group-id` is not already assigned as a slice name in this response, use `group-id`.
3. Else use `<group-id>-<adapter-slug>`, where `<adapter-slug>` is the adapter name segment before `@v` in `target` (e.g. `contracts@v1` → `contracts`, yielding `identity-api-contracts`).

The kernel validates every `depends-on` entry against the set of derived slice names from the same response before writing. Leads may not be legitimately dropped: every surveyed lead must appear in exactly one group **globally** across the response (`plan-reconcile-partition`).

## Project selection

This is the load-bearing decision for fan-out: which **registry project** each `(group, target)` row runs against. Without it a cross-target change cannot fan out — every slice would route to the same default slot, defeating the per-slice one-target / one-project model (D5). Project selection mirrors lead matching — agent judgment under a deterministic kernel — because there is no deterministic function from a slice's domain to the right project when a registry declares several projects sharing one target adapter.

**Kernel pre-pass (deterministic).** In workspace mode the kernel parses and validates `registry.yaml`, then surfaces every entry into the request envelope's `projects[]` as `{ name, target, description }`. In single-repo mode — no `registry.yaml`, an empty registry, or a single-entry registry that behaves like single-repo mode — `projects[]` is omitted and the kernel writes no `project` field (phase work runs in the project root, per [RFC-29c §"Per-slice fan-out (D5)"](rfc-29c-synthesis-typed-model.md)). The agent never re-reads `registry.yaml`; the request is the sole project surface.

**Agent judgment.** For each row the agent selects exactly one `project` from `projects[]` by matching the row's domain — its members, `group-id`, and authored `rationale` — and its `target` against each project's `description` and registered `target`. The `description` field exists precisely to disambiguate when more than one project shares a target adapter (it is required in `registry.yaml` whenever more than one project is declared); that judgment is the agent's, surfaced through the per-row `rationale`.

**Kernel validation (deterministic).** Before writing, the kernel enforces, for every row:

1. **Presence.** A workspace request (`projects[]` non-empty) requires a `project` on every row; a single-repo request forbids it. A missing or stray `project` is `plan-reconcile-project-binding-required`.
2. **Existence.** The named `project` must appear in `registry.yaml`; an unknown name is `plan-reconcile-project-orphan`.
3. **Target agreement.** The chosen project's registered `target` must equal the row's `target` — a `contracts@v1` row may only bind a `contracts@v1` project. A mismatch is `plan-reconcile-project-target-mismatch`.

The kernel never *chooses* a project — even when exactly one registry project matches the row's target, the agent must name it — and never overrides the agent's choice; it only proves the binding is present, real, and target-compatible. The validated `project` is written verbatim to `plan.yaml.slices[].project`, where build-time workspace routing resolves it against `registry.yaml` ([RFC-29c §"Per-slice fan-out (D5)"](rfc-29c-synthesis-typed-model.md), unchanged).

## Match basis and operator review

`match-basis: semantic` (and any member flagged `tentative: true`) is the structured form of the "Tentative merges" Markdown block the agent renders into `change.md` for the operator. Semantic merges are the agent's judgment, surfaced for operator review at Gate 1 — the operator may accept them as-is, run `specrun plan amend --add-alias` to promote a recurring semantic match into a durable alias (so the next survey resolves it on the structural floor), or split the slice. The kernel does not adjudicate whether a semantic merge is *correct*; it only proves the grouping is a well-formed partition that respects the floor. The agent may also call `specrun plan amend --divergence likely` against any written slice whose bound leads carry materially disagreeing summaries; that writer path already exists.

## Agent role

`/spec:plan`'s `propose` sub-step:

1. Calls `specrun plan propose --dry-run --format json` to obtain the request envelope (floor + lead inventory + bound targets + registry `projects[]` in workspace mode).
2. Matches the `ungrouped` leads across sources by judgment — extending floor groups with semantic members and forming new groups — without ever splitting a floor group.
3. Binds each group to one or more targets, expanding to one `(group, target)` slice per binding (cross-target work uses `depends-on`, per D5), and authors per-group `rationale` plus `tentative` flags.
4. In workspace mode, binds each row to a `project` from `projects[]` whose registered target matches the row's target, judging on `description` (§"Project selection"); in single-repo mode, omits `project`.
5. Submits the grouping with `specrun plan propose --from <response.json>`, which validates, enforces the invariants, binds and validates projects, derives slice names, emits `plan.reconcile.agent`, and writes the slices through the existing plan writers.
6. Renders the semantic / `tentative` merges into `change.md` for operator review at Gate 1.

The agent never hand-edits `plan.yaml`, never writes `discovery.md` directly, and never decides authority — its scope is cross-source matching, target binding, project binding, and rationale.

## Wire contracts introduced by this milestone

The canonical closed tables live in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This milestone appends:

- **Journal events:** `plan.reconcile.agent`, `plan.reconcile.completed`.
- **Operational validation codes (`Error::Validation`, not new enum variants):** `plan-reconcile-lead-orphan`, `plan-reconcile-partition`, `plan-reconcile-structural-floor-violated`, `plan-reconcile-project-binding-required`, `plan-reconcile-project-orphan`, `plan-reconcile-project-target-mismatch`, `plan-propose-missing-grouping` — single-signal `plan propose` aborts at exit 2. See [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts) for the error-tiering model.
- **Schema:** `schemas/discovery/proposal.schema.json` (`PROPOSAL_JSON_SCHEMA`), request + response discriminated by closed `kind`.

## Resolved question

Optional lead target-axis *hints* (agent assist, not replacement) are **resolved**: option (a) is adopted as the chosen direction for a dedicated follow-on RFC, while v1 ships pure agent binding unchanged. See [RFC-29 §"Open questions"](rfc-29-fan-in-fan-out.md#open-questions) (Resolved — was Q1).
