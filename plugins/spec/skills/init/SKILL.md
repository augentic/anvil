---
name: specify-init
description: Initialize Specify in a project. Bootstraps the `specify` CLI when missing, picks between a regular single-project init and a registry-only platform hub, then invokes `specify init <capability>` or `specify init --hub` to scaffold `.specify/`, write `project.yaml`, and generate starter `AGENTS.md`. Use when first wiring up a project before any other `/spec:*` or `/change:*` command; not for re-initializing an existing `.specify/`.
argument-hint: <capability>
---

# Specify Init

> **The one Specify skill that may install the CLI.** `/spec:init` bootstraps `specify` when missing, decides regular vs hub topology, then delegates every filesystem write to `specify init`. The CLI is the single writer for `.specify/`, `project.yaml`, root `AGENTS.md`, and `.specify/context.lock`.

## Critical Path

1. **Verify the CLI** — run `specify --version`; install with `cargo install --git https://github.com/augentic/specify-cli` only after explicit user confirmation.
2. **Check existing initialization** — detect `.specify/project.yaml`, ask before reinitializing, and treat reinit as an upgrade path owned by the CLI.
3. **Choose topology** — decide regular project vs registry-only platform hub; capability is required for regular projects and forbidden in hub mode.
4. **Resolve metadata** — choose `$CAPABILITY`, `$PROJECT_NAME`, and optional `$DOMAIN`; never pre-populate `.specify/.cache/`.
5. **Invoke `specify init`** — run either `specify init "$CAPABILITY" ...` or `specify init --hub ...`; let the CLI scaffold files and generate starter context, and surface non-zero CLI errors without hand-rolling scaffold files.
6. **Offer baseline extraction** — for regular projects with code indicators, ask whether to create `initial-baseline`; skip this entirely for hubs.
7. **Summarize the correct shape** — report regular vs hub outputs, next actions, and any baseline-extraction handoff.

## Orientation

`/spec:init` selects between two on-disk shapes per run. A **regular project** carries code and `.specify/` together; the CLI scaffolds `slices/`, `specs/`, `archive/`, `.cache/`, and a `project.yaml` whose `capability:` field drives every downstream pipeline. A **platform hub** carries only platform state (`registry.yaml`, later `change.md` / `plan.yaml` / `workspace/`); `project.yaml` records `hub: true` with no `capability:`, and phase pipelines are disabled on the hub itself.

Capability vs `--hub` is mutually exclusive: `specify init` with neither, or both, exits with `init-requires-capability-or-hub`. A regular project must declare a capability; a hub must declare `--hub` and never carries a `capability:`.

The CLI owns every filesystem write — `.specify/`, `project.yaml`, the resolved capability cache, root `AGENTS.md`, and `.specify/context.lock`. When `AGENTS.md` already exists, the CLI preserves it byte-for-byte. The skill never hand-rolls scaffold files; on non-zero exit it surfaces the CLI error and stops.

After a regular init, the skill optionally detects existing code indicators (`Cargo.toml`, `package.json`, `src/`, etc.) and offers to create an `initial-baseline` slice via `specify slice create`. Hub init skips that step entirely — a hub never carries code. The three render templates (greenfield / brownfield / hub) live in [`../../references/init-output-templates.md`](../../references/init-output-templates.md).

See [`references/init-runbook.md`](references/init-runbook.md) for the operational detail (CLI bootstrap rules, full seven-step procedure with verbatim shell snippets, regular and hub invocation bodies, output templates, and the skill-scope boundaries).

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/init-runbook.md`](references/init-runbook.md) | CLI bootstrap, full seven-step procedure, regular vs hub invocations, output templates, skill-scope boundaries |
| [`../../references/init-output-templates.md`](../../references/init-output-templates.md) | Verbatim greenfield / brownfield / hub output templates rendered after `specify init` returns |
| [`../../references/topology-flow.md`](../../references/topology-flow.md) | Regular project vs platform hub decision tree and on-disk shape |
| [`../../references/capability-resolution.md`](../../references/capability-resolution.md) | Capability identifier resolution (bare name / URL / file URI) and `.specify/.cache/` ownership |
| [`../../references/baseline-detection.md`](../../references/baseline-detection.md) | Manifest / source-dir indicators used to offer `initial-baseline` extraction |
| [`../../references/specify.md`](../../references/specify.md) | High-level Specify mental model and how init seats inside it |

## Guardrails

- **`/spec:init` is the one Specify skill that may install the CLI.** Install only after explicit user confirmation via `cargo install --git https://github.com/augentic/specify-cli`, always verify `specify --version` before invoking `specify init`, and never overwrite `project.yaml` without user confirmation.
- **The CLI is the single writer for `.specify/`, `project.yaml`, root `AGENTS.md`, and `.specify/context.lock`.** On non-zero exit, surface the error and stop; never hand-roll the scaffold. Never pre-populate `.specify/.cache/` — `specify init` owns capability fetch when invoked with the capability positional.
- **Capability vs `--hub` is mutually exclusive.** Pass `$CAPABILITY` as the first positional for regular projects; pass `--hub` (and only `--hub`) for hubs. Hub init refuses to run over an existing `.specify/`; converting a regular project to a hub requires the operator to remove `.specify/` first.
