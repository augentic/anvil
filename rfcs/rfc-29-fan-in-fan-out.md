# RFC-29: Fan-In/Fan-Out Code Contract

> Status: Draft - Depends: [RFC-25](../done/rfc-25-workflow.md), [RFC-27](../done/rfc-27-synthesis.md), [RFC-28](../done/rfc-28-standards-contract.md) - Relates: [RFC-35](rfc-35-synthesis-determinism.md) (synthesis-determinism stepping stone; see §"Relationship to RFC-35") - Enables: provable multi-source fan-in and plan-level multi-slice fan-out, with one target per slice (see §D5)

## Abstract

Specify's architectural promise is a fan-in / fan-out workflow:

- **Fan-in** happens twice per change. Multiple source adapters' `Lead`s fan in at plan time into the `slices[]` rows of `plan.yaml`. Multiple sources' `Evidence` fans in at slice time into one synthesized slice. Both are core's responsibility.
- **Fan-out** happens once per change, at the plan layer. One change decomposes into multiple slices — each slice binding exactly one target — joined by `depends-on` edges. The `refine -> build -> merge` loop runs per slice; baseline merge runs once per slice against one target's baseline.

This is the framework's "one plan entry, one project" decision (see [decision log](../docs/explanation/decision-log.md#one-plan-entry-one-project)). RFC-29 affirms it and does not extend the slice to multi-target.

The gap is that several load-bearing fan-in steps — survey, extract, and plan-time reconciliation — are still agent discipline rather than CLI-owned contract. Slice synthesis itself stays agent-led on purpose: cross-modal reconciliation of design prose, code, and screenshots into a coherent requirement set — deciding which requirements exist, how Evidence claims merge or split into them, what each draws on, and the statements, scenarios, and design narrative that express them — is the heart of synthesis and is judgment work. This RFC does not claim that away. What it makes CLI-owned is the *deterministic projection around* that judgment: RFC-27 authority resolution, REQ-id assignment, provenance projection into `provenance.yaml`, status derivation, and drift validation, all computed over the structure the agent proposes. The contribution to synthesis is the **envelope** — stable inputs, stable outputs, and a kernel that turns the agent's reconciliation into validated, provenance-tracked artifacts — not a deterministic reconstruction of the requirement set.

This RFC turns the fan-in promise into an end-to-end contract by adding:

