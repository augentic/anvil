# Android Shell Review Checks

Structural and integration checks for Crux Android shells (Kotlin/Jetpack Compose). Each check has an ID, description, severity, and detection method.

## AND-001: Missing Screen Composable for ViewModel Variant

**Severity**: critical

Every variant in the Rust `enum ViewModel` that carries a per-page view struct must have a corresponding Kotlin screen composable file in `ui/screens/`.

**Detection**: Extract ViewModel variants from `app.rs`. For each variant with a payload, verify a `.kt` file exists in `ui/screens/` with a composable that accepts the matching view model type.

**Fix**: Create the missing screen composable file following `references/compose-view-patterns.md`.

## AND-002: Missing Root Composable Branch

**Severity**: critical

The root composable `when` expression (in `MainActivity.kt` or `AppView`) must have one branch per ViewModel variant. A missing branch means the shell cannot render that view.

**Detection**: Count branches in the root composable `when`. Compare against the number of ViewModel variants in `app.rs`.

**Fix**: Add the missing branch, rendering the appropriate screen composable.

## AND-003: Missing Effect Handler

**Severity**: critical

Every variant in the Rust `enum Effect` must have a corresponding branch in the `processRequest` `when` expression in `Core.kt`. A missing handler means the core's side-effect request will be silently dropped.

**Detection**: Extract Effect variants from `app.rs`. Verify each has a branch in the `processRequest` method.

**Fix**: Add the missing effect handler branch. See `references/crux-android-shell-pattern.md` for handler templates.

## AND-004: Undispatched Shell-Facing Event

**Severity**: important

Every shell-facing Event variant (those without `#[serde(skip)]` or `#[facet(skip)]`) should be dispatched by at least one composable. An undispatched event means a user action described in the spec has no UI trigger.

**Detection**: Extract shell-facing Event variants from `app.rs`. Search all `.kt` files for `onEvent(Event.VariantName` or `core.update(Event.VariantName`. Flag variants with zero matches. Exclude `Navigate` as it may be handled via Compose Navigation APIs rather than explicit dispatch.

**Fix**: Add the event dispatch to the appropriate screen composable.

## AND-005: Hardcoded Color

**Severity**: important

Composables should use design system color tokens when available, not hardcoded `Color(...)`, `Color.Red`, or hex values.

**Detection**: Search `.kt` files under the **app module** source roots (typically `app/src/main/java/` or `app/src/main/kotlin/`) for:
- `Color(0x` or `Color(red =` (explicit color construction)
- `Color.Red`, `Color.Blue`, etc. (named colors used as semantic colors)
- Hex color patterns `0xFF[0-9A-Fa-f]{6}` outside generated theme files

Exclude Material Theme color references (`MaterialTheme.colorScheme.*`).

**Do not flag** generated theme files under `Android/app/src/main/java/com/vectis/<appname>/ui/theme/` that carry the `// Generated from design-system/tokens.yaml — do not edit manually.` header — these legitimately contain `Color(0xFF...)` emitted from `tokens.yaml` by `vectis:android-writer` (per the generated-layout policy). Detect the carve-out by reading the first 5 lines of each `.kt` file and skipping when the header is present.

**Fix**: Replace with the appropriate design system color token or `MaterialTheme.colorScheme` reference.

## AND-006: Hardcoded Typography

**Severity**: important

Composables should use design system typography tokens or `MaterialTheme.typography` rather than inline `TextStyle(fontSize = ...)`.

**Detection**: Search **app module** `.kt` files for `TextStyle(fontSize` or `fontSize = ` with numeric literals without a preceding design system reference.

Exclude icon sizing in `Icon` composables. Exclude generated theme files under `Android/app/src/main/java/com/vectis/<appname>/ui/theme/` that carry the `// Generated from design-system/tokens.yaml — do not edit manually.` header (token `TextStyle` definitions live there in the current Vectis shell contract; the same header-based carve-out as AND-005 applies).

**Fix**: Replace with the appropriate design system typography token or `MaterialTheme.typography` reference.

## AND-007: Hardcoded Spacing

**Severity**: important

