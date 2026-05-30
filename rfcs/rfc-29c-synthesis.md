# RFC-29c: Slice Synthesis Engine and Typed Model

> Status: Draft — Milestone **M2b** of [RFC-29](rfc-29-fan-in-fan-out.md) — Depends: [RFC-29a](rfc-29a-source-operations.md) (consumes its surveyed/extracted Evidence), [RFC-29b](rfc-29b-lead-reconciliation.md) (consumes its plan rows) — Unblocks: RM-11 machine-readable producer/consumer impact; the M3 build input ([RFC-29d](rfc-29d-target-build-envelope.md))

This is the third independently shippable milestone of [RFC-29](rfc-29-fan-in-fan-out.md). Slice synthesis, the draft/persisted model split, kernel rendering into `spec.md`, and the drift validators form one contract over the Evidence the agent already produces; it consumes M1's surveys/Evidence and M2a's plan rows but not the build envelope. It owns the slice-time judgment/projection split, the typed `model.yaml`, the claim contract (`id` + `kind`), and the confirmation that per-slice fan-out keeps its one-target shape (D5).

The cross-milestone wire contracts this milestone appends to are pinned in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This document is the source of truth for D3, D3a, D4, D5, D8, D10, D11, and D13.

## Decisions owned by this milestone

| ID | Decision |
| -- | -------- |
| **D3 Slice synthesis engine** | Agent-led cross-modal reconciliation of `Evidence[]` into the requirement set; CLI owns the synthesis envelope and the projection kernel over that structure. |
| **D3a Draft vs persisted model** | Synthesis response validates against `draft-model.schema.json`; persisted `model.yaml` validates against `model.schema.json`. |
| **D4 Typed slice model** | Every synthesized slice carries `.specify/slices/<slice>/model.yaml`. |
| **D5 Per-slice fan-out** | Each slice binds exactly one target adapter / project; cross-target changes decompose at plan time into multiple slices joined by `depends-on`. No `outputs[]`. |
| **D8 Shape-brief scope** | Target `shape` briefs parameterise non-requirements model sections only. |
| **D10 Synthesis execution mode** | The synthesis step carries a closed `execution: agent \| tool` enum; agent-first by design. |
| **D11 Standalone provenance projection** | `specrun slice provenance <slice>` is the standalone entry point onto the same projection kernel as D3. |
| **D13 Claim contract (`id` + `kind`)** | Every contributing claim carries a stable `claim-id` and its `kind`; `model.yaml` claims carry `kind` so the kernel resolves per-kind authority. |

## Slice synthesis engine (D3)

### Two layers

There is no deterministic function from `(design-prose, code-AST, vision-output)` to a coherent requirement set, so the engine splits cleanly into two layers — with the judgment layer first:

1. **Synthesis step (judgment, agent-led by default — the heart).** Cross-modal reconciliation of `Evidence[]` into the requirement set: deciding which requirements exist and how claims merge or split into them, declaring each requirement's `(source, claim-id)` claims and an `agreement` verdict (`agreed` when the contributors agree on value after semantic comparison, `disagreed` when they do not — the irreducibly-judgment call), authoring `requirements[].title` / `.statement` / `.scenarios[]` / `.notes`, recording which spec `unit` each requirement renders into, populating the prose fields of the rest of the model (`domain.types[].fields[].description`, `apis.surfaces[].operations[]` request/response/errors prose, `technical-logic.decisions[].statement` / `.rationale`, `observability[].description`, `tasks[].text`), and authoring **prose-only** Markdown drafts (`proposal.md`, `design.md`, `tasks.md`, and spec requirement bodies **without** `ID:` / `Sources:` / `Status:` lines). This is the load-bearing judgment of synthesis and stays with the agent.
2. **Projection kernel (deterministic projection, CLI-owned).** RFC-27 authority resolution, REQ-id assignment in the agent's declaration order, derivation of each requirement's `sources` (the unique source keys of its claims), winner-marker and `status` derivation from the agreement verdict plus authority over those claims, claim-level provenance projection into `provenance.yaml`, **kernel rendering of provenance lines into `spec.md`**, and drift validators in §"Drift validation". This is where RFC-27's authority resolver becomes production code; it projects over the structure the agent returns and never invents, drops, or re-groups requirements, never selects a winner the resolved authority did not, and never overrides the agent's agreement verdict.

