# RFC-13: Immutable core + schema extensions

> Status: Draft · Supersedes: earlier draft at this path (artifact-lifecycle-only framing) · Depends: [RFC-1](archive/rfc-1-cli.md), [RFC-8](archive/rfc-8-api-contracts.md), [RFC-9](archive/rfc-9-platform.md), [RFC-12](archive/rfc-12-refine-rfc-8.md)

## Abstract

**A schema describes how to draft-build-adopt a class of artefacts.** That one sentence is the organising definition of the framework. Everything else — artefact lifecycle, CLI operations, validators, configuration — is an elaboration of it.

RFC-13 reframes Specify so the runtime matches the definition: the **immutable core** is the draft-build-adopt loop engine plus the minimum scaffolding to run it (init, migrate, schema resolver, change driver, status dispatcher, extension dispatcher), and **every other top-level verb in today's CLI — `plan`, `initiative`, `registry`, `contract`, `vectis`, most of `workspace` — is a schema**. 

The core never switches on a schema name; schemas are the only extension point, without exception. Today's schema surface is too small to carry this — it admits only `{ name, version, description, extends?, domain?, pipeline }` — so the RFC adds four declarative blocks (`artifacts:`, `operations:`, `plugin:`, `config-schema:`) that together let a schema describe its artefacts, CLI verbs, lifecycle hooks, and project-level configuration. Schemas that need imperative code (e.g. `vectis verify` shelling out to `xcodebuild`) ship a subprocess plugin the core invokes through a tiny JSON protocol.

The artefact-lifecycle work that was the original scope of this RFC lands as phase 1; extracting `plan`, `initiative`, `registry` and their peers into schemas lands in phase 4.

## Motivation

### The core isn't actually core

Specify's current surface makes the extensibility promise and then breaks it inside the binary:

- `specify-cli/src/cli.rs` carries `Vectis { action: VectisAction }` and `Contract { action: ContractAction }` as top-level subcommands, dispatched through the `specify_vectis` library crate and `specify::validate_baseline_contracts` respectively. Both are schema-specific surfaces wearing a core coat.
- `crates/merge/src/change.rs` takes `specs_dir` and `contracts_dir` as first-class parameters, carries a `ContractPreviewEntry` type, and hard-codes "3-way merge for specs, opaque replace for contracts" as the entire universe of merge strategies.
- `crates/validate/src/lib.rs` re-exports `validate_baseline_contracts` from the core type surface — a contracts-format validator has become part of the core's public API.
- `src/config.rs` exposes `ProjectConfig::specs_dir` and `contracts_dir` as fixed helpers; every subcommand that needs a baseline path reaches for one of those two names.
- `schemas/schema.schema.json` admits only the minimal `{ name, version, description, extends?, domain?, pipeline }` shape. Nothing about artifacts, operations, validators, or config can be expressed in a schema today.

The result is that every new concern — infra, client SDKs, standards, codex rules, design tokens, fixtures — requires a core patch. That is the opposite of the framework's stated direction.

### One extensibility primitive already works

One piece is in place and should be preserved: the `schema:` field in `.specify/project.yaml` is URL-resolvable, with project-local caching under `.specify/.cache/` and inheritance via `extends`. Schemas are already remote, versioned, composable artefacts. That is the distribution mechanism the reframe needs; it does not have to be built.

### Three lifecycles, one runtime (preserved from the earlier draft)

A change running under any current schema produces outputs governed by one of three lifecycles:


| Artifact                                                    | Build writes to                      | Baseline location      | Adopt promotes?              | Drop reverts?    |
| ----------------------------------------------------------- | ------------------------------------ | ---------------------- | ---------------------------- | ---------------- |
| Behavioral specs (declared by `omnia@v1`, `contracts@v1`, `vectis@v2`) | `.specify/changes/<name>/specs/`     | `.specify/specs/`      | Yes (file-level merge)       | Yes              |
| Contracts (`contracts@v1`)                                  | `.specify/changes/<name>/contracts/` | `<root>/contracts/`    | Yes (whole-file replacement) | Yes              |
| Crates (`omnia@v1`)                                         | `<root>/crates/<crate>/`             | (no separate baseline) | No                           | No (git-managed) |
| Shared / iOS / Android / design-system (`vectis@v2`)        | `<root>/<dir>/`                      | (no separate baseline) | No                           | No (git-managed) |


The first two rows are *managed*: Specify owns a versioned baseline, the change-local copy is a proposed delta, and adoption is a transactional promotion with file-granularity conflict detection. The bottom two rows are *external*: the project tree holds the only copy and git provides versioning, conflict detection, and rollback. The asymmetry is correct — the earlier draft explains why uniformity in either direction is the wrong answer — but it is currently encoded in Rust rather than declared by the schema.

The first row is worth lingering on: spec.md files are an artefact like any other. They show up across the three current first-party domain schemas because behavioral requirements happen to drive their generators, not because the core mandates them. Schemas whose primary deliverable isn't behavioral — `plan@v1`'s `plan.yaml`, `registry@v1`'s `registry.yaml`, `initiative@v1`'s `initiative.md`, plus the candidate `infra@v1` / `fixtures@v1` / `standards@v1` / `client-sdk@v1` / `design-tokens@v1` schemas in §What this enables — declare their own primary artefact and skip `specs` entirely. The reframe makes that choice the schema's, not a system fixture.

### What the status quo blocks

- A future `infra@v1` schema generating Terraform cannot declare "the `terraform/` directory is a managed baseline" without patching `specify-cli`.
- A future `standards@v1` schema (roadmap §3) wants `read-only` baselines that sibling changes cite but never mutate; today the only lifecycles on offer are `managed` and `external`.
- The format validators that make `specify contract validate` meaningful live in the core's public API, so a third-party schema cannot ship an equivalent check without patching core.
- Schema-specific operator verbs (`vectis init`, `vectis verify`, `vectis add-shell`, `contract list`, `contract validate`) live in `src/cli.rs`, so adding a new concern adds top-level subcommands and grows the core surface instead of an extension catalogue.

### What this enables

With `schema.yaml` owning artifacts, operations, hooks, and configuration, new concerns ship as schemas instead of core patches. Concrete candidates the reframe unblocks:

- `**infra@v1`** — Terraform / Pulumi plans. `artifacts: { id: terraform, lifecycle: managed, delta: terraform/, baseline: terraform/, merge-strategy: opaque-replace }`. Operations: `list` (resources), `validate` (`terraform validate`), `doctor` (plan diff), `scaffold module`.
- `**client-sdk@v1**` — generated HTTP clients per target language. `extends: contracts` so the generator reads the `contracts` baseline as conformance context; its own artifact is `{ id: clients, lifecycle: external, project-path: clients/ }`. Operations: `scaffold target --lang typescript`, `doctor` (regenerate-and-diff).
- `**standards@v1**` — the roadmap §3 codex. `artifacts: { id: codex, lifecycle: read-only, baseline: codex/ }`. Operations: `list` (rule ids), `inspect <rule-id>`, `validate` (rule text parses cleanly).
- `**design-tokens@v1**` — design system tokens with Swift / Kotlin / CSS generators. Managed token source + external generated outputs; `doctor` = regenerate-and-diff.
- `**fixtures@v1**` — RT replay fixtures. `artifacts: { id: fixtures, lifecycle: audited, baseline: fixtures/ }`. Operations: `list`, `inspect`, `scaffold capture`.

None of these requires a core patch — and every one of them is a concrete Augentic line-item rather than a hypothetical. The reframe's value is the ability to ship these by authoring YAML and (at most) a subprocess binary, not by teaching `specify-cli` about a new concern.

Worth calling out: **none of these five schemas declares a `specs` artefact.** Terraform modules describe themselves; codex rules are the artefact; client SDKs derive from the contracts baseline; design tokens are the source; replay fixtures are captured rather than spec-driven. The spec.md file is a useful artefact for behavior-driven schemas (omnia, vectis, contracts) and a misfit for everything else — and the reframe lets each schema make that call. A schema that wants behavioral specs lists them in `artifacts:` and stages a `specs` brief in `pipeline.define`; a schema that doesn't, omits both. The core doesn't notice the difference.

## Design

### Principle

