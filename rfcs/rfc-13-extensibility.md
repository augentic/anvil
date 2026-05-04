# RFC-13: Extensibility

> Status: Draft · Supersedes: earlier draft at this path (artifact-adoption-only framing) · Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-8](archive/rfc-8-api-contracts.md), [RFC-9](archive/rfc-9-platform.md), [RFC-12](archive/rfc-12-refine-rfc-8.md) · Enables: [RFC-14](rfc-14-workspaces.md)

## Abstract

A capability describes how Specify's existing `define → build → merge` loop handles a class of artefacts. RFC-13 reframes the runtime to match: the **immutable core** is the loop engine plus capability-agnostic scaffolding (init, migrate, capability resolver, change driver, operation dispatcher, clone resolver), and **every capability-specific surface in today's CLI — `plan`, `initiative`, `registry`, `contract`, `vectis`, and `workspace` — becomes capability-owned**. Workflow is treated as a normal first-party capability that owns `workflow.yaml`.

Today's `schema.yaml` surface admits only `{ name, version, description, extends?, domain?, pipeline }`, which is too small to carry that contract and uses the wrong noun. This RFC renames the extension primitive to **capability** and makes `pipeline:` an explicit member of the capability manifest alongside new fields (`artifacts:`, `operations:`, `plugin:`, `config-schema:`, and optional `consumes:`) so a capability can describe its phase briefs, artefacts, CLI verbs, imperative plugin, project-level configuration, and read-only dependencies. Capabilities that need imperative code (e.g. `vectis verify` shelling out to `xcodebuild`) ship a subprocess plugin invoked through a tiny JSON protocol. "Schema" remains the term for JSON Schema / validation shapes only; every dependent RFC and implementation document must use **capability** for this extension primitive after the cut-over.

Operational cut-over:

| Current term / surface                    | Post-RFC term / surface                       |
| ----------------------------------------- | --------------------------------------------- |
| Schema (extension primitive)              | Capability                                    |
| `schema.yaml`                             | `capability.yaml`                             |
| `project.yaml:schema`                     | `project.yaml:capability`                     |
| `specify schema {resolve,check,pipeline}` | `specify capability {resolve,check,pipeline}` |
| `schemas/<name>/schema.yaml`              | `capabilities/<name>/capability.yaml`         |

## Motivation

### The core isn't actually core

Specify's current surface promises extensibility and breaks it inside the binary:

- `specify-cli/src/cli.rs` carries `Vectis { action: VectisAction }` and `Contract { action: ContractAction }` as top-level subcommands, dispatched through `specify_vectis` and `specify::validate_baseline_contracts`. Capability-specific surfaces wearing a core coat.
- `crates/merge/src/change.rs` takes `specs_dir` and `contracts_dir` as first-class parameters, carries a `ContractPreviewEntry` type, and hard-codes "3-way for specs, opaque-replace for contracts" as the entire merge universe.
- `crates/validate/src/lib.rs` re-exports `validate_baseline_contracts` — a contracts-format validator has become part of the core's public API.
- `src/config.rs` exposes `ProjectConfig::specs_dir` and `contracts_dir` as fixed helpers.
- `schemas/schema.schema.json` admits only `{ name, version, description, extends?, domain?, pipeline }`. Nothing about artifacts, operations, validators, or config can be expressed.

Every new concern — infra, client SDKs, standards, codex rules, design tokens, fixtures — therefore requires a core patch.

### One primitive already works

`schema:` in `.specify/project.yaml` is already URL-resolvable, with project-local caching under `.specify/.cache/` and inheritance via `extends`. The rename changes the noun, not the distribution model: capabilities are still remote, versioned, composable artefacts. The migration maps `schema:` to `capability:` and `schema.yaml` to `capability.yaml` (§Migration). Follow-up RFCs that currently say "schema" for the extension primitive must be updated as part of this landing so the post-RFC vocabulary has one meaning: **capability** for Specify extensions, **schema** for validation schemas.

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
- Capability-specific operator verbs (`vectis init`, `vectis verify`, `vectis add-shell`, `contract list`, `contract validate`) live in `src/cli.rs`, so adding a concern grows the core surface instead of an extension catalogue.

## Design

### Principle

**A capability describes how Specify defines, builds, and merges a class of artefacts.** The phase loop is fixed by the core; capabilities populate it with per-class choices (artefacts, validators, operator verbs, configuration). The core never switches on a capability name, never carries capability-specific type surfaces, and never ships capability-specific operator verbs. Imperative extension code is owned by the capability; the core invokes it through a fixed protocol.

"Without exception" is load-bearing. Today's `specify plan`, `specify initiative`, `specify registry`, `specify contract`, and `specify vectis` top-level verbs are capabilities masquerading as core because the reframe hasn't landed; phase 6 extracts them or routes them through capability-owned operations. If a capability-specific feature has no place in `capability.yaml`, that is a gap in the protocol, not a licence for a new core verb.

The `define → build → merge` loop's *shape* is frozen: the phase set, legal transition DAG, and per-phase outcome contract recorded in `.metadata.yaml` are part of the immutable core. Capabilities declare what *flows through* the phases (artefacts, briefs, validators, operations, config) but never the phases themselves. Variation that capabilities legitimately want lives in (a) variable briefs per phase, (b) per-operation mutation modes (§Operation mutation modes), and (c) capability-specific hooks around declared artefacts. See §Non-Goals.

The coordinating principle is the dual: **capabilities own artefacts and their adoption behavior; workflow is one such capability, not a second framework layer.** Every mutable artefact has exactly one capability owner, every reviewed change runs through exactly one capability/scope, and cross-capability outcomes are represented by capability-owned artefacts rather than by fusing capabilities into a larger hidden capability. Outcomes are not necessarily code: they may be contracts, documentation, policy, infrastructure, fixtures, reports, generated clients, workflow graphs, or any other capability-owned artefact.

### The immutable core boundary

The core is what's needed to run the fixed change loop over any capability's artefacts — no more:


