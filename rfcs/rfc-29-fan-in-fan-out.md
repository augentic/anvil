# RFC-29: Fan-In/Fan-Out Code Contract

> Status: Draft - Depends: [RFC-25](../done/rfc-25-workflow.md), [RFC-27](../done/rfc-27-synthesis.md), [RFC-28](../done/rfc-28-standards-contract.md) - Enables: provable multi-source fan-in and plan-level multi-slice fan-out, with one target per slice (see §D5)

## Abstract

Specify's architectural promise is a fan-in / fan-out workflow:

- **Fan-in** happens twice per change. Multiple source adapters' `Candidate`s fan in at plan time into the `slices[]` rows of `plan.yaml`. Multiple sources' `Evidence` fans in at slice time into one synthesized slice. Both are core's responsibility.
- **Fan-out** happens once per change, at the plan layer. One change decomposes into multiple slices — each slice binding exactly one target — joined by `depends-on` edges. The `refine -> build -> merge` loop runs per slice; baseline merge runs once per slice against one target's baseline.

This is the framework's "one plan entry, one project" decision (see [decision log](../docs/explanation/decision-log.md#one-plan-entry-one-project)). RFC-29 affirms it and does not extend the slice to multi-target.

The current system has source adapters, target adapters, `Candidate`, `Evidence`, provenance, authority, `fusion.yaml`, target `shape` briefs, and the `refine -> build -> merge` loop. The gap is that several load-bearing fan-in steps — enumerate, extract, plan-time fusion, slice synthesis — are still implemented as agent discipline rather than deterministic contract.

This RFC turns the fan-in promise into an end-to-end contract by adding:

1. **Executable source operations** - first-class `specrun source enumerate` and `specrun source extract` commands that run source adapters under the declared sandbox, cache, and journal contract.
2. **Deterministic plan-time fusion** - a CLI-owned candidate-fusion engine that proposes slice rows from `Candidate[]`, preserving operator review for ambiguous joins.
3. **Typed slice IR** - a machine-readable slice intermediate representation emitted by refine and used by target builders, while the existing Markdown artifacts remain the human review surface and baseline merge input.
4. **Target build contract** - target adapters consume the slice IR through a stable per-slice build envelope, with per-slice validation, review findings, and merge gates.
5. **Proof fixtures** - acceptance coverage that exercises `N sources -> one slice IR -> 1 target per slice`, with cross-target fan-out proven across multiple slices joined by `depends-on`.

## Motivation

The current codebase can describe the fan-in/fan-out model, but it cannot yet prove it as a framework invariant.

The findings this RFC resolves:


| Finding                                                      | Current state                                                                                                                                | RFC-29 resolution                                                                                                                                                                                  |
| ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source operations are briefs, not executable CLI operations. | `specrun source resolve` exists; `enumerate` and `extract` are agent-run instructions.                                                       | Add `specrun source enumerate` and `specrun source extract` with sandbox, cache, schema validation, and journal events.                                                                            |
| Plan-time candidate fusion is agent-only.                    | `/spec:plan`'s `propose` sub-step reads `discovery.md` and calls `specrun plan add`.                                                         | Split the step in two. Add a deterministic `specrun plan propose --dry-run --format json` (Stage B1) that emits structural candidate groups. The `/spec:plan` agent step keeps target binding (per-slice fan-out, D5) and writes via `specrun plan add`. A full-writer Stage B2 is deferred behind a Candidate target-axis vocabulary (open question 6). |
| Slice-time synthesis has no production resolver.             | CLI validates `spec.md`, Evidence, and `fusion.yaml`; it does not synthesize them.                                                           | Add a `specrun slice synthesize` engine that emits artifacts, `fusion.yaml`, and the typed slice IR from the same model.                                                                           |
| The intermediate representation is implicit.                 | `proposal.md`, `spec.md`, `design.md`, `tasks.md`, Evidence, and `fusion.yaml` together act as the IR, but target builders consume Markdown. | Add `.specify/slices/<slice>/ir.yaml` as generated machine-readable build input, with drift validation against rendered artifacts.                                                                 |
| Target codegen is adapter-brief discipline.                  | Target `build` briefs orchestrate generation, validation, and review, but no stable input/output envelope joins them to core synthesis.      | Add a per-slice target build envelope; each target reports structured status, generated paths, validation commands, and RFC-28 review findings.                                                    |


The goal is not to remove agents from Specify. The goal is to move stable workflow and data-shape obligations into the CLI so agents can focus on judgment, repair, and domain-specific generation rather than reimplementing lifecycle and reconciliation rules.

## Principles

