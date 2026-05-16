# Augentic Plugins - Agent Instructions

## Cursor Cloud specific instructions

This is a **documentation/prompt-engineering repository**. The codebase consists of markdown skill definitions, reference docs, templates, and shell scripts. Generated Rust crates and Swift shells appear in downstream projects, not in this repository itself.

### Vocabulary

Two lifecycle nouns recur throughout this codebase:

- **Slice** — the single unit that flows through the fixed `define → build → merge` loop. Each slice has its own proposal, specs, design, tasks, and merge step. Lives at `.specify/slices/<name>/`. Driven by `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop` and the `specify slice *` CLI verbs.
- **Change** — the operator-defined umbrella that coordinates one or more slices through `change.md` + `plan.yaml`. Driven by `/change:draft`, `/change:execute`, `/change:finalize`, the `specify change *` CLI verbs that own `change.md`, and the sibling `specify plan *` verbs that own the executable plan.

Use *slice loop* for the per-slice lifecycle; reserve *change* for the umbrella that owns `change.md` and `plan.yaml`.

### Workflow overview

Slash commands operators reach for, in roughly the order they appear in a project's life:

- `/spec:init` — scaffold `.specify/`, run once per project.
- `/spec:extract` — extract Specify artifacts from existing source code.
- `/spec:define` — author a new slice (proposal, design, specs, tasks).
- `/spec:build` — implement a slice's tasks.
- `/spec:merge` — fold a slice's deltas into the baseline and archive it.
- `/spec:drop` — abandon a slice without merging.
- `/change:draft` — author a change's `plan.yaml` via the planning brief pipeline; in multi-project hubs `sync-workspace` + `workspace.md` precede the propose step; for legacy-code changes `/change:survey` decomposes sources before propose.
- `/change:survey` — mechanically decompose `legacy-code` sources into surfaces and slice-sized candidates; invoked by `/change:draft` between sync-workspace and propose. Documentation-only changes skip this step entirely.
- `/change:execute` — drive a change's `plan.yaml` through define → build → merge; supports `dry-run`, single-slice supervised run, and `loop` mode with self-heal and SIGINT/SIGTERM handling.
- `/change:finalize` — push branches, observe PR state, run `specify change finalize` once every PR is `MERGED`.

For the three-layer composition (Layer 0 configuration → Layer 1 executing a change → Layer 2 planning a change, with the `specify` CLI as the substrate underneath) and the rename trail from earlier verb names, see [docs/explanation/decision-log.md](docs/explanation/decision-log.md).

### Skill / CLI responsibility split

Phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/spec:init`) are agent-driven orchestrators. Every deterministic operation — kebab-case validation, `.metadata.yaml` reads and writes, lifecycle transitions, capability and brief-pipeline resolution, artifact-completion checks, spec-merge preview, baseline conflict detection, delta merge, coherence validation, archive move — runs through the `specify` CLI. Skill markdown drives the agent-side work: eliciting user intent, reading brief bodies, writing artifacts, invoking plugin skills (e.g. `/omnia:crate-writer`), and rendering summaries.

The CLI surface skills depend on is documented in the [`specify` `--help`](https://github.com/augentic/specify-cli) output. The headline groups: `specify init`, `specify status`, `specify slice {…}` (per-slice verbs), `specify plan {…}` (plan CRUD + lifecycle), `specify change {draft, show, finalize}` (operator brief + canonical closure), `specify registry {add, remove, show, validate}`, `specify workspace {sync, status, push}`, `specify capability {resolve, pipeline}`, and `specify tool run` (WASI tool dispatch — `contract`, `vectis`, …).

Never hand-edit `.metadata.yaml`, never `mkdir -p .specify/...`, and never `mv` anything into `.specify/archive/`. Route through the CLI — it enforces the legal set of lifecycle states and validates inputs in one place for humans, agents, and CI alike.

### Contract skills

The contract plugin provides format-first specialist skills for API contract generation and validation. Each skill carries author / import / verify intents internally and dispatches via its own intent table:

- `/contract:openapi` — author, import, or verify HTTP / resource-style contracts (OpenAPI 3.1).
- `/contract:asyncapi` — author, import, or verify evented / pub-sub / streaming contracts (AsyncAPI 3.0).
- `/contract:json-schema` — author, import, or verify reusable payload schemas (JSON Schema).

The matching CLI surface is the declared `contract` WASI tool, run via `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json`. Cross-project consumer-impact classification is exposed separately as `specify compatibility`.

### Plan-driven loop

`/change:draft` authors the plan, `/change:execute loop` drives it, `/change:finalize` closes it. Plan *entries* are only ever written via `specify plan add` / `specify plan amend`; plan *status* is only ever written via `specify plan transition`. The phase skills themselves stay unaware of the plan — they operate slice-by-slice. Hand-driven fallback: skip `/change:draft` and `/change:execute` and drive the loop yourself via `specify plan next → transition in-progress → /spec:define → /spec:build → /spec:merge → transition done`.

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

### Related coding standards

- CLI binary and crate conventions (errors, DTOs, hint colocation, brevity) live in the CLI repo's [AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) and [docs/standards/](https://github.com/augentic/specify-cli/blob/main/docs/standards/). Skills that shell out to `specify` rely on the kebab-case `error` discriminants documented there.
