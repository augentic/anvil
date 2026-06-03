# Vectis Adapter

- **Identifier:** `vectis` (bundled, first-party)
- **URL:** `https://github.com/augentic/specify/adapters/targets/vectis`
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

The specs and design briefs read baseline contracts at `contracts/` as read-only context. Implementation changes conform to existing contracts; new or changed interface shapes should be introduced through a dedicated `contracts@v1` change before implementation depends on them. The contracts target adapter owns author/import/verify behavior through the format sub-flows in [`adapters/targets/contracts/briefs/build.md`](../../../adapters/targets/contracts/briefs/build.md).

The `composition` brief produces a YAML artifact (not markdown) that describes the spatial layout of each screen. It runs between specs and design so that the design brief can adopt screen names, ViewModel variants, and field names proposed by the composition artifact.

### Build phase

The build brief drives implementation work directly through phase sub-briefs — there are no separate slash-command skills. The build orchestrator is [`adapters/targets/vectis/briefs/build.md`](../../../adapters/targets/vectis/briefs/build.md); the per-phase sub-briefs live under [`adapters/targets/vectis/briefs/build/`](../../../adapters/targets/vectis/briefs/build/):

| Sub-brief | Purpose |
|-----------|---------|
| [`build/composition.md`](../../../adapters/targets/vectis/briefs/build/composition.md) | Regenerate `composition.yaml` from `spec.md` + `design.md` and run the deterministic validator gate. |
| [`build/core/write.md`](../../../adapters/targets/vectis/briefs/build/core/write.md) | Generate / update the Crux shared core. |
| [`build/core/review.md`](../../../adapters/targets/vectis/briefs/build/core/review.md) | Agent-team review of the Rust `shared` crate. |
| [`build/test.md`](../../../adapters/targets/vectis/briefs/build/test.md) | Generate / update Crux tests; run the core verify-repair loop. |
| [`build/ios/write.md`](../../../adapters/targets/vectis/briefs/build/ios/write.md) | Generate / update the SwiftUI iOS shell + verify. |
| [`build/ios/review.md`](../../../adapters/targets/vectis/briefs/build/ios/review.md) | Agent-team review of the iOS shell. |
| [`build/android/write.md`](../../../adapters/targets/vectis/briefs/build/android/write.md) | Generate / update the Compose Android shell + verify. |
| [`build/android/review.md`](../../../adapters/targets/vectis/briefs/build/android/review.md) | Agent-team review of the Android shell. |

Build order: core first, shells second. When `composition.yaml` is present, the build phase runs composition validation checks (field coverage, event coverage, ViewModel mapping) before invoking shell writers. Shell writers use `composition.yaml` as the primary layout guide when present, falling back to inference when absent.

### Merge phase

| Brief | Skills invoked |
|-------|---------------|
| `merge.md` | -- (drives `specrun slice merge {preview, conflict-check, run}` plus adapter-owned post-merge validation through `specrun tool run vectis -- validate composition`) |

The Vectis merge brief validates the merged UI baseline with [`vectis validate`](../cli/vectis.md#vectis-validate). Host toolchain and cap-matrix checks are not part of the WASI scaffold/validate tools; the merge brief at [`adapters/targets/vectis/briefs/merge.md`](../../../adapters/targets/vectis/briefs/merge.md) owns those platform workflow steps.

## Reference material

The Crux core, iOS shell, Android shell, design-system, review check libraries, and the legacy layout-inferer contract live under [`adapters/targets/vectis/references/`](../../../adapters/targets/vectis/references/) — see the [`README`](../../../adapters/targets/vectis/references/README.md) for the full index.

## Feature-centric specs

The Vectis adapter organises specs by **feature** (what the app does), not by software component. A single feature spec at `specs/<feature>/spec.md` contains:

- **Core requirements** (main body) -- platform-neutral behavioral requirements that drive the Crux shared crate.
- **Platform sections** (optional) -- `## iOS Shell Requirements`, `## Android Shell Requirements`, etc.
- **Design system requirements** (optional) -- `## Design System Requirements`.

All requirement IDs share one flat `REQ-###` namespace. Platform sections continue sequential numbering from the last core requirement.

## Platforms

Platforms are an app-level fact declared in `project.yaml` via `specrun init vectis --platforms core,ios,android` and carried to every slice. The proposal's `## Platforms` section is stamped verbatim from `project.yaml.platforms` (not per-slice opt-in).

| Platform | Build sub-brief | Description |
|----------|-----------------|-------------|
| `core` | `build/core/write.md` | Rust Crux shared crate (always required) |
| `ios` | `build/ios/write.md` | SwiftUI iOS shell |
| `android` | `build/android/write.md` | Kotlin/Jetpack Compose Android shell |
| `web` | -- | Web shell (future) |

## Domain context

The Vectis adapter's briefs and references carry domain context about:

- Crux application architecture (Model, Event, ViewModel, Effect, `update()`, `view()`).
- Crux adapters (Render, HTTP, Key-Value, Time, Platform).
- UniFFI FFI scaffolding for cross-platform bridging.
- SwiftUI and Jetpack Compose patterns for shell implementation.
- Shell-local theme code emitted from `tokens.yaml` by each shell writer.

## Project configuration

After `specrun init vectis --platforms core,ios,android`, `project.yaml` carries:

```yaml
target: https://github.com/augentic/specify/adapters/targets/vectis
platforms:
  - core
  - ios
  - android
rules:
  - "Project-specific constraints go here"
```

The `platforms` field is required for vectis and must include `core`. To change platforms after init, re-run `specrun init --upgrade --platforms <csv>`.
