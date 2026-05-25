# RFC-29: Fan-In/Fan-Out Code Generation Contract

> Status: Draft - Depends: [RFC-25](../done/rfc-25-workflow.md), [RFC-27](../done/rfc-27-synthesis.md), [RFC-28](../rfc-28-codex-rules.md) - Enables: provable multi-source, multi-target Specify generation

## Abstract

Specify's architectural promise is a fan-in/fan-out workflow: multiple sources produce evidence; core synthesis reconciles that evidence into an intermediate representation; one or more targets generate code and other deliverables from that representation.

RFC-25 and RFC-27 established the vocabulary and most of the operator-facing workflow. The current system has source adapters, target adapters, `Candidate`, `Evidence`, provenance, authority, `fusion.yaml`, target `shape` briefs, and the `refine -> build -> merge` loop. The gap is that several load-bearing steps are still implemented as agent discipline rather than deterministic framework contract, and v1 only models one target output per slice.

This RFC turns the promise into an end-to-end contract by adding:

1. **Executable source operations** - first-class `specify source enumerate` and `specify source extract` commands that run source adapters under the declared sandbox, cache, and journal contract.
2. **Deterministic plan-time fusion** - a CLI-owned candidate-fusion engine that proposes slice rows from `Candidate[]`, preserving operator review for ambiguous joins.
3. **Typed slice IR** - a machine-readable slice intermediate representation emitted by refine and used by target builders, while the existing Markdown artifacts remain the human review surface and baseline merge input.
4. **Multi-output slices** - a plan entry may target one or more output adapters/projects from the same synthesized slice IR.
5. **Target build contract** - target adapters consume the slice IR through a stable build envelope, with per-output validation, review findings, and merge gates.
6. **Proof fixtures** - acceptance coverage that exercises `N sources -> one slice IR -> M outputs`, not only isolated schema and fixture checks.

## Motivation

The current codebase can describe the fan-in/fan-out model, but it cannot yet prove it as a framework invariant.

The findings this RFC resolves:

| Finding | Current state | RFC-29 resolution |
| --- | --- | --- |
| Source operations are briefs, not executable CLI operations. | `specify source resolve` exists; `enumerate` and `extract` are agent-run instructions. | Add `specify source enumerate` and `specify source extract` with sandbox, cache, schema validation, and journal events. |
| Plan-time candidate fusion is agent-only. | `/spec:plan`'s `propose` sub-step reads `discovery.md` and calls `specify plan add`. | Add a deterministic `specify plan propose` engine that writes proposed entries through existing plan writers. |
| Slice-time synthesis has no production resolver. | CLI validates `spec.md`, Evidence, and `fusion.yaml`; it does not synthesize them. | Add a `specify slice synthesize` engine that emits artifacts, `fusion.yaml`, and the typed slice IR from the same model. |
| The intermediate representation is implicit. | `proposal.md`, `spec.md`, `design.md`, `tasks.md`, Evidence, and `fusion.yaml` together act as the IR, but target builders consume Markdown. | Add `.specify/slices/<slice>/ir.yaml` as generated machine-readable build input, with drift validation against rendered artifacts. |
| Fan-out is not first-class. | `plan.yaml.slices[].target` selects one target adapter. Workspace routing can send different slices to different projects, but one slice cannot generate multiple outputs. | Add `outputs[]` as the multi-output form, with existing `project` / `target` fields as the single-output shorthand. |
| Target codegen is adapter-brief discipline. | Target `build` briefs orchestrate generation, validation, and review, but no stable input/output envelope joins them to core synthesis. | Add a target build envelope keyed by output id; each target reports structured status, generated paths, validation commands, and RFC-28 review findings. |

The goal is not to remove agents from Specify. The goal is to move stable workflow and data-shape obligations into the CLI so agents can focus on judgment, repair, and domain-specific generation rather than reimplementing lifecycle and reconciliation rules.

## Principles

1. **Core owns reconciliation.** If a rule decides how sources combine, it belongs in the CLI or a CLI-owned schema, not only in a skill body.
2. **Markdown remains reviewable.** `proposal.md`, `spec.md`, `design.md`, and `tasks.md` stay the operator-facing artifacts. The IR is the machine view emitted from the same synthesis model.
3. **No second lifecycle.** Multi-output slices still have one slice lifecycle. Per-output status is detail under the slice, not a new plan-entry state machine.
4. **Targets consume, not synthesize.** Target adapters may shape synthesis and build outputs, but they do not create behavioral requirements or provenance.
5. **Agent fallback is explicit.** Where a target still needs model-assisted generation, the input and output envelope is stable and validation catches drift.
6. **Compatibility is additive.** Existing one-source, one-target plans keep working. New fields are either optional or desugar from existing fields.

