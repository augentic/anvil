---
name: ios-writer
description: Generate or update a SwiftUI iOS shell for a Crux application from Specify artifacts. Use when implementing iOS shell tasks from a Specify change, or when the user mentions ios-writer.
---

# Crux iOS Shell Generator

Generate or update a buildable SwiftUI iOS shell for an existing Crux core
application. The shell renders the core's `ViewModel`, dispatches `Event`
values from user interactions, and handles platform side-effects (HTTP, KV,
SSE) on behalf of the core.

When an existing iOS shell is detected, the skill operates in **update mode**:
it compares the current `app.rs` types against the existing Swift code and
makes targeted edits rather than regenerating from scratch.

This skill targets **Swift 6** and **SwiftUI** with iOS 17+ deployment target.

## Arguments

| Argument | Required | Description |
|---|---|---|
| `app-dir` | **Yes** | Path to the Crux app directory (must contain `shared/src/app.rs`) |
| `project-dir` | No | Directory for the iOS shell. Defaults to `{app-dir}/iOS` |
| `change-dir` | No | Path to `.specify/changes/<change>/`. When provided, the skill reads the `## iOS Shell Requirements` section from `{change-dir}/specs/{feature-name}/spec.md` for platform-specific requirements |

## Prerequisites

The following tools must be installed (see README.md for installation):

- Xcode command line tools
- xcode-build-server
- xcbeautify
- swiftformat
- XcodeGen
- cargo-swift (v0.9.0) -- builds the Rust static library as a Swift Package with XCFramework

## Input Analysis

The ios-writer reads the Crux core source to determine what the shell must
render and handle. Read `{app-dir}/shared/src/app.rs` and extract:

| Extract | Source | Maps to |
|---|---|---|
| App struct name | `impl App for X` | App entry point name (UniFFI generates `CoreFfi` in Swift) |
| ViewModel variants | `enum ViewModel` | ContentView switch cases, one screen per variant |
| Per-page view structs | Structs wrapped by ViewModel variants | Screen view properties and layout |
| Shell-facing Event variants | `enum Event` (non-`#[serde(skip)]`) | User interaction handlers in screens |
| Effect variants | `enum Effect` | `processEffect` switch cases in Core.swift |
| Route variants | `enum Route` | Navigation destinations |
| Supporting types | Structs/enums used in view structs | Display data types |

Also read:
- `{app-dir}/shared/src/lib.rs` -- custom capability modules
- `{app-dir}/shared/Cargo.toml` -- capability dependencies
- `design-system/tokens.yaml` -- design tokens for styling
- `design-system/spec.md` -- design system usage rules

When `change-dir` is provided, also read:
- `{change-dir}/specs/{feature-name}/spec.md` -- read the `## iOS Shell Requirements`
  section for platform-specific behavioral requirements (navigation style, gestures,
  haptics, accessibility). Also read the `## iOS Shell Details` section of
  `{change-dir}/design.md` for platform design decisions.

## Mode Detection

- **Create Mode** -- `{project-dir}/` does **not** exist. Generate the entire
  iOS shell from scratch (steps 1--11 below).
- **Update Mode** -- `{project-dir}/` **does** exist and contains `.swift` files.
  Read existing code, diff against the core, and make targeted edits
  (steps U1--U8 below).

Check for `{project-dir}/Core.swift` or `{project-dir}/*/Core.swift` to
detect the mode. If found, switch to update mode.

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

### 2. Read the design system

Read `design-system/tokens.yaml` for color, typography, spacing, and corner
radius values. Read `design-system/spec.md` for usage rules.

If the design system files do not exist, generate views without design system
imports and note this in the output.

### 3. Determine app name

Derive the app name from the `App` struct in `app.rs`:

| Rust struct | App name | Directory name |
|---|---|---|
| `TodoApp` | `Todo` | `Todo` |
| `CounterApp` | `Counter` | `Counter` |
| `NoteEditor` | `NoteEditor` | `NoteEditor` |

The app name is used for the Xcode target, directory, and entry point file.

### 4. Generate directory structure

Create the following directories under `{project-dir}`:

```
{project-dir}/
    {AppName}/
        Views/
```

### 5. Generate `project.yml`

