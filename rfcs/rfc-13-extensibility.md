# RFC-13: Extensibility

> Status: Draft · Supersedes: earlier draft at this path (artifact-adoption-only framing) · Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-8](archive/rfc-8-api-contracts.md), [RFC-9](archive/rfc-9-platform.md), [RFC-12](archive/rfc-12-refine-rfc-8.md) · Enables: [RFC-14](rfc-14-workspaces.md)

## Abstract

A capability describes how Specify's existing `define → build → merge` loop handles an outcome domain and the artefacts that domain owns. RFC-13 reframes the runtime to match: the **immutable core** is the per-project loop engine plus capability-agnostic scaffolding (init, migrate, capability resolver, slice driver, and artefact adoption). Platform features such as registry materialisation and change orchestration are separated into first-party Specify components rather than being folded into either the core or the capability model.

This RFC also renames two lifecycle nouns. The umbrella concept that coordinates a multi-slice outcome — formerly **initiative** — becomes **change**. The single unit that flows through the fixed `define → build → merge` loop — formerly called a change — becomes a **slice**. A change holds one or more slices through its `plan.yaml`; each slice is a per-project transaction with its own proposal, specs, design, tasks, and merge step. The §Migration table records the cut-over.

Today's `schema.yaml` surface admits only `{ name, version, description, pipeline }`, which is too small to carry that contract and uses the wrong noun. This RFC renames the extension primitive to **capability** and makes `pipeline:` an explicit member of the capability manifest alongside new fields (`artifacts:` and optional `consumes:`) so a capability can describe its phase briefs, artefacts, and read-only dependencies. It also draws a line around non-capability foundation components: topology and local materialisation belong to `specify-registry`, and change orchestration belongs above the core loop.

## Motivation

### The core isn't actually core

Specify's current surface promises extensibility and breaks it inside the binary:

- `specify-cli/src/cli.rs` carries `Vectis { action: VectisAction }` and `Contract { action: ContractAction }` as top-level subcommands, dispatched through `specify_vectis` and `specify::validate_baseline_contracts`. Capability-specific surfaces wearing a core coat.
- `crates/merge/src/change.rs` takes `specs_dir` and `contracts_dir` as first-class parameters, carries a `ContractPreviewEntry` type, and hard-codes "3-way for specs, opaque-replace for contracts" as the entire merge universe.
- `crates/validate/src/lib.rs` re-exports `validate_baseline_contracts` — a contracts-format validator has become part of the core's public API.
- `src/config.rs` exposes `ProjectConfig::specs_dir` and `contracts_dir` as fixed helpers.
- `schemas/schema.schema.json` admits only `{ name, version, description, pipeline }`. Nothing about artifacts, validators, or capability-owned dependencies can be expressed.

Every new concern — infra, client SDKs, standards, codex rules, design tokens, fixtures — therefore requires a core patch.

### One primitive already works

`schema:` in `.specify/project.yaml` is already URL-resolvable, with project-local caching under `.specify/.cache/`. The rename changes the noun, not the distribution model: capability manifests are still remote, versioned artefacts. The migration maps `schema:` to `capability:` and `schema.yaml` to `capability.yaml` (§Migration). Follow-up RFCs that currently say "schema" for the extension primitive must be updated as part of this landing so the post-RFC vocabulary has one meaning: **capability** for Specify extensions, **schema** for validation schemas.

### Artefact behavior is encoded in Rust

Today the runtime knows too much about a small set of artefacts:

- Specs are staged and merged file-by-file.
- Contracts are staged and promoted by whole-file replacement.
- Crates and Vectis shells are written directly into the project tree.
- Read-only baselines exist as an intended roadmap concern, but there is no manifest surface for declaring them.

Those are valid mechanics, but they are not core truths. The capability should declare which artefacts it owns, where they live, and how the fixed slice loop treats them.

### What the status quo blocks

- A future `infra@v1` cannot declare "the `terraform/` directory is a staged baseline" without patching `specify-cli`.
- A future `standards@v1` (roadmap §3) needs `read-only` baselines that sibling slices cite but never mutate; today the adoption mechanics are hard-coded for specs, contracts, and direct code generation.
- The format validators behind `specify contract validate` live in the core's public API, so a third-party capability cannot ship an equivalent without patching core.

## Design

### Principle

**A capability describes how Specify creates an outcome domain's artefacts.** The `draft-build-adopt` phase loop is fixed by the core; capabilities populate it with per-domain choices (artefacts and validators). The core never switches on a capability name and never carries capability-specific type surfaces. Imperative code is owned by a capability's skills, which have the tool and script mechanisms needed to execute it.

If a capability-specific artifact behavior has no place in `capability.yaml`, that is a gap in the protocol, not a licence for a new core type surface.