## Normative decisions

| ID | Decision | Implementation consequence |
| --- | --- | --- |
| **D1 Source operation runner** | The CLI runs source adapter `enumerate` and `extract` operations. | Add `specify source enumerate` and `specify source extract`; route through `SourceAdapter::resolve`, declared tools, sandbox preopens, extraction cache, schema validation, and journal events. |
| **D2 Candidate fusion engine** | The CLI owns the structural `Candidate[] -> plan entries` proposal pass. | Add `specify plan propose`; `/spec:plan` invokes it after enumeration. Ambiguous joins emit review annotations and remain operator-amendable at Gate 1. |
| **D3 Slice synthesis engine** | The CLI owns `Evidence[] + target shape -> slice artifacts + fusion.yaml + ir.yaml`. | Add `specify slice synthesize <slice>`; retire the instruction that `/spec:refine` hand-codes synthesis. The engine uses the RFC-27 authority resolver as production code. |
| **D4 Typed slice IR** | Every synthesized slice carries `.specify/slices/<slice>/ir.yaml`. | Add `schemas/slice/ir.schema.json`; `specify slice validate` checks IR/artifact/fusion drift; target build reads the IR as its primary machine input. |
| **D5 Multi-output plan entries** | A slice may declare `outputs[]`; existing `project` and `target` fields are shorthand for one output. | Add schema support and CLI parse/write helpers. `/spec:build` builds every output; the slice reaches `built` only when required outputs pass. |
| **D6 Target build envelope** | Target adapters receive a stable per-output build request and return a stable build report. | Add `schemas/target/build-request.schema.json` and `schemas/target/build-report.schema.json`; reports may include RFC-28 findings. |
| **D7 Acceptance proof path** | The release is not complete until an end-to-end fixture demonstrates fan-in and fan-out together. | Add cross-repo tests for two sources feeding one slice IR that generates at least two outputs. |

## Operator surface

The default operator rhythm does not change:

```bash
/spec:plan identity-refresh source docs=documentation:./docs source legacy=code-typescript:./legacy
specify plan transition identity-refresh reviewed
/spec:execute
/spec:finalize identity-refresh
```

The new CLI surfaces are mostly lower-level breakouts:

```bash
specify source enumerate docs --format json
specify source extract docs password-reset --slice identity-password-reset --format json
specify plan propose --format json
specify slice synthesize identity-password-reset --format json
specify slice ir show identity-password-reset --format json
```

Multi-output slices are still planned once:

```bash
specify plan add identity-api \
  --sources docs=identity-api \
  --output service=omnia@v1:identity-service \
  --output contract=contracts@v1:identity-contracts
```

The shorthand stays legal:

```bash
specify plan add identity-api --target omnia@v1 --project identity-service
```

The shorthand desugars to one output:

```yaml
outputs:
  - id: primary
    target: omnia@v1
    project: identity-service
```

## Source operation runner (D1)

### Commands

Add two commands under the existing `specify source` family:

```bash
specify source enumerate <source-key> [--plan <name>] [--format json]
specify source extract <source-key> <candidate-id> --slice <slice> [--format json]
```

`<source-key>` resolves against `plan.yaml.sources.<key>`, not against adapter name. The command then resolves the adapter from `SourceBinding.adapter`.

### `enumerate`

`enumerate` runs the source adapter's `briefs.enumerate` operation under the source-adapter sandbox:

| Root | Mode | Contents |
| --- | --- | --- |
| `$SOURCE_DIR` | read-only | Bound source path when the source uses `path:`. |
| `$CAPABILITY_DIR` | read-only | Resolved source adapter manifest cache. |
| `$SCRATCH_DIR` | write-only | Per-operation scratch under `.specify/.cache/extractions/<adapter>/`. |
| `$PROJECT_DIR` | none | Not visible to the adapter operation. |

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

| Event | When |
| --- | --- |
| `source.enumerate.cache-hit` | Candidate set was read from cache. |
| `source.enumerate.cache-miss` | Adapter `enumerate` ran. |
| `slice.extract.cache-hit` | Evidence was read from cache. |
| `slice.extract.cache-miss` | Adapter `extract` ran. |
| `slice.extract.completed` | Evidence file was successfully persisted. |

`slice.extract.cache-*` already exists in RFC-27; this RFC adds the enumerate equivalents.

## Candidate fusion engine (D2)

### Command

```bash
specify plan propose [--format json]
```

`propose` reads:

