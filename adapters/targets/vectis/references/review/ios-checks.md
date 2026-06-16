# iOS Shell Review Checks

Structural and integration checks for Crux iOS shells. Each check has an ID, description, severity, and detection method.

## IOS-001: Missing Screen View for ViewModel Variant

**Severity**: critical

Every variant in the Rust `enum ViewModel` that carries a per-page view struct must have a corresponding SwiftUI screen view file in `Views/`.

**Detection**: Extract ViewModel variants from `app.rs`. For each variant with a payload, verify a `.swift` file exists in `Views/` with a struct that accepts the matching view model type.

**Fix**: Create the missing screen view file following `references/swiftui-view-patterns.md`.

## IOS-002: Missing ContentView Switch Case

**Severity**: critical

The `ContentView` switch on `core.view` must have one case per ViewModel variant. A missing case means the shell cannot render that view.

**Detection**: Count cases in the ContentView switch. Compare against the number of ViewModel variants in `app.rs`.

**Fix**: Add the missing case to the switch, rendering the appropriate screen.

## IOS-003: Missing Effect Handler

**Severity**: critical

Every variant in the Rust `enum Effect` must have a corresponding case in the `processEffect` switch in `Core.swift`. A missing handler means the core's side-effect request will be silently dropped.

**Detection**: Extract Effect variants from `app.rs`. Verify each has a case in the `processEffect` method.

**Fix**: Add the missing effect handler case. See `references/crux-ios-shell-pattern.md` for handler templates.

## IOS-004: Undispatched Shell-Facing Event

**Severity**: important

Every shell-facing Event variant (those without `#[serde(skip)]`) should be dispatched by at least one view. An undispatched event means a user action described in the spec has no UI trigger.

**Detection**: Extract shell-facing Event variants from `app.rs`. Search all `.swift` files for `onEvent(.variantName` or `core.update(.variantName`. Flag variants with zero matches. Exclude `Navigate` as it may be handled via SwiftUI navigation APIs rather than explicit dispatch.

**Fix**: Add the event dispatch to the appropriate screen view.

## IOS-005: Hardcoded Color

**Severity**: important

Views should use `VectisColors` tokens, not hardcoded `Color(...)`, `Color.red`, `Color("name")`, or hex values.

**Detection**: Search `.swift` files for:
- `Color(red:` or `Color(white:`
- `Color("` (asset catalog reference)
- `Color.red`, `Color.blue`, etc. (system colors used as semantic colors)
- Hex color patterns `#[0-9A-Fa-f]{6}`

Exclude system-provided styles (`.tint`, `.accentColor`) and SF Symbol rendering colors.

**Fix**: Replace with the appropriate `VectisColors` token.

## IOS-006: Hardcoded Typography

**Severity**: important

Views should use `VectisTypography` tokens, not inline `.font(.system(size:))`.

**Detection**: Search `.swift` files for `.font(.system(size:` without a preceding `VectisTypography` reference on the same line.

Exclude icon sizing (`.font(.system(size:` on `Image` views) which is acceptable for SF Symbol sizing.

**Fix**: Replace with the appropriate `VectisTypography` token.

## IOS-007: Hardcoded Spacing

**Severity**: important

Padding and spacing values should use `VectisSpacing` tokens, not magic numbers.

**Detection**: Search for `.padding(` or `spacing:` with numeric literals that are not 0. Check that the value matches a token; flag if it does not.

**Fix**: Replace with the appropriate `VectisSpacing` token.

## IOS-008: Missing Preview

**Severity**: suggestion

Every screen view should have a `#Preview` block with sample data for development and visual testing.

**Detection**: For each screen view file in `Views/`, check for a `#Preview` or `PreviewProvider` declaration.

**Fix**: Add a `#Preview` block with sample data at the bottom of the file.

## IOS-009: Missing Accessibility Label

**Severity**: important

Interactive icons (buttons with only an `Image` label, no `Text`) must have an `accessibilityLabel`.

**Detection**: Search for `Button { ... } label: { Image(systemName:` patterns without a corresponding `.accessibilityLabel` modifier.

**Fix**: Add `.accessibilityLabel("description")` to the Image or Button.

## IOS-010: Route/Navigation Mismatch

**Severity**: important

If the Rust core defines a `Route` enum, the iOS shell should implement navigation that covers all Route variants.

**Detection**: Extract Route variants from `app.rs`. Verify the shell dispatches `Event.navigate(route)` for each variant via navigation controls (tabs, buttons, links).

**Fix**: Add navigation elements for missing Route variants.

## IOS-011: Core Not @MainActor

**Severity**: critical

The `Core` class must be annotated with `@MainActor` to ensure all UI updates happen on the main thread.

**Detection**: Check for `@MainActor` annotation on the `Core` class declaration.

