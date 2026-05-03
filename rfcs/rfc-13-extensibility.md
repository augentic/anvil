# RFC-13: Immutable core + schema extensions

> Status: Draft · Supersedes: earlier draft at this path (artifact-lifecycle-only framing) · Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-8](archive/rfc-8-api-contracts.md), [RFC-9](archive/rfc-9-platform.md), [RFC-12](archive/rfc-12-refine-rfc-8.md) · Enables: [RFC-14](rfc-14-workspaces.md)

## Abstract

A schema describes how to draft, build, and adopt a class of artefacts. RFC-13 reframes the runtime to match: the **immutable core** is the Layer 1 loop engine plus schema-agnostic scaffolding (init, migrate, schema resolver, change driver, Layer 2 workflow runner, status dispatcher, extension dispatcher), and **every other top-level verb in today's CLI — `plan`, `initiative`, `registry`, `contract`, `vectis`, most of `workspace` — becomes a schema**. The core never switches on a schema name.

Today's schema surface admits only `{ name, version, description, extends?, domain?, pipeline }`, which is too small to carry that contract. The RFC adds four declarative blocks (`artifacts:`, `operations:`, `plugin:`, `config-schema:`) so a schema can describe its artefacts, CLI verbs, lifecycle hooks, and project-level configuration. Schemas that need imperative code (e.g. `vectis verify` shelling out to `xcodebuild`) ship a subprocess plugin invoked through a tiny JSON protocol.

## Motivation

### The core isn't actually core

Specify's current surface promises extensibility and breaks it inside the binary:

- `specify-cli/src/cli.rs` carries `Vectis { action: VectisAction }` and `Contract { action: ContractAction }` as top-level subcommands, dispatched through `specify_vectis` and `specify::validate_baseline_contracts`. Schema-specific surfaces wearing a core coat.
- `crates/merge/src/change.rs` takes `specs_dir` and `contracts_dir` as first-class parameters, carries a `ContractPreviewEntry` type, and hard-codes "3-way for specs, opaque-replace for contracts" as the entire merge universe.
- `crates/validate/src/lib.rs` re-exports `validate_baseline_contracts` — a contracts-format validator has become part of the core's public API.
- `src/config.rs` exposes `ProjectConfig::specs_dir` and `contracts_dir` as fixed helpers.
- `schemas/schema.schema.json` admits only `{ name, version, description, extends?, domain?, pipeline }`. Nothing about artifacts, operations, validators, or config can be expressed.

Every new concern — infra, client SDKs, standards, codex rules, design tokens, fixtures — therefore requires a core patch.

### One primitive already works

`schema:` in `.specify/project.yaml` is URL-resolvable, with project-local caching under `.specify/.cache/` and inheritance via `extends`. Schemas are already remote, versioned, composable artefacts; the distribution mechanism does not have to be built.

### Artefact lifecycles, one runtime

A change under any current schema produces outputs governed by one of these lifecycles (§Design widens to four — `managed`, `external`, `read-only`, `audited`):

| Artifact                                                               | Build writes to                      | Baseline location      | Adopt promotes?              | Drop reverts?    |
| ---------------------------------------------------------------------- | ------------------------------------ | ---------------------- | ---------------------------- | ---------------- |
| Behavioral specs (declared by `omnia@v1`, `contracts@v1`, `vectis@v2`) | `.specify/changes/<name>/specs/`     | `.specify/specs/`      | Yes (file-level merge)       | Yes              |
| Contracts (`contracts@v1`)                                             | `.specify/changes/<name>/contracts/` | `<root>/contracts/`    | Yes (whole-file replacement) | Yes              |
| Crates (`omnia@v1`)                                                    | `<root>/crates/<crate>/`             | (no separate baseline) | No                           | No (git-managed) |
| Shared / iOS / Android / design-system (`vectis@v2`)                   | `<root>/<dir>/`                      | (no separate baseline) | No                           | No (git-managed) |

The first two rows are *managed* — Specify owns a versioned baseline; the bottom two are *external* — git provides versioning. The asymmetry is correct; today it is encoded in Rust rather than declared by the schema.

The strategy: **every artefact's write location(s) are declared by the schema in `artifacts:`, never inferred by the core and never invented by a brief.** Managed artefacts declare `delta:` + `baseline:`; external artefacts declare `project-path:`; read-only artefacts declare `baseline:`; audited artefacts declare `baseline:` plus a recorded checksum. Brief prose references each location through a closed substitution vocabulary (§"Substitution vocabulary"); literal paths are forbidden. Workspace clones resolve every declared location relative to the clone root (§"Workspace-clone path resolution").

`spec.md` is an artefact like any other. It appears today because behaviour drives the current generators; schemas whose primary deliverable is not behavioural declare their own artefact and skip `specs`.

### What the status quo blocks

- A future `infra@v1` cannot declare "the `terraform/` directory is a managed baseline" without patching `specify-cli`.
- A future `standards@v1` (roadmap §3) needs `read-only` baselines that sibling changes cite but never mutate; today the only lifecycles are `managed` and `external`.
- The format validators behind `specify contract validate` live in the core's public API, so a third-party schema cannot ship an equivalent without patching core.
- Schema-specific operator verbs (`vectis init`, `vectis verify`, `vectis add-shell`, `contract list`, `contract validate`) live in `src/cli.rs`, so adding a concern grows the core surface instead of an extension catalogue.

### What this enables

With `schema.yaml` owning artifacts, operations, hooks, and configuration, new concerns ship as schemas. None of these requires a core patch:

| Schema             | Artefact declaration                                                                                          | Headline ops                                                           |
| ------------------ | ------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `infra@v1`         | `terraform`, `lifecycle: managed`, `merge-strategy: opaque-replace`                                           | `list`, `validate` (`terraform validate`), `doctor`, `scaffold module` |
| `client-sdk@v1`    | `extends: contracts`; own artefact `clients`, `lifecycle: external`, `project-path: clients/`                 | `scaffold target --lang typescript`, `doctor`                          |
| `standards@v1`     | `codex`, `lifecycle: read-only`, `baseline: codex/`                                                           | `list`, `inspect <rule-id>`, `validate`                                |
| `design-tokens@v1` | Managed token source + external generated outputs (Swift / Kotlin / CSS)                                     | `doctor` = regenerate-and-diff                                         |
| `fixtures@v1`      | `fixtures`, `lifecycle: audited`, `baseline: fixtures/`                                                       | `list`, `inspect`, `scaffold capture`                                  |

None of these needs a `specs` artefact. Schemas that want behavioural specs declare one and stage a producing brief; schemas that do not simply omit it.

## Design

### Principle

**A schema describes how to draft-build-adopt a class of artefacts.** The core is the loop engine; schemas populate it with per-class choices (artefacts, validators, operator verbs, configuration). The core never switches on a schema name, never carries schema-specific type surfaces, and never ships schema-specific operator verbs. Imperative extension code is owned by the schema; the core invokes it through a fixed protocol.

"Without exception" is load-bearing. Today's `specify plan`, `specify initiative`, `specify registry`, `specify contract`, `specify vectis` top-level verbs are five schemas masquerading as core because the reframe hasn't landed; phase 4 extracts them. If a capability is schema-specific and has no place in `schema.yaml`, that is a gap in the protocol, not a licence for a new core verb.

The loop's *shape* is frozen alongside the verbs: the phase set (`draft` / `build` / `adopt` — today's `define` / `build` / `merge`), the legal transition DAG, and the per-phase outcome contract recorded in `.metadata.yaml` are part of the immutable core. Schemas declare what *flows through* the phases (artefacts, briefs, validators, operations, config) but never the phases themselves. Variation that schemas legitimately want lives in (a) variable briefs per phase, (b) workflow graphs over schema-owned steps, and (c) the heavy-vs-light mutation split (§Heavy vs light mutations). See §Non-Goals.

The coordinating principle is the dual: **schemas own artefact lifecycles; workflows coordinate schemas into outcomes.** Every mutable artefact has exactly one schema owner, every reviewed change runs through exactly one schema/scope, and cross-schema outcomes are achieved by a workflow graph, not by fusing schemas into a larger hidden schema. Outcomes are not necessarily code: they may be contracts, documentation, policy, infrastructure, fixtures, reports, generated clients, or any other schema-owned artefact. `workflow@v1` wires schema-owned steps through the common protocol.

That gives Specify two fixed framework layers:

- **Layer 1 — Change lifecycle.** One schema owns one artefact family; a reviewed change drafts, builds, validates, and adopts that schema's artefacts.
- **Layer 2 — Schema coordination.** A workflow coordinates Layer 1 changes, validations, schema-declared operations, and adoption checks into an outcome graph. The graph is stored in `workflow@v1`; the runner is core-owned and schema-agnostic.

### The immutable core boundary

The core is what's needed to run draft-build-adopt over any schema's artefacts — no more:

| Surface                                                                    | Owner             | What it does                                                                                                                                          |
| -------------------------------------------------------------------------- | ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify init` (+ `--hub`)                                                 | Core              | Bootstrap `.specify/`, resolve schema URL(s), cache briefs. Runs before any schema has loaded.                                                        |
| `specify migrate <migration>`                                              | Core              | One-shot layout migrations.                                                                                                                           |
| `specify schema *`                                                         | Core              | Resolve, check, pipeline.                                                                                                                             |
| `specify change *`                                                         | Core              | The draft-build-adopt loop engine: create, list, status, validate, adopt, drop, transition, archive, journal, outcome, touched-specs, overlap, task. |
| `specify status`                                                           | Core              | Cross-schema dispatcher.                                                                                                                              |
| `specify ext <schema> <op>`                                                | Dispatcher        | Schema-declared operator verbs (§Operations).                                                                                                         |
| Artefact-lifecycle bookkeeping                                             | Core, data-driven | Iterates over schema-declared artefacts.                                                                                                              |
| Layer 2 workflow runner                                                    | Core, data-driven | Walks `workflow@v1` graphs and invokes schema-owned steps through the common protocol.                                                                 |
| Format validators (OpenAPI, JSON Schema, spec-markdown, …)                 | Schema            | Declared as format adapters; core vendors generic ones, schemas may ship their own.                                                                   |
| Operator verbs for a concern (`verify`, `add-shell`, `list`, `inspect`, …) | Schema            | Declared in `operations:`.                                                                                                                            |
| Project-level config for a concern                                         | Schema            | Declared in `config-schema:`, stored under `extensions.<schema>` in `project.yaml`.                                                                   |

The left-hand column is frozen; new capabilities land on the right. The top six rows are the total core CLI surface — six verbs, against today's ten top-level families.

### What becomes a schema

Today's top-level verbs that aren't in the core table above are first-party schemas bundled with the CLI (§First-party schemas and bootstrap):

| Today                  | Becomes                                 | Artefact                                    | Notes                                                                                                                                                                                |
| ---------------------- | --------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `specify plan *`       | `workflow@v1` schema                    | `workflow.yaml`                             | First-party Layer 2 schema coordinator and successor to today's plan surface. Heavy mutations author the graph; light mutations use `scaffold` / `config` / `transition`.               |
| `specify initiative *` | `initiative@v1` schema                  | `initiative.md`                             | Tiny: one brief; `finalize` maps to `adopt` with a close-out hook that verifies every referenced workflow is terminal and every PR merged.                                            |
| `specify registry *`   | `registry@v1` schema                    | `registry.yaml`                             | Heavy mutations go through `specify change`; routine `add`/`remove` use `scaffold` / `config`. The `description-missing-multi-repo` invariant becomes a `baseline-validate` finding. |
| `specify contract *`   | `contracts@v1` schema                   | `contracts/` baseline                       | RFC-12's SemVer + `info.x-specify-id` checks become `baseline-validate`.                                                                                                             |
| `specify vectis *`     | `vectis@v2` schema                      | Shared / iOS / Android / design-system dirs | `verify` → `doctor`; `init` / `add-shell` → `scaffold`; `versions` → `config`.                                                                                                       |
| `specify workspace *`  | **Open question** (see §Open Questions) | Clones directory                            | Mostly git-shelling. May stay core as the cross-schema coordinator, or become a subprocess-plugin-heavy schema.                                                                      |

Every project activates at minimum `workflow@v1` + `initiative@v1` + `registry@v1` alongside its domain schema. `workflow@v1` is framework-level Layer 2, while `initiative@v1` and `registry@v1` remain platform schemas. All three declare structured documents (`workflow.yaml`, `initiative.md`, `registry.yaml`) as their primary artefacts, not `spec.md`.

### Heavy vs light mutations

Not every schema mutation runs the full change loop. Today's `specify registry add` writes a single line; forcing it through a change directory would be absurd.

- **Heavy (reviewed) mutations** — `specify change create → /spec:draft → /spec:build → /spec:adopt`, driven by the schema's pipeline. Used when the mutation needs briefs, review, overlap detection, conflict-check, and journaling. Example: `/spec:plan` authoring a workflow graph; `specify contract build` emitting a new `openapi.yaml`.
- **Light (direct) mutations** — `specify ext <schema> scaffold|config|transition` without a change directory. Example: `specify ext workflow scaffold entry`; `specify ext vectis config set versions.rust 1.82.0`.

Schemas decide which path each verb takes by routing it through `operations:` (light) or `pipeline:` (heavy). The core enforces one invariant: the same artefact cannot be written by both paths within a single change.

### The four-part protocol

The schema surface gains four new top-level blocks, each a flat vocabulary the core knows and the schema populates.

#### 1. Artifacts (declarative lifecycle)

Every output location a schema owns is declared once, with an explicit lifecycle. The canonical example covers all three relevant patterns:

```yaml
# omnia@v1 — Rust + WASM services. Specs drive code generation.
artifacts:
  - id: specs
    lifecycle: managed                  # managed | external | read-only | audited
    delta: specs/
    baseline: .specify/specs/
    merge-strategy: three-way           # three-way | opaque-replace | none
    format: markdown-spec               # core or schema-declared format adapter
  - id: crates
    lifecycle: external
    project-path: crates/
    instance-path-template: <crate-name>/
  - id: codex
    lifecycle: read-only
    baseline: codex/
```

`specs` carries no privilege — the linter sorts by `id` and the renderer iterates declared order, but no core code path keys off "the first artefact" or off the literal `id: specs`. Format adapters are named after the format (`markdown-spec`, `terraform-module`, `workflow-yaml`, `openapi-asyncapi-bundle`), not after artefact roles. A schema like `infra@v1` declares `format: terraform-module` and never lists a `specs` entry; `workflow@v1`'s sole artefact is `id: workflow, format: workflow-yaml`.

Lifecycles:

- `managed` — Specify-owned baseline; build writes to `$CHANGE_DIR/<delta>/`; adopt promotes via `merge-strategy`; drop discards the delta; sibling changes read the baseline as conformance context.
- `external` — downstream toolchain owns the artifact; build writes directly; git provides versioning; no adopt, no drop, no conflict-check.
- `read-only` — Specify-owned baseline that no change mutates; cited by generators and reviewers (roadmap §3 codex).
- `audited` — direct-write baseline with a checksum recorded in the change; adopt bumps the checksum, drop reverts. Implementation deferred but reserved.

`merge-strategy` and `format` are explicit fields rather than implied by id. The core ships generic implementations for `three-way` (today's spec merge) and `opaque-replace` (today's contract merge) so pure-declarative schemas work without extension code.

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

Briefs declare which artefact id(s) they produce via a `produces:` frontmatter field, the symmetric counterpart to `needs:` and `tracks:`. The field binds the brief to one or more ids in the active schema's `artifacts:` block, which is what lets substitutions resolve at render time and what lets `specify check` verify that every declared `managed` artefact has a producing brief in `pipeline.build`:

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

#### 2. Operations (operator CLI surface)

A **closed vocabulary** of operator verbs. Schemas pick which they implement; the core dispatches on `specify ext <schema> <op>`.

| Op                       | Meaning                                                       | Today's equivalent                                                                                                         |
| ------------------------ | ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `list`                   | Enumerate baseline artefact instances                         | `specify contract list`, `specify plan status`                                                                             |
| `validate`               | Run the schema's baseline-wide conformance checks             | `specify contract validate`, `specify plan validate`, `specify registry validate`                                          |
| `inspect <id>`           | Structured projection of one instance (or `--next`, `--show`) | `specify plan next`, `specify initiative show`, `specify registry show`                                                    |
| `doctor`                 | Full diagnostic / "does it still build / satisfy invariants"  | `specify vectis verify`, `specify plan doctor`                                                                             |
| `scaffold <kind>`        | One-shot generator                                            | `specify plan add`, `specify registry add`, `specify vectis init`, `specify vectis add-shell`, `specify initiative create` |
| `config` (get/set/show)  | Read/write the schema's `extensions.<schema>` block           | `specify vectis update-versions`, `specify vectis versions`                                                                |
| `transition <target>`    | State-machine step on an existing instance                    | `specify plan transition`, `specify plan lock`                                                                             |

The vocabulary is closed so tab-completion, JSON schemas, and cross-extension muscle memory stay stable. A schema that needs a novel verb proposes it as a protocol RFC. `transition` exists because state-machine steps recur across plan, change, and initiative.

Each op has a standard JSON-in / JSON-out contract; every schema's `list` has the same output shape, every `validate` has the same finding shape, every `scaffold` returns the same written-files summary. Shapes live in `schemas/ops/<op>.schema.json`.

#### 3. Hooks (core-facing callbacks)

Invoked by core verbs on declared artefacts matching the hook's lifecycle:

| Hook                          | Invoked during                  | Default                                  | Schema responsibility             |
| ----------------------------- | ------------------------------- | ---------------------------------------- | --------------------------------- |
| `artifact-validate <id>`      | `specify change validate`       | none (schema MUST provide for `managed`) | format + brief rules on the delta |
| `artifact-preview-adopt <id>` | `specify change adopt preview`  | core default by `merge-strategy`         | produce structured preview        |
| `artifact-adopt <id>`         | `specify change adopt run`      | core default by `merge-strategy`         | produce merged baseline content   |
| `artifact-drop <id>`          | `specify change drop`           | no-op                                    | schema-side cleanup               |
| `baseline-validate <id>`      | `specify ext <schema> validate` | none (schema MUST provide for `managed`) | project-wide conformance          |

Defaults for `three-way` and `opaque-replace` mean a pure-declarative YAML + markdown schema gets a working draft-build-adopt loop for free.

#### 4. Config (per-schema settings)

A per-extension block in `.specify/project.yaml`, validated against a schema-declared `config.schema.json`:

```yaml
# .specify/project.yaml
name: my-app
schema: https://github.com/augentic/specify/schemas/vectis@v2
domain: …
extensions:
  vectis:
    versions: { rust: 1.82.0, swift: 6.0 }
    shells: [ios, android]
  contracts:
    format-policy: strict-semver
