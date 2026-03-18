# Vectis Schema

- **URL**: `https://github.com/augentic/specify/schemas/vectis`
- **Purpose**: Cross-platform Crux application development
- **Source**: Manual
- **Target**: Rust (Crux shared crate), Swift (iOS shell), VectisDesign (design system)
- **Workflow**: `define` -> `specs` -> `design` -> `tasks` -> `build` (core-writer, ios-writer, design-system-writer)

## Contents

| File | Description |
|------|-------------|
| `schema.yaml` | Blueprint declarations, terminology (`deliverable: module`), validation flags, build config, and defaults (context + per-blueprint rules) |
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

## Module Types

The vectis schema supports three module types in a single unified pipeline:

| Type | Description | Primary Skill |
|------|-------------|---------------|
| `core` | Rust Crux shared crate (business logic, state, effects) | `vectis:core-writer` |
| `ios-shell` | SwiftUI iOS shell for an existing Crux core | `vectis:ios-writer` |
| `design-system` | VectisDesign Swift package from tokens.yaml | `vectis:design-system-writer` |

## Schema Framework

For general schema concepts — directory structure, field reference for
`schema.yaml`, schema resolution, composition, caching, and rules
override — see the [Schemas README](../README.md).
