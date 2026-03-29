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

This skill targets **Kotlin 2.x**, **Jetpack Compose** with Material 3, and
minimum SDK 34.

## Arguments

| Argument | Required | Description |
|---|---|---|
| `app-dir` | **Yes** | Path to the Crux app directory (must contain `shared/src/app.rs`) |
| `project-dir` | No | Directory for the Android shell. Defaults to `{app-dir}/Android` |

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

### @OptIn annotations

Classes that call `.toUByteArray()` need:

```kotlin
@OptIn(ExperimentalUnsignedTypes::class)
class SseClient { ... }
```

## Mode Detection

- **Create Mode** -- `{project-dir}/` does **not** exist. Generate the entire
  Android shell from scratch (steps 1--12 below).
- **Update Mode** -- `{project-dir}/` **does** exist and contains `.kt` files.
  Read existing code, diff against the core, and make targeted edits
  (steps U1--U8 below).

Check for `{project-dir}/app/src/main/java/*/Core.kt` to detect the mode.
If found, switch to update mode.

## Process: Create Mode

### 1. Read and analyze the Crux core

Read `{app-dir}/shared/src/app.rs` and extract all types listed in the
Input Analysis table above. Build a complete picture of:

- Which ViewModel variants exist (determines number of screens)
- Which per-page view struct fields exist (determines screen layout)
- Which shell-facing Event variants exist (determines user interaction points)
- Which Effect variants exist (determines which platform capabilities to implement)
- Which Route variants exist (determines navigation structure)

If `app.rs` cannot be read or parsed, report the error and stop.

### 2. Determine app name and package

Derive the app name and package from the `App` struct in `app.rs`:

| Rust struct | App name | Package name |
|---|---|---|
| `TodoApp` | `Todo` | `com.vectis.todo` |
| `CounterApp` | `Counter` | `com.vectis.counter` |
| `Counter` | `Counter` | `com.vectis.counter` |
| `NoteEditor` | `NoteEditor` | `com.vectis.noteeditor` |

The app name is used for the Gradle project name, theme, and Activity.

### 3. Determine capability dependencies

From the Effect variants, determine which Android libraries are needed:

| Effect variant | Gradle dependencies | Core.kt handler |
|---|---|---|
| `Render` | (none, always present) | Update `_viewModel` from `coreFfi.view()` |
| `Http(HttpRequest)` | Ktor client (core, okhttp, logging) | `HttpClient` class using Ktor |
| `KeyValue(KeyValueOperation)` | (none, uses SharedPreferences) | `KeyValueClient` using SharedPreferences |
| `ServerSentEvents(SseRequest)` | Ktor client (core, okhttp) | `SseClient` using Ktor streaming |
| `Time(TimeRequest)` | (none, uses java.time) | Inline handler in Core.kt |
| `Platform(PlatformRequest)` | (none, uses android.os.Build) | Inline handler in Core.kt |

If more than one non-Render effect exists, use Koin for dependency injection.

### 4. Generate directory structure

Create the following directories under `{project-dir}`:

```
{project-dir}/
    app/
        src/main/
            java/com/vectis/{appname}/
                core/
                ui/
                    screens/
                    theme/
                di/             # if Koin is used
            res/
                values/
                    themes.xml  # REQUIRED -- AndroidManifest references this
                xml/
                    network_security_config.xml  # if HTTP/SSE effects
            AndroidManifest.xml
        build.gradle.kts
    shared/
        build.gradle.kts
    generated/                  # gitignored -- codegen output
    gradle/
        wrapper/
            gradle-wrapper.jar
            gradle-wrapper.properties
        libs.versions.toml
    gradlew                     # REQUIRED -- Gradle wrapper script
    gradlew.bat
    build.gradle.kts            # root
    settings.gradle.kts
    gradle.properties
    local.properties            # gitignored -- sdk.dir path
    Makefile
    .gitignore
```

### 5. Generate `Makefile`