1. **Executable source operations** - first-class `specrun source survey` and `specrun source extract` commands that run source adapters under the declared sandbox, cache, and journal contract.
2. **Deterministic plan-time structural reconciliation** - a CLI-owned lead-reconciliation engine that proposes slice rows from `Lead[]`, preserving operator review for ambiguous joins and operator/agent judgment for target binding.
3. **Slice synthesis engine** - an agent-led cross-modal synthesis step (which decides the requirement set, declares each requirement's sources, and authors its prose) running under a stable input/output envelope, wrapped by a CLI-owned projection kernel that projects over the agent's structure: RFC-27 authority resolution, REQ-id assignment, status derivation, provenance projection into `provenance.yaml`, and drift validators.
4. **Typed slice model** - a machine-readable, schema-pinned view of the slice emitted by refine and used by target builders, while the existing Markdown artifacts remain the human review surface and baseline merge input.
5. **Target build contract** - target adapters consume the slice model through a stable per-slice build envelope, with per-slice validation, review findings, and merge gates.
6. **Proof fixtures** - acceptance coverage that exercises `N sources -> one slice model -> 1 target per slice`, with cross-target fan-out proven across multiple slices joined by `depends-on`, and the kernel / envelope split proven by kernel-projection determinism over a fixed synthesis response, plus cross-target target-neutrality and semantic equivalence of the agent-authored requirement set.

## Motivation

The current codebase can describe the fan-in/fan-out model, but it cannot yet prove it as a framework invariant: source operations are briefs not executable commands, plan-time lead reconciliation is agent-only, slice-time reconciliation has no production resolver, the machine-readable slice view is implicit, and target codegen is adapter-brief discipline with no stable envelope. The normative decisions below close each gap.

The goal is not to remove agents but to move stable workflow, data-shape, and bookkeeping obligations into the CLI so agents keep the judgment work — cross-modal Evidence reconciliation into a requirement set, repair, and domain-specific generation. Cross-modal reconciliation is the canonical case of unavoidable judgment and stays agent-led at the heart of synthesis; the CLI's contribution is the **envelope** around that step plus the deterministic *projection* over its output: stable inputs, stable outputs, RFC-27 authority resolution, REQ-id assignment, provenance projection, drift validation, journal events. The kernel never reconstructs the requirement set; it validates and provenance-tracks the one the agent proposes. Synthesis carries an explicit `agent | executable` execution mode whose default — and designed centre — is `agent`.

## Normative decisions


| ID                              | Decision                                                                                                                                                                                                                                 | Implementation consequence                                                                                                                                                                                            |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **D1 Source operation runner**  | The CLI runs source adapter `survey` and `extract` operations.                                                                                                                                                                        | Add `specrun source survey` and `specrun source extract`; route through `SourceAdapter::resolve`, declared tools, sandbox preopens, extraction cache, schema validation, and journal events.                       |
| **D2 Lead reconciliation engine**  | The CLI owns the **structural** `Lead[] -> lead groups` pass (rules 1–3 — exact id, exact alias, transitive cross-reference). **Target binding** (which group becomes which `(slice, target)` pair) stays agent-driven until a Lead target-axes hint lands. | Ship in two stages. Stage B1: `specrun plan propose --dry-run --format json` returns the structural groups as JSON without writing the plan. `/spec:plan` reads the JSON, decides target binding per group, and writes through the existing `specrun plan add` / `plan amend` writers. Stage B2 (deferred): once Lead target-axes are RFC'd, promote `propose` to a full writer that emits one `(group, target)` slice directly. |
| **D3 Slice synthesis engine** | The agent-led synthesis step owns cross-modal reconciliation of `Evidence[]` into the requirement set — which requirements exist, how claims merge or split, each requirement's declared `sources`, its prose (`title` / `statement` / `scenarios` / `notes`), and the four Markdown artifacts. The CLI owns the projection kernel that *projects over* that structure (RFC-27 authority resolution, REQ-id assignment, status derivation, provenance into `provenance.yaml`, drift validation) and the synthesis envelope. The kernel never reconstructs the requirement set. | Add `specrun slice synthesize <slice>`. The engine resolves authority via the RFC-27 resolver, dispatches the synthesis step (`agent` by default, or a declared `executable` synthesis tool per D10) with the Evidence map and resolved authority, then assigns ids, derives status, projects provenance, validates, and persists. `/spec:refine` stops hand-coding the reconciliation projection and shells out to the engine. |
| **D4 Typed slice model**           | Every synthesized slice carries `.specify/slices/<slice>/model.yaml`.                                                                                                                                                                       | Add `schemas/slice/model.schema.json`; `specrun slice validate` checks model/artifact/provenance drift; target build reads the slice model as its primary machine input.                                                     |
| **D5 Per-slice fan-out**        | Each slice binds exactly one target adapter / project. Cross-target changes decompose at plan time into multiple slices joined by `depends-on`. RFC-29 introduces no per-output schema, lifecycle, or build envelope.                    | No `outputs[]` field on the slice model, build request, or build report. `plan.yaml.slices[].target` / `slices[].project` keep their existing shape and meaning. Cross-slice ordering uses the existing `slices[].depends-on`. |
| **D6 Target build envelope**    | Target adapters receive a stable per-slice build request and return a stable per-slice build report.                                                                                                                                     | Add `schemas/target/build-request.schema.json` and `schemas/target/build-report.schema.json`, keyed on `(slice, target)`; reports may include RFC-28 findings.                                                        |
| **D7 Acceptance proof path**    | The release is not complete until an end-to-end fixture demonstrates fan-in and cross-slice fan-out together.                                                                                                                            | Add a cross-repo test in which two sources feed two slices (one targeting `contracts@v1`, one targeting `omnia@v1`), joined by `depends-on`; each slice independently synthesises, builds, and merges.                |
| **D8 Shape-brief scope**        | Target `shape` briefs may parameterise the slice model's structure for `domain` / `apis` / `configuration` / `technical-logic` / `observability` / `tasks` but MUST NOT influence `requirements[]`, `sources[]`, or any provenance-bearing field. | The whole requirements section — entries, `sources`, statements, scenarios, ordering — is authored by the agent-led synthesis step, which the envelope walls off from `target` and `shape-brief` (`forbidden-inputs-for-requirements-reconciliation`); requirements are therefore **target-neutral by construction**, not by deterministic reconstruction. The kernel's **projection** over that structure (REQ-id assignment in declaration order, `status` derived from authority over each requirement's declared sources, `*.sources` provenance into `provenance.yaml`) is a pure, target-independent function of `(response structure, Evidence[], authority-overrides)` — byte-identical when re-run over a fixed response. Cross-target equivalence of the requirement set is validated semantically (§"Acceptance proof"), not as byte-equality. |
| **D9 Adapter execution mode**   | Source adapters declare a closed `execution: executable | agent-fallback` field; first-party adapters MUST be `executable` before RFC-29 ships, third-party adapters MAY be `agent-fallback` indefinitely.                               | Extend `schemas/source.schema.json` and (symmetrically) `schemas/target.schema.json` with the closed enum. `agent-fallback` forces `cache: opt-out` and emits `source.execution.agent-fallback` per invocation.       |
| **D10 Synthesis execution mode** | The synthesis step inside `specrun slice synthesize` carries a closed `execution: agent | executable` enum. Unlike the adapter enum (D9), synthesis is **agent-first by design**: cross-modal Evidence reconciliation is judgment work, so `agent` is the default and the designed centre — not a fallback. An `execution: executable` path is optional, reserved for future declared synthesis tools that admit narrow deterministic cases (e.g. single-source statement-quality Evidence). | Add the closed enum to slice-synthesis configuration. `agent` forces `cache: opt-out` for the synthesis step (the kernel's projection over the returned structure remains deterministic) and emits `slice.synthesize.agent` per invocation. The engine validates the result against `schemas/slice/model.schema.json` and the six `slice-model-*` drift checks regardless of execution mode. |


## Operator surface

The default operator rhythm does not change:

```bash
/spec:plan identity-refresh source docs=documentation:./docs source legacy=code-typescript:./legacy
specrun plan transition identity-refresh approved
/spec:execute
/spec:finalize identity-refresh
```

The new CLI surfaces are lower-level breakouts:

```bash
specrun source survey docs --format json
specrun source extract docs password-reset --slice identity-password-reset --format json
specrun plan propose --dry-run --format json     # Stage B1 structural grouper (returns groups, never writes plan.yaml)
specrun slice synthesize identity-password-reset --format json
specrun slice model show identity-password-reset --format json
```

`specrun plan propose` without `--dry-run` is reserved for the deferred Stage B2 full writer (see §"Lead reconciliation engine (D2)"); in v1 it returns `propose-target-binding-required` and points at the dry-run form. Target binding stays with the `/spec:plan` agent step, which calls `specrun plan add` per `(group, target)` pair.

Cross-target changes are planned as multiple slices, each bound to one target, joined by `depends-on`:

```bash
specrun plan add identity-contracts \
  --sources docs=identity-api \
  --target contracts@v1 --project identity-contracts

specrun plan add identity-service \
  --sources docs=identity-api,legacy=identity-api \
  --target omnia@v1 --project identity-service \
  --depends-on identity-contracts
```

Each entry keeps its existing one-target shape:

```yaml
slices:
  - name: identity-contracts
    target: contracts@v1
    project: identity-contracts
  - name: identity-service
    target: omnia@v1
    project: identity-service
    depends-on: [identity-contracts]
```

A downstream slice that needs another slice's build report (e.g. `omnia` consuming the `contracts` schema) reads it through `prior-slices[]` on its own build request (see §"Target build envelope"). No multi-output, multi-target shape is added to a single slice — the plan layer is the only place fan-out happens.

## Source operation runner (D1)

### Commands

Add two commands under the existing `specify source` family:

```bash
specrun source survey <source-key> [--plan <name>] [--format json]
specrun source extract <source-key> <lead-id> --slice <slice> [--format json]
```

`<source-key>` resolves against `plan.yaml.sources.<key>`, not against adapter name. The command then resolves the adapter from `SourceBinding.adapter`.

### `survey`

`survey` runs the source adapter's `briefs.survey` operation under the source-adapter sandbox:


| Root              | Mode       | Contents                                                              |
| ----------------- | ---------- | --------------------------------------------------------------------- |
| `$SOURCE_DIR`     | read-only  | Bound source path when the source uses `path:`.                       |
| `$CAPABILITY_DIR` | read-only  | Resolved source adapter manifest cache.                               |
| `$SCRATCH_DIR`    | write-only | Per-operation scratch under `.specify/.cache/extractions/<adapter>/`. |
| `$PROJECT_DIR`    | none       | Not visible to the adapter operation.                                 |


For value-bound sources such as `intent`, `$SOURCE_DIR` is absent and the value is passed through the build request envelope.

Output is a lead set, validated against `schemas/discovery/lead.schema.json`, then merged into `discovery.md` by CLI-owned discovery helpers. Re-running `survey` for the same source replaces leads by canonical `id`, preserves operator aliases, and keeps deterministic ordering.

### `extract`

`extract` runs the source adapter's `briefs.extract` operation for one `(source-key, lead-id)` pair and writes:

```text
.specify/slices/<slice>/evidence/<source-key>.yaml
```

The CLI validates the Evidence document against `schemas/evidence.schema.json` before the write becomes visible to later synthesis. Failure leaves the slice in `refining`.

### Cache and journal

Both operations use the RFC-27 cache fingerprint model:

```text
source identity + adapter name@version + brief sha256 + sorted tool versions + lead id?
```

`lead id` is absent for `survey` and present for `extract`.

Journal events:


| Event                         | When                                      |
| ----------------------------- | ----------------------------------------- |
| `source.survey.cache-hit`  | Lead set was read from cache.        |
| `source.survey.cache-miss` | Adapter `survey` ran.                  |
| `slice.extract.cache-hit`     | Evidence was read from cache.             |
| `slice.extract.cache-miss`    | Adapter `extract` ran.                    |
| `slice.extract.completed`     | Evidence file was successfully persisted. |


`slice.extract.cache-*` already exists in RFC-27; this RFC adds the survey equivalents.

## Lead reconciliation engine (D2)

D2 splits a single conceptual step — `Lead[] -> plan entries` — into two halves:

1. **Structural reconciliation** (rules 1–3 below): exact id, exact alias, transitive cross-reference. Deterministic, pure data, no judgment. **CLI-owned from day one (Stage B1).**
2. **Target binding**: deciding which target adapter(s) each lead group becomes a slice for, under the per-slice fan-out model (D5). Inherently judgment work until Leads carry target-axis hints. **Agent-driven in v1, promoted to the CLI later (Stage B2).**

This split lets RFC-29 land the deterministic half without blocking on a target-axes design.

### Stage B1 — Structural grouper (CLI)

```bash
specrun plan propose --dry-run --format json
```

`propose --dry-run` reads:

- `plan.yaml.sources`;
- `discovery.md` lead inventory (via the in-place `crates/model/src/discovery/` model — `Discovery::parse` + `Discovery::resolve_lead` already cover the join surface);
- optional operator-authored aliases.

It writes **nothing** to disk. It returns a JSON document describing the proposed groups:

```json
{
  "version": 1,
  "groups": [
    {
      "group-id": "identity-api",
      "rule": "exact-id",
      "members": [
        { "source-key": "docs",   "lead-id": "identity-api" },
        { "source-key": "legacy", "lead-id": "identity-api" }
      ],
      "tentative-merges": []
    }
  ],
  "tentative-merges": [
    {
      "left":  { "source-key": "docs",   "lead-id": "password-reset" },
      "right": { "source-key": "legacy", "lead-id": "reset-password" },
      "reason": "no alias or exact id match exists; textual similarity 0.82"
    }
  ]
}
```

The schema lives at `schemas/discovery/proposal.schema.json` (committed alongside the existing `schemas/discovery/lead.schema.json`, reproduced verbatim as Schema D in §"Schemas added by this RFC") and embeds in the `specify-schema` crate as `PROPOSAL_JSON_SCHEMA`. `propose --dry-run` validates its own output before returning so callers can rely on the shape.

### Matching algorithm (B1)

The structural pass is intentionally conservative:

1. Exact canonical `id` match across source keys -> one group.
2. Exact alias match -> one group, persisted under the canonical id.
3. One lead's `sources` list transitively names another source's lead id (the existing `Lead.sources[]` cross-reference field) -> one group.
4. Otherwise each lead stays in its own group.

Textual-similarity may surface as a diagnostic under `tentative-merges[]`, but never auto-merges in v1. That keeps Stage B1 a pure function of the parsed discovery document.

### Agent role under Stage B1

`/spec:plan`'s `propose` sub-step:

1. Calls `specrun plan propose --dry-run --format json` to obtain the structural groups.
2. For each `groups[]` entry, decides which bound target(s) the group should become a slice for. Cross-target work expands to one slice per `(group, target)` pair, per D5. This is the only structural decision the agent still owns.
3. For each `(group, target)` pair, emits one `specrun plan add <slice-name> --sources <key>=<lead-id>… --target <name@vN> [--project <slug>] [--depends-on <other-slice>]` call.
4. For each `tentative-merges[]` entry, raises the diagnostic for operator review at Gate 1 (or runs `specrun plan amend --add-alias` to accept the merge).

Every plan mutation flows through the existing CLI writers in `crates/workflow/src/change/plan/`. The agent never hand-edits `plan.yaml`, never writes `discovery.md`, never decides authority — its scope is target binding and tentative-merge adjudication.

### Stage B2 — Full writer (deferred)

Once Leads carry deterministic target-axis hints (see §"Open questions" below), promote `specrun plan propose` to a full writer:

```bash
specrun plan propose [--format json]
```

The full form reconciles the structural pass with the target-binding pass and writes `plan.yaml.slices[]` directly. Stage B2 is **not** in scope for RFC-29 implementation; it ships in a follow-up RFC that nails down the target-axis vocabulary on `schemas/discovery/lead.schema.json`. Until then, `specrun plan propose` without `--dry-run` returns a `propose-target-binding-required` error directing the caller at the Stage B1 form.

### Review annotations

`tentative-merges[]` is the structured form of the "Tentative merges" Markdown block the agent renders into `change.md` for the operator. The agent may also call `specrun plan amend --divergence likely` against any subsequently-written slice when its bound leads carry materially disagreeing summaries; that writer path already exists.

## Slice synthesis engine (D3)

### Two layers

There is no deterministic function from `(design-prose, code-AST, vision-output)` to a coherent requirement set, so the engine splits cleanly into two layers — with the judgment layer first:

1. **Synthesis step (judgment, agent-led by default — the heart).** Cross-modal reconciliation of `Evidence[]` into the requirement set: deciding which requirements exist and how claims merge or split into them, declaring each requirement's contributing `sources`, authoring `requirements[].title` / `.statement` / `.scenarios[]` / `.notes`, populating the prose fields of the rest of the model (`domain.types[].fields[].description`, `apis.surfaces[].operations[]` request/response/errors prose, `technical-logic.decisions[].statement` / `.rationale`, `observability[].description`, `tasks[].text`), and rendering the four Markdown artifacts. This is the load-bearing judgment of synthesis and stays with the agent.
2. **Projection kernel (deterministic projection, CLI-owned).** RFC-27 authority resolution, REQ-id assignment in the agent's declaration order, `status` derivation from authority over each requirement's declared sources, provenance projection into `provenance.yaml`, and the six drift validators in §"Drift validation". This is where RFC-27's authority resolver becomes production code; it projects over the structure the agent returns and never invents, drops, or re-groups requirements.

The engine resolves authority, runs (1) under the envelope defined in §"Synthesis envelope", then runs (2) over the returned structure and validates the merged result against `schemas/slice/model.schema.json` and the drift validators before the slice transitions to `refined`.

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
3. After the synthesis step returns its requirement set and prose, the kernel projects over that structure — assigning REQ-ids in declaration order, deriving `status` from authority over each requirement's declared sources, and writing `provenance.yaml` — then validates and persists the merged artifacts:

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

The micro-resolver currently pinned in tests becomes black-box coverage for the production resolver. The kernel resolves authority **before** dispatch and passes the resolution into the envelope, so the synthesis step knows which claims win when it decides what each requirement draws on; the step never re-decides authority. After the step returns its requirement set with declared `sources`, the kernel projects the resolution over those declarations to write `provenance.yaml` and derive each requirement's `status`.

### Synthesis envelope

The synthesis step receives a fixed-shape request and returns a fixed-shape response. The engine dispatches the request to the operator's agent under `execution: agent` (the default and designed centre), or to a declared WASI tool when `execution: executable` is configured (D10). Either way, the envelope is stable:

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

`authority.resolved-path` carries the kernel's pre-dispatch RFC-27 resolution so the synthesis step knows which claims win without re-deciding authority. The `constraints.forbidden-inputs-for-requirements-reconciliation` field is part of the contract: a conforming synthesis step reconciles the **entire requirements section** — entries, `sources`, `statement` / `title` / `scenarios[]` / `notes` — from the Evidence map and resolved authority alone, never from `target` or `shape-brief`. (`target` and `shape-brief` remain present in the envelope because they are legitimate inputs to the non-requirements sections of the model.) This is the agent-with-envelope expression of D8's target-neutrality requirement; it is checked at the boundary by the cross-target invariant in §"Acceptance proof (D7)" and re-asserted by the synthesis prompt body shipped with first-party Specify.

The response is the populated `model.yaml` (requirement set, declared `sources`, prose) plus the four Markdown artifacts. The synthesis step does not assign REQ-ids or `status` and does not write `provenance.yaml` — those are the kernel's. The engine projects the kernel over the returned structure (id assignment in declaration order, status derivation, provenance projection), rejects any response that pre-assigns ids or cites a source absent from the Evidence map, then validates and persists. The full request/response shape — including the closed `kind: request | response` discriminator that lets one file validate both directions — is committed verbatim as Schema E in §"Schemas added by this RFC".

### Shape-brief scope (D8)

The bound target's `shape` brief is an input to the **non-requirements sections of the synthesis step only** — the slice model's `domain`, `apis`, `configuration`, `technical-logic`, `observability`, and `tasks` sections (e.g. surface-by-surface vs type-by-type grouping; which optional sub-fields are populated; how much narrative each design decision carries). It is never an input to the requirements section, which the synthesis step reconciles from Evidence and resolved authority alone.

Shape briefs MUST NOT influence:

- `requirements[]` — entries, ids, ordering, statements, status, scenarios, or any other field;
- `requirements[].sources` or any `sources` field elsewhere in the slice model;
- `domain.types[].sources`, `apis.surfaces[].operations[].sources`, `technical-logic.decisions[].sources`, or any other provenance-bearing field.

The engine enforces D8 in two layers:

- **Target-neutrality by construction (the requirement set).** The synthesis envelope hides `target` and `shape-brief` from every input the synthesis step is permitted to use for the requirements section — entries, `sources`, `id` candidates, `title`, `statement`, `scenarios[]`, `notes`. A conforming step therefore reconciles the same requirement set regardless of which target the slice binds. The CLI cannot byte-test a non-deterministic model across runs, so this layer is asserted *by construction* (the `forbidden-inputs-for-requirements-reconciliation` constraint plus the `slice-synthesize-forbidden-input-leak` probe) and validated for cross-target **semantic equivalence** in §"Acceptance proof (D7)" — not as byte-equality.
- **Deterministic projection (the kernel over a fixed structure).** Given a fixed synthesis-step response, the kernel's projection — REQ-id assignment in declaration order, `status` derived from authority over each requirement's declared sources, and every `*.sources` projection into `provenance.yaml` — is a pure, target-independent function of `(response structure, Evidence[], authority-overrides)`. Re-running the kernel over the **same** response yields byte-identical `provenance.yaml` and id/status projection regardless of bound target; this is the byte-testable part of the invariant (D7) and matches the guarantee RFC-35 D6 makes for `provenance.yaml`. Byte-equality is asserted only on this projection, never on the agent-authored requirement set across live runs.

### Rendering

The engine persists `provenance.yaml` and the kernel-owned fields of `model.yaml` (`requirements[].id`, `.status`, and every `*.sources` projection) deterministically; the requirement set and all prose in `model.yaml` are the synthesis step's, persisted as returned once they pass validation. The Markdown artifacts are rendered by the synthesis step (which is the natural author of the prose they contain) and validated for drift against `model.yaml` on ingest. The engine does not parse its own Markdown output to recover state during the same run.

`spec.md` stays the behavioral review artifact and baseline merge input. `model.yaml` is the generated machine view used by target builds. `provenance.yaml` remains audit-only.

## Typed slice model (D4)

### File

```text
.specify/slices/<slice>/model.yaml
```

The slice model is generated by `specrun slice synthesize` and regenerated whole on re-synthesis. Operators should edit `spec.md` or `design.md`, not `model.yaml`; re-running synthesize will overwrite `model.yaml`.

### Shape

The full machine shape is committed at `specify-cli/schemas/slice/model.schema.json` and reproduced verbatim in §"Schemas added by this RFC" below. The slice model is closed at the top level (`additionalProperties: false`) and uses kebab-case field names on disk; required top-level fields are `version`, `slice`, `generated-at`, `generator`, `sources`, `target`, `requirements`, `domain`, `apis`, `configuration`, `technical-logic`, `observability`, and `tasks`. The `project` field is optional (mirroring `plan.yaml.slices[].project`).

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
  - id: REQ-001
    title: Request password reset
    status: agreed
    sources: [docs, legacy]
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

`specrun slice validate` adds six checks:


| Finding                      | Meaning                                                                                                      |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `slice-model-schema`            | `model.yaml` does not match `schemas/slice/model.schema.json`.                                                     |
| `slice-model-requirement-drift` | `model.yaml.requirements[].id` set differs from `spec.md` `REQ-*` set.                                          |
| `slice-model-provenance-drift`      | `model.yaml.requirements[].sources` disagrees with `provenance.yaml` for any matching `REQ-*`.                      |
| `slice-model-target-drift`      | `model.yaml.target` (or `model.yaml.project`) disagrees with `plan.yaml.slices[<slice>].target` / `.project`.      |
| `slice-model-source-orphan`     | A model provenance entry references a source key absent from `model.yaml.sources[].key`.                        |
| `slice-model-cross-ref-orphan`  | A `satisfies[]` `REQ-*` reference does not exist in `requirements[].id`.                                     |


Absence of `model.yaml` is allowed for pre-RFC-29 slices and rejected for slices synthesized by an RFC-29-aware CLI.

### Build input

Target builders consume `model.yaml` as their machine input and may also read rendered Markdown for context. If they disagree, `model.yaml` wins for generated code shape and `spec.md` wins for operator-facing behavior. The drift validator is responsible for keeping that situation rare and visible.

## Per-slice fan-out (D5)

Cross-target fan-out is **plan-level**, not slice-level, per the framework's standing decision (see [decision log §"One plan entry, one project"](../docs/explanation/decision-log.md#one-plan-entry-one-project) and `docs/reference/targets/index.md`). RFC-29 affirms that contract and adds nothing to extend a slice to multi-target. No `outputs[]` field, no per-output lifecycle, no per-output build envelope, no per-output metadata, no per-output journal events.

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

Unchanged from RFC-25. `/spec:build` for a workspace-routed slice resolves the slice's `project` against the registry, prepares that project slot, writes target-specific files, records generated paths in the build report, and restores CWD to the workspace root. The plan lock stays at the workspace root. Cross-slice ordering — e.g. building `identity-contracts` before `identity-service` because the latter `depends-on` the former — is enforced by `specrun plan next`, not by anything inside a slice.

## Target build envelope (D6)

The build request and build report are both closed-shape YAML envelopes, keyed on `(slice, target)`. The full schemas are committed at `schemas/target/build-request.schema.json` and `schemas/target/build-report.schema.json` and reproduced verbatim in §"Schemas added by this RFC" below. The summaries below are illustrative; the schemas are normative.

### Build request

`/spec:build` constructs one build request per slice and either pipes it on stdin to a declared WASI tool (when the target's `execution: executable`) or writes it to `.specify/slices/<slice>/build/request.yaml` (when `execution: agent-fallback`):

```yaml
version: 1
slice: identity-service
target: omnia@v1
phase: build
project-root: /workspace/.specify/workspace/identity-service
workspace-root: /workspace
slice-dir: /workspace/.specify/slices/identity-service
model-path: /workspace/.specify/slices/identity-service/model.yaml
artifacts:
  proposal: proposal.md
  design: design.md
  tasks: tasks.md
  specs:
    - specs/identity/spec.md
  provenance: provenance.yaml
briefs:
  shape: /.../adapters/targets/omnia/briefs/shape.md
  build: /.../adapters/targets/omnia/briefs/build.md
execution:
  mode: executable
  tool:
    name: omnia
    version: v1.4.2
prior-slices:
  - slice: identity-contracts
    target: contracts@v1
    report-path: /workspace/.specify/slices/identity-contracts/build/report.yaml
cache-fingerprint: sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

`prior-slices[]` carries the build reports of slices listed in the current slice's `plan.yaml.slices[].depends-on`. This is the cross-slice analogue of the dropped `prior-outputs[]` field — the canonical use case is the `identity-service` slice (targeting `omnia`) reading the `identity-contracts` slice's report (which lists the generated `.yaml`) before generating the service crate. Each entry's `report-path` is the persisted build report path of a `merged` (or `built` in the current execution window) depended-on slice.

### Build report

Each target returns a build report. `status` is two-state (`success | failure`); partial outcomes land as `success` plus non-blocking findings at severity `optional` or `suggestion`:

```yaml
version: 1
slice: identity-service
target: omnia@v1
phase: build
status: success
started-at: 2026-05-28T05:45:00Z
finished-at: 2026-05-28T05:46:12Z
generator: omnia@v1.4.2
generated-paths:
  - crates/identity_service
  - crates/identity_service/Cargo.toml
validation:
  commands:
    - command: cargo check
      exit-code: 0
      duration-ms: 4123
    - command: cargo test
      exit-code: 0
      duration-ms: 18802
findings: []
evidence-cited: [docs, legacy]
cache:
  fingerprint: sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
  outcome: miss
```

`findings[]` items validate against `schemas/lint/finding.schema.json` (RFC-28). The CLI rejects `status: success` reports carrying any `critical`-severity finding (`target-build-success-with-critical-finding`).

The report is persisted at `.specify/slices/<slice>/build/report.yaml` and surfaces to downstream slices via their own request's `prior-slices[]` entries.

### Target adapter responsibilities

Target `build` briefs change from "read Markdown and decide what to do" to "consume the build request and produce a build report":

- `shape` remains synthesis guidance.
- `build` consumes `model.yaml` and rendered artifacts.
- `merge` consumes build reports and target-specific validation state.
- Any agent-generated code must still pass target-local validation before `status: success`.

### First-party target migration

The first migration path should be:

1. `contracts` first, because API contracts are already structured outputs.
2. `omnia` second, because Rust crate generation benefits most from typed requirements, APIs, configuration, and replay examples.
3. `vectis` third, because UI layout, assets, tokens, and `composition.yaml` need the widest slice-model shape.

## Adapter execution mode (D9)

Source and target adapters declare a closed `execution` field on their respective `adapter.yaml`:

```yaml
# adapters/sources/<name>/adapter.yaml
execution: executable     # or `agent-fallback`
```

The two values are:

- `**executable**` — `survey` and `extract` (sources) or `build` and `merge` (targets) are dispatched through a declared WASI tool or a deterministic Rust adapter path. Inputs and outputs validate against the schemas committed in this RFC. Required for first-party adapters before RFC-29 ships.
- `**agent-fallback**` — the adapter's brief is executed by an agent against the same sandbox preopens. The CLI orchestrates inputs and validates outputs against the same schemas, but does not cache the result. Permitted for third-party adapters indefinitely.

When `execution: agent-fallback`, the CLI:

1. emits a `source.execution.agent-fallback` (sources) or `target.execution.agent-fallback` (targets) journal event on every operation invocation;
2. forces `cache: opt-out` regardless of the adapter's declared cache mode (rejected at parse time as `adapter-execution-agent-fallback-cache-conflict` if the manifest declares any other cache mode);
3. surfaces a `suggestion`-severity `adapter-execution-agent-fallback` finding on the framework standards layer for first-party adapters, and not at all for third-party adapters.

The schema additions are mechanical extensions of `schemas/source.schema.json` and `schemas/target.schema.json`:

```json
{
  "execution": {
    "type": "string",
    "enum": ["executable", "agent-fallback"],
    "description": "Closed adapter execution mode per RFC-29 D9."
  }
}
```

with `execution` added to the `required` list on both schemas. Manifests authored before RFC-29 must add the field at first read; the loader rejects missing values rather than defaulting silently.

## Synthesis execution mode (D10)

The synthesis step inside `specrun slice synthesize` carries a closed `execution: agent | executable` enum. It deliberately does **not** mirror the adapter enum (D9): adapters aspire to `executable` and treat the agent path as a fallback, whereas synthesis is **agent-first by design**. Cross-modal Evidence reconciliation into a requirement set is the load-bearing judgment of the framework, so `agent` is the default and the designed centre — the two values are named as first-class peers, with no "fallback" connotation on the agent path. An `execution: executable` path is optional, reserved for future declared synthesis tools that admit narrow deterministic cases (e.g. single-source slices where Evidence already carries statement-quality prose).

The configuration lives on the workspace, not on individual adapter manifests, because the synthesis step is core-owned and per-slice:

```yaml
# project.yaml (one entry per project; defaults to agent)
synthesize:
  execution: agent     # or `executable`
```

The two values are:

- `**agent**` — the engine resolves authority, hands the synthesis envelope to the operator's agent, then projects the kernel over the returned structure (id assignment, status derivation, `provenance.yaml`), validates the merged `model.yaml` + Markdown against `schemas/slice/model.schema.json` and the six drift validators, and persists. This is the first-party default and the designed centre of synthesis.
- `**executable**` — the engine additionally requires a declared synthesis WASI tool to be configured (`synthesize.tool: { name, version }`), pipes the envelope on stdin, projects and validates the returned response identically, and caches the result under a synthesis-specific fingerprint (Evidence sha256 set + authority-overrides + shape-brief sha256 + tool `name@version`). Optional and reserved for narrow deterministic cases.

When `execution: agent`, the engine:

1. emits a `slice.synthesize.agent` journal event on every invocation;
2. forces `cache: opt-out` for the synthesis step (the kernel's projection over the returned structure remains deterministic, and `provenance.yaml` is reproducible from a fixed response under a kernel-only fingerprint of structure + Evidence + authority-overrides);
3. surfaces no finding by default — `agent` is the expected and recommended mode for cross-modal slices. A `suggestion`-severity `slice-synthesize-agent-mode` finding is raised only when an operator has explicitly opted in to tool-only enforcement (`synthesize.enforce-executable: true`), which is itself an unusual choice the framework does not encourage for cross-modal synthesis.

Regardless of execution mode, the engine validates the response against `schemas/slice/model.schema.json` and the six `slice-model-*` drift checks before the slice transitions to `refined`. The execution mode does not relax any validation; it only changes who authors the requirement set and prose.

## Acceptance proof (D7)

RFC-29 is complete only when the acceptance suite proves the full path — fan-in twice (Leads and Evidence), fan-out once (across slices):

```text
documentation + code-typescript
        -> source survey                 (fan-in #1: Lead sets)
        -> plan propose --dry-run           (CLI proposes structural groups; agent binds each group to one or more targets and writes the slices via plan add)
        -> per slice:
             source extract                 (fan-in #2: Evidence per source)
             slice synthesize               (envelope: agent-led cross-modal reconciliation
                                              into the requirement set + prose;
                                              kernel: deterministic projection — ids, status,
                                              provenance — over the returned structure;
                                              one Evidence map -> one slice model)
             model.yaml + artifacts + provenance.yaml
             target build (one target)
             slice merge (one baseline)
        -> validate cross-slice ordering via depends-on
```

Minimum fixture:

```text
tests/fixtures/rfc-29/fan-in-fan-out/
  sources/
    docs/
    legacy/
  expected/
    discovery.md
    plan.yaml                               # two slices, identity-service depends-on identity-contracts
    slices/identity-contracts/
      evidence/docs.yaml
      proposal.md
      specs/identity/spec.md
      design.md
      tasks.md
      provenance.yaml
      model.yaml                                # target: contracts@v1
      build/report.yaml
    slices/identity-service/
      evidence/docs.yaml
      evidence/legacy.yaml
      proposal.md
      specs/identity/spec.md
      design.md
      tasks.md
      provenance.yaml
      model.yaml                                # target: omnia@v1; sources include docs + legacy
      build/report.yaml                      # prior-slices cites identity-contracts/build/report.yaml
```

Required assertions:

- `specrun source survey` produces schema-valid leads for both sources.
- `specrun plan propose --dry-run --format json` returns one structural group for the shared lead (`rule: exact-id`), validates against `proposal.schema.json`, and writes nothing.
- The fixture's `/spec:plan` agent step (or the test harness simulating it) consumes the JSON, decides the per-group target binding (`contracts@v1` + `omnia@v1`), and issues two `specrun plan add` calls producing two single-target slices with `identity-service.depends-on: [identity-contracts]`.
- `specrun plan propose` without `--dry-run` exits non-zero with `propose-target-binding-required` (proves Stage B2 is gated).
- `specrun source extract` writes schema-valid Evidence for every `(slice, source)` pair.
- `specrun slice synthesize` writes valid artifacts, `provenance.yaml`, and `model.yaml` for each slice.
- `specrun slice validate` catches no provenance or slice-model drift on either slice.
- Each slice builds independently against its single bound target; `identity-service`'s build request carries a `prior-slices[]` entry pointing at `identity-contracts/build/report.yaml`.
- `specrun plan next` orders execution so `identity-contracts` reaches `merged` before `identity-service` starts.
- **Kernel-projection determinism (over a fixed response).** Capture each slice's synthesis-step response as a golden fixture, then re-run only the kernel projection over that fixed response twice. The result — `provenance.yaml` and the kernel-owned projection of `model.yaml` (`requirements[].id`, `.status`, every `*.sources` field, in declaration order) — is **byte-identical** across runs and independent of the bound target, even though `identity-contracts` binds `contracts@v1` and `identity-service` binds `omnia@v1`. This is the byte-testable invariant and matches RFC-35 D6's guarantee for `provenance.yaml`. The full-flow run with a live agent is **not** asserted byte-stable on the requirement set or prose, because the requirement set is agent judgment; those are validated by schema + drift + semantic checks below.
- **D8 invariant (requirement-set target-neutrality).** The two slices share the `docs:identity-api` lead and the same `authority-overrides`; `identity-service` additionally reconciles `legacy:identity-api`. On the shared lead's requirements, the two slices' `requirements[]` entries are validated for **semantic equivalence** — same intent, same declared `sources`, equivalent scenarios — by a fixture-local check (golden text on the simplest cases; LLM-judge with a fixed grader prompt on the more elaborate cases), even though the slices bind different targets. Byte-equality across targets is not asserted; target-neutrality of intent is. This proves shape briefs and target binding do not leak into the requirements section.
- **Forbidden-input-leak probe.** A fixture-local test confirms the envelope walls `target` and `shape-brief` off from the requirements-reconciliation inputs: a probe response whose requirements demonstrably reference target- or shape-brief-specific content is flagged by `slice-synthesize-forbidden-input-leak`, proving the target-neutrality-by-construction layer of D8.
- **Synthesis envelope contract.** A fixture-local test re-runs `specrun slice synthesize` with a deliberately-malformed synthesis-step response that usurps a kernel-owned field — pre-assigns a `REQ-NNN` id, sets `status`, or cites a `sources` key absent from the Evidence map. The engine rejects the response with `slice-synthesize-kernel-field-usurped` (id/status) or `slice-model-source-orphan` (orphan source) rather than persisting it, proving the kernel is the sole authority on id assignment, status derivation, and provenance projection while the agent remains the sole author of the requirement set.

## Schemas added by this RFC

Five new JSON Schemas are committed alongside this RFC. All are embedded in the `specify-schema` crate as `SLICE_MODEL_JSON_SCHEMA`, `BUILD_REQUEST_JSON_SCHEMA`, `BUILD_REPORT_JSON_SCHEMA`, `PROPOSAL_JSON_SCHEMA`, and `SYNTHESIS_ENVELOPE_JSON_SCHEMA` constants and reached through the existing `compile_schema` / `validate_value` plumbing. Field names are kebab-case on disk; top-level shapes are closed (`additionalProperties: false`); reusable closed enums (`kebabName`, `targetRef`, `requirementStatus`, `authorityClass`, the seven id grammars) live under `$defs` and are mirrored byte-identically with the matching `$defs` blocks in `evidence.schema.json`, `provenance.schema.json`, and `plan.schema.json`.

The fifth schema, `schemas/slice/synthesis-envelope.schema.json`, pins the request/response shape exchanged between the engine and the synthesis step (D3, D10) and is reproduced verbatim as Schema E below. The request and response halves share one file discriminated by a closed `kind: request | response` field, so the single `SYNTHESIS_ENVELOPE_JSON_SCHEMA` constant validates both directions; the request's `evidence` map is keyed by source key, and the response embeds the slice model by `$ref` to `schemas/slice/model.schema.json` and carries the four rendered Markdown artifacts as inline content.

The slice-model, build-request, and build-report schemas key on `(slice, target)` per D5 — none of them carries an `outputs[]` or `output-id` field. A future RFC that re-opens multi-target slices would need to widen all three schemas and revisit the lifecycle / merge contract.

`schemas/discovery/proposal.schema.json` (returned by `specrun plan propose --dry-run`) is reproduced verbatim as Schema D below. It is intentionally smaller than the other four — it carries no target binding — because target binding is agent-driven in v1.

### Schema A — `schemas/slice/model.schema.json`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://github.com/augentic/specify-cli/schemas/slice/model.schema.json",
  "title": "Specify slice model.yaml",
  "description": "Validates a slice's typed machine-readable model per RFC-29 §Typed slice model. Generated by `specrun slice synthesize` and regenerated whole on re-synthesis. Operators edit `spec.md` / `design.md` / `tasks.md`; `model.yaml` is the machine view target builders consume. One slice binds one target (RFC-29 D5 §Per-slice fan-out) — `target` is a scalar, not an array. Drift between `model.yaml.requirements[].id` and `spec.md` `REQ-*` ids is reported as `slice-model-requirement-drift`; drift between `model.yaml.requirements[].sources` and `provenance.yaml` is reported as `slice-model-provenance-drift`; drift between `model.yaml.target` / `model.yaml.project` and `plan.yaml.slices[<slice>].target` / `.project` is reported as `slice-model-target-drift`. Closed top-level shape — unknown fields are rejected.",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "version",
    "slice",
    "generated-at",
    "generator",
    "sources",
    "target",
    "requirements",
    "domain",
    "apis",
    "configuration",
    "technical-logic",
    "observability",
    "tasks"
  ],
  "properties": {
    "version": { "type": "integer", "minimum": 1, "maximum": 1 },
    "slice": { "$ref": "#/$defs/kebabName" },
    "generated-at": { "type": "string", "format": "date-time" },
    "generator": { "type": "string", "minLength": 1 },
    "sources": {
      "type": "array",
      "items": { "$ref": "#/$defs/modelSource" }
    },
    "target": { "$ref": "#/$defs/targetRef" },
    "project": { "type": ["string", "null"] },
    "requirements": {
      "type": "array",
      "items": { "$ref": "#/$defs/modelRequirement" }
    },
    "domain": { "$ref": "#/$defs/modelDomain" },
    "apis": { "$ref": "#/$defs/modelApis" },
    "configuration": {
      "type": "array",
      "items": { "$ref": "#/$defs/modelConfiguration" }
    },
    "technical-logic": { "$ref": "#/$defs/modelTechnicalLogic" },
    "observability": {
      "type": "array",
      "items": { "$ref": "#/$defs/modelObservability" }
    },
    "tasks": {
      "type": "array",
      "items": { "$ref": "#/$defs/modelTask" }
    }
  },
  "$defs": {
    "kebabName": {
      "type": "string",
      "pattern": "^[a-z0-9]+(-[a-z0-9]+)*$"
    },
    "reqId":         { "type": "string", "pattern": "^REQ-[0-9]{3}$" },
    "taskId":        { "type": "string", "pattern": "^TASK-[0-9]{3}$" },
    "decisionId":    { "type": "string", "pattern": "^DEC-[0-9]{3}$" },
    "typeId":        { "type": "string", "pattern": "^TYP-[0-9]{3}$" },
    "operationId":   { "type": "string", "pattern": "^OP-[0-9]{3}$" },
    "configId":      { "type": "string", "pattern": "^CFG-[0-9]{3}$" },
    "observabilityId": { "type": "string", "pattern": "^OBS-[0-9]{3}$" },
    "targetRef": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]*@v\\d+$",
      "description": "Mirrors `plan.schema.json` slice `target` pattern."
    },
    "authorityClass": {
      "type": "string",
      "enum": ["intent", "documentation", "behaviour"]
    },
    "requirementStatus": {
      "type": "string",
      "enum": ["agreed", "unknown", "conflict", "divergence"]
    },
    "modelSource": {
      "type": "object",
      "additionalProperties": false,
      "required": ["key", "adapter", "lead", "authority"],
      "properties": {
        "key":           { "$ref": "#/$defs/kebabName" },
        "adapter":       { "$ref": "#/$defs/kebabName" },
        "lead":     { "$ref": "#/$defs/kebabName" },
        "authority":     { "$ref": "#/$defs/authorityClass" },
        "evidence-path": { "type": "string" }
      }
    },
    "modelRequirement": {
      "type": "object",
      "additionalProperties": false,
      "required": ["id", "title", "status", "sources", "statement"],
      "properties": {
        "id":        { "$ref": "#/$defs/reqId" },
        "title":     { "type": "string", "minLength": 1 },
        "status":    { "$ref": "#/$defs/requirementStatus" },
        "sources":   {
          "type": "array",
          "uniqueItems": true,
          "items": { "$ref": "#/$defs/kebabName" }
        },
        "statement": { "type": "string", "minLength": 1 },
        "scenarios": {
          "type": "array",
          "items": { "type": "string", "minLength": 1 }
        },
        "tags": {
          "type": "array",
          "uniqueItems": true,
          "items": {
            "type": "string",
            "enum": ["divergence", "conflict", "unknown"]
          }
        },
        "notes": { "type": "string" }
      }
    },
    "modelDomain": {
      "type": "object",
      "additionalProperties": false,
      "required": ["types"],
      "properties": {
        "types": {
          "type": "array",
          "items": {
            "type": "object",
            "additionalProperties": false,
            "required": ["id", "name", "fields"],
            "properties": {
              "id":   { "$ref": "#/$defs/typeId" },
              "name": { "type": "string", "minLength": 1 },
              "kind": {
                "type": "string",
                "enum": ["record", "enum", "alias", "newtype"],
                "default": "record"
              },
              "fields": {
                "type": "array",
                "items": {
                  "type": "object",
                  "additionalProperties": false,
                  "required": ["name", "type"],
                  "properties": {
                    "name":        { "type": "string", "minLength": 1 },
                    "type":        { "type": "string", "minLength": 1 },
                    "optional":    { "type": "boolean", "default": false },
                    "description": { "type": "string" }
                  }
                }
              },
              "sources": {
                "type": "array",
                "uniqueItems": true,
                "items": { "$ref": "#/$defs/kebabName" }
              }
            }
          }
        }
      }
    },
    "modelApis": {
      "type": "object",
      "additionalProperties": false,
      "required": ["surfaces"],
      "properties": {
        "surfaces": {
          "type": "array",
          "items": {
            "type": "object",
            "additionalProperties": false,
            "required": ["id", "kind", "operations"],
            "properties": {
              "id":   { "$ref": "#/$defs/kebabName" },
              "kind": {
                "type": "string",
                "enum": ["rest", "graphql", "grpc", "asyncapi", "cli", "library", "ui"]
              },
              "operations": {
                "type": "array",
                "items": {
                  "type": "object",
                  "additionalProperties": false,
                  "required": ["id", "name"],
                  "properties": {
                    "id":        { "$ref": "#/$defs/operationId" },
                    "name":      { "type": "string", "minLength": 1 },
                    "request":   { "type": "string" },
                    "response":  { "type": "string" },
                    "errors": {
                      "type": "array",
                      "items": { "type": "string" }
                    },
                    "sources": {
                      "type": "array",
                      "uniqueItems": true,
                      "items": { "$ref": "#/$defs/kebabName" }
                    },
                    "satisfies": {
                      "type": "array",
                      "uniqueItems": true,
                      "items": { "$ref": "#/$defs/reqId" }
                    }
                  }
                }
              }
            }
          }
        }
      }
    },
    "modelConfiguration": {
      "type": "object",
      "additionalProperties": false,
      "required": ["id", "key", "type"],
      "properties": {
        "id":          { "$ref": "#/$defs/configId" },
        "key":         { "type": "string", "minLength": 1 },
        "type":        { "type": "string", "minLength": 1 },
        "default":     { "type": ["string", "null"] },
        "description": { "type": "string" },
        "sources": {
          "type": "array",
          "uniqueItems": true,
          "items": { "$ref": "#/$defs/kebabName" }
        }
      }
    },
    "modelTechnicalLogic": {
      "type": "object",
      "additionalProperties": false,
      "required": ["decisions"],
      "properties": {
        "decisions": {
          "type": "array",
          "items": {
            "type": "object",
            "additionalProperties": false,
            "required": ["id", "statement"],
            "properties": {
              "id":        { "$ref": "#/$defs/decisionId" },
              "statement": { "type": "string", "minLength": 1 },
              "rationale": { "type": "string" },
              "sources": {
                "type": "array",
                "uniqueItems": true,
                "items": { "$ref": "#/$defs/kebabName" }
              },
              "satisfies": {
                "type": "array",
                "uniqueItems": true,
                "items": { "$ref": "#/$defs/reqId" }
              }
            }
          }
        }
      }
    },
    "modelObservability": {
      "type": "object",
      "additionalProperties": false,
      "required": ["id", "kind", "name"],
      "properties": {
        "id":   { "$ref": "#/$defs/observabilityId" },
        "kind": {
          "type": "string",
          "enum": ["metric", "trace", "log", "alert"]
        },
        "name":        { "type": "string", "minLength": 1 },
        "description": { "type": "string" },
        "sources": {
          "type": "array",
          "uniqueItems": true,
          "items": { "$ref": "#/$defs/kebabName" }
        }
      }
    },
    "modelTask": {
      "type": "object",
      "additionalProperties": false,
      "required": ["id", "text"],
      "properties": {
        "id":   { "$ref": "#/$defs/taskId" },
        "text": { "type": "string", "minLength": 1 },
        "depends-on": {
          "type": "array",
          "uniqueItems": true,
          "items": { "$ref": "#/$defs/taskId" }
        },
        "satisfies": {
          "type": "array",
          "uniqueItems": true,
          "items": { "$ref": "#/$defs/reqId" }
        }
      }
    }
  }
}
```

### Schema B — `schemas/target/build-request.schema.json`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://github.com/augentic/specify-cli/schemas/target/build-request.schema.json",
  "title": "Specify target build request",
  "description": "Validates the build-request envelope produced by `specrun slice build` for one (slice, target) pair per RFC-29 §Target build envelope. One slice binds one target (D5 §Per-slice fan-out) — no `output-id` keying. Sent to a target adapter's declared WASI tool on stdin (when `execution: executable`) or written to `.specify/slices/<slice>/build/request.yaml` (when `execution: agent-fallback`). All filesystem paths are absolute under `project-root`. Closed top-level shape — unknown fields are rejected.",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "version",
    "slice",
    "target",
    "phase",
    "project-root",
    "slice-dir",
    "model-path",
    "artifacts",
    "briefs",
    "execution"
  ],
  "properties": {
    "version":        { "type": "integer", "minimum": 1, "maximum": 1 },
    "slice":          { "$ref": "#/$defs/kebabName" },
    "target":         { "$ref": "#/$defs/targetRef" },
    "phase": {
      "type": "string",
      "enum": ["build", "merge"]
    },
    "project-root":   { "type": "string" },
    "workspace-root": { "type": "string" },
    "slice-dir":      { "type": "string" },
    "model-path":        { "type": "string" },
    "artifacts":      { "$ref": "#/$defs/artifacts" },
    "briefs":         { "$ref": "#/$defs/briefs" },
    "execution":      { "$ref": "#/$defs/execution" },
    "prior-slices": {
      "type": "array",
      "items": { "$ref": "#/$defs/priorSlice" }
    },
    "rules": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "resolved-path": { "type": "string" }
      }
    },
    "cache-fingerprint": {
      "type": "string",
      "pattern": "^sha256:[0-9a-f]{64}$"
    }
  },
  "$defs": {
    "kebabName": {
      "type": "string",
      "pattern": "^[a-z0-9]+(-[a-z0-9]+)*$"
    },
    "targetRef": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]*@v\\d+$"
    },
    "artifacts": {
      "type": "object",
      "additionalProperties": false,
      "required": ["proposal", "design", "tasks", "specs"],
      "properties": {
        "proposal": { "$ref": "#/$defs/relativeArtifactPath" },
        "design":   { "$ref": "#/$defs/relativeArtifactPath" },
        "tasks":    { "$ref": "#/$defs/relativeArtifactPath" },
        "specs": {
          "type": "array",
          "minItems": 1,
          "items": { "$ref": "#/$defs/relativeArtifactPath" }
        },
        "provenance":   { "$ref": "#/$defs/relativeArtifactPath" }
      }
    },
    "relativeArtifactPath": {
      "type": "string",
      "minLength": 1
    },
    "briefs": {
      "type": "object",
      "additionalProperties": false,
      "required": ["build"],
      "properties": {
        "shape": { "type": "string" },
        "build": { "type": "string" },
        "merge": { "type": "string" }
      }
    },
    "execution": {
      "type": "object",
      "additionalProperties": false,
      "required": ["mode"],
      "properties": {
        "mode": {
          "type": "string",
          "enum": ["executable", "agent-fallback"]
        },
        "tool": {
          "type": "object",
          "additionalProperties": false,
          "required": ["name", "version"],
          "properties": {
            "name":      { "$ref": "#/$defs/kebabName" },
            "version":   { "type": "string", "pattern": "^v\\d+\\.\\d+\\.\\d+$" },
            "wasm-path": { "type": "string" }
          }
        },
        "tools": {
          "type": "array",
          "items": {
            "type": "object",
            "additionalProperties": false,
            "required": ["name", "version"],
            "properties": {
              "name":    { "$ref": "#/$defs/kebabName" },
              "version": { "type": "string", "pattern": "^v\\d+\\.\\d+\\.\\d+$" }
            }
          }
        }
      }
    },
    "priorSlice": {
      "type": "object",
      "additionalProperties": false,
      "required": ["slice", "target", "report-path"],
      "properties": {
        "slice":       { "$ref": "#/$defs/kebabName" },
        "target":      { "$ref": "#/$defs/targetRef" },
        "report-path": { "type": "string" }
      }
    }
  }
}
```

### Schema C — `schemas/target/build-report.schema.json`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://github.com/augentic/specify-cli/schemas/target/build-report.schema.json",
  "title": "Specify target build report",
  "description": "Validates the build-report envelope returned by a target adapter for one (slice, target) pair per RFC-29 §Target build envelope. One slice binds one target (D5 §Per-slice fan-out) — no `output-id` keying. Persisted at `.specify/slices/<slice>/build/report.yaml`. Closed top-level shape — unknown fields are rejected. `findings[]` entries are validated against `schemas/lint/finding.schema.json` (RFC-28). The CLI rejects `status: success` reports carrying any `critical`-severity finding.",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "version",
    "slice",
    "target",
    "phase",
    "status",
    "started-at",
    "finished-at",
    "generator"
  ],
  "properties": {
    "version":      { "type": "integer", "minimum": 1, "maximum": 1 },
    "slice":        { "$ref": "#/$defs/kebabName" },
    "target":       { "$ref": "#/$defs/targetRef" },
    "phase": {
      "type": "string",
      "enum": ["build", "merge"]
    },
    "status": {
      "type": "string",
      "enum": ["success", "failure"]
    },
    "started-at":   { "type": "string", "format": "date-time" },
    "finished-at":  { "type": "string", "format": "date-time" },
    "generator":    { "type": "string", "minLength": 1 },
    "generated-paths": {
      "type": "array",
      "uniqueItems": true,
      "items": { "type": "string", "minLength": 1 }
    },
    "validation": {
      "type": "object",
      "additionalProperties": false,
      "required": ["commands"],
      "properties": {
        "commands": {
          "type": "array",
          "items": {
            "type": "object",
            "additionalProperties": false,
            "required": ["command", "exit-code"],
            "properties": {
              "command":     { "type": "string", "minLength": 1 },
              "exit-code":   { "type": "integer" },
              "duration-ms": { "type": "integer", "minimum": 0 },
              "stdout-tail": { "type": "string" },
              "stderr-tail": { "type": "string" }
            }
          }
        }
      }
    },
    "findings": {
      "type": "array",
      "items": {
        "$ref": "https://github.com/augentic/specify-cli/schemas/lint/finding.schema.json"
      }
    },
    "evidence-cited": {
      "type": "array",
      "uniqueItems": true,
      "items": { "$ref": "#/$defs/kebabName" }
    },
    "cache": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "fingerprint": { "type": "string", "pattern": "^sha256:[0-9a-f]{64}$" },
        "outcome":     { "type": "string", "enum": ["miss", "hit", "skipped"] }
      }
    },
    "notes": { "type": "string" }
  },
  "$defs": {
    "kebabName": {
      "type": "string",
      "pattern": "^[a-z0-9]+(-[a-z0-9]+)*$"
    },
    "targetRef": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]*@v\\d+$"
    }
  }
}
```