**Fix**: Add `@MainActor` to the class declaration.

## IOS-012: Core Not ObservableObject

**Severity**: critical

The `Core` class must conform to `ObservableObject` and publish the view model via `@Published var view: ViewModel`.

**Detection**: Check the `Core` class declaration for `ObservableObject` conformance and `@Published` on the `view` property.

**Fix**: Add `ObservableObject` conformance and `@Published` annotation.

## IOS-013: Force Try in Core.swift

**Severity**: important

`Core.swift` must not use `try!` for bincode serialization. FFI type mismatches (e.g., after regenerating the core without updating Swift types) should degrade gracefully rather than crash the app.

**Detection**: Search `Core.swift` for `try!`. Flag all occurrences.

**Fix**: Replace with `try?` guarded by `assertionFailure` and a safe fallback value. Use the `deserializeView` and `processEffects` helper pattern from `references/crux-ios-shell-pattern.md`.

## IOS-014: Bare Task in Core.swift

**Severity**: important

Async effect handlers in `Core.swift` must use `Task { @MainActor in ... }`, not bare `Task { ... }`. While Swift 6 inherits actor isolation for `Task.init` in `@MainActor` context, the explicit annotation is required for clarity, cross-version safety, and resilience to refactoring.

**Detection**: Search `Core.swift` for `Task {` or `Task{` that is not immediately followed by `@MainActor in`. Flag all occurrences in `processEffect` method branches.

**Fix**: Replace `Task {` with `Task { @MainActor in`.

## IOS-015: CoreFFI Errors Not Surfaced

**Severity**: critical

`CoreFFI` methods (`view()`, `update()`, `resolve()`) return `Result<Vec<u8>, CoreError>` in Rust, which UniFFI maps to Swift `throws`. Calling these without `try` is a compile error. Unlike bincode serialization (IOS-013), CoreFFI calls throw `CoreError` which contains a meaningful `Bridge` message from the Rust core. Using `try?` discards this message. All CoreFFI calls must use `do/catch` with `\(error)` interpolated into `assertionFailure` so the underlying reason (deserialization failure, invalid effect ID, etc.) is visible in debug builds.

**Detection**: Search `Core.swift` for `core.view()`, `core.update(`, and `core.resolve(` that are not preceded by `try`. Also flag any CoreFFI calls that use `try?` instead of `do/catch` -- the error message is lost.

**Fix**: Wrap each CoreFFI call in `do { let x = try core.xxx(...); ... } catch { assertionFailure("context: \(error)") }`. In `init()`, use `do { self.view = Self.deserializeView(try core.view()) } catch { ... }`. In the `.render` effect handler, use an inline `do/catch` that preserves the existing view on failure. See `references/crux-ios-shell-pattern.md`.

## IOS-016: Render Effect Overwrites View with Loading Fallback

**Severity**: important

The `.render` effect handler must not use `deserializeView` or any pattern that falls back to `.loading` on failure. This would overwrite the user's current view (e.g., a task list) with a loading screen on any transient serialization error. The `deserializeView` helper is only appropriate in `init()` where no prior state exists.

**Detection**: In the `processEffect` method, check the `.render` case for calls to `deserializeView` or any assignment of `.loading` to `self.view`.

**Fix**: Replace with an inline `do/catch` that preserves the existing view and surfaces the `CoreError` message:

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

## IOS-017: Interactive Controls Inside ScrollView in NavigationStack

**Severity**: important

`TextField` and small `Button` elements inside a `ScrollView` that sits within a `NavigationStack` suffer from tap suppression. The underlying `UIScrollView` sets `delaysContentTouches = true`, which delays touch delivery to non-`UIButton` views. Users experience this as controls requiring a long press or double tap to activate.

**Detection**: Search screen view files for a `ScrollView` nested inside a `NavigationStack` (or vice versa). Within that `ScrollView` body, flag any `TextField` or `Button` that is not inside a `List`. Ignore `Button` with `.buttonStyle(.borderedProminent)` or `.buttonStyle(.bordered)` as these map to `UIButton` and are not affected.

**Fix**: Move the interactive control outside the `ScrollView` using `.safeAreaInset(edge:)`, or replace the `ScrollView` with `List` which manages `delaysContentTouches` internally. See `ios-writer/references/swiftui-view-patterns.md` for examples.

## IOS-018: Nested ScrollViews with Tappable Content

**Severity**: important

A horizontal `ScrollView` (e.g. a chip row or filter bar) nested inside a vertical `ScrollView` creates compound gesture recognizer conflicts. The inner and outer `UIScrollView` pan gestures compete, causing missed taps on elements within the inner scroll and erratic scroll behavior.