Padding and spacing values should use design system spacing tokens, not magic numbers.

**Detection**: In **app module** composables, search for `.padding(` or `Arrangement.spacedBy(` with numeric literals (`X.dp`) that are not `0.dp`. Check that the value matches a token; flag if it does not. Skip generated theme files under `Android/app/src/main/java/com/vectis/<appname>/ui/theme/` carrying the `// Generated from design-system/tokens.yaml — do not edit manually.` header (the same header-based carve-out as AND-005 / AND-006).

**Fix**: Replace with the appropriate design system spacing token (e.g. `VectisSpacing.md`). The current writer emits `VectisSpacing` as a shell-local `Spacing.kt` file under `ui/theme/` (`com.vectis.<appname>.ui.theme` package). Consumers in sibling packages (`ui.screens`, `ui.components`) must have `import com.vectis.<appname>.ui.theme.*` — do not use the legacy `import com.vectis.design.*`.

## AND-008: Missing Preview

**Severity**: suggestion

Every screen composable should have a `@Preview` annotated composable with sample data for development and visual testing.

**Detection**: For each screen composable file in `ui/screens/`, check for a `@Preview` annotation.

**Fix**: Add a `@Preview` composable with sample data at the bottom of the file.

## AND-009: Missing Accessibility Description

**Severity**: important

Interactive icons (buttons with only an `Icon` composable, no `Text`) must have a `contentDescription` that is not `null`.

**Detection**: Search for `Icon(` calls inside `IconButton` or `FloatingActionButton` where `contentDescription = null`.

**Fix**: Add a descriptive `contentDescription` to the `Icon`.

## AND-010: Route/Navigation Mismatch

**Severity**: important

If the Rust core defines a `Route` enum, the Android shell should implement navigation that covers all Route variants.

**Detection**: Extract Route variants from `app.rs`. Verify the shell dispatches `onEvent(Event.Navigate(Route.VARIANT))` for each variant via navigation controls (bottom nav, buttons, drawer items).

**Fix**: Add navigation elements for missing Route variants.

## AND-011: Missing UniFFI Library Override

**Severity**: critical

An `Application` class is required in **all** Android shells -- not just those using Koin. Its `onCreate()` must set the JNA library override property BEFORE any UniFFI class is loaded. Without this, JNA tries to load `libuniffi_shared.so` but Cargo produces `libshared.so`, causing an `UnsatisfiedLinkError` crash on launch.

**Detection**: Verify that an Application class exists and that `AndroidManifest.xml` includes the `android:name` attribute pointing to it. Search the Application class for `System.setProperty("uniffi.component.shared.libraryOverride", "shared")`. Verify it appears before `startKoin` or any other code that triggers UniFFI class loading. If no Application class exists at all, flag it as critical.

**Fix**: Create an Application class with `System.setProperty("uniffi.component.shared.libraryOverride", "shared")` as the first statement after `super.onCreate()`, and add `android:name` to the manifest's `<application>` element.

## AND-012: Core Missing StateFlow / mutableStateOf

**Severity**: critical

The `Core` class must expose the ViewModel via either `mutableStateOf` (simple pattern) or `StateFlow` (full pattern with Koin). Without proper state exposure, Compose cannot observe changes and the UI will not update.

**Detection**: Check `Core.kt` for one of:
- `var view: ViewModel by mutableStateOf(...)` (simple pattern)
- `val viewModel: StateFlow<ViewModel>` backed by a `MutableStateFlow` (full pattern)

**Fix**: Add the appropriate state exposure pattern.

## AND-013: Missing Generated Type Imports

**Severity**: critical

All hand-written `.kt` files that reference generated types (`Event`, `ViewModel`, `Effect`, `Request`, etc.) MUST have explicit imports from `com.example.app.*`. The generated types live in a different package than the hand-written code.

**Detection**: Search hand-written `.kt` files for references to generated types without corresponding `import com.example.app.` statements. Also check `Core.kt` for `import uniffi.shared.CoreFfi`.

**Fix**: Add the missing import statements. Never assume generated types are in the same package as hand-written code.