Create `{project-dir}/Makefile` with the build pipeline. The Makefile runs
the Rust codegen binary to produce Kotlin types and UniFFI bindings.

**CRITICAL**: The `cargo` commands must run from the Rust workspace root,
NOT from the `Android/` directory. The Makefile uses `WORKSPACE := ..` and
`cd $(WORKSPACE) &&` to ensure correct path resolution. The `--output-dir`
must be an absolute or relative-to-workspace path pointing back to the
Android generated directory.

```makefile
.PHONY: all build clean lib typegen android-lib apk install launch run

WORKSPACE := ..

all: build

build: typegen

typegen: lib
	@echo "Generating Kotlin types..."
	@cd $(WORKSPACE) && RUST_LOG=info cargo run \
		--package shared \
		--bin codegen \
		--features codegen,facet_typegen \
		-- \
		--language kotlin \
		--output-dir Android/generated

lib:
	@echo "Building Rust shared library..."
	@cd $(WORKSPACE) && cargo build --features uniffi
```

See `references/android-project-config.md` for the complete Makefile template.

### 6. Generate Gradle files and wrapper

Create the Gradle build configuration following the template in
`references/android-project-config.md`.

Key files:
- `{project-dir}/build.gradle.kts` -- root build file with plugin declarations
- `{project-dir}/settings.gradle.kts` -- includes `:app` and `:shared` modules
- `{project-dir}/gradle.properties` -- Android/Kotlin properties
- `{project-dir}/gradle/libs.versions.toml` -- version catalog
- `{project-dir}/app/build.gradle.kts` -- app module with Compose and dependencies
- `{project-dir}/shared/build.gradle.kts` -- shared module with rust-android-gradle

#### Generate Gradle wrapper (CRITICAL)

The Gradle wrapper (`gradlew`) MUST exist before any `./gradlew` command works.
After creating the Gradle files, generate the wrapper:

```bash
cd {project-dir}
# Create local.properties with SDK path first
echo "sdk.dir=$ANDROID_HOME" > local.properties

# Generate wrapper -- version MUST match AGP requirements
# AGP 8.13.x requires Gradle 8.13+
gradle wrapper --gradle-version 8.13
```

This creates `gradlew`, `gradlew.bat`, and `gradle/wrapper/` files. If
`gradle` is not installed globally, download and use a temporary Gradle
distribution to bootstrap the wrapper.

#### gradle.properties MUST pin Java 21

Java 25+ has a version string format that Gradle's embedded Kotlin compiler
cannot parse, causing `java.lang.IllegalArgumentException` during build.
Always include this in `gradle.properties`:

```properties
org.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8
android.useAndroidX=true
kotlin.code.style=official
android.nonTransitiveRClass=true
org.gradle.java.home=/Library/Java/JavaVirtualMachines/jdk-21.jdk/Contents/Home
```

If Java 21 is not at that path, detect it:
```bash
/usr/libexec/java_home -v 21
```

#### Gradle and AGP version alignment

The AGP version in `libs.versions.toml` dictates the minimum Gradle version:

| AGP version | Minimum Gradle version |
|---|---|
| 8.13.x | 8.13 |
| 8.12.x | 8.12 |
| 8.11.x | 8.11 |

Always ensure `gradle-wrapper.properties` `distributionUrl` matches or exceeds
the AGP requirement. If AGP is `8.13.2`, the wrapper MUST use Gradle `8.13+`.

#### Namespace collision prevention

The `shared` module and `app` module MUST have different `namespace` values.
Use `com.vectis.{appname}` for `app` and `com.vectis.{appname}.shared` for
the `shared` module. If they collide, the build emits a confusing warning.

Important Gradle configuration notes:

- The `shared` module uses Mozilla's `rust-android-gradle` plugin to
  cross-compile the Rust crate for all 4 Android ABIs.
- The `shared` module's `sourceSets` includes `../generated` as a Kotlin
  source directory for the codegen output.
