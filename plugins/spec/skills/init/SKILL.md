---
name: specify-init
description: Initialize Specify in a project. Bootstraps the `specrun` CLI when missing, picks between a regular single-project init and a registry-only platform hub, then invokes `specrun init <adapter>` or `specrun init --hub` to scaffold `.specify/`, write `project.yaml`, and generate starter `AGENTS.md`. Use when first wiring up a project before any other `/spec:*` command; not for re-initializing an existing `.specify/`.
argument-hint: <adapter>
---

# Specify Init

## Critical Path

1. **Verify the CLI** — run `specrun --version`; install with `cargo install --git https://github.com/augentic/specify-cli` only after explicit user confirmation.
2. **Check existing initialization** — detect `.specify/project.yaml`, ask before reinitializing, and treat reinit as an upgrade path owned by the CLI.
3. **Choose topology** — decide regular project vs registry-only platform hub; adapter is required for regular projects and forbidden in hub mode.
4. **Resolve metadata** — choose `$ADAPTER`, the project name, and an optional description; never pre-populate `.specify/.cache/`.
5. **Invoke `specrun init`** — run either `specrun init "$ADAPTER" ...` or `specrun init --hub ...`; let the CLI scaffold files and generate starter context, and surface non-zero CLI errors without hand-rolling scaffold files.
6. **Offer baseline extraction** — for regular projects with code indicators, ask whether to create `initial-baseline`; skip this entirely for hubs.
7. **Summarize the correct shape** — report regular vs hub outputs, next actions, and any baseline-extraction handoff.

## Orientation

`/spec:init` selects between two on-disk shapes per run. A **regular project** carries code and `.specify/` together; the CLI scaffolds `slices/`, `specs/`, `archive/`, `.cache/`, and a `project.yaml` whose `adapter:` field drives every downstream pipeline. A **platform hub** carries only platform state (`registry.yaml`, later `change.md` / `plan.yaml` / `workspace/`); `project.yaml` records `hub: true` with no `adapter:`, and phase pipelines are disabled on the hub itself.

Adapter vs `--hub` is mutually exclusive: `specrun init` with neither, or both, exits `2` with clap's standard parse-error diagnostic. A regular project must declare a adapter; a hub must declare `--hub` and never carries a `adapter:`.

The CLI owns every filesystem write — `.specify/`, `project.yaml`, the resolved adapter cache, root `AGENTS.md`, and `.specify/context.lock`. When `AGENTS.md` already exists, the CLI preserves it byte-for-byte. The skill never hand-rolls scaffold files; on non-zero exit it surfaces the CLI error and stops.

After a regular init, the skill optionally detects existing code indicators (`Cargo.toml`, `package.json`, `src/`, etc.) and offers to create an `initial-baseline` slice via `specrun slice create`. Hub init skips that step entirely — a hub never carries code. The three render templates (greenfield / brownfield / hub) live in [`../../references/init-output-templates.md`](../../references/init-output-templates.md).

See [`references/init-runbook.md`](references/init-runbook.md) for the operational detail and [`../../references/init-output-templates.md`](../../references/init-output-templates.md) for the rendered greenfield, brownfield, and hub outputs.

## Guardrails

- **`/spec:init` is the one Specify skill that may install the CLI.** Install only after explicit user confirmation via `cargo install --git https://github.com/augentic/specify-cli`, always verify `specrun --version` before invoking `specrun init`, and never overwrite `project.yaml` without user confirmation.
- **The CLI is the single writer for `.specify/`, `project.yaml`, root `AGENTS.md`, and `.specify/context.lock`.** On non-zero exit, surface the error and stop; never hand-roll the scaffold. Never pre-populate `.specify/.cache/` — `specrun init` owns adapter fetch when invoked with the adapter positional.
- **Adapter vs `--hub` is mutually exclusive.** Pass `$ADAPTER` as the first positional for regular projects; pass `--hub` (and only `--hub`) for hubs. Hub init refuses to run over an existing `.specify/`; converting a regular project to a hub requires the operator to remove `.specify/` first.
