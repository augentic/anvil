# RFC-29c: Slice Synthesis Engine and Typed Model

> Status: Draft — Milestone **M2b** of [RFC-29](rfc-29-fan-in-fan-out.md) — Companion: [RFC-29d](rfc-29d-target.md), the target build envelope that consumes this milestone's `model.yaml`

This milestone defines how a slice's `Evidence` becomes a synthesized, schema-typed requirement set. It owns slice-time synthesis (D3, D3a, D10), the typed `model.yaml` artifact (D4), per-slice fan-out (D5), the scope of the target `shape` brief (D8), standalone provenance projection (D11), and the claim contract that ties each requirement back to its Evidence (D13). The wire contracts these decisions share with the rest of the RFC-29 family — schemas, journal events, and validation-finding codes — are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts).

## Decisions owned by this milestone

| ID | Decision |
| -- | -------- |
| **D3 Slice synthesis** | Agent-led cross-modal reconciliation of `Evidence[]` into the requirement set; CLI owns the synthesis envelope and projection kernel. |
| **D3a Draft vs persisted** | Synthesis response validates against `draft-model.schema.json`; persisted `model.yaml` validates against `model.schema.json`. |
| **D4 Typed slice** | Every synthesized slice carries `.specify/slices/<slice>/model.yaml`. |
| **D5 Per-slice fan-out** | Each slice binds exactly one target adapter / project; cross-target changes decompose at plan time into multiple slices joined by `depends-on`. No `outputs[]`. |
| **D8 Shape-brief scope** | Target `shape` briefs parameterise non-requirements model sections only. |
| **D10 Synthesis execution mode** | The synthesis step carries a closed `execution: agent \| tool` enum (default `agent`). |
| **D11 Standalone provenance projection** | `specrun slice provenance <slice>` runs the same projection kernel as D3. |
| **D13 Claim contract (`id` + `kind`)** | Every contributing claim carries a stable `claim-id` and its `kind`; `model.yaml` claims carry `kind` for per-kind authority resolution. |

## Slice synthesis engine (D3)

Synthesis turns a slice's `Evidence[]` into its requirement set. Cross-modal reconciliation — deciding which requirements exist, how claims from different sources merge or split, and what each requirement means — has no deterministic function, so it stays the agent's judgment. Everything around that judgment that *can* be made deterministic is the CLI's. The engine therefore splits into two layers: an agent-led **synthesis step** and a CLI-owned **projection kernel**.

### Agent and kernel responsibilities

1. **The synthesis step (agent).** The agent reconciles `Evidence[]` into the requirement set: which requirements exist and how claims merge or split. For each requirement it records the contributing `(source, claim-id)` claims, an `agreement` verdict (`agreed` \| `disagreed`), the behavioral prose (`title`, `statement`, `scenarios[]`, `notes`), and the owning `unit`. It also authors the prose for the non-requirements model sections and the prose-only Markdown artifacts — `proposal.md`, `design.md`, `tasks.md`, and the spec bodies, the last of these written **without** `ID:` / `Sources:` / `Status:` lines.
2. **The projection kernel (CLI).** The kernel projects deterministically over whatever structure the agent returns. It resolves authority (§"Authority resolution"), assigns `REQ` ids in declaration order, derives `sources`, winner markers, and `status` from the claims, agreement verdict, and resolved authority, projects `provenance.yaml`, renders provenance lines into `spec.md`, and runs the drift validators (§"Drift validation"). It never invents, drops, or re-groups requirements; it never selects a winner the resolved authority did not; and it never overrides the agent's agreement verdict.

### Command

```bash
specrun slice synthesize <slice> [--format json]
```

The command **reads** the slice metadata and target binding, `plan.yaml.slices[].sources`, the per-source `evidence/*.yaml`, the bound target's `shape` brief, the prior baseline specs, and any operator override fields (`authority-override`).

It **writes** the following artifacts. All writes are staged, and the prior artifacts are kept intact on failure:

```text
.specify/slices/<slice>/proposal.md
.specify/slices/<slice>/specs/<unit>/spec.md
.specify/slices/<slice>/design.md
.specify/slices/<slice>/tasks.md
.specify/slices/<slice>/provenance.yaml
.specify/slices/<slice>/model.yaml
```