## AND-014: Enum Pattern Match Style Mismatch

**Severity**: important

Simple Rust enums (no payloads) are generated as Kotlin `enum class` with `UPPER_CASE` values and must be matched with `==` equality. Sealed interface variants (with payloads) must be matched with `is`. Using the wrong pattern causes compile errors or incorrect matching.

**Detection**: Search for `is` checks against `enum class` values (e.g., `is Filter.All` instead of `Filter.ALL`). Also search for equality checks against `sealed interface` data class variants.

**Fix**: Use `==` for `enum class` values; use `is` for `sealed interface` data class variants; use direct reference for `data object` variants.

## AND-015: Async Effect Handler Missing try/catch or Missing Fallback Resolve

**Severity**: critical

All async effect handlers (SSE, Time) that run inside `scope.launch` blocks MUST wrap their body in `try/catch` to prevent unhandled exceptions from crashing the app. The catch block MUST rethrow `CancellationException` to preserve coroutine cancellation semantics. The catch block MUST also call `resolveAndHandleEffects` with a fallback response so the core request ID is never left unresolved (e.g., `SseResponse.Done` for SSE, `TimeResponse.DurationElapsed` / `TimeResponse.InstantArrived` for timers). A catch block that only logs leaves the core stalled in a loading or pending state.

**Detection**: In `Core.kt`, search for `scope.launch` blocks inside `processRequest` and inside `handleTimeEffect`. Verify each has a `try/catch` wrapping the body. Check that `CancellationException` is rethrown (`catch (e: CancellationException) { throw e }`). Check that the non-cancellation catch branch calls `resolveAndHandleEffects`.

**Fix**: Wrap the `scope.launch` body in:
```kotlin
try {
    // ... effect handling
} catch (e: CancellationException) { throw e }
catch (e: Exception) {
    Log.e(TAG, "effect error", e)
    resolveAndHandleEffects(requestId, fallbackResponse.bincodeSerialize())
}
```

## AND-016: Missing SupervisorJob in CoroutineScope

**Severity**: important

The `Core` class `CoroutineScope` must use `SupervisorJob()` for fault isolation. Without it, one failing coroutine cancels all sibling coroutines, including unrelated effect handlers.

**Detection**: In `Core.kt`, check the `CoroutineScope` constructor for `SupervisorJob()`. Flag if `Job()` is used or if `SupervisorJob` is absent.

**Fix**: Use `CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)`.

## AND-017: Missing themes.xml Resource

**Severity**: critical

`AndroidManifest.xml` references a theme resource (`@style/Theme.{AppName}`). The `res/values/themes.xml` file MUST exist or the build fails with `resource style/Theme.{AppName} not found`.

**Detection**: Check for the existence of `app/src/main/res/values/themes.xml`. Verify it contains a `<style>` element matching the theme name referenced in `AndroidManifest.xml`.

**Fix**: Create `res/values/themes.xml` with the appropriate theme style.

## AND-018: Missing Network Security Config

**Severity**: important

Apps with HTTP or SSE effects must include a `network_security_config.xml` referenced in `AndroidManifest.xml`. Without it, Android 9+ blocks cleartext HTTP traffic and the app crashes with `CLEARTEXT communication not permitted` when connecting to development servers.

**Detection**: If the app has `Effect.Http` or `Effect.ServerSentEvents`, check for:
1. `res/xml/network_security_config.xml` exists
2. `AndroidManifest.xml` has `android:networkSecurityConfig="@xml/network_security_config"`

**Fix**: Create the config file allowing cleartext to localhost and `10.0.2.2` (emulator host alias). Reference it in the manifest.

## AND-019: ULong Displayed Without Conversion

**Severity**: important

`ULong` values from generated types (e.g., count fields mapped from `usize`) must be cast to `Long` when displayed in Compose `Text` composables. Passing `ULong` directly to string interpolation may produce unexpected output.

**Detection**: Search for `Text(` composables containing string interpolation of properties known to be `ULong` (check generated types). Flag any that do not include `.toLong()`.

**Fix**: Add `.toLong()` conversion: `"${viewModel.count.toLong()}"`.

