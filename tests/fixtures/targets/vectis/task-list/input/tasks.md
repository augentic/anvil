# task-list — Tasks

Tasks are organised by build phase, not by feature: core first, shells second. Every task is agent-completable through writer or reviewer work and local build / test commands; no manual mobile testing.

## Core

- [ ] Scaffold the Crux shared core (`specify tool run vectis -- scaffold core TodoApp --caps http,kv`); commit the generated workspace, `shared` crate, `clippy.toml`, and `rust-toolchain.toml`.
- [ ] Implement the Domain Model from `design.md` in `shared/src/app.rs`: `TodoApp`, `Model`, `Page`, `Route`, `Event`, `ViewModel`, `TaskListView`, `TaskRowView`, `AddTaskView`, `ErrorView`, `Effect`, `Task`, `TaskId`, `AddTaskForm`, `DomainError`.
- [ ] Implement `update()` arms for every `Event` variant covering REQ-002, REQ-003, REQ-004, REQ-005, REQ-006; route HTTP and KV side effects through `Command` chains and wire the internal `TasksLoaded` and `PersistComplete` callbacks.
- [ ] Implement `view()` to project `Model` into `ViewModel` with strikethrough rendering for completed tasks (REQ-002), the empty-state copy from REQ-001, and the title-validation error from REQ-003.
- [ ] Generate spec-traced tests in `#[cfg(test)] mod tests` of `app.rs`: one synchronous `#[test]` per scenario across REQ-001..REQ-006 with `/// Spec: task-list > REQ-XXX > Scenario: ...` traceability comments and effect-chain assertions (no `#[tokio::test]`).
- [ ] Run the core verify-repair loop: `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets`, `cargo test`. Resolve every iteration to green before proceeding.
- [ ] Run `vectis:core-reviewer`-equivalent inline checks (CRX / LOG / GEN / UNI) per the build brief's reviewers section; revert any auto-fix batch that regresses `cargo check` / `cargo clippy` / `cargo test`.

## iOS shell

- [ ] Scaffold the iOS shell (`specify tool run vectis -- scaffold ios TodoApp --caps http,kv`); commit `iOS/project.yml`, `Makefile`, Inject SPM wiring, `Core.swift`, `ContentView.swift`, and starter `Views/`.
- [ ] Implement the per-screen SwiftUI views for the `TaskList`, `AddTask`, and `Settings` ViewModel variants; render every `bind` from the regenerated `composition.yaml` and dispatch every `event` through `Core.update(...)`.
- [ ] Implement swipe-to-delete (REQ-008) on each task row, routing through `RequestDelete(id)` and the existing confirmation dialog.
- [ ] Regenerate shell-local `iOS/TodoApp/Theme/` from `tokens.yaml` (HIG fallback when `tokens.yaml` is absent) and `iOS/TodoApp/Resources/Assets.xcassets/` from `assets.yaml`.
- [ ] Run the iOS verify loop in its own sub-agent: `swiftformat iOS/TodoApp/`, `cd iOS && make build`, `cd iOS && make sim-build`. Cap iterations at 3.
- [ ] Run `vectis:ios-reviewer`-equivalent inline checks (IOS / SWF / UNI); apply mechanical auto-fixes; revert the batch if `swiftformat` or `make build` regresses.

## Android shell

- [ ] Scaffold the Android shell (`specify tool run vectis -- scaffold android TodoApp --caps http,kv --android-package com.vectis.todoapp`); commit Gradle build files, `local.properties`, `gradle.properties` (pinned to Java 21), and `Core.kt`.
- [ ] Implement the per-screen Compose composables for each ViewModel variant under `Android/app/src/main/java/com/vectis/todoapp/ui/screens/`; render every `bind` from the regenerated `composition.yaml` and dispatch every `event` through `Core.update(...)`.
- [ ] Wire the `Application` class to call `System.setProperty("uniffi.component.shared.libraryOverride", "shared")` before any UniFFI class loads (REQ-009 + UniFFI bridging).
- [ ] Implement edge-to-edge rendering (REQ-010) with `WindowCompat.setDecorFitsSystemWindows(window, false)` and `Modifier.systemBarsPadding()` on the FAB host.
- [ ] Regenerate shell-local `Android/app/src/main/java/com/vectis/todoapp/ui/theme/` from `tokens.yaml` (Material 3 fallback when `tokens.yaml` is absent) and drawable resources under `Android/app/src/main/res/drawable*/` from `assets.yaml`.
- [ ] Run the Android verify loop in its own sub-agent: pre-flight (`local.properties` / Java 21 / Rust Android targets), Gradle wrapper bootstrap if missing, then `make build`, `./gradlew :shared:cargoBuild`, `./gradlew :app:assembleDebug`. Cap iterations at 3.
- [ ] Run `vectis:android-reviewer`-equivalent inline checks (AND / KTL / UNI); apply mechanical auto-fixes; revert the batch if the Gradle build regresses.