**Detection**: Search screen view files for a `ScrollView(.horizontal` that appears inside a vertical `ScrollView` (or inside a `VStack`/`LazyVStack` within a vertical `ScrollView`). Flag if the inner `ScrollView` contains tappable elements (`Button`, `.onTapGesture`, `NavigationLink`) that do not use `.buttonStyle(.plain)`.

**Fix**: Move the inner horizontal scrollable outside the outer `ScrollView` using `.safeAreaInset(edge:)`. If nesting is unavoidable, ensure all tappable elements use `Button` with `.buttonStyle(.plain)`. See `ios-writer/references/swiftui-view-patterns.md` for examples.

## IOS-019: Recurring Composition Group Without Component Directive

**Severity**: suggestion

Per the component directive contract and reviewer surface, any `group` shape that visibly recurs across `composition.yaml` (≥2 instances on the same screen, or ≥2 instances across different screens) without a `component: <slug>` directive is a candidate for promotion to a named component. Without the directive, the iOS shell ends up with parallel inline copies of the same SwiftUI subtree across `Views/*.swift` files; when the layout changes the operator must hand-edit every copy, and drift compounds silently. The reviewer flags candidate slugs for the operator to evaluate; promotion itself remains an authoring decision (it requires editing `composition.yaml` and adding a sibling `iOS/<App>/Components/<Slug>.swift` file via `vectis:ios-writer`).

**Detection**: When the wired `composition.yaml` is available (sibling at the change-local or baseline path — see SKILL.md "Gather context"):

1. Walk the composition tree collecting every `group` node.
2. Compute a structural skeleton for each group (the same `*-when` presence + nested-item-kind shape that `specify tool run vectis -- validate composition` uses for the §G structural-identity rule).
3. Group instances by skeleton equality. For any skeleton that appears in ≥2 instances **without** a sibling `component:` directive on any of those instances, flag the recurrence as a candidate component.
4. Cross-check the iOS shell: if the recurring composition group already corresponds to an extracted SwiftUI sub-view under `iOS/<App>/Components/`, downgrade severity to `optional` and note the existing extraction; otherwise emit at the canonical `suggestion` severity (the operator will promote both surfaces in lockstep).

When `composition.yaml` is absent (composition-less change, or a change that only touches `app.rs` types), skip this check entirely — there is no source-of-truth recurrence signal in shell code alone.

**Fix**: This is a candidate finding, not a defect. Suggest one of two actions and let the operator pick:

1. **Promote to component.** Add `component: <slug>` to the recurring group(s) in `composition.yaml` (kebab-case slug; not a reserved region name like `header` / `body` / `footer` / `fab`); regenerate the iOS shell via `vectis:ios-writer`; the writer emits a single `iOS/<App>/Components/<Slug>.swift` view and rewrites every call site to use it (per the component directive contract).
2. **Accept the inline duplication.** When the recurring group is intentionally distinct (e.g. two visually similar groups that diverge in a way the skeleton check cannot see — different gesture handling, different state semantics), document the divergence in the composition or in `design.md` and accept the finding.

## IOS-020: Vector Or Raster Asset Rendered As Platform Symbol

**Severity**: important

**Codex**: `rule_id: VECTIS-006`

Per the render-by-`kind` contract ([`ios/design-system-integration.md`](../ios/design-system-integration.md) § Asset integration), composition-referenced ids whose `assets.yaml` entry is `vector` or `raster` must emit `Image("<id>")` from a shell-local imageset copied from the materialized export — never `Image(systemName:)` or another SF Symbol substitute.

**Detection**: When the wired `composition.yaml` and effective `assets.yaml` are available (slice-local path first, then `design-system/assets.yaml` — same precedence as the iOS writer):

1. Walk the composition tree and collect every asset id referenced by `icon`, `image`, `icon-button`, or `fab` items.
2. Resolve each id in the effective `assets.yaml` and retain only those with `kind: vector` or `kind: raster`.
3. For each retained id, verify the iOS shell emits `Image("<id>")` (or an equivalent asset-catalog reference to `<id>.imageset`) in the screen views that render that composition node.
4. Flag any case where the same visual role uses `Image(systemName:` (or `Label` / `Button` labels built from `Image(systemName:`) without a matching `kind: symbol` entry for that composition id.
5. Do **not** flag `Image(systemName:` when the composition id resolves to `kind: symbol`.

When `composition.yaml` or `assets.yaml` is absent, skip this check — there is no cross-artifact signal.

**Fix**: Regenerate the affected screen via `vectis:ios-writer` after `vectis materialize assets` has populated `design-system/assets/exports/ios/` (or after operator pins are in place). If the glyph is genuinely platform-native, change the `assets.yaml` entry to `kind: symbol` with `symbols.ios` / `symbols.android` and update composition to reference the symbol id — do not leave a `vector` / `raster` entry while the shell still substitutes SF Symbols.
