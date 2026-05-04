# RFC-13: Extensibility

> Status: Draft · Supersedes: earlier draft at this path (artifact-adoption-only framing) · Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-8](archive/rfc-8-api-contracts.md), [RFC-9](archive/rfc-9-platform.md), [RFC-12](archive/rfc-12-refine-rfc-8.md) · Enables: [RFC-14](rfc-14-workspaces.md)

## Abstract

A capability describes how Specify's existing `define → build → merge` loop handles a class of artefacts. RFC-13 reframes the runtime to match: the **immutable core** is the loop engine plus capability-agnostic scaffolding (init, migrate, capability resolver, change driver, and clone resolver). Workflow is treated as a normal first-party capability that owns `workflow.yaml`.

Today's `schema.yaml` surface admits only `{ name, version, description, domain?, pipeline }`, which is too small to carry that contract and uses the wrong noun. This RFC renames the extension primitive to **capability** and makes `pipeline:` an explicit member of the capability manifest alongside new fields (`artifacts:` and optional `consumes:`) so a capability can describe its phase briefs, artefacts, and read-only dependencies.

## Motivation

### The core isn't actually core

Specify's current surface promises extensibility and breaks it inside the binary:

- `specify-cli/src/cli.rs` carries `Vectis { action: VectisAction }` and `Contract { action: ContractAction }` as top-level subcommands, dispatched through `specify_vectis` and `specify::validate_baseline_contracts`. Capability-specific surfaces wearing a core coat.
- `crates/merge/src/change.rs` takes `specs_dir` and `contracts_dir` as first-class parameters, carries a `ContractPreviewEntry` type, and hard-codes "3-way for specs, opaque-replace for contracts" as the entire merge universe.
- `crates/validate/src/lib.rs` re-exports `validate_baseline_contracts` — a contracts-format validator has become part of the core's public API.
- `src/config.rs` exposes `ProjectConfig::specs_dir` and `contracts_dir` as fixed helpers.
- `schemas/schema.schema.json` admits only `{ name, version, description, domain?, pipeline }`. Nothing about artifacts, validators, or capability-owned dependencies can be expressed.

Every new concern — infra, client SDKs, standards, codex rules, design tokens, fixtures — therefore requires a core patch.

### One primitive already works

`schema:` in `.specify/project.yaml` is already URL-resolvable, with project-local caching under `.specify/.cache/`. The rename changes the noun, not the distribution model: capabilities are still remote, versioned artefacts. The migration maps `schema:` to `capability:` and `schema.yaml` to `capability.yaml` (§Migration). Follow-up RFCs that currently say "schema" for the extension primitive must be updated as part of this landing so the post-RFC vocabulary has one meaning: **capability** for Specify extensions, **schema** for validation schemas.

### Artefact behavior is encoded in Rust

Today the runtime knows too much about a small set of artefacts:

- Specs are staged and merged file-by-file.
- Contracts are staged and promoted by whole-file replacement.
- Crates and Vectis shells are written directly into the project tree.
- Read-only baselines exist as an intended roadmap concern, but there is no manifest surface for declaring them.

Those are valid mechanics, but they are not core truths. The capability should declare which artefacts it owns, where they live, and how the fixed change loop treats them.

### What the status quo blocks

- A future `infra@v1` cannot declare "the `terraform/` directory is a staged baseline" without patching `specify-cli`.
- A future `standards@v1` (roadmap §3) needs `read-only` baselines that sibling changes cite but never mutate; today the adoption mechanics are hard-coded for specs, contracts, and direct code generation.
- The format validators behind `specify contract validate` live in the core's public API, so a third-party capability cannot ship an equivalent without patching core.

## Design

### Principle

**A capability describes how Specify defines, builds, and merges a class of artefacts.** The phase loop is fixed by the core; capabilities populate it with per-class choices (artefacts and validators). The core never switches on a capability name and never carries capability-specific type surfaces. Imperative extension code is owned by the capability's skills, which already have the tool and script mechanisms needed to execute it.

"Without exception" is load-bearing. If a capability-specific artifact behavior has no place in `capability.yaml`, that is a gap in the protocol, not a licence for a new core type surface.

The `define → build → merge` loop's *shape* is frozen: the phase set, legal transition DAG, and per-phase outcome contract recorded in `.metadata.yaml` are part of the immutable core. Capabilities declare what *flows through* the phases (artefacts, briefs, validators, and read-only dependencies) but never the phases themselves. Variation that capabilities legitimately want lives in variable briefs per phase and capability-specific skill behavior around declared artefacts. See §Non-Goals.

The coordinating principle is the dual: **capabilities own artefacts and their adoption behavior; workflow is one such capability, not a second framework layer.** Every mutable artefact has exactly one capability owner, every reviewed change runs through exactly one capability/scope, and cross-capability outcomes are represented by capability-owned artefacts rather than by fusing capabilities into a larger hidden capability. Outcomes are not necessarily code: they may be contracts, documentation, policy, infrastructure, fixtures, reports, generated clients, workflow graphs, or any other capability-owned artefact.

