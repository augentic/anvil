---
name: specify-init
description: Initialize Specify in a project. Bootstraps the `specify` CLI when missing, picks between a regular single-project init and a registry-only workspace, then invokes `specify init <adapter>` or `specify init --workspace` to scaffold `.specify/`, write `project.yaml`, run an initial `workspace sync`, and generate starter `AGENTS.md`. Use when first wiring up a project before any other `/spec:*` command; supports first-run init, re-entry upgrades, plugin-cache refresh, and major-version migration handoff.
argument-hint: <adapter|workspace>
---

# Specify Init

## Critical Path

1. **Verify the CLI** — run `specify --version`; install with `cargo install --git https://github.com/augentic/specify-cli` only after explicit user confirmation.
1b. **Probe CLI version** — run `specify upgrade --dry-run --format json`; report drift and, on consent, run `specify upgrade --yes`.
1c. **Probe plugin cache** — run `specify plugins doctor --format json`; report drift and, on consent, run `specify plugins refresh --yes`, then stop for a Cursor restart.
1d. **Probe artifact major** — run `specify init --check-migration --format json`; report drift and, on consent, run `specify migrate --yes` (a no-op pass-through today).
2. **Check existing initialization** — detect `.specify/project.yaml`, ask before reinitializing, and route reinit through `specify init --upgrade`.
3. **Choose topology** — decide regular project vs registry-only workspace; adapter is required for regular projects and forbidden in workspace mode. When `$ARGUMENTS[0]` is the literal `workspace`, treat that as workspace init.
4. **Resolve metadata** — choose `$ADAPTER`, the project name, and an optional description; never pre-populate `.specify/cache/`.
4b. **Elicit platforms** — when the resolved target adapter declares `platforms.required` (e.g. vectis), prompt the operator for the platform set. Offer the adapter manifest's `default` set (e.g. `core,ios,android`) as the suggested value, name the `allowed` set (e.g. `core,ios,android,web,desktop`), and note that `core` is mandatory. Store the result as `$PLATFORMS`. When the target does not require platforms, skip this step.
5. **Invoke `specify init`** — run either `specify init "$ADAPTER" ${PLATFORMS:+--platforms "$PLATFORMS"} ...` or `specify init --workspace ...`; the CLI scaffolds files, chains `workspace sync` on workspace init, and generates starter context. Do not call `specify workspace sync` separately after workspace init. Surface non-zero CLI errors without hand-rolling scaffold files.
6. **Offer baseline extraction** — for regular projects with code indicators, ask whether to create `initial-baseline`; skip this entirely for workspaces.
7. **Summarize the correct shape** — report regular vs workspace outputs (including the init envelope's `workspace-sync-message` when present), next actions, and any baseline-extraction handoff.

## Orientation

`/spec:init` selects between two on-disk shapes per run. A **regular project** carries code and `.specify/` together; the CLI scaffolds `slices/`, `specs/`, `archive/`, `cache/`, and a `project.yaml` whose `adapter:` field drives every downstream pipeline. A **workspace** carries only platform state (`registry.yaml`, later `change.md` / `plan.yaml`, workspace slots under `.specify/workspace/`); `project.yaml` records `workspace: true` with no `adapter:`, and phase pipelines are disabled on the workspace itself.

Adapter vs `--workspace` is mutually exclusive: `specify init` with neither, or both, exits `2` with clap's standard parse-error diagnostic. A regular project must declare an adapter; a workspace must use `--workspace` and never carries an `adapter:`. The literal token `workspace` is reserved — it is not a target adapter name.

The CLI owns every filesystem write — `.specify/`, `project.yaml`, the resolved adapter cache, root `AGENTS.md`, and `.specify/context.lock`. When `AGENTS.md` already exists, the CLI preserves it byte-for-byte. The skill never hand-rolls scaffold files; on non-zero exit it surfaces the CLI error and stops.

After a regular init, the skill optionally detects existing code indicators (`Cargo.toml`, `package.json`, `src/`, etc.) and offers to create an `initial-baseline` slice via `specify slice create`. Workspace init skips that step entirely — a workspace never carries code. The four render templates (greenfield / brownfield / workspace / migrated) live in [`../../references/init-output-templates.md`](../../references/init-output-templates.md).

See [`references/init-runbook.md`](references/init-runbook.md) for the operational detail and [`../../references/init-output-templates.md`](../../references/init-output-templates.md) for the rendered greenfield, brownfield, workspace, and migrated outputs.

## Guardrails

- **`/spec:init` is the one Specify skill that may install the CLI.** Install only after explicit user confirmation via `cargo install --git https://github.com/augentic/specify-cli`, always verify `specify --version` before invoking `specify init`, and never overwrite `project.yaml` without user confirmation.
- **`/spec:init` is the one Specify skill that may upgrade the CLI.** Run `specify upgrade --yes` only after the version probe reports drift and the operator consents; on `channel: unknown`, surface the manual-upgrade guidance and never auto-run.
- **`/spec:init` is the one Specify skill that may refresh the Cursor plugin cache.** Run `specify plugins refresh --yes` only after the doctor probe reports drift and the operator consents, then stop and tell them to restart Cursor and re-run `/spec:init` — never continue, the cache repopulates on restart.
- **`/spec:init` is the one Specify skill that may trigger a major-version migration.** Run `specify migrate --yes` only after the check-migration probe reports `needs-migration: true` and the operator consents; the CLI owns the migration, this skill only orchestrates consent.
- **The CLI is the single writer for `.specify/`, `project.yaml`, root `AGENTS.md`, and `.specify/context.lock`.** On non-zero exit, surface the error and stop; never hand-roll the scaffold. Never pre-populate `.specify/cache/` — `specify init` owns adapter fetch when invoked with the adapter positional.
- **Adapter vs `--workspace` is mutually exclusive.** Pass `$ADAPTER` as the first positional for regular projects, or `/spec:init workspace` / `specify init --workspace` for workspaces. Workspace init refuses to run over an existing `.specify/`; converting a regular project to a workspace requires the operator to remove `.specify/` first.
