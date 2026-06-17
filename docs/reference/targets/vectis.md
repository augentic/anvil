# Vectis Adapter

- **Identifier:** `vectis` (bundled, first-party)
- **URL:** `https://github.com/augentic/specify/adapters/targets/vectis`
- **Purpose:** Cross-platform Crux application development
- **Target:** Rust (Crux shared crate), Swift (iOS shell), Kotlin (Android shell)

## Operations

The Vectis target declares exactly three operations — `shape`, `build`, `merge` — matching its [`adapter.yaml`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/adapter.yaml). Core `/spec:refine` synthesises the canonical artifacts (`proposal.md` / `spec.md` / `design.md` / `tasks.md`); the target adapter never writes them. The Vectis target adds no fourth slot — composition regeneration is part of `build`, not a synthesis step.

### shape

`shape` is idiom guidance read into context when core synthesis writes `spec.md` and `design.md` for a `target: vectis` slice — see [`adapters/targets/vectis/briefs/shape.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/briefs/shape.md). The brief is input to synthesis: it does not read sources, write artifacts, or transition lifecycle. It tells the synthesiser how Crux idioms organise canonical artifact content — the flat `REQ-###` namespace, the platform-neutral core body plus optional `## iOS Shell Requirements` / `## Android Shell Requirements` sections in `spec.md`, and the `design.md` domain model (`Model` / `Event` / `ViewModel` / `Route` / per-page view structs, capability matrix) that `build` later turns into code and `composition.yaml`.

`composition.yaml` is **not** a Specify artifact and is **never** synthesised — `shape` only ensures `spec.md` + `design.md` describe screen structure precisely enough for `build` to regenerate it deterministically. Operator-curated `tokens.yaml` / `assets.yaml` / `components.yaml` are build-time inputs, never synthesis inputs; `shape` forbids restating their contents in `spec.md` / `design.md`.

The synthesis briefs treat baseline contracts at `contracts/` as read-only context. Implementation changes conform to existing contracts; new or changed interface shapes should be introduced through a dedicated `contracts@1.0.0` change before implementation depends on them. The contracts target adapter owns author/import/verify behavior through the format sub-flows in [`adapters/targets/contracts/briefs/build.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/contracts/briefs/build.md).

### build

The build brief drives implementation work directly through phase sub-briefs — there are no separate slash-command skills. The build orchestrator is [`adapters/targets/vectis/briefs/build.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/briefs/build.md); the per-phase sub-briefs live under [`adapters/targets/vectis/briefs/build/`](https://github.com/augentic/specify-adapters/tree/main/adapters/targets/vectis/briefs/build/):

| Sub-brief | Purpose |
|-----------|---------|
| [`build/composition.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/briefs/build/composition.md) | Regenerate `composition.yaml` from `spec.md` + `design.md` and run the deterministic validator gate. |
| [`build/core/write.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/briefs/build/core/write.md) | Generate / update the Crux shared core. |
| [`build/core/review.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/briefs/build/core/review.md) | Agent-team review of the Rust `shared` crate. |
| [`build/test.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/briefs/build/test.md) | Generate / update Crux tests; run the core verify-repair loop. |
| [`build/ios/write.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/briefs/build/ios/write.md) | Generate / update the SwiftUI iOS shell + verify. |
| [`build/ios/review.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/briefs/build/ios/review.md) | Agent-team review of the iOS shell. |
| [`build/android/write.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/briefs/build/android/write.md) | Generate / update the Compose Android shell + verify. |
| [`build/android/review.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/briefs/build/android/review.md) | Agent-team review of the Android shell. |

`build` regenerates `composition.yaml` from `spec.md` + `design.md` first, runs the deterministic composition validator gate (field coverage, event coverage, ViewModel mapping), then writes code in dependency order: core first, shells second. Shell writers use the regenerated `composition.yaml` as the primary layout guide. `build` writes the per-slice `composition.yaml` and the implementation code, then records the result in `build/report.yaml`; the CLI's `specify slice build --phase finalize` owns the `built` transition.

### merge

The merge brief lands the built slice through `specify slice merge` (`preview`, `conflict-check`, `run`) per the shared [`/spec:merge`](../../../plugins/spec/skills/merge/SKILL.md) skill body, then runs the Vectis-specific adoption gates — see [`adapters/targets/vectis/briefs/merge.md`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/briefs/merge.md). Two gates are adapter-owned:

- **Composition validation** — `specify extension run vectis -- validate composition` runs against the staged slice before merge and again against the merged baseline after, blocking on errors. This is the WASI [`vectis validate`](../cli/vectis.md#vectis-validate) surface.
- **Host cap-matrix re-verification** — after `specify slice merge` lands the deltas, the brief re-runs `cargo` / `make build` / `gradlew` against the merged tree to catch cross-slice regressions. Host toolchain and cap-matrix checks are not part of the WASI tools; the merge brief owns those platform workflow steps.

`specify slice merge` is the writer of the `merged` lifecycle transition and the archive move; the brief adds no merge envelope of its own.

## Reference material

The Crux core, iOS shell, Android shell, design-system, review check libraries, and the legacy layout-inferer contract live under [`adapters/targets/vectis/references/`](https://github.com/augentic/specify-adapters/tree/main/adapters/targets/vectis/references/) — see the [`README`](https://github.com/augentic/specify-adapters/blob/main/adapters/targets/vectis/references/README.md) for the full index.

## Feature-centric specs

The Vectis adapter organises specs by **feature** (what the app does), not by software component. A single feature spec at `specs/<feature>/spec.md` contains:

- **Core requirements** (main body) -- platform-neutral behavioral requirements that drive the Crux shared crate.
- **Platform sections** (optional) -- `## iOS Shell Requirements`, `## Android Shell Requirements`, etc.
- **Design system requirements** (optional) -- `## Design System Requirements`.

All requirement IDs share one flat `REQ-###` namespace. Platform sections continue sequential numbering from the last core requirement.

## Platforms

Platforms are an app-level fact declared in `project.yaml` via `specify init vectis --platforms core,ios,android` and carried to every slice. The proposal's `## Platforms` section is stamped verbatim from `project.yaml.platforms` (not per-slice opt-in).

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

After `specify init vectis --platforms core,ios,android`, `project.yaml` carries:

```yaml
target: https://github.com/augentic/specify/adapters/targets/vectis
platforms:
  - core
  - ios
  - android
rules:
  - "Project-specific constraints go here"
```

The `platforms` field is required for vectis and must include `core`. To change platforms after init, re-run `specify init --upgrade --platforms <csv>`.