### The immutable core boundary

The core is what's needed to run the fixed change loop over any capability's artefacts — no more:


| Surface                                                                    | Owner             | What it does                                                                                                                                 |
| -------------------------------------------------------------------------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify init` (+ `--hub`)                                                 | Core              | Bootstrap `.specify/`, resolve capability URL(s), cache briefs. Runs before any capability has loaded.                                       |
| `specify migrate <migration>`                                              | Core              | One-shot layout migrations.                                                                                                                  |
| `specify capability *`                                                     | Core              | Resolve, check, pipeline. Replaces today's `specify schema *`.                                                                               |
| `specify change *`                                                         | Core              | Fixed change loop: create, list, status, validate, merge, drop, transition, archive, journal, outcome, touched-specs, overlap, task.         |
| Artefact merge bookkeeping                                                 | Core, data-driven | Iterates over capability-declared artefacts.                                                                                                 |
| Clone resolver                                                             | Core, data-driven | Resolves registry-declared project clones for first-party capabilities that need workspace materialisation; it owns no operator verbs.        |
| Format validators (OpenAPI, JSON Schema, spec-markdown, …)                 | Capability        | Declared as format adapters; core vendors generic ones, capabilities may ship their own.                                                     |


The left-hand column is frozen as the core responsibility boundary; new capability behavior lands on the right.

### What becomes a capability

Today's top-level verbs that aren't in the core table above are first-party capabilities bundled with the CLI (§First-party bootstrap):


| Today                  | Becomes                                 | Artefact                                    | Notes                                                                                                                                                                                |
| ---------------------- | --------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `specify plan *`       | `workflow@v1` capability                | `workflow.yaml`                             | Plan authoring, inspection, status, locking, and transitions become workflow-owned artefact behavior.                                                                               |
| `/spec:execute`        | `/workflow:execute` skill               | `workflow.yaml`                             | The existing execute driver moves to the workflow skill. It may keep `/spec:execute` only as a compatibility alias that delegates to `/workflow:execute`.                            |
| `specify initiative *` | `initiative@v1` capability              | `initiative.md`                             | Tiny: one brief; close-out is performed by initiative skills that verify every referenced workflow is terminal and every PR merged.                                                   |
| `specify registry *`   | `registry@v1` capability                | `registry.yaml`                             | Reviewed mutations go through `specify change`; the `description-missing-multi-repo` invariant becomes a `baseline-validate` finding.                                               |
| `specify contract *`   | `contracts@v1` capability               | `contracts/` baseline                       | RFC-12's SemVer + `info.x-specify-id` checks become `baseline-validate`.                                                                                                             |
| `specify vectis *`     | `vectis@v2` capability                  | Shared / iOS / Android / design-system dirs | Vectis-specific validation and merge behavior moves into Vectis skills and declared artefact mechanics.                                                                              |
| `specify workspace *`  | Core clone resolver plus registry-owned workflow data | Clones directory                            | The core clone resolver supplies path/materialisation helpers without making clone management a new capability extension surface.                                                    |


Every project activates at minimum `workflow@v1` + `initiative@v1` + `registry@v1` alongside its domain capability. All three are first-party capabilities with structured documents (`workflow.yaml`, `initiative.md`, `registry.yaml`) as their primary artefacts, not `spec.md`.

#### First-party bootstrap

`workflow@v1`, `initiative@v1`, and `registry@v1` must be available before any capability URL has been resolved: `specify init` must validate `registry.yaml`, and capability resolution itself runs through `specify capability *`, which is core. Resolution: first-party capabilities are embedded in the CLI binary and exposed through the same `capability.yaml` surface. The resolver checks the embedded set first, then falls back to URL resolution.

Embedded first-party capabilities track the CLI release as an ABI surface and validate exactly like URL-resolved capabilities. Projects pin via `specify_version` in `project.yaml`; embedded capability versions are not pinned independently. A project that opts out of a first-party capability sets `disable-first-party: [workflow]` — intentionally ugly, rarely used. Hub projects (`hub: true`) activate the three first-party capabilities without a domain capability; single-repo projects activate all four.

### Capability manifest and protocol

The capability manifest is the declarative surface the core loads before running the change loop. It combines the existing phase-brief pipeline with the new extension surface:

```yaml
name: omnia
version: 1
description: Omnia Rust WASM workflow
domain: ...
pipeline: ...
artifacts: ...
consumes: ...
```

Only `name`, `version`, `description`, and `pipeline` are always present. `domain` keeps its current meaning. The manifest fields are:


| Field            | Meaning                                                                                |
| ---------------- | -------------------------------------------------------------------------------------- |
| `pipeline:`      | Ordered phase briefs used by the fixed `define → build → merge` loop.                 |
| `artifacts:`     | Capability-owned output and context locations, with adoption mode and format metadata. |
| `consumes:`      | Optional read-only dependencies on other active capabilities' adopted baselines.       |


The protocol pieces below describe how the core interprets that manifest. Imperative behavior stays in capability skills and references, not in `capability.yaml`.

#### Artifacts (declarative adoption)

Every output location a capability owns is declared once, with an explicit adoption mode. The canonical example covers the common patterns:

```yaml
# omnia@v1 — Rust + WASM services. Specs drive code generation.
artifacts:
  - id: specs
    mode: staged                        # staged | direct | read-only
    delta: specs/
    baseline: .specify/specs/
    merge-strategy: three-way           # three-way | opaque-replace | none
    format: markdown-spec               # core or capability-declared format adapter
  - id: crates
    mode: direct
    project-path: crates/
    instance-path-template: <crate-name>/
  - id: codex
    mode: read-only
    baseline: codex/