- `plan.yaml.sources`;
- `discovery.md` candidate inventory;
- optional candidate aliases;
- optional workspace registry routing hints.

It writes `plan.yaml.slices[]` through the same `specify plan add` / `plan amend` writer paths that operators use.

### Matching algorithm

The first implementation is intentionally conservative:

1. Exact canonical `id` match across source keys -> one multi-source slice.
2. Exact alias match -> one multi-source slice, persisted with canonical candidate ids.
3. One candidate from a source has a source list that already names another source's candidate -> one multi-source slice.
4. Otherwise keep candidates separate unless the operator has pre-seeded a join in `discovery.md`.

The engine may compute textual similarity for diagnostics, but textual similarity alone does not auto-merge in v1. That keeps structural fusion deterministic and pushes uncertain merges to Gate 1.

### Review annotations

When the engine sees likely but unmerged candidates, it writes advisory annotations:

```markdown
## Tentative merges

- `docs:password-reset` may match `legacy:reset-password`; no alias or exact id match exists.
```

When exact or alias-matched summaries materially disagree, it writes `divergence: likely` through `specify plan amend --divergence likely` and adds a `## Likely divergences` block to `change.md`.

### Agent role

Agents may still suggest aliases, split oversized candidates, or explain tentative merges. They no longer decide the default structural merge in memory. If they want to change the plan, they call CLI writers.

## Slice synthesis engine (D3)

### Command

```bash
specify slice synthesize <slice> [--format json]
```

The command reads:

- slice metadata and target/output bindings;
- `plan.yaml.slices[].sources`;
- `evidence/*.yaml`;
- target `shape` briefs for every output target;
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

### Rendering

The synthesis engine renders Markdown artifacts from the typed model. It does not parse its own Markdown output to recover state during the same run.

`spec.md` stays the behavioral review artifact and baseline merge input. `ir.yaml` is the generated machine view used by target builds. `fusion.yaml` remains audit-only.

## Typed slice IR (D4)

### File

```text
.specify/slices/<slice>/ir.yaml
```

The IR is generated by `specify slice synthesize` and regenerated whole on re-synthesis. Operators should edit `spec.md` or `design.md`, not `ir.yaml`; re-running synthesize will overwrite `ir.yaml`.

### Shape

High-level schema:

```yaml
version: 1
slice: identity-password-reset
sources:
  - key: docs
    adapter: documentation
    candidate: password-reset
outputs:
  - id: service
    target: omnia@v1
    project: identity-service
  - id: contract
    target: contracts@v1
    project: identity-contracts
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
```

The exact schema belongs in `specify-cli/schemas/slice/ir.schema.json`. Field names are kebab-case on disk.

### Drift validation

`specify slice validate` adds three checks:

| Finding | Meaning |
| --- | --- |
| `slice-ir-schema` | `ir.yaml` does not match `schemas/slice/ir.schema.json`. |
| `slice-ir-requirement-drift` | `ir.yaml.requirements[].id` differs from `spec.md` `REQ-*` ids. |
| `slice-ir-fusion-drift` | `ir.yaml.requirements[].sources` disagrees with `fusion.yaml` / `spec.md` provenance. |

Absence of `ir.yaml` is allowed for pre-RFC-29 slices and rejected for slices synthesized by an RFC-29-aware CLI.

### Build input

Target builders consume `ir.yaml` as their machine input and may also read rendered Markdown for context. If they disagree, `ir.yaml` wins for generated code shape and `spec.md` wins for operator-facing behavior. The drift validator is responsible for keeping that situation rare and visible.

## Multi-output slices (D5)

### Plan schema

Add optional `outputs[]` to `plan.yaml.slices[]`:

```yaml
slices:
  - name: identity-api
    status: pending
    sources:
      - key: docs
        candidate: identity-api
    outputs:
      - id: service
        target: omnia@v1
        project: identity-service
      - id: contract
        target: contracts@v1
        project: identity-contracts
```

Rules:

- `outputs[].id` is kebab-case and unique within the slice.
- `outputs[].target` is the existing `name@vN` target reference.
- `outputs[].project` is optional in single-project mode and required when the output routes to a workspace registry project.
- Existing `project` / `target` fields remain valid and are treated as a single output with id `primary`.
- A slice may not specify both `outputs[]` and the shorthand fields unless the values are byte-equivalent after desugaring.

### Lifecycle

The slice lifecycle stays:

```text
refining -> refined -> built -> merged
```

Per-output build detail is stored in `.metadata.yaml`:

```yaml
outputs:
  service:
    target: omnia@v1
    status: built
    generated-paths:
      - crates/identity_password_reset
  contract:
    target: contracts@v1
    status: built
    generated-paths:
      - contracts/identity.yaml
```