1. **Core owns reconciliation.** If a rule decides how sources combine, it belongs in the CLI or a CLI-owned schema, not only in a skill body.
2. **Markdown remains reviewable.** `proposal.md`, `spec.md`, `design.md`, and `tasks.md` stay the operator-facing artifacts. The IR is the machine view emitted from the same synthesis model.
3. **One slice, one lifecycle, one target.** Each slice binds exactly one target adapter / project and walks one `refining -> refined -> built -> merged` lifecycle. Cross-target fan-out is plan-level — a change decomposes into multiple slices joined by `depends-on`. RFC-29 does not introduce a second per-output lifecycle inside a slice (see [decision log §"One plan entry, one project"](../docs/explanation/decision-log.md#one-plan-entry-one-project)).
4. **Targets consume, not synthesize.** Target adapters may shape synthesis and build outputs, but they do not create behavioral requirements or provenance.
5. **Agent fallback is explicit.** Where a target still needs model-assisted generation, the input and output envelope is stable and validation catches drift.
6. **Compatibility is additive.** Existing one-source, one-target plans keep working. New IR and envelope fields ride alongside the unchanged `slices[].target` / `slices[].project` shape.

## Normative decisions


| ID                              | Decision                                                                                                                                                                                                                                 | Implementation consequence                                                                                                                                                                                            |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **D1 Source operation runner**  | The CLI runs source adapter `enumerate` and `extract` operations.                                                                                                                                                                        | Add `specrun source enumerate` and `specrun source extract`; route through `SourceAdapter::resolve`, declared tools, sandbox preopens, extraction cache, schema validation, and journal events.                       |
| **D2 Candidate fusion engine**  | The CLI owns the **structural** `Candidate[] -> candidate groups` pass (rules 1–3 — exact id, exact alias, transitive cross-reference). **Target binding** (which group becomes which `(slice, target)` pair) stays agent-driven until a Candidate target-axes hint lands. | Ship in two stages. Stage B1: `specrun plan propose --dry-run --format json` returns the structural groups as JSON without writing the plan. `/spec:plan` reads the JSON, decides target binding per group, and writes through the existing `specrun plan add` / `plan amend` writers. Stage B2 (deferred): once Candidate target-axes are RFC'd, promote `propose` to a full writer that emits one `(group, target)` slice directly. |
| **D3 Slice synthesis engine**   | The CLI owns `Evidence[] + target shape -> slice artifacts + fusion.yaml + ir.yaml`.                                                                                                                                                     | Add `specrun slice synthesize <slice>`; retire the instruction that `/spec:refine` hand-codes synthesis. The engine uses the RFC-27 authority resolver as production code.                                            |
| **D4 Typed slice IR**           | Every synthesized slice carries `.specify/slices/<slice>/ir.yaml`.                                                                                                                                                                       | Add `schemas/slice/ir.schema.json`; `specrun slice validate` checks IR/artifact/fusion drift; target build reads the IR as its primary machine input.                                                                 |
| **D5 Per-slice fan-out**        | Each slice binds exactly one target adapter / project. Cross-target changes decompose at plan time into multiple slices joined by `depends-on`. RFC-29 introduces no per-output schema, lifecycle, or build envelope.                    | No `outputs[]` field on the IR, build request, or build report. `plan.yaml.slices[].target` / `slices[].project` keep their existing shape and meaning. Cross-slice ordering uses the existing `slices[].depends-on`. |
| **D6 Target build envelope**    | Target adapters receive a stable per-slice build request and return a stable per-slice build report.                                                                                                                                     | Add `schemas/target/build-request.schema.json` and `schemas/target/build-report.schema.json`, keyed on `(slice, target)`; reports may include RFC-28 findings.                                                        |
| **D7 Acceptance proof path**    | The release is not complete until an end-to-end fixture demonstrates fan-in and cross-slice fan-out together.                                                                                                                            | Add a cross-repo test in which two sources feed two slices (one targeting `contracts@v1`, one targeting `omnia@v1`), joined by `depends-on`; each slice independently synthesises, builds, and merges.                |
| **D8 Shape-brief scope**        | Target `shape` briefs may parameterise IR structure for `design-model` / `apis` / `configuration` / `technical-logic` / `observability` / `tasks` but MUST NOT influence `requirements[]`, `sources[]`, or any provenance-bearing field. | `specrun slice synthesize` computes the requirements section from `(Evidence[], authority-overrides)` alone; the requirements section is byte-equivalent across slices that share Evidence and authority-overrides regardless of bound target.   |
| **D9 Adapter execution mode**   | Source adapters declare a closed `execution: executable | agent-fallback` field; first-party adapters MUST be `executable` before RFC-29 ships, third-party adapters MAY be `agent-fallback` indefinitely.                               | Extend `schemas/source.schema.json` and (symmetrically) `schemas/target.schema.json` with the closed enum. `agent-fallback` forces `cache: opt-out` and emits `source.execution.agent-fallback` per invocation.       |


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
specrun source enumerate docs --format json
specrun source extract docs password-reset --slice identity-password-reset --format json
specrun plan propose --dry-run --format json     # Stage B1 structural grouper (returns groups, never writes plan.yaml)
specrun slice synthesize identity-password-reset --format json
specrun slice ir show identity-password-reset --format json
```

`specrun plan propose` without `--dry-run` is reserved for the deferred Stage B2 full writer (see §"Candidate fusion engine (D2)"); in v1 it returns `propose-target-binding-required` and points at the dry-run form. Target binding stays with the `/spec:plan` agent step, which calls `specrun plan add` per `(group, target)` pair.

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
specrun source enumerate <source-key> [--plan <name>] [--format json]
specrun source extract <source-key> <candidate-id> --slice <slice> [--format json]
```

`<source-key>` resolves against `plan.yaml.sources.<key>`, not against adapter name. The command then resolves the adapter from `SourceBinding.adapter`.

### `enumerate`

`enumerate` runs the source adapter's `briefs.enumerate` operation under the source-adapter sandbox:


| Root              | Mode       | Contents                                                              |
| ----------------- | ---------- | --------------------------------------------------------------------- |
| `$SOURCE_DIR`     | read-only  | Bound source path when the source uses `path:`.                       |
| `$CAPABILITY_DIR` | read-only  | Resolved source adapter manifest cache.                               |
| `$SCRATCH_DIR`    | write-only | Per-operation scratch under `.specify/.cache/extractions/<adapter>/`. |
| `$PROJECT_DIR`    | none       | Not visible to the adapter operation.                                 |


For value-bound sources such as `intent`, `$SOURCE_DIR` is absent and the value is passed through the build request envelope.

Output is a candidate set, validated against `schemas/discovery/candidate.schema.json`, then merged into `discovery.md` by CLI-owned discovery helpers. Re-running `enumerate` for the same source replaces candidates by canonical `id`, preserves operator aliases, and keeps deterministic ordering.

### `extract`

`extract` runs the source adapter's `briefs.extract` operation for one `(source-key, candidate-id)` pair and writes:

```text
.specify/slices/<slice>/evidence/<source-key>.yaml
```

The CLI validates the Evidence document against `schemas/evidence.schema.json` before the write becomes visible to later synthesis. Failure leaves the slice in `refining`.

### Cache and journal

Both operations use the RFC-27 cache fingerprint model:

```text
source identity + adapter name@version + brief sha256 + sorted tool versions + candidate id?
```

`candidate id` is absent for `enumerate` and present for `extract`.

Journal events:


| Event                         | When                                      |
| ----------------------------- | ----------------------------------------- |
| `source.enumerate.cache-hit`  | Candidate set was read from cache.        |
| `source.enumerate.cache-miss` | Adapter `enumerate` ran.                  |
| `slice.extract.cache-hit`     | Evidence was read from cache.             |
| `slice.extract.cache-miss`    | Adapter `extract` ran.                    |
| `slice.extract.completed`     | Evidence file was successfully persisted. |


`slice.extract.cache-*` already exists in RFC-27; this RFC adds the enumerate equivalents.

## Candidate fusion engine (D2)

D2 splits a single conceptual step — `Candidate[] -> plan entries` — into two halves:

1. **Structural fusion** (rules 1–3 below): exact id, exact alias, transitive cross-reference. Deterministic, pure data, no judgment. **CLI-owned from day one (Stage B1).**
2. **Target binding**: deciding which target adapter(s) each candidate group becomes a slice for, under the per-slice fan-out model (D5). Inherently judgment work until Candidates carry target-axis hints. **Agent-driven in v1, promoted to the CLI later (Stage B2).**

This split lets RFC-29 land the deterministic half without blocking on a target-axes design.

### Stage B1 — Structural grouper (CLI)

```bash
specrun plan propose --dry-run --format json
```

`propose --dry-run` reads:

- `plan.yaml.sources`;
- `discovery.md` candidate inventory (via the in-place `crates/domain/src/discovery/` model — `Discovery::parse` + `Discovery::resolve_candidate` already cover the join surface);
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
        { "source-key": "docs",   "candidate-id": "identity-api" },
        { "source-key": "legacy", "candidate-id": "identity-api" }
      ],
      "tentative-merges": []
    }
  ],
  "tentative-merges": [
    {
      "left":  { "source-key": "docs",   "candidate-id": "password-reset" },
      "right": { "source-key": "legacy", "candidate-id": "reset-password" },
      "reason": "no alias or exact id match exists; textual similarity 0.82"
    }
  ]
}
```

The schema lives at `schemas/discovery/proposal.schema.json` (committed alongside the existing `schemas/discovery/candidate.schema.json`) and embeds in the `specify-schema` crate as `PROPOSAL_JSON_SCHEMA`. `propose --dry-run` validates its own output before returning so callers can rely on the shape.

### Matching algorithm (B1)

The structural pass is intentionally conservative:

1. Exact canonical `id` match across source keys -> one group.
2. Exact alias match -> one group, persisted under the canonical id.
3. One candidate's `sources` list transitively names another source's candidate id (the existing `Candidate.sources[]` cross-reference field) -> one group.
4. Otherwise each candidate stays in its own group.

Textual-similarity may surface as a diagnostic under `tentative-merges[]`, but never auto-merges in v1. That keeps Stage B1 a pure function of the parsed discovery document.

### Agent role under Stage B1

`/spec:plan`'s `propose` sub-step:

1. Calls `specrun plan propose --dry-run --format json` to obtain the structural groups.
2. For each `groups[]` entry, decides which bound target(s) the group should become a slice for. Cross-target work expands to one slice per `(group, target)` pair, per D5. This is the only structural decision the agent still owns.
3. For each `(group, target)` pair, emits one `specrun plan add <slice-name> --sources <key>=<candidate-id>… --target <name@vN> [--project <slug>] [--depends-on <other-slice>]` call.
4. For each `tentative-merges[]` entry, raises the diagnostic for operator review at Gate 1 (or runs `specrun plan amend --add-alias` to accept the merge).

Every plan mutation flows through the existing CLI writers in `crates/domain/src/change/plan/`. The agent never hand-edits `plan.yaml`, never writes `discovery.md`, never decides authority — its scope is target binding and tentative-merge adjudication.

### Stage B2 — Full writer (deferred)

Once Candidates carry deterministic target-axis hints (see §"Open questions" below), promote `specrun plan propose` to a full writer:

```bash
specrun plan propose [--format json]
```

The full form fuses the structural pass with the target-binding pass and writes `plan.yaml.slices[]` directly. Stage B2 is **not** in scope for RFC-29 implementation; it ships in a follow-up RFC that nails down the target-axis vocabulary on `schemas/discovery/candidate.schema.json`. Until then, `specrun plan propose` without `--dry-run` returns a `propose-target-binding-required` error directing the caller at the Stage B1 form.

### Review annotations

`tentative-merges[]` is the structured form of the "Tentative merges" Markdown block the agent renders into `change.md` for the operator. The agent may also call `specrun plan amend --divergence likely` against any subsequently-written slice when its bound candidates carry materially disagreeing summaries; that writer path already exists.

## Slice synthesis engine (D3)

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

It writes, from one in-memory synthesis model:

```text
.specify/slices/<slice>/proposal.md
.specify/slices/<slice>/specs/<unit>/spec.md
.specify/slices/<slice>/design.md
.specify/slices/<slice>/tasks.md
.specify/slices/<slice>/fusion.yaml
.specify/slices/<slice>/ir.yaml
```

The write is staged and validated before the slice transitions to `refined`. If any artifact fails validation, the command exits non-zero and leaves the prior artifact set intact where possible.

### Production authority resolver

RFC-27's authority order becomes production code:

1. per-slice `authority-override`;
2. per-Evidence `authority-overrides`;
3. document-level `authority`;
4. tied effective authority -> `conflict`.

The micro-resolver currently pinned in tests becomes black-box coverage for the production resolver.

### Shape-brief scope (D8)

`specrun slice synthesize` reads the target's `shape` brief (one target per slice, per D5) and may use it to parameterise the **structure** of the IR's `design-model`, `apis`, `configuration`, `technical-logic`, `observability`, and `tasks` sections (e.g. surface-by-surface vs type-by-type grouping; which optional sub-fields are populated).

Shape briefs MUST NOT influence:

- `requirements[]` — entries, ids, ordering, statements, status, or scenarios;
- `requirements[].sources` or any `sources` field elsewhere in the IR;
- `domain-model.types[].sources`, `apis.surfaces[].operations[].sources`, `technical-logic.decisions[].sources`, or any other provenance-bearing field.

The engine enforces D8 by computing the requirements section from `(Evidence[], authority-overrides)` alone, independently of the bound target. Acceptance asserts that two slices binding the same `(source-key -> candidate)` map and same `authority-overrides` produce an `ir.yaml` whose `requirements[]` array is byte-identical, even when their `target` fields differ (D7).

### Rendering

The synthesis engine renders Markdown artifacts from the typed model. It does not parse its own Markdown output to recover state during the same run.

`spec.md` stays the behavioral review artifact and baseline merge input. `ir.yaml` is the generated machine view used by target builds. `fusion.yaml` remains audit-only.

## Typed slice IR (D4)

### File

```text
.specify/slices/<slice>/ir.yaml
```

The IR is generated by `specrun slice synthesize` and regenerated whole on re-synthesis. Operators should edit `spec.md` or `design.md`, not `ir.yaml`; re-running synthesize will overwrite `ir.yaml`.

### Shape

The full machine shape is committed at `specify-cli/schemas/slice/ir.schema.json` and reproduced verbatim in §"Schemas added by this RFC" below. The IR is closed at the top level (`additionalProperties: false`) and uses kebab-case field names on disk; required top-level fields are `version`, `slice`, `generated-at`, `generator`, `sources`, `target`, `requirements`, `domain-model`, `apis`, `configuration`, `technical-logic`, `observability`, and `tasks`. The `project` field is optional (mirroring `plan.yaml.slices[].project`).

Sketch of the on-disk shape (illustrative; the schema is normative):

```yaml
version: 1
slice: identity-service
generated-at: 2026-05-28T05:45:00Z
generator: specrun@2.1.0
sources:
  - key: docs
    adapter: documentation
    candidate: password-reset
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
domain-model:
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

