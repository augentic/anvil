# RFC-29c: Slice Synthesis Engine and Typed Model

> Status: Draft — Milestone **M2b** of [RFC-29](rfc-29-fan-in-fan-out.md) — Companion: [RFC-29d](rfc-29d-target.md), the target build envelope that consumes this milestone's `model.yaml`

This milestone defines how slice `Evidence` becomes a reviewed requirement set, a single schema-typed `model.yaml` (carrying provenance inline), and rendered Markdown artifacts. The rule of thumb is simple: the agent decides the requirement set and prose; the CLI owns every deterministic projection around that judgment — ids, authority resolution, status, rendered source lists, winners, inline provenance, drift checks, and wire envelopes. There is one structured artifact and one schema: the kernel re-derives its owned fields and ignores any the agent supplied (normalize, never reject), and the audit provenance view is projected on demand rather than persisted as a second file.

Read this RFC in three passes:

1. **Synthesis flow** — how `specrun slice synthesize` reads Evidence, dispatches the agent/tool step, and persists artifacts.
2. **Projection rules** — how authority, status, rendering, and provenance are derived from the agent's returned structure.
3. **Downstream contract** — what `model.yaml` contains, how one slice binds one target, and which validation/wire contracts RFC-29d consumes.

The shared RFC-29 wire-contract registry — schemas, journal events, and validation-finding codes — remains pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts).

## Decisions owned by this milestone

| Area | IDs | Decision |
| ---- | --- | -------- |
| Synthesis contract | **D3**, **D10** | Agent-led reconciliation of `Evidence[]` runs behind a closed request/response envelope. The response `model` and the persisted `model.yaml` validate against one `model.schema.json`; kernel-owned and header fields are optional, so the kernel re-derives/stamps them and ignores any the agent supplied (normalize, never reject). Synthesis is always agent-dispatched (`cache: opt-out`). |
| Projection kernel | **D8**, **D13** | The CLI derives authority, ids, status, rendered source lists, winners, rendered provenance lines, and the inline provenance carried in `model.yaml`. Shape briefs may influence only non-requirements sections. Claims are traceable by stable `(source, id, kind)`. |
| Slice output | **D4** | Every synthesized slice carries one structured artifact `.specify/slices/<slice>/model.yaml` (provenance inline) beside the Markdown artifacts; the provenance view is projected on demand by `specrun slice provenance`. |
| Planning boundary | **D5** | Each slice binds exactly one target adapter / project. Cross-target changes decompose at plan time into multiple slices joined by `depends-on`; there is no `outputs[]`. |

## Slice synthesis engine (D3)

Synthesis turns a slice's `Evidence[]` into its requirement set. Cross-modal reconciliation — deciding which requirements exist, how claims from different sources merge or split, and what each requirement means — has no deterministic function, so it stays the agent's judgment. Everything around that judgment that *can* be made deterministic is moved into CLI. The engine therefore splits into two layers: an agent-led **synthesis step** and a CLI-owned **projection kernel**.

The flow is:

1. Read the slice binding, Evidence documents, target `shape` brief, and authority overrides.
2. Dispatch a closed synthesis envelope to the operator's agent.
3. Validate the response `model` against `model.schema.json`.
4. Project kernel-owned fields (ids, status, winners, rendered source lists, inline provenance) into the single `model.yaml`.
5. Render provenance lines into `spec.md`, run drift validation, and persist the staged artifacts.

### Agent and kernel responsibilities