Create `{project-dir}/project.yml` following the template in
`references/ios-project-config.md`. Key adaptations:

- Set the project name to `{AppName}`
- Set `bundleIdPrefix` based on the app name
- Calculate the relative path from `{project-dir}` to `design-system/ios/`
  for the VectisDesign package reference
- Use Swift Packages (NOT `projectReferences` / `cargo xcode`):

```yaml
packages:
  SharedTypes:
    path: ./generated/SharedTypes    # Domain types from codegen
  Shared:
    path: ./generated/Shared         # UniFFI bindings from cargo-swift
  VectisDesign:
    path: ../../../design-system/ios # adjust relative path as needed
  Inject:
    url: https://github.com/krzysztofzablocki/Inject.git
    from: "1.5.2"
targets:
  {AppName}:
    dependencies:
      - package: SharedTypes
      - package: Shared
      - package: VectisDesign
      - package: Inject
```

- Include Debug-only settings: `OTHER_LDFLAGS` with `-Xlinker -interposable`
  and `EMIT_FRONTEND_COMMAND_LINES: "YES"` (required for InjectionIII)

### 6. Generate `Makefile`

Create `{project-dir}/Makefile` with the three-phase build pipeline.
Replace `{AppName}` with the actual app name.

The build has three sequential phases:

**Phase 1: typegen** -- Generate SharedTypes (domain types as a Swift package):

```makefile
typegen:
	@echo "Generating SharedTypes..."
	@RUST_LOG=info cargo run --manifest-path $(SHARED_DIR)/Cargo.toml \
		--bin codegen --features codegen,facet_typegen -- \
		--language swift --output-dir generated
```

The codegen binary uses `TypeRegistry` from facet to extract types at compile
time. It creates a Swift package at `generated/SharedTypes/`. No separate
`cargo build` step is needed -- the `cargo run` command compiles what it needs.

**Phase 2: package** -- Build Shared (UniFFI bindings as a Swift package):

```makefile
package:
	@echo "Building Shared Swift package..."
	@cd $(SHARED_DIR) && \
		cargo swift package --name Shared --platforms ios \
			--lib-type static --features uniffi && \
		rm -rf ../iOS/generated/Shared && \
		mkdir -p ../iOS/generated/Shared && \
		cp -r Shared/* ../iOS/generated/Shared/ && \
		rm -rf Shared
```

**Phase 3: xcode** -- Generate the Xcode project:

```makefile
xcode:
	@echo "Generating Xcode project..."
	@xcodegen
```

See `references/ios-project-config.md` for the complete Makefile template
including `sim-build` and `clean` targets.

### 7. Generate `Core.swift`

Create `{project-dir}/{AppName}/Core.swift` following the pattern in
`references/crux-ios-shell-pattern.md`.

`Core.swift` must import both `Shared` (for `CoreFfi`) and `SharedTypes`
(for domain types like `ViewModel`, `Event`, `Request`).

#### Effect handlers

The `processEffect` switch must have one case per Effect variant:

| Effect variant | Handler |
|---|---|
| `Render` | Always included. Updates `@Published var view`. |
| `Http` | Include if `Effect::Http(HttpRequest)` exists. Uses `URLSession`. |
| `KeyValue` | Include if `Effect::KeyValue(KeyValueOperation)` exists. Uses `UserDefaults` or file storage. |
| `ServerSentEvents` | Include if a custom SSE effect exists. Uses async stream. |
| `Time` | Include if `Effect::Time(TimeRequest)` exists. Uses `Task.sleep`. |
| `Platform` | Include if `Effect::Platform(PlatformRequest)` exists. Returns `UIDevice` info. |

Include only the effect handlers that the app actually uses.

#### KV Types (crux_kv)

When generating the KeyValue handler, use these generated types:

- `KeyValueOperation` with cases `.get(key:)`, `.set(key:value:)`,
  `.delete(key:)`, `.exists(key:)`, `.listKeys(prefix:cursor:)`
- `KeyValueResult` with `.ok(response:)` and `.err(error:)`
- `KeyValueResponse` with `.get(value:)`, `.set(previous:)`,
  `.delete(previous:)`, `.exists(isPresent:)`, `.listKeys(keys:nextCursor:)`
