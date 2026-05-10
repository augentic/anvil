---
name: vectis-android-writer
description: Generate or update a Kotlin/Jetpack Compose Android shell for a Crux application. Use when the user wants to create an Android shell, scaffold Android UI, or generate Compose views for a Crux app, or mentions android-writer.
argument-hint: "<slice-dir>"
---

# Crux Android Shell Generator
> **Vectis deterministic tooling runs through declared Specify tools.** Shell scaffolding is `specify tool run vectis -- scaffold android ...`; artifact validation is `specify tool run vectis -- validate ...`. The scaffold is render-only: Android SDK/NDK detection, `local.properties`, Java 21 pinning, Gradle wrapper bootstrap, and Gradle builds remain host-owned steps with explicit verification evidence.

## Critical Path (Quick Reference)

1. Read `{app-dir}/shared/src/app.rs` (and optional `slice-dir` shell-requirements + `composition.yaml`); extract App name, ViewModel/Effect/Event/Route variants and the capability set.
2. Detect mode by checking `{project-dir}/app/src/main/java/*/Core.kt`: missing → run `specify tool run vectis -- scaffold android ...` plus Android host post-processing, then enter Update Mode; present → start Update Mode immediately.
3. Build an implementation inventory of existing Kotlin code (effect handlers, ViewModel cases, screen composables, event dispatches, capability clients, DI modules).
4. Diff Rust core types vs Kotlin inventory by category (Effect → ViewModel → view-fields → Event → Route) and emit a summary edit plan.
5. Apply changes: expand or strip CAP blocks in `Core.kt` + `AndroidManifest.xml` + Gradle, add/remove screen composables for each ViewModel variant, update the root `when`, dispatch new Events.
6. Update build configuration (`libs.versions.toml`, `build.gradle.kts`, manifest permissions, `network_security_config.xml`) to match the changed capability set.
7. Run Android host checks (`local.properties` / Java 21 / NDK / wrapper), `make build`, `./gradlew :shared:cargoBuild`, and `./gradlew :app:assembleDebug` (skipped when the orchestrator passes `skip_verification: true`).

Generate or update a buildable Kotlin/Jetpack Compose Android shell for an existing Crux core application. The shell renders the core's `ViewModel`, dispatches `Event` values from user interactions, and handles platform side-effects (HTTP, KV, SSE, Time, Platform) on behalf of the core.

The Android writer reads `tokens.yaml`, `assets.yaml`, and `composition.yaml` directly and emits **shell-local** theme + asset resources inside the Android shell tree. There is no separate `:vectis-design` Gradle module, no `implementation(project(":vectis-design"))` dependency, and no path back into `design-system/android/` from the rendered shell project. When `tokens.yaml` is absent, the writer falls back to platform-native Material 3 defaults — fallback policy belongs to the shell writer, not to the design-system manifest. See [`references/design-system-integration.md`](references/design-system-integration.md) for the full integration contract and [`references/kotlin-token-templates.md`](references/kotlin-token-templates.md) for the per-category Kotlin code templates.

When an existing Android shell is detected, the skill operates in **update mode**: it compares the current `app.rs` types against the existing Kotlin code and makes targeted edits rather than regenerating from scratch.

When no Android shell exists yet, the skill runs `specify tool run vectis -- scaffold android {AppName} [--caps {caps}] [--android-package <package>]` from the Crux project root to render the project. The scaffold tool owns render-only build and shell files: `Android/Makefile`, the `build.gradle.kts` / `settings.gradle.kts` / `gradle.properties` triad, the `gradle/libs.versions.toml` version catalog, `app/build.gradle.kts` and `shared/build.gradle.kts`, `AndroidManifest.xml` (including conditional `networkSecurityConfig` when HTTP/SSE is selected), the `{AppName}Application.kt` entry point with the UniFFI library override, a render-only baseline `Core.kt` with CAP markers, `MainActivity.kt`, the `ui/screens/HomeScreen.kt` starter, and `res/xml/network_security_config.xml` when needed. Host post-processing owns `local.properties`, Java 21 pinning, NDK checks, and Gradle wrapper bootstrap. The writer adds `ui/components/`, `ui/theme/`, and per-density `res/drawable*/` resources on first generation when the corresponding inputs exist. Once the scaffold exists and host checks pass, this skill switches to **update mode** and layers spec-driven changes over the generated baseline.

