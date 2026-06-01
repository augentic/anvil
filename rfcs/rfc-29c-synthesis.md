# RFC-29c: Slice Synthesis Engine and Typed Model

> Status: Shipped — Milestone **M2b** of [RFC-29](rfc-29-fan-in-fan-out.md); durable spec lives in [`specify-cli` `DECISIONS.md` §"Slice synthesis engine (RFC-29 M2b)"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#slice-synthesis-engine-rfc-29-m2b) and [`docs/standards/workflow.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md) — Companion: [RFC-29d](rfc-29d-target.md), the target build envelope that consumes this milestone's `model.yaml`

This milestone defines how slice `Evidence` becomes a reviewed requirement set, a single schema-typed `model.yaml` (carrying provenance inline), and rendered Markdown artifacts. The rule of thumb is simple: the agent decides the requirement set and prose; the CLI owns every deterministic projection around that judgment — ids, authority resolution, status, rendered source lists, winners, inline provenance, drift checks, and wire envelopes. There is one structured artifact and one schema: the kernel re-derives its owned fields and ignores any the agent supplied (normalize, never reject), and the audit provenance view is projected on demand rather than persisted as a second file.

Read this RFC in three passes:

1. **Synthesis flow** — how `specrun slice synthesize` reads Evidence, dispatches the agent/tool step, and persists artifacts.
2. **Projection rules** — how authority, status, rendering, and provenance are derived from the agent's returned structure.
3. **Downstream contract** — what `model.yaml` contains, how one slice binds one target, and which validation/wire contracts RFC-29d consumes.

The shared RFC-29 wire-contract registry — schemas, journal events, and validation-finding codes — remains pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts).

## Decisions owned by this milestone

| Area | IDs | Decision |
| ---- | --- | -------- |
| Synthesis contract | **D3**, **D10** | Agent-led reconciliation of `Evidence[]` is always agent-dispatched (`cache: opt-out`); since there is no tool consumer, the CLI assembles the step's inputs directly and schema-validates only the returned response — there is no closed *request* wire shape. The response `model` and the persisted `model.yaml` validate against one `model.schema.json`; kernel-owned and header fields are optional, so the kernel re-derives/stamps them and ignores any the agent supplied (normalize, never reject). |
| Projection kernel | **D8**, **D13** | The CLI derives authority, ids, status, rendered source lists, winners, rendered provenance lines, and the inline provenance carried in `model.yaml`. Shape briefs may influence only non-requirements sections. Claims are traceable by stable `(source, id, kind)`. |
| Slice output | **D4** | Every synthesized slice carries one structured artifact `.specify/slices/<slice>/model.yaml` (provenance inline) beside the Markdown artifacts; the provenance view is projected on demand by `specrun slice provenance`. |
| Planning boundary | **D5** | Each slice binds exactly one target adapter / project. Cross-target changes decompose at plan time into multiple slices joined by `depends-on`; there is no `outputs[]`. |

## Slice synthesis engine (D3)

Synthesis turns a slice's `Evidence[]` into its requirement set. Cross-modal reconciliation — deciding which requirements exist, how claims from different sources merge or split, and what each requirement means — has no deterministic function, so it stays the agent's judgment. Everything around that judgment that *can* be made deterministic is moved into CLI. The engine therefore splits into two layers: an agent-led **synthesis step** and a CLI-owned **projection kernel**.

The flow is:

1. Read the slice binding, Evidence documents, and target `shape` brief.
2. Dispatch the synthesis step (inline Evidence plus the resolved shape brief) to the operator's agent.
3. Validate the response against `synthesis.schema.json` (and its `model` against `model.schema.json`).
4. Resolve authority from on-disk Evidence and any per-slice override, then project kernel-owned fields (ids, status, winners, rendered source lists, inline provenance) into the single `model.yaml`.
5. Render provenance lines into `spec.md`, run drift validation, and persist the staged artifacts.

### Agent and kernel responsibilities