**A schema describes how to draft-build-adopt a class of artefacts.** The core is the loop engine; schemas populate it with the per-class choices (what artefacts exist, how they're validated, which verbs the operator invokes, what configuration they accept). The core never switches on a schema name, never carries schema-specific type surfaces, and never ships schema-specific operator verbs. Every schema-specific capability is declared in `schema.yaml` and dispatched through a fixed protocol. Imperative extension code (e.g. `vectis verify` shelling out to `xcodebuild`) is owned by the schema — the core invokes it, but the schema declares where it lives.

"Without exception" is load-bearing. If a capability is schema-specific and has no place in `schema.yaml`, that is a gap in the protocol, not a licence for a new core verb. Corollary: today's `specify plan`, `specify initiative`, `specify registry`, `specify contract`, `specify vectis` top-level verbs are not core — they are five schemas masquerading as core because the reframe hasn't landed yet. Phase 4 extracts them.

### The immutable core boundary

The core is what's needed to run the draft-build-adopt loop over any schema's artefacts — no more:


| Surface                                                                    | Owner             | What it does                                                                                                                                                                                   |
| -------------------------------------------------------------------------- | ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify init` (+ `--hub`)                                                 | Core              | Bootstrap `.specify/`, resolve schema URL(s), cache briefs. Runs before any schema has loaded.                                                                                                 |
| `specify migrate `*                                                        | Core              | One-shot layout migrations. Bootstrap concern.                                                                                                                                                 |
| `specify schema *`                                                         | Core              | Resolve, check, pipeline. The schema resolver itself.                                                                                                                                          |
| `specify change *`                                                         | Core              | The draft-build-adopt loop engine. Every schema's artefacts go through this: create, list, status, validate, adopt, drop, transition, archive, journal, outcome, touched-specs, overlap, task. |
| `specify status`                                                           | Core              | Cross-schema dispatcher. For every active schema, summarise.                                                                                                                                   |
| `specify ext <schema> <op>`                                                | Dispatcher        | Schema-declared operator verbs (see §Operations).                                                                                                                                              |
| Artefact-lifecycle bookkeeping                                             | Core, data-driven | Iterates over schema-declared artefacts.                                                                                                                                                       |
| Format validators (OpenAPI, JSON Schema, spec-markdown, …)                 | Schema            | Declared as format adapters; core vendors generic ones, schemas may ship their own.                                                                                                            |
| Operator verbs for a concern (`verify`, `add-shell`, `list`, `inspect`, …) | Schema            | Declared in `operations:`.                                                                                                                                                                     |
| Project-level config for a concern                                         | Schema            | Declared in `config-schema:`, stored under `extensions.<schema>` in `project.yaml`.                                                                                                            |


The left-hand column is frozen. Any new capability in the right-hand column lands in a schema, never in core. The top six rows are the total core CLI surface; compare with today's ten top-level verb families.

### What becomes a schema

Today's top-level verbs that aren't in the core table above are schemas that haven't been extracted yet. Each is a first-party schema bundled with the CLI (see §First-party schemas and bootstrap):


| Today                  | Becomes                                 | Artefact                                    | Notes                                                                                                                                                                                                                                      |
| ---------------------- | --------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `specify plan *`       | `plan@v1` schema                        | `plan.yaml`                                 | Already has a `pipeline.plan` phase in domain schemas — this extracts it. Heavy mutations go through the plan pipeline; light mutations (`add`, `amend`, `transition`, `lock`) use the schema's `scaffold` / `config` / `transition` ops.  |
| `specify initiative *` | `initiative@v1` schema                  | `initiative.md`                             | Tiny schema: one brief, `finalize` maps to `adopt` with a custom close-out hook that verifies every plan entry is terminal and every PR merged.                                                                                            |
| `specify registry *`   | `registry@v1` schema                    | `registry.yaml`                             | Heavy mutations (multi-project re-shape) go through `specify change`; routine mutations (`add project`, `remove project`) use `scaffold` / `config`. The `description-missing-multi-repo` invariant becomes a `baseline-validate` finding. |
| `specify contract *`   | `contracts@v1` schema                   | `contracts/` baseline                       | Already agreed. RFC-12's SemVer + `info.x-specify-id` checks become the schema's `baseline-validate` hook.                                                                                                                                 |
| `specify vectis *`     | `vectis@v2` schema                      | Shared / iOS / Android / design-system dirs | Already agreed. `verify` → `doctor`; `init` / `add-shell` → `scaffold`; `versions` → `config`.                                                                                                                                             |
| `specify workspace *`  | **Open question** (see §Open Questions) | Clones directory                            | Mostly git-shelling, not clearly an "artefact lifecycle." May stay core as the cross-schema coordinator, or become a subprocess-plugin-heavy schema.                                                                                       |


Every project activates at minimum `plan@v1` + `initiative@v1` + `registry@v1` alongside its domain schema. See §First-party schemas and bootstrap for how this works without a chicken-and-egg problem.

The Artefact column is doing more work than a casual reader notices: the three first-party platform schemas (`plan@v1`, `initiative@v1`, `registry@v1`) declare structured documents — `plan.yaml`, `initiative.md`, `registry.yaml` — as their primary artefact, *not* spec.md. That is the principle of the reframe in action. These schemas pass through the same draft-build-adopt loop as omnia and vectis; they just describe a different class of artefact. Anyone tempted to assume "every schema starts with a spec.md" should look at this table — half of the post-reframe first-party schemas don't, and they are still recognisably Specify schemas.

### Heavy vs light mutations

Not every schema mutation runs the full change loop. Today's `specify registry add` writes a single line; forcing every operator to file a change directory for it would be absurd. The reframe keeps the two paths explicit and per-schema-declared:

- **Heavy (reviewed) mutations** — `specify change create → /spec:draft → /spec:build → /spec:adopt`, driven by the schema's pipeline. Used when the mutation needs briefs, review, overlap detection, conflict-check, and journaling. Example: `/spec:plan` authoring a new plan from scratch; `specify contract build` emitting a new `openapi.yaml` under a change.
- **Light (direct) mutations** — `specify ext <schema> scaffold/config/transition` without a change directory. Used for routine edits that don't merit a change. Example: `specify ext plan scaffold entry`; `specify ext registry scaffold project`; `specify ext vectis config set versions.rust 1.82.0`.

Schemas decide which of their verbs are heavy and which are light by choosing whether to route them through `operations:` (light) or to drive them from `pipeline:` (heavy). The core enforces one invariant: the same artefact cannot be written by both paths within a single change — light mutations outside a change, or the heavy change-driven path, never both at once.

### The four-part protocol

The schema surface gains four new top-level blocks, each a flat vocabulary the core knows and the schema populates.

#### 1. Artifacts (declarative lifecycle)

Every output location a schema owns is declared once, with an explicit lifecycle. The earlier RFC-13 draft's core insight, widened. To break the "specs always come first" reading from today's CLI, three schemas are shown side by side: a domain schema that does drive code from behavioral specs, a domain schema that owns Terraform modules and has no behavioral surface, and a first-party platform schema whose only artefact is a YAML index.

```yaml
# omnia@v1 — Rust + WASM services. Specs are present because behavior drives generation.
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
  - id: codex
    lifecycle: read-only                # sibling changes cite, nobody mutates
    baseline: codex/
```

```yaml
# infra@v1 — Terraform modules. Behavioral specs would add nothing the HCL doesn't say.
artifacts:
  - id: terraform
    lifecycle: managed
    delta: terraform/
    baseline: terraform/
    merge-strategy: opaque-replace
    format: terraform-module
```

```yaml
# plan@v1 — first-party platform schema. Its artefact is plan.yaml; no specs at all.
artifacts:
  - id: plan
    lifecycle: managed
    delta: plan.yaml
    baseline: .specify/plan.yaml
    merge-strategy: opaque-replace
    format: plan-yaml
```

Three observations from the trio:

- `specs` is one entry, not the entry. Two of the three example schemas declare zero `specs` artefact, including a first-party schema bundled with the CLI. The core never expects `specs` to be present.
- The first slot in `artifacts:` carries no privilege — the linter sorts by `id` for diagnostic stability and the renderer iterates the declared order, but no core code path keys off "the first artefact" or off the literal `id: specs`.
- Format adapters are named after the format (`markdown-spec`, `terraform-module`, `plan-yaml`, `openapi-asyncapi-bundle`), not after artefact roles. A schema that wants behavioral specs picks `format: markdown-spec`; a schema that doesn't, doesn't.

Lifecycles:

- `managed` — Specify-owned baseline; build writes to `$CHANGE_DIR/<delta>/`; adopt promotes via `merge-strategy`; drop discards the delta; sibling changes read the baseline as conformance context.
- `external` — downstream toolchain owns the artifact; build writes directly; git provides versioning; no adopt, no drop, no conflict-check.
- `read-only` — Specify-owned baseline that no change mutates; exists to be cited by generators and reviewers (roadmap §3 codex).
- `audited` — direct-write baseline with a checksum recorded in the change; adopt bumps the checksum, drop reverts it. Deferred implementation but reserved in the taxonomy.

`merge-strategy` and `format` are explicit fields rather than implied by artifact id. The core ships generic implementations for `three-way` (today's spec merge) and `opaque-replace` (today's contract merge) so pure-declarative schemas work without any extension code.

`$ARTIFACT_DELTA[<id>]` and `$ARTIFACT_BASELINE[<id>]` become the canonical substitutions in brief prose; direct literal paths (`.specify/changes/<name>/contracts/`) are forbidden and flagged by `specify check`. The substitutions take the artefact's declared id — `$ARTIFACT_DELTA[specs]` for omnia briefs, `$ARTIFACT_DELTA[terraform]` for infra briefs, `$ARTIFACT_DELTA[plan]` for the plan schema — so brief prose never has to know whether a particular schema treats specs as canonical.

#### 2. Operations (operator CLI surface)

A **closed vocabulary** of operator verbs. Schemas pick which ones they implement; the core dispatches on `specify ext <schema> <op>`.


| Op                      | Meaning                                                                | Today's equivalent                                                                                                         |
| ----------------------- | ---------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `list`                  | Enumerate baseline artefact instances                                  | `specify contract list`, `specify plan status`                                                                             |
| `validate`              | Run the schema's baseline-wide conformance checks                      | `specify contract validate`, `specify plan validate`, `specify registry validate`                                          |
| `inspect <id>`          | Structured projection of one artefact instance (or `--next`, `--show`) | `specify plan next`, `specify initiative show`, `specify registry show`                                                    |
| `doctor`                | Full diagnostic / "does it still build / satisfy its invariants"       | `specify vectis verify`, `specify plan doctor`                                                                             |
| `scaffold <kind>`       | One-shot generator                                                     | `specify plan add`, `specify registry add`, `specify vectis init`, `specify vectis add-shell`, `specify initiative create` |
| `config <get|set|show>` | Read/write schema-scoped config                                        | `specify vectis update-versions`, `specify vectis versions`                                                                |
| `transition <target>`   | State-machine step on an existing artefact instance                    | `specify plan transition`, `specify plan lock`                                                                             |


The vocabulary is closed so tab-completion, JSON schemas, and cross-extension muscle memory stay stable. A schema that genuinely needs an additional verb proposes it as an extension to the core protocol (an RFC), not as a one-off in its own namespace. `transition` is added specifically because state-machine steps recur across plan, change, and initiative schemas — a repeating pattern deserves its own verb rather than being squeezed into `scaffold`.

Each op has a standard JSON-in / JSON-out contract; every schema's `list` has the same output shape, every `validate` has the same finding shape, every `scaffold` returns the same written-files summary. The shape lives in `schemas/ops/<op>.schema.json` (in this repo, next to `schema.schema.json`).

#### 3. Hooks (core-facing callbacks)

These are invoked by core verbs, not by operators. Each hook fires on a declared artifact matching its lifecycle:


| Hook                          | Invoked during                  | Default                                  | Schema responsibility             |
| ----------------------------- | ------------------------------- | ---------------------------------------- | --------------------------------- |
| `artifact-validate <id>`      | `specify change validate`       | none (schema MUST provide for `managed`) | format + brief rules on the delta |
| `artifact-preview-adopt <id>` | `specify change adopt preview`  | core default by `merge-strategy`         | produce structured preview        |
| `artifact-adopt <id>`         | `specify change adopt run`      | core default by `merge-strategy`         | produce merged baseline content   |
| `artifact-drop <id>`          | `specify change drop`           | no-op                                    | schema-side cleanup               |
| `baseline-validate <id>`      | `specify ext <schema> validate` | none (schema MUST provide for `managed`) | project-wide conformance          |


The core default implementations for `three-way` and `opaque-replace` mean a schema that writes pure declarative YAML + markdown gets a working draft-build-adopt loop for free.

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

Absent `extensions.<name>` blocks use the schema's defaults. The core validates the block at `ProjectConfig::load` time against the schema's own JSON Schema; invalid config fails loud, never silently.

Single-file config (nested in `project.yaml`) is chosen over a sibling `schema-config.yaml` because the extension count is small and operator friction is the active concern.

#### 5. Schema composition with the new surface

The existing `extends:` field already composes pipelines by appending child phases after parent phases. The four new blocks compose as follows:


| Block            | Composition rule                                                                                                                                                             |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `artifacts:`     | Merge by `id`. Child entry with same `id` **replaces** the parent entry (no field-level merge; lifecycle swap would be too subtle to allow silently). Child MAY add new ids. |
| `operations:`    | Set-union of op names, child-wins on duplicates. Parent's unreferenced ops remain available.                                                                                 |
| `plugin:`        | Child fully replaces parent. A plugin binary is a single artifact, not a composition. Child with no `plugin:` block inherits the parent's.                                   |
| `config-schema:` | Layered via `allOf: [parent-schema, child-schema]`. Child can only tighten the parent's shape; broadening requires replacing (explicit opt-out).                             |


Multi-level `extends:` chains and cycles are rejected at `specify schema check` time with the same error surface as today's pipeline-composition check.

#### 6. Cross-schema coexistence

Every project activates multiple schemas simultaneously — at minimum a domain schema (omnia / contracts / vectis / …) plus the first-party platform schemas (`plan@v1`, `registry@v1`, `initiative@v1`). Two constraints apply across the active schema set:

- **Artefact id uniqueness.** No two active schemas may declare the same `artifact.id`. Checked by `specify check` against the project's active schema list.
- **Baseline-path uniqueness.** No two active schemas may claim the same baseline path or project-path. Prevents `plan@v1`'s `plan.yaml` from colliding with a hypothetical `publish@v1` writing to `plan.yaml`.

Schemas may *consume* each other's baselines as read-only context (e.g. `client-sdk@v1` reads the `contracts@v1` baseline). This is expressed by listing the consumed schema in a `consumes:` array on the consumer; the linter validates that every entry is an active schema on the project.

### First-party schemas and bootstrap

`plan@v1`, `initiative@v1`, `registry@v1` (and the domain-schema-facing scaffolding) need to be available **before any schema URL has been resolved** — `specify init` must know about `registry@v1` to validate `registry.yaml`, and schema resolution itself runs through `specify schema `*, which is core. Chicken-and-egg.

Resolution: first-party schemas are **embedded in the CLI binary** and exposed through the same `schema.yaml` surface as any third-party schema. The schema resolver checks the embedded set first, then falls back to URL resolution. An operator never installs `plan@v1`; it ships with `specify`. But the schema is still structurally a schema — same `artifacts:` / `operations:` / `pipeline:` blocks, same protocol, same linter rules. The "no exceptions" principle holds: these are schemas, just bundled.

First-party schema versioning tracks the CLI release. An embedded schema's `version:` field is effectively part of the CLI's ABI: a `specify` upgrade that changes `plan@v1`'s pipeline is a breaking change for any project that pinned the prior behaviour. Projects may pin the CLI version (`specify_version` in `project.yaml` already does this) but do not independently pin embedded schema versions — the coupling is deliberate.

`project.yaml` declares the domain schema; the core auto-activates the first-party platform schemas alongside it. A hub project (`hub: true`) activates `registry@v1` + `initiative@v1` + `plan@v1` but no domain schema. A single-repo project activates all four. A future project that opts out of a platform schema (rare, but possible for minimal libraries that don't plan) sets `disable-first-party: [plan]` — intentionally ugly, rarely used.

### Distribution: declarative with a subprocess escape

Three runtime models were compared:


| Model                                                     | Reach                                                                                      | Distribution                | Sandbox                 | Verdict                                                                    |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------ | --------------------------- | ----------------------- | -------------------------------------------------------------------------- |
| Pure declarative (YAML + markdown + named format adapter) | Artifact lifecycle, brief rendering, format validation the core vendors                    | Schema repo only            | Total (no code runs)    | **Default path.**                                                          |
| Subprocess plugin (`git-foo` convention)                  | Imperative ops that need the host toolchain (`xcodebuild`, `cargo`, `gradle`, `terraform`) | PATH-installed binary       | None (runs as operator) | **Escape hatch.**                                                          |
| WASM component (wasm32-wasip2)                            | Sandboxed imperative ops                                                                   | Bundled in the schema cache | Strong                  | **Deferred.** Can't reach host toolchains without a host-function surface. |


Pure-declarative schemas (YAML + markdown + a format adapter name) work end-to-end for artifact lifecycle, brief rendering, and format validation when the format adapter is one the core vendors (`markdown-spec`, `openapi`, `asyncapi`, `json-schema` — all available as Rust crates). No extension code needed.

For imperative operations — `vectis verify` running `xcodebuild`, `cargo check` on a generated crate, `gradle` on an Android shell, `terraform plan` on an infra schema — schemas declare a subprocess plugin:

```yaml
plugin:
  binary: specify-ext-vectis         # resolved on PATH, git-foo convention
  protocol-version: 1                # see §Protocol contract
  ops: [doctor, scaffold, config]    # which ops route to the plugin
```

Ops a plugin does not list in `plugin.ops` fall back to declarative handling (or error if none applies). The plugin never calls back into the CLI; all state is passed on the command line or on stdin. WASM-component plugins and in-process dynamic loading are out of scope for this RFC — subprocess is chosen because it is language-agnostic, matches the `git-foo` / `cargo-foo` convention, and keeps the trust boundary explicit.

#### Security posture

Subprocess plugins run with **the operator's full host privileges** — same as any other binary on PATH. The core does not sandbox them, does not lock them to a filesystem subtree, and does not intermediate their network access. This matches `git-foo` / `cargo-foo` and operators' existing trust relationship to their schema source: the project already trusts its declared schema URL (same URL drives code generation), so running a plugin from the same upstream adds no new trust edge. A schema URL from an untrusted source must be vetted before `specify init`. A sandboxed write-fence and WASM-component plugins are both candidates for a follow-up RFC once `specify check`'s inventory of write paths is trustworthy enough to enforce.

#### Workspace-clone path resolution

Under `specify workspace sync`, every `artifacts.*.baseline` / `artifacts.*.project-path` / `artifacts.*.delta` resolves relative to **the clone's project root**, not the hub's. This matches how `specify workspace sync` already treats every other path in the clone and is the only resolution rule that survives the hub/clone boundary.

### Protocol contract

The subprocess protocol has four moving parts: an invocation envelope, an args envelope on stdin, a result envelope on stdout, and a fixed exit-code mapping.

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

Flags are positional-free; the plugin author never parses free-form args. The core chooses `--protocol-version` from the plugin's `plugin.protocol-version` declaration and errors if the declared version is not one of the core's supported versions.

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

`op-args` is validated against `schemas/ops/<op>.schema.json` before the plugin is invoked.

#### Result envelope (stdout)

```jsonc
// Success
{
  "schema-version": 1,
  "op": "scaffold",
  "result": { /* op-specific; validated by schemas/ops/<op>.schema.json */ },
  "written-paths": ["iOS/ContentView.swift", "iOS/App.swift"],
  "warnings": []
}

// Failure
{
  "schema-version": 1,
  "op": "scaffold",
  "error": "missing-prerequisite",
  "message": "xcode-select not configured",
  "context": { /* optional structured payload */ }
}
```

All keys are kebab-case, matching the v2 CLI JSON contract. `error` variants are drawn from a per-op allowed set so consumers can `match` rather than string-compare.

#### Exit-code mapping


| Plugin exit | Meaning                                             | Dispatcher `CliResult` |
| ----------- | --------------------------------------------------- | ---------------------- |
| `0`         | `result` present                                    | `Success`              |
| `1`         | `error` present (generic failure)                   | `GenericFailure`       |
| `2`         | `error` present (validation / missing prerequisite) | `ValidationFailed`     |


The core dispatcher maps the plugin's exit through `CliResult` rather than propagating verbatim — same uniform exit-code contract whether the op was handled declaratively or by a plugin.

#### Self-description

Every plugin MUST implement an implicit `describe` op:

```text
specify-ext-<schema> --op describe --protocol-version 1 < {}
```

returning:

```jsonc
{
  "protocol-versions-supported": [1],
  "ops": [
    { "name": "doctor",   "op-args-schema": { /* JSON Schema */ } },
    { "name": "scaffold", "op-args-schema": { /* JSON Schema */ } }
  ],
  "plugin-version": "0.3.1"
}
```

The core calls `describe` once per session and caches the result; it uses the response to drive `specify ext <schema> --help`, to validate op-args before invoking the plugin, and to detect protocol-version mismatches before a production operation runs.

#### Protocol versioning

The protocol is versioned independently of individual schemas:

- Each core release declares a set of supported protocol versions (initially `[1]`).
- Each plugin declares `plugin.protocol-version` in `schema.yaml` and `protocol-versions-supported` in its `describe` response.
- A plugin-vs-core mismatch fails `specify ext <schema> …` with a `protocol-version-unsupported` diagnostic before any op runs.
- When the protocol evolves (new hook, new op, new envelope field), the next core minor release adds a version to its supported set; the previous version is deprecated in release notes and retired two minor versions later.

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

Everything under today's `specify plan`, `specify initiative`, `specify registry`, `specify workspace`, `specify contract`, `specify vectis` surfaces — and any future schema's verbs — routes through `specify ext <schema> <op>`.

`specify ext <schema>` keeps extensions visually distinguished, prevents name collisions, and keeps core verb tab-completion stable across projects. Short forms (`specify <schema> <op>`) were considered and rejected (§Alternatives). `<schema>` is the schema's `name` field, resolved against the project's active schema set (domain schema from `project.yaml:schema` plus auto-activated platform schemas, plus any peer schemas declared by `registry.yaml`) — the dispatcher refuses to run an op against a schema that isn't active.

#### Discoverability

The dispatcher exposes three layers of help without the operator having to read the schema source:

- `specify ext` — lists every schema the project (and its registry peers) declare, plus the ops each implements. Driven by the cached `describe` response.
- `specify ext <schema>` — lists the ops the named schema implements, with a one-line synopsis from `describe`. Exits non-zero if the schema is unknown.
- `specify ext <schema> <op> --help` — renders the op's `op-args-schema` as a flag list, plus the op's standard result shape from `schemas/ops/<op>.schema.json`.

Help is always a local operation against cached data — no plugin invocation, no network.

#### Concrete cut-overs

**Contracts schema:**

- `specify contract list` → `specify ext contracts list`
- `specify contract validate` → `specify ext contracts validate`

**Vectis schema:**

- `specify vectis init` → `specify ext vectis scaffold project`
- `specify vectis verify` → `specify ext vectis doctor`
- `specify vectis add-shell` → `specify ext vectis scaffold shell --target ios`
- `specify vectis update-versions` → `specify ext vectis config set versions.rust 1.82.0`
- `specify vectis versions` → `specify ext vectis config show`

**Plan schema (first-party, phase 4):**

- `specify plan create` → `specify ext plan scaffold plan` (or via a heavy change with the plan pipeline)
- `specify plan add` → `specify ext plan scaffold entry`
- `specify plan amend` → `specify ext plan scaffold entry --amend`
- `specify plan validate` → `specify ext plan validate`
- `specify plan doctor` → `specify ext plan doctor`
- `specify plan status` → `specify ext plan list`
- `specify plan next` → `specify ext plan inspect --next`
- `specify plan transition <name> <target>` → `specify ext plan transition <name> <target>`
- `specify plan archive` → resolved via `specify change adopt` on the plan artefact
- `specify plan lock {acquire,release,status}` → `specify ext plan config set lock.holder <pid>` / `specify ext plan config show`

**Registry schema (first-party, phase 4):**

- `specify registry add` → `specify ext registry scaffold project`
- `specify registry remove` → `specify ext registry scaffold project --remove`
- `specify registry show` → `specify ext registry inspect`
- `specify registry validate` → `specify ext registry validate`

**Initiative schema (first-party, phase 4):**

- `specify initiative create` → `specify ext initiative scaffold`
- `specify initiative show` → `specify ext initiative inspect`
- `specify initiative finalize` → `specify change adopt` on the initiative artefact (with a custom adopt hook that checks every plan entry is terminal and every PR merged)

**Workspace** — pending the open question; cut-over map depends on whether workspace becomes a schema or remains the cross-schema coordinator in core.

## Alternatives Considered

**Pure-declarative schemas only.** Beautiful but incomplete: `vectis verify` cannot be expressed without calling `xcodebuild`. Rejected as a hard rule; retained as the default path.

**WASM-component plugins.** Sandboxed and matches Omnia's stack, but cannot reach the host toolchains that half the imperative ops depend on without a large host-function surface. Deferred to a follow-up RFC if the subprocess model shows real pain.

**In-process dynamic-library plugins.** Rust ABI instability alone disqualifies this.

**Short-form CLI (`specify <schema> <op>`).** Considered for brevity. Rejected: extensions become indistinguishable from core verbs, tab-completion results shift project-to-project, and every new schema carries a name-collision risk. The two-keystroke `ext` prefix is worth it.

**Keep schema-specific top-level subcommands (status quo, formalised).** Considered as a minimum-pain path: leave `specify contract` / `specify vectis` where they are, just make them discoverable. Rejected: the whole point of the reframe is that the core surface stops growing with every concern.

**Multiple orthogonal escape hatches** (declarative + WASM + subprocess + in-repo Rust). Considered to let schemas pick the right tool. Rejected: one escape is already a burden on operators installing extensions; several would split the ecosystem.

**Keep `artifacts:` lifecycle-only (the earlier RFC-13 draft).** Considered as a narrower landing. Rejected: the artifact split is only one of four schema-specific surfaces hard-coded in the CLI. Landing it in isolation would require a second RFC within weeks to handle operations and config, and the four pieces are easier to reason about together than separately.

**A top-level `artifacts.yaml` (or `operations.yaml`) next to `schema.yaml`.** Considered for symmetry with `registry.yaml`. Rejected: all four extension surfaces are schema-bound, not project-bound, and splitting them across files duplicates the schema-id binding for no win.

## Non-Goals

- **Replacing the draft / build / adopt phase model.** The core lifecycle loop is preserved exactly — the RFC makes it the only thing the core does.
- **Format-level contract evolution.** SemVer + `info.x-specify-id` + cross-repo uniqueness continue to be owned by RFC-12; this RFC only moves where those rules run from.
- **WASM / in-process plugins.** Deferred; subprocess is the only extension runtime in this RFC.
- **A general sandboxed write-fence.** Refusing writes outside declared paths is powerful but invasive; deferred to a follow-up once `specify check` proves the write-path inventory is correct.
- **Cardinality > 1 on delta or baseline.** One artefact → one delta → one baseline. Revisited only when a real schema needs more.
- **Cloud execution semantics.** Orthogonal; the subprocess protocol serialises the same way either way.
- **Back-compat for schemas without the new surface.** See §Migration — the current usage footprint (omnia + core loop only) lets us cut over without a fallback path.
- **Third-party platform schemas.** `plan@v1`, `registry@v1`, `initiative@v1` are first-party and bundled with the CLI. An operator cannot swap in a third-party plan schema today; if the need ever arises, it's a follow-up RFC, not part of this landing.

Multi-schema per project is explicitly **in scope** — it's a requirement of the reframe, since `plan@v1` + `registry@v1` + `initiative@v1` always coexist with a domain schema. See §Cross-schema coexistence.

## Implementation Scope

A staged landing, each stage independently testable and shippable. Every stage preserves working `/spec:draft → /spec:build → /spec:adopt` for the `omnia` schema (the only schema currently in real use).

### Phase 1 — Artifact declarations

Lands the lifecycle surface from the earlier RFC-13 draft, widened to the four-value taxonomy.

1. New schema fields `artifacts:` parsed in `crates/schema/src/`. JSON Schema additions in `schemas/schema.schema.json`.
2. `crates/merge/` refactor: replace the hard-coded `specs_dir` + `contracts_dir` pair with iteration over the active schema's `managed` artifacts, dispatched on `merge-strategy`. Core ships `three-way` and `opaque-replace` defaults.
3. `crates/validate/`: add `--artifact <id>` filter; brief renderer learns `$ARTIFACT_DELTA[<id>]` / `$ARTIFACT_BASELINE[<id>]` substitutions.
4. `src/config.rs`: drop `specs_dir` / `contracts_dir`; add `ProjectConfig::baseline_path(&schema, artifact_id)` and `ProjectConfig::delta_path(&schema, artifact_id)`.
5. `specify check` (RFC-5): lints flag direct literal paths in briefs and skills, plus the per-`managed`-artifact invariants (verifier brief present, no baseline collision with another schema's external project-path, pipeline stays within declared artifacts).

First-party schemas adopt `artifacts:` blocks declaring today's paths exactly — no filesystem changes.

### Phase 2 — Brief renderer + hook defaults

1. Generalise the brief renderer so schemas can declare additional substitution variables. Today's hard-coded vocabulary becomes the core baseline; schemas extend it.
2. Formalise the five lifecycle hooks. Wire the core's `three-way` and `opaque-replace` implementations as the defaults for `artifact-preview-adopt` and `artifact-adopt`.
3. Move `validate_baseline_contracts` out of `crates/validate/src/` into a `format: openapi-asyncapi-bundle` adapter declared by `schemas/contracts/schema.yaml`. The core validate crate stops knowing the word "contract".

### Phase 3 — Operations surface

1. New `specify ext <schema> <op>` dispatcher in `src/cli.rs` and `src/commands/`. Closed vocabulary, JSON-in / JSON-out per op (shapes defined under `schemas/ops/`).
2. Schema surface grows `operations:` listing which ops the schema implements, and (optionally) a `plugin:` block declaring a subprocess binary.
3. `specify-ext-vectis` crate extracted from today's in-binary `specify_vectis` library; ships as its own binary published alongside the `vectis` schema.

### Phase 4 — Extract platform schemas and retire schema-specific core surfaces

This is the largest phase because it proves the reframe: if `specify plan` can be lifted into a schema without operator regressions, the abstraction is real.

1. **Extract `plan@v1`, `registry@v1`, `initiative@v1` as first-party schemas.** Each ships embedded in the CLI binary via `include_str!` or a tidy `embedded-schemas/` tree, exposed through the same schema resolver path as any URL-resolved schema.
2. **Extract their operator verbs into `specify ext <schema>`.** The cut-over map in §CLI surface is the canonical reference. Most verbs land under `scaffold` / `inspect` / `validate` / `transition` / `config`; `specify plan archive` and `specify initiative finalize` route through `specify change adopt` on the schema's artefact (with custom adopt hooks for the close-out invariants).
3. **Delete `Commands::Plan`, `Commands::Initiative`, `Commands::Registry`, `Commands::Vectis`, `Commands::Contract`** from `src/cli.rs`; delete the matching modules under `src/commands/`.
4. **Retire `specify_vectis` as a library dependency of `specify-cli`** and publish `specify-ext-vectis` separately.
5. **Decide the workspace question** (see §Open Questions). Either extract `workspace@v1` as a schema with a subprocess plugin, or document workspace as the one deliberate exception to the schema rule (the cross-schema coordinator).
6. **Retire surviving hard-coded `contracts` / `specs` references** in `crates/merge/`, `crates/validate/`, `src/config.rs`, and `crates/change/` (`Plan`-related types move to the `plan@v1` schema's first-party definition crate).
7. **First-party schemas publish their full surface** — `omnia`, `contracts`, `vectis`, `plan`, `registry`, `initiative` all declare `artifacts:` + `operations:` + (where applicable) `plugin:` + `config-schema:` + `pipeline:` blocks.
8. **Auto-activation of platform schemas at `specify init`.** A project's `project.yaml` declares its domain schema; the core activates `plan@v1` + `registry@v1` + `initiative@v1` alongside it automatically. Hubs activate the three platform schemas without a domain schema.

Estimated total across all four phases: ~~3500–4500 lines of Rust (core + schema extractions) + the extracted `specify-ext-vectis` binary (largely code-movement, not net-new) + schema YAML in this repo. Phase 4 is the largest single tranche (~~60% of the total), so it may land as a sequence of smaller commits rather than one atomic change.

### This repo (`augentic/specify`)

1. Widen `schemas/schema.schema.json` to cover `artifacts:`, `operations:`, `plugin:`, `config-schema:`, `consumes:`.
2. Add `schemas/ops/<op>.schema.json` for each op in the closed vocabulary (`list`, `validate`, `inspect`, `doctor`, `scaffold`, `config`, `transition`).
3. Rewrite `schemas/contracts/schema.yaml`, `schemas/omnia/schema.yaml`, `schemas/vectis/schema.yaml` to declare their full extension surface.
4. Port brief prose to `$ARTIFACT_DELTA[<id>]` / `$ARTIFACT_BASELINE[<id>]` substitutions.
5. Update `plugins/contract/` and `plugins/vectis/` skills to invoke `specify ext <schema> …` instead of the old top-level subcommands.
6. **Phase 4 additions:** check in `schemas/plan/schema.yaml`, `schemas/registry/schema.yaml`, `schemas/initiative/schema.yaml` as the source-of-truth definitions for the embedded platform schemas. The CLI consumes them at build time via `include_str!` or similar. Skills under `plugins/spec/skills/plan/`, `plugins/spec/skills/execute/` re-route their CLI invocations from `specify plan …` to `specify ext plan …`.
7. Document the protocol in `docs/reference/schema-extensions.md`; cross-link from each schema's README. Add a glossary entry for "active schema set," "heavy mutation," "light mutation," "first-party schema."

## Migration

The usage footprint makes this unusually forgiving: only the `omnia` schema and the core `/spec:draft → /spec:build → /spec:adopt` loop are in real-world use. `specify contract `*, `specify vectis *`, and (pending confirmation) the bulk of `specify plan *` / `specify initiative *` / `specify registry *` have no durable external user base to protect. The operator-facing CLI reshapes considerably in phase 4, but the behaviour behind each verb is preserved.

Consequently: **hard cut-over, no fallback path.** Each phase's minor version is a breaking change for the surfaces it touches. There is no deprecation window, no `artifacts:`-absent fallback, no aliasing of old CLI verbs. Pre-reframe schemas simply fail to load against the post-reframe CLI with a clear diagnostic pointing at this RFC.

Three invariants guard the landing:

1. **Omnia keeps working.** Every phase's acceptance criterion includes running `/spec:draft → /spec:build → /spec:adopt` on a canonical omnia change end-to-end.
2. **The core never learns a schema name.** A `specify check` rule greps for hard-coded `"contracts"` / `"vectis"` / `"omnia"` / `"plan"` / `"registry"` / `"initiative"` string literals in core crate sources and fails CI on any match outside tests. The check also covers the extracted platform schemas so their first-party status doesn't quietly re-introduce hard-coding.
3. **The core never learns an artefact id either.** A companion `specify check` rule fails on hard-coded `"specs"` / `"contracts"` / `"crates"` / `"plan"` artefact-id literals in core crate sources, with the same test-only exception. Today's `ProjectConfig::specs_dir` and `ProjectConfig::contracts_dir` are the canonical violations; phase 1 retires them. The rule's existence is what stops `specs` from creeping back in as a system artefact through a future patch — it has to come back via `artifacts:` or not at all.
4. **Platform schemas are still schemas.** A `specify check` rule verifies that `plan@v1`, `registry@v1`, `initiative@v1` each pass the same validation as any third-party schema — `schema.yaml` parses against `schema.schema.json`, all declared briefs exist, `operations:` is a subset of the closed vocabulary, and so on. Embedded schemas get the same invariants as URL-resolved ones.

Linter rules (in `specify-check`, RFC-5) enforce the per-artefact invariants from the earlier draft plus:

- A schema's `operations:` list MUST be a subset of the closed op vocabulary (`list`, `validate`, `inspect`, `doctor`, `scaffold`, `config`, `transition`).
- A schema's `plugin.binary` MUST resolve on PATH or be declared absent.
- A schema's `config-schema:` MUST parse as a JSON Schema.
- **Active-schema-set invariants** (new): artefact-id uniqueness and baseline-path uniqueness across every schema active on a project.
- **First-party schema parity** (new): embedded schemas pass every rule any URL-resolved schema must pass.

## Open Questions

1. **Distribution model beyond subprocess.** Subprocess is this RFC's only extension runtime. When does WASM become worth adding? Provisional: revisit when the third schema asks for it or when a hosting constraint forces sandboxing (RFC-7 cloud execution).
2. **Multiple schemas per project.** Today's `project.yaml:schema` is singular; `extends` is inheritance, not composition. A real multi-concern project (app code + shared contracts + infra) may need `schemas: [omnia, infra]`. Provisional: out of scope here; track as a candidate RFC once a real multi-concern project lands.
3. **Artifact taxonomy — lock in four values now?** `managed`, `external`, `read-only`, `audited`. `audited` has no consumer yet; shipping the value without the implementation means a future schema hits an error it should not. Provisional: ship `managed`, `external`, `read-only` in phase 1; reserve `audited` as a parse-time future-use error.
4. **Closed vs open operations vocabulary.** Closed gives uniform UX; open gives flexibility. Provisional: closed. A schema that needs a novel op proposes it as a protocol RFC.
5. **CLI surface spelling.** `specify ext <schema>` vs `specify x <schema>` vs `specify <schema>`. Provisional: `ext` — explicit, collision-free.
6. **Config location.** Nested `extensions.<name>` under `project.yaml` vs sibling `schema-config.yaml`. Provisional: nested; revisit if extension count grows past a dozen.
7. **Plugin resolution model.** PATH-based (`specify-ext-<schema>`) vs schema-local (`<schema-dir>/plugin/binary`). Provisional: PATH-based, matching `git-foo`. Schema-local complicates caching.
8. **Hook default implementations.** Core ships `three-way` and `opaque-replace`. Does it also ship a default `artifact-validate` (e.g. "brief frontmatter parses")? Provisional: no — validation is where format semantics matter most and a silent default would mask missing schema work.
9. **Lifecycle naming.** `managed` / `external` / `read-only` / `audited` survived the earlier draft's review unscathed; confirm or replace (`specify-owned` / `tooling-owned` / `baselined` / `live`) one more time before phase 1 lands.
10. `**specify ext` exit-code contract.** Does the dispatcher propagate the plugin's exit code verbatim, or map it through the core's `CliResult`? Provisional: map through `CliResult` so the top-level surface stays uniform; plugins that need a richer signal use the JSON `error` field.
11. **Help-text authoring.** Is the per-op help surface (`specify ext <schema> <op> --help`) fully auto-derived from `op-args-schema`, or can a schema ship hand-authored help prose per op? Provisional: auto-derive from schema + an optional `description` field on each op entry in `describe`. Hand-authored long-form help is a future-RFC concern.
12. **Format-adapter registry.** Where does the list of core-vendored format adapters (`markdown-spec`, `openapi`, `asyncapi`, `json-schema`) live, and how does a schema declare a new one? Provisional: a fixed in-core registry to start; revisit when a third-party schema wants to ship its own adapter.
13. **Single `schema:` vs `schemas:` in `project.yaml`.** Once `plan@v1` + `registry@v1` + `initiative@v1` auto-activate alongside the domain schema, does the operator see a single `schema:` field (with platform schemas auto-activated by the core) or a full `schemas: [...]` list? Provisional: single `schema:` with auto-activation — keeps the operator surface minimal. A `disable-first-party:` escape hatch covers the rare opt-out. Revisit if a real project needs a non-default platform-schema set.
14. **First-party schema versioning.** Embedded schemas ship with the CLI; their `version:` is coupled to the CLI release. Does a project pin a first-party schema version independently of the CLI? Provisional: no — the embedded schemas carry the CLI version as their pin and are treated as an ABI surface with SemVer rules. Projects already pin `specify_version`; that same pin gates platform-schema behaviour.
15. **Workspace: schema or core exception?** `specify workspace` is mostly git-shelling (`sync`, `push`, `merge`) and doesn't fit "draft-build-adopt a class of artefacts" as cleanly as plan/registry/initiative. Two reasonable answers: (a) `workspace@v1` schema with a heavy subprocess plugin (artefact = clones directory), preserves "no exceptions"; (b) workspace stays core as the cross-schema coordinator, documented as the one deliberate exception. Provisional: (b) — pretending workspace is an artefact-lifecycle concern stretches the abstraction, and carving it out explicitly is more honest than forcing the frame. Phase 4 locks the decision.
16. **Heavy vs light mutation boundary.** Some schemas (contracts, vectis) naturally use only one path; some (plan, registry) use both. Is the boundary declared per-operation in `schema.yaml`, or inferred by whether the op writes to a `managed` artefact? Provisional: declared per-op via a `mutation: heavy | light` field on `operations:` entries, so the schema author is explicit about which ops bypass the change loop.

## References

- [RFC-1: `specify` CLI](archive/rfc-1-cli.md) — owns the crates the reframe touches: `specify-schema`, `specify-merge`, `specify-validate`, `specify-change`, and the `src/cli.rs` dispatcher.
- [RFC-8: API contracts](archive/rfc-8-api-contracts.md) — introduced the `contracts@v1` schema and the delta-then-promote merge semantics that become the `opaque-replace` default.
- [RFC-2: Execution](archive/rfc-2-execution.md) — `/spec:execute --loop`, plan lifecycle, phase outcomes; the plan schema's `doctor` / `transition` / `inspect --next` ops are lifted from this RFC's CLI surface.
- [RFC-3a: Monoliths](archive/rfc-3a-monoliths.md) — plan authoring pipeline (`/spec:plan`); the two-brief `pipeline.plan` that already exists is evidence the plan-as-schema reframe is half-implemented.
- [RFC-3b: Platform](archive/rfc-3b-platform.md) — registry routing, workspace clones; the registry schema and workspace open question trace back here.
- [RFC-9: Platform](archive/rfc-9-platform.md) — moved registry, plan, initiative, and contracts to the repo root; established the operator-vs-framework path boundary the artifact declarations build on. The cross-repo initiative umbrella's `/spec:plan --orchestrate` sits at the top of the post-reframe schema stack.
- [RFC-12: Refine RFC-8](archive/rfc-12-refine-rfc-8.md) — SemVer + `info.x-specify-id` rules; the `specify contract validate` checks that become the `contracts` schema's `baseline-validate` hook.
- [RFC-5: Framework Linter](rfc-5-lint.md) — home of the lints enforcing the reframe's invariants (no core literals for schema names, declared-path discipline, closed-vocab operation lists).
- [Roadmap](roadmap.md) — §3 (standards/codex) motivates the `read-only` lifecycle; §5 (MCP surface) and §7 (cloud execution) are consumers of a stable core surface; §6 (observability) rides on the `specify ext` dispatcher contract.
- `plugins/contract/references/baseline-vs-delta.md` — cross-format author rules whose path constants become substitution variables.
- `docs/how-to/migrate-to-v2-layout.md` — the v2 layout boundary the artifact declarations make per-artifact configurable.

