# Vectis Capability

- **URL**: `https://github.com/augentic/specify/capabilities/vectis`
- **Purpose**: Cross-platform Crux application development
- **Source**: Manual
- **Target**: Rust (Crux shared crate), Swift (iOS shell), Kotlin (Android shell)
- **Workflow**: `proposal` -> `specs` -> `composition` -> `design` -> `tasks` -> `build` (core-writer, ios-writer, android-writer)

## Contents

| File | Description |
|------|-------------|
| `capability.yaml` | Pipeline stages and per-stage brief references |
| `composition.schema.json` | JSON Schema for `composition.yaml` validation |
| `briefs/proposal.md` | Generation brief for the proposal stage |
| `briefs/specs.md` | Generation brief for the specs stage |
| `briefs/composition.md` | Generation brief for the composition stage (screen layout) |
| `briefs/design.md` | Generation brief for the design stage |
| `briefs/tasks.md` | Generation brief for the tasks stage |
| `briefs/build.md` | Implementation brief for the build stage |
| `briefs/merge.md` | Merge brief for finalizing a change |
| `codex/*.md` | Vectis-specific review rules for Crux core/shell boundaries, state transitions, and platform shell responsibilities |

## Blueprints

The capability declares five blueprints in dependency order:

1. **proposal** — initial proposal document (`proposal.md`)
2. **specs** — detailed specifications (`specs/**/*.md`), requires proposal
3. **composition** — screen layout artifact (`composition.yaml`), requires specs + proposal
4. **design** — technical design with implementation details (`design.md`), requires proposal + specs
5. **tasks** — implementation checklist (`tasks.md`), requires specs + design

Build requires tasks to be complete and is tracked via `tasks.md`.

## Codex

Vectis review rules live under [`codex/`](codex/). This first cut is intentionally small and covers the highest-value checks that are specific to Crux core behavior and generated iOS/Android shells.

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

## Capability Framework

For general capability concepts — directory structure, field reference for `capability.yaml`, capability resolution, composition, caching, and rules override — see the [Capabilities README](../README.md).