The engine resolves authority, runs (1) under the envelope defined in §"Synthesis envelope", then runs (2) over the returned structure. Persist order:

1. Validate the agent response envelope and draft `model` against `draft-model.schema.json`.
2. Reject usurped kernel fields (`generated-at`, `generator`, top-level `sources`, `requirements[].id`, `.status`, `.sources`, `claims[].winner`) with `slice-synthesize-kernel-field-usurped`; reject orphan claims with `slice-model-source-orphan`.
3. Project the kernel over the draft (ids, sources, status, winners, top-level `sources`, `generated-at`, `generator`, `provenance.yaml`).
4. Validate the merged `model.yaml` against `model.schema.json`.
5. Render kernel-owned provenance lines into `spec.md` from the projected model (§"Rendering").
6. Run drift validators and persist if clean.

The slice transitions to `refined` only after step 6 succeeds.

### Command

```bash
specrun slice synthesize <slice> [--format json]
```

The command reads:

- slice metadata and target binding;
- `plan.yaml.slices[].sources`;
- `evidence/*.yaml`;
- the bound target's `shape` brief;
- prior baseline specs when available;
- operator-authored override fields such as `authority-override`.

It produces, in order:

1. Resolved authority — per-Evidence, per-claim effective authority from `(Evidence[], authority-overrides, prior baseline)` via the RFC-27 resolver. No requirement skeleton is pre-computed; the requirement set is the synthesis step's to decide.
2. A synthesis request envelope (see below) handed to the synthesis step, carrying the Evidence map and resolved authority but **not** the bound target's `shape` brief for any requirements input.
3. After the synthesis step returns its draft model (claims, agreement verdicts, prose) and prose-only Markdown artifacts, the kernel runs the persist pipeline in §"Two layers" above.

```text
.specify/slices/<slice>/proposal.md
.specify/slices/<slice>/specs/<unit>/spec.md
.specify/slices/<slice>/design.md
.specify/slices/<slice>/tasks.md
.specify/slices/<slice>/provenance.yaml
.specify/slices/<slice>/model.yaml
```

The write is staged and validated before the slice transitions to `refined`. If any artifact fails schema validation or drift validation, the command exits non-zero and leaves the prior artifact set intact where possible.

### Production authority resolver

RFC-27's authority order becomes production code inside the projection kernel:

1. per-slice `authority-override`;
2. per-Evidence `authority-overrides`;
3. document-level `authority`;
4. tied effective authority -> `conflict`.

The micro-resolver currently pinned in tests becomes black-box coverage for the production resolver. The kernel resolves authority **before** dispatch and passes the resolution into the envelope, so the synthesis step knows which claims win when it decides what each requirement draws on; the step never re-decides authority and never marks winners itself. After the step returns its requirement set — each requirement carrying its `(source, claim-id)` claims and an `agreement` verdict — the kernel projects the resolution over those claims: it marks the winning and losing claims, derives `status` (an `agreed` verdict yields `agreed`; a `disagreed` verdict is resolved by authority into `divergence` with a unique winner, or `conflict` when the top authority class ties), derives the `sources` set, and writes `provenance.yaml`.

### Status and provenance derivation

`status` joins two responsibilities: **agreement classification** is judgment (does "30 minutes" agree with "1800 seconds"? semantic, agent-owned), while **winner selection among disagreements** is authority mechanics (deterministic, kernel-owned). The synthesis step supplies the agreement verdict per requirement; the kernel applies the resolved authority and projects the rest. The mapping is closed:

| `claims` | `agreement` | Kernel `status` | `provenance.yaml` `resolution` | Winner markers |
| --- | --- | --- | --- | --- |
| 0 | _(omitted)_ | `unknown` | `unknown-no-evidence` | none |
| 1 | _(omitted)_ | `agreed` | `single-source` | none |
| ≥2 | `agreed` | `agreed` | `single-value-agreement` | none |
| ≥2 | `disagreed`, unique top authority (or operator override) | `divergence` | `authority-resolved` / `per-slice-override` | kernel marks the authority-resolved winner `true`, every loser `false` |
| ≥2 | `disagreed`, top authority class ties | `conflict` | `tied-conflict` | none |

The agent never names the winning claim; the kernel selects it from the RFC-27 resolution it computed before dispatch, so authority resolution stays the single source of truth (RFC-27) and an agent cannot smuggle in a winner the authority order forbids. The losing claims survive in `provenance.yaml` (`winner: false`) for audit, exactly as [`provenance.md`](../../plugins/spec/references/synthesis/provenance.md) specifies.

The agent's agreement verdict is authoritative for `agreed`-vs-not — value agreement is semantic and the CLI does not adjudicate it. As a non-blocking guard, the kernel runs a cheap normalised-string inequality check over the claims of any requirement the agent marked `agreed`; a mismatch emits a `review`-kind finding (`slice-synthesize-agreement-suspect`, advisory, never a transition blocker) so an operator can eyeball a possible mislabel without the CLI re-litigating semantics.

### Authority over mixed-kind claims

RFC-27 authority is resolved **per claim kind**: the per-Evidence `authority-overrides` map is keyed by `ClaimKind`, and the per-slice `authority-override <kind>=<key>` map likewise pins a class for one kind. A single requirement, however, may draw on claims of **different kinds** (e.g. a `criterion` from `docs` disagreeing with an `example` from `legacy`). The kernel therefore resolves authority at the **claim** granularity, not by assuming one kind per requirement. Because each `model.yaml` claim now carries its own `kind` (D13), the reduction is well-defined and needs no re-read of Evidence to recover kinds:

1. **Per-claim effective class.** For each claim `(source, claim-id, kind)` the kernel computes the effective authority class by the RFC-27 order: per-slice `authority-override[kind]` (if it names this claim's source) → the claim's Evidence `authority-overrides[kind]` → the Evidence document-level `authority` → the workflow default `intent > documentation > behaviour`.
2. **Requirement winner.** Among a `disagreed` requirement's claims, the winner is the claim with the strictly-greatest effective class. The kernel marks it `winner: true` and every other claim `winner: false`, yielding `status: divergence` and `resolution: authority-resolved` (or `per-slice-override` when step 1 fired a per-slice override).
3. **Tie at the top class.** When two or more claims share the strictly-greatest effective class (regardless of kind), the requirement is a `conflict` with `resolution: tied-conflict` and no winner markers — identical to the same-class tie the single-kind table already specifies.

This makes the §"Status and provenance derivation" table's "unique top authority" / "top authority class ties" rows precise for the multi-kind case: "top authority" means the maximum **per-claim** effective class across the requirement's claims, and a per-kind override changes only the effective class of claims of that kind. The micro-resolver pinned in `crates/model/src/evidence/authority.rs` tests already resolves one kind at a time; the production kernel applies it per claim and then takes the strict maximum, so the four pinned scenarios remain black-box coverage of step 1.

### Synthesis envelope

The synthesis step receives a fixed-shape request and returns a fixed-shape response. The engine dispatches the request to the operator's agent under `execution: agent` (the default and designed centre), or to a declared WASI tool when `execution: tool` is configured (D10). Either way, the envelope is stable:

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

`authority.resolved-path` carries the kernel's pre-dispatch RFC-27 resolution so the synthesis step knows which claims win without re-deciding authority. The `constraints.forbidden-inputs-for-requirements-reconciliation` field is part of the contract: a conforming synthesis step reconciles the **entire requirements section** — entries, `sources`, `statement` / `title` / `scenarios[]` / `notes` — from the Evidence map and resolved authority alone, never from `target` or `shape-brief`. (`target` and `shape-brief` remain present in the envelope because they are legitimate inputs to the non-requirements sections of the model.) This is the agent-with-envelope expression of D8's target-neutrality requirement; it is checked at the boundary by the cross-target invariant in [RFC-29d §"Acceptance proof (D7)"](rfc-29d-target-build-envelope.md) and re-asserted by the synthesis prompt body shipped with first-party Specify.

The response carries a **draft model** (requirement set, per-requirement `(source, claim-id)` claims, `agreement` verdict, `unit`, prose — validated against [`draft-model.schema.json`](rfc-29/schemas/slice/draft-model.schema.json)) plus **prose-only** Markdown artifacts. The synthesis step does not assign REQ-ids, does not derive `sources`, does not mark winners, does not derive `status`, does not set `generated-at` / `generator`, and does not write `provenance.yaml` — those are the kernel's. The engine validates the draft, projects the kernel (id assignment in declaration order, `sources`/winner/status derivation, provenance projection), rejects any response that usurps a kernel-owned field or cites a `(source, claim-id)` absent from the Evidence map, validates the merged `model.yaml` against [`model.schema.json`](rfc-29/schemas/slice/model.schema.json), renders provenance lines into `spec.md`, then persists. Full request/response shape: [`rfc-29/schemas/slice/synthesis.schema.json`](rfc-29/schemas/slice/synthesis.schema.json) (`kind: request | response` discriminator); the response `model` `$ref`s the draft schema, not the persisted one (D3a).

### Shape-brief scope (D8)

The bound target's `shape` brief is an input to the **non-requirements sections of the synthesis step only** — the slice model's `domain`, `apis`, `configuration`, `technical-logic`, `observability`, and `tasks` sections (e.g. surface-by-surface vs type-by-type grouping; which optional sub-fields are populated; how much narrative each design decision carries). It is never an input to the requirements section, which the synthesis step reconciles from Evidence and resolved authority alone.

Shape briefs MUST NOT influence:

- `requirements[]` — entries, ids, ordering, statements, status, scenarios, or any other field;
- `requirements[].claims`, `requirements[].agreement`, `requirements[].sources`, or any `sources` field elsewhere in the slice model;
- `domain.types[].sources`, `apis.surfaces[].operations[].sources`, `technical-logic.decisions[].sources`, or any other provenance-bearing field.

The engine enforces D8 with two deterministic gates (see [RFC-29d §"Acceptance proof (D7)"](rfc-29d-target-build-envelope.md)):

1. **Envelope-construction proof** — requirements-relevant synthesis request inputs (`evidence`, resolved authority, `forbidden-inputs-for-requirements-reconciliation`) are byte-identical across target bindings.
2. **Kernel-projection determinism** — given a fixed synthesis response, kernel output (`provenance.yaml`, ids, sources, status, winners) is byte-identical and target-independent.

The `slice-synthesize-forbidden-input-leak` probe (mechanical set-difference) complements gate 1. An optional LLM-judge equivalence check over live agent runs MAY run as a non-gating `review` diagnostic but is never a release gate.

### Rendering

Synthesis persist is a three-phase pipeline:

| Phase | Author | Output |
| --- | --- | --- |
| **Synthesis step** | Agent | Draft model (`claims`, `agreement`, prose fields) + prose-only Markdown (`proposal.md`, `design.md`, `tasks.md`, spec requirement bodies without provenance lines) |
| **Projection kernel** | CLI | Full `model.yaml`, `provenance.yaml` |
| **Render step** | CLI | Injects kernel-owned `ID:` / `Sources:` / `Status:` lines (and status tags in requirement headlines) into `specs/<unit>/spec.md` from projected `model.yaml`, merging agent-authored requirement prose |

The kernel stamps `generated-at`, `generator`, top-level `sources`, `requirements[].id`, `.sources`, `.status`, `requirements[].tags`, and each claim's `winner` marker deterministically. The requirement set, claims, agreement verdicts, and behavioral prose are the synthesis step's, persisted once they pass draft validation.

`spec.md` stays the behavioral review artifact and baseline merge input. Operators may hand-edit behavioral prose after synthesis; hand-edits to kernel-rendered provenance lines without re-synthesis emit `slice-spec-provenance-stale` (§"Drift validation"). `model.yaml` is the machine view target builders consume. `provenance.yaml` remains audit-only and is always kernel-projected.

Re-running `specrun slice synthesize` overwrites `model.yaml`, `provenance.yaml`, and kernel-rendered provenance lines in `spec.md`; operator prose edits outside those lines survive only until the next full re-synthesis if the agent returns different bodies.

### Trade-offs (three provenance-bearing surfaces)

RFC-29 keeps three on-disk surfaces that encode requirement/provenance state:

- **`model.yaml`** — machine source of truth for structure, claims, agreement, and kernel-projected status/sources.
- **`spec.md`** — human review and merge input; provenance lines are **rendered from** `model.yaml`, not independently authored by the agent.
- **`provenance.yaml`** — audit index **projected from** `model.yaml` claims + authority (D11).

Bidirectional drift between three independently-authored copies was the RFC-35 failure mode. Rendering provenance into `spec.md` and projecting `provenance.yaml` from the same kernel output collapses most parity checks to internal consistency (`model` ↔ `provenance.yaml`, by construction) plus one re-sync gate when operators hand-edit provenance lines in `spec.md`.

### Standalone provenance projection (D11)

The projection kernel above is not only reachable through `specrun slice synthesize`. RFC-29 also exposes it as a standalone verb:

```bash
specrun slice provenance <slice> [--format json]
```

`specrun slice provenance` reads the slice's already-persisted `model.yaml` (the agent-authored `requirements[].claims` and `agreement` verdicts), the slice's `Evidence[]`, and the per-slice / per-Evidence authority-overrides, then runs the **identical** projection D3 wraps — RFC-27 authority resolution, winner-marker derivation, `status` derivation from the agreement verdict, and the claim-level projection into `provenance.yaml`. It writes `.specify/slices/<slice>/provenance.yaml` and nothing else. Re-running it over an unchanged `model.yaml` is byte-stable (it is the same pure function of `(model structure, Evidence[], authority-overrides)` described in §"Shape-brief scope (D8)").

Uses:

1. Regenerate `provenance.yaml` without re-synthesis after hand-editing `model.yaml` claims.
2. Single kernel module shared with `specrun slice synthesize`; natural seam for kernel-projection determinism tests (D7).

RFC-35 deferral rationale and how claim-level input resolves it: [RFC-29 §"Relationship to RFC-35"](rfc-29-fan-in-fan-out.md#relationship-to-rfc-35).

`specrun slice provenance` never reads the bound `target` or its `shape` brief, never re-decides the requirement set, never selects a winner the resolved authority did not, and never overrides the agreement verdict — the same kernel constraints D3 lists.

## Typed slice model (D4)

### File

```text
.specify/slices/<slice>/model.yaml
```

The slice model is generated by `specrun slice synthesize` and regenerated whole on re-synthesis. Operators should edit `spec.md` or `design.md`, not `model.yaml`; re-running synthesize will overwrite `model.yaml`.

### Shape

Normative schema: [`rfc-29/schemas/slice/model.schema.json`](rfc-29/schemas/slice/model.schema.json) (lands in `specify-cli` as `schemas/slice/model.schema.json`). The slice model is closed at the top level (`additionalProperties: false`) and uses kebab-case field names on disk; required top-level fields are `version`, `slice`, `generated-at`, `generator`, `sources`, `target`, `requirements`, `domain`, `apis`, `configuration`, `technical-logic`, `observability`, and `tasks`. The `project` field is optional (mirroring `plan.yaml.slices[].project`).

Sketch of the on-disk shape (illustrative; the schema is normative):

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
    # agent authors: title, statement, scenarios, unit, claims, agreement.
    # kernel derives: id, sources, status, and each claim's winner marker.
  - id: REQ-001
    title: Request password reset
    status: agreed
    unit: password-reset
    sources: [docs, legacy]
    agreement: agreed
    claims:
      - { source: docs,   claim-id: password-reset.request,         kind: requirement }
      - { source: legacy, claim-id: users.password-reset.request,   kind: example }
    statement: The system lets a registered user request a password reset link by email.
    scenarios:
      - Given REQ-001 and a registered email, when the user requests a reset, then the system accepts the request.
  - id: REQ-007
    title: Reset link expiry
    status: divergence
    unit: password-reset
    sources: [docs, legacy]
    agreement: disagreed
    claims:
      - { source: docs,   claim-id: password-reset.expiry, kind: criterion, winner: true }
      - { source: legacy, claim-id: password-reset.expiry, kind: example,   winner: false }
    statement: The system expires password reset links 30 minutes after issue.
    tags: [divergence]
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

### ID grammar

`model.yaml` introduces six closed three-digit id grammars in addition to the existing `REQ-NNN` from `crates/model/src/spec/provenance.rs`:


| Id         | Grammar           | Used by                                                                                 |
| ---------- | ----------------- | --------------------------------------------------------------------------------------- |
| `REQ-NNN`  | `^REQ-[0-9]{3}$`  | `requirements[].id`, plus `satisfies[]` references from operations / decisions / tasks. |
| `TASK-NNN` | `^TASK-[0-9]{3}$` | `tasks[].id`, plus `tasks[].depends-on[]`.                                              |
| `DEC-NNN`  | `^DEC-[0-9]{3}$`  | `technical-logic.decisions[].id`.                                                       |
| `TYP-NNN`  | `^TYP-[0-9]{3}$`  | `domain.types[].id`.                                                              |
| `OP-NNN`   | `^OP-[0-9]{3}$`   | `apis.surfaces[].operations[].id`.                                                      |
| `CFG-NNN`  | `^CFG-[0-9]{3}$`  | `configuration[].id`.                                                                   |
| `OBS-NNN`  | `^OBS-[0-9]{3}$`  | `observability[].id`.                                                                   |


All seven grammars are enforced by `schemas/slice/model.schema.json`. The synthesis engine assigns ids in declaration order per section, with no cross-section reuse and no holes after a single synthesis run.

### Drift validation

`specrun slice validate` adds seven checks for RFC-29 slices:


| Finding                      | Meaning                                                                                                      |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `slice-model-schema`            | `model.yaml` does not match `schemas/slice/model.schema.json`.                                                     |
| `slice-spec-provenance-stale` | Kernel-rendered provenance lines in `spec.md` (`ID:`, `Sources:`, `Status:`, status tags) disagree with projected `model.yaml` — typically an operator hand-edit without re-synthesis. |
| `slice-model-provenance-drift`      | `model.yaml.requirements[].claims` disagrees with `provenance.yaml`, compared at `(source, claim-id)` granularity, for any matching `REQ-*`.                      |
| `slice-model-target-drift`      | `model.yaml.target` (or `model.yaml.project`) disagrees with `plan.yaml.slices[<slice>].target` / `.project`.      |
| `slice-model-source-orphan`     | A `claims[]` entry references a `(source, claim-id)` whose source key is absent from `model.yaml.sources[].key` or whose claim id is absent from that source's Evidence.                        |
| `slice-model-cross-ref-orphan`  | A `satisfies[]` `REQ-*` reference does not exist in `requirements[].id`.                                     |
| `slice-model-claim-kind-mismatch` | A `claims[]` entry's `kind` (D13) disagrees with the kind recorded for that `(source, claim-id)` in the source's Evidence.                                     |


Every synthesized slice carries `model.yaml`; its absence is rejected.

### Build input

Target builders consume `model.yaml` as their machine input and may also read rendered Markdown for behavioral context. **`model.yaml` is authoritative for structure and provenance** (ids, status, sources, claims). **`spec.md` is authoritative for behavioral prose** after operator review. Kernel-rendered provenance lines in `spec.md` must match `model.yaml` or `slice-spec-provenance-stale` fires; operators who need to change provenance outcomes edit claims/agreement in a re-synthesis path or amend authority overrides — not provenance lines directly.

## Per-slice fan-out (D5)

Cross-target fan-out is **plan-level**, not slice-level (D5; [decision log §"One plan entry, one project"](../docs/explanation/decision-log.md#one-plan-entry-one-project)).

### Plan schema

Unchanged. Each `plan.yaml.slices[]` entry continues to carry exactly one `target` and (optionally) one `project`:

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

The same `Lead` may appear in more than one slice's `sources[]` when both slices need the same Evidence — this is the fan-in side, not fan-out. Lead reconciliation (D2) proposes one slice per `(target, lead-group)` pair; the operator may split or merge proposed slices at Gate 1.

### Lifecycle

Unchanged. Each slice walks the existing single-state lifecycle:

```text
refining -> refined -> built -> merged
```

Per-slice build detail in `.metadata.yaml`:

```yaml
target: omnia@v1
status: built
generated-paths:
  - crates/identity_service
build-report-path: build/report.yaml
```

`/spec:build` transitions the slice to `built` when the target's build report validates with `status: success`. `/spec:merge` performs one delta merge against one baseline (the slice's bound target / project).

### Workspace routing

Unchanged from RFC-25. The slice's `project` was bound at plan time by the D2 reconciliation step ([RFC-29b §"Project selection"](rfc-29b-lead-reconciliation.md)) and stored on `plan.yaml.slices[].project`; build-time routing only *resolves* that already-chosen name. `/spec:build` for a workspace-routed slice resolves the slice's `project` against the registry, prepares that project slot, writes target-specific files, records generated paths in the build report, and restores CWD to the workspace root. The plan lock stays at the workspace root. Cross-slice ordering — e.g. building `identity-contracts` before `identity-service` because the latter `depends-on` the former — is enforced by `specrun plan next`, not by anything inside a slice.

## Synthesis execution mode (D10)

The synthesis step inside `specrun slice synthesize` carries a closed `execution: agent | tool` enum. It shares the adapter enum's (D9) value names but not its emphasis: where a deterministic path exists, adapters lean toward `tool` (the framework nudges first-party adapters toward it), whereas synthesis is **agent-first by design**. Cross-modal Evidence reconciliation into a requirement set is the load-bearing judgment of the framework, so `agent` is the default and the designed centre — the two values are first-class peers, with no "fallback" connotation on the agent path. An `execution: tool` path is optional, reserved for future declared synthesis tools that admit narrow deterministic cases (e.g. single-source slices where Evidence already carries statement-quality prose).

The configuration lives on the workspace, not on individual adapter manifests, because the synthesis step is core-owned and per-slice:

```yaml
# project.yaml (one entry per project; defaults to agent)
synthesize:
  execution: agent     # or `tool`
```

The two values are:

- `**agent**` — the engine resolves authority, hands the synthesis envelope to the operator's agent, validates the draft response, projects the kernel, validates the merged `model.yaml`, renders provenance into `spec.md`, runs drift validators, and persists. This is the first-party default and the designed centre of synthesis.
- `**tool**` — the engine additionally requires a declared synthesis WASI tool to be configured (`synthesize.tool: { name, version }`), pipes the envelope on stdin, projects and validates the returned response identically, and caches the result under a synthesis-specific fingerprint (Evidence sha256 set + authority-overrides + shape-brief sha256 + tool `name@version`). Optional and reserved for narrow deterministic cases.

When `execution: agent`, the engine:

1. emits a `slice.synthesize.agent` journal event on every invocation;
2. forces `cache: opt-out` for the synthesis step (the kernel's projection over the returned structure remains deterministic, and `provenance.yaml` is reproducible from a fixed response under a kernel-only fingerprint of structure + Evidence + authority-overrides);
3. surfaces no finding by default — `agent` is the expected and recommended mode for cross-modal slices. A `suggestion`-severity `slice-synthesize-agent-mode` finding is raised only when an operator has explicitly opted in to tool-only enforcement (`synthesize.enforce-tool: true`), which is itself an unusual choice the framework does not encourage for cross-modal synthesis.

Regardless of execution mode, the engine validates the draft response against `draft-model.schema.json`, the merged result against `model.schema.json`, and the drift checks before the slice transitions to `refined`. The execution mode does not relax any validation; it only changes who authors the requirement set and prose.

## Claim contract (D13)

Every claim that contributes to a requirement carries a stable `claim-id` **and** its `kind`:

- `schemas/evidence.schema.json` requires `claim-id` on **every** claim kind, so every `(source, claim-id)` cited by a requirement resolves.
- `model.yaml.requirements[].claims[]` carries `kind` (required, mirrors `evidence.schema.json#/$defs/claimKind`) so the projection kernel resolves per-kind authority (§"Authority over mixed-kind claims") and populates the `kind`-bearing `provenance.yaml` `contributing-claims[]` **without re-reading Evidence**.

`specrun slice validate` adds `slice-model-claim-kind-mismatch` when a claim's `kind` disagrees with the kind recorded for that `(source, claim-id)` in Evidence; `slice-model-source-orphan` still catches a `(source, claim-id)` absent from Evidence.

## Wire contracts introduced by this milestone

The canonical closed tables live in [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts). This milestone appends:

- **Journal events:** `slice.synthesize.started`, `slice.synthesize.authority-resolved`, `slice.synthesize.agent`, `slice.synthesize.completed`, `slice.synthesize.failed`, `slice.model.show.requested`.
- **Validation findings (`Diagnostic` codes, validate surface):** `slice-model-schema`, `slice-spec-provenance-stale`, `slice-model-provenance-drift`, `slice-model-target-drift`, `slice-model-source-orphan`, `slice-model-cross-ref-orphan`, `slice-model-claim-kind-mismatch`, `slice-model-id-grammar`, `slice-synthesize-forbidden-input-leak` — emitted by `specrun slice validate` as a `DiagnosticReport`; blocking findings gate the transition at exit 2.
- **Operational validation codes (`Error::Validation`):** `slice-synthesize-kernel-field-usurped`, `slice-synthesize-execution-mode-required` — single-signal `specrun slice synthesize` aborts (the former rejects a draft that usurps a kernel-owned field, before projection). Neither tier adds a new `Error` enum variant; see [RFC-29 §"Shared wire contracts"](rfc-29-fan-in-fan-out.md#shared-wire-contracts) for the error-tiering model.
- **Schemas:** `schemas/slice/model.schema.json` (`SLICE_MODEL_JSON_SCHEMA`), `schemas/slice/draft-model.schema.json` (`DRAFT_MODEL_JSON_SCHEMA`), `schemas/slice/synthesis.schema.json` (`SYNTHESIS_JSON_SCHEMA`) — registered together so relative `$ref`s compile without a registry lookup. Plus the D13 `claim-id` requirement on `schemas/evidence.schema.json`.
