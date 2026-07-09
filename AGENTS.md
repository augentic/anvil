# Augentic Plugins - Agent Instructions

This repository is **Rust plus embedded prose**: the workspace at the repository root produces the `specify` runtime binary, and the surviving markdown (skill wrappers, reference docs, adapter prose) ships alongside or embedded in it. Generated Rust crates and Swift shells appear in downstream projects, not in this repository itself.

## Vocabulary

Specify names two adapter roles and three workflow nouns. Use the terms verbatim.

### Adapter roles

- **source adapter** — input role with two operations: `survey` (plan time) and `extract` (slice time). Ships as a single WebAssembly component exporting the WIT `source` interface (one component, no manifest); the guest crate lives at `sources/<name>/` in `augentic/specify-adapters`. Examples: `intent`, `documentation`, `typescript`, `screenshots`, `captures`.
- **target adapter** — output role with three operations: `guidance` (read by core synthesis), `build`, and `merge`. Ships as a single WebAssembly component exporting the WIT `target` interface; the guest crate lives at `targets/<name>/` in `augentic/specify-adapters`. Examples: `omnia`, `vectis`, `contracts`. See [`docs/explanation/adapter-anatomy.md`](docs/explanation/adapter-anatomy.md) for the full source / target contract, including the [adapter-vs-Cursor-plugin manifest boundary](docs/explanation/adapter-anatomy.md#adapter-manifests-vs-cursor-plugin-manifests).
- **plugin** — historical shorthand for the shared adapter shape. The Rust loaders are `SourceAdapter::resolve(adapter_ref, project_dir)` and `TargetAdapter::resolve(adapter_ref, project_dir)` in [`crates/workflow/src/adapter/`](crates/workflow/src/adapter); each locates the identity's single `.wasm` component and reads its metadata from the component's own `describe` export (no manifest file, no schema validation). The noun "plugin" survives in operator-facing prose where source + target authors share the same audience tag.

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
- **change** — the operator-defined umbrella that coordinates one or more slices through `change.md` + `plan.yaml`. Driven by `/spec:plan`, `specify plan execute`, `/spec:finalize` and the `specify plan *` CLI verbs. `change` is on-disk vocabulary, not a slash-command namespace.

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
| **Workflow**              | Phase orchestration and lifecycle transitions | `/spec:plan`, `specify plan execute`, `specify slice transition`                    |
| **Artifacts**             | Slice-local and baseline product intent       | `spec.md`, `plan.yaml`, `.specify/specs/`                                           |
| **Engineering standards** | Durable policy that outlives any slice        | Rules under `codex/rules/` and per-adapter `prose/rules/` overlays; `specify rules export` |

**Authoring standards** (`docs/standards/`, enforced by the framework-quality cargo tests at `tests/framework_quality/` on this repo) govern skill and doc house style. **Engineering standards** (rules in `augentic/specify-adapters` — `codex/rules/universal/` and per-adapter `prose/rules/` overlays, exported by `specify rules export`) govern generated and hand-written code in consumer projects. Do not conflate them.

`specify lint project` retired from the operational surface (YAGNI — if it earns its way back, it returns as developer tooling); engineering standards reach consumer projects through `specify rules export`. Build-time `REVIEW.md` and plan Gate 1 `approved` are separate surfaces.

### Authority and reconciliation mechanics

The full mechanics — per-slice operator overrides, inline provenance shape — live in [`DECISIONS.md`](DECISIONS.md). The headline rules:

- **Authority resolution order** — per-slice override → Evidence document-level `authority:` → conflict. (A per-Evidence per-kind override is deferred to a future RFC.) See [`plugins/spec/references/synthesis/authority.md`](plugins/spec/references/synthesis/authority.md) for the resolution order and override surface.
- **`captures` source adapter** — consumes runtime capture trees and emits `kind: example` Evidence claims with `replay-digest: sha256:…` anchors and default `authority: behaviour`.
- **Authority-override authoring** — `specify plan amend --authority-override <slice> <kind>=<key>`; orphan source keys are rejected by `specify slice validate` with `slice-authority-override-orphan-source`.
- **Reconciliation checks** — `specify slice validate` catches spec-vs-model staleness and orphan contributing claims; provenance is carried inline in `model.yaml` so there is no separate file to drift.
- **Extraction is agent-only and never cached** — `survey` / `extract` re-run the prompt every time; there is no extraction-result cache.

## Workflow overview

The default rhythm is `/spec:plan` → operator stamps `approved` → `specify plan execute` → `/spec:finalize`. The operator surface, in the order it appears in a project's life:

- `/spec:init` — scaffold `.specify/`, run once per project.
- `/spec:plan` — wrap the guest-routed `specify plan author`: survey each bound source, reconcile leads into `slices[]`, author the Gate 1 prose, validate. Exits at `plan.lifecycle: pending` and prints the literal `specify plan transition <name> approved` command.
- `specify plan transition <name> approved` — **Gate 1.** Operator-only stamp; `/spec:plan` never writes `approved` itself.
- `specify plan execute` — the guest-routed drained loop; refuses unless the plan is `approved`, then runs refine → build → merge per entry until every per-entry `status` is `done` or a stop condition halts it.
- `/spec:refine` — breakout: wrap `specify slice refine` for one slice (extract per bound source, synthesis, validation, the `refined` transition).
- `/spec:build` — breakout: wrap `specify slice build` for one slice.
- `/spec:merge` — breakout: wrap `specify slice merge run` for one slice; the only writer of per-entry `done`.
- `/spec:drop` — abandon a slice without merging (`specify slice drop`).
- `/spec:finalize` — push branches, then run `specify plan archive`. Opening and merging pull requests is operator-owned and happens outside Specify.

N=1 is degenerate, not special: `intent.survey` produces one lead, the operator stamps `approved`, and `specify plan execute` drives the same single-slice rhythm as a 12-slice change.

## Skill / CLI responsibility split

Phase skills are ultrathin invoke-and-relay wrappers: each elicits any missing arguments, invokes the corresponding `specify` command, and relays its output verbatim. Everything else — manifest validation, `metadata.yaml` reads and writes, plan and slice lifecycle transitions, source and target resolution, artifact-completion checks, baseline conflict detection, delta merge, archive move, and the judgment legs (survey, extract, synthesis, target build) — runs through the `specify` CLI and its guest orchestrations. No skill body carries orchestration, synthesis, or validation prose.

The CLI surface skills depend on is documented in [`specify` `--help`](cli). The headline groups: `specify init` (with the re-entry flag `--upgrade`, which bumps the `specify` pin and re-scaffolds preservation-safe files only, and `--platforms <csv>`, which declares the project's target platform set — required when the target adapter declares `platforms.required`), `specify source {resolve, survey, extract}`, `specify target {resolve}`, `specify slice {create, refine, model show, build, transition, validate, provenance, merge}`, `specify plan {create, author, execute, add, amend, transition, next, status, archive}` (`plan status` is the read-only next-action projection — `refine|build|merge <slice>` / `stop <reason>` / `drained` — over plan entries, slice metadata, and the journal tail), `specify archive {prune}` (retention-policy GC over the prunable slice/plan archive), `specify workspace {sync, push, prepare}`, `specify adapters {sync}` (the explicit hydration trigger — hydrate every declared pinned identity into the global adapter store; `--frozen` turns a store miss into the typed `adapter-not-installed` instead of fetching), `specify upgrade` (channel-aware CLI self-update), `specify plugins {doctor, refresh}` (Cursor plugin-cache drift report and invalidation), and `specify journal {emit, show}` (`emit` — the guarded front door onto the closed journal taxonomy for agent-orchestrated phases; `show` — the read-only `--filter`/`--limit` projection over the journal). `specify source survey`/`extract` resolve `<source>` against `plan.yaml.sources.<key>` and run the bound source adapter's compiled-in prompt through one guest orchestration each (source extraction is agent-only). `specify slice build <slice>` is the guest-routed target-build verb: the orchestration assembles + schema-validates the build request, emits `target.execution.agent`, drives the target build prompts, validates the report, and owns the `built` transition, journaling `slice.build.started` / `.succeeded` / `.failed`. `specify slice merge` fires `slice.merge.started` / `.succeeded` / `.failed` on its validator outcome (not on a merge report) alongside the durable `slice.archive.created`.

Never hand-edit `metadata.yaml`, `project.yaml`, `plan.yaml`, `discovery.md`, `sources.yaml`, or `targets.yaml`; never `mkdir -p .specify/...`; never `mv` anything into `.specify/archive/`. Route through the CLI — it enforces the legal lifecycle set and validates inputs in one place for humans, agents, and CI.

## Contracts target adapter

The contracts target adapter owns API contract authoring, import, and validation. Its build operation runs the OpenAPI, AsyncAPI, and JSON Schema format sub-flows, each with author / import / verify references under `targets/contracts/prose/references/` in `augentic/specify-adapters`.

The matching validation surface is the contracts adapter's in-guest validator, run by the target build and merge orchestrations.

## Vectis asset materialization

Vectis-bound projects commit per-platform exports under `design-system/assets/exports/`; shell writers render by `assets.yaml` entry `kind` — never substitute platform glyphs for `vector` / `raster` ids at build time. See [`rfcs/roadmap.md`](rfcs/roadmap.md#recently-implemented) (**Recently implemented**).

| Concern | Where |
| ------- | ----- |
| Materialize + export conventions | In-guest vectis asset materialization — codified in the vectis core's decisions in [`specify-adapters`](https://github.com/augentic/specify-adapters) |
| Build-prelude auto-materialize | The vectis guest's build prelude — runs automatically inside the guest-routed `specify slice build`; no engine-side prepare hook remains |
| Render-by-`kind` review rule | [`targets/vectis/prose/rules/VECTIS-006-asset-render-by-kind.md`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/rules/VECTIS-006-asset-render-by-kind.md) |
| Writer / integration contracts | [`targets/vectis/prose/references/ios/design-system-integration.md`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/references/ios/design-system-integration.md), [`android/design-system-integration.md`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/references/android/design-system-integration.md), [`prompts/build/ios/write.md`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/prompts/build/ios/write.md), [`prompts/build/android/write.md`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/prompts/build/android/write.md) |

## Plan-driven loop

`/spec:plan` authors the plan and exits at Gate 1; the operator stamps `approved`; `specify plan execute` drives the loop; `/spec:finalize` closes it. Plan *entries* are only ever written via `specify plan add` / `specify plan amend`; plan *lifecycle* is only ever written via `specify plan transition`; per-entry `in-progress` is only ever written by `specify plan next`; per-entry `done` is only ever written by `specify slice merge`. Per-entry status walks backwards only via `specify plan transition <entry> --undo`, which refuses to skip rungs (`done → in-progress`, then a second call for `in-progress → pending`) and fires one `plan.transition.undone` journal event per rung. The phase skills themselves stay unaware of the plan — they operate slice-by-slice. Hand-driven fallback: `specify plan next` → `/spec:refine` → `/spec:build` → `/spec:merge`, repeat until drained.

## Testing Philosophy

Specify strictly enforces an **aggressive integration-first posture**. 

- **Design against the public surface:** before adding a unit test, ask whether integration can reach the behavior — reachable through a CLI input or `pub` fn, observable at a public boundary (stdout JSON, exit code, filesystem), and affordable to assert there without a subprocess explosion. If yes, write the integration test; the unit test is redundant.
- **Default to Deletion:** a `src` unit test survives only when it is reachable and observable but cheap *only* in-process against a **private** kernel (a proptest or dense matrix), or covers a genuinely CLI-unreachable branch. If the kernel is already `pub`, re-home the test to `crates/<name>/tests/` instead.
- **Crate-Level Integration:** Put tests in `crates/<name>/tests/` instead of the root `tests/` when they test isolated domain logic that does not require full CLI orchestration. End-to-end and purely CLI-focused tests belong in the root `tests/`.
- **Widening is a last resort:** do NOT alter public APIs simply to support integration tests — prefer collapse-and-keep. The target is *near-zero* unit tests (no redundant or integration-reachable ones), not literal zero. A ratchet gate enforces this in the engine repo (`tests/rust_quality.rs`); adapter posture is enforced by the WIT contract plus each adapter crate's `tests/` suite and the adapters repo's composed-deployment tests (the `composed` test target in `evals/`). `cargo llvm-cov nextest` remains the brake that ensures coverage holds during migrations.

## Commands

All commands are run from the repository root:

- `cargo test --test framework_quality` — the documentation and workflow consistency checks over the prose and manifest surfaces (plain cargo tests at `tests/framework_quality/`). Only a Rust toolchain is required.
- `make ci` — the full local gate: `cargo make ci` (the Rust workspace, `Makefile.toml` at the repo root), which includes the framework-quality and Rust-quality test suites.
- `make use-local-plugins` / `make use-team-plugins` — choose plugin source (reload Cursor after either).

CI is one job: `.github/workflows/ci.yaml` checks out this repo plus `augentic/specify-adapters` (the `SPECIFY_ADAPTERS` checkout feeds the universal rules pack embed) and runs `cargo make ci` from the repo root. See [docs/contributing/checks.md](docs/contributing/checks.md) for the check model.

Full evals guidance, including the scenario packs under [`evals/`](evals/README.md), lives in [docs/contributing/evals.md](docs/contributing/evals.md).

## Skill authoring

Skill authoring rules — markdown style, description grammar, argument-hint grammar, 200/45/512 caps, skill body discipline, cross-cutting guardrails, envelope examples — live in [docs/standards/skill-authoring.md](docs/standards/skill-authoring.md) (with the long-form rationale under `## Rationale`) and [.cursor/rules/project.mdc](.cursor/rules/project.mdc#skill-authoring-conventions). Framework checks are plain cargo tests at [`tests/framework_quality/`](tests/framework_quality/) — policy as module constants, failures as test failures, with no per-file grandfathering. Enforced by `cargo make ci`. Extension model: [docs/contributing/checks.md](docs/contributing/checks.md).

## Gotchas

- In a fresh clone, run `/spec:init` before using other `/spec:*` commands. The workflow skills expect the `.specify/` project structure to exist.
- The framework-quality tests (`cargo test --test framework_quality`) enforce documentation consistency; if you remove or rename workflow terms, update the checks in the same change.
- **Adapter names are unique across axes** — a name appears under `sources/<name>/` xor `targets/<name>/`, never both. Collisions surface as `adapter-name-axis-collision` at `specify init` and at first resolve. See [DECISIONS.md §"Adapter name uniqueness"](DECISIONS.md#adapter-name-uniqueness).
- **First-party adapters resolve from the registry or the dev sibling** — `specify init <adapter>` accepts a package reference (`specify:omnia@1.0.0`) or the first-party shorthand (`omnia`, `omnia@1.0.0`). A semver pin is registry sugar: it installs the published component into the global single-file store (`<store-root>/<name>@<version>.wasm`). A bare name is the development shorthand: it resolves the sibling/in-repo release build at `target/wasm32-wasip2/release/<name>.wasm` (built by `cargo make release` in the adapters repo). GitHub URLs are refused (`adapter-github-uri-unsupported`) — a source checkout no longer yields a usable adapter artifact. See [DECISIONS.md §"One component, no manifest"](DECISIONS.md#one-component-no-manifest-milestone-r64-s).
- Target review prompts in `augentic/specify-adapters` symlink `agent-teams.md` from each adapter's `references/` directory to that repo's shared `codex/references/runtime/review-team-protocol.md` overlay, forked from the canonical `docs/reference/review-team-protocol.md` here. If the canonical document is removed, the prompt's documentation may reference content that no longer resolves (guarded by the canonical-document presence check in `tests/framework_quality/prose.rs`).
- Crossing a major is a hard cut: no silent compatibility aliases for old manifests, verbs, prompt paths, or slash-namespaces, and no migration framework. Pre-1.0, a major bump means re-init — `specify init --upgrade` bumps the pin over an existing project; anything deeper is a fresh `specify init`. See the [`DECISIONS.md`](DECISIONS.md) "Bootstrap and upgrade lifecycle" decision.

## Related coding standards

- CLI binary and crate conventions (errors, DTOs, hint colocation, brevity) live in [the Rust workspace section below](#the-rust-workspace-specify-cli) and [docs/standards/](docs/standards/). Skills that shell out to `specify` rely on the kebab-case `error` discriminants documented there.

## The Rust workspace (`specify` CLI) {#the-rust-workspace-specify-cli}

The repository root is a Rust workspace. It produces the `specify` runtime binary that the workflow skills shell out to. Generated Rust crates and Swift shells produced by the workflow live in downstream consumer repositories; this workspace owns the deterministic CLI primitives those workflows compose.

### Crate graph

The workspace is leaf → root. `error` is the dependency leaf and depends on no other workspace crate.

```text
error                    # leaf — thiserror + serde-saphyr only
schema                   # depends on error (embedded JSON Schemas + jsonschema plumbing; also owns schema::digest — SHA-256 hex via sha2 + base16ct)
diagnostics              # depends on {error,schema} (Diagnostic substrate: report, fingerprint, validator, renderers, blocking)
artifacts                # depends on {error,diagnostics} (artifact types + parsers: spec, task, evidence, discovery; shared atomic writer; artifacts::validate artifact rule registry — NOT on workflow or anything named lint)
standards                # standards layer — depends on {error,schema,diagnostics}; NOT on workflow
workflow                 # workflow layer — depends on {error,schema,artifacts,diagnostics,omnia-guest} (also owns workflow::agents — init-time AGENTS.md context-fence generation — workflow::judgment — the guest judgment legs over the upstream omnia_guest::Model capability (WASI-backed on wasm32; native tests bind the scripted mock in each test binary) — and config::tools, the parse-clean project-scope tools[] DTOs); NOT on standards (no wasmtime in its graph)
dispatch                 # wasm-clean dispatch boundary — clap grammar, envelopes, exit contract, pure verb handlers; shared by the native binary and the specify guest
specify                  # wasm32 wasi:cli/run shim over dispatch + workflow (the core guest — published as specify:core@<binary version> on release; no committed artifact)
runtime                  # dev-only: the runtime-replay binary (the same runtime! macro over ModelDefault, for CI) + the composed-deployment integration tests
examples (top-level)     # example guest components, omnia's examples/ pattern: the echo adapter fixtures (skeleton source/target guests as cdylib examples) the composed tests load
specify-cli (root crate) # the one binary: a single omnia::runtime! command-mode invocation over the cursor-bound backends — no Specify vocabulary; every verb runs in the specify guest
```

`standards` and `workflow` are siblings: neither imports the other. The standards-layer-vs-workflow-layer split is a type-system invariant by the dependency-direction invariant in [DECISIONS.md §"Standards layer split into `standards` and `schema"](./DECISIONS.md#standards-layer-split-into-standards-and-schema) (the review surface carries no lifecycle authority). The artifact validation rule registry lives in `artifacts::validate`: `artifacts` depends on neither `workflow` nor `standards`, so a rule cannot transition a slice or stamp a plan. `artifacts` is the lifecycle-free leaf holding the artifact types and parsers both higher layers read. Both standards and workflow depend on `schema` so the embedded JSON Schemas (and the shared `schema::digest` SHA-256 helpers) live in one place. The neutral `Diagnostic` / `DiagnosticReport` substrate lives in `diagnostics` (depends on `{error,schema}`), so every check producer — validate and review alike — emits the same finding currency without importing the other surface's code. See [DECISIONS.md §"Drained `Error::Validation` and the `Diagnostic` substrate"](./DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate).

Modules of note across the workspace (workflow + standards layers):

- `crates/workflow/src/platform.rs` — closed `Platform` enum (`Core | Ios | Android | Web | Desktop`, `#[serde(rename_all = "kebab-case")]`) representing the set of target platforms a project may declare in `project.yaml`. `Core` is mandatory in every set. `Ios` and `Android` have scaffold/build/verify support; `Web` and `Desktop` are type-system placeholders for future functionality. Includes `Display`, `FromStr`, and `parse_platforms_csv` for the `--platforms` CLI flag.
- `crates/workflow/src/adapter/` — axis-split adapter resolver (one component, no manifest). `SourceAdapter::resolve(adapter_ref, project_dir)` and `TargetAdapter::resolve(adapter_ref, project_dir)` take an `AdapterRef { name, version: Option<semver::Version> }` (the versioned adapter identity) and resolve it to exactly one `.wasm` component: a pinned identity resolves the global single-file store entry (`<store-root>/<name>@<version>.wasm`, where the store root is `$SPECIFY_ADAPTER_STORE`, else `$HOME/.specify/adapters`; D4 verify-on-read against the recorded byte digest, `adapter-digest-mismatch` on drift); a bare name resolves the project component cache (`<project-cache>/components/<name>.wasm`, mirrored at init from an operator-supplied local file) then the sibling/in-repo development release build (`target/wasm32-wasip2/release/<name>.wasm`, built by `cargo make release` in the adapters repo). A miss on every probe is `adapter-not-found`. Metadata (the `specify` host-CLI compatibility floor, a target's `inputs[]` and `PlatformsCapability`) comes from the component's own deterministic `describe` export via `adapter/describe.rs`: host-side dispatch, instance-per-call, registered as a process-global runner by the root binary (`workflow` stays wasmtime-free), cached against the component file's SHA-256 in a `<component>.describe.json` sidecar. A wrong-axis component is the typed `adapter-axis-mismatch`; an unparseable floor is `adapter-floor-malformed`; a floor newer than the running binary raises `Error::AdapterCliTooOld` (`adapter-cli-too-old`) on the exit-3 `EXIT_VERSION_TOO_OLD` path. The closed `SourceOperation` / `TargetOperation` enums in `adapter/operation.rs` remain the typed per-axis operation sets derived from the WIT contract.
- `crates/workflow/src/init/adapter_uri.rs` — `specify init <adapter>` argument parser. Recognises the **package reference** form (`<namespace>:<name>@<semver>`, e.g. `specify:omnia@1.2.0`) via `AdapterPackageRef`: an immutable, content-addressed registry locator with a mandatory exact-SemVer pin and no branch/tag defaulting (a missing or non-SemVer version raises `adapter-package-ref-version-required`). A recognised package reference is **installed on fetch**: the root `specify init` layer calls `recognize_package` → `registry::store::install_tofu` to pull the published component through the wasm-pkg transport and materialize it read-only at the global single-file store entry before scaffolding; `AdapterUri::from_package` then resolves that store entry as the local component (a missing entry is `adapter-package-not-installed`). The first-party **shorthand** splits: `omnia@1.0.0` is package-reference sugar (installs `specify:omnia@1.0.0`); bare `omnia` is the development shorthand resolving the sibling/in-repo release build (`target/wasm32-wasip2/release/omnia.wasm`). GitHub URLs are refused with `adapter-github-uri-unsupported` — a source checkout yields no usable artifact. Only an operator-supplied local `.wasm` file is mirrored into the project component cache (`<project-cache>/components/<name>.wasm`); store entries and dev builds are read in place. `adapter_ref_from_value` recovers an `AdapterRef` from a recorded adapter value (stripping the `<namespace>:` prefix for package references). See [`DECISIONS.md` §"One component, no manifest"](./DECISIONS.md#one-component-no-manifest-milestone-r64-s).
- `crates/workflow/src/hydrate.rs` — standalone-deployment hydration kernel. `collect_refs(project_dir)` gathers every pinned identity a project declares — the `project.yaml.adapter` pin, the optional `project.yaml.adapters:` prefetch list (an unpinned entry is the typed `adapter-prefetch-unpinned`), and `plan.yaml` source pins — deduplicated on `(name, version)`; bare names stay project-local and never hydrate. `hydrate(project_dir, refs, frozen, fetch)` probes the global store per identity, pulls on miss through the injected fetch leg (no caller wires one today — the native provisioning surface retired with the wasm-pkg transport crate, so hydration is store-probe-only until a fetch leg lands in-guest; `workflow` stays wasmtime- and network-free), verifies the recorded sidecar digest (`adapter-digest-mismatch` on drift), and returns the `ResolvedAdapter` set (`name`, `version`, store entry path, `sha256:` digest) — rich enough for the follow-on deployment-manifest stage. The committed cross-machine digest pin lives at `.specify/adapters.lock` (`hydrate::lock::AdaptersLock` — versioned YAML mapping `<name>@<version>` to its `sha256:<hex>` component-byte digest, sorted and machine-written): each resolved entry is verified against the lock when it carries the identity (drift is `adapter-digest-mismatch` naming both digests) and appended atomically when it does not; undeclared entries are left in place. The verification pair (`verify_resolved` + the read-only `verify_locked`) is shared with the binary's drive-time deployment discovery, so the lock gate holds on every manifest-producing path, not just the provisioning triggers. `frozen: true` turns a miss into the typed `adapter-not-installed` naming the identity and the literal `specify adapters sync` command, and never appends to or writes the lock (strictly read-only); no prompt exists at or below the kernel. `specify init` (and `--upgrade`) runs the kernel over the positional adapter plus the `project.yaml` declared set; the `adapters sync` verb is a follow-on stage. Both triggers prepend the core identity `specify:core@<the binary's own version>` when no development override resolves ([DECISIONS.md §"Core versioned by the binary"](./DECISIONS.md) — the kernel itself stays version-agnostic).
- `crates/workflow/src/deploy.rs` — standalone-deployment manifest generator: the pure `generate(project_dir, core, adapters)` renders the manifest (`[[guest]]` per `DeployGuest` — pinned store entries and project-local bare-name components alike — plus the core guest's link allow-list, the writable `"."` mount, `/mcp/<name>` routes, in-process transport) atomically into `<project-cache>/deployment/omnia.toml` after verifying every referenced component exists (a dangling pinned entry is the typed `adapter-not-installed` naming the identity and the literal sync command). No caller regenerates it today — the provisioning triggers (`specify init` hydration, `specify adapters sync`) retired with the native surface and return in-guest; a project-root `omnia.toml` still wins wholesale. The global single-file adapter store layout (`<store-root>/<name>@<version>.wasm` + the `<name>@<version>.meta` SHA-256 sidecar that `schema::cache::verify_store_entry` re-checks at resolve) lives in `schema`'s `cache` module; the wasm-pkg transport that used to populate it (the `specify-registry` crate) is deleted, so the store is resolve-only until an in-guest fetch leg lands.
- `crates/artifacts/src/spec/provenance.rs` — `spec.md` requirement-block parser (`ID:` / `Sources:` / `Status:` lines, closed `RequirementStatus` enum, inline `[…]` tag coherence).
- `crates/workflow/src/change/plan/core/propose.rs` — plan-time lead-reconciliation kernel. Envelope DTOs (closed `kind: request | response`), the pure `build_request` / `build_catalog` / `resolve_topology` assembly, and the `Plan::propose_from` projection kernel driven by the guest `plan author` orchestration. See [`DECISIONS.md` §"Target platform capability and init validation"](./DECISIONS.md#target-platform-capability-and-init-validation) and §"Lead reconciliation".
- `crates/workflow/src/slice/build/` — target build envelope kernel. `wire.rs` holds the closed-shape `BuildRequest` / `BuildReport` DTOs (round-tripping `schemas/target/build-{request,report}.schema.json`), `BuildOutput` (`{ platform: Platform, path }` — the optional per-platform build outputs declared in `BuildReport.outputs[]`), plus the `enforce_report_no_blocking_on_success` and `enforce_report_outputs_exist` gates; `assemble.rs` assembles a request from the bound target adapter's declared `inputs[]` against the slice tree (raising `target-build-input-missing`). The guest `orchestrate::build` orchestration (behind the guest-routed `specify slice build <slice>`) owns request assembly, report validation, the `target-build-*` aborts (including `target-build-output-missing` for absent/empty output paths), the `slice.build.*` events, and the `built` transition gate.
- `crates/workflow/src/journal.rs` — newline-delimited JSON journal event log at `<project_dir>/.specify/journal.jsonl`; closed `Event` / `EventKind` taxonomy with kebab-case wire ids and `snake_case` Rust variants joined by `#[serde(rename = "…")]` (including the single `PlanReconcileCompleted` variant covering a successful `plan author` write, the eval-probe events `plan.entry.advanced` / `workspace.sync.completed` / `workspace.push.completed` with the closed `Actor` enum (`operator | agent`) on `plan.transition.approved`, plus the bootstrap events `cli.upgraded` / `plugins.refreshed`).
- `crates/workflow/src/{upgrade,plugins}.rs` — the bootstrap lifecycle kernels (handlers in `dispatch`, `crates/dispatch/src/commands/`). `upgrade.rs` owns `InstallChannel::detect()` and the channel-native upgrade plan; `plugins.rs` owns Cursor plugin-cache discovery and the `doctor` / `refresh` reports. There is no migration framework: pre-1.0 majors are re-init, not migration. See [`DECISIONS.md` §"Bootstrap and upgrade lifecycle"](./DECISIONS.md#bootstrap-and-upgrade-lifecycle).
- `crates/schema/src/` — embedded JSON Schema constants (`PLAN_JSON_SCHEMA`, `EVIDENCE_JSON_SCHEMA`, `LEAD_JSON_SCHEMA`, `PROPOSAL_JSON_SCHEMA`, `SLICE_MODEL_JSON_SCHEMA`, `SYNTHESIS_JSON_SCHEMA`, `PROVENANCE_JSON_SCHEMA`, `DECISION_JSON_SCHEMA`, `TOPOLOGY_LOCK_JSON_SCHEMA`, `BUILD_REQUEST_JSON_SCHEMA`, `BUILD_REPORT_JSON_SCHEMA`, `COMPONENTS_JSON_SCHEMA`, `RULE_JSON_SCHEMA`, `RESOLVED_RULES_JSON_SCHEMA`, `DIAGNOSTIC_JSON_SCHEMA`, `DIAGNOSTIC_REPORT_JSON_SCHEMA`, `SKILL_JSON_SCHEMA`, `SCENARIO_JSON_SCHEMA`, `MARKETPLACE_JSON_SCHEMA`) and the shared `jsonschema::Validator` plumbing (`compile_schema`, `validate_value`, `validate_serialisable`, `read_yaml_as_json`). Workflow and standards layers consume schemas through this crate; nobody else embeds `include_str!`'d schema JSON. The `crates/schema/tests/schemas.rs` parity test asserts each embedded constant byte-matches its on-disk `schemas/` source.
- `crates/diagnostics/src/` — the neutral `Diagnostic` substrate: the `Diagnostic` / `DiagnosticReport` / `DiagnosticSummary` types with the orthogonal `source` (`deterministic | model-assisted | hybrid | human | tool`) and `kind` (`violation | review`) axes, the fingerprint algorithm, `validate_diagnostic`, the four renderers (`json/pretty/github/compact`), and the `blocking` predicate. Import it directly from `diagnostics`.
- `crates/standards/src/rules/` — rules parser and resolver pipeline (`parse.rs`, `resolve.rs`, `resolve/{filter,sort}.rs`). Kept out of `workflow` by the standards-layer split.
- **No lint engine, no `Check` substrate.** Framework checks over the prose and manifest surfaces are plain cargo tests at [`tests/framework_quality/`](./tests/framework_quality/) (`links`, `skills`, `scenarios`, `prose` modules; policy as module constants, failures as test failures). The repo-local Rust-quality predicates live dev-only at `tests/rust_quality.rs` behind this repo's `cargo test --test rust_quality` gate. Contributor model: [docs/contributing/checks.md](./docs/contributing/checks.md).
- `crates/workflow/src/agents/` — init-time `AGENTS.md` context-fence generation (`workflow::agents`), housed in the workflow crate so its pure logic carries unit tests. Public modules: `detect` (shallow root-marker detection), `render` (deterministic Markdown body + `Input` struct), `fences` (byte-preserving `parse_document` / `plan_agents_write` write planner), `fingerprint` (`InputCollector` + canonical aggregate digest), `lock` (`context.lock` sidecar). All `Ctx`-free; the binary's `src/runtime/commands/agents/{assemble,generate}.rs` adapt a `Ctx` into a `render::Input` and drive these modules. Carries a module-scoped `missing_docs` / `pedantic` / `nursery` allow that preserves the original (binary-internal) lint posture.

The two **adapter validators** (`contract`, `vectis`) no longer live in this repo: they moved to `augentic/specify-adapters` and are now in-guest adapter library code inside each adapter's published component — the host dispatches no adapter WASI tool (the `extension run` verb family retired at S4; see [DECISIONS.md §"Old-stack deletion"](./DECISIONS.md#old-stack-deletion-milestone-s4)). There is no in-repo `wasi-tools/` workspace, checked-in `dist/` blob, or digest drift test. Crux shell presence and launcher-icon heuristics live **only** in the vectis adapter's in-guest core (`augentic/specify-adapters`): the host performs no plan-time shell detection, so there is no in-repo shell-detect crate.

### Exit codes

Part of the CLI wire contract. `Exit::from(&Error)` in [`src/runtime/output.rs`](./src/runtime/output.rs) is the single source of truth.

### Repository map

```text
src/runtime/             the provisioning front (native verb set + blind forwarding)
crates/workflow/         workflow domain logic
crates/specify/          wasm32 core guest shim
examples/                example guest components (echo adapter fixtures, omnia's examples/ pattern)
tests/rust_quality.rs    dev-only Rust-quality predicates + gate
tests/framework_quality/ prose/manifest framework checks as cargo tests
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
| Engineering standards layer (`standards` / `schema`, rules parse/resolve, `specify rules export`)                                                                     | [`DECISIONS.md` §"Standards layer split into `standards` and `schema`](./DECISIONS.md#standards-layer-split-into-standards-and-schema) |
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
- [`docs/standards/workflow.md`](./docs/standards/workflow.md) — the in-force workflow contract this binary implements. Defines the `source` / `target` / `plugin` / `axis` vocabulary, the kebab-case wire format, the `Source` / `Lead` / `Evidence` / `Slice` implementation types, writer ownership, and the CLI surface. Stable `§`-anchors that source comments and adapter prompts cite by name.
- [`docs/release.md`](./docs/release.md) — tagging and the platform-binary release pipeline.
- [`schemas/`](./schemas/) — JSON Schema files distributed with the binary (`evidence.schema.json`, `discovery/lead.schema.json`, `plan/plan.schema.json`, `target/build-request.schema.json`, and `target/build-report.schema.json`); the workflow contract pins each shape. There are no adapter-manifest schemas — adapter metadata is the WIT `manifest` record returned by `describe`.

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
5. If you touch `Slice.target`, `SliceSourceBinding`, `Divergence`, `crates/artifacts/src/spec/provenance.rs`, `crates/workflow/src/adapter/`, `crates/workflow/src/change/plan/core/propose.rs`, `crates/workflow/src/journal.rs`, `crates/schema/src/`, `crates/standards/src/rules/`, the `$CAPABILITY_DIR` env var, or the `adapter--<axis>--<slug>` tool cache scope: `rg <symbol>` across the whole repo — Rust *and* prose (`plugins/`, `docs/`, `codex/`) — and the sibling [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters) checkout, and update every hit in the same PR (workflow §"Note to the implementing agent" applies — the workflow contract spans both repos).
6. A fresh contributor should be able to reach any rule from this spine in three hops or fewer. If you find yourself adding prose here that isn't navigational, it belongs in one of the standards docs.
7. For Rust changes, skim [Rust quality](#rust-quality) before adding types, suppressions, or tests; if you add `#[expect]`, state in the PR why a refactor was infeasible.