```

Absent blocks use the schema's defaults. The core validates at `ProjectConfig::load` time; invalid config fails loud. Single-file config (nested in `project.yaml`) is chosen over a sibling `schema-config.yaml` because the extension count is small and operator friction is the active concern.

#### 5. Schema composition with the new surface

| Block            | Composition rule                                                                                                                                                       |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `artifacts:`     | Merge by `id`. Child entry with same `id` **replaces** the parent (no field-level merge; lifecycle swap would be too subtle to allow silently). Child MAY add new ids. |
| `operations:`    | Set-union of op names, child-wins on duplicates. Parent's unreferenced ops remain available.                                                                           |
| `plugin:`        | Child fully replaces parent. A plugin binary is a single artifact, not a composition. Child with no `plugin:` block inherits the parent's.                             |
| `config-schema:` | Layered via `allOf: [parent-schema, child-schema]`. Child can only tighten the parent's shape; broadening requires replacing.                                          |

Multi-level `extends:` chains and cycles are rejected at `specify schema check`.

#### 6. Cross-schema coexistence

Every project activates multiple schemas — at minimum a domain schema plus `workflow@v1` + `registry@v1` + `initiative@v1`. Two constraints apply across the active set:

- **Artefact id uniqueness.** No two active schemas may declare the same `artifact.id`.
- **Baseline-path uniqueness.** No two active schemas may claim the same baseline path or project-path.

Schemas may *consume* each other's baselines as read-only context (e.g. `client-sdk@v1` reads the `contracts@v1` baseline) by listing the consumed schema in a `consumes:` array.

A repository activates exactly one **domain** schema under this RFC. Multi-domain repositories are covered by [RFC-14](rfc-14-workspaces.md), which adds a Cargo-style `package:` / `workspace:` shape and makes the uniqueness rules scope-aware.

#### Cross-schema coordination

When an outcome spans schemas, the runtime does not fuse their pipelines. Coordination is explicit and graph-shaped: `initiative@v1` records the outcome and close-out criteria, `registry@v1` identifies participating projects, the workspace coordinator resolves project/scope addresses, and `workflow@v1` owns the DAG of schema-addressed steps.

The workflow graph coordinates through the common protocol: nodes target schema-owned changes, validations, operations, or adoption checks; edges express ordering (`needs:`) and blocking conditions. The runner understands node kinds and protocol envelopes, not domain semantics. `consumes:` remains read-only coupling: one schema may read another schema's adopted baseline as context. A workflow may deliver code, but it may also deliver contracts, docs, infrastructure, fixtures, reports, or policy changes.

### First-party schemas and bootstrap

`workflow@v1`, `initiative@v1`, `registry@v1` need to be available **before any schema URL has been resolved** — `specify init` must validate `registry.yaml`, and schema resolution itself runs through `specify schema *`, which is core. Resolution: first-party schemas are **embedded in the CLI binary** and exposed through the same `schema.yaml` surface. The resolver checks the embedded set first, then falls back to URL resolution. They are still structurally schemas — same blocks, same protocol, same linter rules.

`workflow@v1` has stronger status than an ordinary platform schema: it is the schema-shaped contract for Layer 2, backed by the core workflow runner. Its artefact format is declared like any other schema, but its execution semantics are part of the framework ABI. A `specify` upgrade that changes those semantics is therefore breaking. Projects pin via `specify_version` in `project.yaml`; embedded schema versions are not pinned independently. A project that opts out of a first-party schema sets `disable-first-party: [workflow]` — intentionally ugly, rarely used. Hub projects (`hub: true`) activate the three first-party schemas without a domain schema; single-repo projects activate all four.

### Distribution: declarative with a subprocess escape

| Model                                                     | Reach                                                                               | Distribution            | Sandbox               | Verdict                                                                    |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------- | ----------------------- | --------------------- | -------------------------------------------------------------------------- |
| Pure declarative (YAML + markdown + named format adapter) | Artifact lifecycle, brief rendering, format validation the core vendors             | Schema repo only        | Total                 | **Default path.**                                                          |
| Subprocess plugin (`git-foo` convention)                  | Imperative ops needing host toolchain (`xcodebuild`, `cargo`, `gradle`, `terraform`) | PATH-installed binary   | None (operator privs) | **Escape hatch.**                                                          |
| WASM component (wasm32-wasip2)                            | Sandboxed imperative ops                                                            | Bundled in schema cache | Strong                | **Deferred.** Can't reach host toolchains without a host-function surface. |

Pure-declarative schemas (YAML + markdown + a format adapter the core vendors — `markdown-spec`, `openapi`, `asyncapi`, `json-schema`) work end-to-end without extension code. For imperative operations, schemas declare a subprocess plugin:

```yaml
plugin:
  binary: specify-ext-vectis         # resolved on PATH, git-foo convention
  protocol-version: 1
  ops: [doctor, scaffold, config]    # which ops route to the plugin