| Surface                                                                    | Owner             | What it does                                                                                                                                 |
| -------------------------------------------------------------------------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify init` (+ `--hub`)                                                 | Core              | Bootstrap `.specify/`, resolve capability URL(s), cache briefs. Runs before any capability has loaded.                                       |
| `specify migrate <migration>`                                              | Core              | One-shot layout migrations.                                                                                                                  |
| `specify capability *`                                                     | Core              | Resolve, check, pipeline. Replaces today's `specify schema *`.                                                                               |
| `specify change *`                                                         | Core              | Fixed change loop: create, list, status, validate, merge, drop, transition, archive, journal, outcome, touched-specs, overlap, task.         |
| Capability operation dispatcher                                            | Core, data-driven | Invokes capability-declared operations through the common protocol.                                                                          |
| Artefact merge bookkeeping                                                 | Core, data-driven | Iterates over capability-declared artefacts.                                                                                                 |
| Clone resolver                                                             | Core, data-driven | Resolves registry-declared project clones for first-party capabilities that need workspace materialisation; it owns no operator verbs.        |
| Format validators (OpenAPI, JSON Schema, spec-markdown, …)                 | Capability        | Declared as format adapters; core vendors generic ones, capabilities may ship their own.                                                     |
| Operator verbs for a concern (`verify`, `add-shell`, `list`, `inspect`, …) | Capability        | Declared in `operations:`.                                                                                                                   |
| Project-level config for a concern                                         | Capability        | Declared in `config-schema:`, stored under `extensions.<capability>` in `project.yaml`.                                                      |


The left-hand column is frozen as the core responsibility boundary; new capability behavior lands on the right.

### What becomes a capability

Today's top-level verbs that aren't in the core table above are first-party capabilities bundled with the CLI (§First-party bootstrap):


| Today                  | Becomes                                 | Artefact                                    | Notes                                                                                                                                                                                |
| ---------------------- | --------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `specify plan *`       | `workflow@v1` capability                | `workflow.yaml`                             | Plan authoring, inspection, status, locking, and transitions become workflow-owned artefact changes or capability-declared operations.                                               |
| `/spec:execute`        | `/workflow:execute` skill               | `workflow.yaml`                             | The existing execute driver moves to the workflow plugin. It may keep `/spec:execute` only as a compatibility alias that delegates to `/workflow:execute`.                           |
| `specify initiative *` | `initiative@v1` capability              | `initiative.md`                             | Tiny: one brief; close-out maps to a hook or operation that verifies every referenced workflow is terminal and every PR merged.                                                      |
| `specify registry *`   | `registry@v1` capability                | `registry.yaml`                             | Reviewed mutations go through `specify change`; routine `add`/`remove` and workspace-style `sync`/`status`/`push`/`merge` use capability operations. The `description-missing-multi-repo` invariant becomes a `baseline-validate` finding. |
| `specify contract *`   | `contracts@v1` capability               | `contracts/` baseline                       | RFC-12's SemVer + `info.x-specify-id` checks become `baseline-validate`.                                                                                                             |
| `specify vectis *`     | `vectis@v2` capability                  | Shared / iOS / Android / design-system dirs | `verify` → `doctor`; `init` / `add-shell` → `scaffold`; `versions` → `config`.                                                                                                       |
| `specify workspace *`  | Split into `registry@v1` operations + core clone resolver | Clones directory                            | No top-level workspace command family survives. Registry operations own `sync`, `status`, `push`, and `merge`; the core clone resolver supplies path/materialisation helpers.        |


Every project activates at minimum `workflow@v1` + `initiative@v1` + `registry@v1` alongside its domain capability. All three are first-party capabilities with structured documents (`workflow.yaml`, `initiative.md`, `registry.yaml`) as their primary artefacts, not `spec.md`.

#### First-party bootstrap

`workflow@v1`, `initiative@v1`, and `registry@v1` must be available before any capability URL has been resolved: `specify init` must validate `registry.yaml`, and capability resolution itself runs through `specify capability *`, which is core. Resolution: first-party capabilities are embedded in the CLI binary and exposed through the same `capability.yaml` surface. The resolver checks the embedded set first, then falls back to URL resolution.

Embedded first-party capabilities track the CLI release as an ABI surface and validate exactly like URL-resolved capabilities. Projects pin via `specify_version` in `project.yaml`; embedded capability versions are not pinned independently. A project that opts out of a first-party capability sets `disable-first-party: [workflow]` — intentionally ugly, rarely used. Hub projects (`hub: true`) activate the three first-party capabilities without a domain capability; single-repo projects activate all four.

### Operation mutation modes

Not every capability mutation runs the full change loop. Today's `specify registry add` writes a single line; forcing it through a change directory would be absurd.

- **Reviewed mutations** — `specify change create → /spec:define → /spec:build → /spec:merge`, driven by the capability's `pipeline:`. Used when the mutation needs briefs, review, overlap detection, conflict-check, and journaling. Example: a workflow graph is authored as a reviewed change; `specify contract build` emits a new `openapi.yaml`.
- **Immediate mutations** — capability-declared operations such as `scaffold`, `config`, or `transition`, invoked without a change directory. Example: a registry capability can scaffold an entry; a vectis capability can update its configured Rust version.
- **Read-only operations** — capability-declared operations such as `list`, `inspect`, `status`, or `validate` that return projections or diagnostics without mutating project state.

Every operator-visible verb is declared in `operations:` with `mutation: reviewed | immediate | none`. Reviewed operations name the relevant `pipeline:` phase or entry; immediate and read-only operations route to declarative handling or a plugin. The core enforces one invariant: the same artefact cannot be written by both reviewed and immediate paths within a single change.

### Capability manifest and protocol

The capability manifest is the declarative surface the core loads before running the change loop. It combines the existing phase-brief pipeline with the new extension surface:

```yaml
name: omnia
version: 1
description: Omnia Rust WASM workflow
extends: ...
domain: ...
pipeline: ...
artifacts: ...
operations: ...
plugin: ...
config-schema: ...
consumes: ...
```

Only `name`, `version`, `description`, and `pipeline` are always present. `extends` and `domain` keep their current meaning. The manifest fields are:


| Field            | Meaning                                                                                |
| ---------------- | -------------------------------------------------------------------------------------- |
| `pipeline:`      | Ordered phase briefs used by `operations:` entries with `mutation: reviewed`.         |
| `artifacts:`     | Capability-owned output and context locations, with adoption mode and format metadata. |
| `operations:`    | Operator-visible verbs selected from the closed operation vocabulary.                  |
| `plugin:`        | Optional subprocess plugin used for imperative operations and hooks.                   |
| `config-schema:` | Optional JSON Schema for the capability's `extensions.<capability>` config block.      |
| `consumes:`      | Optional read-only dependencies on other active capabilities' adopted baselines.       |


The protocol pieces below describe how the core interprets that manifest. `operations:` is the complete operator/API surface; `pipeline:` is the reviewed mutation implementation detail it delegates to when an operation needs the fixed change loop. Hooks are protocol callbacks derived from declared artefacts and operations; they are not a separate top-level manifest block.

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

#### Operations (operator/API surface)

A **closed vocabulary** of operator verbs. Capabilities pick which they implement and declare a mutation mode for each entry. The core dispatches them through the capability operation dispatcher.


| Op                      | Meaning                                                      | Today's equivalent                                                                                     |
| ----------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------ |
| `list`                  | Enumerate baseline artefact instances                        | `specify contract list`                                                                                |
| `validate`              | Run the capability's baseline-wide conformance checks        | `specify contract validate`, `specify registry validate`                                               |
| `inspect <id>`          | Structured projection of one instance (or `--show`)          | `specify initiative show`, `specify registry show`                                                     |
| `doctor`                | Full diagnostic / "does it still build / satisfy invariants" | `specify vectis verify`                                                                                |
| `scaffold <kind>`       | One-shot generator                                           | `specify registry add`, `specify vectis init`, `specify vectis add-shell`, `specify initiative create` |
| `config` (get/set/show) | Read/write the capability's `extensions.<capability>` block  | `specify vectis update-versions`, `specify vectis versions`                                            |
| `transition <target>`   | State-machine step on an existing instance                   | Plan, initiative, registry, or other capability-owned state transitions                                |


The vocabulary is closed so tab-completion, JSON schemas, and cross-extension muscle memory stay stable. A capability that needs a novel verb proposes it as a protocol RFC. `transition` exists because state-machine steps recur across change-like and initiative-like artefacts.

Each op entry declares:

```yaml
operations:
  - op: scaffold
    mutation: immediate
    plugin-op: scaffold
    description: Scaffold a registry entry or project shell.
  - op: define
    mutation: reviewed
    pipeline: define
    description: Start or continue the reviewed define phase.
  - op: status
    mutation: none
    description: Render capability-owned progress.
