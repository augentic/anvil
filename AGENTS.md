# Augentic Plugins - Agent Instructions

This repository is **Rust plus embedded prose**: the workspace at the repository root produces the `specify` runtime binary, and the surviving markdown (skill wrappers, reference docs, adapter prose) ships alongside or embedded in it. Generated Rust crates and Swift shells appear in downstream projects, not in this repository itself.

## Vocabulary

Specify names two adapter roles and three workflow nouns. Use the terms verbatim.

### Adapter roles

- **source adapter** — input role with two operations: `survey` (plan time) and `extract` (slice time). Lives at `adapters/sources/<name>/adapter.yaml`. Examples: `intent`, `documentation`, `typescript`, `screenshots`, `captures`.
- **target adapter** — output role with three operations: `shape` (read by core synthesis), `build`, and `merge`. Lives at `adapters/targets/<name>/adapter.yaml`. Examples: `omnia`, `vectis`, `contracts`. See [`docs/explanation/adapter-anatomy.md`](docs/explanation/adapter-anatomy.md) for the full source / target contract, including the [adapter-vs-Cursor-plugin manifest boundary](docs/explanation/adapter-anatomy.md#adapter-manifests-vs-cursor-plugin-manifests).
- **plugin** — historical shorthand for the shared adapter shape. The Rust loaders are `SourceAdapter::resolve(name, project_dir)` and `TargetAdapter::resolve(name, project_dir)` in [`crates/workflow/src/adapter/`](crates/workflow/src/adapter); each validates against the matching per-axis `source.schema.json` / `target.schema.json` distributed with the CLI. The noun "plugin" survives in operator-facing prose where source + target authors share the same audience tag.

### Synthesis terms

- **lead** — slice-sized unit emitted by `survey`; one raw, unmerged block per lead under `## Lead inventory` in `discovery.md`, each identified by its `(source, lead)` pair (`lead` is unique only within a `source`).
- **evidence** — per-source result of `extract`; structured document with `claims:` persisted to `.specify/slices/<slice>/evidence/<source>.yaml`.
- **provenance** — the sources behind one requirement (the `Sources:` list in `spec.md`).
- **conflict / divergence** — unresolvable vs authority-resolved disagreement; surfaced inline as `[conflict]` / `[divergence]` tags on requirement headers.
- **authority** — closed enum (`intent` > `documentation` > `behaviour`) controlling who wins a disagreement.
- **model.yaml** — the single structured slice artifact at `.specify/slices/<slice>/model.yaml`, carrying provenance **inline** on each requirement. The provenance audit view is **projected on demand** by `specify slice provenance` — there is no persisted `provenance.yaml`. Audit-only; `spec.md` is the authoritative artifact. See [`plugins/spec/references/synthesis/provenance.md`](plugins/spec/references/synthesis/provenance.md) for the projected shape and audit posture.
- **component catalog** — operator-curated file at `.specify/design-system/components.yaml` declaring shared UI components (`status: confirmed | rejected`). The Vectis target reads the catalog at build time and factors shared component code per shell tree. Follows the same pattern as `tokens.yaml` and `assets.yaml`. Opt-in; absent catalog means no component factoring. Validated by `specify slice validate` (`slice-catalog-drift`) and the vectis adapter's in-guest composition validation (catalog cross-reference check). See [docs/explanation/components.md](docs/explanation/components.md).

### Workflow nouns

- **slice** — the single unit that flows through the fixed `refine → build → merge` loop. Each slice has its own proposal, spec, design, tasks, and merge step. Lives at `.specify/slices/<name>/`. Driven by `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop` and the `specify slice *` CLI verbs.
- **change** — the operator-defined umbrella that coordinates one or more slices through `change.md` + `plan.yaml`. Driven by `/spec:plan`, `/spec:execute`, `/spec:finalize` and the `specify plan *` CLI verbs. `change` is on-disk vocabulary, not a slash-command namespace.

Use *slice loop* for the per-slice lifecycle; reserve *change* for the on-disk umbrella that owns `change.md` and `plan.yaml`.

### Workspace topology (disambiguation)

The word **workspace** overloads three related concepts. Use them verbatim:

| Term               | Meaning                                                                                                            |
| ------------------ | ------------------------------------------------------------------------------------------------------------------ |
| **Workspace**      | Registry-only platform repo: `workspace: true` in `project.yaml`, `registry.yaml`, plan artifacts at the repo root |
| **Workspace slot** | Materialised peer at top-level `workspace/<project>/`                                                              |
| **Workspace sync** | `specify workspace sync` — materialise slots and regenerate `topology.lock`                                        |

`/spec:init workspace` and `specify init --workspace` scaffold a workspace; the CLI chains an initial workspace sync before returning.

### Workflow, standards, and artifacts

Specify separates three concerns. Use the terms verbatim; see [docs/explanation/standards-layer.md](docs/explanation/standards-layer.md) for the full picture.

| Layer                     | Role                                          | Examples                                                                            |
| ------------------------- | --------------------------------------------- | ----------------------------------------------------------------------------------- |
| **Workflow**              | Phase orchestration and lifecycle transitions | `/spec:plan`, `/spec:execute`, `specify slice transition`                           |
| **Artifacts**             | Slice-local and baseline product intent       | `spec.md`, `plan.yaml`, `.specify/specs/`                                           |
| **Engineering standards** | Durable policy that outlives any slice        | Rules under `adapters/**/rules/`; `specify rules export` and `specify lint project` |

**Authoring standards** (`docs/standards/`, enforced by `specify lint framework` / `make lint` on this repo) govern skill and doc house style. **Engineering standards** (rules under `adapters/**/rules/`, exported by `specify rules export` and enforced by `specify lint project`) govern generated and hand-written code in consumer projects. Do not conflate them.

`specify lint project` is CI-native **standards enforcement**, not a workflow phase — findings may block CI but never transition plans or slices. Build-time `REVIEW.md` and plan Gate 1 `approved` are separate surfaces.

### Authority and reconciliation mechanics

The full mechanics — per-slice operator overrides, inline provenance shape — live in [`DECISIONS.md`](DECISIONS.md). The headline rules:

- **Authority resolution order** — per-slice override → Evidence document-level `authority:` → conflict. (A per-Evidence per-kind override is deferred to a future RFC.) See [`plugins/spec/references/synthesis/authority.md`](plugins/spec/references/synthesis/authority.md) for the resolution order and override surface.
- **`captures` source adapter** — consumes runtime capture trees and emits `kind: example` Evidence claims with `replay-digest: sha256:…` anchors and default `authority: behaviour`.
- **Authority-override authoring** — `specify plan amend --authority-override <slice> <kind>=<key>`; orphan source keys are rejected by `specify slice validate` with `slice-authority-override-orphan-source`.
- **Reconciliation checks** — `specify slice validate` catches spec-vs-model staleness and orphan contributing claims; provenance is carried inline in `model.yaml` so there is no separate file to drift.
- **Extraction is agent-only and never cached** — `survey` / `extract` re-run the brief every time; there is no extraction-result cache.

## Workflow overview

The default rhythm is `/spec:plan` → operator stamps `approved` → `/spec:execute` → `/spec:finalize`. Slash commands operators reach for, in the order they appear in a project's life:

- `/spec:init` — scaffold `.specify/`, run once per project.
- `/spec:plan` — author `change.md` and `plan.yaml`: survey each bound source, propose `slices[]` rows by reconciling leads across sources, validate the plan. Exits at `plan.lifecycle: pending` and prints the literal `specify plan transition <name> approved` command.
- `specify plan transition <name> approved` — **Gate 1.** Operator-only stamp; `/spec:plan` never writes `approved` itself.
- `/spec:execute` — refuses unless the plan is `approved`; loops `specify plan next` → `/spec:refine` → `/spec:build` → `/spec:merge` until every per-entry `status` is `done`.
- `/spec:refine` — breakout: for one slice, run `extract` per bound source, synthesize `proposal.md` / `spec.md` / `design.md` / `tasks.md`, validate, transition to `refined`.
- `/spec:build` — breakout: validate artifacts, implement the slice's tasks.
- `/spec:merge` — breakout: fold the slice's deltas into the baseline and archive it; the only writer of per-entry `done`.
- `/spec:drop` — abandon a slice without merging.
- `/spec:finalize` — push branches, then run `specify plan archive`. Opening and merging pull requests is operator-owned and happens outside Specify.

N=1 is degenerate, not special: `intent.survey` produces one lead, the operator stamps `approved`, and `/spec:execute` drives the same single-slice rhythm as a 12-slice change.

## Skill / CLI responsibility split

Phase skills are agent-driven orchestrators. Every deterministic operation — manifest validation, `metadata.yaml` reads and writes, plan and slice lifecycle transitions, source and target resolution, artifact-completion checks, baseline conflict detection, delta merge, archive move — runs through the `specify` CLI. Skill markdown drives the agent-side work: eliciting operator intent, reading brief bodies, writing evidence and synthesized artifacts, running the target adapter's build brief, and rendering summaries.

The CLI surface skills depend on is documented in [`specify` `--help`](cli). The headline groups: `specify init` (with the re-entry flag `--upgrade`, which bumps the `specify` pin and re-scaffolds preservation-safe files only, and `--platforms <csv>`, which declares the project's target platform set — required when the target adapter declares `platforms.required`), `specify source {resolve, survey, extract}`, `specify target {resolve}`, `specify slice {create, refine, model show, build, transition, validate, provenance, merge}`, `specify plan {create, author, execute, add, amend, transition, next, status, archive}` (`plan status` is the read-only next-action projection — `refine|build|merge <slice>` / `stop <reason>` / `drained` — over plan entries, slice metadata, and the journal tail), `specify archive {prune}` (retention-policy GC over the prunable slice/plan archive), `specify workspace {sync, push, prepare}`, `specify upgrade` (channel-aware CLI self-update), `specify plugins {doctor, refresh}` (Cursor plugin-cache drift report and invalidation), and `specify journal {emit, show}` (`emit` — the guarded front door onto the closed journal taxonomy for agent-orchestrated phases; `show` — the read-only `--filter`/`--limit` projection over the journal). `specify source survey`/`extract` resolve `<source>` against `plan.yaml.sources.<key>` and run the bound source adapter's compiled-in brief through one guest orchestration each (source extraction is agent-only). `specify slice build <slice>` is the guest-routed target-build verb: the orchestration assembles + schema-validates the build request, emits `target.execution.agent`, drives the target `build` brief, validates the report, and owns the `built` transition, journaling `slice.build.started` / `.succeeded` / `.failed`. `specify slice merge` fires `slice.merge.started` / `.succeeded` / `.failed` on its validator outcome (not on a merge report) alongside the durable `slice.archive.created`.

Never hand-edit `metadata.yaml`, `project.yaml`, `plan.yaml`, `discovery.md`, `sources.yaml`, or `targets.yaml`; never `mkdir -p .specify/...`; never `mv` anything into `.specify/archive/`. Route through the CLI — it enforces the legal lifecycle set and validates inputs in one place for humans, agents, and CI.

## Contracts target adapter

The contracts target adapter owns API contract authoring, import, and validation. Its `build` brief runs the OpenAPI, AsyncAPI, and JSON Schema format sub-flows, each with author / import / verify references under `adapters/targets/contracts/prose/references/`.

The matching validation surface is the contracts adapter's in-guest validator, run by the target build and merge orchestrations.

## Vectis asset materialization

Vectis-bound projects commit per-platform exports under `design-system/assets/exports/`; shell writers render by `assets.yaml` entry `kind` — never substitute platform glyphs for `vector` / `raster` ids at build time. See [`rfcs/roadmap.md`](rfcs/roadmap.md#recently-implemented) (**Recently implemented**).

| Concern | Where |
| ------- | ----- |
| Materialize + export conventions | In-guest vectis asset materialization — codified in the vectis core's decisions in [`specify-adapters`](https://github.com/augentic/specify-adapters) |
| Build-prelude auto-materialize | The vectis guest's build prelude — runs automatically inside the guest-routed `specify slice build`; no engine-side prepare hook remains |
| Render-by-`kind` review rule | [`adapters/targets/vectis/prose/rules/VECTIS-006-asset-render-by-kind.md`](adapters/targets/vectis/prose/rules/VECTIS-006-asset-render-by-kind.md) |
| Writer / integration contracts | [`adapters/targets/vectis/prose/references/ios/design-system-integration.md`](adapters/targets/vectis/prose/references/ios/design-system-integration.md), [`android/design-system-integration.md`](adapters/targets/vectis/prose/references/android/design-system-integration.md), [`briefs/build/ios/write.md`](adapters/targets/vectis/prose/briefs/build/ios/write.md), [`briefs/build/android/write.md`](adapters/targets/vectis/prose/briefs/build/android/write.md) |

## Plan-driven loop

`/spec:plan` authors the plan and exits at Gate 1; the operator stamps `approved`; `/spec:execute` drives the loop; `/spec:finalize` closes it. Plan *entries* are only ever written via `specify plan add` / `specify plan amend`; plan *lifecycle* is only ever written via `specify plan transition`; per-entry `in-progress` is only ever written by `specify plan next`; per-entry `done` is only ever written by `specify slice merge`. Per-entry status walks backwards only via `specify plan transition <entry> --undo`, which refuses to skip rungs (`done → in-progress`, then a second call for `in-progress → pending`) and fires one `plan.transition.undone` journal event per rung. The phase skills themselves stay unaware of the plan — they operate slice-by-slice. Hand-driven fallback: `specify plan next` → `/spec:refine` → `/spec:build` → `/spec:merge`, repeat until drained.

## Testing Philosophy

Specify strictly enforces an **aggressive integration-first posture**. 

- **Design against the public surface:** before adding a unit test, ask whether integration can reach the behavior — reachable through a CLI input or `pub` fn, observable at a public boundary (stdout JSON, exit code, filesystem), and affordable to assert there without a subprocess explosion. If yes, write the integration test; the unit test is redundant.
- **Default to Deletion:** a `src` unit test survives only when it is reachable and observable but cheap *only* in-process against a **private** kernel (a proptest or dense matrix), or covers a genuinely CLI-unreachable branch. If the kernel is already `pub`, re-home the test to `crates/<name>/tests/` instead.
- **Crate-Level Integration:** Put tests in `crates/<name>/tests/` instead of the root `tests/` when they test isolated domain logic that does not require full CLI orchestration. End-to-end and purely CLI-focused tests belong in the root `tests/`.
- **Widening is a last resort:** do NOT alter public APIs simply to support integration tests — prefer collapse-and-keep. The target is *near-zero* unit tests (no redundant or integration-reachable ones), not literal zero. A ratchet gate enforces this in both repos (this repo's `tests/rust_quality.rs`; adapters `tools/rust-quality`); `cargo llvm-cov nextest` remains the brake that ensures coverage holds during migrations.

## Commands

All commands are run from the repository root:

- `make lint` — builds the in-tree binary and runs `specify lint framework` for documentation and workflow consistency checks (`cargo run -q -p specify -- lint framework --framework-root .`). Only a Rust toolchain is required; the runtime lives in-tree at the repo root, so there is no source pin or sibling checkout to resolve.
- `make ci` — the full local gate: `cargo make ci` (the Rust workspace, `Makefile.toml` at the repo root) followed by `make lint` over the prose.
- `make use-local-plugins` / `make use-team-plugins` — choose plugin source (reload Cursor after either).

The `specify-standards` framework predicate regression suite lives in-tree and runs with the rest of the Rust workspace via `cargo make test`. CI is one job: `.github/workflows/ci.yaml` builds the in-tree binary, runs `cargo make ci` from the repo root, and runs `specify lint framework --framework-root .` over the prose plus a spec-runtime symlink check. See [docs/contributing/checks.md](docs/contributing/checks.md) for the check model.

Full evals guidance, including the scenario packs under [`evals/`](evals/README.md), lives in [docs/contributing/evals.md](docs/contributing/evals.md).

## Skill authoring

Skill authoring rules — markdown style, description grammar, argument-hint grammar, 200/45/512 caps, skill body discipline, cross-cutting guardrails, envelope examples — live in [docs/standards/skill-authoring.md](docs/standards/skill-authoring.md) (with the long-form rationale under `## Rationale`) and [.cursor/rules/project.mdc](.cursor/rules/project.mdc#skill-authoring-conventions). Framework checks are [`CORE-*` rules](adapters/shared/rules/core/) resolved by a generic `specify lint framework` dispatcher. Each rule is either a **declarative hint** (Road A — `kind:` ∈ `schema | reference-resolves | cardinality | set-coverage | constant-eq | unique | fenced-block | regex | path-pattern | presence | field-grammar | cross-reference | cli-contract`, interpreted over the workspace model) or a **name-resolved in-process checker** (Road B — `kind: tool`, e.g. the `rules` / `scenarios` / `skill-body` / `links-registry` / `marketplace` / `prose` family checkers in the CLI binary). All policy (caps, allow-lists, owner maps, expected sets) lives in the rule's `config:`, never in the engine. The `kind: authoring-predicate` bridge has been fully removed — no imperative bridge remains. Enforced strictly by `make lint` — every check fails on the first violation, with no per-file grandfathering. Extension model: [docs/contributing/checks.md](docs/contributing/checks.md).

## Gotchas

- In a fresh clone, run `/spec:init` before using other `/spec:*` commands. The workflow skills expect the `.specify/` project structure to exist.
- `specify lint framework` enforces documentation consistency; if you remove or rename workflow terms, update the checks in the same change.
- **Adapter names are unique across axes** — a name appears under `adapters/sources/<name>/` xor `adapters/targets/<name>/`, never both. Collisions surface as `adapter-name-axis-collision` at `specify init` and at first resolve. See [DECISIONS.md §"Adapter name uniqueness"](DECISIONS.md#adapter-name-uniqueness).
- **First-party adapters resolve project-locally or from GitHub** — `specify init <adapter>` accepts a first-party shorthand (`omnia`, `omnia@1.0.0`; a bare name resolves the single installed identity, a semver pin records the full `name@<semver>` adapter identity) that resolves to the published adapter on GitHub (a networked sparse checkout). The adapter resolver itself is project-local only — manifest-cache mirror then vendored `adapters/` tree — with no environment-variable fallback to an out-of-tree framework checkout. See [DECISIONS.md §"Adapter loader axis routing"](DECISIONS.md#adapter-loader-axis-routing) and §"First-party `<adapter>` shorthand at init".
- Target review briefs symlink `agent-teams.md` from each adapter's `references/` directory to the shared `adapters/shared/references/runtime/review-team-protocol.md` overlay, which resolves to the canonical `docs/reference/review-team-protocol.md`. If a symlink target is removed, the brief's documentation may reference content that no longer resolves.
- Crossing a major is a hard cut: no silent compatibility aliases for old manifests, verbs, brief paths, or slash-namespaces, and no migration framework. Pre-1.0, a major bump means re-init — `specify init --upgrade` bumps the pin over an existing project; anything deeper is a fresh `specify init`. See the [`DECISIONS.md`](DECISIONS.md) "Bootstrap and upgrade lifecycle" decision.

## Related coding standards

- CLI binary and crate conventions (errors, DTOs, hint colocation, brevity) live in [the Rust workspace section below](#the-rust-workspace-specify-cli) and [docs/standards/](docs/standards/). Skills that shell out to `specify` rely on the kebab-case `error` discriminants documented there.

## The Rust workspace (`specify` CLI) {#the-rust-workspace-specify-cli}

The repository root is a Rust workspace. It produces the `specify` runtime binary that the workflow skills shell out to. Generated Rust crates and Swift shells produced by the workflow live in downstream consumer repositories; this workspace owns the deterministic CLI primitives those workflows compose.

### Crate graph

The workspace is leaf → root. `specify-error` is the dependency leaf and depends on no other workspace crate.

```text
specify-error                    # leaf — thiserror + serde-saphyr only
specify-guest-model              # leaf — the local Model capability trait (WASI-backed on wasm32, MockModel off it); stand-in for the upstream omnia-guest capability, unpublished
specify-schema                   # depends on specify-error (embedded JSON Schemas + jsonschema plumbing; also owns schema::digest — SHA-256 hex via sha2 + base16ct)
specify-diagnostics              # depends on specify-{error,schema} (Diagnostic substrate: report, fingerprint, validator, renderers, blocking)
specify-model                    # depends on specify-{error,diagnostics} (artifact types + parsers: spec, task, evidence, discovery; shared atomic writer; model::validate artifact rule registry — NOT on specify-workflow or anything named lint)
specify-extension                # depends on specify-{diagnostics,schema} (WASI extension manifest DTOs + structural validation; wasmtime-free leaf)
specify-registry                 # depends on specify-{error,schema,extension} (WASI runner + OCI transport + adapter pack/store; wasmtime, gated)
specify-standards                # standards layer — depends on specify-{error,schema,diagnostics}; NOT on specify-workflow or specify-registry
specify-workflow                 # workflow layer — depends on specify-{error,schema,extension,model,diagnostics,guest-model} (also owns workflow::agents — init-time AGENTS.md context-fence generation — and workflow::judgment — the guest judgment legs); NOT on specify-standards / specify-registry (no wasmtime in its graph)
specify-dispatch                 # wasm-clean dispatch boundary — clap grammar, envelopes, exit contract, pure verb handlers; consumed by the root binary and the workflow guest shim
specify-workflow-guest           # wasm32 wasi:cli/run shim over specify-dispatch + specify-workflow (release artifact committed at crates/workflow-guest/guest.wasm and embedded by specify-runtime)
specify-echo-guest               # wasm32 skeleton source-adapter guest for the composed runtime tests
specify-runtime                  # composed-deployment host surface (CursorBundle, Hooks, drive, the embedded workflow guest); also builds the specify-runtime-replay test binary
specify (root crate)             # the one binary: triage main — native verbs in-process, guest-owned verbs through specify-runtime::drive (DECISIONS.md §"One `specify` binary")
```

`specify-standards` and `specify-workflow` are siblings: neither imports the other. The standards-layer-vs-workflow-layer split is a type-system invariant by the dependency-direction invariant in [DECISIONS.md §"Standards layer split into `specify-standards` and `specify-schema"](./DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema) (lint carries no lifecycle authority). The artifact validation rule registry lives in `specify_model::validate`: `specify-model` depends on neither `specify-workflow` nor anything named lint, so a rule cannot transition a slice or stamp a plan. `specify-model` is the lifecycle-free leaf holding the artifact types and parsers both higher layers read. Both standards and workflow depend on `specify-schema` so the embedded JSON Schemas (and the shared `schema::digest` SHA-256 helpers) live in one place. The neutral `Diagnostic` / `DiagnosticReport` substrate lives in `specify-diagnostics` (depends on `specify-{error,schema}`), so every check producer — validate and lint alike — emits the same finding currency without any non-lint producer depending on anything named `lint`. See [DECISIONS.md §"Drained `Error::Validation` and the `Diagnostic` substrate"](./DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate).

Modules of note across the workspace (workflow + standards layers):

- `crates/workflow/src/platform.rs` — closed `Platform` enum (`Core | Ios | Android | Web | Desktop`, `#[serde(rename_all = "kebab-case")]`) representing the set of target platforms a project may declare in `project.yaml`. `Core` is mandatory in every set. `Ios` and `Android` have scaffold/build/verify support; `Web` and `Desktop` are type-system placeholders for future functionality. Includes `Display`, `FromStr`, and `parse_platforms_csv` for the `--platforms` CLI flag.
- `crates/workflow/src/adapter/` — axis-split adapter loader. `SourceAdapter::resolve(adapter_ref, project_dir)` and `TargetAdapter::resolve(adapter_ref, project_dir)` take an `AdapterRef { name, version: Option<semver::Version> }` (the versioned adapter identity) and are the per-axis entry points and the only manifest loaders; each carries its closed operation set as the typed `briefs.keys()` source of truth (workflow §"Operations typed at parse boundary"). `locate_axis` probes name-only (the manifest cache, then `<project_dir>/adapters/{sources,targets}/<name>/`) — resolution is project-local only, with no environment-variable fallback to an out-of-tree framework checkout (a miss on both is `adapter-not-found`). `adapter.yaml.version` is a required semver string parsed into `SourceAdapter.version` / `TargetAdapter.version` (`semver::Version`); the `check_version` post-schema gate raises `adapter-version-malformed`, and `check_requested_version` raises `adapter-version-required` when an `AdapterRef` pin does not match the installed identity. Each manifest may also carry an optional `specify` host-CLI compatibility floor parsed into `requires_specify: Option<semver::Version>`; the `check_requires_specify` post-schema gate compares it against the running binary (`env!("CARGO_PKG_VERSION")`) and raises `Error::AdapterCliTooOld` (`adapter-cli-too-old`) on the exit-3 `EXIT_VERSION_TOO_OLD` path when the binary is older. The closed `SourceOperation` / `TargetOperation` enums in `adapter/operation.rs` are the typed `briefs.keys()` carried by each manifest struct. `TargetAdapter` also carries an optional `PlatformsCapability` (`{ required, allowed, default }`) declaring which platforms the target supports; vectis declares `required: true`.
- `crates/workflow/src/init/adapter_uri.rs` — `specify init <adapter>` argument parser. Recognises first-party **shorthand** (`omnia`, `omnia@1.0.0` — bare name resolves the single installed identity, a semver pin records the full `name@<semver>` adapter identity) and expands it to the canonical `https://github.com/augentic/specify/adapters/targets/<name>@<git-ref>` URL, deriving the git checkout ref `v<major>` from the pinned semver — alongside the existing local-path and GitHub-URL forms. It also recognises the **package reference** form (`<namespace>:<name>@<semver>`, e.g. `specify:omnia@1.2.0`) via `AdapterPackageRef`: an immutable, content-addressed registry locator with a mandatory exact-SemVer pin and no branch/tag defaulting (a missing or non-SemVer version raises `adapter-package-ref-version-required`). A recognised package reference is **installed on fetch**: the root `specify init` layer calls `recognize_package` → `registry::store::install_tofu` to pull the immutable artifact and materialize it read-only in the global content-addressed store before scaffolding, and `AdapterUri::from_package` then resolves that store entry as the local source (a missing entry is `adapter-package-not-installed`, never a mutable git fallback). The install reference derives from `registry::oci::adapter_reference` as `${SPECIFY_REGISTRY:-augentic.io}/<namespace>/<name>:<version>` — the same host/namespace the publish workflow pushes to. Install is trust-on-first-use; the store entry's read-only immutability plus a recorded sibling-meta tree digest provide local verify-on-read (D4, re-checked at resolve), while the cross-machine `project.yaml` digest pin remains deferred to a follow-up. `adapter_ref_from_value` recovers an `AdapterRef` from a recorded adapter value (stripping the `<namespace>:` prefix for package references). See [`DECISIONS.md` §"First-party `<adapter>` shorthand at init"](./DECISIONS.md#first-party-adapter-shorthand-at-init).
- `crates/registry/src/{pack,oci,store,host}.rs` — adapter packaging and transport (the `specify-registry` crate). `pack.rs` byte-deterministically packs an adapter tree into one zstd-compressed tar layer (symlinks dereferenced) content-addressed by its sha256 (`pack_adapter` / `content_digest` / `verify_digest`, `ADAPTER_LAYER_MEDIA_TYPE`, pinned `ZSTD_LEVEL`); `oci.rs` pushes / pulls that single layer as an OCI artifact under the immutable `<registry>/<repo>:<version>` reference (`push_adapter` / `pull_adapter`, on `oci-client`) — the Step-1 spike established that `wasm-pkg-client` rejects an opaque blob, so adapter transport uses the raw OCI layer path; `store.rs` is the global content-addressed adapter store keyed by the immutable `(name, version)` identity (`install_tofu` trust-on-first-use install + `install_layer`, recording a sibling `<store>/<name>@<version>.meta` tree-content digest at install) — read-only immutable entries published by atomic temp-then-rename under a sibling install lock, with `specify_schema::cache::adapter_store_entry` the shared path resolver and `specify_schema::cache::verify_store_entry` the verify-on-read gate re-checked at resolve (`crates/workflow/src/adapter/resolve.rs`, raising `adapter-digest-mismatch` for `AdapterLocation::Store`); `host.rs` is the Wasmtime WASI Preview 2 runner boundary (`RunContext`) behind `specify lint project`'s `kind: tool` evaluator. The manifest DTOs are re-exported from the wasmtime-free `specify-extension` leaf as `registry::manifest` / `registry::validate`.
- `crates/model/src/spec/provenance.rs` — `spec.md` requirement-block parser (`ID:` / `Sources:` / `Status:` lines, closed `RequirementStatus` enum, inline `[…]` tag coherence).
- `crates/workflow/src/change/plan/core/propose.rs` — plan-time lead-reconciliation kernel. Envelope DTOs (closed `kind: request | response`), the pure `build_request` / `build_catalog` / `resolve_topology` assembly, and the `Plan::propose_from` projection kernel driven by the guest `plan author` orchestration. See [`DECISIONS.md` §"Target platform capability and init validation"](./DECISIONS.md#target-platform-capability-and-init-validation) and §"Lead reconciliation".
- `crates/workflow/src/slice/build/` — target build envelope kernel. `wire.rs` holds the closed-shape `BuildRequest` / `BuildReport` DTOs (round-tripping `schemas/target/build-{request,report}.schema.json`), `BuildOutput` (`{ platform: Platform, path }` — the optional per-platform build outputs declared in `BuildReport.outputs[]`), plus the `enforce_report_no_blocking_on_success` and `enforce_report_outputs_exist` gates; `assemble.rs` assembles a request from the bound target adapter's declared `inputs[]` against the slice tree (raising `target-build-input-missing`). The guest `orchestrate::build` orchestration (behind the guest-routed `specify slice build <slice>`) owns request assembly, report validation, the `target-build-*` aborts (including `target-build-output-missing` for absent/empty output paths), the `slice.build.*` events, and the `built` transition gate.
- `crates/workflow/src/journal.rs` — newline-delimited JSON journal event log at `<project_dir>/.specify/journal.jsonl`; closed `Event` / `EventKind` taxonomy with kebab-case wire ids and `snake_case` Rust variants joined by `#[serde(rename = "…")]` (including the single `PlanReconcileCompleted` variant covering a successful `plan author` write, the eval-probe events `plan.entry.advanced` / `workspace.sync.completed` / `workspace.push.completed` with the closed `Actor` enum (`operator | agent`) on `plan.transition.approved`, plus the bootstrap events `cli.upgraded` / `plugins.refreshed`).
- `crates/workflow/src/{upgrade,plugins}.rs` — the bootstrap lifecycle (handlers in [`src/runtime/commands/{upgrade,plugins}.rs`](./src/runtime/commands)). `upgrade.rs` owns `InstallChannel::detect()` and the channel-native upgrade plan; `plugins.rs` owns Cursor plugin-cache discovery and the `doctor` / `refresh` reports. There is no migration framework: pre-1.0 majors are re-init, not migration. See [`DECISIONS.md` §"Bootstrap and upgrade lifecycle"](./DECISIONS.md#bootstrap-and-upgrade-lifecycle).
- `crates/schema/src/` — embedded JSON Schema constants (`ADAPTER_JSON_SCHEMA`, `SOURCE_JSON_SCHEMA`, `TARGET_JSON_SCHEMA`, `EXTENSION_JSON_SCHEMA`, `EXTENSION_SIDECAR_JSON_SCHEMA`, `PLAN_JSON_SCHEMA`, `EVIDENCE_JSON_SCHEMA`, `LEAD_JSON_SCHEMA`, `PROPOSAL_JSON_SCHEMA`, `SLICE_MODEL_JSON_SCHEMA`, `SYNTHESIS_JSON_SCHEMA`, `PROVENANCE_JSON_SCHEMA`, `DECISION_JSON_SCHEMA`, `TOPOLOGY_LOCK_JSON_SCHEMA`, `BUILD_REQUEST_JSON_SCHEMA`, `BUILD_REPORT_JSON_SCHEMA`, `COMPONENTS_JSON_SCHEMA`, `RULE_JSON_SCHEMA`, `RESOLVED_RULES_JSON_SCHEMA`, `DIAGNOSTIC_JSON_SCHEMA`, `DIAGNOSTIC_REPORT_JSON_SCHEMA`, `WORKSPACE_MODEL_JSON_SCHEMA`, `SKILL_JSON_SCHEMA`, `SCENARIO_JSON_SCHEMA`, `MARKETPLACE_JSON_SCHEMA`, `CONTRACT_DUMP_JSON_SCHEMA`) and the shared `jsonschema::Validator` plumbing (`compile_schema`, `validate_value`, `validate_serialisable`, `read_yaml_as_json`). Workflow, standards, and registry layers all consume schemas through this crate; nobody else embeds `include_str!`'d schema JSON. The `crates/schema/tests/schemas.rs` parity test asserts each embedded constant byte-matches its on-disk `schemas/` source.
- `crates/diagnostics/src/` — the neutral `Diagnostic` substrate: the `Diagnostic` / `DiagnosticReport` / `DiagnosticSummary` types with the orthogonal `source` (`deterministic | model-assisted | hybrid | human | tool`) and `kind` (`violation | review`) axes, the fingerprint algorithm, `validate_diagnostic`, the four renderers (`json/pretty/github/compact`), and the `blocking` predicate. Import it directly from `specify-diagnostics`.
- `crates/standards/src/rules/` — rules parser and resolver pipeline (`parse.rs`, `resolve.rs`, `resolve/{filter,sort}.rs`). Kept out of `specify-workflow` by the standards-layer split.
- **No imperative framework `Check` substrate.** Every `CORE-*` framework check resolves through the generic lint dispatcher — either a declarative hint (Road A, `lint/eval/*`) or a name-resolved in-process checker (Road B, `kind: tool`, native modules under `crates/standards/src/lint/framework_tools/`); there is no `kind: authoring-predicate` and no `specify_standards::framework` module. The repo-local Rust-quality predicates live dev-only at `tests/rust_quality.rs` behind this repo's `cargo test --test rust_quality` gate; brief path-classification lives in `crates/standards/src/lint/index/brief.rs`. Posture: [DIAGNOSTICS.md §"Steady state"](./DIAGNOSTICS.md), [DECISIONS.md §"Framework lint engine: generic dispatcher (Road A / Road B)"](./DECISIONS.md#framework-lint-engine-generic-dispatcher-road-a--road-b). Contributor model (framework repo): [docs/contributing/checks.md](https://github.com/augentic/specify/blob/main/docs/contributing/checks.md).
- `crates/standards/src/lint/` — `specify lint` and `specify lint framework` surface: `WorkspaceModel` DTOs (`model.rs`), the dual-profile indexer (`index/`), the generic per-kind hint interpreter (`eval/*`), and the shared lint runner. The engine is a rule-agnostic dispatcher: the **Road A** evaluators (`schema`, `reference-resolves`, `cardinality`, `set-coverage`, `constant-eq`, `unique`, `fenced-block`, `regex`, `path-pattern`, `presence`, `field-grammar`, `cross-reference`, `cli-contract`) interpret a declarative hint over `WorkspaceModel` facts, and **Road B** (`eval/tool.rs`) resolves a `kind: tool` hint by name — against the in-process `framework_tools` inventory first (typed findings, no round-trip), else the named WASI tool's `DiagnosticReport`. Each kind reads its mechanism selector from `hint.value` and its policy from `hint.config`: `presence` (`frontmatter` / `file` / `markdown-section` / `directory-index`) flags a missing required artifact, `field-grammar` (`field-tokens` / `field-first-word`) flags a frontmatter field that violates a token / first-word grammar, and `cross-reference` (`adapter-dir` / `expected-set` source against an `adapter-manifest` / `adapter-tool` target) is a relational set-difference / value-equality join. `cli-contract` (`invocations` / `event-ids` / `error-codes` / `test-citations`) checks documentation against the binary-injected `CliContract` DTO — the root binary builds it (the same `build_contract()` behind `specify contract dump`, including the build-time `tests/` inventory embedded by `build.rs`) and hands it to the pipeline via `PipelineConfig.cli_contract`, so the standards crate never imports `clap` or workflow types. The `schema` and `unique` kinds each carry a whole-tree `value: scenario` selector reading the `scenarios` fact family directly. No eval arm embeds rule policy — each reads its caps/sets/maps from the rule's `config:` (forwarded to Road B tools as a second positional arg); the `lint_no_embedded_policy` guard test enforces this. The renderers it returns are the neutral formatters from `specify-diagnostics`. The framework-profile extractors (`index/{skill,adapter,adapter_dir,scenario,brief}.rs`) sit beside the product pass and run when `lint::index::build(project_dir, ScanProfile::Framework, &[], &[])` is invoked; the §F1 walk driver lives in `index/framework.rs` and follows symlinks (recording both endpoints) while the product profile records-without-traverse. `specify lint framework` is the only caller of the framework profile today. The six in-process Road B checkers live beside the engine in `framework_tools/` (`scenarios`, `skill_body`, `links_registry`, `marketplace`, `prose`, `rules` + shared `support`); `framework_tools.rs` exposes the `is_framework_checker` / `run_checker` registry the `eval/tool.rs` dispatch consults before the `ToolRunner` WASI path.
- `crates/workflow/src/agents/` — init-time `AGENTS.md` context-fence generation (`specify_workflow::agents`), housed in the workflow crate so its pure logic carries unit tests. Public modules: `detect` (shallow root-marker detection), `render` (deterministic Markdown body + `Input` struct), `fences` (byte-preserving `parse_document` / `plan_agents_write` write planner), `fingerprint` (`InputCollector` + canonical aggregate digest), `lock` (`context.lock` sidecar). All `Ctx`-free; the binary's `src/runtime/commands/agents/{assemble,generate}.rs` adapt a `Ctx` into a `render::Input` and drive these modules. Carries a module-scoped `missing_docs` / `pedantic` / `nursery` allow that preserves the original (binary-internal) lint posture.

The two **adapter validators** (`contract`, `vectis`) no longer live in this repo: they moved to `augentic/specify-adapters` and are now in-guest adapter library code inside each adapter's committed `guest.wasm` — the host dispatches no adapter WASI tool (the `extension run` verb family retired at S4; see [DECISIONS.md §"Old-stack deletion"](./DECISIONS.md#old-stack-deletion-milestone-s4)). There is no in-repo `wasi-tools/` workspace, checked-in `dist/` blob, or digest drift test. The **seven framework checkers** (`scenarios`, `skill-body`, `links-registry`, `marketplace`, `prose`, `rules`, `extension`) that back the Road B `kind: tool` framework rules run **in-process** as native modules under [`crates/standards/src/lint/framework_tools/`](./crates/standards/src/lint/framework_tools.rs), inside `specify-standards`. The `kind: tool` evaluator resolves a framework checker name against that in-process inventory before the `ToolRunner` trait is consulted, calling it directly for typed `Diagnostic` findings — the trait (and its `WasiToolRunner` impl) survives only for the project-side WASI path (see [DECISIONS.md §"Framework lint engine"](./DECISIONS.md#framework-lint-engine-generic-dispatcher-road-a--road-b)). Crux shell presence and launcher-icon heuristics live **only** in the vectis adapter's in-guest core (`augentic/specify-adapters`): the host performs no plan-time shell detection, so there is no in-repo shell-detect crate.

### Exit codes

Part of the CLI wire contract. `Exit::from(&Error)` in [`src/runtime/output.rs`](./src/runtime/output.rs) is the single source of truth.

### Repository map

```text
src/runtime/          specify dispatch (the single CLI; lint project + lint framework)
crates/workflow/        workflow domain logic
tests/rust_quality/              dev-only Rust-quality predicates + gate (no CORE rule producer)
```

| Code | Name                      | When                                                                                                                   |
| ---- | ------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| 0    | `EXIT_SUCCESS`            | Command succeeded.                                                                                                     |
| 1    | `EXIT_GENERIC_FAILURE`    | Any `Error` variant not listed below (I/O, YAML, schema, merge, tool resolver/runtime, …).                             |
| 2    | `EXIT_VALIDATION_FAILED`  | Validation findings, `Error::Validation`, `Error::Argument`, or an undeclared/over-permissioned tool request.          |
| 3    | `EXIT_VERSION_TOO_OLD`    | `Error::CliTooOld` — `project.yaml.specify` is newer than the binary.                                          |

See [DECISIONS.md §"Exit codes"](./DECISIONS.md#exit-codes) for the long-form rationale (including `Exit::Code(u8)`'s WASI passthrough role).

### Documentation map

| Topic                                                                                                                                                                                 | Document                                                                                                                                                               |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Cross-cutting code-quality rules (naming, error variants, traits-for-testability, archaeology)                                                                                        | [`docs/standards/style.md`](./docs/standards/style.md)                                                                                                                 |
| Lints, comments, brevity, DTOs, YAML/atomic writes, module layout (`<module>.rs` + `<module>/`, no `mod.rs` outside `tests/`)                                                         | [`docs/standards/coding-standards.md`](./docs/standards/coding-standards.md)                                                                                           |
| `Ctx`, `Out`/`Render`/`emit`, exit-code mapping, dispatcher contract                                                                                                                  | [`docs/standards/handler-shape.md`](./docs/standards/handler-shape.md)                                                                                                 |
| Workspace layout, WASI carve-outs, `Layout<'a>`, time injection, `ureq` hardening, atomic-write rationale, workflow domain modules, supply chain                                      | [`docs/standards/architecture.md`](./docs/standards/architecture.md)                                                                                                   |
| `cargo nextest`, integration-first policy, golden files, `REGENERATE_GOLDENS`                                                                                                         | [`docs/standards/testing.md`](./docs/standards/testing.md)                                                                                                             |
| Standing architectural decisions (error layering, exit codes, atomic writes, YAML library, wire compatibility, workflow type renames, plan lifecycle, adapter loader, journal events) | [`DECISIONS.md`](./DECISIONS.md)                                                                                                                                       |
| Engineering standards layer (`specify-standards` / `specify-schema`, `WorkspaceModel`, deterministic hints, `specify lint`)                                                           | [`DECISIONS.md` §"Standards layer split into `specify-standards` and `specify-schema`](./DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema) |
| Vectis asset materialization                                                                                                                      | [`augentic/specify` `rfcs/roadmap.md`](https://github.com/augentic/specify/blob/main/rfcs/roadmap.md#current-priorities) (**Recently implemented**); in-guest build prelude and asset-domain policy live in the vectis core (`augentic/specify-adapters`) — the engine dispatches no prepare hook (S4) |

### Rust quality {#rust-quality}

**Aggressive Integration-First Posture:**
Specify mandates an aggressive integration-first test strategy. Agents must actively work to remove unit tests (`#[cfg(test)]`) in favor of crate-level (`crates/<name>/tests/`) and binary integration tests (`tests/<area>.rs`).
- **Design against the public surface first:** before adding a unit test, ask whether integration can reach the behavior — is it reachable through a CLI input or a `pub` fn, is its effect observable at a public boundary (stdout JSON, exit code, filesystem), and is that affordable without a subprocess-pool explosion? If yes, write the integration test; the unit test is redundant.
- **Default to deletion:** a `src` unit test survives only when it is reachable and observable but cheap *only* in-process against a **private** kernel (a proptest or dense matrix), or it covers a genuinely CLI-unreachable branch. If the kernel is already `pub`, relocate the test to `crates/<name>/tests/` rather than leaving it in `src`.
- **Do NOT widen public APIs to test a private kernel.** Widening trades durable surface stability for coverage you already have; prefer collapse-and-keep. The target is *near-zero* `src` unit tests — no redundant or integration-reachable ones — not literal zero. Use `cargo llvm-cov nextest` to prove coverage holds when removing unit tests.
- **Push crate-specific tests down:** `tests/` at the root of the workspace is for E2E workflows. Crate-specific logic must be tested in `crates/<name>/tests/` via the crate's public API.

Read [style.md](./docs/standards/style.md), [coding-standards.md](./docs/standards/coding-standards.md), and [testing.md § Test naming](./docs/standards/testing.md#test-naming) before adding types, suppressions, or tests. Run `cargo make ci` (not bare `cargo test` — CI uses `RUSTFLAGS=-Dwarnings`).

**Naming:** The module path is context — `registry::show`, not `show_registry`. Test function names are short identifiers; put the narrative in the test body ([testing.md](./docs/standards/testing.md#test-naming)).

**Lint suppressions:** Refactor first. Use `#[expect(lint, reason = "…")]` at the smallest scope. `#![allow]` only at module root when the lint applies to every item below and the reason is contract-locked. `#[allow]` without `reason` fails CI.

**Rust-quality CI:** `cargo test --test rust_quality` runs the dev-only predicates in `tests/rust_quality.rs` over this repo (long test fn names, archaeology in `//!`/`///`, bare `#[allow]`, workflow clock reads) plus the **src unit-test ratchet** `unit_test_budget_holds`, which holds each crate's `src` `#[test]` / `#[tokio::test]` count to the committed budget in [`tests/rust_quality_budget.toml`](./tests/rust_quality_budget.toml): adding a `src` unit test fails CI unless the budget is raised with a review justification, and removing one fails until the budget is ratcheted down. See [testing.md](./docs/standards/testing.md).

| Do not                                                    | Do instead                                                        | See                                                                                                   |
| --------------------------------------------------------- | ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| `#[allow]` / `#[expect]` before trying a split or extract | Extract helper or submodule; suppress only if contract-locked     | [coding-standards § Lint suppression](./docs/standards/coding-standards.md#lint-suppression-posture)  |
| `trait Foo` + sole `RealFoo` for tests                    | `CmdRunner`, `AtomicYaml`, or filesystem/tempdir                  | [style.md § No traits for testability](./docs/standards/style.md#no-traits-for-testability-alone)     |
| `*RenderInput` wrapper for `Render`                       | `Render` on domain type or `ctx.emit_with` closure                | [style.md § One body per command](./docs/standards/style.md#one-body-per-command-no-wrapper-newtype)  |
| `match ctx.format { Json, Text }` in handlers             | `ctx.write` / `output::report`                                    | [handler-shape.md](./docs/standards/handler-shape.md)                                                 |
| RFC/Phase/migration history in `//!` / `///`              | ≤ 3 lines “what today”; history in [DECISIONS.md](./DECISIONS.md) | [style.md § No archaeology](./docs/standards/style.md#no-archaeology-in-code)                         |
| Sentence-length test fn names                             | Short name + `mod` grouping                                       | [testing.md § Test naming](./docs/standards/testing.md#test-naming)                                   |
| Add a `src` `#[cfg(test)]` for CLI-reachable behavior     | Exercise it through the public surface in `crates/<name>/tests/`  | [testing.md § minimize the unit layer](./docs/standards/testing.md#the-three-layers--minimize-the-unit-layer) |
| Nested `struct Body` inside `fn`                          | Top-level `*Body` + `From` impl                                   | [coding-standards § DTOs](./docs/standards/coding-standards.md#dtos)                                  |
| New `Error::Diag` for one-off shapes                      | Typed variant after ≥3 identical call sites                       | [style.md § Error variants](./docs/standards/style.md#error-variants-budgeted-by-recovery-not-source) |

External references:

- [Vocabulary](#vocabulary) at the top of this file — workflow vocabulary (slice / change), skill family, plan-driven loop, contract skills.
- [`docs/standards/workflow.md`](./docs/standards/workflow.md) — the in-force workflow contract this binary implements. Defines the `source` / `target` / `plugin` / `axis` vocabulary, the kebab-case wire format, the `Source` / `Lead` / `Evidence` / `Slice` implementation types, writer ownership, and the CLI surface. Stable `§`-anchors that source comments and skill briefs cite by name.
- [`docs/release.md`](./docs/release.md) — tagging and the platform-binary release pipeline.
- [`schemas/`](./schemas/) — JSON Schema files distributed with the binary (`adapter.schema.json`, `source.schema.json`, `target.schema.json`, `evidence.schema.json`, `discovery/lead.schema.json`, `plan/plan.schema.json`, `target/build-request.schema.json`, and `target/build-report.schema.json`); the workflow contract pins each shape.

### Quick toolchain

All driven by `cargo make` (see [`Makefile.toml`](./Makefile.toml)). Run the full local CI suite before committing; do not rely on narrower substitutes such as `cargo test` or `cargo clippy`.

```bash
cargo make ci             # fmt-check + lint + test + test-docs + doc + vet + deny
cargo make check          # fmt + lint + test + test-docs (the pre-commit subset)
cargo make test           # cargo nextest run --all --all-features --no-tests=pass under -Dwarnings
cargo make lint           # cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo make fmt            # nightly cargo fmt --all
cargo make audit          # cargo-audit; cargo make deny / outdated / deps / vet for the rest
```

Less frequent recipes:

```bash
scripts/regen-wasm-fixtures.sh   # regenerate the checked-in WASI fixtures under tests/fixtures/tools-test-*/wasm/
```

### When working in the Rust workspace

1. Read [`DECISIONS.md`](./DECISIONS.md) before changing error layering, exit codes, atomic writes, the YAML library, the JSON envelope shape, the workflow type names (`Target*` / `Plugin` / `SliceSourceBinding` / `Divergence`), the plan lifecycle (`pending | approved`), the journal event taxonomy, the per-axis cache layout, or adding a new workspace crate.
2. For any Rust change, consult [`docs/standards/`](./docs/standards/) — at minimum the doc that matches the area you are editing, plus [`style.md`](./docs/standards/style.md) for cross-cutting rules.
3. Run `cargo make ci` before committing. If it cannot run, say exactly why and which checks were run instead.
4. When you remove a symbol, `rg <SymbolName> -- AGENTS.md DECISIONS.md docs/` and update every hit in the same PR.
5. If you touch `Slice.target`, `SliceSourceBinding`, `Divergence`, `crates/model/src/spec/provenance.rs`, `crates/workflow/src/adapter/`, `crates/workflow/src/change/plan/core/propose.rs`, `crates/workflow/src/journal.rs`, `crates/schema/src/`, `crates/standards/src/rules/`, `crates/standards/src/lint/`, the `$CAPABILITY_DIR` env var, or the `adapter--<axis>--<slug>` tool cache scope: `rg <symbol>` across the whole repo — Rust *and* prose (`plugins/`, `docs/`, `adapters/`) — and the sibling [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters) checkout, and update every hit in the same PR (workflow §"Note to the implementing agent" applies — the workflow contract spans both repos).
6. A fresh contributor should be able to reach any rule from this spine in three hops or fewer. If you find yourself adding prose here that isn't navigational, it belongs in one of the standards docs.
7. For Rust changes, skim [Rust quality](#rust-quality) before adding types, suppressions, or tests; if you add `#[expect]`, state in the PR why a refactor was infeasible.