End to end, the command resolves authority, builds the synthesis request envelope (Evidence plus resolved authority — and **no** shape brief for the requirements section), dispatches the synthesis step, receives the draft, and then runs the persist pipeline below.

### Persist pipeline

The kernel persists in six ordered steps; the slice transitions to `refined` only after step 6 completes cleanly:

1. Validate the response envelope and the draft `model` against `draft-model.schema.json`.
2. Reject usurped kernel fields (`generated-at`, `generator`, top-level `sources`, `requirements[].id`, `.status`, `.sources`, `claims[].winner`) with `slice-synthesize-kernel-field-usurped`, and reject orphan claims with `slice-model-source-orphan`.
3. Project the kernel over the draft — ids, sources, status, winners, top-level `sources`, `generated-at`, `generator`, and `provenance.yaml`.
4. Validate the merged `model.yaml` against `model.schema.json`.
5. Render the kernel-owned provenance lines into `spec.md` (§"Rendering").
6. Run the drift validators and persist if clean.

### Synthesis envelope

The synthesis step communicates with the kernel over a closed request/response envelope, dispatched under `execution: agent` (default) or `execution: tool` (D10). Its schema is [`synthesis.schema.json`](rfc-29/schemas/slice/synthesis.schema.json), discriminated by `kind: request | response`; the response `model` `$ref`s [`draft-model.schema.json`](rfc-29/schemas/slice/draft-model.schema.json) rather than the persisted schema (D3a).

A request looks like this:

```yaml
version: 1
kind: request
slice: identity-service
target: omnia@v1
shape-brief: /.../adapters/targets/omnia/briefs/shape.md
evidence:
  docs:
    path: .specify/slices/identity-service/evidence/docs.yaml
    authority: documentation
  legacy:
    path: .specify/slices/identity-service/evidence/legacy.yaml
    authority: behaviour
authority:
  resolved-path: .specify/.cache/synthesize/<slice>/authority.resolved.yaml
prior-baseline:
  specs-dir: .specify/specs/
constraints:
  forbidden-inputs-for-requirements-reconciliation: [target, shape-brief]
```

Requirements reconciliation may draw only on Evidence and the resolved authority, as `constraints.forbidden-inputs-for-requirements-reconciliation` records; `target` and `shape-brief` are valid inputs for the non-requirements model sections but never for requirements. The response carries the draft model plus the prose-only Markdown; the kernel-owned fields it must not set are listed in §"Agent and kernel responsibilities".

### Authority resolution

Authority is resolved before dispatch and passed into the envelope. The resolution order is:

1. per-slice `authority-override`;
2. per-Evidence `authority-overrides`;
3. document-level `authority`;
4. tied effective authority → `conflict`.

The synthesis step never re-decides authority or marks winners. Once it returns the claims and `agreement` verdict, the kernel projects the winners and derives `status` and `sources` from them.

**Per-claim resolution (mixed kinds).** Authority is keyed by `ClaimKind`, so a single requirement can mix claim kinds. For each claim `(source, claim-id, kind)` the kernel walks the same order — per-slice `authority-override[kind]` → Evidence `authority-overrides[kind]` → document-level `authority` → the default `intent > documentation > behaviour`. Among `disagreed` claims, the winner is the strictly-greatest effective class; a tie at the top class yields `conflict` with no winner markers.

### Status derivation

The kernel derives each requirement's `status` from the claim count, the agent's `agreement` verdict, and the resolved authority. Agreement classification is the agent's; winner selection among disagreements is the kernel's.

| `claims` | `agreement` | Kernel `status` | `provenance.yaml` `resolution` | Winner markers |
| -------- | ----------- | --------------- | ------------------------------ | -------------- |
| 0 | *(omitted)* | `unknown` | `unknown-no-evidence` | none |
| 1 | *(omitted)* | `agreed` | `single-source` | none |
| ≥2 | `agreed` | `agreed` | `single-value-agreement` | none |
| ≥2 | `disagreed`, unique top authority | `divergence` | `authority-resolved` / `per-slice-override` | winner `true`, losers `false` |
| ≥2 | `disagreed`, top authority ties | `conflict` | `tied-conflict` | none |