1. **The synthesis step (agent).** The agent reconciles source adapter `Evidence[]` into the requirement set: which requirements exist and how claims merge or split. For each requirement it records the contributing `(source, id)` claims, an `agreement` verdict (`agreed` or `disagreed`), the behavioral prose (`title`, `statement`, `scenarios[]`, `notes`), and the owning `unit`. It also authors the prose for the non-requirements model sections and the prose-only Markdown artifacts — `proposal.md`, `design.md`, `tasks.md`, and the spec bodies, the last of these written **without** `ID:` / `Sources:` / `Status:` lines.
2. **The projection kernel (CLI).** The kernel projects deterministically over whatever structure the agent returns. It stamps the `version` / `slice` / `project` header from the slice and its bound project, resolves authority (§"Authority resolution"), assigns `REQ` ids in declaration order, derives rendered source lists, winner markers, and `status` from the claims, agreement verdict, and resolved authority, writes the inline provenance (claim `winner` markers) into `model.yaml`, renders provenance lines into `spec.md`, and runs the drift validators (§"Drift validation"). It never invents, drops, or re-groups requirements; it never selects a winner the resolved authority did not; and it never overrides the agent's agreement verdict. Any kernel-owned field the agent happened to set is ignored and re-derived — the kernel normalizes, it does not reject.

### Command

```bash
specrun slice synthesize <slice> [--format json]
```

The command **reads** the slice metadata and target binding, `plan.yaml.slices[].sources`, the per-source `evidence/*.yaml`, the bound target's `shape` brief, and any operator override fields (`authority-override`).

It **writes** the following artifacts. All writes are staged, and the prior artifacts are kept intact on failure:

```text
.specify/slices/<slice>/proposal.md
.specify/slices/<slice>/specs/<unit>/spec.md
.specify/slices/<slice>/design.md
.specify/slices/<slice>/tasks.md
.specify/slices/<slice>/model.yaml
```

There is no `provenance.yaml` write — provenance is carried inline in `model.yaml` and projected on demand by `specrun slice provenance` (§"Provenance projection").

End to end, the command resolves the slice's `target` from its bound project and reads the shape brief, dispatches the synthesis step (inline Evidence plus that brief), receives the response, resolves authority from the on-disk Evidence and any per-slice override, projects the kernel-owned fields (ignoring any the agent supplied), renders provenance into Markdown, validates drift, and then persists the staged artifacts. It does not persist `target` into `model.yaml` — `target` is resolved on demand the same way `plan.yaml` resolves it.

### Evidence input

Each bound source contributes one Evidence document at `evidence/<source>.yaml` — the per-source result of `extract`, whose normative shape is `schemas/evidence.schema.json` (distributed with the CLI, not under `rfc-29/schemas/`). The document's `(slice, source)` identity is carried by its on-disk path (the slice directory plus the `<source>.yaml` filename) and its adapter resolves from `plan.yaml.sources.<source>.adapter`, so neither is duplicated in-document — the top-level keys are just `authority`, `lead`, and `claims` (the per-Evidence `authority-overrides` key is deferred — see §"Authority resolution"). Every claim carries a stable `id` and a closed `kind`; per-kind body fields (`statement`, `criterion`, `replay-digest`, …) are open. The two documents the `identity-service` envelope binds below are:

```yaml
# evidence/docs.yaml
authority: documentation
lead: password-reset
claims:
  - id: password-reset.request
    kind: requirement
    statement: The system lets a registered user request a password reset link by email.
    path: docs/identity/reset.md#L4
  - id: password-reset.expiry
    kind: criterion
    criterion: Reset links expire after 30 minutes.
    path: docs/identity/reset.md#L7
```

```yaml
# evidence/legacy.yaml
authority: behaviour
lead: password-reset
claims:
  - id: users.password-reset.request
    kind: example
    replay-digest: sha256:9f2b…
    output: "POST /password-reset returns 202 and queues an email."
    path: src/users/reset.ts#L42
  - id: password-reset.expiry
    kind: example
    output: "expiresAt = createdAt + 24h"
    path: src/users/reset.ts#L88
```

The kernel resolves authority per claim from these documents' `authority` fields (and any per-slice `authority-override`); the synthesis step then reconciles the claims across both into the requirement set. These are exactly the claims REQ-001 and REQ-002 cite in the `model.yaml` sketch and the projected provenance view below.