`/spec:build` transitions the slice to `built` only after every required output reports success. Optional outputs are deferred to a later RFC; RFC-29 outputs are all required.

### Workspace routing

In workspace mode, `/spec:build` may need to visit more than one project slot for one slice. The plan lock remains held at the workspace root. Build order follows `outputs[]` order. Each output build:

1. syncs/prepares the target project slot;
2. writes target-specific files in that slot;
3. records generated paths in the build report;
4. restores CWD to the workspace root before the next output.

`/spec:merge` validates every output before the single slice merge. Baseline spec merge still happens once.

## Target build envelope (D6)

### Build request

For each output, `/spec:build` constructs a build request:

```yaml
version: 1
slice: identity-api
output-id: service
target: omnia@v1
project-root: /workspace/.specify/workspace/identity-service
slice-dir: /workspace/.specify/slices/identity-api
ir: /workspace/.specify/slices/identity-api/ir.yaml
artifacts:
  proposal: proposal.md
  specs:
    - specs/identity/spec.md
  design: design.md
  tasks: tasks.md
shape-brief: /.../adapters/targets/omnia/briefs/shape.md
```

The schema lives at `schemas/target/build-request.schema.json`.

### Build report

Each target returns a build report:

```yaml
version: 1
slice: identity-api
output-id: service
target: omnia@v1
status: success
generated-paths:
  - crates/identity_api
validation:
  commands:
    - cargo check
    - cargo test
findings: []
```

The schema lives at `schemas/target/build-report.schema.json`. `findings[]` uses RFC-28's structured review finding shape.

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

## Acceptance proof (D7)

RFC-29 is complete only when the acceptance suite proves the full path:

```text
documentation + code-typescript
        -> source enumerate
        -> plan propose
        -> source extract
        -> slice synthesize
        -> ir.yaml + artifacts + fusion.yaml
        -> contracts output + omnia output
        -> validate + merge
```

Minimum fixture:

```text
tests/fixtures/rfc-29/fan-in-fan-out/
  sources/
    docs/
    legacy/
  expected/
    discovery.md
    plan.yaml
    slices/identity-api/
      evidence/docs.yaml
      evidence/legacy.yaml
      proposal.md
      specs/identity/spec.md
      design.md
      tasks.md
      fusion.yaml
      ir.yaml
      build-reports/
        service.yaml
        contract.yaml
```

Required assertions:

- `specify source enumerate` produces schema-valid candidates for both sources.
- `specify plan propose` creates one multi-source slice through exact id or alias match.
- `specify source extract` writes schema-valid Evidence for both sources.
- `specify slice synthesize` writes valid artifacts, `fusion.yaml`, and `ir.yaml`.
- `specify slice validate` catches no provenance, fusion, or IR drift.
- Two target outputs build from the same IR.
- Re-running the full flow with unchanged inputs produces byte-stable generated artifacts except for explicitly timestamped journal entries.

## Implementation plan

### Wave A - Source runner and cache integration

1. Add CLI DTOs and clap surfaces for `source enumerate` and `source extract`.
2. Reuse `SourceAdapter::resolve` and `SourceOperation::artifact_name`.
3. Route operation execution through declared WASI tools where present; keep agent-run fallback only behind an explicit skill path while first-party adapters migrate.
4. Validate candidate and Evidence outputs before writes.
5. Add enumerate cache events and update `source resolve --explain` to show both operations.

### Wave B - Plan propose

1. Add a `Discovery` API that exposes canonical ids, aliases, source keys, and summaries as typed values.
2. Implement exact-id and alias-based candidate fusion.
3. Add `specify plan propose`.
4. Update `/spec:plan` to call `source enumerate` per source, then `plan propose`.
5. Add fixtures for exact match, alias match, tentative non-match, and likely divergence.

### Wave C - Synthesis engine and IR

1. Add `SynthesisModel` and production authority resolver to `specify-domain`.
2. Add `schemas/slice/ir.schema.json`.
3. Implement renderers for `proposal.md`, `spec.md`, `design.md`, `tasks.md`, `fusion.yaml`, and `ir.yaml`.
4. Add `specify slice synthesize`.
5. Update `/spec:refine` to call the CLI command instead of hand-coding synthesis.
6. Extend `specify slice validate` with IR drift checks.

### Wave D - Multi-output planning