`ir.yaml` introduces six closed three-digit id grammars in addition to the existing `REQ-NNN` from `crates/domain/src/spec/provenance.rs`:


| Id         | Grammar           | Used by                                                                                 |
| ---------- | ----------------- | --------------------------------------------------------------------------------------- |
| `REQ-NNN`  | `^REQ-[0-9]{3}$`  | `requirements[].id`, plus `satisfies[]` references from operations / decisions / tasks. |
| `TASK-NNN` | `^TASK-[0-9]{3}$` | `tasks[].id`, plus `tasks[].depends-on[]`.                                              |
| `DEC-NNN`  | `^DEC-[0-9]{3}$`  | `technical-logic.decisions[].id`.                                                       |
| `TYP-NNN`  | `^TYP-[0-9]{3}$`  | `domain-model.types[].id`.                                                              |
| `OP-NNN`   | `^OP-[0-9]{3}$`   | `apis.surfaces[].operations[].id`.                                                      |
| `CFG-NNN`  | `^CFG-[0-9]{3}$`  | `configuration[].id`.                                                                   |
| `OBS-NNN`  | `^OBS-[0-9]{3}$`  | `observability[].id`.                                                                   |


All seven grammars are enforced by `schemas/slice/ir.schema.json`. The synthesis engine assigns ids in declaration order per section, with no cross-section reuse and no holes after a single synthesis run.