- The `shared` module depends on JNA (AAR) for native library loading.
- The `app` module depends on `:shared` and on capability-specific libraries.
- Use Kotlin DSL (`.gradle.kts`) for all build files.

### 7. Generate `Core.kt`

Create `{project-dir}/app/src/main/java/com/vectis/{appname}/core/Core.kt`
following the pattern in `references/crux-android-shell-pattern.md`.

#### Simple Core (Render only, no DI)

When the only effect is `Render`, `Core` extends `androidx.lifecycle.ViewModel`
and uses Compose `mutableStateOf` directly:

```kotlin
import com.example.app.*         // generated bincode types
import uniffi.shared.CoreFfi     // generated UniFFI bridge

open class Core : androidx.lifecycle.ViewModel() {
    private var coreFfi: CoreFfi = CoreFfi()

    var view: ViewModel by mutableStateOf(
        ViewModel.bincodeDeserialize(coreFfi.view())
    )
        private set

    fun update(event: Event) {
        val effects = coreFfi.update(event.bincodeSerialize())
        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processRequest(request)
        }
    }

    private fun processRequest(request: Request) {
        when (val effect = request.effect) {
            is Effect.Render -> {
                this.view = ViewModel.bincodeDeserialize(coreFfi.view())
            }
        }
    }
}
```

#### Full Core (with side effects and DI)

When HTTP, SSE, or other async effects are present, `Core` is a plain class
injected via Koin, uses `CoroutineScope` with `StateFlow`:

```kotlin
import android.util.Log
import com.example.app.*         // generated bincode types
import uniffi.shared.CoreFfi     // generated UniFFI bridge
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

private const val TAG = "Core"

class Core(
    private val httpClient: HttpClient,
    // other clients...
) {
    private val coreFfi = CoreFfi()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

    private val _viewModel: MutableStateFlow<ViewModel> =
        MutableStateFlow(getViewModel())
    val viewModel: StateFlow<ViewModel> = _viewModel.asStateFlow()

    fun update(event: Event) {
        scope.launch {
            val effects = coreFfi.update(event.bincodeSerialize())
            handleEffects(effects)
        }
    }

    private suspend fun handleEffects(effects: ByteArray) {
        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) { processRequest(request) }
    }

    private suspend fun processRequest(request: Request) {
        when (val effect = request.effect) {
            is Effect.Http -> { /* delegate to httpClient */ }
            is Effect.Render -> { _viewModel.value = getViewModel() }
            is Effect.Time -> {
                // MUST launch in separate coroutine with try/catch
                scope.launch {
                    try {
                        handleTimeEffect(request.id, effect.value)
                    } catch (e: CancellationException) { throw e }
                    catch (e: Exception) { Log.e(TAG, "Time effect error", e) }
                }
            }
            is Effect.ServerSentEvents -> {
                // MUST launch in separate coroutine with try/catch
                scope.launch {
                    try {
                        sseClient.request(effect.value) { response ->
                            resolveAndHandleEffects(request.id, response.bincodeSerialize())
                        }
                    } catch (e: CancellationException) { throw e }
                    catch (e: Exception) { Log.e(TAG, "SSE error: ${e.message}", e) }
                }
            }
            // ...other effects
        }
    }
}
```

**CRITICAL**: All `scope.launch` blocks for async effects (SSE, Time) MUST
wrap their body in `try/catch` to prevent unhandled exceptions from crashing
the app. Always rethrow `CancellationException` to preserve coroutine
cancellation semantics.

Include only the effect handlers that the app actually uses.
See `references/crux-android-shell-pattern.md` for full implementations of
each effect handler.

### 8. Generate capability clients

For each non-Render effect, generate the corresponding client class in
`{project-dir}/app/src/main/java/com/vectis/{appname}/core/`:

| Effect | Client file | Implementation |
|---|---|---|
| `Http` | `HttpClient.kt` | Ktor + OkHttp engine |
| `ServerSentEvents` | `SseClient.kt` | Ktor streaming with channel |
| `KeyValue` | `KeyValueClient.kt` | Android SharedPreferences |
| `Time` | (inline in Core.kt) | `java.time.ZonedDateTime` |
| `Platform` | (inline in Core.kt) | `android.os.Build` |

