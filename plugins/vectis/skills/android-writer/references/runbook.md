# Android Writer Runbook

Operational detail for `vectis-android-writer`. The SKILL.md keeps only the orientation surface (Critical Path + Reference table + Guardrails); everything procedural lives here.

## Prerequisites

`{app-dir}` must contain `shared/src/app.rs`. `{project-dir}` defaults to `{app-dir}/Android`. When `{slice-dir}` is supplied, the writer reads the `## Android Shell Requirements` section from `{slice-dir}/specs/{feature-name}/spec.md` for platform-specific requirements.

Tools: Android SDK (cmd-line tools, platform-tools, emulator); Android NDK (`sdkmanager "ndk;29.0.14206865"`); Rust Android targets (`rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`); Python 3 (Mozilla rust-android-gradle plugin); Java 21 LTS JDK (NOT Java 25+ — Gradle's embedded Kotlin compiler cannot parse Java 25+ version strings).

Environment:

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
export PATH="$ANDROID_HOME/platform-tools:$PATH"
export PATH="$ANDROID_HOME/emulator:$PATH"
```

## Input Analysis

Read `{app-dir}/shared/src/app.rs` and extract:

| Extract | Source | Maps to |
|---|---|---|
| App struct name | `impl App for X` | App name, package name |
| ViewModel variants | `enum ViewModel` | `when` branches in the main composable |
| Per-page view structs | Structs wrapped by ViewModel variants | Screen composable properties and layout |
| Shell-facing Event variants | `enum Event` (non-`#[serde(skip)]`, non-`#[facet(skip)]`) | User interaction handlers in screen composables |
| Effect variants | `enum Effect` | `processRequest` `when` branches in Core.kt |
| Route variants | `enum Route` | Navigation destinations |
| Supporting types | Structs/enums used in view structs | Display data types |
| Screen regions | `composition.yaml` `header`, `body`, `footer`, `fab` | View structure (TopAppBar, Scaffold content, BottomAppBar, FloatingActionButton) |
| Container structure | `composition.yaml` `group` nodes with `direction`, `gap`, `align`, `justify` | `Row`/`Column`/`Box` with `Arrangement` and `Alignment` |
| Sizing | `composition.yaml` `size` on groups and items (`fill`, `hug`, fixed) | `Modifier.fillMaxWidth()`, intrinsic sizing, explicit dimensions |
| Surface decoration | `composition.yaml` `background`, `corner_radius`, `elevation` on groups | `Card`/`Surface` with background, shape, elevation |
| Field bindings | `composition.yaml` `bind` keys on items | Property bindings in composables |
| Event wiring | `composition.yaml` `event` keys on items | `onEvent()` interaction handlers |
| Token references | `composition.yaml` `style`, `color`, `gap`, `padding` | `MaterialTheme.typography.*` / `MaterialTheme.colorScheme.*` (M3 slots filled by the shell-local `VectisTheme`) plus `VectisSpacing.*` / `VectisCornerRadius.*` (under `Android/app/src/main/java/com/vectis/<appname>/ui/theme/`, generated from `tokens.yaml`) |
| Component directive | `composition.yaml` `group.component: <slug>` | One named `@Composable` per slug under `ui/components/`, PascalCased (`task-row` → `TaskRow`) |
| Asset references | `composition.yaml` `image:` / `icon:` / `icon-button:` / `fab:` resolved via `assets.yaml` | `painterResource(id = R.drawable.<asset-id>)` for raster / vector copied into `Android/app/src/main/res/drawable*/`; `Icons.Default.<glyph>` for `kind: symbol` entries |
| Conditional rendering | `composition.yaml` `states` and `*-when` keys | `if`/`when` in composable code |
| Iteration | `composition.yaml` `list.each` / `grid.each` + `item` keys | `LazyColumn items` / `LazyVerticalGrid` |

Also read: `{app-dir}/shared/src/lib.rs` (custom capability modules), `Cargo.toml` (capability deps), `shared/src/ffi.rs` (CoreFFI struct), `shared/src/bin/codegen.rs` (codegen binary), `tokens.yaml` (resolution: change-local then project-level; absent → Material 3 fallback), `assets.yaml` plus referenced files (resolution: change-local then project-level; copied into `res/drawable*/` per [`design-system-integration.md`](design-system-integration.md)).

When `slice-dir` is provided also read `{slice-dir}/specs/{feature-name}/spec.md` `## Android Shell Requirements` and `{slice-dir}/design.md` `## Android Shell Details`, plus `{slice-dir}/composition.yaml` (cross-artifact reference checks pre-enforced by `specify tool run vectis -- validate composition`).

## Generated Type Conventions

The codegen binary produces bincode types in `generated/com/example/app/` and UniFFI bindings in `generated/uniffi/shared/`. Hand-written code lives in `com.vectis.{appname}` and **must** import generated types explicitly. See [`generated-type-conventions.md`](generated-type-conventions.md) for imports, enum vs sealed interface naming, numeric type mapping (`usize` → `ULong` etc.), KeyValue / Time types and required `@OptIn(ExperimentalUnsignedTypes::class)` annotations.

## Mode Detection

- **Create Mode** — `{project-dir}/` does **not** exist. Invoke `specify tool run vectis -- scaffold android` to render the baseline, run host post-processing and checks, then proceed directly into Update Mode.
- **Update Mode** — `{project-dir}/` exists with `.kt` files. Read existing code, diff against the core, apply targeted edits.

Detection rule: check for `{project-dir}/app/src/main/java/*/Core.kt`. If present → Update Mode. If not:

```bash
cd {app-dir}
specify tool run vectis -- scaffold android {AppName} [--caps {caps}] [--android-package com.example.app]
```

`{app-dir}` is the parent of `shared/`; `{project-dir}` is normally `{app-dir}/Android`. `--android-package` defaults to `com.vectis.<appname-lowercase>`. On scaffold failure surface the structured output and stop — do **not** hand-author Gradle, manifest, or baseline `.kt` files.

The scaffolded shell is a render-only baseline with CAP markers in `Core.kt`, `AndroidManifest.xml`, `libs.versions.toml`, and `app/build.gradle.kts` (one block per optional capability: `http`, `kv`, `sse`, `time`, `platform`) plus a starter `HomeScreen.kt`. Update Mode expands CAP blocks for capabilities the core uses, strips CAP blocks for those it does not, and replaces the starter screen with real per-ViewModel-variant screens derived from the current `app.rs`.

## Verification ownership

When the orchestrator passes `skip_verification: true`, the writer stops after code generation and does **not** run U8. The orchestrator's dedicated Android verify sub-agent handles pre-flight checks, `make build`, `./gradlew :shared:cargoBuild`, and `./gradlew :app:assembleDebug` with its own repair loop and iteration limits. When invoked **standalone** (no flag, or `false`), the writer runs its full process including U8.

## Process: Create Mode

Use when no Android shell exists at `{project-dir}`. Scaffold tool owns render-only Android boilerplate (Gradle build files, version catalog, `AndroidManifest.xml` with conditional `networkSecurityConfig`, render-only baseline `Core.kt` with CAP markers, `{AppName}Application.kt` with the UniFFI library override, `MainActivity.kt`, `ui/screens/HomeScreen.kt`, `res/xml/network_security_config.xml` when HTTP/SSE is selected). Host post-processing owns Gradle wrapper bootstrap, `local.properties`, SDK/NDK detection, and the Java 21 pin.

1. **Read the Crux core.** Read `{app-dir}/shared/src/app.rs` and extract the inventory above. Derive the App struct name (used by the scaffold to name the Gradle project, Application class, package folder, theme) and which capabilities the core actually uses. If `app.rs` cannot be read or parsed, report and stop.
2. **Invoke the scaffold tool.** Run the command in Mode Detection above. The tool generates the render-only Gradle project (root + `app` + `shared` modules) and a baseline shell in `Android/app/src/main/java/<package-path>/` with CAP markers for every optional capability. On non-zero exit, surface tool output and stop.
3. **Switch to Update Mode.** After scaffold and host checks return green, treat the scaffolded Android shell as an existing implementation: expand CAP blocks in `Core.kt` (real effect handlers + client classes) for capabilities the core uses, strip CAP blocks for those it does not, expand matching CAP blocks in `AndroidManifest.xml` (`INTERNET`, `networkSecurityConfig`), `app/build.gradle.kts` (Ktor / Koin), and `libs.versions.toml`, replace the `HomeScreen` starter with real per-ViewModel-variant screen files, rewrite the root composable in `MainActivity.kt`, and apply any `## Android Shell Requirements` from the active slice.

Dependency version pins (Kotlin, AGP, Ktor, Koin, Compose BOM, etc.) come from the active Vectis version pins used by `vectis (scaffold)`; route pin changes through the Vectis version/template workflow rather than hand-editing `libs.versions.toml` / `shared/build.gradle.kts`.

## Process: Update Mode

Use when `{project-dir}/` already exists with Kotlin files.

**U1. Read and analyze the Crux core.** Same as create-mode step 1; also read `## Android Shell Requirements` and `## Android Shell Details` when `slice-dir` is supplied.

**U2. Read existing Kotlin code.** Read `core/Core.kt` (effect handler `when`), `ui/screens/*.kt` (current screens), `MainActivity.kt` (root composable + ViewModel switch), `di/AppModule.kt` (DI configuration if present).

**U3. Build implementation inventory.** Extract from existing Kotlin:

| Category | What to extract |
|---|---|
| Effect handlers | Cases in `processRequest` `when` |
| ViewModel cases | Branches in root composable `when` |
| Screen composables | `.kt` files in `ui/screens/` |
| Components / Theme / Drawables | `.kt` files in `ui/components/` (one per `component: <slug>`); `.kt` files in `ui/theme/` (one per `tokens.yaml` category, plus `Theme.kt`); per-density `app/src/main/res/drawable*/` (one set per `kind: raster` / `kind: vector`) |
| Event dispatches | `onEvent(...)` / `core.update(...)` |
| Capability clients | Client classes in `core/` |
| DI modules | Koin module definitions |
| Design system usage | `MaterialTheme.colorScheme` / `MaterialTheme.typography` / `VectisSpacing` / `VectisCornerRadius` / `VectisElevation` references |

**U4. Diff analysis.** Compare Rust core types (U1) and input artifacts (`tokens.yaml`, `assets.yaml`, `composition.yaml`) against the Kotlin inventory (U3). Classify Added / Removed / Modified / Unchanged. Walk in order: Effect variants → ViewModel variants → per-page view fields → Event variants → Route variants → Token categories / asset entries / component directives. Output the diff before editing.

**U5. Apply changes to Core.kt.** Add/remove effect handler cases, capability client classes, DI module entries.

**U6. Apply changes to composables.** Add/remove screen files for added/removed ViewModel variants; update root composable `when`; update screens for changed view fields; add/remove event dispatch calls; verify scrollable containers do not contain fill-max-size children (see [`compose-view-patterns.md`](compose-view-patterns.md) Layout Constraint Rules); ensure screen / component files include `import com.vectis.<appname>.ui.theme.*` (Kotlin auto-imports only within the exact same package).

**U6a-c. Refresh shell-local design system inputs** when `tokens.yaml` / `assets.yaml` / `composition.yaml` change. Rules in [`design-system-integration.md`](design-system-integration.md) and [`kotlin-token-templates.md`](kotlin-token-templates.md):

- **U6a — `ui/theme/` from `tokens.yaml`**: one Kotlin file per category (`spacing` / `cornerRadius` colocated in `Spacing.kt`) plus structural `Theme.kt` defining `VectisTheme`. Generated files carry the `// Generated from design-system/tokens.yaml — do not edit manually.` header (except `Theme.kt`); operator-authored files (no header) are preserved on category removal. The header always uses the canonical project-level path regardless of which file generation actually read. Absent `tokens.yaml` → emit the Material 3 fallback `Theme.kt`.
- **U6b — `ui/components/` from `composition.yaml`**: one named `@Composable` per `group.component: <slug>` under `ui/components/<PascalCaseSlug>.kt`. Props inferred from variation; constants baked in. Structural identity pre-enforced by `specify tool run vectis -- validate composition`. Removed slugs delete files and inline call sites.
- **U6c — `res/drawable*/` from `assets.yaml`**: copy per-density `kind: raster` files into `res/drawable-<density>/<asset-id>.<ext>`, vector drawables into `res/drawable/<asset-id>.xml`; translate kebab-case ids to lowercase-with-underscores. `kind: symbol` skips the resource step (emit `Icons.Default.<glyph>` at the call site). Canonical `source:` SVG is provenance only — never copied. Missing Android exports for referenced vector entries halt generation; the writer never falls back to the canonical SVG, generates a placeholder, or skips the screen.

**U7. Update build configuration.** Update `build.gradle.kts` for new dependencies; `libs.versions.toml` for new library versions; `AndroidManifest.xml` for permission changes (e.g., INTERNET for HTTP).

**U8. Build and verify.** Host pre-flight: write `local.properties` from `$ANDROID_HOME` (`printf 'sdk.dir=%s\n' "$ANDROID_HOME" > local.properties`); ensure NDK exists (`sdkmanager "ndk;29.0.14206865"` if absent); ensure Rust Android targets (`rustup target add ...` if absent); set `org.gradle.java.home` to Java 21 when missing; bootstrap `{project-dir}/gradlew` with `gradle wrapper` when absent. Then: `make build` (regenerate types) → `./gradlew :shared:cargoBuild` (cross-compile Rust) → `./gradlew :app:assembleDebug` (build APK). Fix any build errors.

## Composition Mapping Priority

When `composition.yaml` is present, the region structure and group container tree take precedence over convention-based inference for composable body composition:

- **Groups** → Compose containers: `direction: row` → `Row(horizontalArrangement:)`, `direction: column` → `Column(verticalArrangement:)`, `direction: stack` → `Box`.
- **Sizing** → `Modifier`: `fill` → `Modifier.fillMaxWidth()`, fixed values → `Modifier.width()` / `Modifier.height()`.
- **Surface decoration** → card-like containers: `background` + `corner_radius` → `Card` or `Surface` with shape and color, `elevation` → `Modifier.shadow()` or `Card(elevation:)`.
- **Platform-specific overrides**: when `composition.yaml` contains `platforms.android` region overrides for a screen, use those over the shared regions.

When `composition.yaml` is absent, fall back to convention-based inference.

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

In Update Mode, minimize collateral changes — never regenerate from scratch; preserve developer-added styling, custom composable logic (animations, gestures), `@Preview` blocks, Gradle customizations, and Makefile customizations. See [`../rules.md`](../rules.md) for the full preservation contract and the platform-level Important Notes.

## Examples

| Example | Capabilities | Demonstrates |
|---|---|---|
| `examples/01-simple-counter-android.md` | Render | Minimal shell, Core.kt, two screens, project setup |
| `examples/02-http-counter-android.md` | Render + HTTP + SSE | Async HTTP handling, Koin DI, SSE streaming, Ktor |

## Error Handling

### Build errors

| Error | Resolution |
|---|---|
| `app.rs` not found | Verify `app-dir` points to a Crux app with `shared/src/app.rs` |
| Unknown Effect variant | Add a placeholder `is Effect.XXX -> { }` and report |
| Gradle sync fails | Check `build.gradle.kts` syntax; verify NDK version matches installed |
| Build fails with missing types | Run `make build` to regenerate types; verify `uniffi` matches the active Vectis version pins, then rerun host verification |
| `cargoBuild` fails with `target may not be installed` | Run `rustup target add armv7-linux-androideabi aarch64-linux-android i686-linux-android x86_64-linux-android` |
| NDK not found | Install via `sdkmanager "ndk;29.0.14206865"` or Android Studio SDK Manager |
| Python 3 not found | Required by rust-android-gradle; install via system package manager |
| `./gradlew: No such file or directory` | Run host wrapper bootstrap from `{project-dir}` (`gradle wrapper`) and record the step result |
| `Minimum supported Gradle version is X.Y` | Gradle/AGP drift — route the pin change through the Vectis version/template workflow, rerender, then rerun host verification |
| `java.lang.IllegalArgumentException: 25.0.1` (or similar Java version parse error) | Set `org.gradle.java.home` to Java 21 in `gradle.properties` (`/usr/libexec/java_home -v 21` on macOS when available) |
| `resource style/Theme.{AppName} not found` | Scaffold output is missing `res/values/themes.xml` — rerender and stop if still absent |
| `Unresolved reference 'Event'` (or `ViewModel`, `Effect`, etc.) | Add `import com.example.app.*` imports to the affected file |
| `Unresolved reference 'CoreFfi'` | Add `import uniffi.shared.CoreFfi` to `Core.kt` |
| `Unresolved reference 'Icons'` | Add `material-icons-extended` dependency to `libs.versions.toml` + `app/build.gradle.kts` |
| `Namespace 'X' is used in multiple modules` | Use `com.vectis.{appname}.shared` namespace for the shared module |
| `unresolved module path shared::ffi` (codegen error) | UniFFI version mismatch — rerun `make build`, `./gradlew :shared:cargoBuild`, `./gradlew :app:assembleDebug` and inspect the active Vectis version pins |
| `This declaration needs opt-in` (unsigned types) | Add `@OptIn(ExperimentalUnsignedTypes::class)` to the class |
| `validate composition` reports unresolved token / asset reference | Operator must add the missing entry or remove the reference; writer halts |
| Missing Android export for a `kind: vector` asset | Validation error, not a deferred TODO. Halt and report missing `sources.android` (Vector Drawable XML — SVG sources need converting first) |

### Runtime crashes

| Crash | Resolution |
|---|---|
| `UnsatisfiedLinkError: Unable to load library 'uniffi_shared'` | Scaffold-generated `{AppName}Application.kt` already calls `System.setProperty("uniffi.component.shared.libraryOverride", "shared")` first in `onCreate()`; verify the Application class was not replaced or its body reordered |
| `CLEARTEXT communication not permitted` | Ensure scaffold rendered with HTTP or SSE so it emitted `res/xml/network_security_config.xml` and the matching `networkSecurityConfig` attribute |
| Unhandled exception in SSE/Time coroutine | Wrap `scope.launch` blocks in `try/catch`, rethrow `CancellationException`, and resolve the effect with a fallback (`SseResponse.Done`, `TimeResponse.DurationElapsed`, etc.) so the Rust core is never left awaiting an unresolved ID |
| `IllegalStateException: Vertically scrollable component was measured with an infinity maximum height constraints` | A composable that needs bounded constraints (e.g., `SearchBar` in expanded mode, `BottomSheet` content, any `fillMaxSize()` child) is inside a `verticalScroll` container. Move outside, use `LazyColumn` with bounded items, or use `DockedSearchBar`. See [`compose-view-patterns.md`](compose-view-patterns.md) Layout Constraint Rules. |

## Verification Checklist

**Build**: `gradlew` exists; `local.properties` has `sdk.dir`; `gradle.properties` has `org.gradle.java.home` → Java 21; `make build` completes; `./gradlew :shared:cargoBuild` compiles all 4 ABIs; `./gradlew :app:assembleDebug` builds the APK; APK installs and launches without crashing.

**Structure**: every ViewModel variant has a screen file + a branch in the root composable `when`; every Effect variant has a branch in `processRequest`; every shell-facing Event variant is dispatched by at least one composable; `Core.kt` handles all effects; generated types directory in `.gitignore`; `res/values/themes.xml` exists; `res/xml/network_security_config.xml` exists if HTTP/SSE; manifest references the network security config when applicable; app module namespace differs from shared module namespace.

**Imports and Types**: hand-written `.kt` files import `com.example.app.*`; `Core.kt` imports `uniffi.shared.CoreFfi`; Application class calls `System.setProperty("uniffi.component.shared.libraryOverride", "shared")` first in `onCreate()`; simple enum comparisons use `==` (e.g., `Filter.ALL`); `ULong` cast to `Long` for Compose text; `toUByteArray()` users carry `@OptIn(ExperimentalUnsignedTypes::class)`.

**Design System**: `tokens.yaml` present → `ui/theme/` has one file per token category plus `Theme.kt` defining `VectisTheme` over a static token-derived `ColorScheme` (no `dynamicLight/DarkColorScheme`); absent → only `Theme.kt`, wrapping `MaterialTheme` with M3 dynamic / static defaults per [`design-system-integration.md`](design-system-integration.md). Screens/components reference `MaterialTheme.colorScheme.*` / `MaterialTheme.typography.*` / `VectisSpacing.*` / `VectisCornerRadius.*` — no hardcoded `Color(0xFF…)` or magic `dp` / `sp` outside generated theme files. No `import com.vectis.design.*`, `include(":vectis-design")`, `implementation(project(":vectis-design"))`, or `path: ../../design-system/android` references anywhere.

**Assets**: every `composition.yaml` asset id resolves in `assets.yaml`; raster entries have matching `res/drawable-<density>/<asset-id>.<ext>`; vector entries have `res/drawable/<asset-id>.xml`; `kind: symbol` entries render via `Icons.Default.<glyph>` (no resource copy); no `path: ../../design-system/assets` references.

**Components**: every `component: <slug>` has `ui/components/<PascalCaseSlug>.kt`; every call site uses the named `@Composable`; removed slugs delete their files.

**Quality**: every screen composable has `@Preview` with sample data; interactive icons have `contentDescription`; no `!!` in production code; CoreFFI calls use `try/catch` with `Log.e` including `${e.message}`; bincode calls use `try/catch` with `Log.w` and safe fallback; `Effect.Render` handler preserves existing view on failure (returns without assignment); `initialView()` is the only fallback to `ViewModel.Loading`; HTTP client has timeout configuration; coroutine scopes use `SupervisorJob`; async effects (SSE, Time) wrapped in `try/catch` inside `scope.launch`; `CancellationException` always rethrown; Time `NotifyAfter`/`NotifyAt` jobs tracked in `timerJobs` map with `Clear` cancelling stored job; no fill-max-size components inside unbounded scrollable containers; Application class installs a global crash recovery handler (see [`crux-android-shell-pattern.md`](crux-android-shell-pattern.md) Crash Recovery Handler).

**Command-Line Workflow**: build works from terminal (`./gradlew :app:assembleDebug`); emulator can be launched (`emulator -avd <name>`); app installs (`./gradlew :app:installDebug`) and launches (`adb shell am start -n <package>/.MainActivity`).

## Scaffold ownership

Gradle build files, the version catalog, the Makefile, `AndroidManifest.xml`, CAP-marker scaffolding for the baseline `Core.kt` / `app/build.gradle.kts` / `libs.versions.toml`, and the starter Kotlin layout are owned by the Vectis scaffold templates in [`augentic/specify-cli`](https://github.com/augentic/specify-cli) (`<specify-cli>/crates/vectis/` and the Vectis template sources). Do not hand-edit those files in Create Mode; let `specify tool run vectis -- scaffold android` write them and modify in Update Mode. Host-derived files and settings (`local.properties`, Gradle wrapper artefacts, Java 21 pinning, SDK/NDK checks) are handled by the writer/verify workflow after render.