```

`specs` carries no privilege — the linter sorts by `id` and the renderer iterates declared order, but no core code path keys off "the first artefact" or off the literal `id: specs`. Format adapters are named after the format (`markdown-spec`, `terraform-module`, `workflow-yaml`, `openapi-asyncapi-bundle`), not after artefact roles. A capability like `infra@v1` declares `format: terraform-module` and never lists a `specs` entry; `workflow@v1`'s sole artefact is `id: workflow, format: workflow-yaml`.

Modes:

- `staged` — build writes to `$CHANGE_DIR/<delta>/`; merge promotes to the declared baseline via `merge-strategy`; drop discards the delta; sibling changes read the baseline as conformance context.
- `direct` — build writes directly into a declared project path; git provides review and rollback; there is no separate promote/drop step for the artefact.
- `read-only` — declared baseline context that no change mutates; cited by generators and reviewers (roadmap §3 codex).

An `audited` mode for checksum-recorded direct writes is deferred. The phase 2 manifest parser should reserve the word and fail with a future-use diagnostic rather than treating it as a supported mode.

`merge-strategy` and `format` are explicit fields rather than implied by id. The core ships generic implementations for `three-way` (today's spec merge) and `opaque-replace` (today's contract merge) so pure-declarative capabilities work without extension code.

##### Location fields

Every artefact entry pairs its mode with a fixed set of location fields:


| Mode        | Required location fields | Meaning                                           |
| ----------- | ------------------------ | ------------------------------------------------- |
| `staged`    | `delta:` + `baseline:`   | build writes to delta, merge promotes to baseline |
| `direct`    | `project-path:`          | build writes directly into the project tree       |
| `read-only` | `baseline:`              | sibling changes cite, no change mutates           |


No artefact mixes location fields across modes. Cardinality is fixed at one `delta:`, one `baseline:`, and one `project-path:` per artefact (§Non-Goals).

##### Multi-instance artefacts

Direct artefacts whose `project-path` holds many sibling instances (omnia's `crates/<crate-name>/`, vectis's `<shell>/<target>/`) declare `instance-path-template:` to name the per-instance subdirectory. Staged artefacts may declare it too (a `delta:` of `specs/` with template `<crate-name>/spec.md` is exactly today's spec layout). Single-instance artefacts (`workflow.yaml`, `initiative.md`) omit the field. The template names a single brief-bound variable; the producing brief resolves it from its context. The linter enforces that exactly one variable appears.

##### Substitution vocabulary

The closed vocabulary in brief prose covers every declared location:


| Substitution                    | Resolves to                                          | Available for              |
| ------------------------------- | ---------------------------------------------------- | -------------------------- |
| `$ARTIFACT_DELTA[<id>]`         | the artefact's declared `delta:` path                | `staged`                   |
| `$ARTIFACT_BASELINE[<id>]`      | the artefact's declared `baseline:` path             | `staged`, `read-only`      |
| `$ARTIFACT_PROJECT_PATH[<id>]`  | the artefact's declared `project-path:`              | `direct`                   |
| `$ARTIFACT_INSTANCE_PATH[<id>]` | the location resolved with `instance-path-template:` | any mode that declares one |


Direct literal paths are forbidden and flagged by `specify check`.

##### Brief-to-artefact binding

Briefs declare which artefact id(s) they produce via a `produces:` frontmatter field, the symmetric counterpart to `needs:` and `tracks:`. The field binds the brief to one or more ids in the active capability's `artifacts:` block, which is what lets substitutions resolve at render time and what lets `specify check` verify that every staged artefact has a producing brief in `pipeline.build`:

```yaml
---
id: build
description: Implement the tasks in tasks.md by delegating to the skills below
needs: [specs, design, tasks]
produces: [crates]
tracks: tasks
---
```

A brief that produces a multi-instance artefact resolves the instance variable from its own context. The linter enforces that every artefact whose mode requires authoring (`staged`, `direct`) appears in some brief's `produces:` list. `read-only` artefacts are exempt.

#### Artifact behavior and skills

The core handles deterministic artifact mechanics from the manifest:

| Event | Core default | Capability responsibility |
| ----- | ------------ | ------------------------- |
| Change delta validation | Apply generic format checks where a declared `format:` has a core adapter. | Capability skills add domain-specific checks during define, build, review, or merge briefs. |
| Merge preview | Render the declared `merge-strategy` preview for staged artifacts. | Capability skills interpret the preview and raise behavioral risks in the phase output. |
| Merge run | Promote staged deltas via `merge-strategy`; accept direct artifact writes through git review. | Capability skills perform any prerequisite generation, verification, or close-out before merge is marked complete. |
| Drop | Remove the change delta. | Capability skills document any direct-write cleanup required by their artifacts. |

Defaults for `three-way` and `opaque-replace` mean a pure-declarative YAML + markdown capability gets a working `define → build → merge` loop for free. Anything beyond those deterministic mechanics belongs in the capability's skills and references, where imperative code can already be included, invoked, and reviewed without adding a new core plugin runtime.

#### Consumes (read-only dependencies)

`consumes:` declares read-only dependencies on other active capabilities' adopted baselines. It answers "what may this capability read as context?" and is deliberately separate from `artifacts:`, which answers "what does this capability own?"

Rules:

- A consumed capability MUST be active in the same project.
- Consuming a capability grants no write access and creates no shared ownership of the consumed artefacts.
- A consumed baseline is context for generation, validation, review, or workflow coordination; it is not part of the consuming capability's merge transaction.
- `specify check` validates that consumed capability names and referenced artefact ids resolve in the active capability set.
- RFC-14 adds the `@<scope>` qualifier for workspaces: optional when there is a single provider, mandatory when multiple scopes could provide the consumed capability.

Example: `client-sdk@v1` may consume `contracts@v1` so it can generate clients from adopted OpenAPI / AsyncAPI baselines, but only `contracts@v1` may mutate those baselines.

#### Cross-capability coexistence

Every project activates multiple capabilities — at minimum a domain capability plus `workflow@v1` + `registry@v1` + `initiative@v1`. Two constraints apply across the active set:

- **Artefact id uniqueness.** No two active capabilities may declare the same `artifact.id`.
- **Baseline-path uniqueness.** No two active capabilities may claim the same baseline path or project-path.

Read-only coupling between capabilities is declared through `consumes:` (§Consumes).

A repository activates exactly one **domain** capability under this RFC. Multi-domain repositories are covered by [RFC-14](rfc-14-workspaces.md), which adds a Cargo-style `package:` / `workspace:` shape and makes the uniqueness rules scope-aware.

#### Cross-capability coordination

When an outcome spans capabilities, the runtime does not fuse their pipelines. Coordination is explicit and artefact-owned: `initiative@v1` records the outcome and close-out criteria, `registry@v1` identifies participating projects, the core clone resolver resolves project/scope addresses, and `workflow@v1` owns any DAG of capability-addressed steps.

The workflow graph, if present, coordinates through the same manifest and artifact protocol as every other capability. Nodes may target capability-owned changes, validations, or checks; edges express ordering (`needs:`) and blocking conditions. Any read-only baseline access used by those nodes is still declared through `consumes:` (§Consumes). A workflow may deliver code, but it may also deliver contracts, docs, infrastructure, fixtures, reports, or policy changes.

This RFC does not define a core workflow runner. Workflow authoring, validation, execution, and re-entry are `workflow@v1` capability concerns implemented by workflow skills. The existing `/spec:execute` skill is therefore not a new core lifecycle command; it migrates to `/workflow:execute`, where the skill remains the long-running orchestrator that calls core phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`) and workflow-owned deterministic helpers.