### Schema D — `schemas/discovery/proposal.schema.json`

Returned by `specrun plan propose --dry-run --format json` (D2 Stage B1). It is intentionally the smallest of the five schemas: it carries the structural groups and advisory tentative merges only — no target binding, because target binding stays agent-driven in v1 (D2, open question 6). Each `groups[]` entry records how its members reconciled via the closed `rule` enum, covering all four branches of the §"Matching algorithm (B1)" pass (`exact-id`, `exact-alias`, `transitive-cross-reference`, and `singleton` for a lead that matched nothing). `tentative-merges[]` carries the advisory textual-similarity diagnostics that never auto-merge in v1; it appears both per-group and at the top level, sharing one `tentativeMerge` shape.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://github.com/augentic/specify-cli/schemas/discovery/proposal.schema.json",
  "title": "Specify plan propose structural grouping",
  "description": "Validates the structural lead-grouping document returned by `specrun plan propose --dry-run --format json` per RFC-29 §Lead reconciliation engine (D2) Stage B1. A pure function of the parsed `discovery.md`; the command writes nothing to disk and validates this shape before returning. Each `groups[]` entry records how its members reconciled (`rule`): `exact-id` (rule 1), `exact-alias` (rule 2), `transitive-cross-reference` (rule 3), or `singleton` (rule 4, a lead that matched nothing). Textual-similarity matches never auto-merge in v1; they surface as advisory `tentative-merges[]` for operator/agent adjudication at Gate 1. Target binding is NOT present here — it stays agent-driven until Stage B2. Closed top-level shape — unknown fields are rejected.",
  "type": "object",
  "additionalProperties": false,
  "required": ["version", "groups", "tentative-merges"],
  "properties": {
    "version": { "type": "integer", "minimum": 1, "maximum": 1 },
    "groups": {
      "type": "array",
      "items": { "$ref": "#/$defs/group" }
    },
    "tentative-merges": {
      "type": "array",
      "items": { "$ref": "#/$defs/tentativeMerge" }
    }
  },
  "$defs": {
    "kebabName": {
      "type": "string",
      "pattern": "^[a-z0-9]+(-[a-z0-9]+)*$"
    },
    "group": {
      "type": "object",
      "additionalProperties": false,
      "required": ["group-id", "rule", "members"],
      "properties": {
        "group-id": {
          "$ref": "#/$defs/kebabName",
          "description": "Canonical id the group is persisted under (the shared `id` for rules 1/3, the canonical id behind an alias for rule 2, or the lone lead's id for a singleton)."
        },
        "rule": {
          "type": "string",
          "enum": ["exact-id", "exact-alias", "transitive-cross-reference", "singleton"]
        },
        "members": {
          "type": "array",
          "minItems": 1,
          "items": { "$ref": "#/$defs/memberRef" }
        },
        "tentative-merges": {
          "type": "array",
          "items": { "$ref": "#/$defs/tentativeMerge" }
        }
      }
    },
    "memberRef": {
      "type": "object",
      "additionalProperties": false,
      "required": ["source-key", "lead-id"],
      "properties": {
        "source-key":   { "$ref": "#/$defs/kebabName" },
        "lead-id": { "$ref": "#/$defs/kebabName" }
      }
    },
    "tentativeMerge": {
      "type": "object",
      "additionalProperties": false,
      "required": ["left", "right", "reason"],
      "properties": {
        "left":   { "$ref": "#/$defs/memberRef" },
        "right":  { "$ref": "#/$defs/memberRef" },
        "reason": { "type": "string", "minLength": 1 }
      }
    }
  }
}
```

### Schema E — `schemas/slice/synthesis-envelope.schema.json`

Pins both halves of the synthesis exchange (D3, D10) in one file, discriminated by a closed `kind: request | response` field so the single `SYNTHESIS_ENVELOPE_JSON_SCHEMA` constant validates both directions through one `compile_schema` call. The request carries the Evidence map (keyed by source key), per-source authority, the kernel's resolved-authority path, the bound target's shape brief, and the closed `forbidden-inputs-for-requirements-reconciliation` constraint; `prior-baseline` is optional because baseline specs are passed only when available. No reconciliation skeleton is passed — the synthesis step authors the requirement set, and the kernel projects over it afterward. The response embeds the populated slice model by `$ref` to `schemas/slice/model.schema.json` and carries the four rendered Markdown artifacts as inline content (the engine persists them, so the prose travels in the envelope rather than as paths). The `authorityClass` and `targetRef` `$defs` mirror `model.schema.json` byte-identically.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://github.com/augentic/specify-cli/schemas/slice/synthesis-envelope.schema.json",
  "title": "Specify slice synthesis envelope",
  "description": "Validates the request/response envelope exchanged between the `specrun slice synthesize` engine and the agent-led synthesis step per RFC-29 §Slice synthesis engine (D3) and §Synthesis execution mode (D10). The request (`kind: request`, written to `.specify/slices/<slice>/synthesis/request.yaml` or piped to a declared WASI tool on stdin) carries the Evidence map, per-source authority, the kernel's resolved-authority path, the bound target's shape brief, and the closed `forbidden-inputs-for-requirements-reconciliation` constraint. The response (`kind: response`, read from `.specify/slices/<slice>/synthesis/response.yaml` or the tool's stdout) carries the populated slice `model` — the synthesis step's requirement set, declared sources, and prose — plus the rendered Markdown artifacts. The engine projects the kernel over the returned structure (REQ-id assignment, status derivation, provenance into `provenance.yaml`), rejecting any response that usurps a kernel-owned field with `slice-synthesize-kernel-field-usurped` or cites an orphan source with `slice-model-source-orphan`. Closed shapes — unknown fields are rejected.",
  "oneOf": [
    { "$ref": "#/$defs/request" },
    { "$ref": "#/$defs/response" }
  ],
  "$defs": {
    "kebabName": {
      "type": "string",
      "pattern": "^[a-z0-9]+(-[a-z0-9]+)*$"
    },
    "targetRef": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]*@v\\d+$"
    },
    "authorityClass": {
      "type": "string",
      "enum": ["intent", "documentation", "behaviour"]
    },
    "request": {
      "type": "object",
      "additionalProperties": false,
      "required": ["version", "kind", "slice", "target", "shape-brief", "evidence", "authority", "constraints"],
      "properties": {
        "version":     { "type": "integer", "minimum": 1, "maximum": 1 },
        "kind":        { "const": "request" },
        "slice":       { "$ref": "#/$defs/kebabName" },
        "target":      { "$ref": "#/$defs/targetRef" },
        "shape-brief": { "type": "string", "minLength": 1 },
        "evidence": {
          "type": "object",
          "minProperties": 1,
          "propertyNames": { "$ref": "#/$defs/kebabName" },
          "additionalProperties": { "$ref": "#/$defs/evidenceEntry" }
        },
        "authority": {
          "type": "object",
          "additionalProperties": false,
          "required": ["resolved-path"],
          "properties": {
            "resolved-path": { "type": "string", "minLength": 1 }
          }
        },
        "prior-baseline": {
          "type": "object",
          "additionalProperties": false,
          "required": ["specs-dir"],
          "properties": {
            "specs-dir": { "type": "string", "minLength": 1 }
          }
        },
        "constraints": {
          "type": "object",
          "additionalProperties": false,
          "required": ["forbidden-inputs-for-requirements-reconciliation"],
          "properties": {
            "forbidden-inputs-for-requirements-reconciliation": {
              "type": "array",
              "uniqueItems": true,
              "items": { "type": "string", "enum": ["target", "shape-brief"] }
            }
          }
        }
      }
    },
    "evidenceEntry": {
      "type": "object",
      "additionalProperties": false,
      "required": ["path", "authority"],
      "properties": {
        "path":      { "type": "string", "minLength": 1 },
        "authority": { "$ref": "#/$defs/authorityClass" }
      }
    },
    "response": {
      "type": "object",
      "additionalProperties": false,
      "required": ["version", "kind", "slice", "model", "artifacts"],
      "properties": {
        "version": { "type": "integer", "minimum": 1, "maximum": 1 },
        "kind":    { "const": "response" },
        "slice":   { "$ref": "#/$defs/kebabName" },
        "model": {
          "$ref": "https://github.com/augentic/specify-cli/schemas/slice/model.schema.json"
        },
        "artifacts": { "$ref": "#/$defs/artifacts" }
      }
    },
    "artifacts": {
      "type": "object",
      "additionalProperties": false,
      "required": ["proposal", "design", "tasks", "specs"],
      "properties": {
        "proposal": { "type": "string", "minLength": 1 },
        "design":   { "type": "string", "minLength": 1 },
        "tasks":    { "type": "string", "minLength": 1 },
        "specs": {
          "type": "array",
          "minItems": 1,
          "items": {
            "type": "object",
            "additionalProperties": false,
            "required": ["unit", "content"],
            "properties": {
              "unit":    { "$ref": "#/$defs/kebabName" },
              "content": { "type": "string", "minLength": 1 }
            }
          }
        }
      }
    }
  }
}
```

