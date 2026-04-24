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
- `/spec:execute` (drive an initiative's `.specify/plan.yaml` through define → build → merge; RFC-2 Layer 2, fully landed — `--dry-run` preview, supervised single-change run, self-heal on startup, `--loop` mode with terminal summary + SIGINT/SIGTERM handling, and `sources` / `affects` execution wiring)
- `/spec:plan` (author `.specify/plan.yaml` via `pipeline.plan`; RFC-2 Layer 3 + RFC-3a + RFC-3b — discovery through `/spec:analyze`, optional **sync-peers** when `.specify/registry.yaml` declares multiple projects (`specify workspace sync` + `workspace.md`), propose with glob or **manifest** scopes (Stage C), **project assignment** step for multi-repo plans (RFC-3b: infers `project` per entry from registry descriptions, writes via `specify plan amend --project`), `.specify/plans/<name>/` artefacts archived with the plan; see [rfcs/rfc-3a-monoliths.md](rfcs/archive/rfc-3a-monoliths.md) and [rfcs/rfc-3b-platform.md](rfcs/rfc-3b-platform.md))

This repository provides specialist skills and references that support that workflow.

### Skill / CLI responsibility split

The phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/spec:status`, `/spec:init`) are agent-driven orchestrators. Every deterministic operation — kebab-case name validation, `.metadata.yaml` reads and writes, lifecycle transitions, schema and brief-pipeline resolution, artifact-completion checks, spec-merge preview, baseline conflict detection, delta merge, coherence validation, archive move — runs through the `specify` CLI. The skill markdown drives the agent-side work: eliciting user intent, reading brief bodies, writing artifacts, invoking plugin skills (e.g. `/omnia:crate-writer`), and rendering summaries.

CLI surface the skills depend on:

- `specify init` — scaffold `.specify/` and write `project.yaml`.
- `specify status` — list active changes and per-change progress.
- `specify change {create, list, status, transition, touched-specs, overlap, archive, drop, phase-outcome, journal-append}` — lifecycle verbs. `phase-outcome` stamps the `.metadata.yaml:outcome` that `/spec:execute` reads; `journal-append` writes `question` / `failure` / `recovery` entries into `journal.yaml`.
- `specify plan {init, validate, next, status, create, amend, transition, archive, lock}` — plan CRUD and lifecycle (RFC-2 Layer 1 + RFC-3a). `init` scaffolds an empty plan; `lock {acquire, release, status}` manages `.specify/plan.lock` for `/spec:execute`.
- `specify initiative {brief, registry}` — operator brief and platform registry. `brief {init, show}` owns `.specify/initiative.md`; `registry {show, validate}` owns `.specify/registry.yaml`.
- `specify workspace {sync, status, push}` — materialises `.specify/workspace/<peer>/` for multi-repo planning; pushes workspace clones to remotes after execution.
- `specify schema {resolve, check, pipeline}` — schema resolution and brief topology.
- `specify spec {preview, conflict-check}` — dry-run merge operations and baseline drift detection.
- `specify validate` — structural + semantic artifact checks.
- `specify task {progress, mark}` — task progress and checkbox flips.
- `specify merge` — commit spec merge + archive.

Never hand-edit `.metadata.yaml`, never `mkdir -p .specify/...`, and never `mv` anything into `.specify/archive/`. Route through the CLI — it enforces the legal set of lifecycle states and validates inputs in one place for humans, agents, and CI alike.

### Plan-driven loop (RFC-2, all three layers landed)

When an initiative is coordinated through a `.specify/plan.yaml`, the recommended path is:

1. **Author.** `/spec:plan <initiative-name> --source <key>=<path-or-url> ...` — Layer 3 skill runs `pipeline.plan` briefs, optionally **sync-peers** + `workspace.md` when the registry is multi-project, then `specify plan init` + one `specify plan create` per accepted slice (globs or `--scope-manifest` per RFC-3a Stage C).
2. **Execute.** `/spec:execute --loop` — Layer 2 driver that repeatedly picks `specify plan next`, runs `/spec:define → /spec:build → /spec:merge` on the chosen entry, reads the phase outcome off `.metadata.yaml`, and transitions the plan entry to `done` / `failed` / `blocked`. Exits on `all-done`, `stuck`, self-heal halt, or SIGINT/SIGTERM.
3. **Archive.** `specify plan archive` sweeps `plan.yaml` and the `.specify/plans/<name>/` authoring trail into `.specify/archive/plans/<YYYYMMDD>-<name>/`.

Hand-driven fallback (RFC-2 Layer 1): skip `/spec:plan` and `/spec:execute`, author `plan.yaml` entry-by-entry with `specify plan {init, create, amend}`, and drive the loop yourself via `specify plan next → transition in-progress → /spec:define → /spec:build → /spec:merge → transition done`.

The phase skills themselves stay unaware of the plan — they operate change-by-change. Plan *entries* are only ever written via `specify plan create` / `specify plan amend`; plan *status* is only ever written via `specify plan transition`. A phase that discovers a neighbouring change mid-run (e.g. a define brief uncovering a bug fix that should be tracked) may shell out to `specify plan create` / `specify plan amend` — the same commands humans run. See [rfcs/archive/rfc-2-execution.md](rfcs/archive/rfc-2-execution.md) for the full design.

### Commands

All commands are run from the repository root:

- **`make checks`** -- runs `scripts/checks.ts` via Deno for documentation and workflow consistency checks
- **`make dev-plugins`** -- symlink local plugins into Cursor for development/testing
- **`make prod-plugins`** -- restore Augentic marketplace plugins (reload Cursor after either)

### Gotchas

- In a fresh clone, run `/spec:init` before using other `/spec:*` commands. The workflow skills expect the `.specify/` project structure to exist.
- `checks.ts` enforces documentation consistency; if you remove or rename workflow terms, update the checks in the same change.
- Some skills use symlinks to share reference documents from `plugins/references/`. If a symlink target is removed, the skill's documentation may reference content that no longer resolves.