##### Example: landing an initiative

The end-to-end human loop has three operator checkpoints:

1. **Initiative.** The `initiative@v1` capability defines, builds, and merges `initiative.md`. The artefact is prose containing the desired outcome, scope, impacted projects, feature list, and close-out criteria.
2. **Workflow change.** The `workflow@v1` capability defines, builds, and merges `workflow.yaml`. The operator reviews the graph, dependencies, target projects/scopes, and change boundaries before merge. `/workflow:execute` may then run the accepted graph using workflow-owned helpers for next-entry selection, locking, status updates, and recovery.
3. **Initiative close-out.** The `initiative@v1` capability's finalize action verifies that referenced workflows are terminal, required PRs have merged, and the close-out criteria in `initiative.md` are satisfied.

### Worked `capability.yaml` example

This example shows the full declarative shape for a capability. It is illustrative rather than a frozen `vectis@v2` manifest.

```yaml
name: vectis
version: 2
description: Vectis Crux application workflow
domain: |
  Rust shared core, SwiftUI iOS shell, Kotlin Android shell, VectisDesign tokens.

pipeline:
  define:
    - id: proposal
      brief: briefs/proposal.md
    - id: specs
      brief: briefs/specs.md
    - id: composition
      brief: briefs/composition.md
    - id: design
      brief: briefs/design.md
    - id: tasks
      brief: briefs/tasks.md
  build:
    - id: build
      brief: briefs/build.md
  merge:
    - id: merge
      brief: briefs/merge.md

artifacts:
  - id: specs
    mode: staged
    delta: specs/
    baseline: .specify/specs/
    merge-strategy: three-way
    format: markdown-spec
  - id: shared-core
    mode: direct
    project-path: shared/
  - id: ios-shell
    mode: direct
    project-path: ios/
  - id: android-shell
    mode: direct
    project-path: android/
  - id: design-system
    mode: direct
    project-path: design-system/

consumes:
  - contracts
```

