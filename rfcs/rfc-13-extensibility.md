# RFC-13: Capabilities

> Status: Draft · Supersedes: earlier draft at this path (artifact-lifecycle-only framing) · Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-8](archive/rfc-8-api-contracts.md), [RFC-9](archive/rfc-9-platform.md), [RFC-12](archive/rfc-12-refine-rfc-8.md) · Enables: [RFC-14](rfc-14-workspaces.md)

## Abstract

A capability describes how to draft, build, and adopt a class of artefacts. RFC-13 reframes the runtime to match: the **immutable core** is the **Layer 1 draft-build-adopt loop engine** plus capability-agnostic scaffolding (init, migrate, capability resolver, change driver, **Layer 2 workflow runner**, operation dispatcher), and **every capability-specific surface in today's CLI — `plan`, `initiative`, `registry`, `contract`, `vectis`, most of `workspace` — becomes capability-owned**. The core never switches on a capability name.

Today's `schema.yaml` surface admits only `{ name, version, description, extends?, domain?, pipeline }`, which is too small to carry that contract and uses the wrong noun. This RFC renames the extension primitive to **capability** and adds four declarative blocks (`artifacts:`, `operations:`, `plugin:`, `config-schema:`) so a capability can describe its artefacts, CLI verbs, lifecycle hooks, and project-level configuration. Capabilities that need imperative code (e.g. `vectis verify` shelling out to `xcodebuild`) ship a subprocess plugin invoked through a tiny JSON protocol. "Schema" remains the term for JSON Schema / validation shapes only.

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

`schema:` in `.specify/project.yaml` is already URL-resolvable, with project-local caching under `.specify/.cache/` and inheritance via `extends`. The rename changes the noun, not the distribution model: capabilities are still remote, versioned, composable artefacts. The migration maps `schema:` to `capability:` and `schema.yaml` to `capability.yaml` (§Migration).

### Artefact lifecycles, one runtime

A change under any current capability produces outputs governed by one of these lifecycles (§Design widens to four — `managed`, `external`, `read-only`, `audited`):


| Artifact                                                               | Build writes to                      | Baseline location      | Adopt promotes?              | Drop reverts?    |
| ---------------------------------------------------------------------- | ------------------------------------ | ---------------------- | ---------------------------- | ---------------- |
| Behavioral specs (declared by `omnia@v1`, `contracts@v1`, `vectis@v2`) | `.specify/changes/<name>/specs/`     | `.specify/specs/`      | Yes (file-level merge)       | Yes              |
| Contracts (`contracts@v1`)                                             | `.specify/changes/<name>/contracts/` | `<root>/contracts/`    | Yes (whole-file replacement) | Yes              |
| Crates (`omnia@v1`)                                                    | `<root>/crates/<crate>/`             | (no separate baseline) | No                           | No (git-managed) |
| Shared / iOS / Android / design-system (`vectis@v2`)                   | `<root>/<dir>/`                      | (no separate baseline) | No                           | No (git-managed) |


The first two rows are *managed* — Specify owns a versioned baseline; the bottom two are *external* — git provides versioning. The asymmetry is correct; today it is encoded in Rust rather than declared by the capability.

The strategy: **every artefact's write location(s) are declared by the capability in `artifacts:`, never inferred by the core and never invented by a brief.** Managed artefacts declare `delta:` + `baseline:`; external artefacts declare `project-path:`; read-only artefacts declare `baseline:`; audited artefacts declare `baseline:` plus a recorded checksum. Brief prose references each location through a closed substitution vocabulary (§"Substitution vocabulary"); literal paths are forbidden. Workspace clones resolve every declared location relative to the clone root (§"Workspace-clone path resolution").

`spec.md` is an artefact like any other. It appears today because behaviour drives the current generators; capabilities whose primary deliverable is not behavioural declare their own artefact and skip `specs`.

### What the status quo blocks

- A future `infra@v1` cannot declare "the `terraform/` directory is a managed baseline" without patching `specify-cli`.
- A future `standards@v1` (roadmap §3) needs `read-only` baselines that sibling changes cite but never mutate; today the only lifecycles are `managed` and `external`.
- The format validators behind `specify contract validate` live in the core's public API, so a third-party capability cannot ship an equivalent without patching core.
- Capability-specific operator verbs (`vectis init`, `vectis verify`, `vectis add-shell`, `contract list`, `contract validate`) live in `src/cli.rs`, so adding a concern grows the core surface instead of an extension catalogue.

### What this enables

With `capability.yaml` owning artifacts, operations, hooks, and configuration, new concerns ship as capabilities. None of these requires a core patch:


| Capability         | Artefact declaration                                                                          | Headline ops                                                           |
| ------------------ | --------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `infra@v1`         | `terraform`, `lifecycle: managed`, `merge-strategy: opaque-replace`                           | `list`, `validate` (`terraform validate`), `doctor`, `scaffold module` |
| `client-sdk@v1`    | `extends: contracts`; own artefact `clients`, `lifecycle: external`, `project-path: clients/` | `scaffold target --lang typescript`, `doctor`                          |
| `standards@v1`     | `codex`, `lifecycle: read-only`, `baseline: codex/`                                           | `list`, `inspect <rule-id>`, `validate`                                |
| `design-tokens@v1` | Managed token source + external generated outputs (Swift / Kotlin / CSS)                      | `doctor` = regenerate-and-diff                                         |
| `fixtures@v1`      | `fixtures`, `lifecycle: audited`, `baseline: fixtures/`                                       | `list`, `inspect`, `scaffold capture`                                  |