- `Value` enum: `.none` / `.bytes([UInt8])` (NOT Swift Optional)

Add HTTP helper functions if the HTTP capability is present. See
`references/crux-ios-shell-pattern.md` for the full implementation.

### 8. Generate `ContentView.swift`

Create `{project-dir}/{AppName}/ContentView.swift` following the pattern in
`references/swiftui-view-patterns.md`.

The view body must be a `switch` on `core.view` with one case per
ViewModel variant. Each case renders the corresponding screen view,
passing the per-page view struct and an event callback.

### 9. Generate screen views

For each ViewModel variant, create a screen view file in
`{project-dir}/{AppName}/Views/`:

| ViewModel variant | Screen file | Content |
|---|---|---|
| `Loading` | `LoadingScreen.swift` | `ProgressView` with "Loading..." text |
| `Main(MainView)` | `MainScreen.swift` | Layout driven by `MainView` fields |
| `Error(ErrorView)` | `ErrorScreen.swift` | Error message with optional retry |
| `{Name}({NameView})` | `{Name}Screen.swift` | Layout driven by `{NameView}` fields |

For each screen:

1. Import `SwiftUI`, `SharedTypes`, `VectisDesign`, and `Inject`.
2. Accept the per-page view struct as a `let` property.
3. Accept `let onEvent: (Event) -> Void` for user interactions.
4. Use VectisDesign tokens for all colors, fonts, and spacing.
5. Map each shell-facing Event variant that is relevant to this view to a
   user interaction (button tap, swipe action, pull-to-refresh, etc.).
6. Add a `#Preview` with sample data at the bottom of the file.
7. Add `accessibilityLabel` to interactive icons.
8. Add `@ObserveInjection var inject` property and `.enableInjection()` as
   the outermost modifier in the body (for hot reloading support).

Consult `references/swiftui-view-patterns.md` for layout patterns (lists,
forms, navigation, swipe actions, pull-to-refresh).

Consult `references/design-system-integration.md` for token usage.

### 10. Generate app entry point

Create `{project-dir}/{AppName}/{AppName}App.swift`:

```swift
import Inject
import SwiftUI
import VectisDesign

@main
struct {AppName}App: App {
    @StateObject private var core = Core()
    @ObserveInjection var inject

    var body: some Scene {
        WindowGroup {
            ContentView(core: core)
                .vectisTheme()
        }
    }
}
```

### 11. Format and verify

1. Run `swiftformat {project-dir}/{AppName}/` to format all generated Swift files.
2. Run `make build` in `{project-dir}` to run the full pipeline (typegen → package → xcode).
3. Run `make sim-build` to verify the project compiles for the iOS Simulator.
4. If the build fails, read the error output, fix the issue, and re-run.

## Process: Update Mode

Use this process when `{project-dir}/` already exists with Swift files.

### U1. Read and analyze the Crux core

Same as create mode step 1. Extract all types from the current `app.rs`.

When `change-dir` is provided, also read the `## iOS Shell Requirements` section
from `{change-dir}/specs/{feature-name}/spec.md` and the `## iOS Shell Details`
section from `{change-dir}/design.md` for platform-specific requirements.

### U2. Read existing Swift code

Read all `.swift` files in `{project-dir}/{AppName}/`:

- `Core.swift` -- current effect handler switch cases
- `ContentView.swift` -- current ViewModel switch cases
- `Views/*.swift` -- current screen views
- `{AppName}App.swift` -- app entry point

Also check for existing Inject integration: look for `import Inject` and
`@ObserveInjection` in view files. Record whether Inject is already present
so step U6 knows whether to add it.

### U3. Build implementation inventory

Extract from existing Swift code:

| Category | What to extract |
|---|---|
| Effect handlers | Cases in `processEffect` switch |
| ViewModel cases | Cases in `ContentView` switch |
| Screen views | `.swift` files in `Views/` |
| Event dispatches | All `onEvent(...)` calls |
| Design system usage | `VectisColors`, `VectisTypography`, `VectisSpacing` references |
| Inject integration | `import Inject`, `@ObserveInjection`, `.enableInjection()` per view |

### U4. Diff analysis