## Journal events

The closed `Event` / `EventKind` taxonomy in `crates/workflow/src/journal.rs` gains the following kebab-case event kinds. Wire ids are normative; Rust variants follow the existing `#[serde(rename = …)]` pattern.


| Event                             | When                                                                                                                                                         |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `source.survey.cache-hit`      | Lead set was read from cache.                                                                                                                           |
| `source.survey.cache-miss`     | Source-adapter `survey` ran.                                                                                                                              |
| `source.execution.agent-fallback` | A source-adapter operation ran in `agent-fallback` mode (`survey` or `extract`).                                                                          |
| `slice.extract.cache-hit`         | (Existing) Evidence was read from cache.                                                                                                                     |
| `slice.extract.cache-miss`        | (Existing) Source-adapter `extract` ran.                                                                                                                     |
| `slice.extract.completed`         | (Existing) Evidence file was successfully persisted.                                                                                                         |
| `slice.synthesize.started`        | `specrun slice synthesize` began for a slice.                                                                                                                |
| `slice.synthesize.authority-resolved` | The projection kernel resolved RFC-27 authority over `Evidence[]`. The synthesis envelope is about to be dispatched. No requirement skeleton is pre-computed. |
| `slice.synthesize.agent`          | The synthesis step ran in `agent` mode (the first-party default and designed centre). One event per invocation.                                                |
| `slice.synthesize.completed`      | `specrun slice synthesize` finished and all artifacts (`proposal.md`, `spec.md`, `design.md`, `tasks.md`, `provenance.yaml`, `model.yaml`) validated and persisted. |
| `slice.synthesize.failed`         | `specrun slice synthesize` aborted; prior artifacts left intact where possible.                                                                              |
| `slice.build.started`             | `/spec:build` (or `specrun slice build`) began work on a slice.                                                                                              |
| `slice.build.succeeded`           | A slice's build report validated with `status: success`.                                                                                                     |
| `slice.build.failed`              | A slice's build report carried `status: failure` or failed schema validation.                                                                                |
| `slice.merge.started`             | `/spec:merge` began work on a slice.                                                                                                                         |
| `slice.merge.succeeded`           | A slice's merge report validated with `status: success`.                                                                                                     |
| `slice.merge.failed`              | A slice's merge report carried `status: failure` or failed schema validation.                                                                                |
| `target.execution.agent-fallback` | A target-adapter operation ran in `agent-fallback` mode.                                                                                                     |
| `slice.model.show.requested`         | Operator invoked `specrun slice model show` (audit-only; useful for measuring model-consumer adoption).                                                         |


