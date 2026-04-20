---
name: android-writer
description: Generate or update a Kotlin/Jetpack Compose Android shell for a Crux application. Use when the user wants to create an Android shell, scaffold Android UI, or generate Compose views for a Crux app, or mentions android-writer.
---

# Crux Android Shell Generator

Generate or update a buildable Kotlin/Jetpack Compose Android shell for an
existing Crux core application. The shell renders the core's `ViewModel`,
dispatches `Event` values from user interactions, and handles platform
side-effects (HTTP, KV, SSE, Time, Platform) on behalf of the core.

When an existing Android shell is detected, the skill operates in **update
mode**: it compares the current `app.rs` types against the existing Kotlin code
and makes targeted edits rather than regenerating from scratch.

When no Android shell exists yet, the skill runs `specify vectis add-shell android
--dir {app-dir}` (optionally with `--android-package`) to scaffold the project.
The CLI owns all build infrastructure: `Android/Makefile`, the
`build.gradle.kts` / `settings.gradle.kts` / `gradle.properties` triad, the
`gradle/libs.versions.toml` version catalog, `app/build.gradle.kts` and
`shared/build.gradle.kts`, the Gradle wrapper (`gradlew`, `gradlew.bat`,
`gradle/wrapper/`), `local.properties`, `AndroidManifest.xml` (including
conditional `networkSecurityConfig` when HTTP/SSE is selected), the
`{AppName}Application.kt` entry point with the UniFFI library override, a
render-only baseline `Core.kt` with CAP markers, `MainActivity.kt`, the
`ui/screens/HomeScreen.kt` starter, and `res/xml/network_security_config.xml`
when needed. Once the scaffold exists this skill switches to **update mode**
and layers spec-driven changes over the generated baseline.

This skill targets **Kotlin 2.x**, **Jetpack Compose** with Material 3, and
minimum SDK 34.

## Arguments

| Argument | Required | Description |
|---|---|---|
| `app-dir` | **Yes** | Path to the Crux app directory (must contain `shared/src/app.rs`) |
| `project-dir` | No | Directory for the Android shell. Defaults to `{app-dir}/Android` |
| `change-dir` | No | Path to `.specify/changes/<change>/`. When provided, the skill reads the `## Android Shell Requirements` section from `{change-dir}/specs/{feature-name}/spec.md` for platform-specific requirements |

## Prerequisites

The following tools must be installed:

- Android SDK (command-line tools, platform-tools, emulator)
- Android NDK (install via `sdkmanager "ndk;29.0.14206865"` or through SDK Manager)
- Rust Android targets: `rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`
- Python 3 (required by Mozilla's rust-android-gradle plugin)
- Java 21 LTS JDK (NOT Java 25+ -- Gradle's embedded Kotlin compiler cannot
  parse Java 25+ version strings, causing a cryptic `IllegalArgumentException`
  at build time)

Environment variables must be set:

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
export PATH="$ANDROID_HOME/platform-tools:$PATH"
export PATH="$ANDROID_HOME/emulator:$PATH"
```

## Input Analysis

The android-writer reads the Crux core source to determine what the shell must
render and handle. Read `{app-dir}/shared/src/app.rs` and extract:

| Extract | Source | Maps to |
|---|---|---|
| App struct name | `impl App for X` | App name, package name |
| ViewModel variants | `enum ViewModel` | `when` branches in the main composable |
| Per-page view structs | Structs wrapped by ViewModel variants | Screen composable properties and layout |
| Shell-facing Event variants | `enum Event` (non-`#[serde(skip)]`, non-`#[facet(skip)]`) | User interaction handlers in screen composables |
| Effect variants | `enum Effect` | `processRequest` `when` branches in Core.kt |
| Route variants | `enum Route` | Navigation destinations |
| Supporting types | Structs/enums used in view structs | Display data types |

Also read:
- `{app-dir}/shared/src/lib.rs` -- custom capability modules
- `{app-dir}/shared/Cargo.toml` -- capability dependencies
- `{app-dir}/shared/src/ffi.rs` -- CoreFFI struct definition
- `{app-dir}/shared/src/bin/codegen.rs` -- codegen binary for type generation
- `design-system/tokens.yaml` -- design tokens for styling
- `design-system/spec.md` -- design system usage rules

When `change-dir` is provided, also read:
- `{change-dir}/specs/{feature-name}/spec.md` -- read the `## Android Shell Requirements`
  section for platform-specific behavioral requirements (navigation style, gestures,
  haptics, accessibility). Also read the `## Android Shell Details` section of
  `{change-dir}/design.md` for platform design decisions.

## Generated Type Conventions (CRITICAL)

The codegen binary produces two sets of Kotlin files:

1. **Bincode types** in `generated/com/example/app/` -- `Event`, `ViewModel`,
   `Effect`, `Request`, `Requests`, and all view structs, enums, and capability
   types (`HttpRequest`, `HttpResponse`, `SseRequest`, `SseResponse`,
   `KeyValueOperation`, `TimeRequest`, `TimeResponse`, `Filter`, etc.)
2. **UniFFI bindings** in `generated/uniffi/shared/` -- the `CoreFfi` class
   that bridges to the Rust native library.

### Import rules for all hand-written Kotlin files

Every `.kt` file that references generated types MUST have explicit imports:

```kotlin
// For bincode types (Event, ViewModel, Effect, etc.)
import com.example.app.Event
import com.example.app.ViewModel
import com.example.app.Effect
// ... import each type individually

// For the CoreFfi bridge (only in Core.kt)
import uniffi.shared.CoreFfi
```

**NEVER** assume these types are in the same package as the hand-written code.
The hand-written code lives in `com.vectis.{appname}` but the generated types
are in `com.example.app` and `uniffi.shared`.

### Enum class naming conventions

Simple Rust enums without payloads (e.g., `Filter { All, Active, Completed }`,
`SyncStatus { Idle, Syncing, Offline }`, `SseState`) are generated as Kotlin
`enum class` with **UPPER_CASE** values:

```kotlin
// Generated as:
enum class Filter { ALL, ACTIVE, COMPLETED }
enum class SyncStatus { IDLE, SYNCING, OFFLINE }
enum class SseState { DISCONNECTED, CONNECTING, CONNECTED }
```

Pattern match with `==` equality, NOT `is`:

```kotlin
// CORRECT:
when (filter) {
    Filter.ALL -> ...
    Filter.ACTIVE -> ...
    Filter.COMPLETED -> ...
}

// WRONG (will not compile):
when (filter) {
    is Filter.All -> ...    // ← enum values are not types
}
```

### Sealed interface naming conventions

Rust enums WITH payloads (e.g., `Event`, `ViewModel`, `Effect`) are generated
as Kotlin `sealed interface` with nested `data class` or `data object` variants:

```kotlin
// Generated as:
sealed interface Event {
    data class Navigate(val value: Route) : Event
    data class SetNewTitle(val value: String) : Event
    data object ClearCompleted : Event     // unit variant → data object
}
```

Pattern match with `is` for data classes, direct reference for data objects:

```kotlin
when (event) {
    is Event.Navigate -> event.value       // data class
    is Event.SetNewTitle -> event.value    // data class
    Event.ClearCompleted -> ...            // data object (no `is`)
}
```

### Numeric type mapping

| Rust type | Kotlin generated type | Notes |
|---|---|---|
| `usize` / `u64` | `ULong` | Use `.toLong()` when passing to Compose UI that expects `Long` |
| `u32` | `UInt` | Effect IDs are `UInt` |
| `u16` | `UShort` | HTTP status codes |
| `Vec<u8>` | `List<UByte>` | Use `.toUByteArray().toList()` to convert from `ByteArray` |

### KeyValue types

- `Value.Bytes` takes `List<UByte>` (not `List<Byte>`) -- convert with
  `byteArray.toUByteArray().toList()`
- `KeyValueOperation.Set.value` is `List<UByte>` -- convert back with
  `op.value.map { it.toByte() }.toByteArray()`
- `KeyValueResponse.ListKeys` takes `(keys: List<String>, nextCursor: ULong)` --
  pass `0UL` for no more keys, NOT a `String`
- `KeyValueError` is a sealed interface with variants `Io`, `Timeout`,
  `CursorNotFound`, `Other` -- use `KeyValueError.Other(msg)`, NOT
  `KeyValueError(msg)`

### Time types

- `Duration` has a single field `nanos: ULong` (total nanoseconds), NOT
  separate `secs`/`nanos` fields
- `TimeRequest` variants: `Now`, `NotifyAt(id, instant)`,
  `NotifyAfter(id, duration)`, `Clear(id)` -- each has a `TimerId` field
- `TimeResponse` variants: `Now(instant)`, `InstantArrived(id)`,
  `DurationElapsed(id)`, `Cleared(id)` -- NOT `DURATIONREACHED`
- `NotifyAfter` and `NotifyAt` handlers must store their coroutine `Job` in a
  `timerJobs` map keyed by `TimerId`. `Clear` must cancel and remove the stored
  job before responding with `Cleared`. Without job tracking, cleared timers
  continue to fire stale events into the core.

### @OptIn annotations

Classes that call `.toUByteArray()` need:

```kotlin
@OptIn(ExperimentalUnsignedTypes::class)
class SseClient { ... }
```

## Mode Detection

- **Create Mode** -- `{project-dir}/` does **not** exist. The skill invokes
  `specify vectis add-shell android` to scaffold the baseline, then proceeds directly
  into Update Mode to apply feature-specific changes from the Specify
  artifacts.
- **Update Mode** -- `{project-dir}/` **does** exist and contains `.kt` files.
  Read existing code, diff against the core, and make targeted edits
  (steps U1--U8 below).

Detection rule: check for `{project-dir}/app/src/main/java/*/Core.kt`. If
present, switch to update mode. If not, run:

```bash
specify vectis add-shell android --dir {app-dir} [--android-package com.example.app]
```

`{app-dir}` is the parent directory of `shared/`; the CLI derives the
`Android/` sibling directory automatically. `--android-package` is optional
and defaults to `com.vectis.<appname-lowercase>`. On non-zero exit, surface
the CLI's structured error output to the user and stop -- do **not** attempt
to hand-author Gradle files, the wrapper, `AndroidManifest.xml`, or any of
the baseline `.kt` files.

If the command succeeds, switch to Update Mode. The scaffolded shell is a
render-only baseline with CAP markers in `Core.kt`, `AndroidManifest.xml`,
`libs.versions.toml`, and `app/build.gradle.kts` (one block per optional
capability: `http`, `kv`, `sse`, `time`, `platform`) plus a starter
`HomeScreen.kt`. Update Mode expands CAP blocks for capabilities the core
uses, strips CAP blocks for capabilities it does not, and replaces the starter
screen with real per-ViewModel-variant screens derived from the current
`app.rs`.

## Verification ownership

When the orchestrator passes `skip_verification: true`, the writer stops
after code generation and does **not** run step U8. The orchestrator's
dedicated Android verify sub-agent handles pre-flight checks, `make build`,
`./gradlew :shared:cargoBuild`, and `./gradlew :app:assembleDebug` with its
own repair loop and iteration limits.

When invoked **standalone** (no `skip_verification` flag, or
`skip_verification: false`), the writer runs its full process including step
U8.

---

## Process: Create Mode

Use this process when no Android shell exists at `{project-dir}`. The CLI owns
all Android boilerplate (Gradle build files + wrapper, version catalog,
`AndroidManifest.xml` with conditional `networkSecurityConfig`, render-only
baseline `Core.kt` with CAP markers, `{AppName}Application.kt` with the UniFFI
library override, `MainActivity.kt`, `ui/screens/HomeScreen.kt`,
`res/xml/network_security_config.xml` when HTTP/SSE is selected, and
`local.properties` with a best-effort Java 21 pin via `org.gradle.java.home`).
This skill's only Create-Mode responsibilities are: (1) read the Crux core to
derive the app name and capability set, (2) invoke the CLI, (3) switch to
Update Mode.

### 1. Read the Crux core

Read `{app-dir}/shared/src/app.rs` and extract all types listed in the Input
Analysis table above. In particular, derive the App struct name (used by the
CLI to name the Gradle project, Application class, package folder, and
Android theme) and note which capabilities the core actually uses -- this
drives which CAP blocks Update Mode must expand in the scaffolded `Core.kt`,
`AndroidManifest.xml`, `app/build.gradle.kts`, and `libs.versions.toml`. If
`app.rs` cannot be read or parsed, report the error and stop.

### 2. Invoke the CLI

Run:

```bash
specify vectis add-shell android --dir {app-dir} [--android-package <package>]
```

`--android-package` is optional; when omitted, the CLI uses
`com.vectis.<appname-lowercase>`. The CLI generates the full Gradle project
(root + `app` + `shared` modules), the Gradle wrapper artefacts via a
scratch bootstrap, `local.properties` with the detected SDK path, and a
render-only baseline shell in `Android/app/src/main/java/<package-path>/`
with CAP markers for every optional capability. The output is structured
JSON. On non-zero exit, surface the CLI's error output to the user and stop.

### 3. Switch to Update Mode

After the CLI returns green, treat the scaffolded Android shell as an
existing implementation and execute **Process: Update Mode** below to:

- Expand CAP blocks in `Core.kt` (adding real effect handlers + client
  classes) for capabilities the core uses, and strip CAP blocks for
  capabilities it does not.
- Expand matching CAP blocks in `AndroidManifest.xml` (`INTERNET`
  permission, `networkSecurityConfig`), `app/build.gradle.kts` (Ktor / Koin
  dependencies), and `libs.versions.toml` (capability library versions) -- or
  strip them if the capability is absent.
- Replace the `HomeScreen` starter with real per-ViewModel-variant screen
  files driven by the core's `ViewModel` enum + per-page view structs.
- Rewrite the root composable in `MainActivity.kt` to cover every ViewModel
  variant.
- Apply any `## Android Shell Requirements` from the active Specify change
  (when `change-dir` is provided).

Dependency version pins (Kotlin, AGP, Ktor, Koin, Compose BOM, etc.) come
from the CLI's embedded `versions.toml`; use `specify vectis update-versions` to
refresh them rather than hand-editing `libs.versions.toml` /
`shared/build.gradle.kts`.

## Process: Update Mode

Use this process when `{project-dir}/` already exists with Kotlin files.

### U1. Read and analyze the Crux core

Same as create mode step 1 (read `{app-dir}/shared/src/app.rs` and extract
the full type inventory using the Input Analysis table above).

When `change-dir` is provided, also read the `## Android Shell Requirements` section
from `{change-dir}/specs/{feature-name}/spec.md` and the `## Android Shell Details`
section from `{change-dir}/design.md` for platform-specific requirements.

### U2. Read existing Kotlin code

Read all `.kt` files in the project:

- `core/Core.kt` -- current effect handler `when` branches
- `ui/screens/*.kt` -- current screen composables
- `MainActivity.kt` -- current root composable and ViewModel switch
- `di/AppModule.kt` -- current DI configuration (if present)

### U3. Build implementation inventory

Extract from existing Kotlin code:

| Category | What to extract |
|---|---|
| Effect handlers | Cases in `processRequest` `when` expression |
| ViewModel cases | Branches in root composable `when` expression |
| Screen composables | `.kt` files in `ui/screens/` |
| Event dispatches | All `onEvent(...)` or `core.update(...)` calls |
| Capability clients | Client classes in `core/` |
| DI modules | Koin module definitions |

### U4. Diff analysis

Compare the Rust core types (from U1) against the Kotlin inventory (from U3).
For each category, classify items as Added, Removed, Modified, or Unchanged.

Walk through in this order:

1. **Effect variants** -- new or removed capabilities affect Core.kt and
   may require new client classes.
2. **ViewModel variants** -- new or removed views affect the root composable
   and screen composable files.
3. **Per-page view struct fields** -- changed display data affects screen
   composables.
4. **Event variants** -- new or removed user actions affect screen composables.
5. **Route variants** -- new or removed navigation destinations affect
   navigation code.

Output the diff summary before making edits.

### U5. Apply changes to Core.kt

- Add new effect handler cases for added capabilities.
- Remove effect handler cases for removed capabilities.
- Add or remove capability client classes as needed.
- Update DI module if new dependencies are required.

### U6. Apply changes to composables

- Add new screen composable files for added ViewModel variants.
- Remove screen composable files for removed ViewModel variants.
- Update the root composable `when` to add/remove cases.
- Update existing screen composables for changed per-page view struct fields.
- Add/remove event dispatch calls for changed Event variants.

### U7. Update build configuration

- Update `build.gradle.kts` files if new dependencies are needed.
- Update `libs.versions.toml` if new library versions are needed.
- Update `AndroidManifest.xml` if permissions changed (e.g., INTERNET for HTTP).

### U8. Build and verify

Pre-flight checks before build (all generated by `specify vectis add-shell android` --
this step just confirms nothing drifted):

1. Verify `{project-dir}/gradlew` exists.
2. Verify `local.properties` has `sdk.dir` set.
3. Verify `gradle.properties` has `org.gradle.java.home` pointing to Java 21.
4. Verify Rust Android targets are installed:
   `rustup target list --installed | grep android`.

Build sequence:

1. Run `make build` in `{project-dir}` to regenerate types.
2. Run `./gradlew :shared:cargoBuild` to cross-compile the Rust library.
3. Run `./gradlew :app:assembleDebug` to build the APK.
4. Fix any build errors.

## Spec-to-Code Mapping

| Rust Type (in `app.rs`) | Kotlin Artifact | File |
|---|---|---|
| `enum ViewModel { Loading, Main(MainView) }` | `when (state) { is ViewModel.Loading -> ... is ViewModel.Main -> ... }` | `MainActivity.kt` |
| ViewModel variant `Main(MainView)` | `@Composable fun MainScreen(viewModel: MainView, onEvent: (Event) -> Unit)` | `ui/screens/MainScreen.kt` |
| `struct MainView { pub items: Vec<ItemView> }` | Function parameter: `viewModel: MainView` | `ui/screens/MainScreen.kt` |
| Shell-facing `Event::AddItem(String)` | `onEvent(Event.AddItem(text))` | Relevant screen composable |
| `Effect::Http(HttpRequest)` | `is Effect.Http -> { httpClient.request(effect.value) }` | `core/Core.kt` |
| `enum Route { Main, Settings }` | Navigation destinations | `MainActivity.kt` |

## Preservation Rules (Update Mode)

1. **Never regenerate a file from scratch.** Make targeted edits.
2. **Preserve custom styling** that the developer added beyond the Material 3
   defaults.
3. **Preserve custom composable logic** (e.g., animations, gestures) that is
   not driven by the ViewModel.
4. **Preserve `@Preview` blocks** on unchanged composables.
5. **Preserve Gradle customizations** (signing, flavors, custom build phases).
6. **Preserve `Makefile` customizations** (additional targets, environment
   variables).

## Reference Documentation

| Reference | Purpose |
|---|---|
| `references/crux-android-shell-pattern.md` | Core.kt template, effect handling, serialization protocol |
| `references/compose-view-patterns.md` | Screen patterns, lists, forms, navigation, accessibility |
| `references/design-system-integration.md` | Design system token usage in composables |

Gradle build files, the version catalog, the Gradle wrapper, the Makefile,
`AndroidManifest.xml`, CAP-marker scaffolding for the baseline `Core.kt` /
`app/build.gradle.kts` / `libs.versions.toml`, Java-21 auto-pinning, and the
starter Kotlin layout are owned by the CLI's embedded templates in the
[`augentic/specify-cli`](https://github.com/augentic/specify-cli) repo
(`<specify-cli>/crates/vectis/src/init/android.rs` and
`<specify-cli>/templates/vectis/android/`). Do not hand-edit those files in
Create Mode; let `specify vectis add-shell android` write them and then
modify in Update Mode.

## Examples

| Example | Capabilities | Demonstrates |
|---|---|---|
| `references/examples/01-simple-counter-android.md` | Render | Minimal shell, Core.kt, two screens, project setup |
| `references/examples/02-http-counter-android.md` | Render + HTTP + SSE | Async HTTP handling, Koin DI, SSE streaming, Ktor |

## Error Handling

### Build errors

| Error | Resolution |
|---|---|
| `app.rs` not found | Verify `app-dir` points to a Crux app with `shared/src/app.rs` |
| Unknown Effect variant | Add a placeholder `is Effect.XXX -> { }` and report |
| Gradle sync fails | Check `build.gradle.kts` syntax; verify NDK version matches installed |
| Build fails with missing types | Run `make build` to regenerate types; verify `uniffi` is pinned to `"=0.29.4"` |
| `cargoBuild` fails with `target may not be installed` | Run `rustup target add armv7-linux-androideabi aarch64-linux-android i686-linux-android x86_64-linux-android` |
| NDK not found | Install via `sdkmanager "ndk;29.0.14206865"` or Android Studio SDK Manager |
| Python 3 not found | Required by rust-android-gradle; install via system package manager |
| `./gradlew: No such file or directory` | Scaffold was missing the wrapper -- re-run `specify vectis add-shell android` (the CLI bootstraps `gradlew` from a scratch Gradle invocation) |
| `Minimum supported Gradle version is X.Y` | Gradle/AGP drift -- run `specify vectis update-versions` and re-run `specify vectis add-shell android` (the CLI pins `gradle-wrapper.properties` to match AGP from `versions.toml`) |
| `java.lang.IllegalArgumentException: 25.0.1` (or similar Java version parse error) | Set `org.gradle.java.home` to Java 21 in `gradle.properties` (the CLI auto-pins this when Java 21 is detected; if none was detected at scaffold time, set it by hand) |
| `resource style/Theme.{AppName} not found` | Scaffold was missing `res/values/themes.xml` -- re-run `specify vectis add-shell android` |
| `Unresolved reference 'Event'` (or `ViewModel`, `Effect`, etc.) | Add `import com.example.app.*` imports to the affected Kotlin file |
| `Unresolved reference 'CoreFfi'` | Add `import uniffi.shared.CoreFfi` to `Core.kt` |
| `Unresolved reference 'Icons'` | Add `material-icons-extended` dependency to `libs.versions.toml` + `app/build.gradle.kts` |
| `Namespace 'X' is used in multiple modules` | Use `com.vectis.{appname}.shared` namespace for the shared module |
| `unresolved module path shared::ffi` (codegen error) | UniFFI version mismatch -- ensure `uniffi = "=0.29.4"` in shared `Cargo.toml` |
| `This declaration needs opt-in` (unsigned types) | Add `@OptIn(ExperimentalUnsignedTypes::class)` to the class |

### Runtime crashes

| Crash | Resolution |
|---|---|
| `UnsatisfiedLinkError: Unable to load library 'uniffi_shared'` | The CLI-generated `{AppName}Application.kt` already calls `System.setProperty("uniffi.component.shared.libraryOverride", "shared")` first in `onCreate()`; verify the Application class was not replaced or its body reordered |
| `CLEARTEXT communication not permitted` | Ensure the CLI was invoked with HTTP or SSE capabilities selected so it emitted `res/xml/network_security_config.xml` and the matching `networkSecurityConfig` attribute in `AndroidManifest.xml` |
| Unhandled exception in SSE/Time coroutine | Wrap `scope.launch` blocks for async effects in `try/catch`, rethrow `CancellationException`, and resolve the effect request with a fallback response (`SseResponse.Done`, `TimeResponse.DurationElapsed`, etc.) so the Rust core is never left awaiting an unresolved ID |

## Verification Checklist

### Build

- [ ] `gradlew` exists (Gradle wrapper was generated)
- [ ] `local.properties` has `sdk.dir` set
- [ ] `gradle.properties` has `org.gradle.java.home` pointing to Java 21
- [ ] `make build` completes without errors (type generation)
- [ ] `./gradlew :shared:cargoBuild` compiles Rust for all 4 ABIs
- [ ] `./gradlew :app:assembleDebug` builds the APK without errors
- [ ] APK installs and launches on emulator without crashing

### Structure

- [ ] Every ViewModel variant has a corresponding screen composable file
- [ ] Every ViewModel variant has a branch in the root composable `when`
- [ ] Every Effect variant has a branch in `processRequest` `when`
- [ ] Every shell-facing Event variant is dispatched by at least one composable
- [ ] `Core.kt` handles all effects from the core
- [ ] Generated types directory is in `.gitignore`
- [ ] `res/values/themes.xml` exists with the app theme
- [ ] `res/xml/network_security_config.xml` exists (if HTTP/SSE effects)
- [ ] `AndroidManifest.xml` references the network security config (if HTTP/SSE)
- [ ] App module namespace differs from shared module namespace

### Imports and Types

- [ ] All hand-written `.kt` files import generated types from `com.example.app.*`
- [ ] `Core.kt` imports `uniffi.shared.CoreFfi`
- [ ] Application class calls `System.setProperty("uniffi.component.shared.libraryOverride", "shared")` first in `onCreate()`
- [ ] Simple enum comparisons use `==` (e.g., `Filter.ALL`), not `is`
- [ ] `ULong` values are cast to `Long` for Compose text display
- [ ] Classes using `toUByteArray()` have `@OptIn(ExperimentalUnsignedTypes::class)`

### Design System

When `design-system/tokens.yaml` exists:

- [ ] `settings.gradle.kts` includes `:vectis-design` with correct `projectDir`
- [ ] `app/build.gradle.kts` has `implementation(project(":vectis-design"))`
- [ ] `AppTheme` wraps `VectisTheme`; app `ui/theme/` has no duplicate `Color.kt` / `Type.kt`
- [ ] Screen composables use `MaterialTheme.colorScheme` / `MaterialTheme.typography` (no hardcoded hex in `app/`)
- [ ] Spacing and corner radii use `VectisSpacing` / `VectisCornerRadius` from `com.vectis.design`

### Quality

- [ ] Every screen composable has a `@Preview` with sample data
- [ ] Interactive icons have `contentDescription` for accessibility
- [ ] No force unwraps or `!!` in production code
- [ ] CoreFFI calls (`coreFfi.update()`, `coreFfi.view()`, `coreFfi.resolve()`) use `try/catch` with `Log.e` including `${e.message}`
- [ ] Bincode calls (`bincodeSerialize()`, `bincodeDeserialize()`) use `try/catch` with `Log.w` and safe fallback
- [ ] `Effect.Render` handler preserves existing view on failure (returns without assignment, not fallback to `ViewModel.Loading`)
- [ ] `initialView()` is the only place that falls back to `ViewModel.Loading`
- [ ] HTTP client has proper timeout configuration
- [ ] Coroutine scopes use `SupervisorJob` for fault isolation
- [ ] Async effects (SSE, Time) wrapped in `try/catch` inside `scope.launch`
- [ ] `CancellationException` is always rethrown in catch blocks
- [ ] Time `NotifyAfter`/`NotifyAt` jobs tracked in `timerJobs` map; `Clear` cancels stored job

### Command-Line Workflow

- [ ] Build works from terminal: `./gradlew :app:assembleDebug`
- [ ] Emulator can be launched: `emulator -avd <name>`
- [ ] App can be installed: `./gradlew :app:installDebug`
- [ ] App can be launched: `adb shell am start -n <package>/.MainActivity`

## Important Notes

- **Core must exist first**: This skill generates the Android shell for an
  existing Crux core. Run the core-writer skill first to generate the
  `shared` crate.
- **Shell is thin**: All business logic lives in the Rust core. The shell
  only renders composables and performs platform I/O. Never add business
  logic to Kotlin code.
- **UniFFI bridging**: The shared crate must have `crate-type = ["cdylib", "staticlib", "lib"]`
  and the `uniffi` feature gate. The `uniffi` crate must be pinned to
  `"=0.29.4"` to match `crux_core::cli::bindgen`'s bundled `uniffi_bindgen`.
- **UniFFI library name**: Cargo produces `libshared.so` but JNA expects
  `libuniffi_shared.so` by default. The Application class MUST set
  `System.setProperty("uniffi.component.shared.libraryOverride", "shared")`
  before any UniFFI class is loaded. Without this, the app crashes on launch.
- **Generated types live in `com.example.app`**: The codegen binary produces
  Kotlin types (via facet) in `com.example.app.*` and UniFFI bindings in
  `uniffi.shared.*`. These live in the `generated/` directory, which is
  included as a source directory in the `shared` Gradle module. Hand-written
  Kotlin in `com.vectis.{appname}` MUST import them explicitly. This is the
  most common source of "Unresolved reference" compile errors.
- **rust-android-gradle**: Mozilla's plugin cross-compiles the Rust crate into
  `libshared.so` for 4 ABIs (arm, arm64, x86, x86_64). It requires Python 3.
  If Python 3.13+ causes issues with the `pipes` module, use Python 3.12.
- **Two Core patterns**: Simple apps (Render-only) use `Core` extending
  `ViewModel` with `mutableStateOf`. Complex apps (with HTTP/SSE) use a
  plain class with `StateFlow` injected via Koin. Both patterns require
  an Application class for the UniFFI library override, which the CLI always
  emits as `{AppName}Application.kt`.
- **Gradle wrapper is required**: The `gradlew` script must exist before any
  `./gradlew` command works. `specify vectis add-shell android` bootstraps it by
  invoking a temporary Gradle distribution in a scratch directory and copying
  the wrapper artefacts (`gradlew`, `gradlew.bat`, `gradle/wrapper/*`) into
  `Android/`; the wrapper's `distributionUrl` is pinned to match the AGP
  version in the CLI's embedded `versions.toml`.
- **Java 21 LTS required**: Java 25+ has a version string that Gradle's
  Kotlin compiler cannot parse. `specify vectis add-shell android` detects Java 21
  via `/usr/libexec/java_home -v 21` (and equivalent heuristics on Linux) and
  appends `org.gradle.java.home=<path>` to `gradle.properties` when found.
  When no Java 21 is installed, the CLI leaves the pin unset and the user
  must add it by hand.
- **Network security config**: Android 9+ blocks cleartext HTTP traffic by
  default. Apps with HTTP or SSE effects MUST include a
  `network_security_config.xml` to allow cleartext to localhost/`10.0.2.2`
  for development. Without it, the app crashes on first network request.
- **Defensive error handling**: CoreFFI calls (`coreFfi.update()`,
  `coreFfi.view()`, `coreFfi.resolve()`) throw `CoreException` with a
  meaningful Rust-side error message. Always use `try/catch` with
  `Log.e(TAG, "context: ${e.message}", e)` so the diagnostic is visible in
  logcat. Bincode calls use `try/catch` with `Log.w` and a safe fallback.
  The `Effect.Render` handler must preserve the existing view on failure --
  never fall back to `ViewModel.Loading`. All async effect handlers (SSE,
  Time) that run in `scope.launch` blocks MUST wrap their bodies in
  `try/catch` to prevent unhandled exceptions from crashing the app. Always
  rethrow `CancellationException`.
- **themes.xml is mandatory**: `AndroidManifest.xml` references a theme
  resource. The `res/values/themes.xml` file MUST exist or the build fails
  with `resource style/Theme.{AppName} not found`.
- **No Android Studio required for builds**: The Gradle wrapper (`./gradlew`)
  handles compilation. The emulator can be launched from the command line.
  Android Studio is only needed for initial SDK/NDK installation or for the
  visual layout editor.
- **Hot reloading**: Jetpack Compose's built-in Live Edit and `@Preview`
  composables provide the development-time iteration equivalent of iOS's
  Inject/InjectionIII. No additional library integration is needed -- Live
  Edit is available in Android Studio and updates composables on save. Every
  screen composable should include a `@Preview` with sample data (checked by
  AND-008) to enable visual preview without running the emulator.
- **Specify integration**: When `change-dir` is provided, the skill reads
  the `## Android Shell Requirements` section from the feature spec and the
  `## Android Shell Details` section from design.md. The primary input remains
  `app.rs` from the core; the feature spec's platform section supplements
  with requirements that may not be expressed in the Rust types alone
  (e.g., navigation style, specific UX behaviors, accessibility
  requirements, layout constraints).