### Drift validation

`specrun slice validate` adds six checks:


| Finding                      | Meaning                                                                                                      |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `slice-ir-schema`            | `ir.yaml` does not match `schemas/slice/ir.schema.json`.                                                     |
| `slice-ir-requirement-drift` | `ir.yaml.requirements[].id` set differs from `spec.md` `REQ-*` set.                                          |
| `slice-ir-fusion-drift`      | `ir.yaml.requirements[].sources` disagrees with `fusion.yaml` for any matching `REQ-*`.                      |
| `slice-ir-target-drift`      | `ir.yaml.target` (or `ir.yaml.project`) disagrees with `plan.yaml.slices[<slice>].target` / `.project`.      |
| `slice-ir-source-orphan`     | An IR provenance entry references a source key absent from `ir.yaml.sources[].key`.                          |
| `slice-ir-cross-ref-orphan`  | A `satisfies[]` `REQ-*` reference does not exist in `requirements[].id`.                                     |


Absence of `ir.yaml` is allowed for pre-RFC-29 slices and rejected for slices synthesized by an RFC-29-aware CLI.

### Build input

Target builders consume `ir.yaml` as their machine input and may also read rendered Markdown for context. If they disagree, `ir.yaml` wins for generated code shape and `spec.md` wins for operator-facing behavior. The drift validator is responsible for keeping that situation rare and visible.

## Per-slice fan-out (D5)

### Why this section is short

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
        candidate: identity-api
  - name: identity-service
    status: pending
    target: omnia@v1
    project: identity-service
    depends-on: [identity-contracts]
    sources:
      - key: docs
        candidate: identity-api
      - key: legacy
        candidate: identity-api
