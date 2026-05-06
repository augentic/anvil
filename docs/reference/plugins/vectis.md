# Vectis Plugin

Generate cross-platform [Crux](https://github.com/redbadger/crux) applications: Rust shared core, SwiftUI iOS shell, Kotlin/Jetpack Compose Android shell, and VectisDesign token system.

> **CLI entry point.** Vectis project scaffolding and verification ship as the standalone [`specify-vectis`](../cli/vectis.md) binary (RFC-13 §4.3a) — the five canonical verbs (`init`, `verify`, `add-shell`, `update-versions`, `versions`) are accessible either via `specify-vectis` on `$PATH` or via the `specify-vectis` library API for in-process callers. The pre-RFC-13 `specify vectis ...` subcommand tree was retired in chunk 2.6.

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
- **Create** -- invokes `specify-vectis init` to scaffold, then applies feature-specific code.
- **Update** -- reads existing `app.rs`, compares against specs, and makes targeted edits.

### /vectis:test-writer

Generate or update test suites with spec-to-test traceability.

**Outputs:** Tests in `shared/src/app.rs` under `#[cfg(test)]` with `/// Spec: REQ-XXX` doc comments mapping each test to a requirement.

### /vectis:core-reviewer

Review Crux core code using an agent team (structural, logic, quality specialists + antagonist).

**Categories:** Structural (CRX), Logic (LOG), Quality (GEN), Universal (UNI).

### /vectis:ios-writer

Generate or update the SwiftUI iOS shell.

**Inputs:** `app.rs`, `spec.md`, `design.md`, `tokens.yaml`, and `composition.yaml` (when present). When `composition.yaml` is present, the region structure and group container tree provide deterministic layout instructions -- groups map to `HStack`/`VStack`/`ZStack` with their layout properties, sizing maps to `.frame()` modifiers, and surface decoration maps to styled container views. When absent, the writer falls back to convention-based inference. Platform-specific overrides from `composition.yaml` `platforms.ios` take precedence over shared regions.

**Outputs:** `project.yml`, `Makefile`, `Core.swift` (bridge), `ContentView.swift`, per-screen views under `Views/`, app entry point. All views use the VectisDesign package.

**Modes:** Create (scaffold + generate) and Update (targeted edits).

### /vectis:ios-reviewer

Review iOS shell code using an agent team (structural, quality, integration specialists + antagonist).

### /vectis:android-writer

Generate or update the Kotlin/Jetpack Compose Android shell.

**Inputs:** `app.rs`, `spec.md`, `design.md`, `tokens.yaml`, and `composition.yaml` (when present). When `composition.yaml` is present, groups map to `Row`/`Column`/`Box` with `Arrangement`/`Alignment`, sizing maps to `Modifier.fillMaxWidth()` etc., and surface decoration maps to `Card`/`Surface`. When absent, the writer falls back to inference. Platform-specific overrides from `composition.yaml` `platforms.android` take precedence over shared regions.

**Outputs:** Gradle build files, `Core.kt` (bridge), `MainActivity.kt`, per-screen composables under `ui/screens/`, Material 3 theme. All composables use Material 3 tokens.

**Modes:** Create (scaffold + generate) and Update (targeted edits).

### /vectis:android-reviewer

Review Android shell code using an agent team (structural, quality, integration specialists + antagonist).

### /vectis:design-system-writer

Generate VectisDesign from `tokens.yaml`.

**Inputs:** `design-system/tokens.yaml` (single source of truth).

**Outputs:**
- iOS: Swift Package under `design-system/ios/` (`VectisDesign`).
- Android: Gradle module under `design-system/android/` (`vectis-design`).

Token value shapes: color (`light`/`dark`), font (`size`/`weight`), scalar (plain number).

### /vectis:template-updater

Fix Vectis CLI templates and version pins when upstream crate or tooling bumps break freshly scaffolded projects.

**When to use:** `specify-vectis update-versions --verify` reports failures, or a Crux/UniFFI/Gradle release introduces template drift.

## Platforms

Platforms are declared in the proposal and determine which skills the build phase invokes:

| Platform | Build skill | Description |
|----------|------------|-------------|
| `core` | `vectis:core-writer` | Rust Crux shared crate (always required) |
| `ios` | `vectis:ios-writer` | SwiftUI iOS shell |
| `android` | `vectis:android-writer` | Kotlin/Jetpack Compose Android shell |
| `design-system` | `vectis:design-system-writer` | VectisDesign from tokens.yaml |

Build order: design-system first, core second, shells last.

## Capabilities

The core-writer detects which Crux capabilities your app needs from the design document:

| Capability | When to include |
|-----------|----------------|
| **Render** | Always (automatic) |
| **HTTP** (`crux_http`) | App calls a REST API |
| **Key-Value** (`crux_kv`) | App persists data locally |
| **Time** (`crux_time`) | App uses timers or scheduling |
| **Platform** (`crux_platform`) | App detects the runtime platform |
| **SSE / Streaming** (custom) | App subscribes to server-sent events |

## Design system

The design system provides platform-agnostic tokens with platform-specific implementations:

| Path | Purpose |
|------|---------|
| `design-system/spec.md` | Semantic color roles, typography, spacing rules |
| `design-system/tokens.yaml` | Concrete token values (source of truth) |
| `design-system/ios/` | VectisDesign Swift Package |
| `design-system/android/` | vectis-design Gradle module (Compose M3) |

Update flow: edit `tokens.yaml` then regenerate with the design-system-writer skill.

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
