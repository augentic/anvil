# Vectis Schema

- **URL**: `https://github.com/augentic/specify/schemas/vectis`
- **Purpose**: Cross-platform Crux application development
- **Source**: Manual
- **Target**: Rust (Crux shared crate), Swift (iOS shell), VectisDesign (design system)
- **Workflow**: `define` -> `specs` -> `design` -> `tasks` -> `build` (core-writer, ios-writer, design-system-writer)

## Contents

| File | Description |
|------|-------------|
| `schema.yaml` | Blueprint declarations, terminology (`deliverable: feature`), validation flags, build config, and defaults (context + per-blueprint rules) |
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

## Feature-Centric Specs

Specs are organized by **feature** (what the app does), not by software
component. A single feature spec at `specs/<feature>/spec.md` contains:

- **Core requirements** (main body) — platform-neutral behavioral
  requirements that drive the Crux shared crate.
- **Platform sections** (optional) — platform-specific behavioral
  requirements in dedicated sections (`## iOS Shell Requirements`,
  `## Android Shell Requirements`, etc.).
- **Design system requirements** (optional) — token change requirements
  in a `## Design System Requirements` section.

This means one spec per feature merges into one baseline — no combining
across component boundaries.

## Platforms

The proposal declares which platforms a change targets. Platforms
determine which build skills are invoked, not how specs are structured.

| Platform | Description | Primary Skill |
|----------|-------------|---------------|
| `core` | Rust Crux shared crate (always required) | `vectis:core-writer` |
| `ios` | SwiftUI iOS shell | `vectis:ios-writer` |
| `android` | Android shell (future) | — |
| `web` | Web shell (future) | — |
| `design-system` | VectisDesign Swift package from tokens.yaml | `vectis:design-system-writer` |

## Schema Framework

For general schema concepts — directory structure, field reference for
`schema.yaml`, schema resolution, composition, caching, and rules
override — see the [Schemas README](../README.md).