The `define → build → merge` loop's *shape* is frozen: the phase set, legal transition DAG, and per-phase outcome contract recorded in `.metadata.yaml` are part of the immutable core. Capabilities declare what *flows through* the phases (artefacts, briefs, validators, and read-only dependencies) but never the phases themselves. Variation that capabilities legitimately want lives in variable briefs per phase and capability-specific skill behavior around declared artefacts. See §Non-Goals.

The coordinating principle is: **capabilities own outcome artefacts and their adoption; platform components coordinate where and when those per-project slices run**.

Every mutable artefact has exactly one capability owner, every reviewed slice runs through exactly one capability/scope, and cross-capability outcomes are represented by explicit change plan entries rather than by fusing capabilities into a larger hidden capability. Outcomes are not necessarily code: they may be contracts, documentation, policy, infrastructure, fixtures, reports, generated clients, or any other capability-owned artefact.

### The immutable core boundary

The core is what's needed to run the fixed slice loop over one project root and one resolved capability — no more:

| Surface                                                                    | Owner             | What it does                                                                                                                                 |
| -------------------------------------------------------------------------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify init`                                                             | Core              | Bootstrap `.specify/`, resolve capability URL(s), cache briefs. Runs before any capability has loaded.                                       |
| `specify migrate <migration>`                                              | Core              | One-shot layout migrations.                                                                                                                  |
| `specify capability *`                                                     | Core              | Resolve, check, pipeline. Replaces today's `specify schema *`.                                                                               |
| `specify slice *`                                                          | Core              | Fixed slice loop: create, list, status, validate, merge, drop, transition, archive, journal, outcome, touched-specs, overlap, task.          |
| Artefact merge bookkeeping                                                 | Core, data-driven | Iterates over capability-declared artefacts.                                                                                                 |
| Format validators (OpenAPI, JSON Schema, spec-markdown, …)                 | Capability        | Declared as format adapters; core vendors generic ones, capabilities may ship their own.                                                     |


The left-hand column is frozen as the core responsibility boundary; new capability behavior lands on the right. Platform components sit above this table: they may choose a project root, prepare a materialised registry checkout, or sequence several slices, but they call the core loop rather than becoming part of it.

### Platform components are not capabilities

Registry and the change component are first-party Specify components because they are substrate for multi-project operation, not outcome domains. They may have commands, libraries, and files, but they do not participate in the capability manifest protocol and they are not activated through `capability.yaml`.

| Component | Primary file / state | Responsibility | Must not own |
| --------- | -------------------- | -------------- | ------------ |
| `specify-registry` | `registry.yaml` + `.specify/workspace/` | Topology ledger plus local materialised view: project ids, repository locations, human descriptions, default capabilities, clone/symlink resolution, dirty-state reporting, and explicit push/merge operations for checked-out registry projects. | Change or plan status, contract relationships, validation findings, change execution, capability-specific validation, or PR metadata beyond the local project operation being requested. |
| `specify-change` | `change.md` + `plan.yaml` | Coordinate an operator outcome from brief through executable plan, execution state, and close-out by consuming registry project ids, materialised project paths, and core phase outcomes. | Domain artefact ownership, topology materialisation, or hidden multi-capability transactions. |

The dependency direction is one-way: `specify-core` knows nothing about registry or change orchestration. `specify-change` may depend on `specify-registry` and the core loop because orchestration composes those lower-level services.

### What becomes a capability

Not every top-level noun becomes a capability. Capability is reserved for outcome domains whose artefacts flow through the fixed loop. Foundation and orchestration surfaces become separate Specify components.

The durable post-RFC surfaces are:

| Surface                 | Owner / kind                   | Primary state / artefact                    | Notes                                                                                                                |
| ----------------------- | ------------------------------ | ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `specify init`          | Core                           | `.specify/` + `project.yaml`                | Bootstraps a project or hub before any capability has loaded.                                                        |
| `specify capability *`  | Core                           | capability manifest cache                   | Resolves, checks, and renders capability pipelines. Replaces today's `specify schema *`.                             |
| `specify slice *`       | Core                           | `.specify/slices/`                          | Runs the fixed per-project slice loop against one resolved capability.                                               |
| `specify registry *`    | `specify-registry` component   | `registry.yaml` + `.specify/workspace/`     | Owns topology plus the local materialised view. It is validated and mutated directly, not reviewed through the loop. |
| `specify change *`      | `specify-change` component     | `change.md` + `plan.yaml`                   | Owns change brief, planning graph, execution state, finalization, and archive.                                       |
| `contracts@v1`          | Capability                     | `contracts/` baseline                       | RFC-12's SemVer + `info.x-specify-id` checks become capability validation behavior.                                  |
| `vectis@v2`             | Capability                     | Shared / iOS / Android / design-system dirs | Vectis-specific validation and merge behavior moves into Vectis skills and declared artefact mechanics.              |

The removed or compatibility surfaces are:

| Current / old surface  | Result                      | Replacement                                      | Notes                                                                                                                                  |
| ---------------------- | --------------------------- | ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| `specify schema *`     | Renamed                     | `specify capability *`                           | Vocabulary cut-over from schema to capability.                                                                                         |
| `specify change *` (today's per-loop unit) | Renamed                | `specify slice *`                                | The single unit that flows through `define → build → merge` is now a slice; `change` is reused for the orchestration surface below.    |
| `specify plan *`       | Folded into change          | `specify change plan *`                          | Plan authoring, inspection, status, locking, and transitions are change subresource behavior, not a separate top-level domain.         |
| `specify initiative *` | Renamed                     | `specify change *`                               | The umbrella orchestration noun moves from `initiative` to `change`. The verb set (`create`, `plan`, `execute`, `finalize`, `archive`) is preserved. |
| `/spec:plan`           | Compatibility alias if kept | change planning skill / command                  | The assisted planning skill authors or refreshes the change's executable plan.                                                         |
| `/spec:execute`        | Compatibility alias if kept | change execution skill / command                 | The existing execute driver moves to the change surface and delegates to change execution if the old spelling survives.                |
| `specify workspace *`  | Removed                     | `specify registry *`                             | No compatibility alias is retained. Materialisation commands move to the registry surface, and `.specify/workspace/` is registry state. |
| `specify contract *`   | Folded into capability      | `contracts@v1` capability validation and skills  | Contract format validation and adoption behavior move out of core command modules.                                                      |
| `specify vectis *`     | Folded into capability      | `vectis@v2` capability validation and skills     | Vectis-specific behavior moves out of core command modules.                                                                            |

The durable post-RFC command surface should make `change` the operator-facing orchestration noun. `plan` may remain the file name (`plan.yaml`) and a subresource in command help, but it should not survive as a peer top-level CLI family. The intended shape is `specify change create`, `specify change plan {add,amend,next,status,doctor,lock}`, `specify change execute`, `specify change finalize`, and `specify change archive`. Compatibility aliases for `/spec:plan`, `/spec:execute`, or `specify plan *` may delegate into that surface during the cut-over, but new documentation should teach the change-owned form.

### Capability manifest and protocol

The capability manifest is the declarative surface the core loads before running the slice loop. It combines the existing phase-brief pipeline with the new extension surface:

```yaml
name: omnia
version: 1
description: Omnia Rust WASM workflow
pipeline: ...
artifacts: ...
consumes: ...
```

Only `name`, `version`, `description`, and `pipeline` are always present. The manifest fields are:


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

`specs` carries no privilege — the linter sorts by `id` and the renderer iterates declared order, but no core code path keys off "the first artefact" or off the literal `id: specs`. Format adapters are named after the format (`markdown-spec`, `terraform-module`, `openapi-asyncapi-bundle`), not after artefact roles. A capability like `infra@v1` declares `format: terraform-module` and never lists a `specs` entry.

Modes:

- `staged` — build writes to `$SLICE_DIR/<delta>/`; merge promotes to the declared baseline via `merge-strategy`; drop discards the delta; sibling slices read the baseline as conformance context.
- `direct` — build writes directly into a declared project path; git provides review and rollback; there is no separate promote/drop step for the artefact.
- `read-only` — declared baseline context that no slice mutates; cited by generators and reviewers (roadmap §3 codex).

An `audited` mode for checksum-recorded direct writes is deferred. The phase 2 manifest parser should reserve the word and fail with a future-use diagnostic rather than treating it as a supported mode.

`merge-strategy` and `format` are explicit fields rather than implied by id. The core ships generic implementations for `three-way` (today's spec merge) and `opaque-replace` (today's contract merge) so pure-declarative capabilities work without extension code.

##### Location fields

Every artefact entry pairs its mode with a fixed set of location fields:


| Mode        | Required location fields | Meaning                                           |
| ----------- | ------------------------ | ------------------------------------------------- |
| `staged`    | `delta:` + `baseline:`   | build writes to delta, merge promotes to baseline |
| `direct`    | `project-path:`          | build writes directly into the project tree       |
| `read-only` | `baseline:`              | sibling slices cite, no slice mutates             |


No artefact mixes location fields across modes. Cardinality is fixed at one `delta:`, one `baseline:`, and one `project-path:` per artefact (§Non-Goals).

##### Multi-instance artefacts

Direct artefacts whose `project-path` holds many sibling instances (omnia's `crates/<crate-name>/`, vectis's `<shell>/<target>/`) declare `instance-path-template:` to name the per-instance subdirectory. Staged artefacts may declare it too (a `delta:` of `specs/` with template `<crate-name>/spec.md` is exactly today's spec layout). Single-instance artefacts (`change.md`, `plan.yaml`) omit the field. The template names a single brief-bound variable; the producing brief resolves it from its context. The linter enforces that exactly one variable appears.

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
| Slice delta validation | Apply generic format checks where a declared `format:` has a core adapter. | Capability skills add domain-specific checks during define, build, review, or merge briefs. |
| Merge preview | Render the declared `merge-strategy` preview for staged artifacts. | Capability skills interpret the preview and raise behavioral risks in the phase output. |
| Merge run | Promote staged deltas via `merge-strategy`; accept direct artifact writes through git review. | Capability skills perform any prerequisite generation, verification, or close-out before merge is marked complete. |
| Drop | Remove the slice delta. | Capability skills document any direct-write cleanup required by their artifacts. |

Defaults for `three-way` and `opaque-replace` mean a pure-declarative YAML + markdown capability gets a working `define → build → merge` loop for free. Anything beyond those deterministic mechanics belongs in the capability's skills and references, where imperative code can already be included, invoked, and reviewed without adding a new core plugin runtime.

#### Consumes (read-only dependencies)

`consumes:` declares read-only dependencies on other active capabilities' adopted baselines. It answers "what may this capability read as context?" and is deliberately separate from `artifacts:`, which answers "what does this capability own?"

Rules:

- A consumed capability MUST be active in the same project.
- Consuming a capability grants no write access and creates no shared ownership of the consumed artefacts.
- A consumed baseline is context for generation, validation, review, or change coordination; it is not part of the consuming capability's merge transaction.
- `specify check` validates that consumed capability names and referenced artefact ids resolve in the active capability set.
- RFC-14 adds the `@<scope>` qualifier for workspaces: optional when there is a single provider, mandatory when multiple scopes could provide the consumed capability.

Example: `client-sdk@v1` may consume `contracts@v1` so it can generate clients from adopted OpenAPI / AsyncAPI baselines, but only `contracts@v1` may mutate those baselines.

#### Cross-capability coexistence

Within a project or scope, capability coexistence is governed by two constraints:

- **Artefact id uniqueness.** No two active capabilities may declare the same `artifact.id`.
- **Baseline-path uniqueness.** No two active capabilities may claim the same baseline path or project-path.

Read-only coupling between capabilities is declared through `consumes:` (§Consumes).

A repository activates exactly one **domain** capability under this RFC. Multi-domain repositories are covered by [RFC-14](rfc-14-workspaces.md), which adds a Cargo-style `package:` / `workspace:` shape and makes the uniqueness rules scope-aware.

#### Cross-capability coordination

When an outcome spans capabilities, the runtime does not fuse their pipelines. Coordination is explicit and platform-owned: `specify-change` records the operator outcome, close-out criteria, executable plan, execution state, and finalization checks, while `specify-registry` identifies participating projects and resolves their materialised project roots.

The change plan coordinates capability-owned slices, validations, or checks; edges express ordering (`needs:`) and blocking conditions. Any read-only baseline access used by those nodes is still declared by the target capability through `consumes:` (§Consumes). A change may deliver code, but it may also deliver contracts, docs, infrastructure, fixtures, reports, or policy artefacts.

This RFC does not define a core change runner. Change planning, validation, execution, re-entry, and finalization are `specify-change` concerns. The existing `/spec:execute` skill is therefore not a new core lifecycle command; it migrates to the change surface, where it remains the long-running orchestrator that calls core phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`) and change-owned deterministic helpers.