```

Ops not listed in `plugin.ops` fall back to declarative handling (or error). The plugin never calls back into the CLI; all state is passed on the command line or stdin. WASM-component plugins and in-process dynamic loading are out of scope; subprocess is chosen because it is language-agnostic, matches `git-foo` / `cargo-foo`, and keeps the trust boundary explicit.

#### Security posture

Subprocess plugins run with **the operator's full host privileges** — same as any other binary on PATH. The core does not sandbox them; this matches `git-foo` / `cargo-foo` and the existing trust relationship to schema source (the project already trusts its declared schema URL — same URL drives code generation, so running a plugin from the same upstream adds no new trust edge). A schema URL from an untrusted source must be vetted before `specify init`. A sandboxed write-fence and WASM-component plugins are candidates for a follow-up RFC.

#### Workspace-clone path resolution

Under `specify workspace sync`, every `artifacts.*.{baseline, project-path, delta}` resolves relative to **the clone's project root**, not the hub's.

### Protocol contract

The subprocess protocol has four moving parts: invocation envelope, args envelope on stdin, result envelope on stdout, and fixed exit-code mapping.

#### Invocation

```text
specify-ext-<schema> \
  --op <op> \
  --project-dir <abs-path> \
  --schema-cache <abs-path> \
  --protocol-version 1 \
  --format json
  < <stdin: args-json>
  > <stdout: result-json>
```

Flags are positional-free. The core chooses `--protocol-version` from the schema declaration and errors before invocation if unsupported.

#### Args envelope (stdin)

```jsonc
{
  "op": "scaffold",
  "op-args": { "kind": "shell", "target": "ios" },
  "config": { /* resolved extensions.<schema> block */ },
  "schema-name": "vectis",
  "schema-version": 2
}
```

`op-args` is validated against `schemas/ops/<op>.schema.json` before invocation.

#### Result envelope (stdout)

Plugins return either `{ "result": …, "written-paths": […], "warnings": […] }` or `{ "error": <code>, "message": <text>, "context": … }`, plus `schema-version` and `op`. All keys are kebab-case. `result` payloads and `error` variants are validated against `schemas/ops/<op>.schema.json`.

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
specify-ext-<schema> --op describe --protocol-version 1 < {}
```

It returns supported protocol versions, implemented ops, each op's args schema, and the plugin version. The core caches the response and uses it for help, op-arg validation, and protocol mismatch detection.

#### Protocol versioning

- Each core release declares a set of supported protocol versions (initially `[1]`).
- Each plugin declares `plugin.protocol-version` in `schema.yaml` and `protocol-versions-supported` in `describe`.
- A mismatch fails `specify ext <schema> …` with `protocol-version-unsupported` before any op runs.
- New protocol versions add to the core's set; previous versions are deprecated in release notes and retired two minor versions later.

### CLI surface

Post-reframe, the full top-level CLI is six verbs:

```text
# Immutable core — bootstrap, schema resolver, loop engine, dispatchers
specify init      [ --hub ]
specify migrate   <migration>
specify schema    <action>            # resolve, check, pipeline
specify change    <action>            # create, list, status, validate, adopt, drop, transition, archive, journal, outcome, touched-specs, overlap, task
specify status                        # cross-schema dashboard
specify ext       <schema> <op>       # schema-declared operator verbs

# Shell completions — build-time generated
specify completions <shell>
```

Everything under today's `specify plan|initiative|registry|workspace|contract|vectis` — and any future schema's verbs — routes through `specify ext`. `<schema>` is the schema's `name` field, resolved against the project's active schema set (domain schema from `project.yaml:schema` plus auto-activated framework/platform schemas, plus any peer schemas declared by `registry.yaml`); the dispatcher refuses unknown schemas. Short forms (`specify <schema> <op>`) were rejected (§Alternatives) — extensions become indistinguishable from core verbs and tab-completion shifts project-to-project.

#### Discoverability

The dispatcher exposes three layers of help without reading schema source:

- `specify ext` — lists every active schema and its declared ops. Driven by cached `describe`.
- `specify ext <schema>` — lists ops with a one-line synopsis from `describe`.
- `specify ext <schema> <op> --help` — renders `op-args-schema` as a flag list, plus the standard result shape from `schemas/ops/<op>.schema.json`.

Help is always local — no plugin invocation, no network.

#### Concrete cut-overs