Losing claims survive in `provenance.yaml` with `winner: false`. As a non-blocking cross-check, the kernel runs a normalised-string comparison over `agreed` requirements; a mismatch emits `slice-synthesize-agreement-suspect` as a `review` finding, not a transition blocker.

### Shape-brief scope (D8)

The shape brief parameterises the **non-requirements** sections only — `domain`, `apis`, `configuration`, `technical-logic`, `observability`, and `tasks`. It MUST NOT influence `requirements[]`, claims, `agreement`, `sources[]`, or any provenance-bearing field.

Two gates enforce this (see [RFC-29d §"Acceptance proof (D7)"](rfc-29d-target.md#acceptance-proof-d7)):

1. **Envelope proof** — the requirements-relevant request inputs are byte-identical across target bindings.
2. **Kernel determinism** — given a fixed synthesis response, kernel output is byte-identical and target-independent.

The `slice-synthesize-forbidden-input-leak` finding complements gate 1 by flagging a response whose requirements section references `target` or `shape-brief` content.

### Rendering

Synthesis output is rendered in three phases, each with a single owner:

| Phase | Author | Output |
| ----- | ------ | ------ |
| Synthesis step | Agent | Draft model + prose-only Markdown |
| Projection kernel | CLI | Full `model.yaml`, `provenance.yaml` |
| Render step | CLI | Injects `ID:` / `Sources:` / `Status:` (and status tags) into `specs/<unit>/spec.md` |

`spec.md` remains the behavioral review and merge input, and its provenance lines are rendered from `model.yaml`. Hand-editing a kernel-rendered provenance line without re-synthesising raises `slice-spec-provenance-stale`. `provenance.yaml` is audit-only and always kernel-projected. Re-synthesis overwrites `model.yaml`, `provenance.yaml`, and the kernel-rendered provenance lines; operator prose outside those lines survives until the agent returns different bodies.

### Standalone provenance (D11)

```bash
specrun slice provenance <slice> [--format json]
```

This is a standalone entry point onto the same projection kernel as D3, for regenerating `provenance.yaml` without a full re-synthesis. It reads the persisted `model.yaml`, the `Evidence[]`, and any authority overrides; runs the identical projection; and writes `provenance.yaml` only. Output is byte-stable over unchanged inputs. It never reads the target or shape brief, and never re-decides requirements, winners, or agreement.

Typical uses are regenerating `provenance.yaml` after hand-editing claims, and exercising the shared kernel module in determinism tests.

## Typed slice model (D4)

Every synthesized slice carries a machine-readable `model.yaml` alongside its Markdown artifacts. The Markdown stays the human review surface; `model.yaml` is the schema-pinned view that target builders consume.

### File

```text
.specify/slices/<slice>/model.yaml
```

The file is generated whole by `specrun slice synthesize`. Operators edit `spec.md` and `design.md`, never `model.yaml` directly.

### Shape

The normative shape is [`model.schema.json`](rfc-29/schemas/slice/model.schema.json): a closed top level, kebab-case on disk. The required keys are `version`, `slice`, `generated-at`, `generator`, `sources`, `target`, `requirements`, `domain`, `apis`, `configuration`, `technical-logic`, `observability`, and `tasks`; `project` is optional.

The sketch below is illustrative (comments mark which fields the kernel owns and which the agent authors):

```yaml
version: 1
slice: identity-service
generated-at: 2026-05-28T05:45:00Z
generator: specrun@2.1.0
sources:
  - key: docs
    adapter: documentation
    lead: password-reset
    authority: documentation
    evidence-path: .specify/slices/identity-service/evidence/docs.yaml
target: omnia@v1
project: identity-service
requirements:
  - id: REQ-001          # kernel
    title: Request password reset
    status: agreed       # kernel
    unit: password-reset
    sources: [docs, legacy]  # kernel
    agreement: agreed    # agent
    claims:
      - { source: docs,   claim-id: password-reset.request,       kind: requirement }
      - { source: legacy, claim-id: users.password-reset.request, kind: example }
    statement: The system lets a registered user request a password reset link by email.
    scenarios:
      - Given REQ-001 and a registered email, when the user requests a reset, then the system accepts the request.
domain:
  types: []
apis:
  surfaces: []
configuration: []
technical-logic:
  decisions: []
observability: []
tasks:
  - id: TASK-001
    text: Implement password reset request handling.
    satisfies: [REQ-001]
```

### Provenance index

```text
.specify/slices/<slice>/provenance.yaml
```

`provenance.yaml` is kernel-projected from the persisted `model.yaml` claims, the `Evidence[]`, and any authority overrides. Its normative shape is `schemas/slice/provenance.schema.json`. It is audit-only — downstream verbs read `spec.md`, not this file. The sketch below covers the same `identity-service` slice and illustrates the three resolution paths:

```yaml
version: 1
slice: identity-service
generated-at: 2026-05-28T05:45:00Z
generator: specrun@2.1.0
requirements:
  - id: REQ-001
    status: agreed
    sources: [docs, legacy]
    contributing-claims:
      - source: docs
        claim-id: password-reset.request
        kind: requirement
        value: "The system lets a registered user request a password reset link by email."
        path: docs/identity/reset.md#L4
      - source: legacy
        claim-id: users.password-reset.request
        kind: example
        value: "POST /password-reset returns 202 and queues an email."
        path: src/users/reset.ts#L42
    resolution: single-value-agreement
  - id: REQ-002
    status: divergence
    sources: [docs, legacy]
    contributing-claims:
      - source: docs
        claim-id: password-reset.expiry
        kind: criterion
        value: "Reset links expire after 30 minutes."
        path: docs/identity/reset.md#L7
        winner: true
      - source: legacy
        claim-id: password-reset.expiry
        kind: example
        value: "expiresAt = createdAt + 24h"
        path: src/users/reset.ts#L88
        winner: false
    resolution: authority-resolved
    resolution-trace:
      step: document-authority-ordering
      winner: docs
  - id: REQ-003
    status: unknown
    sources: []
    contributing-claims: []
    resolution: unknown-no-evidence
```

The three entries map onto the status table: REQ-001 mirrors the `model.yaml` sketch (`single-value-agreement`, no winner markers); REQ-002 is the per-kind authority case, where the documentation-class `criterion` claim beats the behaviour-class `example` claim and the loser survives with `winner: false`; and REQ-003 is an agent-declared requirement with no contributing Evidence.

### ID grammar

Every section assigns its own closed three-digit id grammar:

| Id | Grammar | Used by |
| -- | ------- | ------- |
| `REQ-NNN` | `^REQ-[0-9]{3}$` | `requirements[].id`; `satisfies[]` refs |
| `TASK-NNN` | `^TASK-[0-9]{3}$` | `tasks[].id`; `tasks[].depends-on[]` |
| `DEC-NNN` | `^DEC-[0-9]{3}$` | `technical-logic.decisions[].id` |
| `TYP-NNN` | `^TYP-[0-9]{3}$` | `domain.types[].id` |
| `OP-NNN` | `^OP-[0-9]{3}$` | `apis.surfaces[].operations[].id` |
| `CFG-NNN` | `^CFG-[0-9]{3}$` | `configuration[].id` |
| `OBS-NNN` | `^OBS-[0-9]{3}$` | `observability[].id` |

Ids are assigned in declaration order within each section, never reused across sections, and contain no holes after a single synthesis run.

### Drift validation

`specrun slice validate` adds the following findings over the typed model:

| Finding | Meaning |
| ------- | ------- |
| `slice-model-schema` | `model.yaml` does not match schema. |
| `slice-spec-provenance-stale` | Kernel-rendered provenance in `spec.md` disagrees with `model.yaml`. |
| `slice-model-provenance-drift` | `model.yaml` claims disagree with `provenance.yaml` at `(source, claim-id)`. |
| `slice-model-target-drift` | `model.yaml.target` / `.project` disagrees with `plan.yaml`. |
| `slice-model-source-orphan` | Claim references absent source key or Evidence claim id. |
| `slice-model-cross-ref-orphan` | `satisfies[]` `REQ-*` reference missing from `requirements[].id`. |
| `slice-model-claim-kind-mismatch` | Claim `kind` disagrees with Evidence (D13). |

Every synthesized slice must carry `model.yaml`.

### Build input

Target builders consume `model.yaml` for structure and provenance, and the rendered Markdown for behavioral context. `model.yaml` is authoritative for ids, status, sources, and claims; `spec.md` is authoritative for behavioral prose once it has been reviewed.

## Per-slice fan-out (D5)

Cross-target fan-out happens at the plan layer, not within a slice: each plan entry binds exactly one target adapter and an optional project ([decision log §"One plan entry, one project"](../docs/explanation/decision-log.md#one-plan-entry-one-project)). Each slice then follows the lifecycle `refining → refined → built → merged`.

Every `plan.yaml.slices[]` entry carries exactly one `target` and an optional `project`:

```yaml
slices:
  - name: identity-contracts
    status: pending
    target: contracts@v1
    project: identity-contracts
    sources:
      - key: docs
        lead: identity-api
  - name: identity-service
    status: pending
    target: omnia@v1
    project: identity-service
    depends-on: [identity-contracts]
    sources:
      - key: docs
        lead: identity-api
      - key: legacy
        lead: identity-api
```

The same `Lead` may appear in several slices' `sources[]` — that is fan-in. Plan reconciliation yields one slice per `(scope, project)` row by default, and the operator may split or merge those rows at Gate 1. Cross-slice ordering is enforced by `specrun plan next` through `depends-on`.

## Synthesis execution mode (D10)

The synthesis step carries a closed `execution: agent | tool` enum on `project.yaml`, defaulting to `agent`:

```yaml
synthesize:
  execution: agent     # or `tool`
```

- **`agent`** — the envelope is handed to the operator's agent; the engine emits a `slice.synthesize.agent` journal event, and the synthesis step is `cache: opt-out`. This is the default for cross-modal slices.
- **`tool`** — requires `synthesize.tool: { name, version }`; the envelope is piped to a WASI tool on stdin. Validation and projection are identical, and the result is cached under the Evidence sha256 set + authority-overrides + shape-brief sha256 + tool `name@version`.

Both modes validate the draft against `draft-model.schema.json`, validate the merged model against `model.schema.json`, and run the drift checks before transitioning to `refined`. When `synthesize.enforce-tool: true` is set, `execution: agent` raises `slice-synthesize-agent-mode` as a `suggestion`.

## Claim contract (D13)

The claim contract keeps every requirement traceable to its Evidence by `(source, claim-id)` and by `kind`:

- `schemas/evidence.schema.json` requires `claim-id` on every claim kind.
- `model.yaml.requirements[].claims[]` requires `kind` (mirroring `claimKind`) so the kernel can resolve per-kind authority and populate `provenance.yaml` without re-reading Evidence.
- `slice-model-claim-kind-mismatch` fires on kind drift, and `slice-model-source-orphan` fires on an absent `(source, claim-id)`.

## Wire contracts

The following are registered in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts):

- **Journal events:** `slice.synthesize.started`, `slice.synthesize.authority-resolved`, `slice.synthesize.agent`, `slice.synthesize.completed`, `slice.synthesize.failed`, `slice.model.show.requested`.
- **Validation findings:** `slice-model-schema`, `slice-spec-provenance-stale`, `slice-model-provenance-drift`, `slice-model-target-drift`, `slice-model-source-orphan`, `slice-model-cross-ref-orphan`, `slice-model-claim-kind-mismatch`, `slice-model-id-grammar`, `slice-synthesize-forbidden-input-leak` — blocking findings gate the transition at exit 2.
- **Operational validation codes:** `slice-synthesize-kernel-field-usurped`, `slice-synthesize-execution-mode-required` — these abort `specrun slice synthesize` before projection.
- **Schemas:** `schemas/slice/model.schema.json`, `schemas/slice/draft-model.schema.json`, `schemas/slice/synthesis.schema.json` — registered together so relative `$ref`s resolve. Plus the D13 `claim-id` requirement on `schemas/evidence.schema.json`.