None of these needs a `specs` artefact. Capabilities that want behavioural specs declare one and stage a producing brief; capabilities that do not simply omit it.

## Design

### Principle

**A capability describes how to draft-build-adopt a class of artefacts.** Layer 1 is the draft-build-adopt loop; capabilities populate it with per-class choices (artefacts, validators, operator verbs, configuration). The core never switches on a capability name, never carries capability-specific type surfaces, and never ships capability-specific operator verbs. Imperative extension code is owned by the capability; the core invokes it through a fixed protocol.

"Without exception" is load-bearing. Today's `specify plan`, `specify initiative`, `specify registry`, `specify contract`, `specify vectis` top-level verbs are five capabilities masquerading as core because the reframe hasn't landed; phase 4 extracts them. If a capability-specific feature has no place in `capability.yaml`, that is a gap in the protocol, not a licence for a new core verb.

Layer 1 is the draft-build-adopt loop. Its *shape* is frozen alongside the verbs: the phase set (`draft` / `build` / `adopt` — today's `define` / `build` / `merge`), the legal transition DAG, and the per-phase outcome contract recorded in `.metadata.yaml` are part of the immutable core. Capabilities declare what *flows through* the phases (artefacts, briefs, validators, operations, config) but never the phases themselves. Variation that capabilities legitimately want lives in (a) variable briefs per phase, (b) Layer 2 workflow graphs over capability-owned steps, and (c) the heavy-vs-light mutation split (§Heavy vs light mutations). See §Non-Goals.

The coordinating principle is the dual: **capabilities own artefact lifecycles; workflows coordinate capabilities into outcomes.** Every mutable artefact has exactly one capability owner, every reviewed change runs through exactly one capability/scope, and cross-capability outcomes are achieved by a workflow graph, not by fusing capabilities into a larger hidden capability. Outcomes are not necessarily code: they may be contracts, documentation, policy, infrastructure, fixtures, reports, generated clients, or any other capability-owned artefact. `workflow@v1` wires capability-owned steps through the common protocol.

That gives Specify two fixed framework layers:

- **Layer 1 — Draft-build-adopt.** One capability owns one artefact family; a reviewed change drafts, builds, validates, and adopts that capability's artefacts.
- **Layer 2 — Workflow.** A workflow coordinates Layer 1 changes, validations, capability-declared operations, and adoption checks into an outcome graph. The graph is stored in `workflow@v1`; the runner is core-owned and capability-agnostic.

### The immutable core boundary

The core is what's needed to run Layer 1 draft-build-adopt over any capability's artefacts and Layer 2 workflows over capability-owned steps — no more:


| Surface                                                                    | Owner             | What it does                                                                                                                                 |
| -------------------------------------------------------------------------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify init` (+ `--hub`)                                                 | Core              | Bootstrap `.specify/`, resolve capability URL(s), cache briefs. Runs before any capability has loaded.                                        |
| `specify migrate <migration>`                                              | Core              | One-shot layout migrations.                                                                                                                  |
| `specify capability *`                                                      | Core              | Resolve, check, pipeline. Replaces today's `specify schema *`.                                                                                |
| `specify change *`                                                         | Core              | Layer 1 draft-build-adopt: create, list, status, validate, adopt, drop, transition, archive, journal, outcome, touched-specs, overlap, task. |
| Capability operation dispatcher                                            | Core, data-driven | Invokes capability-declared operations through the common protocol.                                                                           |
| Artefact-lifecycle bookkeeping                                             | Core, data-driven | Iterates over capability-declared artefacts.                                                                                                  |
| Layer 2 workflow runner                                                    | Core, data-driven | Walks `workflow@v1` graphs and invokes capability-owned steps through the common protocol.                                                     |
| Format validators (OpenAPI, JSON Schema, spec-markdown, …)                 | Capability        | Declared as format adapters; core vendors generic ones, capabilities may ship their own.                                                       |
| Operator verbs for a concern (`verify`, `add-shell`, `list`, `inspect`, …) | Capability        | Declared in `operations:`.                                                                                                                    |
| Project-level config for a concern                                         | Capability        | Declared in `config-schema:`, stored under `extensions.<capability>` in `project.yaml`.                                                       |


The left-hand column is frozen as the core responsibility boundary; new capability behavior lands on the right. This table intentionally does not freeze the final operator-facing command spelling for capability operations.

### What becomes a capability

Today's top-level verbs that aren't in the core table above are first-party capabilities bundled with the CLI (§First-party capabilities and bootstrap):


| Today                  | Becomes                                 | Artefact                                    | Notes                                                                                                                                                                                |
| ---------------------- | --------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `specify plan *`       | `workflow@v1` capability                | `workflow.yaml`                             | First-party Layer 2 workflow capability and successor to today's plan surface. Heavy mutations author the graph; light mutations use `scaffold` / `config` / `transition`.           |
| `specify initiative *` | `initiative@v1` capability              | `initiative.md`                             | Tiny: one brief; `finalize` maps to `adopt` with a close-out hook that verifies every referenced workflow is terminal and every PR merged.                                           |
| `specify registry *`   | `registry@v1` capability                | `registry.yaml`                             | Heavy mutations go through `specify change`; routine `add`/`remove` use `scaffold` / `config`. The `description-missing-multi-repo` invariant becomes a `baseline-validate` finding. |
| `specify contract *`   | `contracts@v1` capability               | `contracts/` baseline                       | RFC-12's SemVer + `info.x-specify-id` checks become `baseline-validate`.                                                                                                             |
| `specify vectis *`     | `vectis@v2` capability                  | Shared / iOS / Android / design-system dirs | `verify` → `doctor`; `init` / `add-shell` → `scaffold`; `versions` → `config`.                                                                                                       |
| `specify workspace *`  | **Open question** (see §Open Questions) | Clones directory                            | Mostly git-shelling. May stay core as the cross-capability coordinator, or become a subprocess-plugin-heavy capability.                                                              |


Every project activates at minimum `workflow@v1` + `initiative@v1` + `registry@v1` alongside its domain capability. `workflow@v1` is framework-level Layer 2, while `initiative@v1` and `registry@v1` remain platform capabilities. All three declare structured documents (`workflow.yaml`, `initiative.md`, `registry.yaml`) as their primary artefacts, not `spec.md`.

### Heavy vs light mutations

Not every capability mutation runs the full change loop. Today's `specify registry add` writes a single line; forcing it through a change directory would be absurd.

- **Heavy (reviewed) mutations** — `specify change create → /spec:draft → /spec:build → /spec:adopt`, driven by the capability's pipeline. Used when the mutation needs briefs, review, overlap detection, conflict-check, and journaling. Example: `/spec:plan` authoring a workflow graph; `specify contract build` emitting a new `openapi.yaml`.
- **Light (direct) mutations** — capability-declared operations such as `scaffold`, `config`, or `transition`, invoked without a change directory. Example: a workflow capability can scaffold an entry; a vectis capability can update its configured Rust version.

Capabilities decide which path each verb takes by routing it through `operations:` (light) or `pipeline:` (heavy). The core enforces one invariant: the same artefact cannot be written by both paths within a single change.

### The four-part protocol

The capability surface gains four new top-level blocks, each a flat vocabulary the core knows and the capability populates.

#### 1. Artifacts (declarative lifecycle)

Every output location a capability owns is declared once, with an explicit lifecycle. The canonical example covers all three relevant patterns:

```yaml
# omnia@v1 — Rust + WASM services. Specs drive code generation.
artifacts:
  - id: specs
    lifecycle: managed                  # managed | external | read-only | audited
    delta: specs/
    baseline: .specify/specs/
    merge-strategy: three-way           # three-way | opaque-replace | none
    format: markdown-spec               # core or capability-declared format adapter
  - id: crates
    lifecycle: external
    project-path: crates/
    instance-path-template: <crate-name>/
  - id: codex
    lifecycle: read-only
    baseline: codex/
```

`specs` carries no privilege — the linter sorts by `id` and the renderer iterates declared order, but no core code path keys off "the first artefact" or off the literal `id: specs`. Format adapters are named after the format (`markdown-spec`, `terraform-module`, `workflow-yaml`, `openapi-asyncapi-bundle`), not after artefact roles. A capability like `infra@v1` declares `format: terraform-module` and never lists a `specs` entry; `workflow@v1`'s sole artefact is `id: workflow, format: workflow-yaml`.

Lifecycles:

- `managed` — Specify-owned baseline; build writes to `$CHANGE_DIR/<delta>/`; adopt promotes via `merge-strategy`; drop discards the delta; sibling changes read the baseline as conformance context.
- `external` — downstream toolchain owns the artifact; build writes directly; git provides versioning; no adopt, no drop, no conflict-check.
- `read-only` — Specify-owned baseline that no change mutates; cited by generators and reviewers (roadmap §3 codex).
- `audited` — direct-write baseline with a checksum recorded in the change; adopt bumps the checksum, drop reverts. Implementation deferred but reserved.

`merge-strategy` and `format` are explicit fields rather than implied by id. The core ships generic implementations for `three-way` (today's spec merge) and `opaque-replace` (today's contract merge) so pure-declarative capabilities work without extension code.

##### Location fields

Every artefact entry pairs its lifecycle with a fixed set of location fields:


| Lifecycle   | Required location fields | Meaning                                                          |
| ----------- | ------------------------ | ---------------------------------------------------------------- |
| `managed`   | `delta:` + `baseline:`   | build writes to delta, adopt promotes to baseline                |
| `external`  | `project-path:`          | build writes directly into the project tree; no Specify baseline |
| `read-only` | `baseline:`              | sibling changes cite, no change mutates                          |
| `audited`   | `baseline:`              | direct-write baseline; change records a checksum, adopt bumps it |


No artefact mixes location fields across lifecycles. Cardinality is fixed at one per field per artefact (§Non-Goals).

##### Multi-instance artefacts

External artefacts whose `project-path` holds many sibling instances (omnia's `crates/<crate-name>/`, vectis's `<shell>/<target>/`) declare `instance-path-template:` to name the per-instance subdirectory. Managed artefacts may declare it too (a `delta:` of `specs/` with template `<crate-name>/spec.md` is exactly today's spec layout). Single-instance artefacts (`workflow.yaml`, `initiative.md`) omit the field. The template names a single brief-bound variable; the producing brief resolves it from its context. The linter enforces that exactly one variable appears.

##### Substitution vocabulary

The closed vocabulary in brief prose covers every declared location:


| Substitution                    | Resolves to                                          | Available for                     |
| ------------------------------- | ---------------------------------------------------- | --------------------------------- |
| `$ARTIFACT_DELTA[<id>]`         | the artefact's declared `delta:` path                | `managed`                         |
| `$ARTIFACT_BASELINE[<id>]`      | the artefact's declared `baseline:` path             | `managed`, `read-only`, `audited` |
| `$ARTIFACT_PROJECT_PATH[<id>]`  | the artefact's declared `project-path:`              | `external`                        |
| `$ARTIFACT_INSTANCE_PATH[<id>]` | the location resolved with `instance-path-template:` | any lifecycle that declares one   |


Direct literal paths are forbidden and flagged by `specify check`.

##### Brief-to-artefact binding

Briefs declare which artefact id(s) they produce via a `produces:` frontmatter field, the symmetric counterpart to `needs:` and `tracks:`. The field binds the brief to one or more ids in the active capability's `artifacts:` block, which is what lets substitutions resolve at render time and what lets `specify check` verify that every declared `managed` artefact has a producing brief in `pipeline.build`:

```yaml
---
id: build
description: Implement the tasks in tasks.md by delegating to the skills below
needs: [specs, design, tasks]
produces: [crates]
tracks: tasks
---
```

A brief that produces a multi-instance artefact resolves the instance variable from its own context. The linter enforces that every artefact whose lifecycle requires authoring (`managed`, `external`, `audited`) appears in some brief's `produces:` list. `read-only` artefacts are exempt.

#### 2. Operations (operator surface)

A **closed vocabulary** of operator verbs. Capabilities pick which they implement; the core dispatches them through the capability operation dispatcher.


| Op                      | Meaning                                                       | Today's equivalent                                                                                                         |
| ----------------------- | ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `list`                  | Enumerate baseline artefact instances                         | `specify contract list`, `specify plan status`                                                                             |
| `validate`              | Run the capability's baseline-wide conformance checks         | `specify contract validate`, `specify plan validate`, `specify registry validate`                                          |
| `inspect <id>`          | Structured projection of one instance (or `--next`, `--show`) | `specify plan next`, `specify initiative show`, `specify registry show`                                                    |
| `doctor`                | Full diagnostic / "does it still build / satisfy invariants"  | `specify vectis verify`, `specify plan doctor`                                                                             |
| `scaffold <kind>`       | One-shot generator                                            | `specify plan add`, `specify registry add`, `specify vectis init`, `specify vectis add-shell`, `specify initiative create` |
| `config` (get/set/show) | Read/write the capability's `extensions.<capability>` block   | `specify vectis update-versions`, `specify vectis versions`                                                                |
| `transition <target>`   | State-machine step on an existing instance                    | `specify plan transition`, `specify plan lock`                                                                             |


The vocabulary is closed so tab-completion, JSON schemas, and cross-extension muscle memory stay stable. A capability that needs a novel verb proposes it as a protocol RFC. `transition` exists because state-machine steps recur across plan, change, and initiative.

Each op has a standard JSON-in / JSON-out contract; every capability's `list` has the same output shape, every `validate` has the same finding shape, every `scaffold` returns the same written-files summary. The concrete schema locations are implementation detail, but the shared result shapes are part of the protocol.

#### 3. Hooks (core-facing callbacks)

Invoked by core verbs on declared artefacts matching the hook's lifecycle:


| Hook                          | Invoked during                  | Default                                      | Capability responsibility         |
| ----------------------------- | ------------------------------- | ---------------------------------------- | --------------------------------- |
| `artifact-validate <id>`      | `specify change validate`       | none (capability MUST provide for `managed`) | format + brief rules on the delta |
| `artifact-preview-adopt <id>` | `specify change adopt preview`  | core default by `merge-strategy`         | produce structured preview        |
| `artifact-adopt <id>`         | `specify change adopt run`      | core default by `merge-strategy`         | produce merged baseline content   |
| `artifact-drop <id>`          | `specify change drop`           | no-op                                        | capability-side cleanup           |
| `baseline-validate <id>`      | capability `validate` operation     | none (capability MUST provide for `managed`) | project-wide conformance      |


Defaults for `three-way` and `opaque-replace` mean a pure-declarative YAML + markdown capability gets a working draft-build-adopt loop for free.

#### 4. Config (per-capability settings)

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

#### 5. Capability composition with the new surface


| Block            | Composition rule                                                                                                                                                       |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `artifacts:`     | Merge by `id`. Child entry with same `id` **replaces** the parent (no field-level merge; lifecycle swap would be too subtle to allow silently). Child MAY add new ids. |
| `operations:`    | Set-union of op names, child-wins on duplicates. Parent's unreferenced ops remain available.                                                                           |
| `plugin:`        | Child fully replaces parent. A plugin binary is a single artifact, not a composition. Child with no `plugin:` block inherits the parent's.                             |
| `config-schema:` | Layered via `allOf: [parent-schema, child-schema]`. Child can only tighten the parent's shape; broadening requires replacing.                                          |


Multi-level `extends:` chains and cycles are rejected at `specify capability check`.

#### 6. Cross-capability coexistence

Every project activates multiple capabilities — at minimum a domain capability plus `workflow@v1` + `registry@v1` + `initiative@v1`. Two constraints apply across the active set:

- **Artefact id uniqueness.** No two active capabilities may declare the same `artifact.id`.
- **Baseline-path uniqueness.** No two active capabilities may claim the same baseline path or project-path.

Capabilities may *consume* each other's baselines as read-only context (e.g. `client-sdk@v1` reads the `contracts@v1` baseline) by listing the consumed capability in a `consumes:` array.

A repository activates exactly one **domain** capability under this RFC. Multi-domain repositories are covered by [RFC-14](rfc-14-workspaces.md), which adds a Cargo-style `package:` / `workspace:` shape and makes the uniqueness rules scope-aware.

#### Cross-capability coordination

When an outcome spans capabilities, the runtime does not fuse their pipelines. Coordination is explicit and graph-shaped: `initiative@v1` records the outcome and close-out criteria, `registry@v1` identifies participating projects, the workspace coordinator resolves project/scope addresses, and `workflow@v1` owns the DAG of capability-addressed steps.

The workflow graph coordinates through the common protocol: nodes target capability-owned changes, validations, operations, or adoption checks; edges express ordering (`needs:`) and blocking conditions. The runner understands node kinds and protocol envelopes, not domain semantics. `consumes:` remains read-only coupling: one capability may read another capability's adopted baseline as context. A workflow may deliver code, but it may also deliver contracts, docs, infrastructure, fixtures, reports, or policy changes.

### First-party capabilities and bootstrap

`workflow@v1`, `initiative@v1`, `registry@v1` need to be available **before any capability URL has been resolved** — `specify init` must validate `registry.yaml`, and capability resolution itself runs through `specify capability *`, which is core. Resolution: first-party capabilities are **embedded in the CLI binary** and exposed through the same `capability.yaml` surface. The resolver checks the embedded set first, then falls back to URL resolution. They are still structurally capabilities — same blocks, same protocol, same linter rules.

`workflow@v1` has stronger status than an ordinary platform capability: it is the capability-shaped contract for Layer 2 workflow, backed by the core workflow runner. Its artefact format is declared like any other capability, but its execution semantics are part of the framework ABI. A `specify` upgrade that changes those semantics is therefore breaking. Projects pin via `specify_version` in `project.yaml`; embedded capability versions are not pinned independently. A project that opts out of a first-party capability sets `disable-first-party: [workflow]` — intentionally ugly, rarely used. Hub projects (`hub: true`) activate the three first-party capabilities without a domain capability; single-repo projects activate all four.

### Distribution: declarative with a subprocess escape


| Model                                                     | Reach                                                                                | Distribution            | Sandbox               | Verdict                                                                    |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------ | ----------------------- | --------------------- | -------------------------------------------------------------------------- |
| Pure declarative (YAML + markdown + named format adapter) | Artifact lifecycle, brief rendering, format validation the core vendors              | Capability repo only    | Total                 | **Default path.**                                                          |
| Subprocess plugin (`git-foo` convention)                  | Imperative ops needing host toolchain (`xcodebuild`, `cargo`, `gradle`, `terraform`) | PATH-installed binary   | None (operator privs) | **Escape hatch.**                                                          |
| WASM component (wasm32-wasip2)                            | Sandboxed imperative ops                                                             | Bundled in capability cache | Strong             | **Deferred.** Can't reach host toolchains without a host-function surface. |


Pure-declarative capabilities (YAML + markdown + a format adapter the core vendors — `markdown-spec`, `openapi`, `asyncapi`, `json-schema`) work end-to-end without extension code. For imperative operations, capabilities declare a subprocess plugin:

```yaml
plugin:
  binary: specify-ext-vectis         # resolved on PATH, git-foo convention
  protocol-version: 1
  ops: [doctor, scaffold, config]    # which ops route to the plugin
```

Ops not listed in `plugin.ops` fall back to declarative handling (or error). The plugin never calls back into the CLI; all state is passed on the command line or stdin. WASM-component plugins and in-process dynamic loading are out of scope; subprocess is chosen because it is language-agnostic, matches `git-foo` / `cargo-foo`, and keeps the trust boundary explicit.

#### Security posture

Subprocess plugins run with **the operator's full host privileges** — same as any other binary on PATH. The core does not sandbox them; this matches `git-foo` / `cargo-foo` and the existing trust relationship to capability source (the project already trusts its declared capability URL — same URL drives code generation, so running a plugin from the same upstream adds no new trust edge). A capability URL from an untrusted source must be vetted before `specify init`. A sandboxed write-fence and WASM-component plugins are candidates for a follow-up RFC.

#### Workspace-clone path resolution

Under `specify workspace sync`, every `artifacts.*.{baseline, project-path, delta}` resolves relative to **the clone's project root**, not the hub's.

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


The dispatcher maps plugin exits through `CliResult` so declarative and plugin-backed ops share the same top-level contract.

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

This RFC freezes the capability operation protocol, not the final operator-facing command spelling. The core must be able to resolve an active capability by name, reject unknown capabilities, expose local help from cached capability metadata, validate operation arguments, and invoke declarative or plugin-backed operations through the same result contract.

The CLI may expose that dispatcher as a dedicated prefix, as capability-scoped subcommands, as compatibility aliases for first-party capabilities, or as some combination of those. That choice is deliberately left to the implementation phase because it is product UX, not the immutable core boundary.

Capability-specific top-level families in today's CLI still leave the core. `plan`, `initiative`, `registry`, `contract`, and `vectis` become first-party capabilities with declared artefacts and operations; workflow close-out actions such as `archive` and `finalize` can still route through `specify change adopt` when they promote a capability-owned artefact.


## Alternatives Considered

- **Pure-declarative capabilities only.** Rejected as a hard rule because `vectis verify` needs host tools like `xcodebuild`; retained as the default path.
- **WASM-component plugins.** Deferred: sandboxed and aligned with Omnia, but cannot reach host toolchains without a large host-function surface.
- **In-process dynamic-library plugins.** Rejected because Rust ABI instability disqualifies them.
- **Freeze the final capability-operation CLI in this RFC.** Rejected because the architectural boundary only needs a dispatcher and protocol; command spelling is product UX and can be resolved during implementation.
- **Keep capability-specific top-level subcommands.** Rejected because the core surface would keep growing with every concern.
- **Multiple escape hatches.** Rejected because several plugin models would split the ecosystem.
- **Keep `artifacts:` lifecycle-only.** Rejected because artifacts are only one of four capability-specific surfaces hard-coded today.
- **A top-level `artifacts.yaml` next to `capability.yaml`.** Rejected because the extension surfaces are capability-bound, not project-bound.

## Non-Goals

- **Replacing or capability-configuring Layer 1 draft-build-adopt.** The loop's *shape* (phase set, transition DAG, per-phase outcome contract) is part of the immutable core. Capabilities declare what flows through the phases (artefacts, briefs, validators, operations, config) but never the phases themselves. Variability lives in (a) variable briefs per phase, (b) Layer 2 workflow graphs over capability-owned steps, and (c) the heavy-vs-light split. A capability that genuinely cannot fit any of those would justify proposing a *second* fixed loop shape as a peer to this one — never open-ended phase configuration.
- **Format-level contract evolution.** SemVer + `info.x-specify-id` + cross-repo uniqueness continue to be owned by RFC-12; this RFC only moves where the rules run from.
- **WASM / in-process plugins.** Subprocess is the only extension runtime in this RFC.
- **A general sandboxed write-fence.** Deferred until `specify check`'s write-path inventory is trustworthy enough to enforce.
- **Cardinality > 1 on delta or baseline.** One artefact → one delta → one baseline. Revisited only if a real capability needs more.
- **Cloud execution semantics.** Orthogonal; the subprocess protocol serialises the same either way.
- **Back-compat for capabilities without the new surface.** See §Migration — current usage footprint lets us cut over without a fallback path.
- **Third-party framework/platform capabilities.** `workflow@v1`, `registry@v1`, `initiative@v1` are first-party and bundled. Swapping them is a follow-up RFC; swapping `workflow@v1` is especially constrained because it defines Layer 2's graph contract.
- **Multiple domain capabilities per repository.** Covered by [RFC-14](rfc-14-workspaces.md), strictly additive on top of this RFC's four-block protocol.
- **Cross-capability changes in a single transaction.** Multi-capability outcomes are coordinated by workflow graphs, not by one change that writes multiple capabilities' baselines. RFC-14 applies the same rule to scopes: cross-scope work is a workflow with multiple entries, not a multi-scope change.

Multi-capability *per project* is in scope at the framework/platform level — `workflow` + `registry` + `initiative` always coexist with a domain capability (§Cross-capability coexistence). Multi-*domain*-capability per project is the RFC-14 layer.

## Implementation Scope

A staged landing, each stage independently testable and shippable. Every stage preserves working `/spec:draft → /spec:build → /spec:adopt` for the `omnia` capability (the only capability currently in real use).

### Phase 1 — Artifact declarations

Lands the lifecycle surface, widened to the four-value taxonomy.

1. New `artifacts:` fields parsed in `crates/schema/src/` (renamed to a capability crate in the cut-over) — `id`, `lifecycle`, the location-field set, `instance-path-template`, `merge-strategy`, `format`. JSON Schema additions in the capability manifest schema enforce the lifecycle ↔ location-field pairings from §"Location fields".
2. `crates/merge/` refactor: replace the hard-coded `specs_dir` + `contracts_dir` pair with iteration over the active capability's `managed` artifacts, dispatched on `merge-strategy`. Core ships `three-way` and `opaque-replace` defaults.
3. `crates/validate/`: add `--artifact <id>` filter; brief renderer learns the closed substitution vocabulary.
4. `src/config.rs`: drop `specs_dir` / `contracts_dir`; add `ProjectConfig::{baseline_path, delta_path, project_path}(&capability, artifact_id)`. An instance-resolving variant takes the brief context and applies `instance-path-template`.
5. Brief frontmatter parser learns `produces:` (single id or list). Brief loader binds each entry to an artefact in the active capability; unbound ids fail load with a diagnostic.
6. `specify check` (RFC-5) lints flag direct literal paths and the per-artefact invariants (verifier brief present for `managed`, no baseline / project-path collision, pipeline stays within declared artifacts, every authoring-required artefact appears in some brief's `produces:` list, every `instance-path-template` names exactly one variable).

First-party capabilities adopt `artifacts:` blocks declaring today's paths exactly — no filesystem changes.

### Phase 2 — Brief renderer + hook defaults

1. Generalise the brief renderer so capabilities can declare additional substitution variables.
2. Formalise the five lifecycle hooks. Wire `three-way` and `opaque-replace` as defaults for `artifact-preview-adopt` and `artifact-adopt`.
3. Move `validate_baseline_contracts` out of `crates/validate/src/` into a `format: openapi-asyncapi-bundle` adapter declared by the contracts capability. The core validate crate stops knowing the word "contract".

### Phase 3 — Operations surface

1. New capability operation dispatcher in `src/cli.rs` and `src/commands/`. Closed vocabulary, JSON-in / JSON-out per op.
2. Capability surface grows `operations:` and (optionally) a `plugin:` block.
3. `specify-ext-vectis` extracted from today's in-binary `specify_vectis` library; ships as its own binary alongside the `vectis` capability.

### Phase 4 — Extract framework/platform capabilities and retire capability-specific core surfaces

The largest phase: it proves the reframe.

1. **Extract `workflow@v1`, `registry@v1`, `initiative@v1` as first-party capabilities** embedded in the CLI via `include_str!` or a tidy `embedded-capabilities/` tree, exposed through the same resolver path as URL-resolved capabilities. `workflow@v1` is the Layer 2 framework capability; `registry@v1` and `initiative@v1` are platform capabilities.
2. **Cut their operator verbs over to the capability operation dispatcher**; `archive` and `finalize` route through `specify change adopt` with custom adopt hooks for close-out invariants.
3. **Delete `Commands::{Plan, Initiative, Registry, Vectis, Contract}`** from `src/cli.rs` and the matching modules under `src/commands/`.
4. **Retire `specify_vectis` as a library dependency** of `specify-cli` and publish `specify-ext-vectis` separately.
5. **Decide the workspace question** (§Open Questions). Either extract `workspace@v1` or document workspace as the deliberate exception.
6. **Retire surviving hard-coded `contracts` / `specs` references** in `crates/merge/`, `crates/validate/`, `src/config.rs`, `crates/change/`.
7. **First-party capabilities publish their full surface** — `omnia`, `contracts`, `vectis`, `workflow`, `registry`, `initiative` declare `artifacts:` + `operations:` + (where applicable) `plugin:` + `config-schema:` + `pipeline:`.
8. **Auto-activation at `specify init`.** A project's `project.yaml` declares its domain capability; the core auto-activates the Layer 2 workflow capability plus the two platform capabilities. Hubs activate the three without a domain capability.

Phase 4 is the largest slice and may land as a sequence of smaller commits.

### This repo (`augentic/specify`)

1. Add `capabilities/capability.schema.json` (or rename the existing manifest schema) to cover `artifacts:`, `operations:`, `plugin:`, `config-schema:`, `consumes:`.
2. Add `schemas/ops/<op>.schema.json` for each op in the closed vocabulary (`list`, `validate`, `inspect`, `doctor`, `scaffold`, `config`, `transition`).
3. Rewrite `capabilities/{contracts,omnia,vectis}/capability.yaml` to declare their full extension surface.
4. Port brief prose to `$ARTIFACT_DELTA[<id>]` / `$ARTIFACT_BASELINE[<id>]` substitutions.
5. Update `plugins/contract/` and `plugins/vectis/` skills to invoke capability operations through the new dispatcher.
6. **Phase 4 additions:** check in `capabilities/{workflow,registry,initiative}/capability.yaml` as the source-of-truth definitions for the embedded framework/platform capabilities. CLI consumes them at build time. Skills under `plugins/spec/skills/{plan,execute}/` re-route invocations to workflow capability operations.
7. Document the protocol in `docs/reference/capabilities.md`; cross-link from each capability's README. Add glossary entries for "active capability set," "Layer 1," "Layer 2," "workflow graph," "heavy mutation," "light mutation," and "first-party capability."

## Migration

Only the `omnia` capability and the core loop are in real-world use. `specify contract *`, `specify vectis *`, and the bulk of `specify plan|initiative|registry *` have no durable external user base to protect. The operator-facing CLI reshapes considerably in phase 4, but the behaviour behind each verb is preserved.

**Hard cut-over, no fallback path.** Each phase's minor version is a breaking change for the surfaces it touches. No deprecation window, no `artifacts:`-absent fallback, no aliasing of old CLI verbs. Pre-reframe capability manifests fail to load against the post-reframe CLI with a clear diagnostic pointing at this RFC and the capability rename.

The rename is part of that cut-over:

| Current term / surface                    | Post-RFC term / surface                 |
| ----------------------------------------- | --------------------------------------- |
| Schema (extension primitive)              | Capability                              |
| `schema.yaml`                             | `capability.yaml`                       |
| `project.yaml:schema`                     | `project.yaml:capability`               |
| `specify schema {resolve,check,pipeline}` | `specify capability {resolve,check,pipeline}` |
| `schemas/<name>/schema.yaml`              | `capabilities/<name>/capability.yaml`   |

JSON Schema remains JSON Schema. `config-schema:` and `*.schema.json` continue to name validation schemas, not Specify capabilities.

Four invariants guard the landing:

1. **Omnia keeps working.** Every phase's acceptance criterion includes running `/spec:draft → /spec:build → /spec:adopt` on a canonical omnia change end-to-end.
2. **The core never learns a capability name.** `specify check` rejects hard-coded capability-name literals in core crate sources outside tests, including first-party names after extraction.
3. **The core never learns an artefact id either.** A companion rule rejects hard-coded artefact-id literals such as `"specs"`, `"contracts"`, `"crates"`, and `"workflow"`; phase 1 retires the current canonical violations (`ProjectConfig::{specs_dir, contracts_dir}`).
4. **Framework/platform capabilities are still capabilities.** A rule verifies `workflow@v1`, `registry@v1`, `initiative@v1` each pass the same validation as any third-party capability — `capability.yaml` parses against the capability manifest JSON Schema, all declared briefs exist, `operations:` is a subset of the closed vocabulary, and so on.

Linter rules in `specify-check` (RFC-5) enforce, additionally:

- A capability's `operations:` MUST be a subset of the closed op vocabulary.
- A capability's `plugin.binary` MUST resolve on PATH or be declared absent.
- A capability's `config-schema:` MUST parse as a JSON Schema.
- **Active-capability-set invariants:** artefact-id, baseline-path, and project-path uniqueness across active capabilities.
- **First-party capability parity:** embedded capabilities pass every rule URL-resolved capabilities must pass.
- **Brief-binding completeness:** every artefact whose lifecycle requires authoring (`managed`, `external`, `audited`) appears in some brief's `produces:` list. `read-only` artefacts are exempt.
- **Path-substitution discipline:** brief prose references locations only via the closed substitution vocabulary; direct literal paths fail the lint.

## Open Questions

Genuinely open:

1. **Distribution model beyond subprocess.** When does WASM become worth adding? Provisional: revisit when the third capability asks, or when a hosting constraint forces sandboxing (RFC-7 cloud execution).
2. **Lifecycle naming.** `managed` / `external` / `read-only` / `audited` survived the earlier draft; confirm or replace (`specify-owned` / `tooling-owned` / `baselined` / `live`) one more time before phase 1.
3. **Workspace: capability or core exception?** `specify workspace` is mostly git-shelling and doesn't fit "draft-build-adopt a class of artefacts" cleanly. Provisional: stay core as the cross-capability coordinator, documented as the deliberate exception. Phase 4 locks the decision.
4. **Operator CLI spelling.** Dedicated prefix, capability-scoped subcommands, compatibility aliases, or a mix? Provisional: choose during implementation after testing discoverability and completion behavior.
5. **Heavy vs light mutation boundary.** Declared per-op in `capability.yaml`, or inferred by whether the op writes to a `managed` artefact? Provisional: declared per-op via a `mutation: heavy | light` field on `operations:` entries.
6. **Instance-variable resolution for multi-instance briefs.** Should the capability declare the binding source explicitly (e.g. `instance-source: artifact:specs.subdirs`), or remain a brief-side concern wired through skill code? Provisional: brief-side for now; revisit when a capability appears whose binding can't be expressed as a one-liner.

Resolved with provisional answer (see body for context):

- **Multiple capabilities per project / `capability:` shape.** Resolved by [RFC-14](rfc-14-workspaces.md): `package:` / `workspace:` shape, scope-aware uniqueness rules, back-compat shim for Mode-A repos, `disable-first-party:` survives.
- **Artifact taxonomy in phase 1.** Ship `managed`, `external`, `read-only`; reserve `audited` as a parse-time future-use error.
- **Operations vocabulary closed vs open.** Closed; novel ops require a protocol RFC.
- **Config location.** Nested `extensions.<name>` under `project.yaml`; revisit if extension count grows past a dozen.
- **Plugin resolution.** PATH-based (`specify-ext-<capability>`), matching `git-foo`. Capability-local complicates caching.
- **Default `artifact-validate`.** No core default — validation is where format semantics matter most and a silent default would mask missing capability work.
- **Exit-code propagation.** Map through `CliResult` so the top-level surface stays uniform; richer signal goes via JSON `error`.
- **Per-op help authoring.** Auto-derive from `op-args-schema` plus an optional `description` on each op entry in `describe`. Hand-authored long-form help is a future-RFC concern.
- **Format-adapter registry.** Fixed in-core registry to start; revisit when a third-party capability wants to ship its own.
- **First-party capability versioning.** Embedded capabilities track the CLI release as an ABI surface; projects pin via `specify_version` only.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) — owns the crates the reframe touches (`specify-schema`, `specify-merge`, `specify-validate`, `specify-change`) and the `src/cli.rs` dispatcher.
- [RFC-8: API contracts](archive/rfc-8-api-contracts.md) — `contracts@v1` capability; delta-then-promote semantics become the `opaque-replace` default.
- [RFC-2: Execution](archive/rfc-2-execution.md) — `/spec:execute --loop`; the workflow capability's `doctor` / `transition` / `inspect --next` ops inform this RFC's operation vocabulary.
- [RFC-3a: Monoliths](archive/rfc-3a-monoliths.md) — plan authoring pipeline; the existing two-brief `pipeline.plan` is the predecessor to `workflow@v1`.
- [RFC-3b: Platform](archive/rfc-3b-platform.md) — registry routing and workspace clones.
- [RFC-9: Platform](archive/rfc-9-platform.md) — moved registry, plan, initiative, and contracts to repo root; `/spec:plan --orchestrate` is the predecessor to workflow-driven orchestration.
- [RFC-12: Refine RFC-8](archive/rfc-12-refine-rfc-8.md) — SemVer + `info.x-specify-id` rules become `contracts`'s `baseline-validate` hook.
- [RFC-5: Framework Linter](rfc-5-lint.md) — home of the lints enforcing the reframe's invariants.
- [Roadmap](roadmap.md) — §3 motivates `read-only`; §5 / §6 / §7 are consumers of a stable core surface.
- `plugins/contract/references/baseline-vs-delta.md`, `docs/how-to/migrate-to-v2-layout.md` — path constants and v2 layout boundary the artifact declarations make per-artefact configurable.