1. **The synthesis step (agent).** The agent reconciles source adapter `Evidence[]` into the requirement set: which requirements exist and how claims merge or split. For each requirement it records the contributing `(source, id)` claims, an `agreement` verdict (`agreed` or `disagreed`), the behavioral prose (`title`, `statement`, `scenarios[]`, `notes`), and the owning `unit`. It also authors the prose for the non-requirements model sections and the prose-only Markdown artifacts — `proposal.md`, `design.md`, `tasks.md`, and the spec bodies, the last of these written **without** `ID:` / `Sources:` / `Status:` lines.
2. **The projection kernel (CLI).** The kernel projects deterministically over whatever structure the agent returns. It stamps the `version` / `slice` / `target` / `project` header from the envelope and the slice's bound project, resolves authority (§"Authority resolution"), assigns `REQ` ids in declaration order, derives rendered source lists, winner markers, and `status` from the claims, agreement verdict, and resolved authority, writes the inline provenance (claim `value` / `path` / `winner`, `resolution`) into `model.yaml`, renders provenance lines into `spec.md`, and runs the drift validators (§"Drift validation"). It never invents, drops, or re-groups requirements; it never selects a winner the resolved authority did not; and it never overrides the agent's agreement verdict. Any kernel-owned field the agent happened to set is ignored and re-derived — the kernel normalizes, it does not reject.

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

End to end, the command resolves authority, builds the synthesis request envelope (inline Evidence plus the resolved `authority` array — the shape brief is resolved from `target`, not carried), dispatches the synthesis step, receives the response `model`, projects the kernel-owned fields (ignoring any the agent supplied), renders provenance into Markdown, validates drift, and then persists the staged artifacts.

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

The synthesis request embeds each document's `lead` and `claims` inline rather than a path to `evidence/<source>.yaml` (§"Synthesis envelope"), so the dispatched synthesis step needs no host filesystem access. Because the kernel resolves authority before dispatch, the document-level `authority` is consumed there and is not echoed into the envelope; the on-disk Evidence document remains the kernel's source of truth for projection, the provenance projection, and drift validation.

### Claim contract (D13)

The claim contract keeps every requirement traceable to its Evidence by `(source, id)` and by `kind`:

- `schemas/evidence.schema.json` requires `id` on the `requirement`, `criterion`, and `example` claim kinds (optional on other kinds).
- `model.yaml.requirements[].claims[]` requires `kind` (mirroring `claimKind`) so the kernel can resolve per-kind authority and project the inline provenance view.
- `slice-model-claim-kind-mismatch` fires on kind drift, and `slice-model-source-orphan` fires on an absent `(source, id)`.

### Synthesis dispatch (D10)

The synthesis step is always handed to the operator's agent. Cross-modal reconciliation has no deterministic function (§"Slice synthesis engine (D3)"), so there is no tool path and no cached, byte-stable synthesis variant — the engine emits a `slice.synthesize.agent` journal event and the synthesis step is `cache: opt-out`.

The returned `model` is validated against `model.schema.json`, the merged model re-validated against the same schema after projection, and the drift checks run before the slice transitions to `refined`.

### Shape-brief scope (D8)

The shape brief parameterises the **non-requirements** sections only — `domain`, `apis`, `configuration`, `technical-logic`, `observability`, and `tasks`. It MUST NOT influence `requirements[]`, claims, `agreement`, rendered source lists, or any provenance-bearing field.

The shape brief is **not** carried in the synthesis envelope. `specrun slice synthesize` resolves it from the bound `target` (`TargetAdapter::resolve`) and reads it during synthesis, keeping target resolution a CLI responsibility and the envelope free of host paths. The synthesis step is always agent-dispatched (D10), so it has the resolved brief in hand.

