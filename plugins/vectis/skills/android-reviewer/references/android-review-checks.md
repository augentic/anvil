# Android Shell Review Checks

Structural and integration checks for Crux Android shells (Kotlin/Jetpack
Compose). Each check has an ID, description, severity, and detection method.

## AND-001: Missing Screen Composable for ViewModel Variant

**Severity**: Critical

Every variant in the Rust `enum ViewModel` that carries a per-page view struct
must have a corresponding Kotlin screen composable file in `ui/screens/`.

**Detection**: Extract ViewModel variants from `app.rs`. For each variant with
a payload, verify a `.kt` file exists in `ui/screens/` with a composable that
accepts the matching view model type.

**Fix**: Create the missing screen composable file following
`references/compose-view-patterns.md`.

## AND-002: Missing Root Composable Branch

**Severity**: Critical

The root composable `when` expression (in `MainActivity.kt` or `AppView`)
must have one branch per ViewModel variant. A missing branch means the shell
cannot render that view.

**Detection**: Count branches in the root composable `when`. Compare against
the number of ViewModel variants in `app.rs`.

**Fix**: Add the missing branch, rendering the appropriate screen composable.

## AND-003: Missing Effect Handler

**Severity**: Critical

Every variant in the Rust `enum Effect` must have a corresponding branch in
the `processRequest` `when` expression in `Core.kt`. A missing handler means
the core's side-effect request will be silently dropped.

**Detection**: Extract Effect variants from `app.rs`. Verify each has a branch
in the `processRequest` method.

**Fix**: Add the missing effect handler branch. See
`references/crux-android-shell-pattern.md` for handler templates.

## AND-004: Undispatched Shell-Facing Event

**Severity**: Warning

Every shell-facing Event variant (those without `#[serde(skip)]` or
`#[facet(skip)]`) should be dispatched by at least one composable. An
undispatched event means a user action described in the spec has no UI trigger.

**Detection**: Extract shell-facing Event variants from `app.rs`. Search all
`.kt` files for `onEvent(Event.VariantName` or `core.update(Event.VariantName`.
Flag variants with zero matches. Exclude `Navigate` as it may be handled via
Compose Navigation APIs rather than explicit dispatch.

**Fix**: Add the event dispatch to the appropriate screen composable.

## AND-005: Hardcoded Color

**Severity**: Warning

Composables should use design system color tokens when available, not
hardcoded `Color(...)`, `Color.Red`, or hex values.

**Detection**: Search `.kt` files for:
- `Color(0x` or `Color(red =` (explicit color construction)
- `Color.Red`, `Color.Blue`, etc. (named colors used as semantic colors)
- Hex color patterns `0xFF[0-9A-Fa-f]{6}` outside design system definitions

Exclude Material Theme color references (`MaterialTheme.colorScheme.*`).

**Fix**: Replace with the appropriate design system color token or
`MaterialTheme.colorScheme` reference.

## AND-006: Hardcoded Typography

**Severity**: Warning

Composables should use design system typography tokens or `MaterialTheme.typography`
rather than inline `TextStyle(fontSize = ...)`.

**Detection**: Search `.kt` files for `TextStyle(fontSize` or
`fontSize = ` with numeric literals without a preceding design system
reference.

Exclude icon sizing in `Icon` composables.

**Fix**: Replace with the appropriate design system typography token or
`MaterialTheme.typography` reference.

## AND-007: Hardcoded Spacing

**Severity**: Warning

Padding and spacing values should use design system spacing tokens, not
magic numbers.

**Detection**: Search for `.padding(` or `Arrangement.spacedBy(` with numeric
literals (`X.dp`) that are not `0.dp`. Check that the value matches a token;
flag if it does not.

**Fix**: Replace with the appropriate design system spacing token.

## AND-008: Missing Preview

**Severity**: Info

Every screen composable should have a `@Preview` annotated composable with
sample data for development and visual testing.

**Detection**: For each screen composable file in `ui/screens/`, check for a
`@Preview` annotation.

**Fix**: Add a `@Preview` composable with sample data at the bottom of the
file.

## AND-009: Missing Accessibility Description

**Severity**: Warning

Interactive icons (buttons with only an `Icon` composable, no `Text`) must
have a `contentDescription` that is not `null`.

**Detection**: Search for `Icon(` calls inside `IconButton` or
`FloatingActionButton` where `contentDescription = null`.

**Fix**: Add a descriptive `contentDescription` to the `Icon`.

## AND-010: Route/Navigation Mismatch

**Severity**: Warning

If the Rust core defines a `Route` enum, the Android shell should implement
navigation that covers all Route variants.

**Detection**: Extract Route variants from `app.rs`. Verify the shell
dispatches `onEvent(Event.Navigate(Route.VARIANT))` for each variant via
navigation controls (bottom nav, buttons, drawer items).

**Fix**: Add navigation elements for missing Route variants.

## AND-011: Missing UniFFI Library Override

**Severity**: Critical

The `Application` class `onCreate()` must set the JNA library override
property BEFORE any UniFFI class is loaded. Without this, JNA tries to load
`libuniffi_shared.so` but Cargo produces `libshared.so`, causing an
`UnsatisfiedLinkError` crash on launch.

**Detection**: Search the Application class for
`System.setProperty("uniffi.component.shared.libraryOverride", "shared")`.
Verify it appears before `startKoin` or any other code that triggers UniFFI
class loading.

**Fix**: Add `System.setProperty("uniffi.component.shared.libraryOverride", "shared")`
as the first statement after `super.onCreate()`.

## AND-012: Core Missing StateFlow / mutableStateOf

**Severity**: Critical

