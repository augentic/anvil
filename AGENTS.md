# Augentic Plugins - Agent Instructions

## Cursor Cloud specific instructions

This is a **documentation/prompt-engineering repository**. The codebase consists of markdown skill definitions, reference docs, templates, and shell scripts. Generated Rust crates and Swift shells appear in downstream projects, not in this repository itself.

### Workflow overview

Humans are expected to work through stock Specify:

- `/spec:init` (once per project)
- `/spec:define`
- `/spec:build`
- `/spec:merge`
- `/spec:drop`
- `/spec:verify` (detect drift between code and baseline specs)
- `/spec:explore` (thinking partner for ideas and requirements)
- `/spec:status` (check artifact completion and task progress)
- `/spec:extract` (extract Specify artifacts from existing source code)
- `/spec:execute` (drive an initiative's `.specify/plan.yaml` through define → build → merge; RFC-2 Layer 2 — `--dry-run` scaffold + supervised single-change run, with three outcome paths: success → `done`, failure → `failed`, deferred → `blocked`. Self-heal on startup, `--loop`, and `sources`/`affects` wiring forthcoming.)

This repository provides specialist skills and references that support that workflow.

### Skill / CLI responsibility split

The phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`,
`/spec:status`, `/spec:init`) are agent-driven orchestrators. Every
deterministic operation — kebab-case name validation, `.metadata.yaml`
reads and writes, lifecycle transitions, schema and brief-pipeline
resolution, artifact-completion checks, spec-merge preview, baseline
conflict detection, delta merge, coherence validation, archive move —
runs through the `specify` CLI. The skill markdown drives the agent-side
work: eliciting user intent, reading brief bodies, writing artifacts,
invoking plugin skills (e.g. `/omnia:crate-writer`), and rendering
summaries.

CLI surface the skills depend on:

- `specify init` — scaffold `.specify/` and write `project.yaml`.
- `specify status` — list active changes and per-change progress.
- `specify change {create, list, status, transition, touched-specs, overlap, archive, drop}` — lifecycle verbs.
- `specify plan {validate, next, status, create, amend, transition, archive, lock}` — plan-level verbs backing RFC-2 Layer 1 (humans drive today; `/spec:execute` will drive in Layer 2). `plan lock {acquire, release, status}` manages the `.specify/plan.lock` PID stamp that the `/spec:execute` driver takes for the duration of a run.
- `specify schema {resolve, check, pipeline}` — schema resolution and brief topology.
- `specify spec {preview, conflict-check}` — dry-run merge operations and baseline drift detection.
- `specify validate` — structural + semantic artifact checks.
- `specify task {progress, mark}` — task progress and checkbox flips.
- `specify merge` — commit spec merge + archive.

Never hand-edit `.metadata.yaml`, never `mkdir -p .specify/...`, and never
`mv` anything into `.specify/archive/`. Route through the CLI — it
enforces the legal set of lifecycle states and validates inputs in one
place for humans, agents, and CI alike.

### Plan-driven loop (RFC-2 Layer 1)

When an initiative is coordinated through a `.specify/plan.yaml`, the
loop is hand-driven today:

1. `specify plan next` — pick the next eligible entry.
2. `specify plan transition <name> in-progress` — claim it.
3. `/spec:define` → `/spec:build` → `/spec:merge` (or `/spec:drop`) as usual.
4. `specify plan transition <name> {done, failed, blocked}` — close the loop.

The phase skills themselves stay unaware of the plan — they operate
change-by-change. Plan *entries* are only ever written via `specify
plan create` / `specify plan amend`; plan *status* is only ever written
via `specify plan transition`. A phase that discovers a neighbouring
change mid-run (e.g. a define brief uncovering a bug fix that should be
tracked) may shell out to `specify plan create` / `specify plan amend`
— the same commands humans run. RFC-2 Layer 2 will land `/spec:execute`
as an automated driver against this same CLI surface; the hand-driven
path remains the fallback. See [rfcs/rfc-2-execution.md](rfcs/rfc-2-execution.md).

### Commands

All commands are run from the repository root:

- **`make checks`** -- runs `scripts/checks.ts` via Deno for documentation and workflow consistency checks
- **`make dev-plugins`** -- symlink local plugins into Cursor for development/testing
- **`make prod-plugins`** -- restore Augentic marketplace plugins (reload Cursor after either)

### Gotchas

- In a fresh clone, run `/spec:init` before using other `/spec:*` commands. The workflow skills expect the `.specify/` project structure to exist.
- `checks.ts` enforces documentation consistency; if you remove or rename workflow terms, update the checks in the same change.
- Some skills use symlinks to share reference documents from `plugins/references/`. If a symlink target is removed, the skill's documentation may reference content that no longer resolves.
