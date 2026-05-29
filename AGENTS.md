# Augentic Plugins - Agent Instructions

This is a **documentation/prompt-engineering repository**. The codebase consists of markdown skill definitions, reference docs, templates, and shell scripts. Generated Rust crates and Swift shells appear in downstream projects, not in this repository itself.

## Vocabulary

Specify 2.0 names two adapter roles and three workflow nouns. Use the terms verbatim.

### Adapter roles

- **source adapter** — input role with two operations: `survey` (plan time) and `extract` (slice time). Lives at `adapters/sources/<name>/adapter.yaml`. Examples: `intent`, `documentation`, `code-typescript`, `screenshots`, `captures`.
- **target adapter** — output role with three operations: `shape` (read by core synthesis), `build`, and `merge`. Lives at `adapters/targets/<name>/adapter.yaml`. Replaces the unqualified 1.x "adapter". Examples: `omnia`, `vectis`, `contracts`. See [`docs/explanation/adapter-anatomy.md`](docs/explanation/adapter-anatomy.md) for the full source / target contract, including the [adapter-vs-Cursor-plugin manifest boundary](docs/explanation/adapter-anatomy.md#adapter-manifests-vs-cursor-plugin-manifests).
- **plugin** — historical shorthand for the shared adapter shape. The Rust loaders are `SourceAdapter::resolve(name, project_dir)` and `TargetAdapter::resolve(name, project_dir)` in [`crates/domain/src/adapter/`](https://github.com/augentic/specify-cli/tree/main/crates/domain/src/adapter); each validates against the matching per-axis `source.schema.json` / `target.schema.json` distributed with the CLI. The noun "plugin" survives in operator-facing prose where source + target authors share the same audience tag.

### Synthesis terms

- **lead** — slice-sized unit emitted by `survey`; one block per lead under `## Lead inventory` in `discovery.md`, with stable `id` and `sources[]`.
- **evidence** — per-source result of `extract`; structured document with `claims:` persisted to `.specify/slices/<slice>/evidence/<source-key>.yaml`.
- **provenance** — the sources behind one requirement (the `Sources:` list in `spec.md`).
- **conflict / divergence** — unresolvable vs authority-resolved disagreement; surfaced inline as `[conflict]` / `[divergence]` tags on requirement headers.
- **authority** — closed enum (`intent` > `documentation` > `behaviour`) controlling who wins a disagreement.
- **reconciliation.yaml** — reconciliation index at `.specify/slices/<slice>/reconciliation.yaml`. Audit-only; `spec.md` is the authoritative artifact. See [`plugins/spec/references/synthesis/reconciliation.md`](plugins/spec/references/synthesis/reconciliation.md) for the reconciliation-index shape and audit posture.
- **cache fingerprints** — closed five-input key for the extraction cache (source path, adapter name@version, brief sha256, sorted tool versions, lead id). See [`plugins/spec/references/synthesis/claim-reconciliation.md`](plugins/spec/references/synthesis/claim-reconciliation.md) and the CLI extraction-cache implementation for the stable cache inputs.
- **component catalog** — operator-curated file at `.specify/design-system/components.yaml` declaring shared UI components (`status: confirmed | rejected`). The Vectis target reads the catalog at build time and factors shared component code per shell tree. Follows the same pattern as `tokens.yaml` and `assets.yaml`. Opt-in; absent catalog means no component factoring. Validated by `specrun slice validate` (`slice-catalog-drift`) and `specrun tool run vectis -- validate composition` (catalog cross-reference check). See [docs/explanation/components.md](docs/explanation/components.md).

### Workflow nouns

- **slice** — the single unit that flows through the fixed `refine → build → merge` loop. Each slice has its own proposal, spec, design, tasks, and merge step. Lives at `.specify/slices/<name>/`. Driven by `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop` and the `specrun slice *` CLI verbs.
- **change** — the operator-defined umbrella that coordinates one or more slices through `change.md` + `plan.yaml`. Driven by `/spec:plan`, `/spec:execute`, `/spec:finalize` and the `specrun plan *` CLI verbs. `change` is on-disk vocabulary in 2.0, not a slash-command namespace.

Use *slice loop* for the per-slice lifecycle; reserve *change* for the on-disk umbrella that owns `change.md` and `plan.yaml`.

### Workflow, standards, and artifacts

Specify separates three concerns. Use the terms verbatim; see [docs/explanation/standards-layer.md](docs/explanation/standards-layer.md) for the full picture.

| Layer | Role | Examples |
| --- | --- | --- |
| **Workflow** | Phase orchestration and lifecycle transitions | `/spec:plan`, `/spec:execute`, `specrun slice transition` |
| **Artifacts** | Slice-local and baseline product intent | `spec.md`, `plan.yaml`, `.specify/specs/` |
| **Engineering standards** | Durable policy that outlives any slice | Rules under `adapters/**/rules/`; `specrun rules export` and `specrun lint` |

**Authoring standards** (`docs/standards/`, enforced by `specdev lint` / `make lint` on this repo) govern skill and doc house style. **Engineering standards** (rules under `adapters/**/rules/`, exported by `specrun rules export` and enforced by `specrun lint`) govern generated and hand-written code in consumer projects. Do not conflate them.

`specrun lint` is CI-native **standards enforcement**, not a workflow phase — findings may block CI but never transition plans or slices. Build-time `REVIEW.md` and plan Gate 1 `approved` are separate surfaces.

### Authority and reconciliation mechanics

The full mechanics — per-kind authority overrides, per-slice operator overrides, reconciliation-index shape, cache-fingerprint inputs, extraction-cache layout — live in the cli repo's [`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md). The headline rules:

- **Authority resolution order** — per-slice override → per-Evidence per-kind override → Evidence document-level `authority:` → conflict. See [`plugins/spec/references/synthesis/authority.md`](plugins/spec/references/synthesis/authority.md) for the resolution order and override surfaces.
- **`captures` source adapter** — consumes runtime capture trees and emits `kind: example` Evidence claims with `replay-digest: sha256:…` anchors and default `authority: behaviour`.
- **Authority-override authoring** — `specrun plan amend --authority-override <slice> <kind>=<key>`; orphan source keys are rejected by `specrun slice validate` with `slice-authority-override-orphan-source-key`.
- **Reconciliation drift** — `specrun slice validate` catches REQ-id and contributing-claim drift under `slice-reconciliation-drift`.
- **Adapter opt-out of extraction cache** — `cache: opt-out` on `adapter.yaml`.

## Workflow overview

The default rhythm is `/spec:plan` → operator stamps `approved` → `/spec:execute` → `/spec:finalize`. Slash commands operators reach for, in the order they appear in a project's life:

- `/spec:init` — scaffold `.specify/`, run once per project.
- `/spec:plan` — author `change.md` and `plan.yaml`: survey each bound source, propose `slices[]` rows by reconciling leads across sources, validate the plan. Exits at `plan.lifecycle: pending` and prints the literal `specrun plan transition <name> approved` command.
- `specrun plan transition <name> approved` — **Gate 1.** Operator-only stamp; `/spec:plan` never writes `approved` itself.
- `/spec:execute` — refuses unless the plan is `approved`; loops `specrun plan next` → `/spec:refine` → `/spec:build` → `/spec:merge` until every per-entry `status` is `done`.
- `/spec:refine` — breakout: for one slice, run `extract` per bound source, synthesize `proposal.md` / `spec.md` / `design.md` / `tasks.md`, validate, transition to `refined`.
- `/spec:build` — breakout: validate artifacts, implement the slice's tasks.
- `/spec:merge` — breakout: fold the slice's deltas into the baseline and archive it; the only writer of per-entry `done`.
- `/spec:drop` — abandon a slice without merging.
- `/spec:finalize` — push branches, observe PR state, run `specrun plan archive` once every PR is `MERGED`.

N=1 is degenerate, not special: `intent.survey` produces one lead, the operator stamps `approved`, and `/spec:execute` drives the same single-slice rhythm as a 12-slice change.

## Skill / CLI responsibility split

Phase skills are agent-driven orchestrators. Every deterministic operation — manifest validation, `.metadata.yaml` reads and writes, plan and slice lifecycle transitions, source and target resolution, artifact-completion checks, baseline conflict detection, delta merge, archive move — runs through the `specify` CLI. Skill markdown drives the agent-side work: eliciting operator intent, reading brief bodies, writing evidence and synthesized artifacts, invoking specialist skills (e.g. `/omnia:crate-writer`), and rendering summaries.

The CLI surface skills depend on is documented in [`specify` `--help`](https://github.com/augentic/specify-cli). The headline groups: `specrun init`, `specrun source {resolve}`, `specrun target {resolve}`, `specrun slice {create, transition, validate, merge}`, `specrun plan {create, add, amend, transition, next, archive}`, `specrun workspace {sync, push, prepare}`, and `specrun tool run` (WASI tool dispatch — `contract`, `vectis`, …).

Never hand-edit `.metadata.yaml`, `project.yaml`, `plan.yaml`, `discovery.md`, `sources.yaml`, or `targets.yaml`; never `mkdir -p .specify/...`; never `mv` anything into `.specify/archive/`. Route through the CLI — it enforces the legal lifecycle set and validates inputs in one place for humans, agents, and CI.

## Contracts target adapter

The contracts target adapter owns API contract authoring, import, and validation. Its `build` brief runs the OpenAPI, AsyncAPI, and JSON Schema format sub-flows, each with author / import / verify references under `adapters/targets/contracts/references/`.

The matching CLI validation surface is the declared `contract` WASI tool, run via `specrun tool run contract -- "$PROJECT_ROOT/contracts" --format json`.

## Plan-driven loop

`/spec:plan` authors the plan and exits at Gate 1; the operator stamps `approved`; `/spec:execute` drives the loop; `/spec:finalize` closes it. Plan *entries* are only ever written via `specrun plan add` / `specrun plan amend`; plan *lifecycle* is only ever written via `specrun plan transition`; per-entry `in-progress` is only ever written by `specrun plan next`; per-entry `done` is only ever written by `specrun slice merge`. Per-entry status walks backwards only via `specrun plan transition <entry> --undo`, which refuses to skip rungs (`done → in-progress`, then a second call for `in-progress → pending`) and fires one `plan.transition.undone` journal event per rung. The phase skills themselves stay unaware of the plan — they operate slice-by-slice. Hand-driven fallback: `specrun plan next` → `/spec:refine` → `/spec:build` → `/spec:merge`, repeat until drained.

## Commands

All commands are run from the repository root:

- `make lint` — forwards to `specdev lint` (`cargo run --release --manifest-path ../specify-cli/Cargo.toml --bin specdev -- lint --framework-root .`) for documentation and workflow consistency checks.
- `make use-local-plugins` / `make use-team-plugins` — choose plugin source (reload Cursor after either).

The `specify-authoring` predicate regression suite is owned and run by `augentic/specify-cli` (its `cargo make test` runs the whole workspace, including `specify-authoring`); this repo's CI runs only `make lint` against the live tree.

Full acceptance guidance, including the manual cross-repo scenario, lives in [docs/contributing/acceptance.md](docs/contributing/acceptance.md).

## Skill authoring

Skill authoring rules — markdown style, description grammar, argument-hint grammar, 200/45/512 caps, skill body discipline, cross-cutting guardrails, envelope examples — live in [docs/standards/skill-authoring.md](docs/standards/skill-authoring.md) (with the long-form rationale under `## Rationale`) and [.cursor/rules/project.mdc](.cursor/rules/project.mdc#skill-authoring-conventions). Predicate implementations live in the `specify-authoring` crate in `augentic/specify-cli`. Enforced strictly by `specdev lint` (`make lint` locally) — every predicate fails on the first violation, with no per-file grandfathering.

## Gotchas

- In a fresh clone, run `/spec:init` before using other `/spec:*` commands. The workflow skills expect the `.specify/` project structure to exist.
- `specdev lint` enforces documentation consistency; if you remove or rename workflow terms, update the checks in the same change.
- **Adapter names are unique across axes** — a name appears under `adapters/sources/<name>/` xor `adapters/targets/<name>/`, never both. Collisions surface as `adapter-name-axis-collision` at `specrun init` and at first resolve. See [DECISIONS.md §"Adapter name uniqueness"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-name-uniqueness).
- Target review briefs symlink `agent-teams.md` from each adapter's `references/` directory to the canonical `docs/reference/review-team-protocol.md`. If a symlink target is removed, the brief's documentation may reference content that no longer resolves.
- 2.0 is a hard cut from 1.x. No compatibility aliases for old manifests, verbs, brief paths, or the retired `change:` slash-namespace.

## Related coding standards

- CLI binary and crate conventions (errors, DTOs, hint colocation, brevity) live in the CLI repo's [AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) and [docs/standards/](https://github.com/augentic/specify-cli/blob/main/docs/standards/). Skills that shell out to `specify` rely on the kebab-case `error` discriminants documented there.