See `references/crux-android-shell-pattern.md` for implementations.

### 9. Generate DI module and Application class

When using Koin (more than one non-Render effect), generate:

- `{project-dir}/app/src/main/java/com/vectis/{appname}/di/AppModule.kt`
- `{project-dir}/app/src/main/java/com/vectis/{appname}/{AppName}Application.kt`

And register the Application class in `AndroidManifest.xml`.

#### UniFFI library name override (CRITICAL)

The Application class `onCreate()` MUST set the JNA library override BEFORE
any UniFFI class is loaded. Without this, JNA looks for `libuniffi_shared.so`
but Cargo produces `libshared.so`, causing an `UnsatisfiedLinkError` crash on
launch:

```kotlin
class {AppName}Application : Application() {
    override fun onCreate() {
        super.onCreate()
        // MUST be first -- JNA loads libuniffi_shared.so by default,
        // but Cargo builds libshared.so
        System.setProperty("uniffi.component.shared.libraryOverride", "shared")

        startKoin {
            androidContext(this@{AppName}Application)
            modules(appModule)
        }
    }
}
```

The property name follows the pattern `uniffi.component.{crate_name}.libraryOverride`
where `{crate_name}` matches the `[lib] name = "shared"` in `Cargo.toml`.

### 10. Generate screen composables

For each ViewModel variant, create a screen composable file in
`{project-dir}/app/src/main/java/com/vectis/{appname}/ui/screens/`:

| ViewModel variant | Screen file | Content |
|---|---|---|
| `Loading` | `LoadingScreen.kt` | `CircularProgressIndicator` with "Loading..." text |
| `Main(MainView)` | `MainScreen.kt` | Layout driven by `MainView` fields |
| `Error(ErrorView)` | `ErrorScreen.kt` | Error message with optional retry |
| `{Name}({NameView})` | `{Name}Screen.kt` | Layout driven by `{NameView}` fields |

For each screen:

1. Import `androidx.compose.material3.*` and the generated types from
   `com.example.app.*`. Every screen file needs explicit imports for the types
   it uses (e.g., `import com.example.app.Event`, `import com.example.app.TodoListView`).
2. Accept the per-page view struct as a parameter.
3. Accept `onEvent: (Event) -> Unit` for user interactions.
4. Use Material 3 theme tokens for all colors, fonts, and spacing.
5. Map each shell-facing Event variant that is relevant to this view to a
   user interaction (button click, swipe, pull-to-refresh, etc.).
6. Add a `@Preview` with sample data at the bottom of the file.
7. Add `contentDescription` to interactive icons for accessibility.
8. If using Material Icons beyond the default set (e.g., `Icons.Filled.CheckCircle`,
   `Icons.Outlined.Circle`, `Icons.Default.Delete`), add the
   `material-icons-extended` dependency to `libs.versions.toml` and
   `app/build.gradle.kts`:
   ```toml
   # in libs.versions.toml [libraries]
   androidx-material-icons-extended = { group = "androidx.compose.material", name = "material-icons-extended" }
   ```
   ```kotlin
   // in app/build.gradle.kts
   implementation(libs.androidx.material.icons.extended)
   ```
9. When displaying `ULong` values from the generated types (e.g., count fields),
   cast to `Long` for Compose text: `"${viewModel.count.toLong()}"`.
   Do NOT pass `ULong` directly to string interpolation in `Text()` composables.

### 11. Generate `MainActivity.kt`

Create `{project-dir}/app/src/main/java/com/vectis/{appname}/MainActivity.kt`
following the pattern in `references/compose-view-patterns.md`.

The activity sets up the theme and renders the root composable. The root
composable observes the `ViewModel` and switches on its variants to display
the appropriate screen.

#### Simple pattern (ViewModel-based Core):

