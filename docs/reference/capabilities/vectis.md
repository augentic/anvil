# Vectis Capability

- **Identifier:** `vectis` (bundled, first-party)
- **URL:** `https://github.com/augentic/specify/capabilities/vectis`
- **Purpose:** Cross-platform Crux application development
- **Target:** Rust (Crux shared crate), Swift (iOS shell), Kotlin (Android shell)

## Brief pipeline

### Define phase

| Brief | Output | Dependencies |
|-------|--------|-------------|
| `proposal.md` | `proposal.md` | -- |
| `specs.md` | `specs/<feature>/spec.md` | proposal |
| `composition.md` | `composition.yaml` | specs, proposal |
| `design.md` | `design.md` | proposal, specs |
| `tasks.md` | `tasks.md` | specs, design |

The specs and design briefs read baseline contracts at `contracts/` as read-only context. Implementation changes conform to existing contracts; new or changed interface shapes should be introduced through a dedicated `contracts@v1` change before implementation depends on them. See [Contract Plugin](../plugins/contract.md) for skill details.

The `composition` brief produces a YAML artifact (not markdown) that describes the spatial layout of each screen. It runs between specs and design so that the design brief can adopt screen names, ViewModel variants, and field names proposed by the composition artifact.

### Build phase

| Brief | Skills invoked |
|-------|---------------|
| `build.md` | `/vectis:core-writer`, `/vectis:test-writer`, `/vectis:core-reviewer`, `/vectis:ios-writer`, `/vectis:ios-reviewer`, `/vectis:android-writer`, `/vectis:android-reviewer` |

Build order: core first, shells second. Each skill reads the single feature spec and extracts the sections relevant to it. When `composition.yaml` is present, the build phase runs composition validation checks (field coverage, event coverage, ViewModel mapping) before invoking shell writers. Shell writers use `composition.yaml` as the primary layout guide when present, falling back to inference when absent.

### Merge phase

| Brief | Skills invoked |
|-------|---------------|
| `merge.md` | -- (drives `specify slice merge {preview, conflict-check, run}` plus capability-owned post-merge validation through `specify tool run vectis -- validate composition`) |

The Vectis merge brief validates the merged UI baseline with [`vectis validate`](../cli/vectis.md#vectis-validate). Host toolchain and cap-matrix checks are not part of the WASI scaffold/validate tools; writer, reviewer, and template-updater skills own those platform workflow steps.

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
| `/vectis:template-updater` | Fix CLI templates when upstream versions change |

See [Vectis Plugin](../plugins/vectis.md) for full skill documentation.

## Feature-centric specs

The Vectis capability organises specs by **feature** (what the app does), not by software component. A single feature spec at `specs/<feature>/spec.md` contains:

- **Core requirements** (main body) -- platform-neutral behavioral requirements that drive the Crux shared crate.
- **Platform sections** (optional) -- `## iOS Shell Requirements`, `## Android Shell Requirements`, etc.
- **Design system requirements** (optional) -- `## Design System Requirements`.

All requirement IDs share one flat `REQ-###` namespace. Platform sections continue sequential numbering from the last core requirement.

## Platforms

The proposal declares which platforms a slice targets:

| Platform | Skill | Description |
|----------|-------|-------------|
| `core` | `vectis:core-writer` | Rust Crux shared crate (always required) |
| `ios` | `vectis:ios-writer` | SwiftUI iOS shell |
| `android` | `vectis:android-writer` | Kotlin/Jetpack Compose Android shell |
| `web` | -- | Web shell (future) |

## Domain context

The Vectis capability's briefs and skills carry domain context about:

- Crux application architecture (Model, Event, ViewModel, Effect, `update()`, `view()`).
- Crux capabilities (Render, HTTP, Key-Value, Time, Platform).
- UniFFI FFI scaffolding for cross-platform bridging.
- SwiftUI and Jetpack Compose patterns for shell implementation.
- Shell-local theme code emitted from `tokens.yaml` by each shell writer.

## Project configuration

After `/spec:init vectis`, `project.yaml` carries:

```yaml
capability: https://github.com/augentic/specify/capabilities/vectis
rules:
  - "Project-specific constraints go here"
```
