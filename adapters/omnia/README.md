# Omnia Schema

- **URL**: `https://github.com/augentic/specify/adapters/omnia`
- **Purpose**: Rust WASM development (greenfield or migration)
- **Source**: Git Repository, Source Code, or Manual (all analyzed via `/spec:extract`)
- **Target**: Rust WASM (Omnia SDK)
- **Workflow**: `define` -> `specs` (from Code or Manual) -> `design` -> `tasks` -> `build` (crate-writer)

## Contents

| File | Description |
|------|-------------|
| `adapter.yaml` | Pipeline stages and per-stage brief references |
| `briefs/proposal.md` | Generation brief for the proposal stage |
| `briefs/specs.md` | Generation brief for the specs stage |
| `briefs/design.md` | Generation brief for the design stage |
| `briefs/tasks.md` | Generation brief for the tasks stage |
| `briefs/build.md` | Implementation brief for the build stage |
| `briefs/merge.md` | Merge brief for finalizing a change |
| `codex/` | Omnia-specific review rules for provider usage, WASM constraints, Rust error handling, and host-managed secrets |

## Blueprints

The schema declares four blueprints in dependency order:

1. **proposal** — initial proposal document (`proposal.md`)
2. **specs** — detailed specifications (`specs/**/*.md`), requires proposal
3. **design** — technical design with implementation details (`design.md`), requires proposal
4. **tasks** — implementation checklist (`tasks.md`), requires specs + design

Build requires tasks to be complete and is tracked via `tasks.md`.

## Codex

Omnia review rules live under [`codex/`](codex/). This first cut is intentionally small and covers the highest-value checks that are specific to Rust WASM guest components using the Omnia SDK.

## Adapter Framework

For general adapter concepts — directory structure, field reference for `adapter.yaml`, adapter resolution, composition, caching, and rules override — see the [Adapters README](../README.md).
