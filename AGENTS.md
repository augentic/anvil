# Augentic Plugins - Agent Instructions

## Cursor Cloud specific instructions

This is a **documentation/prompt-engineering repository**. The codebase consists of markdown skill definitions, reference docs, templates, and shell scripts. Generated Rust crates and Swift shells appear in downstream projects, not in this repository itself.

### Vocabulary

Specify 2.0 names two adapter roles and three workflow nouns. Use the terms verbatim:

- **source adapter** — input role with two operations: `enumerate` (plan time) and `extract` (slice time). Examples: `intent`, `documentation`, `code-typescript`, `screenshots`, `code-runtime` (RFC-27 §D1; consumes captured fixture trees and emits `kind: example` Evidence claims with `fixture-digest: sha256:…` anchors and default `authority: behaviour`). Lives at `adapters/sources/<name>/adapter.yaml`.
- **target adapter** — output role with three operations: `shape` (read by core synthesis), `build`, and `merge`. Examples: `omnia`, `vectis`, `contracts`. Lives at `adapters/targets/<name>/adapter.yaml`. Replaces the unqualified 1.x "adapter". Adapter names are unique across axes — the same `name` must not appear under both `adapters/sources/` and `adapters/targets/` (rejected at `specify init` and `*Adapter::resolve` time as `adapter-name-axis-collision`). See [`docs/explanation/adapter-anatomy.md`](docs/explanation/adapter-anatomy.md) for the full source / target contract, including the [adapter-vs-Cursor-plugin manifest boundary](docs/explanation/adapter-anatomy.md#adapter-manifests-vs-cursor-plugin-manifests).
- **plugin** — historical shorthand for the shared adapter shape. The Rust loader lives at `crates/domain/src/adapter/` (single entry point `Adapter::resolve(axis, name, project_dir)`) and validates manifests against the per-axis `source.schema.json` / `target.schema.json` distributed with the CLI. The vocabulary noun "plugin" stays in operator-facing prose where source + target authors share the same audience tag.
- **candidate** — slice-sized unit emitted by `enumerate`; one block per candidate under `## Candidate inventory` in `discovery.md`, with stable `id` and `sources[]`.
- **evidence** — per-source result of `extract`; structured document with `claims:` persisted to `.specify/slices/<slice>/evidence/<source-key>.yaml`.
- **provenance** — the sources behind one requirement (the `Sources:` list in `spec.md`).
- **conflict / divergence** — unresolvable vs authority-resolved disagreement; surfaced inline as `[conflict]` / `[divergence]` tags on requirement headers.
- **authority** — closed enum (`intent` > `documentation` > `behaviour`) controlling who wins a disagreement. RFC-27 §D2 promotes authority from a property of the Evidence document alone to a property of `(Evidence document, claim kind)` via optional per-kind `authority-overrides:` maps on Evidence files. RFC-27 §D3 adds per-slice operator overrides on `plan.yaml.slices[].authority-override` (claim-kind → source-key) authored by `specify plan amend --authority-override <slice> <kind>=<key>`; orphan source keys are rejected by `specify slice validate` with `slice-authority-override-orphan-source-key`. Resolution order: per-slice → per-Evidence → document-level → conflict.
- **fusion.yaml** — RFC-27 §D4 reconciliation index at `.specify/slices/<slice>/fusion.yaml`. One entry per `REQ-*` id in `spec.md` listing every `(source, claim-id)` pair the synthesis consulted, inline `value` payloads with a 16 KiB cap, `winner` markers on entries dropped by authority resolution, and a `resolution` enum (`single-source`, `single-value-agreement`, `authority-resolved`, `per-slice-override`, `unknown-no-evidence`, `tied-conflict`). Written atomically by `/spec:refine` between `tasks.md` and `slice validate`. Audit-only — `spec.md` remains the authoritative artifact. Inspect the file directly; the drift gate `specify slice validate` catches REQ-id and contributing-claim drift under `slice-fusion-drift`.
- **cache fingerprints** — RFC-27 §D8 makes every `extract` lookup key on the closed five-input fingerprint (source path canonicalised, adapter name@version, brief sha256, sorted declared-tool versions, candidate id). Recorded in `.specify/.cache/extractions/<adapter>/index.jsonl` (per-adapter, not per-axis — only source adapters extract); journal events `slice.extract.cache-hit` / `.cache-miss` carry the fingerprint and a closed `reason` enum on miss. Adapters opt out with `cache: opt-out` on `adapter.yaml`. The extraction-cache tree is disjoint from the per-axis manifest cache at `.specify/.cache/manifests/{sources,targets}/<name>/` — see [DECISIONS.md §"Cache layout"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#cache-layout).

Two workflow nouns recur throughout the codebase:

- **Slice** — the single unit that flows through the fixed `refine → build → merge` loop. Each slice has its own proposal, spec, design, tasks, and merge step. Lives at `.specify/slices/<name>/`. Driven by `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop` and the `specify slice *` CLI verbs.
- **Change** — the operator-defined umbrella that coordinates one or more slices through `change.md` + `plan.yaml`. Driven by `/spec:plan`, `/spec:execute`, `/spec:finalize` and the `specify plan *` CLI verbs. `change` is on-disk vocabulary in 2.0, not a slash-command namespace.

Use *slice loop* for the per-slice lifecycle; reserve *change* for the on-disk umbrella that owns `change.md` and `plan.yaml`.

### Workflow overview

The default rhythm is `/spec:plan` → operator stamps `reviewed` → `/spec:execute` → `/spec:finalize`. Slash commands operators reach for, in the order they appear in a project's life:

- `/spec:init` — scaffold `.specify/`, run once per project.
- `/spec:plan` — author `change.md` and `plan.yaml`: enumerate each bound source, propose `slices[]` rows by fusing candidates across sources, validate the plan. Exits at `plan.lifecycle: pending` and prints the literal `specify plan transition <name> reviewed` command.
- `specify plan transition <name> reviewed` — **Gate 1.** Operator-only stamp; `/spec:plan` never writes `reviewed` itself.
- `/spec:execute` — refuses unless the plan is `reviewed`; loops `specify plan next` → `/spec:refine` → `/spec:build` → `/spec:merge` until every per-entry `status` is `done`.
- `/spec:refine` — breakout: for one slice, run `extract` per bound source, synthesize `proposal.md` / `spec.md` / `design.md` / `tasks.md`, validate, transition to `refined`.
- `/spec:build` — breakout: validate artifacts, implement the slice's tasks.
- `/spec:merge` — breakout: fold the slice's deltas into the baseline and archive it; the only writer of per-entry `done`.
- `/spec:drop` — abandon a slice without merging.
- `/spec:finalize` — push branches, observe PR state, run `specify plan finalize` once every PR is `MERGED`.

N=1 is degenerate, not special: `intent.enumerate` produces one candidate, the operator stamps `reviewed`, and `/spec:execute` drives the same single-slice rhythm as a 12-slice change.

### Skill / CLI responsibility split

Phase skills are agent-driven orchestrators. Every deterministic operation — manifest validation, `.metadata.yaml` reads and writes, plan and slice lifecycle transitions, source and target resolution, artifact-completion checks, baseline conflict detection, delta merge, archive move — runs through the `specify` CLI. Skill markdown drives the agent-side work: eliciting operator intent, reading brief bodies, writing evidence and synthesized artifacts, invoking specialist skills (e.g. `/omnia:crate-writer`), and rendering summaries.

The CLI surface skills depend on is documented in [`specify` `--help`](https://github.com/augentic/specify-cli). The headline groups: `specify init`, `specify source {resolve}`, `specify target {resolve}`, `specify slice {create, transition, validate, merge}`, `specify plan {create, add, amend, transition, next, finalize}`, `specify workspace {sync, push, prepare}`, and `specify tool run` (WASI tool dispatch — `contract`, `vectis`, …).

Never hand-edit `.metadata.yaml`, `project.yaml`, `plan.yaml`, `discovery.md`, `sources.yaml`, or `targets.yaml`; never `mkdir -p .specify/...`; never `mv` anything into `.specify/archive/`. Route through the CLI — it enforces the legal lifecycle set and validates inputs in one place for humans, agents, and CI.

### Contracts target adapter

The contracts target adapter owns API contract authoring, import, and validation. Its `build` brief runs the OpenAPI, AsyncAPI, and JSON Schema format sub-flows, each with author / import / verify references under `adapters/targets/contracts/references/`.

The matching CLI validation surface is the declared `contract` WASI tool, run via `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json`.

### Plan-driven loop

`/spec:plan` authors the plan and exits at Gate 1; the operator stamps `reviewed`; `/spec:execute` drives the loop; `/spec:finalize` closes it. Plan *entries* are only ever written via `specify plan add` / `specify plan amend`; plan *lifecycle* is only ever written via `specify plan transition`; per-entry `in-progress` is only ever written by `specify plan next`; per-entry `done` is only ever written by `specify slice merge`. The phase skills themselves stay unaware of the plan — they operate slice-by-slice. Hand-driven fallback: `specify plan next` → `/spec:refine` → `/spec:build` → `/spec:merge`, repeat until drained.

### Commands

All commands are run from the repository root:

- `make checks` — runs `scripts/checks.ts` via Deno for documentation and workflow consistency checks.
- `make test` — direct cross-repo Deno acceptance test (skips cleanly when no suitable binary is available).
- `make use-local-plugins` / `make use-team-plugins` — choose plugin source (reload Cursor after either).

The cross-repo test requires a built `specify` binary. Set `SPECIFY_BIN=/absolute/path/to/specify-cli/target/release/specify` (the system PATH `specify` is typically the older v0.1.0 install and the test will skip against it). Full operator guide: [docs/contributing/acceptance.md](docs/contributing/acceptance.md).

### Skill authoring

Skill authoring rules — markdown style, description grammar, argument-hint grammar, 200/45/512 caps, skill body discipline, cross-cutting guardrails, envelope examples — live in [docs/standards/skill-authoring.md](docs/standards/skill-authoring.md) (with the long-form rationale under `## Rationale`) and [.cursor/rules/project.mdc](.cursor/rules/project.mdc#skill-authoring-conventions). Predicate implementations live in [scripts/checks/](scripts/checks/). Enforced strictly by `make checks` — every predicate fails on the first violation, with no per-file grandfathering.

### Gotchas

- In a fresh clone, run `/spec:init` before using other `/spec:*` commands. The workflow skills expect the `.specify/` project structure to exist.
- `checks.ts` enforces documentation consistency; if you remove or rename workflow terms, update the checks in the same change.
- Some skills use symlinks to share reference documents from `plugins/references/`. If a symlink target is removed, the skill's documentation may reference content that no longer resolves.
- 2.0 is a hard cut from 1.x. No compatibility aliases for old manifests, verbs, brief paths, or the retired `change:` slash-namespace.

### Related coding standards

- CLI binary and crate conventions (errors, DTOs, hint colocation, brevity) live in the CLI repo's [AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) and [docs/standards/](https://github.com/augentic/specify-cli/blob/main/docs/standards/). Skills that shell out to `specify` rely on the kebab-case `error` discriminants documented there.