The `Core` class must expose the ViewModel via either `mutableStateOf` (simple
pattern) or `StateFlow` (full pattern with Koin). Without proper state
exposure, Compose cannot observe changes and the UI will not update.

**Detection**: Check `Core.kt` for one of:
- `var view: ViewModel by mutableStateOf(...)` (simple pattern)
- `val viewModel: StateFlow<ViewModel>` backed by a `MutableStateFlow` (full pattern)

**Fix**: Add the appropriate state exposure pattern.

## AND-013: Missing Generated Type Imports

**Severity**: Critical

All hand-written `.kt` files that reference generated types (`Event`,
`ViewModel`, `Effect`, `Request`, etc.) MUST have explicit imports from
`com.example.app.*`. The generated types live in a different package than
the hand-written code.

**Detection**: Search hand-written `.kt` files for references to generated
types without corresponding `import com.example.app.` statements. Also check
`Core.kt` for `import uniffi.shared.CoreFfi`.

**Fix**: Add the missing import statements. Never assume generated types are
in the same package as hand-written code.

## AND-014: Enum Pattern Match Style Mismatch

**Severity**: Warning

Simple Rust enums (no payloads) are generated as Kotlin `enum class` with
`UPPER_CASE` values and must be matched with `==` equality. Sealed interface
variants (with payloads) must be matched with `is`. Using the wrong pattern
causes compile errors or incorrect matching.

**Detection**: Search for `is` checks against `enum class` values (e.g.,
`is Filter.All` instead of `Filter.ALL`). Also search for equality checks
against `sealed interface` data class variants.

**Fix**: Use `==` for `enum class` values; use `is` for `sealed interface`
data class variants; use direct reference for `data object` variants.

## AND-015: Async Effect Handler Missing try/catch

**Severity**: Critical

All async effect handlers (SSE, Time) that run inside `scope.launch` blocks
MUST wrap their body in `try/catch` to prevent unhandled exceptions from
crashing the app. The catch block MUST rethrow `CancellationException` to
preserve coroutine cancellation semantics.

**Detection**: In `Core.kt`, search for `scope.launch` blocks inside
`processRequest`. Verify each has a `try/catch` wrapping the body. Check
that `CancellationException` is rethrown (`catch (e: CancellationException) { throw e }`).

**Fix**: Wrap the `scope.launch` body in:
```kotlin
try {
    // ... effect handling
} catch (e: CancellationException) { throw e }
catch (e: Exception) { Log.e(TAG, "effect error", e) }
```

## AND-016: Missing SupervisorJob in CoroutineScope

**Severity**: Warning

The `Core` class `CoroutineScope` must use `SupervisorJob()` for fault
isolation. Without it, one failing coroutine cancels all sibling coroutines,
including unrelated effect handlers.

**Detection**: In `Core.kt`, check the `CoroutineScope` constructor for
`SupervisorJob()`. Flag if `Job()` is used or if `SupervisorJob` is absent.

**Fix**: Use `CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)`.

## AND-017: Missing themes.xml Resource

**Severity**: Critical

`AndroidManifest.xml` references a theme resource (`@style/Theme.{AppName}`).
The `res/values/themes.xml` file MUST exist or the build fails with
`resource style/Theme.{AppName} not found`.

**Detection**: Check for the existence of
`app/src/main/res/values/themes.xml`. Verify it contains a `<style>` element
matching the theme name referenced in `AndroidManifest.xml`.

**Fix**: Create `res/values/themes.xml` with the appropriate theme style.

## AND-018: Missing Network Security Config

**Severity**: Warning

Apps with HTTP or SSE effects must include a `network_security_config.xml`
referenced in `AndroidManifest.xml`. Without it, Android 9+ blocks cleartext
HTTP traffic and the app crashes with `CLEARTEXT communication not permitted`
when connecting to development servers.

**Detection**: If the app has `Effect.Http` or `Effect.ServerSentEvents`,
check for:
1. `res/xml/network_security_config.xml` exists
2. `AndroidManifest.xml` has `android:networkSecurityConfig="@xml/network_security_config"`

**Fix**: Create the config file allowing cleartext to localhost and `10.0.2.2`
(emulator host alias). Reference it in the manifest.

## AND-019: ULong Displayed Without Conversion

**Severity**: Warning

`ULong` values from generated types (e.g., count fields mapped from `usize`)
must be cast to `Long` when displayed in Compose `Text` composables.
Passing `ULong` directly to string interpolation may produce unexpected
output.

**Detection**: Search for `Text(` composables containing string interpolation
of properties known to be `ULong` (check generated types). Flag any that do
not include `.toLong()`.

**Fix**: Add `.toLong()` conversion: `"${viewModel.count.toLong()}"`.

## AND-020: Missing @OptIn for Unsigned Types

**Severity**: Warning

Classes that call `.toUByteArray()` require the
`@OptIn(ExperimentalUnsignedTypes::class)` annotation. Without it, the build
emits warnings or errors depending on compiler settings.

**Detection**: Search for `.toUByteArray()` calls. Verify the containing class
or function has `@OptIn(ExperimentalUnsignedTypes::class)`.

**Fix**: Add the annotation to the class declaration.

## AND-021: Namespace Collision Between Modules

**Severity**: Warning

The `app` module and `shared` module MUST have different `namespace` values
in their `build.gradle.kts` files. If they collide, the build emits confusing
warnings or fails.

**Detection**: Compare the `namespace` values in `app/build.gradle.kts` and
`shared/build.gradle.kts`. Flag if they are identical.

**Fix**: Use `com.vectis.{appname}` for `app` and
`com.vectis.{appname}.shared` for `shared`.