```

`mutation: reviewed` delegates to a named `pipeline:` phase or entry and therefore runs through the fixed change loop. `mutation: immediate` runs without a change directory through declarative handling or a plugin. `mutation: none` is read-only. Each op has a standard JSON-in / JSON-out contract; every capability's `list` has the same output shape, every `validate` has the same finding shape, every `scaffold` returns the same written-files summary. The concrete schema locations are implementation detail, but the shared result shapes are part of the protocol.

##### Workflow-owned deterministic operations

`workflow@v1` owns the deterministic plan-driver operations that today's `/spec:execute` skill reaches through `specify plan *`. Those operations are capability behavior, not core lifecycle commands. They may be exposed through the capability operation dispatcher, through a workflow extension binary invoked by `/workflow:execute`, or both:

- `workflow next` — select the next eligible workflow entry and return the same machine-readable fields the driver needs (`name`, `project`, `description`, `sources`, dependency status).
- `workflow status` / `workflow validate` — render and validate workflow progress.
- `workflow transition` — update workflow-entry status (`pending`, `in-progress`, `done`, `failed`, `blocked`) with a structured reason.
- `workflow lock acquire|status|release` — manage the execution lock now represented by `.specify/plan.lock`.
- `workflow journal append` — append workflow-owned recovery or execution notes when the driver needs an audit trail.

These commands are deterministic helpers for the workflow skill. They do not need to be promoted as new human-facing `specify` core commands, and they do not change the generic core change-loop API. `/workflow:execute` may still call `specify change outcome show`, `/spec:drop`, and the phase skills because those surfaces are generic to every capability-owned change.

#### Protocol hooks (core-facing callbacks)

Invoked by core verbs on declared artefacts matching the hook's mode:


| Hook                          | Fires when                      | Default                          | Capability responsibility                |
| ----------------------------- | ------------------------------- | -------------------------------- | ---------------------------------------- |
| `artifact-validate <id>`      | A change delta is validated     | none for `staged` artefacts      | format + brief rules on the delta        |
| `artifact-preview-merge <id>` | A merge preview is requested    | core default by `merge-strategy` | produce structured preview               |
| `artifact-merge <id>`         | A merge is run                  | core default by `merge-strategy` | promote or realize the accepted artefact |
| `artifact-drop <id>`          | A change is dropped             | no-op                            | capability-side cleanup                  |
| `baseline-validate <id>`      | A baseline validate op runs     | none for `staged` artefacts      | project-wide conformance                 |


Defaults for `three-way` and `opaque-replace` mean a pure-declarative YAML + markdown capability gets a working `define → build → merge` loop for free.

Most `artifact-merge` hooks are short: promote a staged delta, accept a direct artefact's already-written project-tree state, or run a close-out check. If `workflow@v1` needs long-running execution or resumability, it expresses that as capability-owned behavior through its artefact format, operations, or plugin; this RFC does not add special workflow execution semantics to the core loop.

#### Config (per-capability settings)

A per-extension block in `.specify/project.yaml`, validated against a capability-declared `config.schema.json`:

```yaml
# .specify/project.yaml
name: my-app
capability: https://github.com/augentic/specify/capabilities/vectis@v2
domain: …
extensions:
  vectis:
    versions: { rust: 1.82.0, swift: 6.0 }
    shells: [ios, android]
  contracts:
    format-policy: strict-semver