1. Add `PlanEntry.outputs` and `OutputRef` types.
2. Keep `project` / `target` as single-output shorthand in the loader and serializer.
3. Add `specify plan add --output <id>=<target>[:<project>]` and matching amend support.
4. Update `specify plan validate` for output id uniqueness, target version resolution, and workspace routing.
5. Update `/spec:execute`, `/spec:build`, and `/spec:merge` skill bodies for output fan-out.

### Wave E - Target build envelope

1. Add build request/report schemas and Rust DTOs.
2. Update contracts build to consume the build request and emit a report.
3. Update Omnia build to consume `ir.yaml` for crate/test/guest generation and emit a report.
4. Update Vectis build after the IR has enough UI/layout structure.
5. Integrate RFC-28 findings into build reports.

### Wave F - Proof fixtures and docs

1. Add the RFC-29 end-to-end fixture.
2. Update `docs/explanation/concepts.md` and `docs/explanation/adapter-anatomy.md` to distinguish source fan-in, slice IR, and target fan-out.
3. Update CLI reference pages for source, plan, slice, and target build reports.
4. Update acceptance docs with the new proof command sequence.

## Migration

Existing projects continue to work:

- Plans with `target` and `project` but no `outputs[]` load as one `primary` output.
- Slices without `ir.yaml` validate under the pre-RFC-29 compatibility path unless re-synthesized.
- Target build briefs may initially read Markdown and ignore `ir.yaml`, but first-party targets must migrate before RFC-29 is marked implemented.
- Source adapters may initially keep agent-run briefs, but first-party adapters must expose executable operations before RFC-29 is marked implemented.

Once a slice has been synthesized by an RFC-29-aware CLI, `ir.yaml` becomes required for that slice and drift validation applies.

## Non-goals

- No hosted execution or cloud runner. RFC-29 is local-first.
- No replacement of `spec.md` as the human behavioral artifact or baseline merge input.
- No graph database or global knowledge store for synthesis.
- No automatic merging of semantically similar candidates without exact id, alias, or operator-seeded evidence.
- No optional outputs in v1. Every listed output must build before the slice reaches `built`.
- No target-specific behavior in core synthesis beyond reading target `shape` briefs and rendering target-neutral IR fields.
- No requirement that all target generation be deterministic on day one; the envelope and validation contract are deterministic even when generation uses an agent.

## Alternatives considered

### Keep synthesis in skills

Rejected. The current skill-driven synthesis can work in practice, but it cannot be tested, cached, or reused as a framework guarantee. The fan-in/fan-out promise depends on a stable reconciliation engine.

### Make Markdown the only IR

Rejected. Markdown is excellent for review and version control, but target builders need structured requirements, sources, APIs, configuration, tasks, and examples. Parsing Markdown in every target would duplicate fragile logic and create inconsistent generators.

### Make `fusion.yaml` the IR

Rejected. `fusion.yaml` answers "why did this requirement win?" not "what should targets build?" It remains an audit index.

### One plan entry per target

Rejected as the primary fan-out model. It preserves existing lifecycle semantics but duplicates refine, evidence extraction, authority resolution, and operator review for each target. A single slice should synthesize once and build many outputs when the behavior is shared.

### Allow arbitrary semantic candidate auto-merge

Rejected for v1. It would move too much judgment into the framework. Exact ids and aliases are reviewable; textual similarity is advisory until an operator accepts it.

## Open questions

1. Should `ir.yaml` be YAML for consistency with existing Specify artifacts, or JSON for easier tool consumption? Current preference: YAML on disk, JSON available via `specify slice ir show --format json`.
2. Should target `shape` briefs influence the IR itself or only the rendered `design.md` / `tasks.md` fields? Current preference: shape can influence target-neutral design/task structure, but not behavioral requirements.
3. How much of Omnia crate generation can be made deterministic from the IR before model-assisted repair remains necessary?
4. Should `outputs[]` support output dependencies in a later RFC, for example contracts before service code? Current preference: keep RFC-29 order-only.
5. Should `source enumerate` and `source extract` require WASI tools for all third-party adapters, or allow agent-run briefs indefinitely? Current preference: first-party executable, third-party agent fallback allowed with explicit validation.

## References

- [RFC-25: Workflow](../done/rfc-25-workflow.md)
- [RFC-27: Synthesis Sharpening](../done/rfc-27-synthesis.md)
- RFC-28: Codex Resolution and Structured Review Findings
- RFC-31: WorkspaceModel and Declarative Rule Execution
- [Core concepts](../../docs/explanation/concepts.md)
- [Anatomy of an adapter](../../docs/explanation/adapter-anatomy.md)
- [Claim fusion](../../plugins/spec/references/synthesis/claim-fusion.md)
- [Reconciliation index](../../plugins/spec/references/synthesis/fusion.md)