### What this enables

With `capability.yaml` owning phase briefs, artefact declarations, and read-only dependencies, new concerns ship as capabilities. None of these requires a core patch:

| Capability         | Artefact declaration                                                                   | Capability behavior                                           |
| ------------------ | -------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| `infra@v1`         | `terraform`, `mode: staged`, `merge-strategy: opaque-replace`                          | Infra skills can shell out to `terraform validate`.           |
| `client-sdk@v1`    | Own artefact `clients`, `mode: direct`, `project-path: clients/`; consumes `contracts` | Build can generate clients from consumed contract baselines.  |
| `standards@v1`     | `codex`, `mode: read-only`, `baseline: codex/`                                         | Generators and reviewers can cite adopted standards.          |
| `design-tokens@v1` | Staged token source + direct generated outputs (Swift / Kotlin / CSS)                  | Design-token skills can regenerate outputs and report drift.  |

None of these needs a `specs` artefact. Capabilities that want behavioural specs declare one and stage a producing brief; capabilities that do not simply omit it.

### Distribution: declarative manifest plus skills

Pure-declarative capabilities (YAML + markdown + a format adapter the core vendors — `markdown-spec`, `openapi`, `asyncapi`, `json-schema`) work end-to-end without extension code. Capabilities that need host tools, generators, or reviewers carry that imperative behavior in their skills and references. This keeps the core protocol small: the manifest declares artifact ownership and brief flow; skills decide how to produce, validate, review, or clean up those artifacts.

Skill-owned imperative code runs through the same mechanisms agents already use today: checked-in helper scripts, generated code, shell commands, package-manager tools, and language-specific toolchains invoked by the skill. The security posture is therefore the existing skill/tooling posture, not a second plugin trust model hidden behind `capability.yaml`.

#### Workspace-clone path resolution

When workflow execution materialises registry-declared projects, every `artifacts.*.{baseline, project-path, delta}` resolves relative to **the clone's project root**, not the hub's. The core clone resolver supplies the normalized project-root mapping used by registry-aware workflow execution; it does not expose a `specify workspace *` command family.

## Alternatives Considered

- **Subprocess capability plugins.** Rejected because capability skills already own imperative behavior and already have mechanisms for invoking scripts, tools, and generated code. A second plugin runtime would duplicate the skill layer and introduce a separate trust model.
- **WASM-component plugins.** Rejected for the same reason as subprocess plugins; sandboxing imperative capability code belongs in the agent/tool execution model, not in `capability.yaml`.
- **In-process dynamic-library plugins.** Rejected because Rust ABI instability disqualifies them and because the capability protocol does not need a second imperative extension path.
- **Keep `specify workspace *` as a core exception.** Rejected because it weakens the "no capability-specific operator verbs in core" rule. The retained core need is only clone resolution.
- **Extract a standalone `workspace@v1` capability.** Rejected for the first landing because workspace materialisation is registry-driven context for workflow execution. A separate capability would mostly wrap registry state while making clone path resolution less canonical.
- **Multiple imperative escape hatches.** Rejected because capability skills are the single imperative escape hatch.
- **Keep `artifacts:` adoption-only.** Rejected because artifacts need format validators and read-only dependencies to describe real capability behavior.
- **A top-level `artifacts.yaml` next to `capability.yaml`.** Rejected because the extension surfaces are capability-bound, not project-bound.

## Non-Goals

- **Replacing or capability-configuring the `define → build → merge` loop.** The loop's *shape* (phase set, transition DAG, per-phase outcome contract) is part of the immutable core. Capabilities declare what flows through the phases (artefacts, briefs, validators, and read-only dependencies) but never the phases themselves. Variability lives in variable briefs per phase and skill-owned artifact behavior. A capability that genuinely cannot fit any of those would justify proposing a *second* fixed loop shape as a peer to this one — never open-ended phase configuration.
- **Format-level contract evolution.** SemVer + `info.x-specify-id` + cross-repo uniqueness continue to be owned by RFC-12; this RFC only moves where the rules run from.
- **A new plugin runtime.** Imperative behavior remains in skills; this RFC only defines the declarative capability manifest.
- **A general sandboxed write-fence.** Deferred until `specify check`'s write-path inventory is trustworthy enough to enforce.
- **Cardinality > 1 on location fields.** One artefact may declare at most one `delta:`, one `baseline:`, and one `project-path:`. Revisited only if a real capability needs more.
- **Cloud execution semantics.** Orthogonal; capability skills should run through whatever tool execution model the host provides.
- **Back-compat for capabilities without the new surface.** See §Migration — current usage footprint lets us cut over without a fallback path.
- **Third-party platform capabilities.** `workflow@v1`, `registry@v1`, `initiative@v1` are first-party and bundled. Swapping them is a follow-up RFC.
- **Multiple domain capabilities per repository.** Covered by [RFC-14](rfc-14-workspaces.md), strictly additive on top of this RFC's capability manifest protocol.
- **Cross-capability changes in a single transaction.** Multi-capability outcomes are coordinated by capability-owned artefacts, not by one change that writes multiple capabilities' baselines. RFC-14 applies the same rule to scopes: cross-scope work is a workflow with multiple entries, not a multi-scope change.

