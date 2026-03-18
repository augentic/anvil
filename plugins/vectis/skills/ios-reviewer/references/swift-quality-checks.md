# Swift Quality Checks

Language-level quality checks for Swift/SwiftUI code in Crux iOS shells.

## SWF-001: Force Unwrap in Production Code

**Severity**: Warning

No `!` force unwraps outside of test files and `Core.swift` Bincode
serialization (where `try!` on `bincodeSerialize`/`bincodeDeserialize` is
acceptable for well-formed types).

**Detection**: Search `.swift` files (excluding `*Tests.swift` and
`*Previews.swift`) for `!` used as force unwrap (not `!=` or `!==`).
In `Core.swift`, allow `try!` on `bincodeSerialize()` and
`bincodeDeserialize(input:)` calls only.

**Fix**: Replace with `guard let`, `if let`, or `??` with a default value.

## SWF-002: Debug Output

**Severity**: Warning

No `print()`, `debugPrint()`, or `dump()` calls in production code.

**Detection**: Search `.swift` files (excluding test files) for `print(`,
`debugPrint(`, `dump(` at the start of a line or after whitespace.

**Fix**: Remove or replace with `os.Logger` if logging is needed.

## SWF-003: Concurrency Safety

**Severity**: Warning

With Swift 6 strict concurrency:
- `Core` must be `@MainActor`.
- Views that capture `Core` must not pass it across isolation boundaries.
- `Task { ... }` closures in `@MainActor` context inherit main actor isolation.

**Detection**: Check for:
- `nonisolated` on methods that access `@Published` properties.
- `@Sendable` closures that capture non-`Sendable` types.
- `DispatchQueue.main.async` (should use `@MainActor` instead).

**Fix**: Use `@MainActor` annotation and structured concurrency patterns.

## SWF-004: State Management

**Severity**: Warning

SwiftUI state management must follow these rules:
- `Core` is owned by the app entry point as `@StateObject`.
- Views receive `Core` as `@ObservedObject`, never `@StateObject`.
- Local editing state (text fields, toggles) uses `@State`.
- View model data is passed as `let` properties, never `@State` or `@Binding`.

**Detection**: Check for:
- `@StateObject` on `Core` in views other than the app entry point.
- `@State` on properties that hold view model data.
- `@Binding` on per-page view struct properties (should be `let`).

**Fix**: Correct the property wrapper usage.

## SWF-005: View Body Complexity

**Severity**: Info

A view's `body` computed property should not exceed 50 lines. Complex views
should be split into extracted sub-views or helper methods.

**Detection**: Count lines in each `var body: some View { ... }` block.

**Fix**: Extract sections into private sub-view properties or separate
view structs.

## SWF-006: Missing VectisDesign Import

**Severity**: Warning

Every `.swift` file in `Views/` that uses design system tokens must import
`VectisDesign`.

**Detection**: Search for `VectisColors`, `VectisTypography`, `VectisSpacing`,
or `VectisCornerRadius` usage without a corresponding `import VectisDesign`.

**Fix**: Add `import VectisDesign` at the top of the file.

## SWF-007: Deprecated API Usage

**Severity**: Info

Avoid deprecated SwiftUI APIs:
- `NavigationView` → use `NavigationStack` (iOS 16+)
- `.onChange(of:perform:)` single-parameter → use two-parameter version (iOS 17+)
- `PreviewProvider` → use `#Preview` macro (iOS 17+)

**Detection**: Search for deprecated type and method names.

**Fix**: Replace with the modern equivalent.

## SWF-008: Missing Content Transition

**Severity**: Info

Text that updates frequently (counters, status labels) benefits from
`.contentTransition(.numericText())` for smooth animations.

**Detection**: Identify `Text` views bound to frequently-changing view model
fields (counts, timestamps). Check for `.contentTransition` modifier.

**Fix**: Add `.contentTransition(.numericText())` for numeric text.

## SWF-009: Hardcoded Strings

**Severity**: Info

User-facing strings should be localizable. For apps intended for
internationalization, use `String(localized:)` or `LocalizedStringKey`.

**Detection**: Search for hardcoded string literals in `Text()` calls that
are not derived from the view model (e.g., button labels, navigation titles).

**Fix**: Replace with `String(localized: "key")` or note as acceptable if
the app is single-language.

## SWF-010: Event Callback Naming

**Severity**: Info

Event callback closures should be named `onEvent` consistently across all
screen views for uniformity.

**Detection**: Check screen view structs for the event callback property name.
Flag if it is not `onEvent`.

**Fix**: Rename to `onEvent: (Event) -> Void`.