```kotlin
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            AppTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) { AppView() }
            }
        }
    }
}

@Composable
fun AppView(core: Core = viewModel()) {
    // switch on core.view to render screens
}
```

#### DI pattern (Koin-injected Core):

```kotlin
class MainActivity : ComponentActivity() {
    private val core by inject<Core>()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        // send initial events
        setContent {
            AppTheme {
                val state by core.viewModel.collectAsState()
                Surface(...) { AppView(state) { core.update(it) } }
            }
        }
    }
}
```

### 12. Generate theme files

Create Material 3 theme files in
`{project-dir}/app/src/main/java/com/vectis/{appname}/ui/theme/`:

- `Color.kt` -- color definitions
- `Theme.kt` -- `AppTheme` composable with dynamic color support
- `Type.kt` -- typography configuration

### 13. Generate `AndroidManifest.xml` and Android resources

#### AndroidManifest.xml

Create `{project-dir}/app/src/main/AndroidManifest.xml`. When the app uses
HTTP or SSE effects, include the `networkSecurityConfig` attribute:

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <uses-permission android:name="android.permission.INTERNET" />
    <application
        android:name=".{AppName}Application"
        android:allowBackup="true"
        android:label="{AppName}"
        android:supportsRtl="true"
        android:theme="@style/Theme.{AppName}"
        android:networkSecurityConfig="@xml/network_security_config">
        <activity
            android:name=".MainActivity"
            android:exported="true"
            android:theme="@style/Theme.{AppName}">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
```

Omit the `android:name` Application attribute if Koin is not used.
Omit the `networkSecurityConfig` if no HTTP/SSE effects.

#### themes.xml (REQUIRED)

Create `{project-dir}/app/src/main/res/values/themes.xml`. Without this file,
the build fails with `resource style/Theme.{AppName} not found`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <style name="Theme.{AppName}" parent="android:Theme.Material.Light.NoActionBar" />
</resources>
```

#### network_security_config.xml (if HTTP or SSE effects)

Create `{project-dir}/app/src/main/res/xml/network_security_config.xml`.
Android 9+ blocks cleartext HTTP by default. Without this file, the app
crashes with `CLEARTEXT communication not permitted` when connecting to
development servers:

```xml
<?xml version="1.0" encoding="utf-8"?>
<network-security-config>
    <domain-config cleartextTrafficPermitted="true">
        <domain includeSubdomains="true">localhost</domain>
        <domain includeSubdomains="true">10.0.2.2</domain>
        <domain includeSubdomains="true">10.0.3.2</domain>
        <domain includeSubdomains="true">127.0.0.1</domain>
    </domain-config>
</network-security-config>
```

The address `10.0.2.2` is the Android emulator's alias for the host machine's
`localhost`. Include it for development builds.

### 14. Build and verify

Pre-flight checks before first build:

1. Verify Gradle wrapper exists: `ls {project-dir}/gradlew`. If missing,
   generate it (see step 6).
2. Verify `local.properties` has `sdk.dir` set.
3. Verify `gradle.properties` has `org.gradle.java.home` pointing to Java 21.
4. Verify Rust Android targets are installed:
   `rustup target list --installed | grep android`

Build sequence:

1. Run `make build` in `{project-dir}` to generate types.
2. Run `./gradlew :shared:cargoBuild` to cross-compile the Rust library.
3. Run `./gradlew :app:assembleDebug` to build the APK.
4. If the build fails, read the error output, fix the issue, and re-run.

To run on the emulator from the command line:

```bash
emulator -list-avds
emulator -avd <avd_name> &
./gradlew :app:installDebug
adb shell am start -n com.vectis.{appname}/.MainActivity
```

Debugging crashes on the emulator:

```bash
adb logcat -b crash -d   # dump crash buffer
adb logcat | grep -i "fatal\|exception\|error"
```

## Process: Update Mode

Use this process when `{project-dir}/` already exists with Kotlin files.

### U1. Read and analyze the Crux core