## AND-020: Missing @OptIn for Unsigned Types

**Severity**: important

Classes that call `.toUByteArray()` require the `@OptIn(ExperimentalUnsignedTypes::class)` annotation. Without it, the build emits warnings or errors depending on compiler settings.

**Detection**: Search for `.toUByteArray()` calls. Verify the containing class or function has `@OptIn(ExperimentalUnsignedTypes::class)`.

**Fix**: Add the annotation to the class declaration.

## AND-021: Namespace Collision Between Modules

**Severity**: important

The `app` module and `shared` module MUST have different `namespace` values in their `build.gradle.kts` files. If they collide, the build emits confusing warnings or fails.

**Detection**: Compare the `namespace` values in `app/build.gradle.kts` and `shared/build.gradle.kts`. Flag if they are identical.

**Fix**: Use `com.vectis.{appname}` for `app` and `com.vectis.{appname}.shared` for `shared`.

## AND-022: Time Effect Clear Handler Missing Job Cancellation

**Severity**: critical

If the app handles `Effect.Time`, the `TimeRequest.Clear` branch must actually cancel the coroutine job for any previously scheduled `NotifyAfter` or `NotifyAt` timer. Without job tracking and cancellation, cleared timers continue to fire stale `DurationElapsed` or `InstantArrived` events into the core, producing incorrect state transitions.

**Detection**: In `Core.kt`, verify all three conditions:

1. A `MutableMap<TimerId, Job>` (or equivalent) field exists to track active timer coroutine jobs.
2. `NotifyAfter` and `NotifyAt` branches store their launched coroutine `Job` in the map, keyed by the timer's `TimerId`.
3. The `Clear` branch removes and cancels the stored job (`timerJobs.remove(timerId)?.cancel()`) before responding with `TimeResponse.Cleared`.

Also verify that `NotifyAfter` and `NotifyAt` coroutines clean up their map entry on natural completion and on `CancellationException`.

**Fix**: Add a `timerJobs` map to the `Core` class. In `NotifyAfter` and `NotifyAt`, launch a child coroutine via `scope.launch`, store the `Job` in the map, and remove the entry in a `finally`-equivalent path (after the delay completes or when cancelled). In `Clear`, call `timerJobs.remove(timeRequest.value)?.cancel()` before resolving. See `references/crux-android-shell-pattern.md` for the full implementation.

## AND-023: CoreFFI Errors Not Surfaced

**Severity**: critical

`CoreFfi` methods (`view()`, `update()`, `resolve()`) return `Result<Vec<u8>, CoreError>` in Rust, which UniFFI maps to Kotlin functions that throw `CoreException`. The exception contains a meaningful `Bridge` error message from the Rust core (deserialization failure, invalid effect ID, etc.). Calling these without `try/catch` lets the exception propagate unhandled and crash the app. Using a generic catch that discards `e.message` loses the diagnostic information needed to debug type mismatches after core regeneration.

Unlike bincode serialization (AND-014), CoreFFI calls throw structured errors with context from the Rust side. All CoreFFI calls must use `try/catch` with `Log.e(TAG, "context: ${e.message}", e)` so the underlying reason is visible in logcat during development.

**Detection**: Search `Core.kt` for `coreFfi.view()`, `coreFfi.update(`, and `coreFfi.resolve(` calls. Verify each is wrapped in a `try/catch` block that logs `e.message`. Flag any CoreFFI calls that:

1. Have no `try/catch` at all (exception propagates to the caller)
2. Use a catch block that discards the message (e.g., catches `Exception` but only logs a static string without `${e.message}`)
3. Use a catch block that rethrows without logging (diagnostic is lost unless the caller also logs)

**Fix**: Wrap each CoreFFI call in a `try/catch` block:

```kotlin
val effects = try {
    coreFfi.update(serialized)
} catch (e: Exception) {
    Log.e(TAG, "Failed to update core: ${e.message}", e)
    return
}
```

In `initialView()` (called during construction), fall back to `ViewModel.Loading`. In effect handlers, preserve the existing view or return without state changes. See the android-writer skill step 8 for the full pattern.