Multi-capability *per project* is in scope — `workflow` + `registry` + `initiative` always coexist with a domain capability (§Cross-capability coexistence). Multi-*domain*-capability per project is the RFC-14 layer.

## Glossary

| Term | Meaning |
| ---- | ------- |
| Active capability set | The domain capability plus first-party capabilities active for a project or scope. |
| Capability | A versioned Specify extension manifest that declares phase briefs, artefacts, and read-only dependencies. |
| Domain capability | The primary project capability such as `omnia@v1`, `contracts@v1`, or `vectis@v2`. RFC-14 adds multiple domain capabilities through scopes. |
| First-party capability | A capability bundled with the CLI release and resolved through the same manifest path as URL capabilities. |
| Platform capability | A first-party capability that supports coordination or project metadata, such as `registry@v1` or `initiative@v1`. |
| Workflow capability | `workflow@v1`, the first-party capability that owns `workflow.yaml` and `/workflow:execute`. It is not a core runner. |
| Format adapter | The handler for artefact syntax and validation, such as `markdown-spec`, `openapi`, `asyncapi`, or `json-schema`. |
| Clone resolver | The small core service that maps registry-declared projects to materialised clone roots without owning workspace operator verbs. |

## Implementation Scope

An incremental landing, each stage independently testable and shippable. Every stage preserves working `/spec:define → /spec:build → /spec:merge` for the `omnia` capability (the only capability currently in real use). The phases are delivery slices for this RFC, not separate RFCs.

Sizing guide:

| Phase | Expected size | Acceptance focus |
| ----- | ------------- | ---------------- |
| 1. Capability vocabulary cut-over | ~400-700 lines | Rename surfaces and diagnostics while preserving existing `pipeline:` behavior. |
| 2. Artifact declarations and adoption | ~900-1300 lines | Remove fixed `specs` / `contracts` path handling and drive merge from declared artefacts. |
| 3. Brief bindings, substitutions, and lints | ~700-1000 lines | Bind briefs to artefacts and enforce substitution/path invariants. |
| 4. First-party extraction and core cleanup | ~900-1400 lines | Extract first-party capability manifests and delete concern-specific core type surfaces. |

Estimated total: ~3200-4800 lines across `specify-cli`, schema updates, fixture refreshes, and plugin documentation.

### Phase 1 — Capability vocabulary cut-over

Lands the rename without changing artefact mechanics.

1. Rename the extension primitive in manifests and project config: `schema.yaml` → `capability.yaml`, `project.yaml:schema` → `project.yaml:capability`, and `specify schema {resolve,check,pipeline}` → `specify capability {resolve,check,pipeline}`.
2. Rename the schema/manifest crate and CLI help text where they refer to Specify extensions. JSON Schema remains JSON Schema.
3. Preserve the existing `pipeline:` behavior byte-for-byte so the only behavior change in this phase is the vocabulary cut-over.
4. Update docs, fixtures, and diagnostics to use **capability** for Specify extensions and **schema** only for validation schemas.

Acceptance: a canonical omnia change still completes through `/spec:define → /spec:build → /spec:merge`, and pre-cut-over manifests fail with a clear "schema has become capability" diagnostic.

### Phase 2 — Artifact declarations and adoption

Lands the artefact adoption surface, widened to the three supported modes.

1. New `artifacts:` fields parsed in the capability manifest crate — `id`, `mode`, the location-field set, `instance-path-template`, `merge-strategy`, `format`. JSON Schema additions enforce the mode ↔ location-field pairings from §"Location fields".
2. `crates/merge/` refactor: replace the hard-coded `specs_dir` + `contracts_dir` pair with iteration over the active capability's `staged` artifacts, dispatched on `merge-strategy`. Core ships `three-way` and `opaque-replace` defaults.
3. `crates/validate/`: add `--artifact <id>` filter.
4. `src/config.rs`: drop `specs_dir` / `contracts_dir`; add `ProjectConfig::{baseline_path, delta_path, project_path}(&capability, artifact_id)`. An instance-resolving variant takes the brief context and applies `instance-path-template`.
5. First-party capabilities adopt `artifacts:` blocks declaring today's paths exactly — no filesystem changes.

Acceptance: the core no longer carries fixed `specs` / `contracts` path helpers, and RFC-14 can layer scope-aware path resolution on the declared artefact model.

### Phase 3 — Brief bindings, substitutions, and lints

Lands the authoring contract that lets briefs refer to capability-owned locations without hard-coded paths.