## Error discriminants and exit codes

`Exit::from(&Error)` in `src/runtime/output.rs` is the single source of truth for the wire contract; this RFC adds the closed `Error` variants below. The CLI dispatch table maps each one to a fixed exit code.


| Error variant (kebab-case discriminant)           | Exit | Cause                                                                                                   |
| ------------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------------- |
| `slice-model-schema`                                 | 2    | `model.yaml` does not match `schemas/slice/model.schema.json`.                                                |
| `slice-model-requirement-drift`                      | 2    | `model.yaml.requirements[].id` set differs from `spec.md` `REQ-*` set.                                     |
| `slice-model-provenance-drift`                           | 2    | `model.yaml.requirements[].sources` disagrees with `provenance.yaml`.                                          |
| `slice-model-target-drift`                           | 2    | `model.yaml.target` (or `model.yaml.project`) disagrees with `plan.yaml.slices[<slice>].target` / `.project`. |
| `slice-model-source-orphan`                          | 2    | A model provenance entry references a source key absent from `model.yaml.sources[].key`.                   |
| `slice-model-cross-ref-orphan`                       | 2    | A `satisfies[]` `REQ-*` reference does not exist in `requirements[].id`.                                |
| `slice-model-id-grammar`                             | 2    | A REQ / TASK / DEC / TYP / OP / CFG / OBS id does not match its closed three-digit grammar.             |
| `target-build-request-schema`                     | 2    | A build request fails `schemas/target/build-request.schema.json`.                                       |
| `target-build-report-schema`                      | 2    | A build report fails `schemas/target/build-report.schema.json`.                                         |
| `target-build-success-with-critical-finding`      | 2    | A build report sets `status: success` while carrying a finding at severity `critical`.                  |
| `target-build-prior-slice-not-built`              | 2    | A build request's `prior-slices[]` entry names a slice that has not produced a persisted build report.  |
| `adapter-execution-mode-required`                 | 2    | An adapter manifest does not declare `execution`.                                                       |
| `adapter-execution-agent-fallback-cache-conflict` | 2    | An adapter manifest sets `execution: agent-fallback` together with any cache mode other than `opt-out`. |
| `propose-target-binding-required`                 | 2    | `specrun plan propose` was invoked without `--dry-run` in v1; target binding stays agent-driven until Stage B2 ships. |
| `slice-synthesize-execution-mode-required`        | 2    | A workspace declares `synthesize.execution: executable` without configuring `synthesize.tool: { name, version }`. |
| `slice-synthesize-kernel-field-usurped`           | 2    | A synthesis-step response set a kernel-owned field it does not author — a `requirements[].id` (`REQ-NNN`) or `requirements[].status` value. The engine assigns ids and derives status; it rejects the response rather than persisting it. (Orphan `sources` are caught separately by `slice-model-source-orphan`.) |
| `slice-synthesize-forbidden-input-leak`           | 2    | A synthesis-step response's requirements section (entries, `sources`, `statement`, `title`, `scenarios`, `notes`) demonstrably referenced `target` or `shape-brief` content (detected by fixture-local target-neutrality probes). |


