---
name: vectis-ios-writer
description: Generate or update a SwiftUI iOS shell for a Crux application from Specify artifacts. Use when a Specify slice has pending iOS shell tasks routed through a Vectis capability; not for the Rust core (`core-writer`) or the Android shell (`android-writer`).
argument-hint: <app-dir> [project-dir] [slice-dir]
---

# Crux iOS Shell Generator

> **Vectis deterministic tooling runs through declared Specify tools.** Shell scaffolding is `specify tool run vectis -- scaffold ios ...`; artifact validation is `specify tool run vectis -- validate ...`. The scaffold is render-only: iOS type generation, packaging, Xcode generation, and build checks remain host-owned steps with explicit verification evidence.

## Critical Path

1. **Read the input contract** — inspect `app.rs`, `lib.rs`, `Cargo.toml`, `tokens.yaml`, `assets.yaml`, `composition.yaml`, and any iOS-specific spec/design sections.
2. **Detect mode** — if no shell exists, run `specify tool run vectis -- scaffold ios ...` plus host checks; otherwise inventory existing Swift code for update mode.
3. **Diff core and UI artifacts** — classify effects, ViewModel variants, page fields, events, routes, token categories, assets, components, and legacy `VectisDesign` references.
4. **Apply core/view updates** — edit `Core.swift`, `ContentView.swift`, screen views, navigation, Inject boilerplate, and build config with targeted changes only.
5. **Refresh generated UI surfaces** — regenerate shell-local `Theme/`, `Components/`, and `Resources/Assets.xcassets/` from validated artifacts while preserving operator-owned files.
6. **Verify or delegate verification** — run `swiftformat`, `make typegen`, `make package`, and `make xcode` unless the orchestrator passed `skip_verification: true`.
7. **Enforce shell boundaries** — keep business logic in the Rust core, remove legacy design-system imports, and avoid known SwiftUI interaction hazards.

## Orientation

The iOS writer generates or updates a buildable SwiftUI iOS shell (Swift 6, iOS 17+) for an existing Crux core. The shell renders the core's `ViewModel`, dispatches `Event` values from user interactions, and handles platform side-effects (HTTP, KV, SSE) on behalf of the core.

The writer reads `tokens.yaml`, `assets.yaml`, and `composition.yaml` directly and emits **shell-local** theme + asset catalog code inside the iOS shell tree. There is no shared Swift Package, no `import VectisDesign`, and no path back into `design-system/ios/` from the rendered shell project. When `tokens.yaml` is absent, the writer falls back to platform-native HIG defaults — fallback policy belongs to the shell writer, not to the design-system manifest.

When an existing iOS shell is detected, the skill operates in **update mode** and applies targeted edits rather than regenerating from scratch. When no shell exists yet, it runs `specify tool run vectis -- scaffold ios {AppName} [--caps {caps}]` from the Crux project root and then switches to update mode. The scaffold tool owns `iOS/project.yml`, `iOS/Makefile`, Inject SPM wiring, the `{AppName}App.swift` entry point, a render-only `Core.swift` with CAP markers, a baseline `ContentView.swift`, and the starter `Views/LoadingScreen.swift` / `Views/HomeScreen.swift`. The writer adds `Theme/`, `Components/`, and `Resources/Assets.xcassets/` on first generation when the corresponding inputs exist.

The orchestrator may pass `skip_verification: true` to skip the build/verify loop in favour of its dedicated iOS verify sub-agent.

See [`references/runbook.md`](references/runbook.md) for prerequisites, full input analysis, mode-detection rules, the Create / Update step bodies (1–3 / U1–U8), composition mapping, the spec-to-code table, preservation rules, error-handling tables, and the verification checklist.

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Prerequisites, input analysis, mode detection, Create/Update step bodies, composition mapping, spec-to-code, error tables, verification checklist |
| [`references/crux-ios-shell-pattern.md`](references/crux-ios-shell-pattern.md) | Core.swift template, effect handling, serialization protocol |
| [`references/swiftui-view-patterns.md`](references/swiftui-view-patterns.md) | Screen patterns, lists, forms, navigation, accessibility |
| [`references/design-system-integration.md`](references/design-system-integration.md) | Shell-local theme + asset integration: generated layout under `iOS/<App>/Theme/`, HIG fallback when `tokens.yaml` is absent, asset copy-on-generate rules, component-directive contract |
| [`references/swift-token-templates.md`](references/swift-token-templates.md) | Concrete Swift code templates per token category (color, typography, scalar, border, theme bundle); shell-local emission |

## Guardrails

- **NEVER add business logic to Swift code** — run `core-writer` first; the shell only renders views and performs platform I/O.
- **NEVER place `TextField` or small `Button` inside a `ScrollView` within a `NavigationStack`** — the `UIScrollView` touch-delay mechanism suppresses taps. See [`references/swiftui-view-patterns.md`](references/swiftui-view-patterns.md).
- **ALWAYS treat `app.rs` as the primary input** and leave Inject hot-reload boilerplate in place. See [`references/runbook.md`](references/runbook.md) "Platform Rules (Full)" for the complete NEVER/ALWAYS list (UniFFI pinning, dual Swift packages, slice-dir supplements).
