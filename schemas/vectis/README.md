# Vectis Schema

- **URL**: `https://github.com/augentic/specify/schemas/vectis`
- **Purpose**: Cross-platform Crux application development
- **Source**: Manual
- **Target**: Rust (Crux shared crate), Swift (iOS shell), Kotlin (Android shell), VectisDesign (design system)
- **Workflow**: `define` -> `specs` -> `design` -> `tasks` -> `build` (core-writer, ios-writer, android-writer, design-system-writer)

## Contents

| File | Description |
|------|-------------|
| `schema.yaml` | Pipeline stages, domain context, and per-stage brief references |
| `briefs/proposal.md` | Generation brief for the proposal stage |
| `briefs/specs.md` | Generation brief for the specs stage |
| `briefs/design.md` | Generation brief for the design stage |
| `briefs/tasks.md` | Generation brief for the tasks stage |
| `briefs/build.md` | Implementation brief for the build stage |
| `briefs/merge.md` | Merge brief for finalizing a change |

## Blueprints

The schema declares four blueprints in dependency order:

1. **proposal** — initial proposal document (`proposal.md`)
2. **specs** — detailed specifications (`specs/**/*.md`), requires proposal
3. **design** — technical design with implementation details (`design.md`), requires proposal
4. **tasks** — implementation checklist (`tasks.md`), requires specs + design

Build requires tasks to be complete and is tracked via `tasks.md`.

## Feature-Centric Specs

Specs are organized by **feature** (what the app does), not by software component. A single feature spec at `specs/<feature>/spec.md` contains:

- **Core requirements** (main body) — platform-neutral behavioral requirements that drive the Crux shared crate.
- **Platform sections** (optional) — platform-specific behavioral requirements in dedicated sections (`## iOS Shell Requirements`, `## Android Shell Requirements`, etc.).
- **Design system requirements** (optional) — token change requirements in a `## Design System Requirements` section.

This means one spec per feature merges into one baseline — no combining across component boundaries.

## Platforms

The proposal declares which platforms a change targets. Platforms determine which build skills are invoked, not how specs are structured.

| Platform | Description | Primary Skill |
|----------|-------------|---------------|
| `core` | Rust Crux shared crate (always required) | `vectis:core-writer` |
| `ios` | SwiftUI iOS shell | `vectis:ios-writer` |
| `android` | Kotlin/Jetpack Compose Android shell | `vectis:android-writer` |
| `web` | Web shell (future) | — |
| `design-system` | VectisDesign from tokens.yaml (iOS SPM + Android library) | `vectis:design-system-writer` |

## Schema Framework

For general schema concepts — directory structure, field reference for `schema.yaml`, schema resolution, composition, caching, and rules override — see the [Schemas README](../README.md).