##### Example: landing a change

The end-to-end human loop has two operator checkpoints:

1. **Change definition.** The operator authors or refreshes `change.md`; `specify-change` authors or updates the change's `plan.yaml`. The operator reviews the desired outcome, scope, impacted projects, feature list, close-out criteria, dependencies, target projects/scopes, and slice boundaries before execution.
2. **Change execution and close-out.** The change execution action drives eligible plan entries through the core slice loop. The finalization action verifies that the plan is terminal, required PRs have merged, and the close-out criteria in `change.md` are satisfied.

### Worked `capability.yaml` example

This example shows the full declarative shape for a capability. It is illustrative rather than a frozen `vectis@v2` manifest.

```yaml
name: vectis
version: 2
description: Vectis Crux application workflow

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

#### Registry-materialised path resolution

When change execution materialises registry-declared projects, every `artifacts.*.{baseline, project-path, delta}` resolves relative to **the clone's project root**, not the hub's. `specify-registry` supplies the normalized project-root mapping used by registry-aware change execution; the core receives only the project root it should run against.

## Alternatives Considered

- **Subprocess capability plugins.** Rejected because capability skills already own imperative behavior and already have mechanisms for invoking scripts, tools, and generated code. A second plugin runtime would duplicate the skill layer and introduce a separate trust model.
- **WASM-component plugins.** Rejected for the same reason as subprocess plugins; sandboxing imperative capability code belongs in the agent/tool execution model, not in `capability.yaml`.
- **In-process dynamic-library plugins.** Rejected because Rust ABI instability disqualifies them and because the capability protocol does not need a second imperative extension path.
- **Keep `specify workspace *` as a core exception.** Rejected because it weakens the core boundary. Registry materialisation is first-party Specify behavior above core, not a core verb family.
- **Extract a standalone `workspace@v1` capability.** Rejected because materialisation is topology-driven substrate for change execution, not an outcome domain with capability-owned artifacts.
- **Split registry and workspace into separate components.** Rejected because they are two faces of the same domain: declared topology and its local materialised view. Keeping them separate also overloads "workspace" just as RFC-14 needs that noun for in-repo scopes.
- **Treat registry or change orchestration as capabilities.** Rejected because it recreates the "everything is extensible" monolith in a new vocabulary. Registry is topology plus local materialisation, and the change component is orchestration from operator brief through plan execution and close-out; each has a different lifecycle from capability-owned domain artefacts.
- **Split change orchestration and workflow into separate components.** Rejected because they are two faces of the same domain: operator intent and the executable graph that lands it. Keeping them separate gives the plan a false top-level identity, just as keeping workspace separate from registry gave materialisation a false top-level identity.
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
- **Third-party replacements for foundation components.** Registry and the change component are first-party Specify components in this RFC. Swapping them or making them externally pluggable is a follow-up RFC.
- **Multiple domain capabilities per repository.** Covered by [RFC-14](rfc-14-workspaces.md), strictly additive on top of this RFC's capability manifest protocol.
- **Cross-capability slices in a single transaction.** Multi-capability outcomes are coordinated by change plan entries, not by one slice that writes multiple capabilities' baselines. RFC-14 applies the same rule to scopes: cross-scope work is a change plan with multiple entries, not a multi-scope slice.

Multi-capability *per project* is in scope for domain capabilities (§Cross-capability coexistence). Multi-*domain*-capability per project is the RFC-14 layer.

## Glossary

| Term | Meaning |
| ---- | ------- |
| Active capability set | The domain capability set active for a project or scope. Platform components are outside this set. |
| Capability | A versioned Specify extension manifest that declares phase briefs, artefacts, and read-only dependencies. |
| Change | The umbrella orchestration concept (formerly *initiative*): an operator-defined outcome that coordinates one or more slices through `change.md` + `plan.yaml`. |
| Slice | The single unit that flows through the fixed `define → build → merge` loop (formerly *change*): a per-project transaction with its own proposal, specs, design, tasks, and merge step. |
| Domain capability | The primary project capability such as `omnia@v1`, `contracts@v1`, or `vectis@v2`. RFC-14 adds multiple domain capabilities through scopes. |
| First-party capability | A domain capability bundled with the CLI release and resolved through the same manifest path as URL capabilities. |
| Platform component | A first-party Specify subsystem above core, such as `specify-registry` or `specify-change`. Platform components are not capabilities. |
| Change component | `specify-change`, the first-party component that owns operator brief, plan, orchestration state, execution, finalization, and archive. It is not a core runner and not a capability. |
| Format adapter | The handler for artefact syntax and validation, such as `markdown-spec`, `openapi`, `asyncapi`, or `json-schema`. |
| Registry materialisation resolver | The `specify-registry` service that maps registry-declared projects to materialised project roots. |

## Implementation Scope

An incremental landing, each stage independently testable and shippable. Every stage preserves working `/spec:define → /spec:build → /spec:merge` for the `omnia` capability (the only capability currently in real use). The phases are delivery increments for this RFC, not separate RFCs.

Sizing guide:

| Phase | Expected size | Acceptance focus |
| ----- | ------------- | ---------------- |
| 1. Capability vocabulary cut-over | ~400-700 lines | Rename surfaces and diagnostics while preserving existing `pipeline:` behavior. |
| 2. Artifact declarations and adoption | ~900-1300 lines | Remove fixed `specs` / `contracts` path handling and drive merge from declared artefacts. |
| 3. Brief bindings, substitutions, and lints | ~700-1000 lines | Bind briefs to artefacts and enforce substitution/path invariants. |
| 4. Component extraction and core cleanup | ~900-1400 lines | Extract platform components, keep capabilities domain-focused, and delete concern-specific core type surfaces. |

Estimated total: ~3200-4800 lines across `specify-cli`, schema updates, fixture refreshes, and plugin documentation.

### Phase 1 — Capability vocabulary cut-over

Lands the rename without changing artefact mechanics.

1. Rename the extension primitive in manifests and project config: `schema.yaml` → `capability.yaml`, `project.yaml:schema` → `project.yaml:capability`, and `specify schema {resolve,check,pipeline}` → `specify capability {resolve,check,pipeline}`.
2. Rename the schema/manifest crate and CLI help text where they refer to Specify extensions. JSON Schema remains JSON Schema.
3. Preserve the existing `pipeline:` behavior byte-for-byte so the only behavior change in this phase is the vocabulary cut-over.
4. Update docs, fixtures, and diagnostics to use **capability** for Specify extensions and **schema** only for validation schemas.

Acceptance: a canonical omnia slice still completes through `/spec:define → /spec:build → /spec:merge`, and pre-cut-over manifests fail with a clear "schema has become capability" diagnostic.

### Phase 2 — Artifact declarations and adoption

Lands the artefact adoption surface, widened to the three supported modes.

1. New `artifacts:` fields parsed in the capability manifest crate — `id`, `mode`, the location-field set, `instance-path-template`, `merge-strategy`, `format`. JSON Schema additions enforce the mode ↔ location-field pairings from §"Location fields".
2. `crates/merge/` refactor: replace the hard-coded `specs_dir` + `contracts_dir` pair with iteration over the active capability's `staged` artifacts, dispatched on `merge-strategy`. Core ships `three-way` and `opaque-replace` defaults.
3. `crates/validate/`: add `--artifact <id>` filter.
4. `src/config.rs`: drop `specs_dir` / `contracts_dir`; add `ProjectConfig::{baseline_path, delta_path, project_path}(&capability, artifact_id)`. An instance-resolving variant takes the brief context and applies `instance-path-template`.
5. Domain capabilities adopt `artifacts:` blocks declaring today's paths exactly — no filesystem changes.

Acceptance: the core no longer carries fixed `specs` / `contracts` path helpers, and RFC-14 can layer scope-aware path resolution on the declared artefact model.

### Phase 3 — Brief bindings, substitutions, and lints

Lands the authoring contract that lets briefs refer to capability-owned locations without hard-coded paths.

1. Brief frontmatter parser learns `produces:` (single id or list). Brief loader binds each entry to an artefact in the active capability; unbound ids fail load with a diagnostic.
2. Brief renderer learns the closed substitution vocabulary (`$ARTIFACT_DELTA[...]`, `$ARTIFACT_BASELINE[...]`, `$ARTIFACT_PROJECT[...]`, `$SLICE_DIR`) and resolves instance templates from brief context.
3. `specify check` (RFC-5) lints flag direct literal paths and the per-artefact invariants: verifier brief present for `staged`, no baseline / project-path collision, pipeline stays within declared artefacts, every authoring-required artefact appears in some brief's `produces:` list, and every `instance-path-template` names exactly one variable.
4. Port capability brief prose to the substitution vocabulary.

Acceptance: a capability can add or rename an artefact location without changing core code or brief prose outside declared substitutions.

### Phase 4 — Component extraction and core surface cleanup

The largest phase: it proves the reframe without changing the lifecycle model.

1. **Extract `specify-registry` as the topology and materialisation crate.** Move `registry.yaml` parsing, add/remove/show helpers, topology validation, clone/symlink resolution, dirty-state reporting, push, and merge support out of the schema/capability crate. Registry entries carry project id, repository URL, description, and default capability; they must not embed contract roles, change status, plan status, or validation findings.
2. **Keep `.specify/workspace/` as derived registry state.** The directory may continue to hold clones or symlinks, but its contents are the local materialised view of `registry.yaml`, not a separate component-owned topology.
3. **Extract `specify-change` as the orchestration crate.** Change brief management, plan authoring, next-entry selection, locking, status updates, recovery, execution, finalization, and archive move here. `/spec:plan` and `/spec:execute` become change-surface commands or skills; any retained `/spec:plan` or `/spec:execute` spelling is only a compatibility alias.
4. **Keep change helpers internal to change execution.** The change component may use skill-owned scripts or library helpers for next-entry selection, locking, status updates, and recovery. Generic slice-loop reads such as `specify slice outcome show` stay core.
5. **Delete concern-specific core type surfaces where the artifact model replaces them.** `Commands::{Vectis, Contract}` and the matching command modules stop being the place where artifact validation and merge behavior live.
6. **Retire surviving hard-coded `contracts` / `specs` references** in `crates/merge/`, `crates/validate/`, `src/config.rs`, and the slice-loop crate (today's `crates/change/`, renamed in this phase to `crates/slice/`).
7. **First-party domain capabilities publish their full surface** — `omnia`, `contracts`, and `vectis` declare `artifacts:` + `pipeline:`. Platform components publish their own file formats and command contracts separately.
8. **Initialization wires components, not active capabilities.** A project's `project.yaml` declares its domain capability. Hub init enables registry and change-component files as platform state, but the core does not auto-activate them as capabilities.

Phase 4 may land as a sequence of smaller commits, but every commit keeps the existing `define → build → merge` lifecycle intact. The lifecycle vocabulary cut-over (today's `change` → `slice`, today's `initiative` → `change`) lands in this phase together with the component extractions that depend on it; the old surfaces are not preserved as deprecated aliases.

### This repo (`augentic/specify`)

1. Add `capabilities/capability.schema.json` (or rename the existing manifest schema) to cover `artifacts:` and `consumes:`.
2. Rewrite `capabilities/{contracts,omnia,vectis}/capability.yaml` to declare their full extension surface.
3. Port brief prose to `$ARTIFACT_DELTA[<id>]` / `$ARTIFACT_BASELINE[<id>]` substitutions.
4. Move `plugins/spec/skills/plan/` and `plugins/spec/skills/execute/` to the change surface; keep any `/spec:plan` or `/spec:execute` material as a compatibility shim only.
5. Update `plugins/contract/`, `plugins/vectis/`, and change-facing skills to own any imperative validation, generation, or review behavior that used to sit behind in-binary command modules.
6. Document the manifest protocol in `docs/reference/capabilities.md`; cross-link from each capability's README. Add companion references for `specify-registry` and `specify-change`, including registry materialisation behavior, change planning/execution behavior, and their dependency direction relative to core.

## Migration

Only the `omnia` capability and the core loop are in real-world use. `specify contract *`, `specify vectis *`, and the bulk of today's `specify plan|initiative|registry|workspace *` have no durable external user base to protect.

**Hard cut-over, no fallback path.** Each phase's minor version is a breaking change for the surfaces it touches. No deprecation window and no `artifacts:`-absent fallback: pre-reframe capability manifests fail to load against the post-reframe CLI with a clear diagnostic pointing at this RFC and the capability rename. `/spec:plan` and `/spec:execute` are not retained as `spec` plugin responsibilities; if either spelling survives, it delegates to the change surface.

### Migration TL;DR

Two vocabulary cut-overs land together: the **schema → capability** rename for the extension primitive, and the **change → slice** / **initiative → change** lifecycle rename. Every row below is a hard cut-over with a load-time diagnostic; no compatibility aliases are kept.


| Current term / surface                    | Post-RFC term / surface                       |
| ----------------------------------------- | --------------------------------------------- |
| Schema (extension primitive)              | Capability                                    |
| `schema.yaml`                             | `capability.yaml`                             |
| `project.yaml:schema`                     | `project.yaml:capability`                     |
| `specify schema {resolve,check,pipeline}` | `specify capability {resolve,check,pipeline}` |
| `schemas/<name>/schema.yaml`              | `capabilities/<name>/capability.yaml`         |
| Change (single per-loop unit)             | Slice                                         |
| Initiative (umbrella orchestration)       | Change                                        |
| `specify change *` (today's per-loop)     | `specify slice *`                             |
| `specify initiative *`                    | `specify change *`                            |
| `.specify/changes/`                       | `.specify/slices/`                            |
| `initiative.md`                           | `change.md`                                   |
| `specify-initiative` (crate / component)  | `specify-change`                              |
| `specify-change` (crate, slice loop)      | `specify-slice`                               |
| `crates/change/`                          | `crates/slice/`                               |
| `$CHANGE_DIR` (brief substitution)        | `$SLICE_DIR`                                  |


JSON Schema remains JSON Schema. `*.schema.json` continues to name validation schemas, not Specify capabilities.

The `change → slice` / `initiative → change` rows reuse the noun "change" with a new meaning. Inside a Specify project the post-cut-over reading is unambiguous: a *change* is the operator-defined umbrella, a *slice* is what flows through `define → build → merge`, and "the change loop" no longer exists as a phrase — call it the *slice loop*.

### Deferred phase rename

This RFC does **not** rename `define` / `merge` to `draft` / `adopt`. That rename would touch slash commands, CLI verbs, brief ids, journal language, metadata fields, downstream skill references, and existing fixtures. If the product wants that vocabulary later, it should land as a separate lifecycle RFC after the capability data reframe is stable.

Four invariants guard the landing:

1. **Omnia keeps working.** Every phase's acceptance criterion includes running `/spec:define → /spec:build → /spec:merge` on a canonical omnia slice end-to-end.
2. **The core never learns a capability name.** `specify check` rejects hard-coded capability-name literals in core crate sources outside tests, including first-party domain capability names after extraction.
3. **The core never learns an artefact id either.** A companion rule rejects hard-coded artefact-id literals such as `"specs"`, `"contracts"`, and `"crates"`; phase 2 retires the current canonical violations (`ProjectConfig::{specs_dir, contracts_dir}`).
4. **Platform components stay outside the active capability set.** A rule verifies `specify-core` does not depend on `specify-registry` or `specify-change`; dependency direction flows from the change component down to registry/core, never the reverse.

The hard-coded-name lints are RFC-5 design work, not a naive string-literal ban. RFC-5 should define the crate allowlist, generated-code exemptions, test exemptions, and AST-aware matching needed to avoid flagging unrelated prose or diagnostics.

Linter rules in `specify-check` (RFC-5) enforce, additionally:

- **Active-capability-set invariants:** artefact-id, baseline-path, and project-path uniqueness across active domain capabilities.
- **First-party capability parity:** bundled domain capabilities pass every rule URL-resolved capabilities must pass.
- **Brief-binding completeness:** every artefact whose mode requires authoring (`staged`, `direct`) appears in some brief's `produces:` list. `read-only` artefacts are exempt.
- **Path-substitution discipline:** brief prose references locations only via the closed substitution vocabulary; direct literal paths fail the lint.

## Open Questions

Genuinely open:

1. **Mode naming.** `staged` / `direct` / `read-only` is the provisional vocabulary; confirm or replace one more time before phase 2. `audited` remains future work, not part of the phase 2 mode set.
2. **Instance-variable resolution for multi-instance briefs.** Should the capability declare the binding source explicitly (e.g. `instance-source: artifact:specs.subdirs`), or remain a brief-side concern wired through skill code? Provisional: brief-side for now; revisit when a capability appears whose binding can't be expressed as a one-liner.

Resolved with provisional answer (see body for context):

- **Multiple capabilities per project / `capability:` shape.** Resolved by [RFC-14](rfc-14-workspaces.md): `package:` / `workspace:` shape, scope-aware uniqueness rules, and a back-compat shim for Mode-A repos.
- **Artifact mode taxonomy in phase 2.** Ship `staged`, `direct`, `read-only`; reserve `audited` as a parse-time future-use error.
- **Default `artifact-validate`.** No core default — validation is where format semantics matter most and a silent default would mask missing capability work.
- **Format-adapter catalog.** Fixed in-core catalog to start; revisit when a third-party capability wants to ship its own adapter.
- **First-party capability versioning.** Bundled domain capabilities track the CLI release as an ABI surface; projects pin via `specify_version` only.
- **Registry materialisation ownership.** Resolved by extracting `specify-registry`; registry data drives materialisation, the change component consumes the resolved project roots, and the core only receives a project root.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) — owns the crates the reframe touches (today's `specify-schema`, `specify-merge`, `specify-validate`, and `specify-change`; the slice loop crate is renamed to `specify-slice` in this RFC) and the `src/cli.rs` dispatcher.
- [RFC-8: API contracts](archive/rfc-8-api-contracts.md) — `contracts@v1` capability; delta-then-promote semantics become the `opaque-replace` default.
- [RFC-2: Execution](archive/rfc-2-execution.md) — `/spec:execute --loop`; informs the `specify-change` (formerly `specify-initiative`) extraction, but this RFC does not change the lifecycle model.
- [RFC-3a: Monoliths](archive/rfc-3a-monoliths.md) — plan authoring pipeline; the existing two-brief `pipeline.plan` is the predecessor to change plan authoring.
- [RFC-3b: Platform](archive/rfc-3b-platform.md) — registry routing and materialised project clones.
- [RFC-9: Platform](archive/rfc-9-platform.md) — moved registry, plan, initiative, and contracts to repo root; `/spec:plan --orchestrate` is the predecessor to change-driven orchestration, but not to retained top-level plan CLI families.
- [RFC-12: Refine RFC-8](archive/rfc-12-refine-rfc-8.md) — SemVer + `info.x-specify-id` rules become contracts capability validation behavior.
- [RFC-5: Framework Linter](rfc-5-lint.md) — home of the lints enforcing the reframe's invariants, including the hard-coded-name lint design.
- [Roadmap](roadmap.md) — §3 motivates `read-only`; §5 / §6 / §7 are consumers of a stable core surface.
- `plugins/contract/references/baseline-vs-delta.md`, `docs/how-to/migrate-to-v2-layout.md` — references in this `augentic/specify` repository that define the path constants and v2 layout boundary the artifact declarations make per-artefact configurable.