```

Absent blocks use the capability's defaults. The core validates at `ProjectConfig::load` time; invalid config fails loud. Single-file config (nested in `project.yaml`) is chosen over a sibling capability-config file because the extension count is small and operator friction is the active concern.

#### Capability composition with the new surface


| Block            | Composition rule                                                                                                                                                  |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pipeline:`      | Child phase arrays replace the same parent phase array as a whole; omitted phases inherit from the parent. Brief entries are ordered and are not field-merged.    |
| `artifacts:`     | Merge by `id`. Child entry with same `id` **replaces** the parent (no field-level merge; mode swap would be too subtle to allow silently). Child MAY add new ids. |
| `operations:`    | Merge by `op`; child entries replace parent entries with the same `op`. Parent's unreferenced ops remain available.                                               |
| `plugin:`        | Child fully replaces parent. A plugin binary is a single artifact, not a composition. Child with no `plugin:` block inherits the parent's.                        |
| `config-schema:` | Layered via `allOf: [parent-schema, child-schema]`. Child can only tighten the parent's shape; broadening requires replacing.                                     |


Multi-level `extends:` chains and cycles are rejected at `specify capability check`.

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

When an outcome spans capabilities, the runtime does not fuse their pipelines. Coordination is explicit and artefact-owned: `initiative@v1` records the outcome and close-out criteria, `registry@v1` identifies participating projects and owns workspace-style operations, the core clone resolver resolves project/scope addresses, and `workflow@v1` owns any DAG of capability-addressed steps.

The workflow graph, if present, coordinates through the same manifest and operation protocol as every other capability. Nodes may target capability-owned changes, validations, operations, or checks; edges express ordering (`needs:`) and blocking conditions. Any read-only baseline access used by those nodes is still declared through `consumes:` (§Consumes). A workflow may deliver code, but it may also deliver contracts, docs, infrastructure, fixtures, reports, or policy changes.

This RFC does not define a core workflow runner. Workflow authoring, validation, execution, and re-entry are `workflow@v1` capability concerns and must use the same declarative or plugin-backed mechanisms available to any other capability. The existing `/spec:execute` skill is therefore not a new core lifecycle command; it migrates to `/workflow:execute`, where the skill remains the long-running orchestrator that calls core phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`) and workflow-owned deterministic operations.

##### Example: landing an initiative

The end-to-end human loop has three operator checkpoints:

1. **Initiative.** The `initiative@v1` capability defines, builds, and merges `initiative.md`. The artefact is prose containing the desired outcome, scope, impacted projects, feature list, and close-out criteria.
2. **Workflow change.** The `workflow@v1` capability defines, builds, and merges `workflow.yaml`. The operator reviews the graph, dependencies, target projects/scopes, and change boundaries before merge. `/workflow:execute` may then run the accepted graph using workflow-owned operations for next-entry selection, locking, status updates, and recovery.
3. **Initiative close-out.** The `initiative@v1` capability's finalize action verifies that referenced workflows are terminal, required PRs have merged, and the close-out criteria in `initiative.md` are satisfied.

### Worked `capability.yaml` example

This example shows the full shape for a plugin-backed capability. It is illustrative rather than a frozen `vectis@v2` manifest.

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

operations:
  - op: validate
    mutation: none
  - op: doctor
    mutation: none
    plugin-op: doctor
  - op: scaffold
    mutation: immediate
    plugin-op: scaffold
  - op: config
    mutation: immediate
    plugin-op: config

plugin:
  binary: specify-ext-vectis
  protocol-version: 1
  ops: [doctor, scaffold, config]

config-schema: schemas/vectis-config.schema.json
consumes:
  - contracts
```

### What this enables

With `capability.yaml` owning phase briefs, artefact declarations, operations, plugin routing, configuration, and read-only dependencies, new concerns ship as capabilities. None of these requires a core patch:

| Capability         | Artefact declaration                                                                   | Headline ops                                                           |
| ------------------ | -------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `infra@v1`         | `terraform`, `mode: staged`, `merge-strategy: opaque-replace`                          | `list`, `validate` (`terraform validate`), `doctor`, `scaffold module` |
| `client-sdk@v1`    | `extends: contracts`; own artefact `clients`, `mode: direct`, `project-path: clients/` | `scaffold target --lang typescript`, `doctor`                          |
| `standards@v1`     | `codex`, `mode: read-only`, `baseline: codex/`                                         | `list`, `inspect <rule-id>`, `validate`                                |
| `design-tokens@v1` | Staged token source + direct generated outputs (Swift / Kotlin / CSS)                  | `doctor` = regenerate-and-diff                                         |

None of these needs a `specs` artefact. Capabilities that want behavioural specs declare one and stage a producing brief; capabilities that do not simply omit it.

### Distribution: declarative with a subprocess escape


| Model                                                     | Reach                                                                                | Distribution                | Sandbox               | Verdict                                                                    |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------ | --------------------------- | --------------------- | -------------------------------------------------------------------------- |
| Pure declarative (YAML + markdown + named format adapter) | Artifact adoption, brief rendering, format validation the core vendors               | Capability repo only        | Total                 | **Default path.**                                                          |
| Subprocess plugin (`git-foo` convention)                  | Imperative ops needing host toolchain (`xcodebuild`, `cargo`, `gradle`, `terraform`) | PATH-installed binary       | None (operator privs) | **Escape hatch.**                                                          |
| WASM component (wasm32-wasip2)                            | Sandboxed imperative ops                                                             | Bundled in capability cache | Strong                | **Deferred.** Can't reach host toolchains without a host-function surface. |


Pure-declarative capabilities (YAML + markdown + a format adapter the core vendors — `markdown-spec`, `openapi`, `asyncapi`, `json-schema`) work end-to-end without extension code. For imperative operations, capabilities declare a subprocess plugin:

```yaml
plugin:
  binary: specify-ext-vectis         # resolved on PATH, git-foo convention
  protocol-version: 1
  ops: [doctor, scaffold, config]    # which ops route to the plugin
```

Ops not listed in `plugin.ops` fall back to declarative handling (or error). The plugin never calls back into the CLI; all state is passed on the command line or stdin. First-party plugin binaries ship as crates in the `specify-cli` workspace and are installed on PATH next to the `specify` binary; third-party capabilities provide their own `specify-ext-<capability>` binary through the operator's normal installation path. WASM-component plugins and in-process dynamic loading are out of scope; subprocess is chosen because it is language-agnostic, matches `git-foo` / `cargo-foo`, and keeps the trust boundary explicit.

#### Security posture

Subprocess plugins run with **the operator's full host privileges** — same as any other binary on PATH. The core does not sandbox them; this matches `git-foo` / `cargo-foo`. The plugin binary's PATH resolution is the trust boundary, not the capability URL: a project may trust a manifest for generation rules while still needing to vet the executable it asks the host to run. A capability URL from an untrusted source must be vetted before `specify init`, and a plugin binary from an untrusted source must not be installed on PATH. A sandboxed write-fence and WASM-component plugins are candidates for a follow-up RFC.

#### Workspace-clone path resolution

Under the `registry@v1` sync operation, every `artifacts.*.{baseline, project-path, delta}` resolves relative to **the clone's project root**, not the hub's. The core clone resolver supplies the normalized project-root mapping used by registry operations and workflow execution; it does not expose a `specify workspace *` command family.

### Protocol contract

The subprocess protocol has four moving parts: invocation envelope, args envelope on stdin, result envelope on stdout, and fixed exit-code mapping.

#### Invocation

```text
specify-ext-<capability> \
  --op <op> \
  --project-dir <abs-path> \
  --capability-cache <abs-path> \
  --protocol-version 1 \
  --format json
  < <stdin: args-json>
  > <stdout: result-json>
```

Flags are positional-free. The core chooses `--protocol-version` from the capability declaration and errors before invocation if unsupported.

#### Args envelope (stdin)

```jsonc
{
  "op": "scaffold",
  "op-args": { "kind": "shell", "target": "ios" },
  "config": { /* resolved extensions.<capability> block */ },
  "capability-name": "vectis",
  "capability-version": 2
}
```

`op-args` is validated against `schemas/ops/<op>.schema.json` before invocation.

#### Result envelope (stdout)

Plugins return either `{ "result": …, "written-paths": […], "warnings": […] }` or `{ "error": <code>, "message": <text>, "context": … }`, plus `capability-version` and `op`. All keys are kebab-case. `result` payloads and `error` variants are validated against `schemas/ops/<op>.schema.json`.

#### Exit-code mapping


| Plugin exit | Meaning                                             | Dispatcher `CliResult` |
| ----------- | --------------------------------------------------- | ---------------------- |
| `0`         | `result` present                                    | `Success`              |
| `1`         | `error` present (generic failure)                   | `GenericFailure`       |
| `2`         | `error` present (validation / missing prerequisite) | `ValidationFailed`     |


Exit codes are intentionally coarse. The dispatcher maps plugin exits through `CliResult` so declarative and plugin-backed ops share the same top-level contract; richer signal flows through the JSON `error` code and context payload.

#### Self-description

Every plugin MUST implement an implicit `describe` op:

```text
specify-ext-<capability> --op describe --protocol-version 1 < {}
```

It returns supported protocol versions, implemented ops, each op's args schema, and the plugin version. The core caches the response and uses it for help, op-arg validation, and protocol mismatch detection.

#### Protocol versioning

- Each core release declares a set of supported protocol versions (initially `[1]`).
- Each plugin declares `plugin.protocol-version` in `capability.yaml` and `protocol-versions-supported` in `describe`.
- A mismatch fails the requested capability operation with `protocol-version-unsupported` before any op runs.
- New protocol versions add to the core's set; previous versions are deprecated in release notes and retired two minor versions later.

### Operator surface

This RFC freezes the capability operation protocol, not the final operator-facing command spelling, except for the migration commitments already stated: `/spec:execute` moves to `/workflow:execute`, and no `specify workspace *` core family survives. The core must be able to resolve an active capability by name, reject unknown capabilities, expose local help from cached capability metadata, validate operation arguments, and invoke declarative or plugin-backed operations through the same result contract.

The CLI may expose that dispatcher as a dedicated prefix, as capability-scoped subcommands, as compatibility aliases for first-party capabilities, or as some combination of those. That choice is deliberately left to the implementation phase because it is product UX, not the immutable core boundary.

Capability-specific top-level families in today's CLI still leave the core. `plan`, `initiative`, `registry`, `contract`, and `vectis` become first-party capabilities with declared artefacts and operations. Whether their old spellings remain as compatibility aliases is a product decision; if retained, they route through the capability operation dispatcher rather than through core-owned command enums.

The current `workspace` family is resolved by splitting its responsibilities. `sync`, `status`, `push`, and `merge` are operations on `registry.yaml`'s declared projects, so they move under `registry@v1`. The small, reusable part the core keeps is a clone resolver: given a registry entry and workspace root, it returns the materialised project path and scope root used by registry operations, workflow execution, and artifact path resolution. This keeps git-shelling and operator verbs capability-owned while preserving one canonical clone-location algorithm.

## Alternatives Considered

