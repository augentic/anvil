# iOS Shell Review Checks

Structural and integration checks for Crux iOS shells. Each check has an ID,
description, severity, and detection method.

## IOS-001: Missing Screen View for ViewModel Variant

**Severity**: Critical

Every variant in the Rust `enum ViewModel` that carries a per-page view struct
must have a corresponding SwiftUI screen view file in `Views/`.

**Detection**: Extract ViewModel variants from `app.rs`. For each variant with
a payload, verify a `.swift` file exists in `Views/` with a struct that accepts
the matching view model type.

**Fix**: Create the missing screen view file following
`references/swiftui-view-patterns.md`.

## IOS-002: Missing ContentView Switch Case

**Severity**: Critical

The `ContentView` switch on `core.view` must have one case per ViewModel
variant. A missing case means the shell cannot render that view.

**Detection**: Count cases in the ContentView switch. Compare against the
number of ViewModel variants in `app.rs`.

**Fix**: Add the missing case to the switch, rendering the appropriate screen.

## IOS-003: Missing Effect Handler

**Severity**: Critical

Every variant in the Rust `enum Effect` must have a corresponding case in the
`processEffect` switch in `Core.swift`. A missing handler means the core's
side-effect request will be silently dropped.

**Detection**: Extract Effect variants from `app.rs`. Verify each has a case
in the `processEffect` method.

**Fix**: Add the missing effect handler case. See
`references/crux-ios-shell-pattern.md` for handler templates.

## IOS-004: Undispatched Shell-Facing Event

**Severity**: Warning

Every shell-facing Event variant (those without `#[serde(skip)]`) should be
dispatched by at least one view. An undispatched event means a user action
described in the spec has no UI trigger.

**Detection**: Extract shell-facing Event variants from `app.rs`. Search all
`.swift` files for `onEvent(.variantName` or `core.update(.variantName`. Flag
variants with zero matches. Exclude `Navigate` as it may be handled via
SwiftUI navigation APIs rather than explicit dispatch.

**Fix**: Add the event dispatch to the appropriate screen view.

## IOS-005: Hardcoded Color

**Severity**: Warning

Views should use `VectisColors` tokens, not hardcoded `Color(...)`,
`Color.red`, `Color("name")`, or hex values.

**Detection**: Search `.swift` files for:
- `Color(red:` or `Color(white:`
- `Color("` (asset catalog reference)
- `Color.red`, `Color.blue`, etc. (system colors used as semantic colors)
- Hex color patterns `#[0-9A-Fa-f]{6}`

Exclude system-provided styles (`.tint`, `.accentColor`) and SF Symbol
rendering colors.

**Fix**: Replace with the appropriate `VectisColors` token.

## IOS-006: Hardcoded Typography

**Severity**: Warning

Views should use `VectisTypography` tokens, not inline `.font(.system(size:))`.

**Detection**: Search `.swift` files for `.font(.system(size:` without a
preceding `VectisTypography` reference on the same line.

Exclude icon sizing (`.font(.system(size:` on `Image` views) which is
acceptable for SF Symbol sizing.

**Fix**: Replace with the appropriate `VectisTypography` token.

## IOS-007: Hardcoded Spacing

**Severity**: Warning

Padding and spacing values should use `VectisSpacing` tokens, not magic
numbers.

**Detection**: Search for `.padding(` or `spacing:` with numeric literals
that are not 0. Check that the value matches a token; flag if it does not.

**Fix**: Replace with the appropriate `VectisSpacing` token.

## IOS-008: Missing Preview

**Severity**: Info

Every screen view should have a `#Preview` block with sample data for
development and visual testing.

**Detection**: For each screen view file in `Views/`, check for a `#Preview`
or `PreviewProvider` declaration.

**Fix**: Add a `#Preview` block with sample data at the bottom of the file.

## IOS-009: Missing Accessibility Label

**Severity**: Warning

Interactive icons (buttons with only an `Image` label, no `Text`) must have
an `accessibilityLabel`.

**Detection**: Search for `Button { ... } label: { Image(systemName:` patterns
without a corresponding `.accessibilityLabel` modifier.

**Fix**: Add `.accessibilityLabel("description")` to the Image or Button.

## IOS-010: Route/Navigation Mismatch

**Severity**: Warning

If the Rust core defines a `Route` enum, the iOS shell should implement
navigation that covers all Route variants.

