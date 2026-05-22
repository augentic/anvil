# Vectis Plugin

Generate cross-platform [Crux](https://github.com/redbadger/crux) applications: Rust shared core, SwiftUI iOS shell, and Kotlin/Jetpack Compose Android shell.

> **Tool entry point.** Vectis deterministic helpers are declared WASI tools run through [`specify tool`](../cli/tool.md). Use `specify tool run vectis -- validate <mode> [path]` for UI input validation and `specify tool run vectis -- scaffold core|ios|android ...` for render-only scaffolding. Cargo, Xcode, Gradle, SDK setup, registry behavior, and end-to-end verification remain skill-owned host workflow.

## Why Crux

- Support multiple runtime platforms -- iOS, Android, Web, macOS, Linux, Windows -- from a single shared core.
- All application behavior lives in the shared core, testable independently of the runtime platform.
- An opinionated application structure well-suited to AI-assisted code generation.

Crux is written in Rust and documented at [docs.rs/crux_core](https://docs.rs/crux_core/latest/crux_core/).

## Prerequisites

### Rust toolchain

- [Install Rust](https://rust-lang.org/tools/install/)
- Install the [Rust Analyzer](https://open-vsx.org/extension/rust-lang/rust-analyzer) Cursor extension

### iOS development

Required only for iOS shells:

```shell
brew install xcode-build-server xcbeautify swiftformat xcodegen
cargo install cargo-swift
```

Install the [Swift Language Support](https://open-vsx.org/extension/chrisatwindsurf/swift-vscode) and [SweetPad](https://marketplace.visualstudio.com/items?itemName=SweetPad.sweetpad) Cursor extensions.

iOS simulator targets:

```shell
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
```

### Android development

Required only for Android shells:

- Android SDK (via Android Studio or command-line tools)
- Android NDK: `sdkmanager "ndk;29.0.14206865"`
- Java 21 LTS JDK (not Java 25+ -- Gradle compatibility)
- Gradle: `brew install gradle`
- Python 3 (required by rust-android-gradle)

```shell
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

Set environment variables:

```shell
export ANDROID_HOME="$HOME/Library/Android/sdk"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$PATH"
```

## Skills

### /vectis:core-writer

Generate or update the Rust Crux shared crate from Specify artifacts.

**Inputs:** `spec.md`, `design.md`. The core-writer does not read `composition.yaml` directly -- layout is a shell concern. Per-page view struct fields align with `composition.yaml` field bindings via `design.md`.

**Outputs:** `shared/Cargo.toml`, `shared/src/app.rs` (Model, Event, ViewModel, Effect, `update()`, `view()`, tests), `shared/src/ffi.rs`, `shared/src/lib.rs`, workspace `Cargo.toml`, `clippy.toml`, `rust-toolchain.toml`.

**Modes:**
- **Create** -- invokes `specify tool run vectis -- scaffold core <app-name>` to render the core scaffold, then applies feature-specific code and host workflow steps.
- **Update** -- reads existing `app.rs`, compares against specs, and makes targeted edits.

### /vectis:test-writer

Generate or update test suites with spec-to-test traceability.

**Outputs:** Tests in `shared/src/app.rs` under `#[cfg(test)]` with `/// Spec: REQ-XXX` doc comments mapping each test to a requirement.

### /vectis:core-reviewer

Review Crux core code using an agent team (structural, logic, quality specialists + antagonist).

**Categories:** Structural (CRX), Logic (LOG), Quality (GEN), Universal (UNI).

### /vectis:ios-writer

Generate or update the SwiftUI iOS shell.

**Inputs:** `app.rs`, `spec.md`, `design.md`, `tokens.yaml`, `assets.yaml`, and `composition.yaml` (when present). When `composition.yaml` is present, the region structure and group container tree provide deterministic layout instructions -- groups map to `HStack`/`VStack`/`ZStack` with their layout properties, sizing maps to `.frame()` modifiers, and surface decoration maps to styled container views. When absent, the writer falls back to convention-based inference. Platform-specific overrides from `composition.yaml` `platforms.ios` take precedence over shared regions.

**Outputs:** `project.yml`, `Makefile`, `Core.swift` (bridge), `ContentView.swift`, per-screen views under `Views/`, app entry point, shell-local theme code under `Theme/`.

**Modes:** Create (scaffold + generate) and Update (targeted edits).

### /vectis:ios-reviewer

Review iOS shell code using an agent team (structural, quality, integration specialists + antagonist).

### /vectis:android-writer

Generate or update the Kotlin/Jetpack Compose Android shell.

**Inputs:** `app.rs`, `spec.md`, `design.md`, `tokens.yaml`, `assets.yaml`, and `composition.yaml` (when present). When `composition.yaml` is present, groups map to `Row`/`Column`/`Box` with `Arrangement`/`Alignment`, sizing maps to `Modifier.fillMaxWidth()` etc., and surface decoration maps to `Card`/`Surface`. When absent, the writer falls back to inference. Platform-specific overrides from `composition.yaml` `platforms.android` take precedence over shared regions.

**Outputs:** Gradle build files, `Core.kt` (bridge), `MainActivity.kt`, per-screen composables under `ui/screens/`, Material 3 theme. All composables use Material 3 tokens.

**Modes:** Create (scaffold + generate) and Update (targeted edits).

### /vectis:android-reviewer

Review Android shell code using an agent team (structural, quality, integration specialists + antagonist).

### Screenshots source adapter

Spatial inference over screenshots lives on the [`screenshots` source adapter](../../../adapters/sources/screenshots/adapter.yaml), not the Vectis plugin. The two operations of the source adapter — [`enumerate`](../../../adapters/sources/screenshots/briefs/enumerate.md) and [`extract`](../../../adapters/sources/screenshots/briefs/extract.md) — replace the retired `vectis-image-layout-inferer` skill; the inference algorithm is unchanged. `/spec:plan` runs `enumerate` to identify candidate screens; `/spec:refine` runs `extract` to emit `region` / `container` / `leaf` Evidence claims with `documentation` authority. Downstream `adapters/targets/vectis/build` consumes those claims when regenerating `composition.yaml` from the synthesised `spec.md` / `design.md`.

### /vectis:template-updater

Fix Vectis CLI templates and version pins when upstream crate or tooling bumps break freshly scaffolded projects.

**When to use:** a skill-owned fresh scaffold or verification pass reports template or version-pin drift, or a Crux/UniFFI/Gradle release introduces template drift.

## Platforms

Platforms are declared in the proposal and determine which skills the build phase invokes:

| Platform | Build skill | Description |
|----------|------------|-------------|
| `core` | `vectis:core-writer` | Rust Crux shared crate (always required) |
| `ios` | `vectis:ios-writer` | SwiftUI iOS shell |
| `android` | `vectis:android-writer` | Kotlin/Jetpack Compose Android shell |

Build order: core first, shells second.

## Adapters

The core-writer detects which Crux adapters your app needs from the design document:

| Adapter | When to include |
|-----------|----------------|
| **Render** | Always (automatic) |
| **HTTP** (`crux_http`) | App calls a REST API |
| **Key-Value** (`crux_kv`) | App persists data locally |
| **Time** (`crux_time`) | App uses timers or scheduling |
| **Platform** (`crux_platform`) | App detects the runtime platform |
| **SSE / Streaming** (custom) | App subscribes to server-sent events |

## Design system

Each shell writer reads `tokens.yaml` and `assets.yaml` directly and emits shell-local theme + asset code under its own tree (`iOS/<App>/Theme/` for iOS, `Android/.../ui/theme/` for Android). There is no shared design-system library.

| Path | Purpose |
|------|---------|
| `design-system/spec.md`     | Semantic color roles, typography, spacing rules               |
| `design-system/tokens.yaml` | Concrete token values (source of truth)                       |
| `design-system/assets.yaml` | Asset manifest (images, icons, vectors)                       |

Spatial layout enters the workflow through the [`screenshots` source adapter](../../../adapters/sources/screenshots/adapter.yaml). Its `enumerate` identifies candidate screens from a bound directory; its `extract` emits structured spatial Evidence (`region` / `container` / `leaf` claims). Core synthesis folds those claims into `spec.md` / `design.md`, and the Vectis target's `build` produces a target-specific `composition.yaml` alongside implementation code. `composition.yaml` is no longer a Specify artifact in 2.0 — it is regenerated on each `/spec:execute`.

Update flow: edit `tokens.yaml` or `assets.yaml`, then re-run the relevant shell writer. For layout changes, re-bind the `screenshots` source on the next `/spec:plan` (or hand-author equivalent claims via a local source adapter).

## Working with Xcode

After generating an iOS shell:

```bash
cd path/to/ios
make build          # typegen + package + xcode project generation
open MyApp.xcodeproj
```

The `.xcodeproj` is generated from `project.yml` (XcodeGen) and gitignored. Regenerate with `make xcode` if Xcode state becomes corrupted.

## Working with Android

After generating an Android shell:

```bash
cd path/to/Android
make build          # typegen + cross-compile Rust
./gradlew :shared:cargoBuild
./gradlew :app:assembleDebug
```

### Common issues

- **`UnsatisfiedLinkError` on launch** -- ensure the `Application` class sets the UniFFI library override before any UniFFI class loads.
- **Java 25+ `IllegalArgumentException`** -- pin `org.gradle.java.home` to Java 21 in `gradle.properties`.
- **Gradle version mismatch** -- update `gradle-wrapper.properties` to match AGP requirements.