| Today                                         | Becomes                                                                 |
| --------------------------------------------- | ----------------------------------------------------------------------- |
| `specify contract list`                       | `specify ext contracts list`                                            |
| `specify contract validate`                   | `specify ext contracts validate`                                        |
| `specify vectis init`                         | `specify ext vectis scaffold project`                                   |
| `specify vectis verify`                       | `specify ext vectis doctor`                                             |
| `specify vectis add-shell`                    | `specify ext vectis scaffold shell --target ios`                        |
| `specify vectis update-versions`              | `specify ext vectis config set versions.rust 1.82.0`                    |
| `specify vectis versions`                     | `specify ext vectis config show`                                        |
| `specify plan create`                         | `specify ext workflow scaffold workflow` (or heavy change via workflow pipeline) |
| `specify plan add`                            | `specify ext workflow scaffold entry`                                   |
| `specify plan amend`                          | `specify ext workflow scaffold entry --amend`                           |
| `specify plan validate`                       | `specify ext workflow validate`                                         |
| `specify plan doctor`                         | `specify ext workflow doctor`                                           |
| `specify plan status`                         | `specify ext workflow list`                                             |
| `specify plan next`                           | `specify ext workflow inspect --next`                                   |
| `specify plan transition <name> <target>`     | `specify ext workflow transition <name> <target>`                       |
| `specify plan archive`                        | `specify change adopt` on the workflow artefact                         |
| `specify plan lock {acquire,release,status}`  | `specify ext workflow config set lock.holder <pid>` / `config show`     |
| `specify registry add`                        | `specify ext registry scaffold project`                                 |
| `specify registry remove`                     | `specify ext registry scaffold project --remove`                        |
| `specify registry show`                       | `specify ext registry inspect`                                          |
| `specify registry validate`                   | `specify ext registry validate`                                         |
| `specify initiative create`                   | `specify ext initiative scaffold`                                       |
| `specify initiative show`                     | `specify ext initiative inspect`                                        |
| `specify initiative finalize`                 | `specify change adopt` on the initiative artefact (custom adopt hook)   |
| `specify workspace *`                         | Pending §Open Questions — schema or core coordinator                    |

## Alternatives Considered

- **Pure-declarative schemas only.** Rejected as a hard rule because `vectis verify` needs host tools like `xcodebuild`; retained as the default path.
- **WASM-component plugins.** Deferred: sandboxed and aligned with Omnia, but cannot reach host toolchains without a large host-function surface.
- **In-process dynamic-library plugins.** Rejected because Rust ABI instability disqualifies them.
- **Short-form CLI (`specify <schema> <op>`).** Rejected because extensions become indistinguishable from core verbs and tab-completion shifts project-to-project.
- **Keep schema-specific top-level subcommands.** Rejected because the core surface would keep growing with every concern.
- **Multiple escape hatches.** Rejected because several plugin models would split the ecosystem.
- **Keep `artifacts:` lifecycle-only.** Rejected because artifacts are only one of four schema-specific surfaces hard-coded today.
- **A top-level `artifacts.yaml` next to `schema.yaml`.** Rejected because the extension surfaces are schema-bound, not project-bound.

## Non-Goals

- **Replacing or schema-configuring the draft / build / adopt phase model.** The loop's *shape* (phase set, transition DAG, per-phase outcome contract) is part of the immutable core. Schemas declare what flows through the phases (artefacts, briefs, validators, operations, config) but never the phases themselves. Variability lives in (a) variable briefs per phase, (b) workflow graphs over schema-owned steps, and (c) the heavy-vs-light split. A schema that genuinely cannot fit any of those would justify proposing a *second* fixed loop shape as a peer to this one — never open-ended phase configuration.
- **Format-level contract evolution.** SemVer + `info.x-specify-id` + cross-repo uniqueness continue to be owned by RFC-12; this RFC only moves where the rules run from.
- **WASM / in-process plugins.** Subprocess is the only extension runtime in this RFC.
- **A general sandboxed write-fence.** Deferred until `specify check`'s write-path inventory is trustworthy enough to enforce.
- **Cardinality > 1 on delta or baseline.** One artefact → one delta → one baseline. Revisited only if a real schema needs more.
- **Cloud execution semantics.** Orthogonal; the subprocess protocol serialises the same either way.
- **Back-compat for schemas without the new surface.** See §Migration — current usage footprint lets us cut over without a fallback path.
- **Third-party framework/platform schemas.** `workflow@v1`, `registry@v1`, `initiative@v1` are first-party and bundled. Swapping them is a follow-up RFC; swapping `workflow@v1` is especially constrained because it defines Layer 2's graph contract.
- **Multiple domain schemas per repository.** Covered by [RFC-14](rfc-14-workspaces.md), strictly additive on top of this RFC's four-block protocol.
- **Cross-schema changes in a single transaction.** Multi-schema outcomes are coordinated by workflow graphs, not by one change that writes multiple schemas' baselines. RFC-14 applies the same rule to scopes: cross-scope work is a workflow with multiple entries, not a multi-scope change.

Multi-schema *per project* is in scope at the framework/platform level — `workflow` + `registry` + `initiative` always coexist with a domain schema (§Cross-schema coexistence). Multi-*domain*-schema per project is the RFC-14 layer.

## Implementation Scope

A staged landing, each stage independently testable and shippable. Every stage preserves working `/spec:draft → /spec:build → /spec:adopt` for the `omnia` schema (the only schema currently in real use).

### Phase 1 — Artifact declarations

Lands the lifecycle surface, widened to the four-value taxonomy.