**Detection**: Extract Route variants from `app.rs`. Verify the shell
dispatches `Event.navigate(route)` for each variant via navigation controls
(tabs, buttons, links).

**Fix**: Add navigation elements for missing Route variants.

## IOS-011: Core Not @MainActor

**Severity**: Critical

The `Core` class must be annotated with `@MainActor` to ensure all UI updates
happen on the main thread.

**Detection**: Check for `@MainActor` annotation on the `Core` class declaration.

**Fix**: Add `@MainActor` to the class declaration.

## IOS-012: Core Not ObservableObject

**Severity**: Critical

The `Core` class must conform to `ObservableObject` and publish the view model
via `@Published var view: ViewModel`.

**Detection**: Check the `Core` class declaration for `ObservableObject`
conformance and `@Published` on the `view` property.

**Fix**: Add `ObservableObject` conformance and `@Published` annotation.

## IOS-013: Force Try in Core.swift

**Severity**: Warning

`Core.swift` must not use `try!` for bincode serialization. FFI type
mismatches (e.g., after regenerating the core without updating Swift types)
should degrade gracefully rather than crash the app.

**Detection**: Search `Core.swift` for `try!`. Flag all occurrences.

**Fix**: Replace with `try?` guarded by `assertionFailure` and a safe
fallback value. Use the `deserializeView` and `processEffects` helper
pattern from `references/crux-ios-shell-pattern.md`.

## IOS-014: Bare Task in Core.swift

**Severity**: Warning

Async effect handlers in `Core.swift` must use `Task { @MainActor in ... }`,
not bare `Task { ... }`. While Swift 6 inherits actor isolation for
`Task.init` in `@MainActor` context, the explicit annotation is required
for clarity, cross-version safety, and resilience to refactoring.

**Detection**: Search `Core.swift` for `Task {` or `Task{` that is not
immediately followed by `@MainActor in`. Flag all occurrences in
`processEffect` method branches.

**Fix**: Replace `Task {` with `Task { @MainActor in`.

## IOS-015: CoreFFI Errors Not Surfaced

**Severity**: Critical

`CoreFFI` methods (`view()`, `update()`, `resolve()`) return
`Result<Vec<u8>, CoreError>` in Rust, which UniFFI maps to Swift `throws`.
Calling these without `try` is a compile error. Unlike bincode
serialization (IOS-013), CoreFFI calls throw `CoreError` which contains a
meaningful `Bridge` message from the Rust core. Using `try?` discards this
message. All CoreFFI calls must use `do/catch` with `\(error)` interpolated
into `assertionFailure` so the underlying reason (deserialization failure,
invalid effect ID, etc.) is visible in debug builds.

**Detection**: Search `Core.swift` for `core.view()`, `core.update(`,
and `core.resolve(` that are not preceded by `try`. Also flag any CoreFFI
calls that use `try?` instead of `do/catch` -- the error message is lost.

**Fix**: Wrap each CoreFFI call in `do { let x = try core.xxx(...); ... }
catch { assertionFailure("context: \(error)") }`. In `init()`, use
`do { self.view = Self.deserializeView(try core.view()) } catch { ... }`.
In the `.render` effect handler, use an inline `do/catch` that preserves
the existing view on failure. See `references/crux-ios-shell-pattern.md`.

## IOS-016: Render Effect Overwrites View with Loading Fallback

**Severity**: Warning

The `.render` effect handler must not use `deserializeView` or any
pattern that falls back to `.loading` on failure. This would overwrite the
user's current view (e.g., a task list) with a loading screen on any transient
serialization error. The `deserializeView` helper is only appropriate in
`init()` where no prior state exists.

**Detection**: In the `processEffect` method, check the `.render` case for
calls to `deserializeView` or any assignment of `.loading` to `self.view`.

**Fix**: Replace with an inline `do/catch` that preserves the existing view
and surfaces the `CoreError` message:

```swift
case .render:
    do {
        let data = try core.view()
        guard let vm = try? ViewModel.bincodeDeserialize(input: [UInt8](data)) else {
            assertionFailure("Failed to deserialize ViewModel from bincode")
            break
        }
        self.view = vm
    } catch {
        assertionFailure("Failed to get view from core: \(error)")
    }
```

See `references/crux-ios-shell-pattern.md`.