The synthesis step is given each document's `lead` and `claims` inline rather than a path to `evidence/<source>.yaml` (§"Synthesis response"), so it needs no host filesystem access. The kernel resolves authority *after* the response returns, reading the document-level `authority` and any per-slice override from disk; the on-disk Evidence document remains the kernel's source of truth for projection, the provenance projection, and drift validation.

### Claim contract (D13)

The claim contract keeps every requirement traceable to its Evidence by `(source, id)` and by `kind`:

- `schemas/evidence.schema.json` requires `id` on the `requirement`, `criterion`, and `example` claim kinds (optional on other kinds).
- `model.yaml.requirements[].claims[]` requires `kind` (mirroring `claimKind`) so the kernel can resolve per-kind authority and project the inline provenance view.
- `slice-model-claim-kind-mismatch` fires on kind drift, and `slice-model-source-orphan` fires on an absent `(source, id)`.

### Synthesis dispatch (D10)

The synthesis step is always handed to the operator's agent. Cross-modal reconciliation has no deterministic function (§"Slice synthesis engine (D3)"), so there is no tool path and no cached, byte-stable synthesis variant — the engine emits a `slice.synthesize.agent` journal event and the synthesis step is `cache: opt-out`.

The returned `model` is validated against `model.schema.json`, the merged model re-validated against the same schema after projection, and the drift checks run before the slice transitions to `refined`.

### Shape-brief scope (D8)

The shape brief parameterises the **non-requirements** sections only — today that is `tasks` (the `domain` / `apis` / `configuration` / `technical-logic` / `observability` sub-trees are deferred until a consumer earns them; see §"Slice model (D4)"). It MUST NOT influence `requirements[]`, claims, `agreement`, rendered source lists, or any provenance-bearing field.

The shape brief is **not** a wire-schema field. `specrun slice synthesize` resolves the bound `target` (`TargetAdapter::resolve`), reads its shape brief, and provides it to the agent-dispatched synthesis step (D10) — keeping target resolution a CLI responsibility. The brief informs only the non-requirements sections; the requirements wall is upheld by the kernel-determinism property below, not by withholding the brief from the step.