1. New `artifacts:` fields parsed in `crates/schema/src/` — `id`, `lifecycle`, the location-field set, `instance-path-template`, `merge-strategy`, `format`. JSON Schema additions in `schemas/schema.schema.json` enforce the lifecycle ↔ location-field pairings from §"Location fields".
2. `crates/merge/` refactor: replace the hard-coded `specs_dir` + `contracts_dir` pair with iteration over the active schema's `managed` artifacts, dispatched on `merge-strategy`. Core ships `three-way` and `opaque-replace` defaults.
3. `crates/validate/`: add `--artifact <id>` filter; brief renderer learns the closed substitution vocabulary.
4. `src/config.rs`: drop `specs_dir` / `contracts_dir`; add `ProjectConfig::{baseline_path, delta_path, project_path}(&schema, artifact_id)`. An instance-resolving variant takes the brief context and applies `instance-path-template`.
5. Brief frontmatter parser learns `produces:` (single id or list). Brief loader binds each entry to an artefact in the active schema; unbound ids fail load with a diagnostic.
6. `specify check` (RFC-5) lints flag direct literal paths and the per-artefact invariants (verifier brief present for `managed`, no baseline / project-path collision, pipeline stays within declared artifacts, every authoring-required artefact appears in some brief's `produces:` list, every `instance-path-template` names exactly one variable).

First-party schemas adopt `artifacts:` blocks declaring today's paths exactly — no filesystem changes.

### Phase 2 — Brief renderer + hook defaults

1. Generalise the brief renderer so schemas can declare additional substitution variables.
2. Formalise the five lifecycle hooks. Wire `three-way` and `opaque-replace` as defaults for `artifact-preview-adopt` and `artifact-adopt`.
3. Move `validate_baseline_contracts` out of `crates/validate/src/` into a `format: openapi-asyncapi-bundle` adapter declared by `schemas/contracts/schema.yaml`. The core validate crate stops knowing the word "contract".

### Phase 3 — Operations surface

1. New `specify ext <schema> <op>` dispatcher in `src/cli.rs` and `src/commands/`. Closed vocabulary, JSON-in / JSON-out per op (shapes under `schemas/ops/`).
2. Schema surface grows `operations:` and (optionally) a `plugin:` block.
3. `specify-ext-vectis` extracted from today's in-binary `specify_vectis` library; ships as its own binary alongside the `vectis` schema.

### Phase 4 — Extract framework/platform schemas and retire schema-specific core surfaces

The largest phase: it proves the reframe.

1. **Extract `workflow@v1`, `registry@v1`, `initiative@v1` as first-party schemas** embedded in the CLI via `include_str!` or a tidy `embedded-schemas/` tree, exposed through the same resolver path as URL-resolved schemas. `workflow@v1` is the Layer 2 framework schema; `registry@v1` and `initiative@v1` are platform schemas.
2. **Cut their operator verbs over to `specify ext <schema>`** per the §"Concrete cut-overs" table; `archive` and `finalize` route through `specify change adopt` with custom adopt hooks for close-out invariants.
3. **Delete `Commands::{Plan, Initiative, Registry, Vectis, Contract}`** from `src/cli.rs` and the matching modules under `src/commands/`.
4. **Retire `specify_vectis` as a library dependency** of `specify-cli` and publish `specify-ext-vectis` separately.
5. **Decide the workspace question** (§Open Questions). Either extract `workspace@v1` or document workspace as the deliberate exception.
6. **Retire surviving hard-coded `contracts` / `specs` references** in `crates/merge/`, `crates/validate/`, `src/config.rs`, `crates/change/`.
7. **First-party schemas publish their full surface** — `omnia`, `contracts`, `vectis`, `workflow`, `registry`, `initiative` declare `artifacts:` + `operations:` + (where applicable) `plugin:` + `config-schema:` + `pipeline:`.
8. **Auto-activation at `specify init`.** A project's `project.yaml` declares its domain schema; the core auto-activates the Layer 2 workflow schema plus the two platform schemas. Hubs activate the three without a domain schema.

Estimated total: ~3500–4500 lines of Rust + the extracted `specify-ext-vectis` binary (largely code-movement) + schema YAML in this repo. Phase 4 is ~60% of the total and may land as a sequence of smaller commits.

### This repo (`augentic/specify`)

1. Widen `schemas/schema.schema.json` to cover `artifacts:`, `operations:`, `plugin:`, `config-schema:`, `consumes:`.
2. Add `schemas/ops/<op>.schema.json` for each op in the closed vocabulary (`list`, `validate`, `inspect`, `doctor`, `scaffold`, `config`, `transition`).
3. Rewrite `schemas/{contracts,omnia,vectis}/schema.yaml` to declare their full extension surface.
4. Port brief prose to `$ARTIFACT_DELTA[<id>]` / `$ARTIFACT_BASELINE[<id>]` substitutions.
5. Update `plugins/contract/` and `plugins/vectis/` skills to invoke `specify ext <schema> …`.
6. **Phase 4 additions:** check in `schemas/{workflow,registry,initiative}/schema.yaml` as the source-of-truth definitions for the embedded framework/platform schemas. CLI consumes them at build time. Skills under `plugins/spec/skills/{plan,execute}/` re-route invocations to `specify ext workflow …`.
7. Document the protocol in `docs/reference/schema-extensions.md`; cross-link from each schema's README. Add glossary entries for "active schema set," "Layer 1," "Layer 2," "workflow graph," "heavy mutation," "light mutation," and "first-party schema."

## Migration

Only the `omnia` schema and the core loop are in real-world use. `specify contract *`, `specify vectis *`, and the bulk of `specify plan|initiative|registry *` have no durable external user base to protect. The operator-facing CLI reshapes considerably in phase 4, but the behaviour behind each verb is preserved.

**Hard cut-over, no fallback path.** Each phase's minor version is a breaking change for the surfaces it touches. No deprecation window, no `artifacts:`-absent fallback, no aliasing of old CLI verbs. Pre-reframe schemas fail to load against the post-reframe CLI with a clear diagnostic pointing at this RFC.

Four invariants guard the landing:

1. **Omnia keeps working.** Every phase's acceptance criterion includes running `/spec:draft → /spec:build → /spec:adopt` on a canonical omnia change end-to-end.
2. **The core never learns a schema name.** `specify check` rejects hard-coded schema-name literals in core crate sources outside tests, including first-party names after extraction.
3. **The core never learns an artefact id either.** A companion rule rejects hard-coded artefact-id literals such as `"specs"`, `"contracts"`, `"crates"`, and `"workflow"`; phase 1 retires the current canonical violations (`ProjectConfig::{specs_dir, contracts_dir}`).
4. **Framework/platform schemas are still schemas.** A rule verifies `workflow@v1`, `registry@v1`, `initiative@v1` each pass the same validation as any third-party schema — `schema.yaml` parses against `schema.schema.json`, all declared briefs exist, `operations:` is a subset of the closed vocabulary, and so on.

Linter rules in `specify-check` (RFC-5) enforce, additionally:

- A schema's `operations:` MUST be a subset of the closed op vocabulary.
- A schema's `plugin.binary` MUST resolve on PATH or be declared absent.
- A schema's `config-schema:` MUST parse as a JSON Schema.
- **Active-schema-set invariants:** artefact-id, baseline-path, and project-path uniqueness across active schemas.
- **First-party schema parity:** embedded schemas pass every rule URL-resolved schemas must pass.
- **Brief-binding completeness:** every artefact whose lifecycle requires authoring (`managed`, `external`, `audited`) appears in some brief's `produces:` list. `read-only` artefacts are exempt.
- **Path-substitution discipline:** brief prose references locations only via the closed substitution vocabulary; direct literal paths fail the lint.

## Open Questions

Genuinely open:

1. **Distribution model beyond subprocess.** When does WASM become worth adding? Provisional: revisit when the third schema asks, or when a hosting constraint forces sandboxing (RFC-7 cloud execution).
2. **Lifecycle naming.** `managed` / `external` / `read-only` / `audited` survived the earlier draft; confirm or replace (`specify-owned` / `tooling-owned` / `baselined` / `live`) one more time before phase 1.
3. **Workspace: schema or core exception?** `specify workspace` is mostly git-shelling and doesn't fit "draft-build-adopt a class of artefacts" cleanly. Provisional: stay core as the cross-schema coordinator, documented as the deliberate exception. Phase 4 locks the decision.
4. **Heavy vs light mutation boundary.** Declared per-op in `schema.yaml`, or inferred by whether the op writes to a `managed` artefact? Provisional: declared per-op via a `mutation: heavy | light` field on `operations:` entries.
5. **Instance-variable resolution for multi-instance briefs.** Should the schema declare the binding source explicitly (e.g. `instance-source: artifact:specs.subdirs`), or remain a brief-side concern wired through skill code? Provisional: brief-side for now; revisit when a schema appears whose binding can't be expressed as a one-liner.

Resolved with provisional answer (see body for context):

- **Multiple schemas per project / `schema:` vs `schemas:` shape.** Resolved by [RFC-14](rfc-14-workspaces.md): `package:` / `workspace:` shape, scope-aware uniqueness rules, back-compat shim for Mode-A repos, `disable-first-party:` survives.
- **Artifact taxonomy in phase 1.** Ship `managed`, `external`, `read-only`; reserve `audited` as a parse-time future-use error.
- **Operations vocabulary closed vs open.** Closed; novel ops require a protocol RFC.
- **CLI prefix.** `ext` (not `x`, not bare `<schema>`) — explicit, collision-free.
- **Config location.** Nested `extensions.<name>` under `project.yaml`; revisit if extension count grows past a dozen.
- **Plugin resolution.** PATH-based (`specify-ext-<schema>`), matching `git-foo`. Schema-local complicates caching.
- **Default `artifact-validate`.** No core default — validation is where format semantics matter most and a silent default would mask missing schema work.
- **Exit-code propagation.** Map through `CliResult` so the top-level surface stays uniform; richer signal goes via JSON `error`.
- **Per-op help authoring.** Auto-derive from `op-args-schema` plus an optional `description` on each op entry in `describe`. Hand-authored long-form help is a future-RFC concern.
- **Format-adapter registry.** Fixed in-core registry to start; revisit when a third-party schema wants to ship its own.
- **First-party schema versioning.** Embedded schemas track the CLI release as an ABI surface; projects pin via `specify_version` only.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) — owns the crates the reframe touches (`specify-schema`, `specify-merge`, `specify-validate`, `specify-change`) and the `src/cli.rs` dispatcher.
- [RFC-8: API contracts](archive/rfc-8-api-contracts.md) — `contracts@v1` schema; delta-then-promote semantics become the `opaque-replace` default.
- [RFC-2: Execution](archive/rfc-2-execution.md) — `/spec:execute --loop`; the workflow schema's `doctor` / `transition` / `inspect --next` ops are lifted from this RFC's CLI surface.
- [RFC-3a: Monoliths](archive/rfc-3a-monoliths.md) — plan authoring pipeline; the existing two-brief `pipeline.plan` is the predecessor to `workflow@v1`.
- [RFC-3b: Platform](archive/rfc-3b-platform.md) — registry routing and workspace clones.
- [RFC-9: Platform](archive/rfc-9-platform.md) — moved registry, plan, initiative, and contracts to repo root; `/spec:plan --orchestrate` is the predecessor to workflow-driven orchestration.
- [RFC-12: Refine RFC-8](archive/rfc-12-refine-rfc-8.md) — SemVer + `info.x-specify-id` rules become `contracts`'s `baseline-validate` hook.
- [RFC-5: Framework Linter](rfc-5-lint.md) — home of the lints enforcing the reframe's invariants.
- [Roadmap](roadmap.md) — §3 motivates `read-only`; §5 / §6 / §7 are consumers of a stable core surface.
- `plugins/contract/references/baseline-vs-delta.md`, `docs/how-to/migrate-to-v2-layout.md` — path constants and v2 layout boundary the artifact declarations make per-artefact configurable.