Two non-blocking properties uphold this (see [RFC-29d §"Acceptance proof (D7)"](rfc-29d-target.md#acceptance-proof-d7)):

1. **Envelope construction** — the requirements-relevant request inputs are byte-identical across target bindings.
2. **Kernel determinism** — given a fixed synthesis response, kernel output is byte-identical and target-independent.

These are proven as fixture properties, not policed at runtime: there is no input-leak finding. The real quality gate sits downstream at build time — replay/golden behavioral equivalence plus target-local checks (see [RFC-29d §"Target adapter responsibilities"](rfc-29d-target.md)) — which verifies the *result* rather than the agent's intermediate reasoning.

### Synthesis envelope

The synthesis step communicates with the kernel over a closed request/response envelope, dispatched to the operator's agent (D10). Its schema is `[synthesis.schema.json](rfc-29/schemas/slice/synthesis.schema.json)`, discriminated by `kind: request | response`; the response `model` `$ref`s `[model.schema.json](rfc-29/schemas/slice/model.schema.json)` — the single slice-model schema, whose kernel-owned and header fields are optional so the agent omits them.

Each `evidence.<source>` entry embeds that source's `extract` output inline — its `lead` and `claims` — rather than a path to `evidence/<source>.yaml`, so the dispatched step (agent or WASI tool) reconciles without host filesystem access. The on-disk Evidence document stays the kernel's source of truth for projection, the provenance projection, and drift validation; the per-document `authority` is **not** echoed per source, because the kernel resolves authority before dispatch and embeds the result inline as the `authority` array (§"Authority resolution"). Together with inline Evidence, the requirements-reconciliation inputs are fully self-contained in the envelope — deciding the requirement set needs no filesystem access. The non-requirements sections additionally draw on the target's shape brief, which the synthesis step resolves from `target` rather than receiving as an envelope field (§"Shape-brief scope (D8)").

The closed request shape leaves deliberate room for one future, **optional** field: a read-only `advisory-context` block (e.g. the existing baseline `spec.md` for a unit the slice re-touches, or hits from a cross-slice retrieval index). It is **deferred** ([RFC-29 Q2](rfc-29-fan-in-fan-out.md#open-questions)) and out of scope here; the contract it must honour when it lands is that it is advisory only — Evidence stays the sole producer of requirements, the advisory block never originates a requirement and never appears in provenance, and because it feeds only the already-nondeterministic agent step, the kernel-determinism property is untouched. Recording the envelope's room for it now keeps the schema forward-compatible without making it a provenance-bearing input.

A request looks like this:

```yaml
version: 1
kind: request
slice: identity-service
target: omnia@v1
evidence:
  docs:
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
  legacy:
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
authority:
  - { source: docs,   kind: requirement, class: documentation }
  - { source: docs,   kind: criterion,   class: documentation }
  - { source: legacy, kind: example,     class: behaviour }
```

Requirements reconciliation may draw only on Evidence and the resolved authority; `target` and the target's shape brief (resolved from `target`, not carried in the envelope) are valid inputs for the non-requirements model sections but never for requirements. This wall is not carried as a wire field and is not policed by a runtime finding — it is upheld by the envelope-construction and kernel-determinism properties (§"Shape-brief scope (D8)") and, decisively, by the downstream build-time ground-truth gate. The response carries the `model` plus the prose-only Markdown; the kernel-owned fields the agent simply omits are listed in §"Agent and kernel responsibilities".

The matching response carries the `model` — validated against `model.schema.json`, whose kernel-owned and header fields are optional, so the agent omits `version` / `slice` / `target` / `project` and the per-requirement `id` / `status` / `winner` — plus the prose-only Markdown under `artifacts`:

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
  domain: { types: [] }
  apis: { surfaces: [] }
  configuration: []
  technical-logic: { decisions: [] }
  observability: []
  tasks:                             # section ids other than REQ (TASK/DEC/TYP/…) are agent-authored
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

The kernel-owned fields (`requirements[].id`, `.status`, `claims[].winner`, rendered `sources`, `resolution`) are optional in the schema; if the agent supplies any of them the kernel ignores the supplied value and re-derives it (normalize, never reject). The kernel then stamps the `version` / `slice` / `target` / `project` header from the envelope and the slice's bound project, and writes the inline provenance, to produce the persisted `model.yaml` shown in §"Slice model (D4)".

### Authority resolution

Authority is resolved before dispatch and passed into the envelope inline as the `authority` array. v1 keeps the resolution surface deliberately small (decision-log §"Authority: document-level plus one override (v1)"); the resolution order is:

1. per-slice `authority-override`;
2. document-level `authority`;
3. tied effective authority → `conflict`.

The `authority` array carries one `{ source, kind, class }` row per contributing `(source, kind)` — the **effective authority class** after the walk above, nothing more. It records resolution inputs, not outcomes: winner markers and `status` are kernel-projected *after* the synthesis step returns (§"Status derivation"), so they never appear here. Keying by `(source, kind)` rather than per claim is sufficient because every claim of a given kind in one Evidence document shares that document's effective class.

The synthesis step never re-decides authority or marks winners. Once it returns the claims and `agreement` verdict, the kernel projects the winners and derives `status` plus rendered source lists from them.

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

| `claims` | `agreement`                       | Kernel `status` | inline `resolution`                         | Winner markers                |
| -------- | --------------------------------- | --------------- | ------------------------------------------- | ----------------------------- |
| 0        | *(omitted)*                       | `unknown`       | `unknown-no-evidence`                       | none                          |
| 1        | *(omitted)*                       | `agreed`        | `single-source`                             | none                          |
| ≥2       | `agreed`                          | `agreed`        | `single-value-agreement`                    | none                          |
| ≥2       | `disagreed`, unique top authority | `divergence`    | `authority-resolved` / `per-slice-override` | winner `true`, losers `false` |
| ≥2       | `disagreed`, top authority ties   | `conflict`      | `tied-conflict`                             | none                          |

`resolution` and the per-claim winner markers are written inline on the requirement in `model.yaml`. Losing claims survive there with `winner: false`.

### Persist pipeline

The kernel persists in five ordered steps; the slice transitions to `refined` only after step 5 completes cleanly:

1. Validate the response envelope and the `model` against `model.schema.json`, and reject orphan claims with `slice-model-source-orphan`.
2. Project the kernel over the response — ids, status, winners, rendered source lists, and the inline provenance (claim `value` / `path` / `winner`, `resolution`) — ignoring and re-deriving any kernel-owned field the agent supplied.
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

The normative shape is `[model.schema.json](rfc-29/schemas/slice/model.schema.json)`: a closed top level, kebab-case on disk. The persisted file always carries `version`, `slice`, `target`, `requirements`, `domain`, `apis`, `configuration`, `technical-logic`, `observability`, and `tasks` (`project` optional). The same schema validates the agent's synthesis response, where the header (`version` / `slice` / `target` / `project`) and the kernel-owned per-requirement fields are optional and omitted; the schema's `required` set is therefore the seven always-present sections, and the kernel guarantees the header on the persisted artifact.

The sketch below is illustrative (comments mark which fields the kernel owns and which the agent authors); the kernel-owned fields shown — `id`, `status`, claim `winner` / `value` / `path`, `sources`, `resolution` — carry the inline provenance:

```yaml
version: 1                 # kernel (header)
slice: identity-service    # kernel (header)
target: omnia@v1           # kernel (header)
project: identity-service  # kernel (header)
requirements:
  - id: REQ-001          # kernel
    title: Request password reset
    status: agreed       # kernel
    unit: password-reset
    agreement: agreed    # agent
    claims:              # agent authors source/id/kind; kernel projects value/path/winner
      - source: docs
        id: password-reset.request
        kind: requirement
        value: "The system lets a registered user request a password reset link by email."  # kernel
        path: docs/identity/reset.md#L4                                                     # kernel
      - source: legacy
        id: users.password-reset.request
        kind: example
        value: "POST /password-reset returns 202 and queues an email."  # kernel
        path: src/users/reset.ts#L42                                    # kernel
    sources: [docs, legacy]            # kernel (rendered source list)
    resolution: single-value-agreement # kernel (inline provenance)
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
        value: "Reset links expire after 30 minutes."  # kernel
        path: docs/identity/reset.md#L7                 # kernel
        winner: true                                    # kernel
      - source: legacy
        id: password-reset.expiry
        kind: example
        value: "expiresAt = createdAt + 24h"  # kernel
        path: src/users/reset.ts#L88          # kernel
        winner: false                         # kernel
    sources: [docs, legacy]        # kernel
    resolution: authority-resolved # kernel
    resolution-trace:              # kernel
      step: document-authority-ordering
      winner: docs
    statement: Reset links expire after 30 minutes.
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

REQ-001 is the `single-value-agreement` case (both sources agree, no winner markers); REQ-002 is the per-kind authority case, where the documentation-class `criterion` claim beats the behaviour-class `example` claim and the loser survives with `winner: false`.

### Provenance projection

There is no `provenance.yaml` file. Provenance is carried inline on each requirement in `model.yaml` (the `claims[]` with `value` / `path` / `winner`, the rendered `sources` list, `resolution`, and `resolution-trace`), so the model and its provenance can never drift from one another. The audit view operators reach for is projected from `model.yaml` on demand:

```bash
specrun slice provenance <slice> [--format json]
```

The projection reshapes the inline data into the per-requirement audit shape (`{ id, status, sources, contributing-claims, resolution, resolution-trace }`) — byte-stable given the same `model.yaml`. It is audit-only: downstream verbs read `spec.md` and `model.yaml`, never a persisted provenance file. The projected view of the sketch above lists REQ-001 (`single-value-agreement`), REQ-002 (`authority-resolved`, the documentation-class `criterion` beating the behaviour-class `example`), and any `unknown-no-evidence` requirement with empty `contributing-claims`.

### ID grammar

Every section assigns its own closed three-digit id grammar:

| Id         | Grammar           | Used by                                 |
| ---------- | ----------------- | --------------------------------------- |
| `REQ-NNN`  | `^REQ-[0-9]{3}$`  | `requirements[].id`; `satisfies[]` refs |
| `TASK-NNN` | `^TASK-[0-9]{3}$` | `tasks[].id`; `tasks[].depends-on[]`    |
| `DEC-NNN`  | `^DEC-[0-9]{3}$`  | `technical-logic.decisions[].id`        |
| `TYP-NNN`  | `^TYP-[0-9]{3}$`  | `domain.types[].id`                     |
| `OP-NNN`   | `^OP-[0-9]{3}$`   | `apis.surfaces[].operations[].id`       |
| `CFG-NNN`  | `^CFG-[0-9]{3}$`  | `configuration[].id`                    |
| `OBS-NNN`  | `^OBS-[0-9]{3}$`  | `observability[].id`                    |

Ids are assigned in declaration order within each section, never reused across sections, and contain no holes after a single synthesis run.

### Drift validation

`specrun slice validate` adds the following findings over the typed model:

| Finding                           | Meaning                                                                      |
| --------------------------------- | ---------------------------------------------------------------------------- |
| `slice-model-schema`              | `model.yaml` does not match schema.                                          |
| `slice-spec-provenance-stale`     | Kernel-rendered provenance in `spec.md` disagrees with `model.yaml`.         |
| `slice-model-target-drift`        | `model.yaml.project` disagrees with `plan.yaml`, or `model.yaml.target` disagrees with the target resolved from that bound project. |
| `slice-model-source-orphan`       | Claim references absent source key or Evidence claim id.                     |
| `slice-model-cross-ref-orphan`    | `satisfies[]` `REQ-*` reference missing from `requirements[].id`.            |
| `slice-model-claim-kind-mismatch` | Claim `kind` disagrees with Evidence (D13).                                  |

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

- **Journal events:** `slice.synthesize.started`, `slice.synthesize.authority-resolved`, `slice.synthesize.agent`, `slice.synthesize.completed`, `slice.synthesize.failed`, `slice.model.show.requested`.
- **Validation findings:** `slice-model-schema`, `slice-spec-provenance-stale`, `slice-model-target-drift`, `slice-model-source-orphan`, `slice-model-cross-ref-orphan`, `slice-model-claim-kind-mismatch`, `slice-model-id-grammar` — blocking findings gate the transition at exit 2.
- **Schemas:** `schemas/slice/model.schema.json`, `schemas/slice/synthesis.schema.json` — registered together so relative `$ref`s resolve. One `model.schema.json` validates both the agent response `model` and the persisted `model.yaml`. Plus the D13 `id` requirement on `schemas/evidence.schema.json`.