This skill targets **Kotlin 2.x**, **Jetpack Compose** with Material 3, and minimum SDK 34.

## Arguments

| Argument | Required | Description |
|---|---|---|
| `app-dir` | **Yes** | Path to the Crux app directory (must contain `shared/src/app.rs`) |
| `project-dir` | No | Directory for the Android shell. Defaults to `{app-dir}/Android` |
| `slice-dir` | No | Path to `.specify/slices/<change>/`. When provided, the skill reads the `## Android Shell Requirements` section from `{slice-dir}/specs/{feature-name}/spec.md` for platform-specific requirements |

## Prerequisites

The following tools must be installed:

- Android SDK (command-line tools, platform-tools, emulator)
- Android NDK (install via `sdkmanager "ndk;29.0.14206865"` or through SDK Manager)
- Rust Android targets: `rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`
- Python 3 (required by Mozilla's rust-android-gradle plugin)
- Java 21 LTS JDK (NOT Java 25+ -- Gradle's embedded Kotlin compiler cannot parse Java 25+ version strings, causing a cryptic `IllegalArgumentException` at build time)

Environment variables must be set:

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
export PATH="$ANDROID_HOME/platform-tools:$PATH"
export PATH="$ANDROID_HOME/emulator:$PATH"
```

## Input Analysis

The android-writer reads the Crux core source to determine what the shell must render and handle. Read `{app-dir}/shared/src/app.rs` and extract:

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
| Surface decoration | `composition.yaml` `background`, `corner_radius`, `elevation` on groups | `Card`/`Surface` with background, shape, and elevation |
| Field bindings | `composition.yaml` `bind` keys on items | Property bindings in composables |
| Event wiring | `composition.yaml` `event` keys on items | `onEvent()` interaction handlers |
| Token references | `composition.yaml` `style`, `color`, `gap`, `padding` | `MaterialTheme.typography.*` / `MaterialTheme.colorScheme.*` (M3 slots filled by the shell-local `VectisTheme`) plus `VectisSpacing.*` / `VectisCornerRadius.*` (defined under `Android/app/src/main/java/com/vectis/<appname>/ui/theme/`, generated from `tokens.yaml`) |
| Component directive | `composition.yaml` `group.component: <slug>` | One named `@Composable` per slug under `Android/app/src/main/java/com/vectis/<appname>/ui/components/`, PascalCased (`task-row` → `TaskRow`) |
| Asset references | `composition.yaml` `image:` / `icon:` / `icon-button:` / `fab:` resolved through `assets.yaml` | `painterResource(id = R.drawable.<asset-id>)` for raster / vector copied into `Android/app/src/main/res/drawable*/`; `Icons.Default.<glyph>` for `kind: symbol` entries |
| Conditional rendering | `composition.yaml` `states` and `*-when` keys | `if`/`when` in composable code |
| Iteration | `composition.yaml` `list.each` / `grid.each` + `item` keys | `LazyColumn items` / `LazyVerticalGrid` |

Also read:
- `{app-dir}/shared/src/lib.rs` -- custom capability modules
- `{app-dir}/shared/Cargo.toml` -- capability dependencies
- `{app-dir}/shared/src/ffi.rs` -- CoreFFI struct definition
- `{app-dir}/shared/src/bin/codegen.rs` -- codegen binary for type generation
- `tokens.yaml` -- design tokens. Resolution: change-local `{change-dir}/tokens.yaml` then project-level `design-system/tokens.yaml`. Absent → Material 3 fallback (see [`references/design-system-integration.md`](references/design-system-integration.md)).
- `assets.yaml` plus referenced files -- asset inventory for `image` / `icon` / `icon-button` / `fab` references in `composition.yaml`. Resolution: change-local `{change-dir}/assets.yaml` + `{change-dir}/assets/` then `design-system/assets.yaml` + `design-system/assets/`. Writer copies into `Android/app/src/main/res/drawable*/` per copy-on-generate (see [`references/design-system-integration.md`](references/design-system-integration.md)).

When `slice-dir` is provided, also read:
- `{slice-dir}/specs/{feature-name}/spec.md` -- read the `## Android Shell Requirements` section for platform-specific behavioral requirements (navigation style, gestures, haptics, accessibility). Also read the `## Android Shell Details` section of `{slice-dir}/design.md` for platform design decisions.
- `{slice-dir}/composition.yaml` or `.specify/specs/composition.yaml` -- the wired composition artifact. Cross-artifact reference checks (token / asset / `bind` / `event`) are pre-enforced by `specify tool run vectis -- validate composition`; the writer consumes the validated input.

## Generated Type Conventions (CRITICAL)

The codegen binary produces bincode types in `generated/com/example/app/` and UniFFI bindings in `generated/uniffi/shared/`. Hand-written code lives in `com.vectis.{appname}` and **must** import generated types explicitly. See [`references/generated-type-conventions.md`](references/generated-type-conventions.md) for the full rules covering imports, enum vs sealed interface naming, numeric type mapping (`usize` → `ULong` etc.), KeyValue types, Time types and required `@OptIn(ExperimentalUnsignedTypes::class)` annotations.

## Mode Detection

- **Create Mode** -- `{project-dir}/` does **not** exist. The skill invokes `specify tool run vectis -- scaffold android` to render the baseline, runs Android host post-processing and checks, then proceeds directly into Update Mode to apply feature-specific changes from the Specify artifacts.
- **Update Mode** -- `{project-dir}/` **does** exist and contains `.kt` files. Read existing code, diff against the core, and make targeted edits (steps U1--U8 below).

Detection rule: check for `{project-dir}/app/src/main/java/*/Core.kt`. If present, switch to update mode. If not, run:

```bash
cd {app-dir}
specify tool run vectis -- scaffold android {AppName} [--caps {caps}] [--android-package com.example.app]
```

`{app-dir}` is the parent directory of `shared/`; `{project-dir}` is normally `{app-dir}/Android`. `--android-package` is optional and defaults to `com.vectis.<appname-lowercase>`. On scaffold failure, surface the tool's structured output and stop -- do **not** attempt to hand-author Gradle files, `AndroidManifest.xml`, or any of the baseline `.kt` files. After scaffold success, run the host post-processing and verification steps in U8; record each step as `name`, `passed`, and a failure snippet.

If the scaffold and host checks succeed, switch to Update Mode. The scaffolded shell is a render-only baseline with CAP markers in `Core.kt`, `AndroidManifest.xml`, `libs.versions.toml`, and `app/build.gradle.kts` (one block per optional capability: `http`, `kv`, `sse`, `time`, `platform`) plus a starter `HomeScreen.kt`. Update Mode expands CAP blocks for capabilities the core uses, strips CAP blocks for capabilities it does not, and replaces the starter screen with real per-ViewModel-variant screens derived from the current `app.rs`.

## Verification ownership

When the orchestrator passes `skip_verification: true`, the writer stops after code generation and does **not** run step U8. The orchestrator's dedicated Android verify sub-agent handles pre-flight checks, `make build`, `./gradlew :shared:cargoBuild`, and `./gradlew :app:assembleDebug` with its own repair loop and iteration limits.

When invoked **standalone** (no `skip_verification` flag, or `skip_verification: false`), the writer runs its full process including step U8.

---

## Process: Create Mode

Use this process when no Android shell exists at `{project-dir}`. The scaffold tool owns render-only Android boilerplate (Gradle build files, version catalog, `AndroidManifest.xml` with conditional `networkSecurityConfig`, render-only baseline `Core.kt` with CAP markers, `{AppName}Application.kt` with the UniFFI library override, `MainActivity.kt`, `ui/screens/HomeScreen.kt`, and `res/xml/network_security_config.xml` when HTTP/SSE is selected). Host post-processing owns Gradle wrapper bootstrap, `local.properties`, SDK/NDK detection, and the Java 21 pin. This skill's Create-Mode responsibilities are: (1) read the Crux core to derive the app name and capability set, (2) invoke the scaffold tool, (3) run host post-processing and verification, (4) switch to Update Mode.

### 1. Read the Crux core

Read `{app-dir}/shared/src/app.rs` and extract all types listed in the Input Analysis table above. In particular, derive the App struct name (used by the scaffold to name the Gradle project, Application class, package folder, and Android theme) and note which capabilities the core actually uses -- this drives which CAP blocks Update Mode must expand in the scaffolded `Core.kt`, `AndroidManifest.xml`, `app/build.gradle.kts`, and `libs.versions.toml`. If `app.rs` cannot be read or parsed, report the error and stop.

### 2. Invoke the scaffold tool

Run:

```bash
cd {app-dir}
specify tool run vectis -- scaffold android {AppName} [--caps {caps}] [--android-package <package>]
```

`--android-package` is optional; when omitted, the scaffold uses `com.vectis.<appname-lowercase>`. The tool generates the render-only Gradle project (root + `app` + `shared` modules) and a baseline shell in `Android/app/src/main/java/<package-path>/` with CAP markers for every optional capability. The output is structured. On non-zero exit, surface the tool output to the user and stop.

### 3. Switch to Update Mode

After the scaffold and host checks return green, treat the scaffolded Android shell as an existing implementation and execute **Process: Update Mode** below to:

- Expand CAP blocks in `Core.kt` (adding real effect handlers + client classes) for capabilities the core uses, and strip CAP blocks for capabilities it does not.
- Expand matching CAP blocks in `AndroidManifest.xml` (`INTERNET` permission, `networkSecurityConfig`), `app/build.gradle.kts` (Ktor / Koin dependencies), and `libs.versions.toml` (capability library versions) -- or strip them if the capability is absent.
- Replace the `HomeScreen` starter with real per-ViewModel-variant screen files driven by the core's `ViewModel` enum + per-page view structs.
- Rewrite the root composable in `MainActivity.kt` to cover every ViewModel variant.
- Apply any `## Android Shell Requirements` from the active Specify change (when `slice-dir` is provided).

Dependency version pins (Kotlin, AGP, Ktor, Koin, Compose BOM, etc.) come from the active Vectis version pins used by `vectis` (`scaffold`); route pin changes through the Vectis version/template workflow rather than hand-editing `libs.versions.toml` / `shared/build.gradle.kts`.

## Process: Update Mode

Use this process when `{project-dir}/` already exists with Kotlin files.

### U1. Read and analyze the Crux core

Same as create mode step 1 (read `{app-dir}/shared/src/app.rs` and extract the full type inventory using the Input Analysis table above).

When `slice-dir` is provided, also read the `## Android Shell Requirements` section from `{slice-dir}/specs/{feature-name}/spec.md` and the `## Android Shell Details` section from `{slice-dir}/design.md` for platform-specific requirements.

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
| Component composables / Theme files / Drawable resources | `.kt` files in `ui/components/` (one per `component: <slug>`); `.kt` files in `ui/theme/` (one per `tokens.yaml` category, plus `Theme.kt`); per-density files under `app/src/main/res/drawable*/` (one set per `assets.yaml` `kind: raster` / `kind: vector` entry) |
| Event dispatches | All `onEvent(...)` or `core.update(...)` calls |
| Capability clients | Client classes in `core/` |
| DI modules | Koin module definitions |
| Design system usage | `MaterialTheme.colorScheme` / `MaterialTheme.typography` / `VectisSpacing` / `VectisCornerRadius` / `VectisElevation` references; presence or absence of `import com.vectis.design.*` and `implementation(project(":vectis-design"))` (legacy — must be removed) |

### U4. Diff analysis

Compare the Rust core types (from U1) and the input artifacts (`tokens.yaml`, `assets.yaml`, `composition.yaml`) against the Kotlin inventory (from U3). For each category, classify items as Added, Removed, Modified, or Unchanged.

Walk through in this order:

1. **Effect variants** -- new or removed capabilities affect Core.kt and may require new client classes.
2. **ViewModel variants** -- new or removed views affect the root composable and screen composable files.
3. **Per-page view struct fields** -- changed display data affects screen composables.
4. **Event variants** -- new or removed user actions affect screen composables.
5. **Route variants** -- new or removed navigation destinations affect navigation code.
6. **Token categories / asset entries / component directives** -- diff `tokens.yaml` against `ui/theme/`, `assets.yaml` against `res/drawable*/`, and `composition.yaml` `component:` slugs against `ui/components/` per the U6a-c contract; refer to [`references/kotlin-token-templates.md`](references/kotlin-token-templates.md) and [`references/design-system-integration.md`](references/design-system-integration.md) for category-by-category rules.
7. **Legacy `:vectis-design` references** -- `include(":vectis-design")` in `settings.gradle.kts` and `implementation(project(":vectis-design"))` in `app/build.gradle.kts` are migration debt and MUST be removed; `import com.vectis.design.*` lines MUST be replaced with `import com.vectis.<appname>.ui.theme.*`. Shell-local `ui/theme/` now satisfies the same role.

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
- Verify newly generated scrollable containers do not contain fill-max-size children (see `references/compose-view-patterns.md` Layout Constraint Rules).
- Replace lingering `import com.vectis.design.*` lines in screen / component / activity / Application files with `import com.vectis.<appname>.ui.theme.*`. Theme types (`VectisSpacing`, `VectisCornerRadius`, etc.) live in the `ui.theme` sibling package and require an explicit import from `ui.screens` / `ui.components` (Kotlin only auto-imports within the exact same package).

### U6a-c. Refresh shell-local design system inputs

When `tokens.yaml` / `assets.yaml` / `composition.yaml` change, regenerate the matching shell-local trees using the rules in [`references/design-system-integration.md`](references/design-system-integration.md) and [`references/kotlin-token-templates.md`](references/kotlin-token-templates.md):

- **U6a — `ui/theme/` from `tokens.yaml`**: one Kotlin file per category (`spacing` / `cornerRadius` colocated in `Spacing.kt`) plus structural `Theme.kt` defining `VectisTheme`. Generated files carry the `// Generated from design-system/tokens.yaml — do not edit manually.` header (except `Theme.kt`); operator-authored files (no header) are preserved on category removal. The header always uses the canonical project-level path `design-system/tokens.yaml` regardless of whether the current generation reads from a change-local file — this keeps the header stable across the change lifecycle and avoids breaking header-based detection of generated files. Absent `tokens.yaml` → emit the Material 3 fallback `Theme.kt`.
- **U6b — `ui/components/` from `composition.yaml`**: one named `@Composable` per `group.component: <slug>` under `ui/components/<PascalCaseSlug>.kt`. Props inferred from variation; constants baked in. Structural identity is pre-enforced by `specify tool run vectis -- validate composition`. Removed slugs have their files deleted and call sites inlined.
- **U6c — `res/drawable*/` from `assets.yaml`**: copy per-density `kind: raster` files into `res/drawable-<density>/<asset-id>.<ext>`, vector drawables into `res/drawable/<asset-id>.xml`; translate kebab-case ids to lowercase-with-underscores. `kind: symbol` skips the resource step (emit `Icons.Default.<glyph>` at the call site). Canonical `source:` SVG is provenance only — never copied. Missing Android exports for referenced vector entries halt generation; the writer never falls back to the canonical SVG, generates a placeholder, or skips the screen.

### U7. Update build configuration

- Update `build.gradle.kts` files if new dependencies are needed.
- Update `libs.versions.toml` if new library versions are needed.
- Update `AndroidManifest.xml` if permissions changed (e.g., INTERNET for HTTP).
- Remove legacy `include(":vectis-design")` from `settings.gradle.kts` and the matching `implementation(project(":vectis-design"))` from `app/build.gradle.kts`. The shell-local `ui/theme/` files satisfy the same role; leaving the entries breaks the build once `design-system/android/` is removed post-Phase 4.1.

### U8. Build and verify

Host post-processing and pre-flight checks before build:

Write `local.properties` from `$ANDROID_HOME` (`printf 'sdk.dir=%s\n' "$ANDROID_HOME" > local.properties`), ensure the requested NDK exists (`sdkmanager "ndk;29.0.14206865"` if absent), ensure Rust Android targets exist (`rustup target add armv7-linux-androideabi aarch64-linux-android i686-linux-android x86_64-linux-android` if absent), set `org.gradle.java.home` to Java 21 when missing, and bootstrap `{project-dir}/gradlew` with `gradle wrapper` when absent.

Build sequence:

1. Run `make build` in `{project-dir}` to regenerate types.
2. Run `./gradlew :shared:cargoBuild` to cross-compile the Rust library.
3. Run `./gradlew :app:assembleDebug` to build the APK.
4. Fix any build errors.

## Composition Mapping Priority

When `composition.yaml` is present, the region structure and group container tree take precedence over convention-based inference for composable body composition:

- **Groups** map to Compose containers: `direction: row` → `Row(horizontalArrangement:)`, `direction: column` → `Column(verticalArrangement:)`, `direction: stack` → `Box`.
- **Sizing** maps to `Modifier` calls: `fill` → `Modifier.fillMaxWidth()`, fixed values → `Modifier.width()` / `Modifier.height()`.
- **Surface decoration** maps to card-like containers: `background` + `corner_radius` → `Card` or `Surface` with shape and color, `elevation` → `Modifier.shadow()` or `Card(elevation:)`.
- **Platform-specific overrides**: When `composition.yaml` contains `platforms.android` region overrides for a screen, use those in preference to the shared regions.

When `composition.yaml` is absent, the existing inference behavior is unchanged — this preserves backward compatibility for projects that predate the wired-composition input set.

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

In Update Mode, minimize collateral changes — never regenerate from scratch; preserve developer-added styling, custom composable logic (animations, gestures), `@Preview` blocks, Gradle customizations, and Makefile customizations. See [`rules.md`](rules.md) for the full preservation contract and the platform-level Important Notes.

## Reference Documentation

| Reference | Purpose |
|---|---|
| `references/crux-android-shell-pattern.md` | Core.kt template, effect handling, serialization protocol |
| `references/compose-view-patterns.md` | Screen patterns, lists, forms, navigation, accessibility |
| `references/design-system-integration.md` | Shell-local theme + asset integration: generated layout, M3 fallback, copy-on-generate, component directive contract |
| `references/kotlin-token-templates.md` | Concrete Kotlin code templates per token category (color, typography, scalar, border, theme composable) |

Gradle build files, the version catalog, the Makefile, `AndroidManifest.xml`, CAP-marker scaffolding for the baseline `Core.kt` / `app/build.gradle.kts` / `libs.versions.toml`, and the starter Kotlin layout are owned by the Vectis scaffold templates in the [`augentic/specify-cli`](https://github.com/augentic/specify-cli) repo (`<specify-cli>/crates/vectis/` and the Vectis template sources). Do not hand-edit those files in Create Mode; let `specify tool run vectis -- scaffold android` write them and then modify in Update Mode. Host-derived files and settings (`local.properties`, Gradle wrapper artefacts, Java 21 pinning, SDK/NDK checks) are handled by the writer/verify workflow after render.

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
| Build fails with missing types | Run `make build` to regenerate types; verify `uniffi` matches the active Vectis version pins, then rerun the Android host verification steps |
| `cargoBuild` fails with `target may not be installed` | Run `rustup target add armv7-linux-androideabi aarch64-linux-android i686-linux-android x86_64-linux-android` |
| NDK not found | Install via `sdkmanager "ndk;29.0.14206865"` or Android Studio SDK Manager |
| Python 3 not found | Required by rust-android-gradle; install via system package manager |
| `./gradlew: No such file or directory` | Run the host wrapper bootstrap from `{project-dir}` (`gradle wrapper`) and record the step result |
| `Minimum supported Gradle version is X.Y` | Gradle/AGP drift -- route the pin change through the Vectis version/template workflow, rerender with `specify tool run vectis -- scaffold android ...`, then rerun host verification |
| `java.lang.IllegalArgumentException: 25.0.1` (or similar Java version parse error) | Set `org.gradle.java.home` to Java 21 in `gradle.properties` (derive with `/usr/libexec/java_home -v 21` on macOS when available) |
| `resource style/Theme.{AppName} not found` | Scaffold output is missing `res/values/themes.xml` -- rerender with `specify tool run vectis -- scaffold android ...` and stop if the file is still absent |
| `Unresolved reference 'Event'` (or `ViewModel`, `Effect`, etc.) | Add `import com.example.app.*` imports to the affected Kotlin file |
| `Unresolved reference 'CoreFfi'` | Add `import uniffi.shared.CoreFfi` to `Core.kt` |
| `Unresolved reference 'Icons'` | Add `material-icons-extended` dependency to `libs.versions.toml` + `app/build.gradle.kts` |
| `Namespace 'X' is used in multiple modules` | Use `com.vectis.{appname}.shared` namespace for the shared module |
| `unresolved module path shared::ffi` (codegen error) | UniFFI version mismatch -- rerun `make build`, `./gradlew :shared:cargoBuild`, and `./gradlew :app:assembleDebug` and inspect the active Vectis version pins |
| `This declaration needs opt-in` (unsigned types) | Add `@OptIn(ExperimentalUnsignedTypes::class)` to the class |
| Build fails on `:vectis-design` (`Project ':vectis-design' not found` or `Unresolved reference 'com.vectis.design'`) | Legacy migration debt — the shell now emits theme code under `app/src/main/java/com/vectis/<appname>/ui/theme/`. Drop the `include(":vectis-design")` line from `settings.gradle.kts`, the matching `implementation(project(":vectis-design"))` from `app/build.gradle.kts`, and replace `import com.vectis.design.*` lines with `import com.vectis.<appname>.ui.theme.*`; `MaterialTheme.colorScheme.*` / `MaterialTheme.typography.*` continue to resolve via standard M3 imports, while `VectisSpacing.*` / `VectisCornerRadius.*` etc. require the explicit theme-package import because `ui.theme` is a sibling package to `ui.screens` and `ui.components` |
| `specify tool run vectis -- validate composition` reports unresolved token / asset reference | The composition references a token or asset id missing from `tokens.yaml` / `assets.yaml`. Writer halts; operator must add the missing entry or remove the reference |
| Missing Android export for a `kind: vector` asset | This is a validation error, not a deferred TODO. Halt the affected screen and report the missing `sources.android` (Vector Drawable XML — SVG sources need converting first) |

### Runtime crashes

| Crash | Resolution |
|---|---|
| `UnsatisfiedLinkError: Unable to load library 'uniffi_shared'` | The scaffold-generated `{AppName}Application.kt` already calls `System.setProperty("uniffi.component.shared.libraryOverride", "shared")` first in `onCreate()`; verify the Application class was not replaced or its body reordered |
| `CLEARTEXT communication not permitted` | Ensure the scaffold was rendered with HTTP or SSE capabilities selected so it emitted `res/xml/network_security_config.xml` and the matching `networkSecurityConfig` attribute in `AndroidManifest.xml` |
| Unhandled exception in SSE/Time coroutine | Wrap `scope.launch` blocks for async effects in `try/catch`, rethrow `CancellationException`, and resolve the effect request with a fallback response (`SseResponse.Done`, `TimeResponse.DurationElapsed`, etc.) so the Rust core is never left awaiting an unresolved ID |
| `IllegalStateException: Vertically scrollable component was measured with an infinity maximum height constraints` | A composable that needs bounded constraints (e.g., `SearchBar` in expanded mode, `BottomSheet` content, any `fillMaxSize()` child) is inside a `verticalScroll` container. Move the component outside the scrollable area, use `LazyColumn` with bounded items, or use `DockedSearchBar`. See `references/compose-view-patterns.md` Layout Constraint Rules. |

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

- [ ] `tokens.yaml` present → `app/src/main/java/com/vectis/<appname>/ui/theme/` has one Kotlin file per token category plus a `Theme.kt` that defines `VectisTheme` over a static token-derived `ColorScheme` (no `dynamicLightColorScheme`/`dynamicDarkColorScheme`); `tokens.yaml` absent → only `Theme.kt` exists, wrapping `MaterialTheme` with M3 dynamic / static defaults per `references/design-system-integration.md`
- [ ] Screens and components reference `MaterialTheme.colorScheme.*` / `MaterialTheme.typography.*` / `VectisSpacing.*` / `VectisCornerRadius.*`; no hardcoded `Color(0xFF…)` or magic `dp` / `sp` outside generated theme files
- [ ] No `import com.vectis.design.*`, `include(":vectis-design")`, `implementation(project(":vectis-design"))`, or `path: ../../design-system/android` references anywhere

### Assets

- [ ] Every `composition.yaml` asset id resolves in `assets.yaml`; raster entries have matching `res/drawable-<density>/<asset-id>.<ext>` files, vector entries have `res/drawable/<asset-id>.xml`, `kind: symbol` entries render via `Icons.Default.<glyph>` (no resource copy); no `path: ../../design-system/assets` references

### Components

- [ ] Every `component: <slug>` has `ui/components/<PascalCaseSlug>.kt`, every call site uses the named `@Composable`, and removed slugs have their files deleted

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
- [ ] No fill-max-size components (SearchBar, BottomSheet, fillMaxSize()) placed inside unbounded scrollable containers (verticalScroll, horizontalScroll)
- [ ] Application class installs a global crash recovery handler (see references/crux-android-shell-pattern.md Crash Recovery Handler)

### Command-Line Workflow

- [ ] Build works from terminal: `./gradlew :app:assembleDebug`
- [ ] Emulator can be launched: `emulator -avd <name>`
- [ ] App can be installed: `./gradlew :app:installDebug`
- [ ] App can be launched: `adb shell am start -n <package>/.MainActivity`

## Important Notes

The platform-level normative facts — UniFFI bridging and library override, generated-type packages, Gradle wrapper bootstrap, Java 21 pin, network security config, defensive `CoreFFI` error handling, mandatory `themes.xml`, the crash-recovery pattern, and how `slice-dir` integrates — are covered in [`rules.md`](rules.md). Read it once at the start of any Android run before editing the scaffold.
