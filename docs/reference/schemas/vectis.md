# Vectis Schema

- **URL:** `https://github.com/augentic/specify/schemas/vectis`
- **Purpose:** Cross-platform Crux application development
- **Target:** Rust (Crux shared crate), Swift (iOS shell), Kotlin (Android shell), VectisDesign (design system)

## Brief pipeline

### Define phase

| Brief | Output | Dependencies |
|-------|--------|-------------|
| `proposal.md` | `proposal.md` | -- |
| `specs.md` | `specs/<feature>/spec.md` | proposal |
| `design.md` | `design.md` | proposal |
| `tasks.md` | `tasks.md` | specs, design |

### Build phase

| Brief | Skills invoked |
|-------|---------------|
| `build.md` | `/vectis:design-system-writer`, `/vectis:core-writer`, `/vectis:test-writer`, `/vectis:core-reviewer`, `/vectis:ios-writer`, `/vectis:ios-reviewer`, `/vectis:android-writer`, `/vectis:android-reviewer` |

Build order: design-system first, core second, shells last. Each skill reads the single feature spec and extracts the sections relevant to it.

### Merge phase

| Brief | Skills invoked |
|-------|---------------|
| `merge.md` | -- (drives merge operations directly) |

## Specialist skills

| Skill | Purpose |
|-------|---------|
| `/vectis:core-writer` | Generate or update Rust Crux shared crate |
| `/vectis:test-writer` | Generate tests with spec-to-test traceability |
| `/vectis:core-reviewer` | Agent team review of Crux core |
| `/vectis:ios-writer` | Generate or update SwiftUI iOS shell |
| `/vectis:ios-reviewer` | Agent team review of iOS shell |
| `/vectis:android-writer` | Generate or update Kotlin/Compose Android shell |
| `/vectis:android-reviewer` | Agent team review of Android shell |
| `/vectis:design-system-writer` | Generate VectisDesign from tokens.yaml |
| `/vectis:template-updater` | Fix CLI templates when upstream versions change |

See [Vectis Plugin](../plugins/vectis.md) for full skill documentation.

## Feature-centric specs

The Vectis schema organises specs by **feature** (what the app does), not by software component. A single feature spec at `specs/<feature>/spec.md` contains:

- **Core requirements** (main body) -- platform-neutral behavioral requirements that drive the Crux shared crate.
- **Platform sections** (optional) -- `## iOS Shell Requirements`, `## Android Shell Requirements`, etc.
- **Design system requirements** (optional) -- `## Design System Requirements`.

All requirement IDs share one flat `REQ-###` namespace. Platform sections continue sequential numbering from the last core requirement.

## Platforms

The proposal declares which platforms a change targets:

| Platform | Skill | Description |
|----------|-------|-------------|
| `core` | `vectis:core-writer` | Rust Crux shared crate (always required) |
| `ios` | `vectis:ios-writer` | SwiftUI iOS shell |
| `android` | `vectis:android-writer` | Kotlin/Jetpack Compose Android shell |
| `web` | -- | Web shell (future) |
| `design-system` | `vectis:design-system-writer` | VectisDesign from tokens.yaml |

## Domain context

The Vectis schema injects domain context about:

- Crux application architecture (Model, Event, ViewModel, Effect, `update()`, `view()`).
- Crux capabilities (Render, HTTP, Key-Value, Time, Platform).
- UniFFI FFI scaffolding for cross-platform bridging.
- SwiftUI and Jetpack Compose patterns for shell implementation.
- VectisDesign token system for design consistency.

## Project configuration

After `/spec:init` with the Vectis schema:

```yaml
schema: https://github.com/augentic/specify/schemas/vectis
domain: |
  Describe your app's domain, purpose, and constraints here.
rules:
  - "Project-specific constraints go here"
```
