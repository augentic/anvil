# Omnia Schema

- **URL**: `https://github.com/augentic/specify/schemas/omnia`
- **Purpose**: Rust WASM development (greenfield or migration)
- **Source**: JIRA Epic (`/plan:epic-analyzer`), Git Repository (`/rt:code-analyzer`), or Manual
- **Target**: Rust WASM (Omnia SDK)
- **Workflow**: `define` -> `specs` (from Epic, Code, or Manual) -> `design` -> `tasks` -> `build` (crate-writer)

## Contents

| File | Description |
|------|-------------|
| `schema.yaml` | Blueprint declarations, terminology (`unit: crate`), validation flags, and build config |
| `config.yaml` | Starter config installed by `/spec:init` with Omnia-specific context and per-blueprint rules |
| `instructions/proposal.md` | Generation instructions for the proposal blueprint |
| `instructions/specs.md` | Generation instructions for the specs blueprint |
| `instructions/design.md` | Generation instructions for the design blueprint |
| `instructions/tasks.md` | Generation instructions for the tasks blueprint |
| `instructions/build.md` | Implementation instructions for the build phase |

## Blueprints

The schema declares four blueprints in dependency order:

1. **proposal** — initial proposal document (`proposal.md`)
2. **specs** — detailed specifications (`specs/**/*.md`), requires proposal
3. **design** — technical design with implementation details (`design.md`), requires proposal
4. **tasks** — implementation checklist (`tasks.md`), requires specs + design

Build requires tasks to be complete and is tracked via `tasks.md`.

## Schema Framework

For general schema concepts — directory structure, field reference for
`schema.yaml` and `config.yaml`, schema resolution, composition, caching,
and rules override — see the [Schemas README](../README.md).
