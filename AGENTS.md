# Specify - Agent Instructions

This repository is **Rust plus embedded prose**: the workspace at the repository root produces the `specify` runtime binary, and the surviving markdown (ultrathin `/spec:*` skill wrappers, reference docs) ships alongside it. Source and target adapter prose lives in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters). Generated Rust crates and Swift shells appear in downstream projects, not in this repository itself.

## Vocabulary

Specify names two adapter roles and three workflow nouns. Use the terms verbatim.

### Adapter roles

- **source adapter** — input role with two operations: `survey` (plan time) and `extract` (slice time). Ships as a single WebAssembly component exporting the WIT `source` interface (one component, no manifest); the guest crate lives at `sources/<name>/` in `augentic/specify-adapters`. Examples: `intent`, `documentation`, `typescript`, `screenshots`, `captures`.
- **target adapter** — output role with three operations: `guidance` (read by core synthesis), `build`, and `merge`. Ships as a single WebAssembly component exporting the WIT `target` interface; the guest crate lives at `targets/<name>/` in `augentic/specify-adapters`. Examples: `omnia`, `vectis`, `contracts`. See [`docs/explanation/adapter-anatomy.md`](docs/explanation/adapter-anatomy.md) for the full source / target contract, including the [adapter-vs-Cursor-plugin manifest boundary](docs/explanation/adapter-anatomy.md#adapter-manifests-vs-cursor-plugin-manifests).
- **plugin** (adapter vocabulary) — operator-facing shorthand for the shared adapter shape, used where source + target authors share the same audience tag. Workflow code resolves adapters through the provider-carried `adapter::Resolver` capability. The shipped WASI provider delegates to `adapter::resolver::Component`, which locates the identity's single `.wasm` component and reads its metadata from the component's own `metadata` export (no manifest file, no schema validation).

Do not confuse that noun with **Cursor plugins** under `plugins/` (e.g. `plugins/spec/`). Those are the IDE distribution surface for `/spec:*` skill wrappers and marketplace manifests; they are invisible to the `specify` CLI.

### Synthesis terms

- **lead** — slice-sized unit emitted by `survey`; one raw, unmerged block per lead under `## Lead inventory` in `discovery.md`, each identified by its `(source, lead)` pair (`lead` is unique only within a `source`).
- **evidence** — per-source result of `extract`; structured document with `claims:` persisted to `.specify/slices/<slice>/evidence/<source>.yaml`.
- **provenance** — the sources behind one requirement (the `Sources:` list in `spec.md`).
- **conflict / divergence** — unresolvable vs authority-resolved disagreement; surfaced inline as `[conflict]` / `[divergence]` tags on requirement headers.
- **authority** — closed enum (`intent` > `documentation` > `behaviour`) controlling who wins a disagreement.
- **model.yaml** — the single structured slice artifact at `.specify/slices/<slice>/model.yaml`, carrying provenance **inline** on each requirement. The provenance audit view is **projected on demand** by `specify slice provenance` — there is no persisted `provenance.yaml`. Audit-only; `spec.md` is the authoritative artifact. See [`docs/reference/provenance.md`](docs/reference/provenance.md) for the projected shape and audit posture.
- **component catalog** — operator-curated file at `.specify/design-system/components.yaml` declaring shared UI components (`status: confirmed | rejected`). The Vectis target reads the catalog at build time and factors shared component code per shell tree. Follows the same pattern as `tokens.yaml` and `assets.yaml`. Opt-in; absent catalog means no component factoring. Validated by `specify slice validate` (`slice-catalog-drift`) and the vectis adapter's in-guest composition validation (catalog cross-reference check). See [docs/explanation/components.md](docs/explanation/components.md).

### Workflow nouns

- **slice** — the single unit that flows through the fixed `refine → build → merge` loop. Each slice has its own proposal, spec, design, tasks, and merge step. Lives at `.specify/slices/<name>/`. Driven by `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop` and the `specify slice *` CLI verbs.
- **change** — the operator-defined umbrella that coordinates one or more slices through `change.md` + `plan.yaml`. Driven by `/spec:plan`, `specify plan execute`, `/spec:finalize` and the `specify plan *` CLI verbs. `change` is on-disk vocabulary, not a slash-command namespace.

Use *slice loop* for the per-slice lifecycle; reserve *change* for the on-disk umbrella that owns `change.md` and `plan.yaml`.

### Workspace topology (disambiguation)

The word **workspace** overloads two related concepts. Use them verbatim:

| Term               | Meaning                                                                                                            |
| ------------------ | ------------------------------------------------------------------------------------------------------------------ |
| **Workspace**      | Registry-only platform repo: `workspace: true` in `project.yaml`, `registry.yaml`, plan artifacts at the repo root |
| **Workspace slot** | Materialised peer at top-level `workspace/<project>/`                                                              |

`/spec:init workspace` and `specify init --workspace` scaffold a workspace. Slot materialisation and publication are operator-owned outside Specify.

### Workflow, standards, and artifacts

Specify separates three concerns. Use the terms verbatim; see [docs/explanation/standards-layer.md](docs/explanation/standards-layer.md) for the full picture.

| Layer                     | Role                                          | Examples                                                                                            |
| ------------------------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| **Workflow**              | Phase orchestration and lifecycle transitions | `/spec:plan`, `specify plan execute`, `specify plan transition`                                    |
| **Artifacts**             | Slice-local and baseline product intent       | `spec.md`, `plan.yaml`, `.specify/specs/`                                                           |
| **Engineering standards** | Durable policy that outlives any slice        | Rules under `codex/rules/` and per-adapter `prose/rules/` overlays, embedded in each target adapter |

**Authoring standards** (`docs/standards/`, enforced by the framework-quality cargo tests at `tests/framework/` on this repo) govern docs house style and the thin skill-wrapper shape. **Engineering standards** (rules in `augentic/specify-adapters` — `codex/rules/universal/` and per-adapter `prose/rules/` overlays, embedded in each target adapter's component and served by its references server) govern generated and hand-written code in consumer projects. Do not conflate them.

Engineering standards reach consumer projects through the target adapters' embedded prose, applied by their build review prompts — there is no engine-side lint or rules-export surface. Build-time `REVIEW.md` and plan Gate 1 `approved` are separate surfaces.

### Authority and reconciliation mechanics

The headline rules:

- **Authority resolution order** — per-slice override → Evidence document-level `authority:` → conflict. (A per-Evidence per-kind override is deferred to a future RFC.) See [`crates/slice/prompts/synthesis/authority.md`](crates/slice/prompts/synthesis/authority.md) for the resolution order and override surface.
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
- `/spec:finalize` — run `specify plan archive` after operator-owned publication and merge.

N=1 is degenerate, not special: `intent.survey` produces one lead, the operator stamps `approved`, and `specify plan execute` drives the same single-slice rhythm as a 12-slice change.

## Skill / CLI responsibility split

Phase skills are ultrathin invoke-and-relay wrappers: each elicits any missing arguments, invokes the corresponding `specify` command, and relays its output verbatim. Everything else — manifest validation, `metadata.yaml` reads and writes, plan and slice lifecycle transitions, source and target resolution, artifact-completion checks, baseline conflict detection, delta merge, archive move, and the judgment legs (survey, extract, synthesis, target build) — runs through the `specify` CLI and its guest orchestrations. No skill body carries orchestration, synthesis, or validation prose.

The CLI surface skills depend on is documented in [`specify` `--help`](cli). The headline groups: `specify init` (with the re-entry flag `--upgrade`, which bumps the `specify` pin and re-scaffolds preservation-safe files only, and `--platforms <csv>`, which declares the project's target platform set — required when the target adapter declares `platforms.required`), `specify source {resolve, survey, extract}` and `specify target {resolve}` (the adapter debug/breakout surface), `specify slice {list, refine, model show, build, validate, provenance, merge, drop}`, `specify plan {author, execute, add, amend, remove, transition, next, status, archive}` (`plan status` is the read-only next-action projection — `refine|build|merge <slice>` / `stop <reason>` / `drained` — over plan entries, slice metadata, and the journal tail), `specify archive {prune}` (retention-policy GC over the prunable slice/plan archive), and `specify journal {emit, show}` (`emit` — the guarded front door onto the closed journal taxonomy for agent-orchestrated phases; `show` — the read-only `--filter`/`--limit` projection over the journal). `specify source survey`/`extract` resolve `<source>` against `plan.yaml.sources.<key>` and run the bound source adapter's compiled-in prompt through one guest orchestration each (source extraction is agent-only). `specify slice build <slice>` is the guest-routed target-build verb: the orchestration assembles + schema-validates the build request, emits `target.execution.agent`, drives the target build prompts, validates the report, and owns the `built` transition, journaling `slice.build.started` / `.succeeded` / `.failed`. `specify slice merge` dispatches the bound target's phased merge gates (WIT `merge-phase`: `preflight` before the deterministic commit, `postflight` after it), schema-gates and persists each gate's report, and fires `slice.merge.started`, then `slice.merge.succeeded` / `slice.merge.failed` — or `slice.merge.postflight-failed` when the postflight gate fails after the commit (non-rollback, the merge stands) — alongside the durable `slice.archive.created`.

Never hand-edit `metadata.yaml`, `project.yaml`, `plan.yaml`, `discovery.md`, `sources.yaml`, or `targets.yaml`; never `mkdir -p .specify/...`; never `mv` anything into `.specify/archive/`. Route through the CLI — it enforces the legal lifecycle set and validates inputs in one place for humans, agents, and CI.

## Contracts target adapter

The contracts target adapter owns API contract authoring, import, and validation. Its build operation runs the OpenAPI, AsyncAPI, and JSON Schema format sub-flows, each with author / import / verify references under `targets/contracts/prose/references/` in `augentic/specify-adapters`.

The matching validation surface is the contracts adapter's in-guest validator, run by the target build and merge orchestrations.

## Vectis asset materialization

Vectis-bound projects commit per-platform exports under `design-system/assets/exports/`; shell writers render by `assets.yaml` entry `kind` — never substitute platform glyphs for `vector` / `raster` ids at build time.

| Concern                          | Where                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Materialize + export conventions | In-guest vectis asset materialization — codified in the vectis core's decisions in [`specify-adapters`](https://github.com/augentic/specify-adapters)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Build-prelude auto-materialize   | The vectis guest's build prelude — runs automatically inside the guest-routed `specify slice build`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Render-by-`kind` review rule     | [`targets/vectis/prose/rules/VECTIS-006-asset-render-by-kind.md`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/rules/VECTIS-006-asset-render-by-kind.md)                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Writer / integration contracts   | [`targets/vectis/prose/references/ios/design-system-integration.md`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/references/ios/design-system-integration.md), [`android/design-system-integration.md`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/references/android/design-system-integration.md), [`prompts/build/ios/write.md`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/prompts/build/ios/write.md), [`prompts/build/android/write.md`](https://github.com/augentic/specify-adapters/blob/main/targets/vectis/prose/prompts/build/android/write.md) |

## Plan-driven loop

`/spec:plan` authors the plan and exits at Gate 1; the operator stamps `approved`; `specify plan execute` drives the loop; `/spec:finalize` closes it. Plan *entries* are only ever written via `specify plan add` / `specify plan amend`; plan *lifecycle* is only ever written via `specify plan transition`; per-entry `in-progress` is only ever written by `specify plan next`; per-entry `done` is only ever written by `specify slice merge`. Per-entry status walks backwards only via `specify plan transition <entry> --undo`, which refuses to skip rungs (`done → in-progress`, then a second call for `in-progress → pending`) and fires one `plan.transition.undone` journal event per rung. The phase skills themselves stay unaware of the plan — they operate slice-by-slice. Hand-driven fallback: `specify plan next` → `/spec:refine` → `/spec:build` → `/spec:merge`, repeat until drained.

## Testing Philosophy

Specify strictly enforces an **aggressive integration-first posture**. 

- **Design against the public surface:** before adding a unit test, ask whether integration can reach the behavior — reachable through a CLI input or `pub` fn, observable at a public boundary (stdout JSON, exit code, filesystem), and affordable to assert there without a subprocess explosion. If yes, write the integration test; the unit test is redundant.
- **Default to Deletion:** a `src` unit test survives only when it is reachable and observable but cheap *only* in-process against a **private** kernel (a proptest or dense matrix), or covers a genuinely CLI-unreachable branch. If the kernel is already `pub`, re-home the test to `crates/<name>/tests/` instead.
- **Crate-Level Integration:** Put tests in `crates/<name>/tests/` instead of the root `tests/` when they test isolated domain logic that does not require full CLI orchestration. End-to-end and purely CLI-focused tests belong in the root `tests/`.
- **Widening is a last resort:** do NOT alter public APIs simply to support integration tests — prefer collapse-and-keep. The target is *near-zero* unit tests (no redundant or integration-reachable ones), not literal zero. `cargo llvm-cov nextest` remains the brake that ensures coverage holds during migrations; adapter posture is enforced in `augentic/specify-adapters` by the WIT contract plus each adapter crate's `tests/` suite and that repo's composed-deployment tests.

The test surface is three rungs: native integration tests over the fixture adapter and scripted models (`cargo make test`, per push), one WASM boundary smoke over the real component seam (`cargo make test-wasm`, weekly/path-filtered/manual; required for release), and one ignored native live-model workflow test (`cargo make test-live`, explicit). `omnia-testkit` owns generic model/scripted/runtime test mechanics; Specify owns workflow scenario semantics and assertions. See [`docs/standards/testing.md`](docs/standards/testing.md) and [`docs/contributing/quality-gates.md`](docs/contributing/quality-gates.md).

## Commands

All commands are run from the repository root:

- `cargo test --test framework` — the documentation and workflow consistency checks over the prose and manifest surfaces (the lightweight `framework` package at `tests/framework/`). Only a Rust toolchain is required.
- `make ci` — the full local gate: `cargo make ci` (the Rust workspace, `Makefile.toml` at the repo root), which includes the framework-quality test suite.

Per-push CI is the shared org workflow (nextest over the default workspace members — `crates/*`, `harness/fixtures`, and `tests/framework` — with clippy/doc/doctest/vet/deny over the whole workspace) plus one `wasm32-wasip2` compile check; no sibling checkout is required — the engine embeds no adapter-authored prose. WASM boundary execution runs on the weekly/path-filtered `.github/workflows/wasm.yaml` workflow (locally: `cargo make test-wasm`), not per push. See [docs/contributing/checks.md](docs/contributing/checks.md) for the check model.

The seven `/spec:*` skills are ultrathin invoke-and-relay wrappers (see [Skill / CLI responsibility split](#skill--cli-responsibility-split)). Their frontmatter shape is enforced by `schemas/authoring/skill.schema.json` plus [`tests/framework/skills.rs`](tests/framework/skills.rs); marketplace ↔ `plugins/` consistency by [`tests/framework/prose.rs`](tests/framework/prose.rs). There is no prose authoring standard for skill bodies beyond staying ultrathin. Extension model: [docs/contributing/checks.md](docs/contributing/checks.md). Local Cursor preview: `cursor-agent --plugin-dir plugins/<name>` (see [docs/contributing/operator-plugins.md](docs/contributing/operator-plugins.md)).

## Gotchas

- In a fresh clone, run `/spec:init` before using other `/spec:*` commands. The workflow skills expect the `.specify/` project structure to exist.
- The framework-quality tests (`cargo test --test framework`) enforce documentation consistency; if you remove or rename workflow terms, update the checks in the same change.
- **Adapter names are unique across axes** — a name appears under `sources/<name>/` xor `targets/<name>/`, never both. The store carries no axis segment, so a colliding name would make a binding's axis ambiguous; the `<axis>:<name>` adapter-id routing at the metadata/dispatch seam is the enforcement point.
- **First-party adapters resolve from the registry or a project-contained dev build** — `specify init <adapter>` accepts a package reference (`specify:omnia@1.0.0`) or the first-party shorthand (`omnia`, `omnia@1.0.0`). A semver pin is registry sugar: it installs the published component into the global single-file store (`<store-root>/<name>@<version>.wasm`). A bare name is the development shorthand: it resolves the project component cache (`<project-cache>/components/<name>.wasm`) then the project's own release build at `target/wasm32-wasip2/release/<name>.wasm`. There is no sibling-checkout probe — an adapter built elsewhere reaches the project as an operator-supplied local `.wasm` at init or a pinned store install. GitHub URLs are refused (`adapter-github-uri-unsupported`).
- Target review prompts in `augentic/specify-adapters` symlink `agent-teams.md` from each adapter's `references/` directory to that repo's shared `codex/references/runtime/review-team-protocol.md` overlay, forked from the canonical `docs/reference/review-team-protocol.md` here. If the canonical document is removed, the prompt's documentation may reference content that no longer resolves (guarded by the canonical-document presence check in `tests/framework/prose.rs`).
- Crossing a major is a hard cut: no silent compatibility aliases for old manifests, verbs, prompt paths, or slash-namespaces, and no migration framework. Pre-1.0, a major bump means re-init — `specify init --upgrade` bumps the pin over an existing project; anything deeper is a fresh `specify init`.

## Related coding standards

- CLI binary and crate conventions (errors, DTOs, hint colocation, brevity) live in [the Rust workspace section below](#the-rust-workspace-specify-cli) and [docs/standards/](docs/standards/). Skills that shell out to `specify` rely on the kebab-case `error` discriminants documented there.

## The Rust workspace (`specify` CLI) {#the-rust-workspace-specify-cli}

The repository root is a Rust workspace. It produces the `specify` runtime binary that the workflow skills shell out to. Generated Rust crates and Swift shells produced by the workflow live in downstream consumer repositories; this workspace owns the deterministic CLI primitives those workflows compose.

### Crate graph

The workspace is leaf → root. `error` is the dependency leaf and depends on no other workspace crate.

```text
error                    # leaf — thiserror + serde-saphyr only
schema                   # depends on error (embedded JSON Schemas + jsonschema plumbing; owns schema::digest — SHA-256 hex via sha2 + base16ct — and schema::diagnostics, the neutral Diagnostic substrate: report, fingerprint, validator, blocking)
artifacts                # depends on {error,schema} (artifact types + parsers: spec, task, evidence, discovery; shared atomic writer; artifacts::validate artifact rule registry — NOT on the workflow crates or anything named lint)
project                  # foundation — depends on {error,schema,artifacts,omnia-guest}: init (+ project::agents — init-time AGENTS.md context-fence generation), adapter resolution, config/Layout, journal, registry, the plan data model (Plan/Entry/Status/Lifecycle + transitions, doctor, the propose kernel), the slice data model (metadata/lifecycle/outcome), the seam capability traits + build wire DTOs (project::seam), the judgment kernel (schema_gated, MAX_REPAIRS), and the shared handler plumbing (project::handler: Anchor, Ctx, Render, ReportBody, the operation-layer Error); operation families in handlers submodules: journal::handlers, registry::handlers, adapter::handlers, init::handlers
slice                    # the slice loop — depends on project: refine/build/merge orchestration (slice::orchestrate incl. the extract half of the source axis), synthesis + the synthesize judgment leg, validation, provenance, the delta-merge engine, slice::handlers (the specify slice operations) + slice::source (source extract), and its own prompts/ corpus (synthesize.md + synthesis/*)
change                   # the change loop — depends on {project,slice}: plan author/execute orchestration (change::orchestrate incl. the survey half of the source axis and workspace routing), the propose judgment leg, change::plan::handlers (the specify plan operations) + change::source (source survey), and its own prompts/ corpus (propose.md)
transport                # wasm-clean transport assembly — explicit typed command/HTTP routers over Invoker, exhaustive Args-to-Input TryFrom conversions, projectors, and exit contract; depends on {project,slice,change}
prose                    # build-dependency crate — embed-time prompt-corpus walk + link check generating each crate's DOCS table
harness/fixtures         # the fixture adapter (deterministic native core supplying both specify:adapter axes for engine tests + the combined fixture_adapter guest for hosted WASM deployments)
specify (root crate) # Omnia deployment unit under src/: guest lib (wasm32, exporting wasi:cli/run + wasi:http/incoming-handler over the shared typed transport routers, published as specify:core@<binary version>) + shipped runtime
```

The artifact validation rule registry lives in `artifacts::validate`: `artifacts` depends on none of the workflow crates, so a rule cannot transition a slice or stamp a plan. `artifacts` is the lifecycle-free leaf holding the artifact types and parsers the workflow layer reads. The embedded JSON Schemas (and the shared `schema::digest` SHA-256 helpers) live in one place, `schema`, which also carries the neutral `Diagnostic` / `DiagnosticReport` substrate (`schema::diagnostics`), so every check producer — validate and review alike — emits the same finding currency without importing the other surface's code. Engineering-standards rules ship inside the target adapters in `augentic/specify-adapters`; there is no engine-side rules crate.

Modules of note across the workspace (workflow layer):

- `crates/project/src/platform.rs` — closed `Platform` enum (`Core | Ios | Android | Web | Desktop`, `#[serde(rename_all = "kebab-case")]`) representing the set of target platforms a project may declare in `project.yaml`. `Core` is mandatory in every set. `Ios` and `Android` have scaffold/build/verify support; `Web` and `Desktop` are type-system placeholders for future functionality. Includes `Display`, `FromStr`, and `parse_platforms_csv` for the `--platforms` CLI flag.
- `crates/project/src/adapter/` — deployment-neutral `Resolver` capability plus the shipped `resolver::Component` implementation (one component, no manifest). Operations and kernels receive resolution through their provider; the WASI provider supplies component resolution and the native harness supplies linked Rust resolution from `specify-adapters/harness/native`. `resolver::Component` takes an `AdapterRef { name, version: Option<semver::Version> }`: a pinned identity resolves the global single-file store entry (`<store-root>/<name>@<version>.wasm`, where the store root is `$SPECIFY_ADAPTER_STORE`, else `$HOME/.specify/adapters`; verify-on-read against the recorded byte digest, `adapter-digest-mismatch` on drift); a bare name resolves the project component cache (`<project-cache>/components/<name>.wasm`, mirrored at init from an operator-supplied local file) then the project's own development release build (`target/wasm32-wasip2/release/<name>.wasm`); there is no sibling-checkout probe. A miss on every probe is `adapter-not-found`. Metadata (the `specify` host-CLI compatibility floor, a target's `inputs[]` and `PlatformsCapability`) comes from the component's deterministic `metadata` export through a runner passed explicitly to `resolver::Component`; answers are cached against the component file's SHA-256 in a `<component>.metadata.json` sidecar. There is no process-global resolver or metadata registration. A binding on the wrong axis fails at the dispatch seam (no deployed guest exports the requested `<axis>:<name>` id); an unparseable floor is `adapter-floor-malformed`; a floor newer than the running binary raises `Error::AdapterCliTooOld` (`adapter-cli-too-old`) on the exit-3 `EXIT_VERSION_TOO_OLD` path. Resolved values carry an opaque `Origin` (`label`, display `reference`), so workflow code does not enumerate deployment mechanisms. The closed `SourceOperation` / `TargetOperation` enums in `adapter/operation.rs` are the typed per-axis operation sets derived from the WIT contract.
- `crates/project/src/init/adapter_uri.rs` — `specify init <adapter>` argument parser. Recognises the **package reference** form (`<namespace>:<name>@<semver>`, e.g. `specify:omnia@1.2.0`) via `AdapterPackageRef`: an immutable, content-addressed registry locator with a mandatory exact-SemVer pin and no branch/tag defaulting (a missing or non-SemVer version raises `adapter-package-ref-version-required`). A recognised package reference must already exist in the global single-file store; a missing entry is `adapter-package-not-installed`. The first-party **shorthand** splits: `omnia@1.0.0` is package-reference sugar for `specify:omnia@1.0.0`; bare `omnia` is the development shorthand — parse records the identity without demanding an artifact, and the injected `Resolver` locates the component (the project component cache, then the project's own release build at `target/wasm32-wasip2/release/omnia.wasm` in the shipped path; linked Rust crates in the native harness). GitHub URLs are refused with `adapter-github-uri-unsupported`; adapters resolve from an installed store entry, a local component, or a dev build. Only an operator-supplied local `.wasm` file is mirrored into the project component cache (`<project-cache>/components/<name>.wasm`); store entries and dev builds are read in place. `AdapterRef::from_value` recovers an internal `AdapterRef` from a recorded adapter value.
- `crates/artifacts/src/spec/provenance.rs` — `spec.md` requirement-block parser (`ID:` / `Sources:` / `Status:` lines, closed `RequirementStatus` enum, inline `[…]` tag coherence).
- `crates/project/src/plan/propose.rs` — plan-time lead-reconciliation kernel. Envelope DTOs (closed `kind: request | response`), the pure `build_request` / `build_catalog` / `resolve_topology` assembly, and the `Plan::propose_from` projection kernel driven by the guest `plan author` orchestration.
- `crates/slice/src/build/` — target build envelope kernel. `wire.rs` holds the closed-shape `BuildRequest` / `BuildReport` DTOs (round-tripping `schemas/target/build-{request,report}.schema.json`), `BuildOutput` (`{ platform: Platform, path }` — the optional per-platform build outputs declared in `BuildReport.outputs[]`), plus the `BuildReport::enforce_no_blocking` and `BuildReport::enforce_outputs_exist` gates; `assemble.rs` assembles a request from the bound target adapter's declared `inputs[]` against the slice tree (raising `target-build-input-missing`). The guest `orchestrate::build` orchestration (behind the guest-routed `specify slice build <slice>`) owns request assembly, report validation, the `target-build-*` aborts (including `target-build-output-missing` for absent/empty output paths), the `slice.build.*` events, and the `built` transition gate.
- `crates/project/src/journal.rs` — newline-delimited JSON journal event log at `<project_dir>/.specify/journal.jsonl`; closed `Event` / `EventKind` taxonomy with kebab-case wire ids and `snake_case` Rust variants joined by `#[serde(rename = "…")]`, including the single `PlanReconcileCompleted` variant covering a successful `plan author` write, `plan.entry.advanced`, and the closed `Actor` enum (`operator | agent`) on `plan.transition.approved`.
- `crates/schema/src/` — embedded JSON Schema constants (`PLAN_JSON_SCHEMA`, `EVIDENCE_JSON_SCHEMA`, `LEAD_JSON_SCHEMA`, `PROPOSAL_JSON_SCHEMA`, `SLICE_MODEL_JSON_SCHEMA`, `SYNTHESIS_JSON_SCHEMA`, `PROVENANCE_JSON_SCHEMA`, `DECISION_JSON_SCHEMA`, `TOPOLOGY_LOCK_JSON_SCHEMA`, `BUILD_REQUEST_JSON_SCHEMA`, `BUILD_REPORT_JSON_SCHEMA`, `COMPONENTS_JSON_SCHEMA`, `DIAGNOSTIC_JSON_SCHEMA`, `DIAGNOSTIC_REPORT_JSON_SCHEMA`, `SKILL_JSON_SCHEMA`, `MARKETPLACE_JSON_SCHEMA`) and the shared `jsonschema::Validator` plumbing (`compile_schema`, `validate_value`, `validate_serialisable`, `read_yaml_as_json`). Every consumer reaches schemas through this crate; nobody else embeds `include_str!`'d schema JSON. The `crates/schema/tests/schemas.rs` parity test asserts each embedded constant byte-matches its on-disk `schemas/` source.
- `crates/schema/src/diagnostics/` — the neutral `Diagnostic` substrate: the `Diagnostic` / `DiagnosticReport` / `DiagnosticSummary` types with the orthogonal `source` (`deterministic | model-assisted | hybrid | human | tool`) and `kind` (`violation | review`) axes, the fingerprint algorithm, `validate_diagnostic`, and the `blocking` predicate. Import it from `schema::diagnostics`.
- **No lint engine, no `Check` substrate.** Framework checks over the prose and manifest surfaces are plain cargo tests at [`tests/framework/`](./tests/framework/) (`links`, `skills`, `prose` modules; policy as module constants, failures as test failures). Contributor model: [docs/contributing/checks.md](./docs/contributing/checks.md).
- `crates/project/src/agents/` — crate-private init-time `AGENTS.md` context generation: shallow root-marker detection, deterministic Markdown rendering, context-fence parsing, input fingerprinting, and `context.lock` writing. `specify init` drives it through `project::init` (generate root `AGENTS.md` + `.specify/context.lock` when `AGENTS.md` is absent; skip inside materialised workspace slots).

The two **adapter validators** (`contract`, `vectis`) are in-guest adapter library code inside each adapter's published component in `augentic/specify-adapters` — the host dispatches no adapter WASI tool. Crux shell presence and launcher-icon heuristics live **only** in the vectis adapter's in-guest core: the host performs no plan-time shell detection.

### Exit codes

Part of the CLI wire contract. `Exit::from(&Error)` in [`crates/transport/src/command/output.rs`](./crates/transport/src/command/output.rs) is the single source of truth.

### Repository map

```text
src/runtime.rs           shipped binary — omnia::runtime! command mode over cursor backends
src/lib.rs               wasm32 core guest shim (mod command; mod http; mod provider;)
src/provider.rs          WIT-backed Provider (Anchor + Model + SourceSeam + TargetSeam over the world's imports)
src/command.rs              struct Cli + Guest::run + route(cli)
src/http.rs              struct Http + Guest impl + the HTTP route table
crates/transport/         shared command/HTTP routing, clap grammar, conversions, projectors, and exit contract
crates/project/          foundation — init, adapter resolution, config, journal, registry, plan + slice data models, seam traits, judgment kernel
crates/slice/            the slice loop — refine/build/merge orchestration, synthesis, validation, merge engine, prompts
crates/change/           the change loop — plan author/execute orchestration, plan operations, prompts
harness/wasm/            the WASM boundary smoke — hosts specify.wasm with the combined fixture-adapter component
harness/fixtures/        the fixture adapter (native core for both specify:adapter axes + the combined fixture_adapter guest)
harness/live/            the ignored native live-model workflow test (cargo make test-live)
tests/framework/         prose/manifest framework checks as cargo tests
tests/fixtures/          shared fixture trees referenced by crate-level suites
```

| Code | Name                     | When                                                                  |
| ---- | ------------------------ | --------------------------------------------------------------------- |
| 0    | `EXIT_SUCCESS`           | Command succeeded.                                                    |
| 1    | `EXIT_GENERIC_FAILURE`   | Any `Error` variant not listed below (I/O, YAML, schema, merge, …).   |
| 2    | `EXIT_VALIDATION_FAILED` | Validation findings, `Error::Validation`, `Error::Argument`.          |
| 3    | `EXIT_VERSION_TOO_OLD`   | `Error::CliTooOld` — `project.yaml.specify` is newer than the binary. |

### Documentation map

| Topic                                                                                                                                                                                 | Document                                                                                                                                    |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Cross-cutting code-quality rules (naming, error variants, traits-for-testability, archaeology)                                                                                        | [`docs/standards/style.md`](./docs/standards/style.md)                                                                                      |
| Lints, comments, brevity, DTOs, YAML/atomic writes, module layout (`<module>.rs` + `<module>/`, no `mod.rs` outside `tests/`)                                                         | [`docs/standards/coding-standards.md`](./docs/standards/coding-standards.md)                                                                |
| `Operation`, `Ctx`, `Render`, projectors, exit-code mapping, typed router contract                                                                                                   | [`docs/standards/handler-shape.md`](./docs/standards/handler-shape.md)                                                                      |
| Workspace layout, WASI carve-outs, `Layout<'a>`, time injection, atomic-write rationale, workflow domain modules, supply chain                                                        | [`docs/standards/architecture.md`](./docs/standards/architecture.md)                                                                        |
| `cargo nextest`, integration-first policy, golden files, `REGENERATE_GOLDENS`                                                                                                         | [`docs/standards/testing.md`](./docs/standards/testing.md)                                                                                  |
| Standing architectural decisions (error layering, exit codes, atomic writes, YAML library, wire compatibility, workflow type renames, plan lifecycle, adapter loader, journal events) | [`docs/standards/`](./docs/standards/) (per-area docs) and git history                                                                      |
| Engineering standards layer (adapter-embedded rules)                                                                                                                                  | [`docs/explanation/standards-layer.md`](./docs/explanation/standards-layer.md)                                                              |
| Vectis asset materialization                                                                                                                                                          | In-guest build prelude and asset-domain policy live in the vectis core (`augentic/specify-adapters`); the engine dispatches no prepare hook |

### Rust quality {#rust-quality}

**Aggressive Integration-First Posture:**
Specify mandates an aggressive integration-first test strategy. Agents must actively work to remove unit tests (`#[cfg(test)]`) in favor of crate-level (`crates/<name>/tests/`) and binary integration tests (`tests/<area>.rs`).
- **Design against the public surface first:** before adding a unit test, ask whether integration can reach the behavior — is it reachable through a CLI input or a `pub` fn, is its effect observable at a public boundary (stdout JSON, exit code, filesystem), and is that affordable without a subprocess-pool explosion? If yes, write the integration test; the unit test is redundant.
- **Default to deletion:** a `src` unit test survives only when it is reachable and observable but cheap *only* in-process against a **private** kernel (a proptest or dense matrix), or it covers a genuinely CLI-unreachable branch. If the kernel is already `pub`, relocate the test to `crates/<name>/tests/` rather than leaving it in `src`.
- **Do NOT widen public APIs to test a private kernel.** Widening trades durable surface stability for coverage you already have; prefer collapse-and-keep. The target is *near-zero* `src` unit tests — no redundant or integration-reachable ones — not literal zero. Use `cargo llvm-cov nextest` to prove coverage holds when removing unit tests.
- **Push crate-specific tests down:** `tests/` at the root of the workspace is for E2E workflows. Crate-specific logic must be tested in `crates/<name>/tests/` via the crate's public API.

Read [style.md](./docs/standards/style.md), [coding-standards.md](./docs/standards/coding-standards.md), and [testing.md § Test naming](./docs/standards/testing.md#test-naming) before adding types, suppressions, or tests. Run `cargo make ci` (not bare `cargo test` — CI uses `RUSTFLAGS=-Dwarnings`).

**Naming:** The module path is context — `registry::show`, not `show_registry`. Test function names are short identifiers; put the narrative in the test body ([testing.md](./docs/standards/testing.md#test-naming)).

**Lint suppressions:** Refactor first. Use `#[expect(lint, reason = "…")]` at the smallest scope. `#![allow]` only at module root when the lint applies to every item below and the reason is contract-locked. Prefer `#[expect]` with a reason over bare `#[allow]`.

| Do not                                                    | Do instead                                                       | See                                                                                                           |
| --------------------------------------------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `#[allow]` / `#[expect]` before trying a split or extract | Extract helper or submodule; suppress only if contract-locked    | [coding-standards § Lint suppression](./docs/standards/coding-standards.md#lint-suppression-posture)          |
| `trait Foo` + sole `RealFoo` for tests                    | `CmdRunner`, `AtomicYaml`, or filesystem/tempdir                 | [style.md § No traits for testability](./docs/standards/style.md#no-traits-for-testability-alone)             |
| `*RenderInput` wrapper for `Render`                       | `Render` on domain type or `ctx.emit_with` closure               | [style.md § One body per command](./docs/standards/style.md#one-body-per-command-no-wrapper-newtype)          |
| Transport formatting inside operations                    | Return a typed `Serialize + Render` body; project at the router   | [handler-shape.md](./docs/standards/handler-shape.md)                                                         |
| RFC/Phase/migration history in comments                   | ≤ 3 lines “what today”; history stays in git                     | [style.md § No archaeology](./docs/standards/style.md#no-archaeology-in-code)                                 |
| Sentence-length test fn names                             | Short name + `mod` grouping                                      | [testing.md § Test naming](./docs/standards/testing.md#test-naming)                                           |
| Add a `src` `#[cfg(test)]` for CLI-reachable behavior     | Exercise it through the public surface in `crates/<name>/tests/` | [testing.md § minimize the unit layer](./docs/standards/testing.md#the-three-layers--minimize-the-unit-layer) |
| Nested `struct Body` inside `fn`                          | Top-level `*Body` + `From` impl                                  | [coding-standards § DTOs](./docs/standards/coding-standards.md#dtos)                                          |
| New `Error::Diag` for one-off shapes                      | Typed variant after ≥3 identical call sites                      | [style.md § Error variants](./docs/standards/style.md#error-variants-budgeted-by-recovery-not-source)         |

External references:

- [Vocabulary](#vocabulary) at the top of this file — workflow vocabulary (slice / change), adapter `plugin` vs Cursor `plugins/`, plan-driven loop.
- [`docs/standards/workflow.md`](./docs/standards/workflow.md) — the in-force workflow contract this binary implements. Defines the `source` / `target` / `plugin` / `axis` vocabulary, the kebab-case wire format, the `Source` / `Lead` / `Evidence` / `Slice` implementation types, writer ownership, and the CLI surface. Stable `§`-anchors that source comments and adapter prompts cite by name.
- [`docs/release.md`](./docs/release.md) — tagging and the platform-binary release pipeline.
- [`schemas/`](./schemas/) — JSON Schema files distributed with the binary (`evidence.schema.json`, `discovery/lead.schema.json`, `plan/plan.schema.json`, `target/build-request.schema.json`, and `target/build-report.schema.json`); the workflow contract pins each shape. There are no adapter-manifest schemas — adapter metadata is the WIT `metadata` record returned by `metadata`.

### Quick toolchain

All driven by `cargo make` (see [`Makefile.toml`](./Makefile.toml)). Run the full local CI suite before committing; do not rely on narrower substitutes such as `cargo test` or `cargo clippy`.

```bash
cargo make ci             # fmt-check + lint + wasm + test + test-docs + doc + vet + deny
cargo make check          # fmt-check + lint + wasm + test + test-docs + doc (the pre-commit subset; `cargo make fmt` fixes formatting)
cargo make test           # cargo nextest run --locked --all-features --no-tests=pass over the default members, under -Dwarnings
cargo make test-wasm    # builds the WASM guests then runs the opt-in WASM boundary smoke
cargo make test-live    # the explicit live-model workflow test (operator-invoked; needs cursor-agent credentials)
cargo make lint           # cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo make fmt            # nightly cargo fmt --all
cargo make audit          # cargo-audit; cargo make deny / outdated / deps / vet for the rest
```

### When working in the Rust workspace

1. Read the matching [`docs/standards/`](./docs/standards/) doc before changing error layering, exit codes, atomic writes, the YAML library, the JSON envelope shape, the workflow type names (`Target*` / `Plugin` / `SliceSourceBinding` / `Divergence`), the plan lifecycle (`pending | approved`), the journal event taxonomy, the per-axis cache layout, or adding a new workspace crate.
2. For any Rust change, consult [`docs/standards/`](./docs/standards/) — at minimum the doc that matches the area you are editing, plus [`style.md`](./docs/standards/style.md) for cross-cutting rules.
3. Run `cargo make ci` before committing. If it cannot run, say exactly why and which checks were run instead.
4. When you remove a symbol, `rg <SymbolName> -- AGENTS.md docs/` and update every hit in the same PR.
5. If you touch `Slice.target`, `SliceSourceBinding`, `Divergence`, `crates/artifacts/src/spec/provenance.rs`, `crates/project/src/adapter/`, `crates/project/src/plan/propose.rs`, `crates/project/src/journal.rs`, or `crates/schema/src/`: `rg <symbol>` across the whole repo — Rust *and* prose (`plugins/`, `docs/`, `codex/`) — and the sibling [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters) checkout, and update every hit in the same PR (workflow §"Note to the implementing agent" applies — the workflow contract spans both repos).
6. A fresh contributor should be able to reach any rule from this spine in three hops or fewer. If you find yourself adding prose here that isn't navigational, it belongs in one of the standards docs.
7. For Rust changes, skim [Rust quality](#rust-quality) before adding types, suppressions, or tests; if you add `#[expect]`, state in the PR why a refactor was infeasible.