One non-blocking property upholds this (see [RFC-29d §"Acceptance proof (D7)"](rfc-29d-target.md#acceptance-proof-d7)):

- **Kernel determinism** — given a fixed synthesis response, kernel output is byte-identical and target-independent.

Target-neutrality of the requirements-relevant inputs is a by-construction property of the kernel (the shape brief is resolved from `target` and feeds only the non-requirements sections), so it needs no separate cross-target fixture — D5 binds one slice to one target, so a single slice is never synthesized against two targets in production anyway. The property is proven as a fixture, not policed at runtime: there is no input-leak finding. The real quality gate sits downstream at build time — replay/golden behavioral equivalence plus target-local checks (see [RFC-29d §"Target adapter responsibilities"](rfc-29d-target.md)) — which verifies the *result* rather than the agent's intermediate reasoning.

### Synthesis response

Synthesis is always agent-dispatched (D10): there is no tool consumer, so there is no closed *request* wire shape to honour. `specrun slice synthesize` assembles the synthesis step's inputs directly — the slice's inline Evidence (each source's `lead` and `claims`, embedded inline rather than as a path to `evidence/<source>.yaml`, so the step needs no host filesystem access) plus the bound target's shape brief, which the CLI resolves from `target` and provides to the step (§"Shape-brief scope (D8)"). Requirements reconciliation draws solely on Evidence; authority is **not** passed to the step — the kernel resolves it after the response returns, from the on-disk Evidence and any per-slice override (§"Authority resolution"). The on-disk Evidence document stays the kernel's source of truth for projection, the provenance projection, and drift validation.

The single schema-validated wire is therefore the **response**. Its schema is `[synthesis.schema.json](rfc-29/schemas/slice/synthesis.schema.json)` (`kind: response`); the response `model` `$ref`s `[model.schema.json](rfc-29/schemas/slice/model.schema.json)` — the single slice-model schema, whose kernel-owned and header fields are optional so the agent omits them. The two schemas are registered together so the relative `$ref` resolves.

The synthesis inputs leave deliberate room for one future, **optional** read-only `advisory-context` input (e.g. the existing baseline `spec.md` for a unit the slice re-touches, or hits from a cross-slice retrieval index). It is **deferred** ([RFC-29 Q2](rfc-29-fan-in-fan-out.md#open-questions)) and out of scope here; the contract it must honour when it lands is that it is advisory only — Evidence stays the sole producer of requirements, the advisory block never originates a requirement and never appears in provenance, and because it feeds only the already-nondeterministic agent step, the kernel-determinism property is untouched.

The response carries the `model` — validated against `model.schema.json`, whose kernel-owned and header fields are optional, so the agent omits `version` / `slice` / `project` and the per-requirement `id` / `status` / `winner` — plus the prose-only Markdown under `artifacts`:

```yaml
kind: response
version: 1
slice: identity-service
model:                              # model.schema.json — agent omits kernel-owned + header fields; kernel stamps them
  requirements:
    - title: Request password reset    # no id / status / winner — kernel projects those
      unit: password-reset
      agreement: agreed
      claims:
        - { source: docs,   id: password-reset.request,       kind: requirement }
        - { source: legacy, id: users.password-reset.request, kind: example }
      statement: The system lets a registered user request a password reset link by email.
      scenarios:
        - Given a registered email, when the user requests a reset, then the system accepts it.
  tasks:                             # `TASK` ids are agent-authored; `REQ` ids are kernel-projected
    - id: TASK-001
      text: Implement password reset request handling.
      satisfies: [REQ-001]
artifacts:
  proposal: "# Password reset\n…"
  design:   "# Design\n…"
  tasks:    "# Tasks\n- [ ] TASK-001 …"
  specs:
    - unit: password-reset
      content: "## Request password reset\nThe system lets a registered user…"  # no ID:/Sources:/Status: lines
```

The kernel-owned fields (`requirements[].id`, `.status`, `claims[].winner`, rendered `sources`) are optional in the schema; if the agent supplies any of them the kernel ignores the supplied value and re-derives it (normalize, never reject). The kernel then resolves authority from the on-disk Evidence, stamps the `version` / `slice` / `project` header from the slice's bound project, and writes the inline provenance, to produce the persisted `model.yaml` shown in §"Slice model (D4)".

### Authority resolution

Authority is resolved by the kernel **after** the synthesis response returns — it is never passed to the agent step, which decides the `agreement` verdict from Evidence semantics alone. The kernel reads the resolution inputs (document-level `authority` and any per-slice override) from disk. v1 keeps the resolution surface deliberately small (decision-log §"Authority: document-level plus one override (v1)"); the resolution order is:

1. per-slice `authority-override`;
2. document-level `authority`;
3. tied effective authority → `conflict`.

Authority is keyed by `(source, kind)`: every claim of a given kind in one Evidence document shares that document's effective class, so the kernel resolves each contributing `(source, kind)` once. Winner markers and `status` are then projected from the resolved authority plus the agent's `agreement` verdict (§"Status derivation"); they are not a separate persisted resolution input.

The synthesis step never re-decides authority or marks winners. Once it returns the claims and `agreement` verdict, the kernel resolves authority, projects the winners, and derives `status` plus rendered source lists from them.

**Per-claim resolution (mixed kinds).** Authority is keyed by `ClaimKind`, so a single requirement can mix claim kinds. For each claim `(source, id, kind)` the kernel walks the same order — per-slice `authority-override[kind]` → document-level `authority` → the default `intent > documentation > behaviour`. Among `disagreed` claims, the winner is the strictly-greatest effective class; a tie at the top class yields `conflict` with no winner markers.

The single operator override surface is the per-slice `authority-override` on `plan.yaml`, keyed by `ClaimKind`. It maps a kind directly to the winning **source key** (authored via `specrun plan amend --authority-override`):

```yaml
# plan.yaml.slices[] — per-slice override: force a source to win for a kind
authority-override:
  criterion: docs        # `docs` wins every `criterion` claim in this slice
```

> **Deferred (future RFC).** A per-Evidence `authority-overrides` surface that lifts a kind's **authority class** for one Evidence document — and the finer-grained class-lifting precedence it implies — is out of scope for v1. Document-level `authority` plus the single per-slice override covers the common case; the per-Evidence surface can be earned back when a real slice needs it.

### Status derivation

The kernel derives each requirement's `status` from the claim count, the agent's `agreement` verdict, and the resolved authority. Agreement classification is the agent's; winner selection among disagreements is the kernel's.

| `claims` | `agreement`                       | Kernel `status` | Winner markers                |
| -------- | --------------------------------- | --------------- | ----------------------------- |
| 0        | *(omitted)*                       | `unknown`       | none                          |
| 1        | *(omitted)*                       | `agreed`        | none                          |
| ≥2       | `agreed`                          | `agreed`        | none                          |
| ≥2       | `disagreed`, unique top authority | `divergence`    | winner `true`, losers `false` |
| ≥2       | `disagreed`, top authority ties   | `conflict`      | none                          |

`status` and the per-claim winner markers are the only outcome fields written inline on the requirement in `model.yaml`; losing claims survive there with `winner: false`. The finer-grained `resolution` label (`single-source`, `single-value-agreement`, `authority-resolved`, `per-slice-override`, `unknown-no-evidence`, `tied-conflict`) is **not persisted** — it is a deterministic function of the same three inputs and is recomputed on demand by `specrun slice provenance` (§"Provenance projection").

### Persist pipeline

The kernel persists in five ordered steps; the slice transitions to `refined` only after step 5 completes cleanly:

1. Validate the response against `synthesis.schema.json` and its `model` against `model.schema.json`, and reject orphan claims with `slice-model-source-orphan`.
2. Resolve authority from the on-disk Evidence and any per-slice override, then project the kernel over the response — ids, status, winners, rendered source lists, and the inline provenance (per-claim `winner` markers) — ignoring and re-deriving any kernel-owned field the agent supplied.
3. Re-validate the merged `model.yaml` against `model.schema.json`.
4. Render the kernel-owned provenance lines into `spec.md` (§"Rendering").
5. Run the drift validators and persist if clean.

### Rendering

Synthesis output is rendered in three phases, each with a single owner:

| Phase             | Author | Output                                                                               |
| ----------------- | ------ | ------------------------------------------------------------------------------------ |
| Synthesis step    | Agent  | Response `model` (no kernel-owned/header fields) + prose-only Markdown                |
| Projection kernel | CLI    | Full `model.yaml` (provenance inline)                                                |
| Render step       | CLI    | Injects `ID:` / `Sources:` / `Status:` (and status tags) into `specs/<unit>/spec.md` |

A rendered `specs/password-reset/spec.md` block looks like this — the agent authors the heading and body, and the render step injects the three provenance lines from `model.yaml`:

```markdown
## Request password reset

ID: REQ-001
Sources: docs, legacy
Status: agreed

The system lets a registered user request a password reset link by email.
```

`spec.md` remains the behavioral review and merge input, and its provenance lines are rendered from `model.yaml`. Hand-editing a kernel-rendered provenance line without re-synthesising raises `slice-spec-provenance-stale`. The provenance view is audit-only and always projected from `model.yaml` on demand (§"Provenance projection"). Re-synthesis overwrites `model.yaml` and the kernel-rendered provenance lines; operator prose outside those lines survives until the agent returns different bodies.

## Slice model (D4)

Every synthesized slice carries a machine-readable `model.yaml` alongside its Markdown artifacts. The Markdown stays the human review surface; `model.yaml` is the schema-pinned view that target builders consume.

### File

```text
.specify/slices/<slice>/model.yaml
```

The file is generated whole by `specrun slice synthesize`. Operators edit `spec.md` and `design.md`, never `model.yaml` directly.

### Shape

The normative shape is `[model.schema.json](rfc-29/schemas/slice/model.schema.json)`: a closed top level, kebab-case on disk. The persisted file always carries `version`, `slice`, `requirements`, and `tasks` (`project` optional). The top level stays closed (`additionalProperties: false`) so the deferred non-requirements sub-trees (`domain` / `apis` / `configuration` / `technical-logic` / `observability`) can be re-added cleanly once an `execution: tool` target or a contract gate names the sub-tree it consumes; until then the model carries only the earned core. The same schema validates the agent's synthesis response, where the header (`version` / `slice` / `project`) and the kernel-owned per-requirement fields are optional and omitted; the schema's `required` set is therefore `requirements` and `tasks`, and the kernel guarantees the header on the persisted artifact. `target` is **not** a `model.yaml` field — it is resolved on demand from the bound project.

The sketch below is illustrative (comments mark which fields the kernel owns and which the agent authors); the kernel-owned fields shown — `id`, `status`, claim `winner`, `sources` — carry the inline provenance:

```yaml
version: 1                 # kernel (header)
slice: identity-service    # kernel (header)
project: identity-service  # kernel (header)
requirements:
  - id: REQ-001          # kernel
    title: Request password reset
    status: agreed       # kernel
    unit: password-reset
    agreement: agreed    # agent
    claims:              # agent authors source/id/kind; kernel projects winner
      - source: docs
        id: password-reset.request
        kind: requirement
      - source: legacy
        id: users.password-reset.request
        kind: example
    sources: [docs, legacy]            # kernel (rendered source list)
    statement: The system lets a registered user request a password reset link by email.
    scenarios:
      - Given REQ-001 and a registered email, when the user requests a reset, then the system accepts the request.
  - id: REQ-002          # kernel
    title: Reset link expiry
    status: divergence   # kernel
    unit: password-reset
    agreement: disagreed # agent
    claims:
      - source: docs
        id: password-reset.expiry
        kind: criterion
        winner: true                                    # kernel
      - source: legacy
        id: password-reset.expiry
        kind: example
        winner: false                         # kernel
    sources: [docs, legacy]        # kernel
    statement: Reset links expire after 30 minutes.
tasks:
  - id: TASK-001
    text: Implement password reset request handling.
    satisfies: [REQ-001]
```

REQ-001 is the agreement case (both sources agree, `status: agreed`, no winner markers); REQ-002 is the per-kind authority case (`status: divergence`), where the documentation-class `criterion` claim beats the behaviour-class `example` claim and the loser survives with `winner: false`. The finer `resolution` label (`single-value-agreement` for REQ-001, `authority-resolved` for REQ-002) and the per-claim `value` / `path` are not persisted here — the provenance projection recomputes the label and reads `value` / `path` from on-disk Evidence on demand.

### Provenance projection

There is no `provenance.yaml` file. The load-bearing provenance is carried inline on each requirement in `model.yaml` (the `claims[]` with their `winner` markers, the rendered `sources` list, and `status`), so the model and its provenance can never drift from one another. The audit view operators reach for is projected from `model.yaml` plus on-disk Evidence on demand:

```bash
specrun slice provenance <slice> [--format json]
```

The projection reshapes the inline data into the per-requirement audit shape (`{ id, status, sources, contributing-claims, resolution, resolution-trace }`) — byte-stable given the same `model.yaml` and Evidence. The two derived fields are **recomputed**, not read from `model.yaml`:

- `resolution` (and the optional `resolution-trace`) is a deterministic function of the claim count, the per-claim `winner` markers, and the resolved authority — the same inputs §"Status derivation" uses — so the projection recomputes it (`single-source`, `single-value-agreement`, `authority-resolved`, `per-slice-override`, `unknown-no-evidence`, `tied-conflict`) rather than persisting a third encoding of the winner.
- each contributing claim's `value` (first-line payload) and `path` anchor are read from `evidence/<source>.yaml` keyed by the `(source, id)` the claim already carries — the same Evidence the projection already reads for `slice-model-source-orphan` and `slice-model-claim-kind-mismatch`.

It is audit-only: downstream verbs read `spec.md` and `model.yaml`, never a persisted provenance file. The projected view of the sketch above lists REQ-001 (`single-value-agreement`), REQ-002 (`authority-resolved`, the documentation-class `criterion` beating the behaviour-class `example`), and any `unknown` requirement with empty `contributing-claims`.

### ID grammar

Each section assigns its own closed three-digit id grammar. The model carries only the two earned sections today; the `DEC` / `TYP` / `OP` / `CFG` / `OBS` grammars are deferred with their sub-trees (§"Slice model (D4)") and re-added when a consumer earns them:

| Id         | Grammar           | Used by                                 |
| ---------- | ----------------- | --------------------------------------- |
| `REQ-NNN`  | `^REQ-[0-9]{3}$`  | `requirements[].id`; `satisfies[]` refs |
| `TASK-NNN` | `^TASK-[0-9]{3}$` | `tasks[].id`; `tasks[].depends-on[]`    |

Ids are assigned in declaration order within each section, never reused across sections, and contain no holes after a single synthesis run.

### Drift validation

`specrun slice validate` adds the following findings over the typed model:

| Finding                           | Meaning                                                                      |
| --------------------------------- | ---------------------------------------------------------------------------- |
| `slice-model-schema`              | `model.yaml` does not match schema.                                          |
| `slice-spec-provenance-stale`     | Kernel-rendered provenance in `spec.md` disagrees with `model.yaml`.         |
| `slice-model-target-drift`        | `model.yaml.project` disagrees with `plan.yaml.slices[<slice>].project`. (`target` is not persisted, so there is no target-vs-resolved-target half.) |
| `slice-model-source-orphan`       | Claim references absent source key or Evidence claim id.                     |
| `slice-model-cross-ref-orphan`    | `satisfies[]` `REQ-*` reference missing from `requirements[].id`.            |
| `slice-model-claim-kind-mismatch` | Claim `kind` disagrees with Evidence (D13).                                  |
| `slice-model-id-grammar`          | A `REQ` or `TASK` id does not match its closed three-digit grammar.          |

There is no `model.yaml`-vs-`provenance.yaml` drift finding: provenance is inline in `model.yaml`, so the two representations cannot disagree.

Every synthesized slice must carry `model.yaml`.

### Build input

Target builders consume `model.yaml` for structure and provenance, and the rendered Markdown for behavioral context. `model.yaml` is authoritative for ids, status, and claim provenance; `spec.md` is authoritative for behavioral prose once it has been reviewed.

## Per-slice fan-out (D5)

Cross-target fan-out happens at the plan layer, not within a slice: each plan entry binds one project, and that project resolves to exactly one target adapter ([decision log §"One plan entry, one project"](../docs/explanation/decision-log.md#one-plan-entry-one-project)). Each slice then follows the lifecycle `refining → refined → built → merged`.

Every `plan.yaml.slices[]` entry binds a `project` (optional on disk — an omitted value resolves to the sole topology project); the target adapter is resolved on demand from that project and is not stored per slice:

```yaml
slices:
  - name: identity-contracts
    status: pending
    project: identity-contracts
    sources:
      - source: docs
        lead: identity-api
  - name: identity-service
    status: pending
    project: identity-service
    depends-on: [identity-contracts]
    sources:
      - source: docs
        lead: identity-api
      - source: legacy
        lead: identity-api
```

The same `Lead` may appear in several slices' `sources[]` — that is fan-in. Plan reconciliation yields one slice per `(scope, project)` row by default, and the operator may split or merge those rows at Gate 1. Cross-slice ordering is enforced by `specrun plan next` through `depends-on`.

## Wire contracts

The following are registered in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts):

- **Journal events:** `slice.synthesize.started`, `slice.synthesize.agent`, `slice.synthesize.completed`, `slice.synthesize.failed`.
- **Validation findings:** `slice-model-schema`, `slice-spec-provenance-stale`, `slice-model-target-drift`, `slice-model-source-orphan`, `slice-model-cross-ref-orphan`, `slice-model-claim-kind-mismatch`, `slice-model-id-grammar` — blocking findings gate the transition at exit 2.
- **Schemas:** `schemas/slice/model.schema.json`, `schemas/slice/synthesis.schema.json` — registered together so relative `$ref`s resolve. One `model.schema.json` validates both the agent response `model` and the persisted `model.yaml`. Plus the D13 `id` requirement on `schemas/evidence.schema.json`.