1. Brief frontmatter parser learns `produces:` (single id or list). Brief loader binds each entry to an artefact in the active capability; unbound ids fail load with a diagnostic.
2. Brief renderer learns the closed substitution vocabulary (`$ARTIFACT_DELTA[...]`, `$ARTIFACT_BASELINE[...]`, `$ARTIFACT_PROJECT[...]`, `$CHANGE_DIR`) and resolves instance templates from brief context.
3. `specify check` (RFC-5) lints flag direct literal paths and the per-artefact invariants: verifier brief present for `staged`, no baseline / project-path collision, pipeline stays within declared artefacts, every authoring-required artefact appears in some brief's `produces:` list, and every `instance-path-template` names exactly one variable.
4. Port first-party brief prose to the substitution vocabulary.

Acceptance: a capability can add or rename an artefact location without changing core code or brief prose outside declared substitutions.

### Phase 4 — First-party extraction and core surface cleanup

The largest phase: it proves the reframe without changing the lifecycle model.

1. **Extract `workflow@v1`, `registry@v1`, `initiative@v1`, `contracts@v1`, and `vectis@v2` as first-party capabilities** embedded in the CLI via `include_str!` or a tidy `embedded-capabilities/` tree, exposed through the same resolver path as URL-resolved capabilities.
2. **Move the execute driver to the workflow skill.** `/spec:execute` becomes `/workflow:execute`; any retained `/spec:execute` spelling is only a compatibility alias that delegates to the workflow skill.
3. **Keep workflow helpers internal to workflow execution.** `/workflow:execute` may use skill-owned scripts or library helpers for next-entry selection, locking, status updates, and recovery. Generic change-loop reads such as `specify change outcome show` stay core.
4. **Delete concern-specific core type surfaces where the artifact model replaces them.** `Commands::{Vectis, Contract}` and the matching command modules stop being the place where artifact validation and merge behavior live.
5. **Keep only the core clone resolver needed by registry and workflow code paths.**
6. **Retire surviving hard-coded `contracts` / `specs` references** in `crates/merge/`, `crates/validate/`, `src/config.rs`, `crates/change/`.
7. **First-party capabilities publish their full surface** — `omnia`, `contracts`, `vectis`, `workflow`, `registry`, `initiative` declare `artifacts:` + `pipeline:`.
8. **Auto-activation at `specify init`.** A project's `project.yaml` declares its domain capability; the core auto-activates `workflow@v1`, `registry@v1`, and `initiative@v1`. Hubs activate the three without a domain capability.

Phase 4 may land as a sequence of smaller commits, but every commit keeps the existing `define → build → merge` lifecycle intact.

### This repo (`augentic/specify`)

1. Add `capabilities/capability.schema.json` (or rename the existing manifest schema) to cover `artifacts:` and `consumes:`.
2. Rewrite `capabilities/{contracts,omnia,vectis,workflow,registry,initiative}/capability.yaml` to declare their full extension surface.
3. Port brief prose to `$ARTIFACT_DELTA[<id>]` / `$ARTIFACT_BASELINE[<id>]` substitutions.
4. Move `plugins/spec/skills/execute/` to the workflow capability surface as `/workflow:execute`; keep any `/spec:execute` material as a compatibility shim only.
5. Update `plugins/contract/`, `plugins/vectis/`, and workflow-facing skills to own any imperative validation, generation, or review behavior that used to sit behind in-binary command modules.
6. Document the manifest protocol in `docs/reference/capabilities.md`; cross-link from each capability's README. Add glossary entries for "active capability set," "workflow graph," and "first-party capability."

## Migration

Only the `omnia` capability and the core loop are in real-world use. `specify contract *`, `specify vectis *`, and the bulk of `specify plan|initiative|registry *` have no durable external user base to protect.

**Hard cut-over, no fallback path.** Each phase's minor version is a breaking change for the surfaces it touches. No deprecation window and no `artifacts:`-absent fallback: pre-reframe capability manifests fail to load against the post-reframe CLI with a clear diagnostic pointing at this RFC and the capability rename. `/spec:execute` is not retained as a `spec` plugin responsibility; if the spelling survives, it delegates to `/workflow:execute`.

### Migration TL;DR

The rename is part of that cut-over:


| Current term / surface                    | Post-RFC term / surface                       |
| ----------------------------------------- | --------------------------------------------- |
| Schema (extension primitive)              | Capability                                    |
| `schema.yaml`                             | `capability.yaml`                             |
| `project.yaml:schema`                     | `project.yaml:capability`                     |
| `specify schema {resolve,check,pipeline}` | `specify capability {resolve,check,pipeline}` |
| `schemas/<name>/schema.yaml`              | `capabilities/<name>/capability.yaml`         |


JSON Schema remains JSON Schema. `*.schema.json` continues to name validation schemas, not Specify capabilities.

### Deferred phase rename