Compare the Rust core types (from U1) against the Swift inventory (from U3).
For each category, classify items as Added, Removed, Modified, or Unchanged.

Walk through in this order:

1. **Effect variants** -- new or removed capabilities affect Core.swift.
2. **ViewModel variants** -- new or removed views affect ContentView and
   screen view files.
3. **Per-page view struct fields** -- changed display data affects screen views.
4. **Event variants** -- new or removed user actions affect screen views.
5. **Route variants** -- new or removed navigation destinations affect
   navigation code.

Output the diff summary before making edits.

### U5. Apply changes to Core.swift

- Add new effect handler cases for added capabilities.
- Remove effect handler cases for removed capabilities.
- Add or remove HTTP/KV/SSE helper functions as needed.

### U6. Apply changes to views

- Add new screen view files for added ViewModel variants.
- Remove screen view files for removed ViewModel variants.
- Update ContentView.swift switch to add/remove cases.
- Update existing screen views for changed per-page view struct fields.
- Add/remove event dispatch calls for changed Event variants.
- If Inject is missing from any view file (including `ContentView.swift`,
  `{AppName}App.swift`, and all screen views), add the boilerplate:
  `import Inject`, `@ObserveInjection var inject` property, and
  `.enableInjection()` as the outermost body modifier.

### U7. Update build configuration

- Update `project.yml` if new dependencies are needed.
- Update `Makefile` if build targets changed.
- If `project.yml` lacks the `Inject` SPM package, add it along with the
  `- package: Inject` target dependency, Debug-only `OTHER_LDFLAGS`
  (`["-w", "-Xlinker", "-interposable"]`), and
  `EMIT_FRONTEND_COMMAND_LINES: "YES"` in the Debug config.

### U8. Format and verify

Same as create mode step 11:

1. Run `swiftformat` on modified files.
2. Run `make build` to verify compilation.
3. Fix any build errors.

## Spec-to-Code Mapping

| Rust Type (in `app.rs`) | Swift Artifact | File |
|---|---|---|
| `enum ViewModel { Loading, Main(MainView) }` | `switch core.view { case .loading: ... case .main(let vm): ... }` | `ContentView.swift` |
| ViewModel variant `Main(MainView)` | `struct MainScreen: View` | `Views/MainScreen.swift` |
| `struct MainView { pub items: Vec<ItemView> }` | Screen properties: `let viewModel: MainView` | `Views/MainScreen.swift` |
| Shell-facing `Event::AddItem(String)` | `onEvent(.addItem(text))` | Relevant screen view |
| `Effect::Http(HttpRequest)` | `case .http(let req): Task { @MainActor in ... }` | `Core.swift` |
| `enum Route { Main, Settings }` | Navigation tabs or stack paths | `ContentView.swift` |

## Preservation Rules (Update Mode)

1. **Never regenerate a file from scratch.** Make targeted edits.
2. **Preserve custom styling** that the developer added beyond the design
   system defaults.
3. **Preserve custom view logic** (e.g., animations, gestures) that is not
   driven by the ViewModel.
4. **Preserve `#Preview` blocks** on unchanged views.
5. **Preserve `project.yml` customizations** (signing, entitlements, custom
   build phases).
6. **Preserve `Makefile` customizations** (additional targets, environment
   variables).

## Reference Documentation

| Reference | Purpose |
|---|---|
| `references/crux-ios-shell-pattern.md` | Core.swift template, effect handling, serialization protocol |
| `references/swiftui-view-patterns.md` | Screen patterns, lists, forms, navigation, accessibility |
| `references/ios-project-config.md` | XcodeGen project.yml, Makefile, build configuration |
| `references/design-system-integration.md` | VectisDesign token usage in views |

## Examples

| Example | Capabilities | Demonstrates |
|---|---|---|
| `references/examples/01-simple-counter-ios.md` | Render | Minimal shell, Core.swift, two screens, project setup |
| `references/examples/02-http-counter-ios.md` | Render + HTTP | Async HTTP handling, error view, three screens |

## Error Handling