`EXIT_VALIDATION_FAILED = 2` is the only new code RFC-29 needs. Adapter resolution failures, sandbox preopen failures, WASI tool runtime failures, and I/O errors keep the existing `EXIT_GENERIC_FAILURE = 1` mapping.

## Implementation plan

A PR-sized breakdown of these waves lands in a companion `rfc-29-plan.md` (mirroring the [rfc-34-core-rules.md](./rfc-34-core-rules.md) / [rfc-34-plan.md](./rfc-34-plan.md) split). Each wave owns a defined set of new schemas, error variants, and journal events from the tables above.

### Wave A - Source runner and cache integration

1. Add the closed `execution: executable | agent-fallback` field to `schemas/source.schema.json`; thread it through `SourceAdapter` parse and add `adapter-execution-mode-required` / `adapter-execution-agent-fallback-cache-conflict` `Error` variants.
2. Add CLI DTOs and clap surfaces for `specrun source survey` and `specrun source extract`.
3. Reuse `SourceAdapter::resolve` and `SourceOperation::artifact_name`; branch dispatch on `execution`.
4. Route `executable` operations through declared WASI tools; route `agent-fallback` operations through the existing agent-run path but force `cache: opt-out` and emit `source.execution.agent-fallback`.
5. Validate lead output against `lead.schema.json` and Evidence output against `evidence.schema.json` before writes.
6. Add `source.survey.cache-{hit,miss}` cache events and update `specrun source resolve --explain` to show both operations.
7. Pin the `survey` cache fingerprint inputs explicitly in code and tests: source identity (path or value sha256) + adapter `name@version` + `survey` brief sha256 + sorted declared-tool versions.