- **Pure-declarative capabilities only.** Rejected as a hard rule because `vectis verify` needs host tools like `xcodebuild`; retained as the default path.
- **WASM-component plugins.** Deferred: sandboxed and aligned with Omnia, but cannot reach host toolchains without a large host-function surface.
- **In-process dynamic-library plugins.** Rejected because Rust ABI instability disqualifies them.
- **Freeze the final capability-operation CLI in this RFC.** Rejected because the architectural boundary only needs a dispatcher and protocol; command spelling is product UX and can be resolved during implementation.
- **Keep capability-specific top-level subcommands.** Rejected because the core surface would keep growing with every concern.
- **Keep `specify workspace *` as a core exception.** Rejected because it weakens the "no capability-specific operator verbs in core" rule. The retained core need is only clone resolution; the operator verbs are registry operations over registry-declared projects.
- **Extract a standalone `workspace@v1` capability.** Rejected for the first landing because `sync`, `status`, `push`, and `merge` are naturally operations on `registry@v1` data. A separate capability would mostly wrap registry state while making clone path resolution less canonical.
- **Multiple escape hatches.** Rejected because several plugin models would split the ecosystem.
- **Keep `artifacts:` adoption-only.** Rejected because artifacts are only one of four capability-specific surfaces hard-coded today.
- **A top-level `artifacts.yaml` next to `capability.yaml`.** Rejected because the extension surfaces are capability-bound, not project-bound.

## Non-Goals

- **Replacing or capability-configuring the `define → build → merge` loop.** The loop's *shape* (phase set, transition DAG, per-phase outcome contract) is part of the immutable core. Capabilities declare what flows through the phases (artefacts, briefs, validators, operations, config) but never the phases themselves. Variability lives in (a) variable briefs per phase, (b) operation mutation modes, and (c) capability hooks. A capability that genuinely cannot fit any of those would justify proposing a *second* fixed loop shape as a peer to this one — never open-ended phase configuration.
- **Format-level contract evolution.** SemVer + `info.x-specify-id` + cross-repo uniqueness continue to be owned by RFC-12; this RFC only moves where the rules run from.
- **WASM / in-process plugins.** Subprocess is the only extension runtime in this RFC.
- **A general sandboxed write-fence.** Deferred until `specify check`'s write-path inventory is trustworthy enough to enforce.
- **Cardinality > 1 on location fields.** One artefact may declare at most one `delta:`, one `baseline:`, and one `project-path:`. Revisited only if a real capability needs more.
- **Cloud execution semantics.** Orthogonal; the subprocess protocol serialises the same either way.
- **Back-compat for capabilities without the new surface.** See §Migration — current usage footprint lets us cut over without a fallback path.
- **Third-party platform capabilities.** `workflow@v1`, `registry@v1`, `initiative@v1` are first-party and bundled. Swapping them is a follow-up RFC.
- **Multiple domain capabilities per repository.** Covered by [RFC-14](rfc-14-workspaces.md), strictly additive on top of this RFC's capability manifest protocol.
- **Cross-capability changes in a single transaction.** Multi-capability outcomes are coordinated by capability-owned artefacts and operations, not by one change that writes multiple capabilities' baselines. RFC-14 applies the same rule to scopes: cross-scope work is a workflow with multiple entries, not a multi-scope change.

Multi-capability *per project* is in scope — `workflow` + `registry` + `initiative` always coexist with a domain capability (§Cross-capability coexistence). Multi-*domain*-capability per project is the RFC-14 layer.

## Glossary

| Term | Meaning |
| ---- | ------- |
| Active capability set | The domain capability plus first-party capabilities active for a project or scope. |
| Capability | A versioned Specify extension manifest that declares phase briefs, artefacts, operations, plugin routing, config, and read-only dependencies. |
| Domain capability | The primary project capability such as `omnia@v1`, `contracts@v1`, or `vectis@v2`. RFC-14 adds multiple domain capabilities through scopes. |
| First-party capability | A capability bundled with the CLI release and resolved through the same manifest path as URL capabilities. |
| Platform capability | A first-party capability that supports coordination or project metadata, such as `registry@v1` or `initiative@v1`. |
| Workflow capability | `workflow@v1`, the first-party capability that owns `workflow.yaml` and `/workflow:execute`. It is not a core runner. |
| Operation dispatcher | The core data-driven entry point that validates and invokes capability-declared operations. |
| Reviewed mutation | An operation with `mutation: reviewed`; it delegates to `pipeline:` and runs through `define → build → merge`. |
| Immediate mutation | An operation with `mutation: immediate`; it writes through declarative handling or a plugin without a change directory. |
| Read-only operation | An operation with `mutation: none`; it returns projections or diagnostics without writing project state. |
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
| 4. Operations and config surface | ~700-1100 lines | Add operation metadata, config schemas, and dispatcher plumbing without subprocess plugins. |
| 5. Subprocess protocol and first extraction | ~900-1400 lines | Prove plugin protocol with Vectis and move contract validation behind a capability adapter. |
| 6. First-party extraction and core cleanup | ~1200-1800 lines | Move first-party command families behind capabilities and delete concern-specific core enums. |

Estimated total: ~4800-7300 lines across `specify-cli`, schema updates, fixture refreshes, and plugin documentation.

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

### Phase 4 — Operations and config surface

Lands the declarative operator surface without requiring subprocess plugins.

1. New capability operation dispatcher in `src/cli.rs` and `src/commands/`. The dispatcher resolves an active capability, validates op arguments, and returns standard JSON-in / JSON-out results.
2. Capability surface grows `operations:` with the closed vocabulary and per-op metadata, including `mutation: reviewed | immediate | none`.
3. Capability surface grows `config-schema:`. The core validates `extensions.<capability>` blocks at `ProjectConfig::load` and exposes resolved config to declarative operations.
4. Add `schemas/ops/<op>.schema.json` for each op in the closed vocabulary (`list`, `validate`, `inspect`, `doctor`, `scaffold`, `config`, `transition`).

Acceptance: a capability can declare operator-visible verbs and project config without adding a concern-specific enum variant to the core CLI.

### Phase 5 — Subprocess protocol and first extraction

Lands the imperative escape hatch and proves it with a real first-party capability.