| Error | Resolution |
|---|---|
| `app.rs` not found | Verify `app-dir` points to a Crux app with `shared/src/app.rs` |
| Unknown Effect variant | Add a placeholder `case` with a `fatalError("unhandled")` and report |
| `xcodegen` fails | Check `project.yml` syntax; verify path references |
| Build fails with missing types | Verify `uniffi` is pinned to `"=0.29.4"` in `shared/Cargo.toml`, matching the version bundled in `crux_core::cli::bindgen` |
| VectisDesign not found | Check package path in `project.yml` relative to `{project-dir}` |

## Verification Checklist

### Build

- [ ] `make setup` completes without errors
- [ ] `make build` compiles the iOS app for simulator
- [ ] `swiftformat --lint` reports no formatting issues

### Structure

- [ ] Every ViewModel variant has a corresponding screen view file
- [ ] Every ViewModel variant has a case in ContentView switch
- [ ] Every Effect variant has a case in `processEffect` switch
- [ ] Every shell-facing Event variant is dispatched by at least one view
- [ ] `Core.swift` is `@MainActor` and `ObservableObject`
- [ ] App entry point uses `@StateObject` for the core
- [ ] App entry point applies `.vectisTheme()`

### Design System

- [ ] All color references use `VectisColors` (no hardcoded hex)
- [ ] All font references use `VectisTypography` (no inline `.system(size:)`)
- [ ] All spacing values use `VectisSpacing` (no magic numbers)
- [ ] All corner radius values use `VectisCornerRadius`

### Hot Reloading

- [ ] `project.yml` includes `Inject` SPM package
- [ ] `project.yml` Debug config has `OTHER_LDFLAGS` with `-Xlinker -interposable`
- [ ] `project.yml` Debug config has `EMIT_FRONTEND_COMMAND_LINES: "YES"`
- [ ] Every view (including ContentView and app entry point) has `import Inject`
- [ ] Every view struct has `@ObserveInjection var inject`
- [ ] Every view body ends with `.enableInjection()`

### Quality

- [ ] Every screen view has a `#Preview` with sample data
- [ ] Interactive icons have `accessibilityLabel`
- [ ] No force unwraps (`!`) or force try (`try!`) in production code
- [ ] Bincode serialization failures use `assertionFailure` + safe fallback, not `try!`
- [ ] CoreFFI calls (`core.view()`, `core.update()`, `core.resolve()`) use `do/catch` with `assertionFailure` including `\(error)`
- [ ] `.render` effect handler preserves existing view on failure (inline guard + break, not `deserializeView`)
- [ ] Async effect handlers use `Task { @MainActor in }`, not bare `Task { }`
- [ ] Swift strict concurrency checking enabled (`SWIFT_STRICT_CONCURRENCY: complete`)

## Important Notes

- **Core only must exist first**: This skill generates the iOS shell for an
  existing Crux core. Run the core-writer skill first to generate the
  `shared` crate.
- **Shell is thin**: All business logic lives in the Rust core. The shell
  only renders views and performs platform I/O. Never add business logic
  to Swift code.
- **UniFFI bridging**: The shared crate must have `crate-type = ["staticlib"]`
  and the `uniffi` feature gate. The ios-writer assumes this is already
  configured by the core-writer. The `uniffi` crate must be pinned to
  `"=0.29.4"` to match `crux_core::cli::bindgen`'s bundled `uniffi_bindgen`.
- **Generated types**: Two Swift packages are produced: `SharedTypes` (domain
  types via facet_typegen) and `Shared` (UniFFI bindings + XCFramework via
  cargo-swift).
- **Hot reloading**: All generated shells include the
  [Inject](https://github.com/krzysztofzablocki/Inject) library for hot
  reloading during development. Inject is a no-op in Release builds (stripped
  by LLVM), so the boilerplate can remain permanently. Each developer must
  install [InjectionIII](https://github.com/nicklama/InjectionIII/releases)
  separately -- see `references/ios-project-config.md` for setup details.
- **Specify integration**: When `change-dir` is provided, the skill reads
  the `## iOS Shell Requirements` section from the feature spec and the
  `## iOS Shell Details` section from design.md. The primary input remains
  `app.rs` from the core; the feature spec's platform section supplements
  with requirements that may not be expressed in the Rust types alone
  (e.g., navigation style, specific UX behaviors, accessibility
  requirements, layout constraints).