### Wave B - Plan propose (Stage B1 only)

Stage B2 (full writer) is explicitly deferred — see §"Lead reconciliation engine (D2) → Stage B2" and the new "Lead target-axis vocabulary" open question.

1. Reuse the existing `Discovery` model in `crates/model/src/discovery/` (parse, `resolve_lead`, `check_alias_collisions` are already implemented and tested). No new parsing.
2. Implement the structural grouper as a pure function: `discovery::propose::group(&Discovery) -> Vec<Group>` covering rules 1 (exact id), 2 (exact alias), and 3 (transitive cross-reference). Surface diagnostic-only textual-similarity matches under `tentative_merges`.
3. Commit `schemas/discovery/proposal.schema.json` and embed it as `PROPOSAL_JSON_SCHEMA` in `specify-schema`.
4. Add `specrun plan propose --dry-run --format json` that runs the grouper, validates the output against `proposal.schema.json`, and prints. Reject every other `propose` form with `propose-target-binding-required` until Stage B2 lands.
5. Update `/spec:plan` to call `specrun source survey` per source, then `specrun plan propose --dry-run`, then issue one `specrun plan add` per `(group, target)` pair the agent decides on. `specrun plan add` continues to be the only writer.
6. Add fixtures for exact match, alias match, transitive cross-reference, tentative non-match, and per-group multi-target fan-out (the agent emits two `plan add` calls — the fixture asserts both slices land with the expected `target` and `depends-on`).

### Wave C - Synthesis engine and slice model