1. Capability surface grows `plugin:` with PATH-based binary resolution, protocol versioning, and `plugin.ops`.
2. Implement the JSON subprocess protocol: invocation envelope, args envelope, result envelope, exit-code mapping, and implicit `describe`.
3. Extract `specify-ext-vectis` from today's in-binary `specify_vectis` library; ship it as a sibling crate in the `specify-cli` workspace, installed on PATH next to `specify`.
4. Move `validate_baseline_contracts` out of `crates/validate/src/` into a `format: openapi-asyncapi-bundle` adapter declared by the contracts capability. The core validate crate stops knowing the word "contract".

Acceptance: at least one first-party concern runs through the same protocol a third-party capability would use.

### Phase 6 — First-party extraction and core surface cleanup

The largest phase: it proves the reframe without changing the lifecycle model.

1. **Extract `workflow@v1`, `registry@v1`, `initiative@v1`, `contracts@v1`, and `vectis@v2` as first-party capabilities** embedded in the CLI via `include_str!` or a tidy `embedded-capabilities/` tree, exposed through the same resolver path as URL-resolved capabilities.
2. **Move the execute driver to the workflow plugin.** `/spec:execute` becomes `/workflow:execute`; any retained `/spec:execute` spelling is only a compatibility alias that delegates to the workflow skill.
3. **Port plan-driver primitives to workflow-owned deterministic operations.** Today's `specify plan next`, `status`, `validate`, `transition`, and `lock acquire|status|release` become `workflow@v1` operations or workflow extension-binary commands used by `/workflow:execute`. Generic change-loop reads such as `specify change outcome show` stay core.
4. **Cut operator verbs over to the capability operation dispatcher.** Compatibility aliases for existing command spellings may remain, but they must route through the dispatcher and capability metadata.
5. **Delete `Commands::{Plan, Initiative, Registry, Vectis, Contract}`** as capability-specific core implementations from `src/cli.rs` and the matching modules under `src/commands/`.
6. **Split workspace behavior.** Move `sync`, `status`, `push`, and `merge` under `registry@v1` operations and keep only the core clone resolver needed by registry and workflow code paths.
7. **Retire surviving hard-coded `contracts` / `specs` references** in `crates/merge/`, `crates/validate/`, `src/config.rs`, `crates/change/`.
8. **First-party capabilities publish their full surface** — `omnia`, `contracts`, `vectis`, `workflow`, `registry`, `initiative` declare `artifacts:` + `operations:` + (where applicable) `plugin:` + `config-schema:` + `pipeline:`.
9. **Auto-activation at `specify init`.** A project's `project.yaml` declares its domain capability; the core auto-activates `workflow@v1`, `registry@v1`, and `initiative@v1`. Hubs activate the three without a domain capability.

Phase 6 may land as a sequence of smaller commits, but every commit keeps the existing `define → build → merge` lifecycle intact.

### This repo (`augentic/specify`)

1. Add `capabilities/capability.schema.json` (or rename the existing manifest schema) to cover `artifacts:`, `operations:`, `plugin:`, `config-schema:`, `consumes:`.
2. Add `schemas/ops/<op>.schema.json` for each op in the closed vocabulary (`list`, `validate`, `inspect`, `doctor`, `scaffold`, `config`, `transition`).
3. Rewrite `capabilities/{contracts,omnia,vectis,workflow,registry,initiative}/capability.yaml` to declare their full extension surface.
4. Port brief prose to `$ARTIFACT_DELTA[<id>]` / `$ARTIFACT_BASELINE[<id>]` substitutions.
5. Move `plugins/spec/skills/execute/` to the workflow plugin surface as `/workflow:execute`; keep any `/spec:execute` material as a compatibility shim only.
6. Update `plugins/contract/`, `plugins/vectis/`, and workflow-facing skills to invoke capability operations through the new dispatcher or their capability-owned extension binaries.
7. Document the protocol in `docs/reference/capabilities.md`; cross-link from each capability's README. Add glossary entries for "active capability set," "workflow graph," "reviewed mutation," "immediate mutation," "read-only operation," and "first-party capability."

## Migration

Only the `omnia` capability and the core loop are in real-world use. `specify contract *`, `specify vectis *`, and the bulk of `specify plan|initiative|registry *` have no durable external user base to protect. The operator-facing CLI reshapes considerably in phase 6. Capability behavior is preserved behind the dispatcher.

**Hard cut-over, no fallback path.** Each phase's minor version is a breaking change for the surfaces it touches. No deprecation window and no `artifacts:`-absent fallback: pre-reframe capability manifests fail to load against the post-reframe CLI with a clear diagnostic pointing at this RFC and the capability rename. Compatibility aliases for command spelling are allowed only if they route through capability-owned operations. `/spec:execute` is not retained as a `spec` plugin responsibility; if the spelling survives, it delegates to `/workflow:execute`.

### Migration TL;DR

The rename is part of that cut-over:


| Current term / surface                    | Post-RFC term / surface                       |
| ----------------------------------------- | --------------------------------------------- |
| Schema (extension primitive)              | Capability                                    |
| `schema.yaml`                             | `capability.yaml`                             |
| `project.yaml:schema`                     | `project.yaml:capability`                     |
| `specify schema {resolve,check,pipeline}` | `specify capability {resolve,check,pipeline}` |
| `schemas/<name>/schema.yaml`              | `capabilities/<name>/capability.yaml`         |


JSON Schema remains JSON Schema. `config-schema:` and `*.schema.json` continue to name validation schemas, not Specify capabilities.

### Deferred phase rename

This RFC does **not** rename `define` / `merge` to `draft` / `adopt`. That rename would touch slash commands, CLI verbs, brief ids, journal language, metadata fields, downstream skill references, and existing fixtures. If the product wants that vocabulary later, it should land as a separate lifecycle RFC after the capability data reframe is stable.

Four invariants guard the landing:

1. **Omnia keeps working.** Every phase's acceptance criterion includes running `/spec:define → /spec:build → /spec:merge` on a canonical omnia change end-to-end.
2. **The core never learns a capability name.** `specify check` rejects hard-coded capability-name literals in core crate sources outside tests, including first-party names after extraction.
3. **The core never learns an artefact id either.** A companion rule rejects hard-coded artefact-id literals such as `"specs"`, `"contracts"`, `"crates"`, and `"workflow"`; phase 2 retires the current canonical violations (`ProjectConfig::{specs_dir, contracts_dir}`).
4. **First-party capabilities are still capabilities.** A rule verifies `workflow@v1`, `registry@v1`, `initiative@v1` each pass the same validation as any third-party capability — `capability.yaml` parses against the capability manifest JSON Schema, all declared briefs exist, `operations:` is a subset of the closed vocabulary, and so on.

The hard-coded-name lints are RFC-5 design work, not a naive string-literal ban. RFC-5 should define the crate allowlist, generated-code exemptions, test exemptions, and AST-aware matching needed to avoid flagging unrelated prose or diagnostics.

Linter rules in `specify-check` (RFC-5) enforce, additionally:

- A capability's `operations:` MUST be a subset of the closed op vocabulary.
- A capability's `plugin.binary` MUST resolve on PATH or be declared absent.
- A capability's `config-schema:` MUST parse as a JSON Schema.
- **Active-capability-set invariants:** artefact-id, baseline-path, and project-path uniqueness across active capabilities.
- **First-party capability parity:** embedded capabilities pass every rule URL-resolved capabilities must pass.
- **Brief-binding completeness:** every artefact whose mode requires authoring (`staged`, `direct`) appears in some brief's `produces:` list. `read-only` artefacts are exempt.
- **Path-substitution discipline:** brief prose references locations only via the closed substitution vocabulary; direct literal paths fail the lint.

## Open Questions

Genuinely open:

1. **Distribution model beyond subprocess.** When does WASM become worth adding? Provisional: revisit when the third capability asks, or when a hosting constraint forces sandboxing (RFC-7 cloud execution).
2. **Mode naming.** `staged` / `direct` / `read-only` is the provisional vocabulary; confirm or replace one more time before phase 2. `audited` remains future work, not part of the phase 2 mode set.
3. **Operator CLI spelling.** Dedicated prefix, capability-scoped subcommands, compatibility aliases, or a mix? Provisional: choose during implementation after testing discoverability and completion behavior.
4. **Instance-variable resolution for multi-instance briefs.** Should the capability declare the binding source explicitly (e.g. `instance-source: artifact:specs.subdirs`), or remain a brief-side concern wired through skill code? Provisional: brief-side for now; revisit when a capability appears whose binding can't be expressed as a one-liner.

Resolved with provisional answer (see body for context):

- **Multiple capabilities per project / `capability:` shape.** Resolved by [RFC-14](rfc-14-workspaces.md): `package:` / `workspace:` shape, scope-aware uniqueness rules, back-compat shim for Mode-A repos, `disable-first-party:` survives.
- **Artifact mode taxonomy in phase 2.** Ship `staged`, `direct`, `read-only`; reserve `audited` as a parse-time future-use error.
- **Operations vocabulary closed vs open.** Closed; novel ops require a protocol RFC.
- **Operation mutation boundary.** Declared per op via `mutation: reviewed | immediate | none`; reviewed operations delegate to `pipeline:`, immediate operations route to declarative handling or plugins, and `none` is read-only.
- **Config location.** Nested `extensions.<name>` under `project.yaml`; revisit if extension count grows past a dozen.
- **Plugin resolution.** PATH-based (`specify-ext-<capability>`), matching `git-foo`. Capability-local complicates caching.
- **Default `artifact-validate`.** No core default — validation is where format semantics matter most and a silent default would mask missing capability work.
- **Exit-code propagation.** Map through `CliResult` so the top-level surface stays uniform; richer signal goes via JSON `error`.
- **Per-op help authoring.** Auto-derive from `op-args-schema` plus an optional `description` on each op entry in `describe`. Hand-authored long-form help is a future-RFC concern.
- **Format-adapter registry.** Fixed in-core registry to start; revisit when a third-party capability wants to ship its own.
- **First-party capability versioning.** Embedded capabilities track the CLI release as an ABI surface; projects pin via `specify_version` only.
- **Workspace ownership.** Resolved by splitting the surface: `registry@v1` owns `sync`, `status`, `push`, and `merge` operations over registry-declared projects; the core keeps only a clone resolver with no operator command family.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) — owns the crates the reframe touches (`specify-schema`, `specify-merge`, `specify-validate`, `specify-change`) and the `src/cli.rs` dispatcher.
- [RFC-8: API contracts](archive/rfc-8-api-contracts.md) — `contracts@v1` capability; delta-then-promote semantics become the `opaque-replace` default.
- [RFC-2: Execution](archive/rfc-2-execution.md) — `/spec:execute --loop`; informs the workflow capability migration, but this RFC does not change the lifecycle model.
- [RFC-3a: Monoliths](archive/rfc-3a-monoliths.md) — plan authoring pipeline; the existing two-brief `pipeline.plan` is the predecessor to workflow artefact authoring.
- [RFC-3b: Platform](archive/rfc-3b-platform.md) — registry routing and workspace clones.
- [RFC-9: Platform](archive/rfc-9-platform.md) — moved registry, plan, initiative, and contracts to repo root; `/spec:plan --orchestrate` is the predecessor to workflow-driven orchestration, but not to a retained workflow CLI family.
- [RFC-12: Refine RFC-8](archive/rfc-12-refine-rfc-8.md) — SemVer + `info.x-specify-id` rules become `contracts`'s `baseline-validate` hook.
- [RFC-5: Framework Linter](rfc-5-lint.md) — home of the lints enforcing the reframe's invariants, including the hard-coded-name lint design.
- [Roadmap](roadmap.md) — §3 motivates `read-only`; §5 / §6 / §7 are consumers of a stable core surface.
- `plugins/contract/references/baseline-vs-delta.md`, `docs/how-to/migrate-to-v2-layout.md` — references in this `augentic/specify` repository that define the path constants and v2 layout boundary the artifact declarations make per-artefact configurable.

