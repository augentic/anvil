# Augentic Plugins - Agent Instructions

This is a **documentation/prompt-engineering repository**. The codebase consists of markdown skill definitions, reference docs, templates, and shell scripts. Generated Rust crates and Swift shells appear in downstream projects, not in this repository itself.

## Vocabulary

Specify names two adapter roles and three workflow nouns. Use the terms verbatim.

### Adapter roles

- **source adapter** — input role with two operations: `survey` (plan time) and `extract` (slice time). Lives at `adapters/sources/<name>/adapter.yaml`. Examples: `intent`, `documentation`, `typescript`, `screenshots`, `captures`.
- **target adapter** — output role with three operations: `shape` (read by core synthesis), `build`, and `merge`. Lives at `adapters/targets/<name>/adapter.yaml`. Examples: `omnia`, `vectis`, `contracts`. See [`docs/explanation/adapter-anatomy.md`](docs/explanation/adapter-anatomy.md) for the full source / target contract, including the [adapter-vs-Cursor-plugin manifest boundary](docs/explanation/adapter-anatomy.md#adapter-manifests-vs-cursor-plugin-manifests).
- **plugin** — historical shorthand for the shared adapter shape. The Rust loaders are `SourceAdapter::resolve(name, project_dir)` and `TargetAdapter::resolve(name, project_dir)` in [`crates/workflow/src/adapter/`](https://github.com/augentic/specify-cli/tree/main/crates/workflow/src/adapter); each validates against the matching per-axis `source.schema.json` / `target.schema.json` distributed with the CLI. The noun "plugin" survives in operator-facing prose where source + target authors share the same audience tag.

### Synthesis terms

- **lead** — slice-sized unit emitted by `survey`; one raw, unmerged block per lead under `## Lead inventory` in `discovery.md`, each identified by its `(source, lead)` pair (`lead` is unique only within a `source`).
- **evidence** — per-source result of `extract`; structured document with `claims:` persisted to `.specify/slices/<slice>/evidence/<source>.yaml`.
- **provenance** — the sources behind one requirement (the `Sources:` list in `spec.md`).
- **conflict / divergence** — unresolvable vs authority-resolved disagreement; surfaced inline as `[conflict]` / `[divergence]` tags on requirement headers.
- **authority** — closed enum (`intent` > `documentation` > `behaviour`) controlling who wins a disagreement.
- **model.yaml** — the single structured slice artifact at `.specify/slices/<slice>/model.yaml`, carrying provenance **inline** on each requirement. The provenance audit view is **projected on demand** by `specify slice provenance` — there is no persisted `provenance.yaml`. Audit-only; `spec.md` is the authoritative artifact. See [`plugins/spec/references/synthesis/provenance.md`](plugins/spec/references/synthesis/provenance.md) for the projected shape and audit posture.
- **component catalog** — operator-curated file at `.specify/design-system/components.yaml` declaring shared UI components (`status: confirmed | rejected`). The Vectis target reads the catalog at build time and factors shared component code per shell tree. Follows the same pattern as `tokens.yaml` and `assets.yaml`. Opt-in; absent catalog means no component factoring. Validated by `specify slice validate` (`slice-catalog-drift`) and `specify tool run vectis -- validate composition` (catalog cross-reference check). See [docs/explanation/components.md](docs/explanation/components.md).

### Workflow nouns

- **slice** — the single unit that flows through the fixed `refine → build → merge` loop. Each slice has its own proposal, spec, design, tasks, and merge step. Lives at `.specify/slices/<name>/`. Driven by `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop` and the `specify slice *` CLI verbs.
- **change** — the operator-defined umbrella that coordinates one or more slices through `change.md` + `plan.yaml`. Driven by `/spec:plan`, `/spec:execute`, `/spec:finalize` and the `specify plan *` CLI verbs. `change` is on-disk vocabulary, not a slash-command namespace.

Use *slice loop* for the per-slice lifecycle; reserve *change* for the on-disk umbrella that owns `change.md` and `plan.yaml`.

### Workspace topology (disambiguation)

The word **workspace** overloads three related concepts. Use them verbatim:

| Term               | Meaning                                                                                                            |
| ------------------ | ------------------------------------------------------------------------------------------------------------------ |
| **Workspace**      | Registry-only platform repo: `workspace: true` in `project.yaml`, `registry.yaml`, plan artifacts at the repo root |
| **Workspace slot** | Materialised peer at `.specify/workspace/<project>/`                                                               |
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

The full mechanics — per-slice operator overrides, inline provenance shape — live in the cli repo's [`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md). The headline rules:

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
- `/spec:finalize` — push branches, observe PR state, run `specify plan archive` once every PR is `MERGED`.

N=1 is degenerate, not special: `intent.survey` produces one lead, the operator stamps `approved`, and `/spec:execute` drives the same single-slice rhythm as a 12-slice change.

## Skill / CLI responsibility split

Phase skills are agent-driven orchestrators. Every deterministic operation — manifest validation, `metadata.yaml` reads and writes, plan and slice lifecycle transitions, source and target resolution, artifact-completion checks, baseline conflict detection, delta merge, archive move — runs through the `specify` CLI. Skill markdown drives the agent-side work: eliciting operator intent, reading brief bodies, writing evidence and synthesized artifacts, running the target adapter's build brief, and rendering summaries.

The CLI surface skills depend on is documented in [`specify` `--help`](https://github.com/augentic/specify-cli). The headline groups: `specify init` (with the re-entry flags `--upgrade`, which bumps `specify_version` and re-scaffolds preservation-safe files only, and `--check-migration`, the read-only major-version probe, and `--platforms <csv>`, which declares the project's target platform set — required when the target adapter declares `platforms.required`), `specify source {resolve, survey, extract}`, `specify target {resolve}`, `specify slice {create, synthesize, model show, build, transition, validate, provenance, merge}`, `specify plan {create, add, amend, transition, next, status, archive}` (`plan status` is the read-only next-action projection — `refine|build|merge <slice>` / `stop <reason>` / `drained` — over plan entries, slice metadata, and the journal tail), `specify archive {prune}` (retention-policy GC over the prunable slice/plan archive), `specify workspace {sync, push, prepare}`, `specify tool run` (WASI tool dispatch — `contract`, `vectis`, …), `specify migrate` (run registered migrators), `specify upgrade` (channel-aware CLI self-update), `specify plugins {doctor, refresh}` (Cursor plugin-cache drift report and invalidation), and `specify journal {emit, show}` (`emit` — the guarded front door onto the closed journal taxonomy for agent-orchestrated phases; `show` — the read-only `--filter`/`--limit` projection over the journal). `specify source survey`/`extract` resolve `<source>` against `plan.yaml.sources.<key>` and run the bound source adapter's brief through the two-phase agent handoff (source extraction is agent-only). `specify slice build <slice>` is the two-phase target-build verb the `/spec:build` skill drives: `specify slice build --phase prepare` assembles + schema-validates the build request and emits `target.execution.agent`, the skill runs the target `build` brief, and `specify slice build --phase finalize` validates the report and owns the `built` transition (the skill no longer hand-transitions), journaling `slice.build.started` / `.succeeded` / `.failed`. `specify slice merge` fires `slice.merge.started` / `.succeeded` / `.failed` on its validator outcome (not on a merge report) alongside the durable `slice.archive.created`.

Never hand-edit `metadata.yaml`, `project.yaml`, `plan.yaml`, `discovery.md`, `sources.yaml`, or `targets.yaml`; never `mkdir -p .specify/...`; never `mv` anything into `.specify/archive/`. Route through the CLI — it enforces the legal lifecycle set and validates inputs in one place for humans, agents, and CI.

## Contracts target adapter

The contracts target adapter owns API contract authoring, import, and validation. Its `build` brief runs the OpenAPI, AsyncAPI, and JSON Schema format sub-flows, each with author / import / verify references under `adapters/targets/contracts/references/`.

The matching CLI validation surface is the declared `contract` WASI tool, run via `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json`.

## Vectis asset materialization

Vectis-bound projects commit per-platform exports under `design-system/assets/exports/`; shell writers render by `assets.yaml` entry `kind` — never substitute platform glyphs for `vector` / `raster` ids at build time. See [`rfcs/roadmap.md`](rfcs/roadmap.md#current-priorities) (**Recently implemented**).

| Concern | Where |
| ------- | ----- |
| Materialize CLI + export conventions | `specify tool run vectis -- materialize assets` — codified in [`specify-cli` `wasi-tools/vectis/DECISIONS.md` §K](https://github.com/augentic/specify-cli/blob/main/wasi-tools/vectis/DECISIONS.md#k--materialization-and-render-by-kind) |
| Prepare auto-materialize | `specify slice build --phase prepare` — sole automatic in-loop caller; scope in DECISIONS §K (prepare hook) |
| Bootstrap `app-icon` gate | `plan-bootstrap-app-icon-missing` on `specify plan validate`; bootstrap-only — codified in [`wasi-tools/vectis/DECISIONS.md` §L](https://github.com/augentic/specify-cli/blob/main/wasi-tools/vectis/DECISIONS.md#l--bootstrap-app-icon-gate) |
| Render-by-`kind` review rule | [`adapters/targets/vectis/rules/VECTIS-006-asset-render-by-kind.md`](adapters/targets/vectis/rules/VECTIS-006-asset-render-by-kind.md) |
| Writer / integration contracts | [`adapters/targets/vectis/references/ios/design-system-integration.md`](adapters/targets/vectis/references/ios/design-system-integration.md), [`android/design-system-integration.md`](adapters/targets/vectis/references/android/design-system-integration.md), [`briefs/build/ios/write.md`](adapters/targets/vectis/briefs/build/ios/write.md), [`briefs/build/android/write.md`](adapters/targets/vectis/briefs/build/android/write.md) |

## Plan-driven loop

`/spec:plan` authors the plan and exits at Gate 1; the operator stamps `approved`; `/spec:execute` drives the loop; `/spec:finalize` closes it. Plan *entries* are only ever written via `specify plan add` / `specify plan amend`; plan *lifecycle* is only ever written via `specify plan transition`; per-entry `in-progress` is only ever written by `specify plan next`; per-entry `done` is only ever written by `specify slice merge`. Per-entry status walks backwards only via `specify plan transition <entry> --undo`, which refuses to skip rungs (`done → in-progress`, then a second call for `in-progress → pending`) and fires one `plan.transition.undone` journal event per rung. The phase skills themselves stay unaware of the plan — they operate slice-by-slice. Hand-driven fallback: `specify plan next` → `/spec:refine` → `/spec:build` → `/spec:merge`, repeat until drained.

## Commands

All commands are run from the repository root:

- `make lint` — delegates to `cargo +nightly -Zscript scripts/specify.rs lint framework`, a single-file Cargo script that reads the `cli` source spec from `Specify.toml` (or a gitignored `Specify.local.toml` overlay), **builds** that pinned `specify-cli` source with Cargo, and runs `specify lint framework` for documentation and workflow consistency checks. Only a Rust toolchain is required (currently nightly, since cargo-script needs `-Zscript`); no published binary is downloaded. Co-develop the CLI with a gitignored `Specify.local.toml` `cli = { path = "../specify-cli" }`.
- `make use-local-plugins` / `make use-team-plugins` — choose plugin source (reload Cursor after either).

The `specify-standards` framework predicate regression suite is owned and run by `augentic/specify-cli` (its `cargo make test` runs the whole workspace, including `specify-standards` framework). This repo's CI runs the same framework checks through a different, intentional path: `.github/workflows/ci.yaml` checks out the sibling `specify-cli` (branch-matching, `main` fallback) on a **stable** toolchain and runs `cargo run … -- lint framework --framework-root .` plus a spec-runtime symlink check — it does not invoke `make lint` (the local nightly `-Zscript` resolver). See [docs/contributing/checks.md](docs/contributing/checks.md#binding-to-a-specify-source) for the local-vs-CI split.

Full evals guidance, including the scenario packs under [`evals/`](evals/README.md), lives in [docs/contributing/evals.md](docs/contributing/evals.md).

## Skill authoring

Skill authoring rules — markdown style, description grammar, argument-hint grammar, 200/45/512 caps, skill body discipline, cross-cutting guardrails, envelope examples — live in [docs/standards/skill-authoring.md](docs/standards/skill-authoring.md) (with the long-form rationale under `## Rationale`) and [.cursor/rules/project.mdc](.cursor/rules/project.mdc#skill-authoring-conventions). Framework checks are [`CORE-*` rules](adapters/shared/rules/core/) resolved by a generic `specify lint framework` dispatcher. Each rule is either a **declarative hint** (Road A — `kind:` ∈ `schema | reference-resolves | cardinality | set-coverage | set-eq | constant-eq | content-digest-eq | unique | fenced-block | regex | path-pattern | presence | field-grammar | cross-reference | cli-contract`, interpreted over the workspace model) or a **name-resolved WASI tool** (Road B — `kind: tool`, e.g. the `rules` / `scenarios` / `skill-body` / `agent-teams` / `links-registry` / `marketplace` / `prose` family tools). All policy (caps, allow-lists, owner maps, expected sets) lives in the rule's `config:`, never in the engine. The `kind: authoring-predicate` bridge has been fully removed — no imperative bridge remains. Enforced strictly by `make lint` — every check fails on the first violation, with no per-file grandfathering. Extension model: [docs/contributing/checks.md](docs/contributing/checks.md).

## Gotchas

- In a fresh clone, run `/spec:init` before using other `/spec:*` commands. The workflow skills expect the `.specify/` project structure to exist.
- `specify lint framework` enforces documentation consistency; if you remove or rename workflow terms, update the checks in the same change.
- **Adapter names are unique across axes** — a name appears under `adapters/sources/<name>/` xor `adapters/targets/<name>/`, never both. Collisions surface as `adapter-name-axis-collision` at `specify init` and at first resolve. See [DECISIONS.md §"Adapter name uniqueness"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-name-uniqueness).
- **First-party adapters resolve project-locally or from GitHub** — `specify init <adapter>` accepts a first-party shorthand (`omnia`, `omnia@v1`; ref defaults to `v1`) that resolves to the published adapter on GitHub (a networked sparse checkout). The adapter resolver itself is project-local only — manifest-cache mirror then vendored `adapters/` tree — with no environment-variable fallback to an out-of-tree framework checkout. See [DECISIONS.md §"Adapter loader axis routing"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-loader-axis-routing) and §"First-party `<adapter>` shorthand at init".
- Target review briefs symlink `agent-teams.md` from each adapter's `references/` directory to the shared `adapters/shared/references/runtime/review-team-protocol.md` overlay, which resolves to the canonical `docs/reference/review-team-protocol.md`. If a symlink target is removed, the brief's documentation may reference content that no longer resolves.
- Crossing a major is a hard cut: no silent compatibility aliases for old manifests, verbs, brief paths, or slash-namespaces. Crossing a major runs through a registered migrator, not an alias — each major version bump requires a registered `MigrationKind` (with a `Migrator` impl + golden fixture) before `specify_version` rolls, so migration is a covered routine step via `specify migrate` rather than a flag-day break. See the cli repo's [`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) "Bootstrap, upgrade, and migration lifecycle" decision.

## Related coding standards

- CLI binary and crate conventions (errors, DTOs, hint colocation, brevity) live in the CLI repo's [AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) and [docs/standards/](https://github.com/augentic/specify-cli/blob/main/docs/standards/). Skills that shell out to `specify` rely on the kebab-case `error` discriminants documented there.