1. Commit `schemas/slice/model.schema.json` and embed it as `SLICE_MODEL_JSON_SCHEMA` in the `specify-schema` crate alongside the existing `*_JSON_SCHEMA` constants.
2. Add the production authority resolver and the projection kernel to `specify-workflow`. The kernel resolves RFC-27 authority over `Evidence[]` **before** dispatch, and — **after** the synthesis step returns its requirement set — projects over that structure: assigns `REQ-NNN` ids in declaration order, derives each requirement's `status` from authority over its declared `sources`, and writes `provenance.yaml`. The kernel never invents, drops, or re-groups requirements, and never reads the bound `target` or its `shape` brief.
3. Commit `schemas/slice/synthesis-envelope.schema.json` (request + response) and embed it as `SYNTHESIS_ENVELOPE_JSON_SCHEMA`. The request carries the Evidence map, the resolved-authority path, shape brief, and the closed `forbidden-inputs-for-requirements-reconciliation` constraint list (no reconciliation skeleton). The response carries the populated `model.yaml` — the agent-authored requirement set, declared `sources`, and prose — plus the four Markdown artifacts.
4. Implement the synthesis dispatcher: for `execution: agent` (the first-party default), write the request to `.specify/slices/<slice>/synthesis/request.yaml` and read the agent's response back from `.specify/slices/<slice>/synthesis/response.yaml`; for `execution: executable`, pipe the request to a declared synthesis WASI tool on stdin. In both modes, reject any response that usurps a kernel-owned field with `slice-synthesize-kernel-field-usurped` or cites an orphan source with `slice-model-source-orphan`, then project the kernel over the returned structure and validate.
5. Enforce D8 in two layers. **Layer 1 (kernel-projection determinism).** Unit-test the kernel over a captured, fixed synthesis-step response: project it twice and assert `provenance.yaml` and the kernel-owned projection of `model.yaml` (every `id`, `status`, `*.sources` field, in declaration order) are **byte-identical** and target-independent. **Layer 2 (requirement-set target-neutrality).** Integration-test the engine end-to-end: synthesise two slices binding different `target` values against the same Evidence map via the real dispatcher (using a fixed-seed test stub for the agent) and assert the shared-lead requirements are semantically equivalent across targets by both (a) byte-equality on golden cases the stub renders deterministically, and (b) a fixture-local LLM-judge check with a fixed grader prompt on the more elaborate cases. Add the `slice-synthesize-forbidden-input-leak` probe as a separate fixture.
6. Add `specrun slice synthesize` plus the `slice.synthesize.{started,authority-resolved,agent,completed,failed}` journal events.
7. Add `specrun slice model show <slice> [--format json]`.
8. Update `/spec:refine` to call the CLI command. The engine handles the dispatch and the kernel projection; the agent's contribution is authoring the requirement set and prose, not driving the lifecycle.
9. Extend `specrun slice validate` with the six slice-model drift checks and their `Error` variants (`slice-model-{schema,requirement-drift,reconciliation-drift,target-drift,source-orphan,cross-ref-orphan,id-grammar}`). Drift validation runs identically regardless of execution mode.
10. Add the workspace-level `synthesize: { execution, tool?, enforce-executable? }` field to `schemas/workspace.schema.json` (or `project.yaml`'s schema where it lives today), default `execution: agent`, and wire `slice-synthesize-execution-mode-required` rejection for `executable` without a declared tool.

### Wave D - Plan loader confirmation

No plan-schema change. `plan.yaml.slices[].target` / `slices[].project` stay singular per D5. This wave is a small chassis confirmation, not a feature:

1. Add a parser regression test asserting that `plan.yaml.slices[]` rejects an `outputs[]` field if a stray draft ever introduces one. This pins D5 in code rather than only in this RFC.
2. Confirm `specrun plan add` / `specrun plan amend` continue to refuse an `--output` flag; the only legal target binding is `--target <name@vN> [--project <slug>]`.
3. Confirm `specrun plan propose --dry-run` (Wave B / Stage B1) emits one structural group per matched lead set and that `/spec:plan`'s agent step issues one `specrun plan add` call per `(group, target)` pair with `depends-on` edges populated from operator-declared ordering hints in `discovery.md`.

### Wave E - Target build envelope

1. Commit `schemas/target/build-request.schema.json` and `schemas/target/build-report.schema.json` and embed both as `BUILD_REQUEST_JSON_SCHEMA` / `BUILD_REPORT_JSON_SCHEMA` in `specify-schema`. Both are keyed on `(slice, target)`; no `output-id`.
2. Add the closed `execution: executable | agent-fallback` field to `schemas/target.schema.json` symmetric with the source side; thread it through `TargetAdapter` parse.
3. Add `slice.build.{started,succeeded,failed}`, `slice.merge.{started,succeeded,failed}`, and `target.execution.agent-fallback` journal events.
4. Wire `prior-slices[]` population in the build-request builder: for each entry in the current slice's `plan.yaml.slices[].depends-on`, resolve the depended-on slice's `build/report.yaml` path and reject (`target-build-prior-slice-not-built`) when missing.
5. Update `contracts` build to consume the build request and emit a report (executable mode via WASI tool).
6. Update `omnia` build to consume `model.yaml` for crate/test/guest generation, read `prior-slices[]` to pick up upstream contract schemas, and emit a report (executable mode where deterministic; `agent-fallback` for the model-assisted phases that remain).
7. Update `vectis` build after the slice model has enough UI/layout structure.
8. Integrate RFC-28 findings into build reports; enforce `target-build-success-with-critical-finding` at the CLI boundary.

### Wave F - Proof fixtures and docs

1. Add the RFC-29 end-to-end fixture (D7): two slices over two sources, joined by `depends-on`, each binding one target. Include the D8 invariant assertion (the two slices share a lead; the shared-prefix of their `requirements[]` arrays is byte-identical).
2. Update `docs/explanation/concepts.md` and `docs/explanation/adapter-anatomy.md` to distinguish source fan-in (Leads + Evidence) from slice fan-out (plan-level decomposition with `depends-on`). Reaffirm "one slice, one target" alongside the existing `docs/explanation/decision-log.md` entry.
3. Update CLI reference pages for source, plan, slice, and target build reports — none of them gain an `outputs[]` field.
4. Update acceptance docs with the new proof command sequence (two `specrun plan add` calls, one per target, second with `--depends-on`).

## Migration

Existing projects continue to work without any change to `plan.yaml`:

- `plan.yaml.slices[]` keeps its existing one-`target`, optional-`project` shape. There is no `outputs[]` desugar to perform, and no `primary` literal to reserve. Any draft pre-RFC-29 plan referring to `outputs[]` is rejected as an unknown field on the existing plan schema.
- Slices without `model.yaml` validate under the pre-RFC-29 compatibility path unless re-synthesised.
- Target build briefs may initially read Markdown and ignore `model.yaml`, but first-party targets must migrate before RFC-29 is marked implemented.
- Source adapters may initially keep agent-run briefs, but first-party adapters must declare `execution: executable` before RFC-29 is marked implemented. Third-party adapters MAY remain `execution: agent-fallback` indefinitely.
- Existing first-party adapter manifests must add the new `execution` field at first read; the loader rejects missing values with `adapter-execution-mode-required` rather than defaulting silently. The companion `rfc-29-plan.md` PR list pins which adapters land each migration.
- Slice synthesis ships with `synthesize.execution: agent` as the first-party default (D10). Projects that already have a `/spec:refine` agent workflow continue to use it; the change is that the agent now operates inside a CLI-orchestrated envelope — authoring the requirement set and prose while the kernel projects ids, status, and provenance over its output — rather than driving the lifecycle. There is no "synthesis must become executable" deadline; `executable` is an optional path reserved for future declared synthesis tools, and `agent` is the designed centre, not a stopgap.

Once a slice has been synthesized by an RFC-29-aware CLI, `model.yaml` becomes required for that slice and drift validation applies.

## Non-goals

- No hosted execution or cloud runner. RFC-29 is local-first.
- No replacement of `spec.md` as the human behavioral artifact or baseline merge input.
- No graph database or global knowledge store for synthesis.
- No automatic merging of semantically similar leads without exact id, alias, or operator-seeded evidence.
- **No multi-target slices.** A slice binds exactly one target adapter / project (D5). Cross-target fan-out is plan-level, achieved by decomposing a change into multiple slices joined by `slices[].depends-on`. RFC-29 introduces no `outputs[]` array, no per-output lifecycle, no per-output build envelope, no per-output `.metadata.yaml` keying, and no per-output journal events. A future RFC that wishes to re-open this question must first amend `docs/explanation/decision-log.md` §"One plan entry, one project" and account for the multi-baseline merge contract that decision deliberately rules out.
- No target-specific behavior in the projection kernel. The bound target's `shape` brief is an input to the non-requirements sections of the synthesis step only (D8). Shape briefs MUST NOT influence `requirements[]` or any provenance-bearing field.
- No claim of deterministic requirement reconciliation. Deciding which requirements exist and how Evidence claims merge or split into them is the heart of synthesis and remains agent judgment, under the envelope defined in §"Synthesis envelope" (D3) and the execution mode in §"Synthesis execution mode" (D10). RFC-29 commits to a deterministic *kernel projection* — id assignment, status derivation, and provenance — over the agent's structure, **not** to a deterministic reconstruction of the requirement set or its prose. Cross-target consistency of requirements is target-neutrality by construction plus semantic equivalence, not byte-equality.
- No commitment to per-target determinism on day one. RFC-29 commits only to a stable build envelope and validation contract; per-target determinism milestones are tracked in each target adapter's manifest and changelog.

## Relationship to RFC-35

[RFC-35](rfc-35-synthesis-determinism.md) is the near-term stepping stone; RFC-29 builds its kernel on top. RFC-35 should land first (small, additive, unblocks the current agent-driven loop). The two RFCs share modules rather than duplicating logic:

- **Shared kernel, shared direction.** RFC-35 D6's `specrun slice reconcile` derives `provenance.yaml` *from* the agent-authored `spec.md` — agent-first. RFC-29 D3 now takes the same direction: the agent authors the requirement set and the kernel projects `provenance.yaml` over it, rather than computing a requirement skeleton up front. RFC-35 D6 is therefore a thin standalone entry point onto the same projection kernel RFC-29 D3 wraps inside `specrun slice synthesize`. Whichever RFC lands the module first, the other consumes it.
- **One journal emitter.** RFC-35 D7 `specrun journal emit` is the writer RFC-29's new `source.*` and `slice.synthesize.*` event kinds use; RFC-29 adds kinds to the closed `EventKind` taxonomy, not a second emission path.
- **One brief-location surface.** RFC-35 D9 `briefs_dir` is the deterministic brief-location output RFC-29 D1 and D3 rely on instead of cache-path arithmetic.

Roadmap [RM-06](roadmap.md#rm-06-fan-infan-out-workflow-contract) tracks RFC-29.

## Open questions

Exactly one question is unresolved. RFC-29 deliberately does **not** answer it in v1 and is fully implementable without it — the v1 decision is already pinned (option (d), per **D2 Stage B1**), and the question is only the prerequisite for the deferred **D2 Stage B2** full writer.

**Q1. Lead target-axis vocabulary (Stage B2 prerequisite).** Under D5, promoting `specrun plan propose` to a full writer would need a deterministic policy for turning a lead group into `(group, target)` slices. The four leads considered are:

   - **(a) Target hints on Leads.** Source adapters tag each lead with a closed `axes: [api, service, ui, …]` enum at `survey` time; `propose` cross-products groups by their members' union of axes. Cleanest long-term shape; requires extending `schemas/discovery/lead.schema.json` and per-source-adapter authoring discipline. Probably needs its own RFC.
   - **(b) Cross-product over plan-bound targets.** Emit `|groups| × |bound-targets|` slices and let the operator delete the irrelevant ones at Gate 1. Over-generates badly at scale; not viable past a handful of targets.
   - **(c) Operator post-amend.** `propose` emits `target: null` rows; operator runs `specrun plan amend --target` per slice. Pushes mechanical work onto the operator.
   - **(d) Status quo: agent decides — the v1 decision.** Per the **D2 Stage B1** decision in this revision, this is what v1 does. Honest about the judgment involved; keeps the CLI free of an arbitrary heuristic. Costs us a deterministic acceptance assertion on `plan.yaml.slices[]` byte-stability and keeps target binding out of the CLI's audit / journal trail.

   v1 ships option (d). This question is the explicit — and only — blocker for **D2 Stage B2** (`specrun plan propose` as a full writer), which is deferred to its own future RFC once a Lead target-axis vocabulary (option (a)) is designed. It blocks no other RFC-29 wave; D1, D3, D4, D5, D6, and D7 all land against the Stage B1 + agent form.

## References

- [RFC-25: Workflow](../done/rfc-25-workflow.md)
- [RFC-27: Synthesis Sharpening](../done/rfc-27-synthesis.md)
- RFC-28: Engineering Standards — Codex Contract and Findings
- RFC-32: Engineering Standards — Deterministic Enforcement
- [RFC-35: Synthesis Determinism](rfc-35-synthesis-determinism.md) — the synthesis-determinism stepping stone; its `specrun slice reconcile` (D6) is the standalone entry point onto the projection kernel RFC-29 D3 wraps (see §"Relationship to RFC-35")
- [Core concepts](../../docs/explanation/concepts.md)
- [Anatomy of an adapter](../../docs/explanation/adapter-anatomy.md)
- [Claim reconciliation](../../plugins/spec/references/synthesis/claim-reconciliation.md)
- [Provenance index](../../plugins/spec/references/synthesis/provenance.md)