This RFC does **not** rename `define` / `merge` to `draft` / `adopt`. That rename would touch slash commands, CLI verbs, brief ids, journal language, metadata fields, downstream skill references, and existing fixtures. If the product wants that vocabulary later, it should land as a separate lifecycle RFC after the capability data reframe is stable.

Four invariants guard the landing:

1. **Omnia keeps working.** Every phase's acceptance criterion includes running `/spec:define → /spec:build → /spec:merge` on a canonical omnia change end-to-end.
2. **The core never learns a capability name.** `specify check` rejects hard-coded capability-name literals in core crate sources outside tests, including first-party names after extraction.
3. **The core never learns an artefact id either.** A companion rule rejects hard-coded artefact-id literals such as `"specs"`, `"contracts"`, `"crates"`, and `"workflow"`; phase 2 retires the current canonical violations (`ProjectConfig::{specs_dir, contracts_dir}`).
4. **First-party capabilities are still capabilities.** A rule verifies `workflow@v1`, `registry@v1`, `initiative@v1` each pass the same validation as any third-party capability — `capability.yaml` parses against the capability manifest JSON Schema, all declared briefs exist, artifact locations are valid, and so on.

The hard-coded-name lints are RFC-5 design work, not a naive string-literal ban. RFC-5 should define the crate allowlist, generated-code exemptions, test exemptions, and AST-aware matching needed to avoid flagging unrelated prose or diagnostics.

Linter rules in `specify-check` (RFC-5) enforce, additionally:

- **Active-capability-set invariants:** artefact-id, baseline-path, and project-path uniqueness across active capabilities.
- **First-party capability parity:** embedded capabilities pass every rule URL-resolved capabilities must pass.
- **Brief-binding completeness:** every artefact whose mode requires authoring (`staged`, `direct`) appears in some brief's `produces:` list. `read-only` artefacts are exempt.
- **Path-substitution discipline:** brief prose references locations only via the closed substitution vocabulary; direct literal paths fail the lint.

## Open Questions

Genuinely open:

1. **Mode naming.** `staged` / `direct` / `read-only` is the provisional vocabulary; confirm or replace one more time before phase 2. `audited` remains future work, not part of the phase 2 mode set.
2. **Instance-variable resolution for multi-instance briefs.** Should the capability declare the binding source explicitly (e.g. `instance-source: artifact:specs.subdirs`), or remain a brief-side concern wired through skill code? Provisional: brief-side for now; revisit when a capability appears whose binding can't be expressed as a one-liner.

Resolved with provisional answer (see body for context):

- **Multiple capabilities per project / `capability:` shape.** Resolved by [RFC-14](rfc-14-workspaces.md): `package:` / `workspace:` shape, scope-aware uniqueness rules, back-compat shim for Mode-A repos, `disable-first-party:` survives.
- **Artifact mode taxonomy in phase 2.** Ship `staged`, `direct`, `read-only`; reserve `audited` as a parse-time future-use error.
- **Default `artifact-validate`.** No core default — validation is where format semantics matter most and a silent default would mask missing capability work.
- **Format-adapter registry.** Fixed in-core registry to start; revisit when a third-party capability wants to ship its own.
- **First-party capability versioning.** Embedded capabilities track the CLI release as an ABI surface; projects pin via `specify_version` only.
- **Workspace ownership.** Resolved by keeping only a clone resolver in core; workflow and registry artifacts provide the data that drives materialisation.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) — owns the crates the reframe touches (`specify-schema`, `specify-merge`, `specify-validate`, `specify-change`) and the `src/cli.rs` dispatcher.
- [RFC-8: API contracts](archive/rfc-8-api-contracts.md) — `contracts@v1` capability; delta-then-promote semantics become the `opaque-replace` default.
- [RFC-2: Execution](archive/rfc-2-execution.md) — `/spec:execute --loop`; informs the workflow capability migration, but this RFC does not change the lifecycle model.
- [RFC-3a: Monoliths](archive/rfc-3a-monoliths.md) — plan authoring pipeline; the existing two-brief `pipeline.plan` is the predecessor to workflow artefact authoring.
- [RFC-3b: Platform](archive/rfc-3b-platform.md) — registry routing and workspace clones.
- [RFC-9: Platform](archive/rfc-9-platform.md) — moved registry, plan, initiative, and contracts to repo root; `/spec:plan --orchestrate` is the predecessor to workflow-driven orchestration, but not to a retained workflow CLI family.
- [RFC-12: Refine RFC-8](archive/rfc-12-refine-rfc-8.md) — SemVer + `info.x-specify-id` rules become contracts capability validation behavior.
- [RFC-5: Framework Linter](rfc-5-lint.md) — home of the lints enforcing the reframe's invariants, including the hard-coded-name lint design.
- [Roadmap](roadmap.md) — §3 motivates `read-only`; §5 / §6 / §7 are consumers of a stable core surface.
- `plugins/contract/references/baseline-vs-delta.md`, `docs/how-to/migrate-to-v2-layout.md` — references in this `augentic/specify` repository that define the path constants and v2 layout boundary the artifact declarations make per-artefact configurable.