## AND-024: Render Effect Overwrites View with Loading Fallback

**Severity**: important

The `Effect.Render` handler must not fall back to `ViewModel.Loading` (or any other default ViewModel) on deserialization failure. This would overwrite the user's current view (e.g., a task list, a form with entered data) with a loading screen on any transient serialization error. The `initialView()` helper is the only place where a `ViewModel.Loading` fallback is safe, because no prior state exists at construction time.

**Detection**: In the `processRequest` method, check the `Effect.Render` branch for:

1. Calls to `initialView()` or any helper that falls back to `ViewModel.Loading`
2. Direct assignment of `ViewModel.Loading` in a catch block
3. Any pattern that assigns a default ViewModel value when deserialization fails

**Fix**: Replace with an inline pattern that preserves the existing view on failure:

```kotlin
is Effect.Render -> {
    val data = try {
        coreFfi.view()
    } catch (e: Exception) {
        Log.e(TAG, "Failed to get view from core: ${e.message}", e)
        return
    }
    val vm = try {
        ViewModel.bincodeDeserialize(data)
    } catch (e: Exception) {
        Log.w(TAG, "Failed to deserialize ViewModel", e)
        return
    }
    _viewModel.value = vm
}
```

See the android-writer skill step 8 for the full Core.kt pattern.

## AND-025: Fill-Max-Size Component Inside Unbounded Scrollable Container

**Severity**: critical

Components that internally expand to fill available space (Material 3 `SearchBar` in expanded mode, `BottomSheet` modal content, or any composable applying `fillMaxSize()` / `fillMaxHeight()`) must not be placed inside a `verticalScroll` or `horizontalScroll` container. The scrollable container provides infinite max constraints; the fill-sizing component cannot resolve against infinity, and Compose throws `IllegalStateException` at runtime.

**Detection**: In screen composable files, find `Column` or `Row` modifiers containing `.verticalScroll(` or `.horizontalScroll(`. Within those containers, flag any child that is: (a) `SearchBar` (not `DockedSearchBar`), (b) applies `Modifier.fillMaxSize()` or `Modifier.fillMaxHeight()`, or (c) contains a composable known to expand internally (e.g., `ModalBottomSheet` content).

**Fix**: Move the fill-sizing component outside the scrollable container, use `Modifier.weight(1f)` on the scrollable portion only, switch to `LazyColumn` with bounded items, or use `DockedSearchBar` instead of `SearchBar`. See `compose-view-patterns.md` Layout Constraint Rules.

## AND-026: Missing Crash Recovery Handler

**Severity**: important

The Application class should install a global `Thread.setDefaultUncaughtExceptionHandler` that persists crash info and schedules a restart via `PendingIntent` / `AlarmManager`, then delegates to the previous default handler so crash reporters (e.g. Crashlytics) still run and the system handles process termination normally. The handler must include a crash-loop guard (cooldown window persisted in SharedPreferences) to avoid rapid restart loops when the crash occurs during startup. For Crux apps, the core manages state independently of the shell, so an Activity restart re-creates the Core and re-renders the current ViewModel with minimal user disruption.

**Detection**: In the Application class, search for `Thread.setDefaultUncaughtExceptionHandler` or `Thread.getDefaultUncaughtExceptionHandler`. If absent, flag as missing. If present, verify it: (a) schedules restart via `PendingIntent` / `AlarmManager` rather than calling `startActivity` + `exitProcess` directly, (b) delegates to the previous `defaultHandler` after scheduling, and (c) includes a crash-loop guard. Also check `MainActivity` for crash flag detection in `onCreate` (reading from SharedPreferences key `"crux_crash_recovery"` or equivalent).

**Fix**: Add the crash recovery handler to the Application class and the crash flag detection to `MainActivity.onCreate`. See `crux-android-shell-pattern.md` Crash Recovery Handler section.

## AND-027: Recurring Composition Group Without Component Directive

**Severity**: suggestion