```

The same `Candidate` may appear in more than one slice's `sources[]` when both slices need the same Evidence — this is the fan-in side, not fan-out. Candidate fusion (D2) proposes one slice per `(target, candidate-group)` pair; the operator may split or merge proposed slices at Gate 1.

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
ir-path: /workspace/.specify/slices/identity-service/ir.yaml
artifacts:
  proposal: proposal.md
  design: design.md
  tasks: tasks.md
  specs:
    - specs/identity/spec.md
  fusion: fusion.yaml
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
- `build` consumes `ir.yaml` and rendered artifacts.
- `merge` consumes build reports and target-specific validation state.
- Any agent-generated code must still pass target-local validation before `status: success`.

### First-party target migration

The first migration path should be:

1. `contracts` first, because API contracts are already structured outputs.
2. `omnia` second, because Rust crate generation benefits most from typed requirements, APIs, configuration, and replay examples.
3. `vectis` third, because UI layout, assets, tokens, and `composition.yaml` need the widest IR shape.

## Adapter execution mode (D9)

Source and target adapters declare a closed `execution` field on their respective `adapter.yaml`:

```yaml
# adapters/sources/<name>/adapter.yaml
execution: executable     # or `agent-fallback`
```

The two values are:

- `**executable**` — `enumerate` and `extract` (sources) or `build` and `merge` (targets) are dispatched through a declared WASI tool or a deterministic Rust adapter path. Inputs and outputs validate against the schemas committed in this RFC. Required for first-party adapters before RFC-29 ships.
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

## Acceptance proof (D7)

RFC-29 is complete only when the acceptance suite proves the full path — fan-in twice (Candidates and Evidence), fan-out once (across slices):

```text
documentation + code-typescript
        -> source enumerate                 (fan-in #1: Candidate sets)
        -> plan propose --dry-run           (CLI proposes structural groups; agent binds each group to one or more targets and writes the slices via plan add)
        -> per slice:
             source extract                 (fan-in #2: Evidence per source)
             slice synthesize               (one Evidence map -> one IR)
             ir.yaml + artifacts + fusion.yaml
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
      fusion.yaml
      ir.yaml                                # target: contracts@v1
      build/report.yaml
    slices/identity-service/
      evidence/docs.yaml
      evidence/legacy.yaml
      proposal.md
      specs/identity/spec.md
      design.md
      tasks.md
      fusion.yaml
      ir.yaml                                # target: omnia@v1; sources include docs + legacy
      build/report.yaml                      # prior-slices cites identity-contracts/build/report.yaml
```

Required assertions:

- `specrun source enumerate` produces schema-valid candidates for both sources.
- `specrun plan propose --dry-run --format json` returns one structural group for the shared candidate (`rule: exact-id`), validates against `proposal.schema.json`, and writes nothing.
- The fixture's `/spec:plan` agent step (or the test harness simulating it) consumes the JSON, decides the per-group target binding (`contracts@v1` + `omnia@v1`), and issues two `specrun plan add` calls producing two single-target slices with `identity-service.depends-on: [identity-contracts]`.
- `specrun plan propose` without `--dry-run` exits non-zero with `propose-target-binding-required` (proves Stage B2 is gated).
- `specrun source extract` writes schema-valid Evidence for every `(slice, source)` pair.
- `specrun slice synthesize` writes valid artifacts, `fusion.yaml`, and `ir.yaml` for each slice.
- `specrun slice validate` catches no provenance, fusion, or IR drift on either slice.
- Each slice builds independently against its single bound target; `identity-service`'s build request carries a `prior-slices[]` entry pointing at `identity-contracts/build/report.yaml`.
- `specrun plan next` orders execution so `identity-contracts` reaches `merged` before `identity-service` starts.
- Re-running the full flow with unchanged inputs produces byte-stable generated artifacts except for explicitly timestamped journal entries.
- **D8 invariant.** The two slices — which share the `docs:identity-api` candidate and the same `authority-overrides` — produce `ir.yaml` files whose `requirements[]` arrays are byte-identical, even though `identity-contracts` binds `contracts@v1` and `identity-service` binds `omnia@v1` (and the latter additionally fuses `legacy:identity-api`, which appears as extra `requirements[]` entries appended deterministically after the shared set). The shared-prefix assertion proves shape briefs do not leak into the requirements section.

## Schemas added by this RFC

Four new JSON Schemas are committed alongside this RFC. All are embedded in the `specify-schema` crate as `IR_JSON_SCHEMA`, `BUILD_REQUEST_JSON_SCHEMA`, `BUILD_REPORT_JSON_SCHEMA`, and `PROPOSAL_JSON_SCHEMA` constants and reached through the existing `compile_schema` / `validate_value` plumbing. Field names are kebab-case on disk; top-level shapes are closed (`additionalProperties: false`); reusable closed enums (`kebabName`, `targetRef`, `requirementStatus`, `authorityClass`, the seven id grammars) live under `$defs` and are mirrored byte-identically with the matching `$defs` blocks in `evidence.schema.json`, `fusion.schema.json`, and `plan.schema.json`.

The IR, build-request, and build-report schemas key on `(slice, target)` per D5 — none of them carries an `outputs[]` or `output-id` field. A future RFC that re-opens multi-target slices would need to widen all three schemas and revisit the lifecycle / merge contract.

`schemas/discovery/proposal.schema.json` (returned by `specrun plan propose --dry-run`) is described inline in §"Candidate fusion engine (D2) → Stage B1". It is intentionally smaller than the other three — it carries no target binding — because target binding is agent-driven in v1.

### Schema A — `schemas/slice/ir.schema.json`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://github.com/augentic/specify-cli/schemas/slice/ir.schema.json",
  "title": "Specify slice ir.yaml",
  "description": "Validates a slice's typed intermediate representation per RFC-29 §Typed slice IR. Generated by `specrun slice synthesize` and regenerated whole on re-synthesis. Operators edit `spec.md` / `design.md` / `tasks.md`; `ir.yaml` is the machine view target builders consume. One slice binds one target (RFC-29 D5 §Per-slice fan-out) — `target` is a scalar, not an array. Drift between `ir.yaml.requirements[].id` and `spec.md` `REQ-*` ids is reported as `slice-ir-requirement-drift`; drift between `ir.yaml.requirements[].sources` and `fusion.yaml` is reported as `slice-ir-fusion-drift`; drift between `ir.yaml.target` / `ir.yaml.project` and `plan.yaml.slices[<slice>].target` / `.project` is reported as `slice-ir-target-drift`. Closed top-level shape — unknown fields are rejected.",
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
    "domain-model",
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
      "items": { "$ref": "#/$defs/irSource" }
    },
    "target": { "$ref": "#/$defs/targetRef" },
    "project": { "type": ["string", "null"] },
    "requirements": {
      "type": "array",
      "items": { "$ref": "#/$defs/irRequirement" }
    },
    "domain-model": { "$ref": "#/$defs/irDomainModel" },
    "apis": { "$ref": "#/$defs/irApis" },
    "configuration": {
      "type": "array",
      "items": { "$ref": "#/$defs/irConfiguration" }
    },
    "technical-logic": { "$ref": "#/$defs/irTechnicalLogic" },
    "observability": {
      "type": "array",
      "items": { "$ref": "#/$defs/irObservability" }
    },
    "tasks": {
      "type": "array",
      "items": { "$ref": "#/$defs/irTask" }
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
    "irSource": {
      "type": "object",
      "additionalProperties": false,
      "required": ["key", "adapter", "candidate", "authority"],
      "properties": {
        "key":           { "$ref": "#/$defs/kebabName" },
        "adapter":       { "$ref": "#/$defs/kebabName" },
        "candidate":     { "$ref": "#/$defs/kebabName" },
        "authority":     { "$ref": "#/$defs/authorityClass" },
        "evidence-path": { "type": "string" }
      }
    },
    "irRequirement": {
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
    "irDomainModel": {
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
    "irApis": {
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
    "irConfiguration": {
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
    "irTechnicalLogic": {
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
    "irObservability": {
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
    "irTask": {
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
    "ir-path",
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
    "ir-path":        { "type": "string" },
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
        "fusion":   { "$ref": "#/$defs/relativeArtifactPath" }
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

## Journal events

The closed `Event` / `EventKind` taxonomy in `crates/domain/src/journal.rs` gains the following kebab-case event kinds. Wire ids are normative; Rust variants follow the existing `#[serde(rename = …)]` pattern.


| Event                             | When                                                                                                                                                         |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `source.enumerate.cache-hit`      | Candidate set was read from cache.                                                                                                                           |
| `source.enumerate.cache-miss`     | Source-adapter `enumerate` ran.                                                                                                                              |
| `source.execution.agent-fallback` | A source-adapter operation ran in `agent-fallback` mode (`enumerate` or `extract`).                                                                          |
| `slice.extract.cache-hit`         | (Existing) Evidence was read from cache.                                                                                                                     |
| `slice.extract.cache-miss`        | (Existing) Source-adapter `extract` ran.                                                                                                                     |
| `slice.extract.completed`         | (Existing) Evidence file was successfully persisted.                                                                                                         |
| `slice.synthesize.started`        | `specrun slice synthesize` began for a slice.                                                                                                                |
| `slice.synthesize.completed`      | `specrun slice synthesize` finished and all artifacts (`proposal.md`, `spec.md`, `design.md`, `tasks.md`, `fusion.yaml`, `ir.yaml`) validated and persisted. |
| `slice.synthesize.failed`         | `specrun slice synthesize` aborted; prior artifacts left intact where possible.                                                                              |
| `slice.build.started`             | `/spec:build` (or `specrun slice build`) began work on a slice.                                                                                              |
| `slice.build.succeeded`           | A slice's build report validated with `status: success`.                                                                                                     |
| `slice.build.failed`              | A slice's build report carried `status: failure` or failed schema validation.                                                                                |
| `slice.merge.started`             | `/spec:merge` began work on a slice.                                                                                                                         |
| `slice.merge.succeeded`           | A slice's merge report validated with `status: success`.                                                                                                     |
| `slice.merge.failed`              | A slice's merge report carried `status: failure` or failed schema validation.                                                                                |
| `target.execution.agent-fallback` | A target-adapter operation ran in `agent-fallback` mode.                                                                                                     |
| `slice.ir.show.requested`         | Operator invoked `specrun slice ir show` (audit-only; useful for measuring IR-consumer adoption).                                                            |


## Error discriminants and exit codes

`Exit::from(&Error)` in `src/runtime/output.rs` is the single source of truth for the wire contract; this RFC adds the closed `Error` variants below. The CLI dispatch table maps each one to a fixed exit code.


| Error variant (kebab-case discriminant)           | Exit | Cause                                                                                                   |
| ------------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------------- |
| `slice-ir-schema`                                 | 2    | `ir.yaml` does not match `schemas/slice/ir.schema.json`.                                                |
| `slice-ir-requirement-drift`                      | 2    | `ir.yaml.requirements[].id` set differs from `spec.md` `REQ-*` set.                                     |
| `slice-ir-fusion-drift`                           | 2    | `ir.yaml.requirements[].sources` disagrees with `fusion.yaml`.                                          |
| `slice-ir-target-drift`                           | 2    | `ir.yaml.target` (or `ir.yaml.project`) disagrees with `plan.yaml.slices[<slice>].target` / `.project`. |
| `slice-ir-source-orphan`                          | 2    | An IR provenance entry references a source key absent from `ir.yaml.sources[].key`.                     |
| `slice-ir-cross-ref-orphan`                       | 2    | A `satisfies[]` `REQ-*` reference does not exist in `requirements[].id`.                                |
| `slice-ir-id-grammar`                             | 2    | A REQ / TASK / DEC / TYP / OP / CFG / OBS id does not match its closed three-digit grammar.             |
| `target-build-request-schema`                     | 2    | A build request fails `schemas/target/build-request.schema.json`.                                       |
| `target-build-report-schema`                      | 2    | A build report fails `schemas/target/build-report.schema.json`.                                         |
| `target-build-success-with-critical-finding`      | 2    | A build report sets `status: success` while carrying a finding at severity `critical`.                  |
| `target-build-prior-slice-not-built`              | 2    | A build request's `prior-slices[]` entry names a slice that has not produced a persisted build report.  |
| `adapter-execution-mode-required`                 | 2    | An adapter manifest does not declare `execution`.                                                       |
| `adapter-execution-agent-fallback-cache-conflict` | 2    | An adapter manifest sets `execution: agent-fallback` together with any cache mode other than `opt-out`. |
| `propose-target-binding-required`                 | 2    | `specrun plan propose` was invoked without `--dry-run` in v1; target binding stays agent-driven until Stage B2 ships. |


`EXIT_VALIDATION_FAILED = 2` is the only new code RFC-29 needs. Adapter resolution failures, sandbox preopen failures, WASI tool runtime failures, and I/O errors keep the existing `EXIT_GENERIC_FAILURE = 1` mapping.

## Implementation plan

A PR-sized breakdown of these waves lands in a companion `rfc-29-plan.md` (mirroring the [rfc-34-core-rules.md](./rfc-34-core-rules.md) / [rfc-34-plan.md](./rfc-34-plan.md) split). Each wave owns a defined set of new schemas, error variants, and journal events from the tables above.

### Wave A - Source runner and cache integration

1. Add the closed `execution: executable | agent-fallback` field to `schemas/source.schema.json`; thread it through `SourceAdapter` parse and add `adapter-execution-mode-required` / `adapter-execution-agent-fallback-cache-conflict` `Error` variants.
2. Add CLI DTOs and clap surfaces for `specrun source enumerate` and `specrun source extract`.
3. Reuse `SourceAdapter::resolve` and `SourceOperation::artifact_name`; branch dispatch on `execution`.
4. Route `executable` operations through declared WASI tools; route `agent-fallback` operations through the existing agent-run path but force `cache: opt-out` and emit `source.execution.agent-fallback`.
5. Validate candidate output against `candidate.schema.json` and Evidence output against `evidence.schema.json` before writes.
6. Add `source.enumerate.cache-{hit,miss}` cache events and update `specrun source resolve --explain` to show both operations.
7. Pin the `enumerate` cache fingerprint inputs explicitly in code and tests: source identity (path or value sha256) + adapter `name@version` + `enumerate` brief sha256 + sorted declared-tool versions.

### Wave B - Plan propose (Stage B1 only)

Stage B2 (full writer) is explicitly deferred — see §"Candidate fusion engine (D2) → Stage B2" and the new "Candidate target-axis vocabulary" open question.

1. Reuse the existing `Discovery` model in `crates/domain/src/discovery/` (parse, `resolve_candidate`, `check_alias_collisions` are already implemented and tested). No new parsing.
2. Implement the structural grouper as a pure function: `discovery::propose::group(&Discovery) -> Vec<Group>` covering rules 1 (exact id), 2 (exact alias), and 3 (transitive cross-reference). Surface diagnostic-only textual-similarity matches under `tentative_merges`.
3. Commit `schemas/discovery/proposal.schema.json` and embed it as `PROPOSAL_JSON_SCHEMA` in `specify-schema`.
4. Add `specrun plan propose --dry-run --format json` that runs the grouper, validates the output against `proposal.schema.json`, and prints. Reject every other `propose` form with `propose-target-binding-required` until Stage B2 lands.
5. Update `/spec:plan` to call `specrun source enumerate` per source, then `specrun plan propose --dry-run`, then issue one `specrun plan add` per `(group, target)` pair the agent decides on. `specrun plan add` continues to be the only writer.
6. Add fixtures for exact match, alias match, transitive cross-reference, tentative non-match, and per-group multi-target fan-out (the agent emits two `plan add` calls — the fixture asserts both slices land with the expected `target` and `depends-on`).

### Wave C - Synthesis engine and IR

1. Commit `schemas/slice/ir.schema.json` and embed it as `IR_JSON_SCHEMA` in the `specify-schema` crate alongside the existing `*_JSON_SCHEMA` constants.
2. Add `SynthesisModel` and the production authority resolver to `specify-domain`.
3. Implement renderers for `proposal.md`, `spec.md`, `design.md`, `tasks.md`, `fusion.yaml`, and `ir.yaml` from one in-memory model (no second-pass Markdown reparse).
4. Enforce D8: requirements section is a function of `(Evidence[], authority-overrides)` only; add a unit test that synthesises two slices binding different `target` values against the same Evidence map and asserts the shared-prefix of their `requirements[]` arrays is byte-identical.
5. Add `specrun slice synthesize` plus `slice.synthesize.{started,completed,failed}` journal events.
6. Add `specrun slice ir show <slice> [--format json]`.
7. Update `/spec:refine` to call the CLI command instead of hand-coding synthesis.
8. Extend `specrun slice validate` with the six IR drift checks and their `Error` variants (`slice-ir-{schema,requirement-drift,fusion-drift,target-drift,source-orphan,cross-ref-orphan,id-grammar}`).

### Wave D - Plan loader confirmation

No plan-schema change. `plan.yaml.slices[].target` / `slices[].project` stay singular per D5. This wave is a small chassis confirmation, not a feature:

1. Add a parser regression test asserting that `plan.yaml.slices[]` rejects an `outputs[]` field if a stray draft ever introduces one. This pins D5 in code rather than only in this RFC.
2. Confirm `specrun plan add` / `specrun plan amend` continue to refuse an `--output` flag; the only legal target binding is `--target <name@vN> [--project <slug>]`.
3. Confirm `specrun plan propose --dry-run` (Wave B / Stage B1) emits one structural group per matched candidate set and that `/spec:plan`'s agent step issues one `specrun plan add` call per `(group, target)` pair with `depends-on` edges populated from operator-declared ordering hints in `discovery.md`.

### Wave E - Target build envelope

1. Commit `schemas/target/build-request.schema.json` and `schemas/target/build-report.schema.json` and embed both as `BUILD_REQUEST_JSON_SCHEMA` / `BUILD_REPORT_JSON_SCHEMA` in `specify-schema`. Both are keyed on `(slice, target)`; no `output-id`.
2. Add the closed `execution: executable | agent-fallback` field to `schemas/target.schema.json` symmetric with the source side; thread it through `TargetAdapter` parse.
3. Add `slice.build.{started,succeeded,failed}`, `slice.merge.{started,succeeded,failed}`, and `target.execution.agent-fallback` journal events.
4. Wire `prior-slices[]` population in the build-request builder: for each entry in the current slice's `plan.yaml.slices[].depends-on`, resolve the depended-on slice's `build/report.yaml` path and reject (`target-build-prior-slice-not-built`) when missing.
5. Update `contracts` build to consume the build request and emit a report (executable mode via WASI tool).
6. Update `omnia` build to consume `ir.yaml` for crate/test/guest generation, read `prior-slices[]` to pick up upstream contract schemas, and emit a report (executable mode where deterministic; `agent-fallback` for the model-assisted phases that remain).
7. Update `vectis` build after the IR has enough UI/layout structure.
8. Integrate RFC-28 findings into build reports; enforce `target-build-success-with-critical-finding` at the CLI boundary.

### Wave F - Proof fixtures and docs

1. Add the RFC-29 end-to-end fixture (D7): two slices over two sources, joined by `depends-on`, each binding one target. Include the D8 invariant assertion (the two slices share a candidate; the shared-prefix of their `requirements[]` arrays is byte-identical).
2. Update `docs/explanation/concepts.md` and `docs/explanation/adapter-anatomy.md` to distinguish source fan-in (Candidates + Evidence) from slice fan-out (plan-level decomposition with `depends-on`). Reaffirm "one slice, one target" alongside the existing `docs/explanation/decision-log.md` entry.
3. Update CLI reference pages for source, plan, slice, and target build reports — none of them gain an `outputs[]` field.
4. Update acceptance docs with the new proof command sequence (two `specrun plan add` calls, one per target, second with `--depends-on`).

## Migration

Existing projects continue to work without any change to `plan.yaml`:

- `plan.yaml.slices[]` keeps its existing one-`target`, optional-`project` shape. There is no `outputs[]` desugar to perform, and no `primary` literal to reserve. Any draft pre-RFC-29 plan referring to `outputs[]` is rejected as an unknown field on the existing plan schema.
- Slices without `ir.yaml` validate under the pre-RFC-29 compatibility path unless re-synthesised.
- Target build briefs may initially read Markdown and ignore `ir.yaml`, but first-party targets must migrate before RFC-29 is marked implemented.
- Source adapters may initially keep agent-run briefs, but first-party adapters must declare `execution: executable` before RFC-29 is marked implemented. Third-party adapters MAY remain `execution: agent-fallback` indefinitely.
- Existing first-party adapter manifests must add the new `execution` field at first read; the loader rejects missing values with `adapter-execution-mode-required` rather than defaulting silently. The companion `rfc-29-plan.md` PR list pins which adapters land each migration.
- Cross-repo references that anticipated the dropped multi-output model — notably `rfcs/next/rfc-30-init.md`'s "`slices[].target` → `outputs[]` once RFC-29 lands" line and `rfcs/roadmap.md` §RM-06's "D5 multi-output plan entries" follow-on bullet — must be retracted in the same PR train that lands D5's per-slice form.

Once a slice has been synthesized by an RFC-29-aware CLI, `ir.yaml` becomes required for that slice and drift validation applies.

## Non-goals

- No hosted execution or cloud runner. RFC-29 is local-first.
- No replacement of `spec.md` as the human behavioral artifact or baseline merge input.
- No graph database or global knowledge store for synthesis.
- No automatic merging of semantically similar candidates without exact id, alias, or operator-seeded evidence.
- **No multi-target slices.** A slice binds exactly one target adapter / project (D5). Cross-target fan-out is plan-level, achieved by decomposing a change into multiple slices joined by `slices[].depends-on`. RFC-29 introduces no `outputs[]` array, no per-output lifecycle, no per-output build envelope, no per-output `.metadata.yaml` keying, and no per-output journal events. A future RFC that wishes to re-open this question must first amend `docs/explanation/decision-log.md` §"One plan entry, one project" and account for the multi-baseline merge contract that decision deliberately rules out.
- No target-specific behavior in core synthesis beyond reading the bound target's `shape` brief to parameterise IR structure (D8). Shape briefs MUST NOT influence `requirements[]` or any provenance-bearing field.
- No commitment to per-target determinism on day one. RFC-29 commits only to a stable build envelope and validation contract; per-target determinism milestones are tracked in each target adapter's manifest and changelog.

## Alternatives considered

### Keep synthesis in skills

Rejected. The current skill-driven synthesis can work in practice, but it cannot be tested, cached, or reused as a framework guarantee. The fan-in/fan-out promise depends on a stable reconciliation engine.

### Make Markdown the only IR

Rejected. Markdown is excellent for review and version control, but target builders need structured requirements, sources, APIs, configuration, tasks, and examples. Parsing Markdown in every target would duplicate fragile logic and create inconsistent generators.

### Make `fusion.yaml` the IR

Rejected. `fusion.yaml` answers "why did this requirement win?" not "what should targets build?" It remains an audit index.

### Multi-output slices (one slice, many targets)

Considered in an earlier draft of this RFC under "D5 Multi-output plan entries" and rejected. A single slice driving multiple targets would have required the per-slice lifecycle to manage multiple project roots, multiple `shape` briefs, multiple build reports, and (most awkwardly) multiple baseline merges inside one `refining -> refined -> built -> merged` walk. That contradicts the existing "one plan entry, one project" decision (see [decision log](../docs/explanation/decision-log.md#one-plan-entry-one-project), `docs/reference/targets/index.md`, and `docs/explanation/adapter-anatomy.md`) and would have forced rework of `specrun slice merge`'s single-baseline contract.

The accepted model is per-slice fan-out (D5): one slice, one target, one baseline, one merge. Cross-target changes decompose into N slices joined by `depends-on`. This costs some duplicate Evidence extraction when two slices fuse the same Candidate, but the duplication is bounded by the extraction cache (RFC-27) and pays for a single, well-defined per-slice lifecycle.

### Allow arbitrary semantic candidate auto-merge

Rejected for v1. It would move too much judgment into the framework. Exact ids and aliases are reviewable; textual similarity is advisory until an operator accepts it.

## Open questions

The five open questions from the original draft are resolved as normative decisions in this revision:

1. **IR on-disk format.** Resolved: YAML on disk; JSON only via `specrun slice ir show <slice> --format json`. See §"Typed slice IR (D4)".
2. **Shape-brief scope.** Resolved as **D8** — shape briefs may parameterise IR structure for `design-model` / `apis` / `configuration` / `technical-logic` / `observability` / `tasks` but not `requirements[]` or any provenance-bearing field. See §"Slice synthesis engine (D3) → Shape-brief scope (D8)".
3. **Per-target determinism.** Dropped from the RFC contract; tracked per target adapter. RFC-29 commits only to envelope and validation determinism. See §"Non-goals".
4. **Slice fan-out shape.** Resolved as **D5** — fan-out is plan-level (one slice per target, joined by `slices[].depends-on`). The dropped multi-output-per-slice form is documented in §"Alternatives considered → Multi-output slices". Cross-slice build context flows through `prior-slices[]` on the build envelope (§"Target build envelope (D6)").
5. **Adapter execution fallback.** Resolved as **D9** — closed `execution: executable | agent-fallback` enum on adapter manifests; first-party adapters must be `executable`, third-party adapters may be `agent-fallback` indefinitely. See §"Adapter execution mode (D9)".

The per-slice fan-out revision (D5) opens one new question that RFC-29 deliberately does **not** answer in v1:

6. **Candidate target-axis vocabulary (Stage B2 prerequisite).** Under D5, `specrun plan propose` would need a deterministic policy for turning a candidate group into `(group, target)` slices. The four candidates considered are:

   - **6.a Target hints on Candidates.** Source adapters tag each candidate with a closed `axes: [api, service, ui, …]` enum at `enumerate` time; `propose` cross-products groups by their members' union of axes. Cleanest long-term shape; requires extending `schemas/discovery/candidate.schema.json` and per-source-adapter authoring discipline. Probably needs its own RFC.
   - **6.b Cross-product over plan-bound targets.** Emit `|groups| × |bound-targets|` slices and let the operator delete the irrelevant ones at Gate 1. Over-generates badly at scale; not viable past a handful of targets.
   - **6.c Operator post-amend.** `propose` emits `target: null` rows; operator runs `specrun plan amend --target` per slice. Pushes mechanical work onto the operator.
   - **6.d Status quo: agent decides.** Per the **D2 Stage B1** decision in this revision, this is what v1 does. Honest about the judgment involved; keeps the CLI free of an arbitrary heuristic. Costs us a deterministic acceptance assertion on `plan.yaml.slices[]` byte-stability and keeps target binding out of the CLI's audit / journal trail.

   This question is the explicit blocker for **D2 Stage B2** (`specrun plan propose` as a full writer). It is not a blocker for any other RFC-29 wave; D1, D3, D4, D5, D6, and D7 all land against the Stage B1 + agent form.

## References

- [RFC-25: Workflow](../done/rfc-25-workflow.md)
- [RFC-27: Synthesis Sharpening](../done/rfc-27-synthesis.md)
- RFC-28: Engineering Standards — Codex Contract and Findings
- RFC-32: Engineering Standards — Deterministic Enforcement
- [Core concepts](../../docs/explanation/concepts.md)
- [Anatomy of an adapter](../../docs/explanation/adapter-anatomy.md)
- [Claim fusion](../../plugins/spec/references/synthesis/claim-fusion.md)
- [Reconciliation index](../../plugins/spec/references/synthesis/fusion.md)

