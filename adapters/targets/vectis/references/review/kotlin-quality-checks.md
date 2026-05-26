# Kotlin Quality Checks

Language-level quality checks for Kotlin/Jetpack Compose code in Crux Android shells.

## KTL-001: Force Unwrap / Non-Null Assertion in Production Code

**Severity**: important

No `!!` non-null assertions outside of test files and preview composables. A `!!` on a null value throws `NullPointerException`, crashing the app with no recovery path.

`Core.kt` has two categories of throwing calls, each with a different error-handling pattern:

- **Bincode calls** (`bincodeSerialize()`, `bincodeDeserialize()`) use `try/catch` with `Log.w(TAG, "context", e)` and a safe fallback. These throw generic errors without structured messages.
- **CoreFFI calls** (`coreFfi.update()`, `coreFfi.view()`, `coreFfi.resolve()`) use `try/catch` with `Log.e(TAG, "context: ${e.message}", e)`. These throw `CoreException` containing a meaningful `Bridge` message from the Rust core -- using a generic catch without `e.message` would discard this diagnostic information.

In `initialView()`, view deserialization falls back to `ViewModel.Loading` (no prior state exists). In the `Effect.Render` handler, the existing view must be preserved by returning without assignment -- never fall back to `ViewModel.Loading`, which would overwrite the user's current screen. Event serialization failures use a no-op return (event is dropped). This ensures Debug builds surface type mismatches via logcat while Release builds degrade gracefully.

**Detection**: Search `.kt` files (excluding `*Test.kt`) for `!!`. Skip occurrences inside `@Preview` composables and test files. Flag all other occurrences including those in `Core.kt`. Also flag CoreFFI calls that have no `try/catch` or that use a catch block without `e.message` logging.

**Fix**: Replace `!!` with safe alternatives: `?.let { ... }`, `?: fallback`, `requireNotNull` with a descriptive message (only for true preconditions), or `try/catch` for deserialization. For CoreFFI calls, use `try/catch` with `Log.e(TAG, "context: ${e.message}", e)`. For bincode calls, use `try/catch` with `Log.w(TAG, "context", e)` and a fallback value.

## KTL-002: Debug Output

**Severity**: important

No `println()`, `System.out.println()`, or `e.printStackTrace()` calls in production code. Use `android.util.Log` or a structured logging framework.

**Detection**: Search `.kt` files (excluding test files) for `println(`, `System.out.`, `System.err.`, and `e.printStackTrace()`.

**Fix**: Remove or replace with `Log.d(TAG, ...)` / `Log.e(TAG, ...)`.

## KTL-003: Concurrency Safety

**Severity**: important

Kotlin coroutine best practices for Crux Android shells:

- `Core` must use `Dispatchers.Main.immediate` for the scope (not `Dispatchers.Default`) so UI state updates happen synchronously when already on the main thread.
- Async effect handlers must launch in `scope.launch` (not `GlobalScope`).
- `CancellationException` must always be rethrown in catch blocks.
- Network and I/O operations must use `withContext(Dispatchers.Default)` or `Dispatchers.IO`, not the main dispatcher.

**Detection**: Check for:
- `GlobalScope.launch` anywhere in the shell (should use Core's scoped coroutines).
- `Dispatchers.Default` or `Dispatchers.IO` as the Core scope's main dispatcher.
- Catch blocks that catch `Exception` or `Throwable` without rethrowing `CancellationException`.
- Network calls on `Dispatchers.Main` without switching context.

**Fix**: Use Core's `CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)`. Always rethrow `CancellationException`. Use `withContext(Dispatchers.Default)` for network/IO work.

## KTL-004: State Management

**Severity**: important

Compose state management must follow the Crux shell patterns:

- Simple Core: `Core` extends `androidx.lifecycle.ViewModel`; view is `var view: ViewModel by mutableStateOf(...)` with `private set`.
- Full Core: `Core` is a plain class with `StateFlow<ViewModel>`. Activity collects via `collectAsState()`.
- Composable parameters are immutable (`val`). Screen composables receive view model data as a value and event callbacks as `(Event) -> Unit`.
- Local editing state (text fields) uses `remember { mutableStateOf(...) }`.
- Screen composables should never hold a reference to `Core` directly.

**Detection**: Check for:
- `Core` reference passed directly to screen composables.
- `mutableStateOf` used for view model data inside screen composables (should be a parameter).
- `StateFlow` collected inside a screen composable rather than at the root.
- Missing `private set` on `mutableStateOf` view property.

**Fix**: Correct the state ownership pattern per the Core pattern in use.

## KTL-005: Composable Body Complexity

**Severity**: suggestion

A composable function body should not exceed 60 lines. Complex composables should be decomposed into smaller extracted composables or private helper functions.

**Detection**: Count lines in each `@Composable fun` body block.

**Fix**: Extract sections into private composable functions or separate files.

## KTL-007: Deprecated Compose API Usage

**Severity**: suggestion

Avoid deprecated Jetpack Compose APIs:

- `Scaffold(scaffoldState = ...)` → use `Scaffold()` without scaffold state (Material 3 Scaffold does not take scaffold state)
- `rememberCoroutineScope` inside `LaunchedEffect` → use the `LaunchedEffect` coroutine scope directly
- Old Material 2 imports (`androidx.compose.material.*`) when Material 3 is available (`androidx.compose.material3.*`)

**Detection**: Search for deprecated type and method names.

**Fix**: Replace with the modern Material 3 equivalent.

## KTL-008: Missing Content Description on Icons

**Severity**: important

Decorative icons may use `contentDescription = null`, but interactive icons (inside `IconButton`, `FloatingActionButton`, or `Button`) MUST have a non-null `contentDescription` for accessibility.

**Detection**: Search for `Icon(` inside interactive containers. Flag those with `contentDescription = null`.

**Fix**: Add a descriptive `contentDescription` string.

## KTL-009: Hardcoded Strings in UI

**Severity**: suggestion

User-facing strings should be extracted to string resources for internationalization. For apps intended for localization, use `stringResource(R.string.key)`.

**Detection**: Search for hardcoded string literals in `Text()` calls that are not derived from the view model (e.g., button labels, navigation titles, static headings).

**Fix**: Extract to `res/values/strings.xml` and use `stringResource()`, or note as acceptable if the app is single-language.

## KTL-010: Event Callback Naming

**Severity**: suggestion

Event callback parameters should be named `onEvent` consistently across all screen composables for uniformity.

**Detection**: Check screen composable function signatures for the event callback parameter name. Flag if it is not `onEvent`.

**Fix**: Rename to `onEvent: (Event) -> Unit`.