Same as create mode step 1. Extract all types from the current `app.rs`.

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

Same as create mode step 14:

1. Run `make build` to regenerate types.
2. Run `./gradlew :app:assembleDebug` to verify compilation.
3. Fix any build errors.

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
| `references/android-project-config.md` | Gradle files, Makefile, build configuration |

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
| `./gradlew: No such file or directory` | Generate the Gradle wrapper -- see step 6 |
| `Minimum supported Gradle version is X.Y` | Update `gradle-wrapper.properties` to match AGP requirement -- see step 6 |
| `java.lang.IllegalArgumentException: 25.0.1` (or similar Java version parse error) | Set `org.gradle.java.home` to Java 21 in `gradle.properties` |
| `resource style/Theme.{AppName} not found` | Create `res/values/themes.xml` -- see step 13 |
| `Unresolved reference 'Event'` (or `ViewModel`, `Effect`, etc.) | Add `import com.example.app.*` imports to the affected Kotlin file |
| `Unresolved reference 'CoreFfi'` | Add `import uniffi.shared.CoreFfi` to `Core.kt` |
| `Unresolved reference 'Icons'` | Add `material-icons-extended` dependency -- see step 10 |
| `Namespace 'X' is used in multiple modules` | Use `com.vectis.{appname}.shared` namespace for the shared module |
| `unresolved module path shared::ffi` (codegen error) | UniFFI version mismatch -- ensure `uniffi = "=0.29.4"` in shared `Cargo.toml` |
| `This declaration needs opt-in` (unsigned types) | Add `@OptIn(ExperimentalUnsignedTypes::class)` to the class |

### Runtime crashes

| Crash | Resolution |
|---|---|
| `UnsatisfiedLinkError: Unable to load library 'uniffi_shared'` | Add `System.setProperty("uniffi.component.shared.libraryOverride", "shared")` to Application `onCreate()` -- see step 9 |
| `CLEARTEXT communication not permitted` | Create `network_security_config.xml` and reference in AndroidManifest -- see step 13 |
| Unhandled exception in SSE/Time coroutine | Wrap `scope.launch` blocks for async effects in `try/catch` -- see step 7 |

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

### Quality

- [ ] Every screen composable has a `@Preview` with sample data
- [ ] Interactive icons have `contentDescription` for accessibility
- [ ] No force unwraps or `!!` in production code
- [ ] HTTP client has proper timeout configuration
- [ ] Coroutine scopes use `SupervisorJob` for fault isolation
- [ ] Async effects (SSE, Time) wrapped in `try/catch` inside `scope.launch`
- [ ] `CancellationException` is always rethrown in catch blocks

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
  plain class with `StateFlow` injected via Koin.
- **Gradle wrapper is required**: The `gradlew` script must be generated as
  part of the shell creation. Without it, no `./gradlew` command works.
  See step 6 for generation instructions.
- **Java 21 LTS required**: Java 25+ has a version string that Gradle's
  Kotlin compiler cannot parse. Always pin `org.gradle.java.home` to Java 21
  in `gradle.properties`.
- **Network security config**: Android 9+ blocks cleartext HTTP traffic by
  default. Apps with HTTP or SSE effects MUST include a
  `network_security_config.xml` to allow cleartext to localhost/`10.0.2.2`
  for development. Without it, the app crashes on first network request.
- **Defensive error handling in coroutines**: All async effect handlers
  (SSE, Time) that run in `scope.launch` blocks MUST wrap their bodies in
  `try/catch` to prevent unhandled exceptions from crashing the app. Always
  rethrow `CancellationException`.
- **themes.xml is mandatory**: `AndroidManifest.xml` references a theme
  resource. The `res/values/themes.xml` file MUST exist or the build fails
  with `resource style/Theme.{AppName} not found`.
- **No Android Studio required for builds**: The Gradle wrapper (`./gradlew`)
  handles compilation. The emulator can be launched from the command line.
  Android Studio is only needed for initial SDK/NDK installation or for the
  visual layout editor.