Per the component directive contract and reviewer surface, any `group` shape that visibly recurs across `composition.yaml` (≥2 instances on the same screen, or ≥2 instances across different screens) without a `component: <slug>` directive is a candidate for promotion to a named component. Without the directive, the Android shell ends up with parallel inline copies of the same Compose subtree across `ui/screens/*.kt` files; when the layout changes the operator must hand-edit every copy, and drift compounds silently. The reviewer flags candidate slugs for the operator to evaluate; promotion itself remains an authoring decision (it requires editing `composition.yaml` and adding a sibling `Android/app/src/main/java/com/vectis/<appname>/ui/components/<Slug>.kt` file via `vectis:android-writer`).

**Detection**: When the wired `composition.yaml` is available (sibling at the change-local or baseline path — see SKILL.md "Gather context"):

1. Walk the composition tree collecting every `group` node.
2. Compute a structural skeleton for each group (the same `*-when` presence + nested-item-kind shape that `specify tool run vectis -- validate composition` uses for the §G structural-identity rule).
3. Group instances by skeleton equality. For any skeleton that appears in ≥2 instances **without** a sibling `component:` directive on any of those instances, flag the recurrence as a candidate component.
4. Cross-check the Android shell: if the recurring composition group already corresponds to an extracted `@Composable` under `ui/components/`, downgrade severity to `optional` and note the existing extraction; otherwise emit at the canonical `suggestion` severity (the operator will promote both surfaces in lockstep).

When `composition.yaml` is absent (composition-less change, or a change that only touches `app.rs` types), skip this check entirely — there is no source-of-truth recurrence signal in shell code alone.

**Fix**: This is a candidate finding, not a defect. Suggest one of two actions and let the operator pick:

1. **Promote to component.** Add `component: <slug>` to the recurring group(s) in `composition.yaml` (kebab-case slug; not a reserved region name like `header` / `body` / `footer` / `fab`); regenerate the Android shell via `vectis:android-writer`; the writer emits a single `ui/components/<Slug>.kt` `@Composable` and rewrites every call site to use it (per the component directive contract).
2. **Accept the inline duplication.** When the recurring group is intentionally distinct (e.g. two visually similar groups that diverge in a way the skeleton check cannot see — different gesture handling, different state semantics), document the divergence in the composition or in `design.md` and accept the finding.

## AND-028: Vector Or Raster Asset Rendered As Platform Symbol

**Severity**: important

**Codex**: `rule_id: VECTIS-006`

Per the render-by-`kind` contract ([`android/design-system-integration.md`](../android/design-system-integration.md) § Asset integration), composition-referenced ids whose `assets.yaml` entry is `vector` or `raster` must emit `painterResource(R.drawable.<id_snake>)` (or equivalent per-density drawables) from materialized exports — never `Icons.Default.*` or other Material Icons substitutes.

**Detection**: When the wired `composition.yaml` and effective `assets.yaml` are available (slice-local path first, then `design-system/assets.yaml` — same precedence as the Android writer):

1. Walk the composition tree and collect every asset id referenced by `icon`, `image`, `icon-button`, or `fab` items.
2. Resolve each id in the effective `assets.yaml` and retain only those with `kind: vector` or `kind: raster`.
3. For each retained id, convert the kebab-case id to snake_case and verify the Android shell uses `painterResource(R.drawable.<id_snake>)` (or `Image(painter = painterResource(…))`) in the composables that render that composition node.
4. Flag any case where the same visual role uses `Icons.Default.` / `material.icons.Icons.` without a matching `kind: symbol` entry for that composition id.
5. Do **not** flag Material Icons when the composition id resolves to `kind: symbol`.

When `composition.yaml` or `assets.yaml` is absent, skip this check — there is no cross-artifact signal.

**Fix**: Regenerate the affected screen via `vectis:android-writer` after `vectis materialize assets` has populated `design-system/assets/exports/android/` (or after operator pins are in place). If the glyph is genuinely platform-native, change the `assets.yaml` entry to `kind: symbol` with `symbols.ios` / `symbols.android` and update composition to reference the symbol id — do not leave a `vector` / `raster` entry while the shell still substitutes Material Icons.
