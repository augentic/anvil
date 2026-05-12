---
name: vectis-android-writer
description: Generate or update a Kotlin/Jetpack Compose Android shell for a Crux application. Use when a Specify slice has pending Android shell tasks, or when an existing Android shell needs to be regenerated after a core or layout update; not for the Rust core (`core-writer`) or the iOS shell (`ios-writer`).
argument-hint: <app-dir> [project-dir] [slice-dir]
---

# Crux Android Shell Generator

> **Vectis deterministic tooling runs through declared Specify tools.** Shell scaffolding is `specify tool run vectis -- scaffold android ...`; artifact validation is `specify tool run vectis -- validate ...`. The scaffold is render-only: Android SDK/NDK detection, `local.properties`, Java 21 pinning, Gradle wrapper bootstrap, and Gradle builds remain host-owned steps with explicit verification evidence.

## Critical Path

1. Read `{app-dir}/shared/src/app.rs` (and optional `slice-dir` shell-requirements + `composition.yaml`); extract App name, ViewModel/Effect/Event/Route variants and the capability set.
2. Detect mode by checking `{project-dir}/app/src/main/java/*/Core.kt`: missing → run `specify tool run vectis -- scaffold android ...` plus Android host post-processing, then enter Update Mode; present → start Update Mode immediately.
3. Build an implementation inventory of existing Kotlin code (effect handlers, ViewModel cases, screen composables, event dispatches, capability clients, DI modules).
4. Diff Rust core types vs Kotlin inventory by category (Effect → ViewModel → view-fields → Event → Route) and emit a summary edit plan.
5. Apply changes: expand or strip CAP blocks in `Core.kt` + `AndroidManifest.xml` + Gradle, add/remove screen composables for each ViewModel variant, update the root `when`, dispatch new Events.
6. Update build configuration (`libs.versions.toml`, `build.gradle.kts`, manifest permissions, `network_security_config.xml`) to match the changed capability set.
7. Run Android host checks (`local.properties` / Java 21 / NDK / wrapper), `make build`, `./gradlew :shared:cargoBuild`, and `./gradlew :app:assembleDebug` (skipped when the orchestrator passes `skip_verification: true`).

## Orientation

The Android writer generates or updates a buildable Kotlin/Jetpack Compose shell for an existing Crux core. The shell renders the core's `ViewModel`, dispatches `Event` values from user interactions, and handles platform side-effects (HTTP, KV, SSE, Time, Platform). It targets **Kotlin 2.x**, **Jetpack Compose** with Material 3, and minimum SDK 34.

The writer reads `tokens.yaml`, `assets.yaml`, and `composition.yaml` directly and emits **shell-local** theme + asset resources inside the Android shell tree. There is no separate `:vectis-design` Gradle module and no path back into `design-system/android/` from the rendered shell project. When `tokens.yaml` is absent, the writer falls back to platform-native Material 3 defaults — fallback policy belongs to the shell writer, not to the design-system manifest.

When an existing Android shell is detected, the skill operates in **update mode**: it diffs the current `app.rs` against the existing Kotlin code and applies targeted edits. When no shell exists yet, it runs `specify tool run vectis -- scaffold android {AppName} ...` to render the project, then switches to update mode.

The orchestrator may pass `skip_verification: true` to skip the build/verify loop in favour of its own dedicated verify sub-agent.

See [`references/runbook.md`](references/runbook.md) for prerequisites, full input analysis, mode-detection rules, the Create / Update step bodies (1–3 / U1–U8), composition mapping priority, the spec-to-code table, error-handling tables, and the verification checklist.

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Prerequisites, input analysis, mode detection, Create/Update step bodies, composition mapping, spec-to-code, error tables, verification checklist |
| [`references/crux-android-shell-pattern.md`](references/crux-android-shell-pattern.md) | Core.kt template, effect handling, serialization protocol, crash-recovery handler |
| [`references/compose-view-patterns.md`](references/compose-view-patterns.md) | Screen patterns, lists, forms, navigation, accessibility, layout constraint rules |
| [`references/design-system-integration.md`](references/design-system-integration.md) | Shell-local theme + asset integration: generated layout, M3 fallback, copy-on-generate, component directive contract |
| [`references/kotlin-token-templates.md`](references/kotlin-token-templates.md) | Kotlin code templates per token category (color, typography, scalar, border, theme composable) |
| [`references/generated-type-conventions.md`](references/generated-type-conventions.md) | Generated bincode + UniFFI imports, enum vs sealed interface naming, numeric type mapping, KeyValue / Time, `@OptIn(ExperimentalUnsignedTypes::class)` |
| [`rules.md`](rules.md) | Platform-level normative facts (UniFFI bridging, library override, generated-type packages, Gradle wrapper bootstrap, Java 21 pin, network security config, defensive `CoreFFI` error handling, mandatory `themes.xml`, `{slice-dir}` integration) and the full preservation contract |

## Guardrails

- **ALWAYS read [`rules.md`](rules.md) first.** The platform-level normative facts live there; edit no scaffold output before consulting it.
