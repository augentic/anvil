---
name: vectis-ios-writer
description: Generate or update a SwiftUI iOS shell for a Crux application from Specify artifacts. Use when implementing iOS shell tasks from a Specify change, or when the user mentions ios-writer.
argument-hint: "<change-dir>"
---

# Crux iOS Shell Generator

Generate or update a buildable SwiftUI iOS shell for an existing Crux core application. The shell renders the core's `ViewModel`, dispatches `Event` values from user interactions, and handles platform side-effects (HTTP, KV, SSE) on behalf of the core.

The iOS writer reads `tokens.yaml`, `assets.yaml`, and `composition.yaml` directly and emits **shell-local** theme + asset catalog code inside the iOS shell tree (RFC-11 §L "Generated layout"). There is no shared Swift Package, no `import VectisDesign`, and no path back into `design-system/ios/` from the rendered shell project. When `tokens.yaml` is absent, the writer falls back to platform-native HIG defaults (RFC-11 §F "Fallback policy belongs to shell writers"). See [`references/design-system-integration.md`](references/design-system-integration.md) for the full integration contract and [`references/swift-token-templates.md`](references/swift-token-templates.md) for the per-category Swift code templates.

When an existing iOS shell is detected, the skill operates in **update mode**: it compares the current `app.rs` types against the existing Swift code and makes targeted edits rather than regenerating from scratch.

When no iOS shell exists yet, the skill runs `specify vectis add-shell ios --dir {app-dir}` to scaffold the project. The CLI owns `iOS/project.yml`, `iOS/Makefile`, the Inject SPM wiring, the `{AppName}App.swift` entry point, a render-only `Core.swift` with CAP markers, a baseline `ContentView.swift`, and the starter `Views/LoadingScreen.swift` / `Views/HomeScreen.swift`. The writer adds `Theme/`, `Components/`, and `Resources/Assets.xcassets/` on first generation when the corresponding inputs exist. Once the scaffold exists this skill switches to **update mode** and layers spec-driven changes over the generated baseline.

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
- cargo-swift (must be compatible with the pinned UniFFI contract; run `specify vectis verify` to check) -- builds the Rust static library as a Swift Package with XCFramework

## Input Analysis

The ios-writer reads the Crux core source to determine what the shell must render and handle. Read `{app-dir}/shared/src/app.rs` and extract:

| Extract | Source | Maps to |
|---|---|---|
| App struct name | `impl App for X` | App entry point name (UniFFI generates `CoreFfi` in Swift) |
| ViewModel variants | `enum ViewModel` | ContentView switch cases, one screen per variant |
| Per-page view structs | Structs wrapped by ViewModel variants | Screen view properties and layout |
| Shell-facing Event variants | `enum Event` (non-`#[serde(skip)]`) | User interaction handlers in screens |
| Effect variants | `enum Effect` | `processEffect` switch cases in Core.swift |
| Route variants | `enum Route` | Navigation destinations |
| Supporting types | Structs/enums used in view structs | Display data types |
| Screen regions | `composition.yaml` `header`, `body`, `footer`, `fab` | View structure (NavigationTitle + toolbar, content, bottom toolbar, overlay button) |
| Container structure | `composition.yaml` `group` nodes with `direction`, `gap`, `align`, `justify` | `HStack`/`VStack`/`ZStack` with spacing and alignment |
| Sizing | `composition.yaml` `size` on groups and items (`fill`, `hug`, fixed) | `.frame(maxWidth: .infinity)`, intrinsic sizing, explicit dimensions |
| Surface decoration | `composition.yaml` `background`, `corner_radius`, `elevation` on groups | Styled container views with background, cornerRadius, shadow |
| Field bindings | `composition.yaml` `bind` keys on items | Property bindings in views |
| Event wiring | `composition.yaml` `event` keys on items | `onEvent()` interaction handlers |
| Token references | `composition.yaml` `style`, `color`, `gap`, `padding` | Shell-local Theme types: `VectisTypography.*` / `VectisColors.*` / `VectisSpacing.*` (defined under `iOS/<App>/Theme/`, generated from `tokens.yaml`) |
| Component directive | `composition.yaml` `group.component: <slug>` | One named SwiftUI `View` per slug under `iOS/<App>/Components/`, PascalCased (`task-row` → `TaskRow`) |
| Asset references | `composition.yaml` `image:` / `icon:` / `icon-button:` / `fab:` resolved through `assets.yaml` | `Image("<asset-id>")` for raster / vector copied into `iOS/<App>/Resources/Assets.xcassets/`; `Image(systemName: "<sf-symbol>")` for `kind: symbol` entries |
| Conditional rendering | `composition.yaml` `states` and `*-when` keys | `if`/`switch` in view code |
| Iteration | `composition.yaml` `list.each` / `grid.each` + `item` keys | `ForEach` / `List` / `LazyVStack` |

Also read:
- `{app-dir}/shared/src/lib.rs` -- custom capability modules
- `{app-dir}/shared/Cargo.toml` -- capability dependencies
- `tokens.yaml` -- design tokens for styling. Resolution order: change-local `{change-dir}/tokens.yaml`, then project-level `design-system/tokens.yaml`. When neither exists the writer falls back to platform-native HIG defaults (see [`references/design-system-integration.md`](references/design-system-integration.md)).
- `assets.yaml` plus referenced files -- asset inventory for image / icon / symbol references in `composition.yaml`. Resolution order: change-local `{change-dir}/assets.yaml` plus `{change-dir}/assets/`, then `design-system/assets.yaml` plus `design-system/assets/`. The writer copies referenced files into `iOS/<App>/Resources/Assets.xcassets/` per the copy-on-generate rule (RFC-11 §E, §I).

When `change-dir` is provided, also read:
- `{change-dir}/specs/{feature-name}/spec.md` -- read the `## iOS Shell Requirements` section for platform-specific behavioral requirements (navigation style, gestures, haptics, accessibility). Also read the `## iOS Shell Details` section of `{change-dir}/design.md` for platform design decisions.
- `{change-dir}/composition.yaml` or `.specify/specs/composition.yaml` -- wired composition artifact for deterministic layout instructions. Cross-artifact reference checks (every `bind` / `event` / token / asset reference resolves) are owned by `specify vectis validate composition`; the writer consumes the already-validated input set rather than re-validating at generation time.

## Mode Detection

- **Create Mode** -- `{project-dir}/` does **not** exist. The skill invokes `specify vectis add-shell ios` to scaffold the baseline, then proceeds directly into Update Mode to apply feature-specific changes from the Specify artifacts.
- **Update Mode** -- `{project-dir}/` **does** exist and contains `.swift` files. Read existing code, diff against the core, and make targeted edits (steps U1--U8 below).

Detection rule: check for `{project-dir}/*/Core.swift`. If present, switch to update mode. If not, run:

```bash
specify vectis add-shell ios --dir {app-dir}
```

`{app-dir}` is the parent directory of `shared/`; the CLI derives the `iOS/` sibling directory automatically. On non-zero exit, surface the CLI's structured error output to the user and stop -- do **not** attempt to hand-author `project.yml`, `Makefile`, or any of the baseline `.swift` files.

If the command succeeds, switch to Update Mode. The scaffolded shell is a render-only baseline with CAP markers in `Core.swift` (one per optional capability: `http`, `kv`, `time`, `platform`, `sse`) and starter screens for `Loading` + `Home`. Update Mode replaces CAP markers and starter screens with real effect handlers and per-ViewModel-variant screen views derived from the current `app.rs`.

## Verification ownership

When the orchestrator passes `skip_verification: true`, the writer stops after code generation and does **not** run step U8. The orchestrator's dedicated iOS verify sub-agent handles formatting, `make build`, and `make sim-build` with its own repair loop and iteration limits.

When invoked **standalone** (no `skip_verification` flag, or `skip_verification: false`), the writer runs its full process including step U8.

---

## Process: Create Mode

Use this process when no iOS shell exists at `{project-dir}`. The CLI owns all iOS boilerplate (`project.yml`, `Makefile`, entry point, render-only baseline `Core.swift`, `ContentView.swift`, `Views/LoadingScreen.swift`, `Views/HomeScreen.swift`, and Inject/XcodeGen wiring). This skill's only Create-Mode responsibilities are: (1) read the Crux core to derive the app name and capability set, (2) invoke the CLI, (3) switch to Update Mode.

### 1. Read the Crux core

Read `{app-dir}/shared/src/app.rs` and extract all types listed in the Input Analysis table above. In particular, derive the App struct name (used by the CLI to name the Xcode target, directory, and entry point file) and note which capabilities the core actually uses -- this drives which CAP markers Update Mode must replace in the scaffolded `Core.swift`. If `app.rs` cannot be read or parsed, report the error and stop.

### 2. Invoke the CLI

Run:

```bash
specify vectis add-shell ios --dir {app-dir}
```

The CLI derives the app name from `shared/Cargo.toml` / `app.rs` and produces `iOS/project.yml`, `iOS/Makefile`, `iOS/{AppName}/{AppName}App.swift`, `iOS/{AppName}/Core.swift` (with CAP markers for every optional capability), `iOS/{AppName}/ContentView.swift`, and the starter `iOS/{AppName}/Views/{Loading,Home}Screen.swift`. The output is structured JSON. On non-zero exit, surface the CLI's error output to the user and stop.

### 3. Switch to Update Mode

After the CLI returns green, treat the scaffolded iOS shell as an existing implementation and execute **Process: Update Mode** below to:

- Strip CAP markers for capabilities the core does not use, and expand CAP blocks (with real effect handlers + helpers) for capabilities the core does use.
- Replace the `HomeScreen` starter with real per-ViewModel-variant screen files driven by the core's `ViewModel` enum + per-page view structs.
- Rewrite `ContentView.swift`'s `switch` to cover every ViewModel variant.
- Apply any `## iOS Shell Requirements` from the active Specify change (when `change-dir` is provided).

## Process: Update Mode

Use this process when `{project-dir}/` already exists with Swift files.

### U1. Read and analyze the Crux core

Same as create mode step 1 (read `{app-dir}/shared/src/app.rs` and extract the full type inventory using the Input Analysis table above).

When `change-dir` is provided, also read the `## iOS Shell Requirements` section from `{change-dir}/specs/{feature-name}/spec.md` and the `## iOS Shell Details` section from `{change-dir}/design.md` for platform-specific requirements.

### U2. Read existing Swift code

Read all `.swift` files in `{project-dir}/{AppName}/`:

- `Core.swift` -- current effect handler switch cases
- `ContentView.swift` -- current ViewModel switch cases
- `Views/*.swift` -- current screen views
- `{AppName}App.swift` -- app entry point

Also check for existing Inject integration: look for `import Inject` and `@ObserveInjection` in view files. Record whether Inject is already present so step U6 knows whether to add it.

### U3. Build implementation inventory

Extract from existing Swift code:

| Category | What to extract |
|---|---|
| Effect handlers | Cases in `processEffect` switch |
| ViewModel cases | Cases in `ContentView` switch |
| Screen views | `.swift` files in `Views/` |
| Component views | `.swift` files in `Components/` (one per `component: <slug>`) |
| Theme files | `.swift` files in `Theme/` (one per `tokens.yaml` category, plus `Theme.swift`) |
| Asset catalog entries | Subdirectories of `Resources/Assets.xcassets/` (one per `assets.yaml` entry of `kind: raster` or `kind: vector`) |
| Event dispatches | All `onEvent(...)` calls |
| Design system usage | `VectisColors`, `VectisTypography`, `VectisSpacing`, `VectisCornerRadius` references; presence or absence of `import VectisDesign` (legacy — must be removed) |
| Inject integration | `import Inject`, `@ObserveInjection`, `.enableInjection()` per view |

### U4. Diff analysis

Compare the Rust core types (from U1) and the input artifacts (`tokens.yaml`, `assets.yaml`, `composition.yaml`) against the Swift inventory (from U3). For each category, classify items as Added, Removed, Modified, or Unchanged.

Walk through in this order:

1. **Effect variants** -- new or removed capabilities affect Core.swift.
2. **ViewModel variants** -- new or removed views affect ContentView and screen view files.
3. **Per-page view struct fields** -- changed display data affects screen views.
4. **Event variants** -- new or removed user actions affect screen views.
5. **Route variants** -- new or removed navigation destinations affect navigation code.
6. **Token categories** -- categories added / removed / changed in `tokens.yaml` map to files added / removed / rewritten under `Theme/` (per [`references/swift-token-templates.md`](references/swift-token-templates.md)). Categories absent from `tokens.yaml` follow the HIG fallback in [`references/design-system-integration.md`](references/design-system-integration.md).
7. **Asset entries** -- entries added / removed / changed in `assets.yaml` map to copies / deletions / re-copies under `Resources/Assets.xcassets/` per the copy-on-generate rule.
8. **Component directives** -- `component: <slug>` entries in `composition.yaml` map to files added / removed / refreshed under `Components/`.
9. **Legacy VectisDesign references** -- any `import VectisDesign` line, any `package: VectisDesign` entry in `project.yml`, and any `path: ../../../design-system/ios` package declaration are migration debt and MUST be removed (RFC-11 §L "Compatibility policy"); the shell-local `Theme/` files now satisfy the same role.

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
- If Inject is missing from any view file (including `ContentView.swift`, `{AppName}App.swift`, and all screen views), add the boilerplate: `import Inject`, `@ObserveInjection var inject` property, and `.enableInjection()` as the outermost body modifier.
- Remove any lingering `import VectisDesign` lines from screen / component / app entry-point files. Token references (`VectisColors.*`, `VectisSpacing.*`, etc.) keep working unchanged because they now resolve to the shell-local `Theme/` files in the same target.

### U6a. Refresh `Theme/` from `tokens.yaml`

When `tokens.yaml` is present, regenerate `iOS/<App>/Theme/` from the YAML per [`references/swift-token-templates.md`](references/swift-token-templates.md): one Swift file per token category (with `spacing` / `cornerRadius` colocated in `Theme/Spacing.swift`), plus the structural `Theme/Theme.swift` scaffold. Files carry the `// Generated from design-system/tokens.yaml — do not edit manually.` header (except `Theme.swift`) so a subsequent regeneration overwrites them safely. The header always uses the canonical project-level path `design-system/tokens.yaml` regardless of whether the current generation reads from a change-local file — this keeps the header stable across the change lifecycle and avoids breaking header-based detection of generated files.

When a category is removed from `tokens.yaml`, delete the corresponding file under `Theme/`. Files that lack the "Generated from" header are operator-owned and must be preserved. When `tokens.yaml` is **absent** the writer skips this step entirely and follows the HIG fallback policy from [`references/design-system-integration.md`](references/design-system-integration.md).

### U6b. Refresh `Components/` from `composition.yaml`

For every `group` carrying `component: <slug>` (RFC-11 §G, §I), emit one named SwiftUI `View` under `iOS/<App>/Components/<PascalCaseSlug>.swift`. Props are inferred from variation observed across instances of the slug: `bind`, `event`, `error`, `asset`, token references, `*-when` keys, and free text content that differ across instances become parameters; values constant across all instances are baked into the view body. The structural-identity rule is enforced by `specify vectis validate composition` before this skill runs, so the writer can trust every instance of the slug shares the same skeleton.

When a slug disappears from `composition.yaml`, delete the corresponding `Components/<PascalCaseSlug>.swift` file and rewrite each former call site to inline the group body. See [`references/design-system-integration.md`](references/design-system-integration.md) for a worked `task-row` example.

### U6c. Refresh `Resources/Assets.xcassets/` from `assets.yaml`

For every `assets.yaml` entry referenced from `composition.yaml`, copy the per-platform iOS source(s) into `iOS/<App>/Resources/Assets.xcassets/<asset-id>.imageset/` (raster or vector). `kind: symbol` entries do not generate a catalog entry — emit `Image(systemName: "<sf-symbol>")` at the call site instead. The canonical `source:` SVG (when present in a vector entry) is provenance only and is **never** copied into the shell tree.

When an entry is removed from `assets.yaml` or its references are removed from `composition.yaml`, delete the corresponding generated catalog entry. Operator-authored entries (e.g. `AppIcon.appiconset/`) are preserved.

Missing iOS exports for `vector` entries referenced from `composition.yaml` are validation errors, not deferred TODOs (RFC-11 §E "Missing vector exports"). If the validator reports a missing export the writer halts shell generation for the affected screen and surfaces the validator output verbatim — it does **not** silently fall back to the canonical SVG, generate a placeholder, or skip the screen.

### U7. Update build configuration

- Update `project.yml` if new dependencies are needed.
- Update `Makefile` if build targets changed.
- If `project.yml` lacks the `Inject` SPM package, add it along with the `- package: Inject` target dependency, Debug-only `OTHER_LDFLAGS` (`["-w", "-Xlinker", "-interposable"]`), and `EMIT_FRONTEND_COMMAND_LINES: "YES"` in the Debug config.
- Remove any legacy `VectisDesign` package declaration (`packages: VectisDesign: { path: ../../../design-system/ios }`) and the matching `- package: VectisDesign` target dependency. The shell-local `Theme/` files satisfy the same role without an external Swift Package; leaving the entries causes XcodeGen to fail when `design-system/ios/` no longer exists post-Phase 4.1.

### U8. Format and verify

1. Run `swiftformat` on modified files.
2. Run `make build` to verify compilation (the CLI-generated `Makefile` runs the three-phase `typegen -> package -> xcode` pipeline).
3. Run `make sim-build` to verify the project compiles for the iOS Simulator.
4. Fix any build errors.

## Composition Mapping Priority

When `composition.yaml` is present, the region structure and group container tree take precedence over convention-based inference for view body composition:

- **Groups** map to SwiftUI stacks: `direction: row` → `HStack(spacing:)`, `direction: column` → `VStack(spacing:)`, `direction: stack` → `ZStack`.
- **Sizing** maps to `.frame()` modifiers: `fill` → `.frame(maxWidth: .infinity)`, fixed values → `.frame(width:)` / `.frame(height:)`.
- **Surface decoration** maps to styled container views: `background` → `.background()`, `corner_radius` → `.cornerRadius()` or `.clipShape(RoundedRectangle())`, `elevation` → `.shadow()`.
- **Platform-specific overrides**: When `composition.yaml` contains `platforms.ios` region overrides for a screen, use those in preference to the shared regions.

When `composition.yaml` is absent, the existing inference behavior is unchanged — this preserves backward compatibility for pre-RFC-7 projects.

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
2. **Preserve custom styling** that the developer added beyond the design system defaults.
3. **Preserve custom view logic** (e.g., animations, gestures) that is not driven by the ViewModel.
4. **Preserve `#Preview` blocks** on unchanged views.
5. **Preserve `project.yml` customizations** (signing, entitlements, custom build phases).
6. **Preserve `Makefile` customizations** (additional targets, environment variables).

## Reference Documentation

| Reference | Purpose |
|---|---|
| `references/crux-ios-shell-pattern.md` | Core.swift template, effect handling, serialization protocol |
| `references/swiftui-view-patterns.md` | Screen patterns, lists, forms, navigation, accessibility |
| `references/design-system-integration.md` | Shell-local theme + asset integration: generated layout under `iOS/<App>/Theme/`, HIG fallback when `tokens.yaml` is absent, asset copy-on-generate rules, component-directive contract |
| `references/swift-token-templates.md` | Concrete Swift code templates per token category (color, typography, scalar, border, theme bundle); migrated from `design-system-writer` per RFC-11 §J |

XcodeGen `project.yml`, the `Makefile` pipeline, and all baseline shell scaffolding (`project.yml` packages, Inject SPM wiring, CAP markers, starter screens) are owned by the CLI's embedded templates in the [`augentic/specify-cli`](https://github.com/augentic/specify-cli) repo (`<specify-cli>/crates/vectis/src/init/ios.rs` and `<specify-cli>/templates/vectis/ios/`). Do not hand-edit those files in Create Mode; let `specify vectis add-shell ios` write them and then modify in Update Mode.

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
| Build fails with missing types | Verify `uniffi` is pinned to the expected version (run `specify vectis update-versions --dry-run` to inspect). Run `specify vectis verify` to detect mismatches |
| Build fails on `import VectisDesign` | Legacy migration debt — the shell now emits theme code under `iOS/<App>/Theme/` (RFC-11 §L). Drop the `import` line and the `package: VectisDesign` entry from `project.yml`; existing token references (`VectisColors.*`, `VectisSpacing.*`, etc.) keep working because the Theme files live in the same target |
| `specify vectis validate composition` reports unresolved token / asset reference | The composition is referencing a token or asset id that is not declared in `tokens.yaml` / `assets.yaml`. The writer halts; the operator must either add the missing entry or remove the reference (RFC-11 §F, §I "Validation gate") |
| Missing iOS export for a `kind: vector` asset | Per RFC-11 §E, this is an error and not a deferred TODO. Halt shell generation for the affected screen and report the missing `sources.ios` field |

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
- [ ] App entry point applies `.vectisTheme()` when `tokens.yaml` is present (the modifier is defined in the shell-local `Theme/Theme.swift`); HIG-fallback shells omit the modifier

### Design System

When `tokens.yaml` is **present**:

- [ ] `iOS/<App>/Theme/` exists with one Swift file per token category plus `Theme.swift`
- [ ] All color references use `VectisColors` (no hardcoded hex)
- [ ] All font references use `VectisTypography` (no inline `.system(size:)`)
- [ ] All spacing values use `VectisSpacing` (no magic numbers)
- [ ] All corner radius values use `VectisCornerRadius`
- [ ] No `import VectisDesign` lines anywhere in the shell tree
- [ ] No `package: VectisDesign` entry in `project.yml`
- [ ] No `path: ../../../design-system/ios` reference in `project.yml`

When `tokens.yaml` is **absent** (HIG fallback path):

- [ ] No `iOS/<App>/Theme/` directory generated
- [ ] Color references use SwiftUI semantic colors (`.primary`, `.secondary`, `.accentColor`, `Color(.systemBackground)`)
- [ ] Font references use `Font.system(.body)` etc.
- [ ] No hardcoded hex via `Color(red:green:blue:)` or `Color("named-token")`

### Assets

- [ ] Every asset id referenced by `composition.yaml` resolves to an entry in `assets.yaml`
- [ ] Every referenced raster / vector entry has a corresponding `<asset-id>.imageset/` under `iOS/<App>/Resources/Assets.xcassets/`
- [ ] No `path: ../../../design-system/assets` reference in `project.yml`
- [ ] `kind: symbol` entries are rendered via `Image(systemName: "<sf-symbol>")` at the call site (no catalog entry)

### Components

- [ ] Every `component: <slug>` directive in `composition.yaml` has a corresponding `Components/<PascalCaseSlug>.swift` file
- [ ] Every call site of the slug uses the named `View` (e.g. `TaskRow(...)`), not an inlined group body
- [ ] Slugs that no longer appear in `composition.yaml` have their `Components/<PascalCaseSlug>.swift` files deleted

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
- [ ] No `TextField` or small `Button` inside a `ScrollView` within a `NavigationStack` -- use `.safeAreaInset(edge:)` to pin interactive controls outside the scroll content, or use `List`
- [ ] No horizontal `ScrollView` nested inside a vertical `ScrollView` with tappable content -- use `.safeAreaInset(edge:)` for the inner scrollable, or ensure tappable elements use `Button` with `.buttonStyle(.plain)`

## Important Notes

- **Core only must exist first**: This skill generates the iOS shell for an existing Crux core. Run the core-writer skill first to generate the `shared` crate.
- **Shell is thin**: All business logic lives in the Rust core. The shell only renders views and performs platform I/O. Never add business logic to Swift code.
- **UniFFI bridging**: The shared crate must have `crate-type = ["staticlib"]` and the `uniffi` feature gate. The ios-writer assumes this is already configured by the core-writer. The `uniffi` crate pin must be compatible with the UniFFI contract expected by `cargo-swift` and the `crux_core` bundled bindgen (strict version equality is not required; cargo-swift 0.11 can read uniffi 0.29.x metadata). Run `specify vectis verify` to detect mismatches.
- **Generated types**: Two Swift packages are produced: `SharedTypes` (domain types via facet_typegen) and `Shared` (UniFFI bindings + XCFramework via cargo-swift).
- **Hot reloading**: All generated shells include the [Inject](https://github.com/krzysztofzablocki/Inject) library for hot reloading during development. Inject is a no-op in Release builds (stripped by LLVM), so the boilerplate can remain permanently. Each developer must install [InjectionIII](https://github.com/nicklama/InjectionIII/releases) separately. The CLI wires Inject into `project.yml` (SPM package + Debug-only `OTHER_LDFLAGS: -Xlinker -interposable` + `EMIT_FRONTEND_COMMAND_LINES: YES`); Update Mode only has to add `@ObserveInjection`/`.enableInjection()` to new screen views.
- **ScrollView interaction hazards**: Do not place `TextField` or small `Button` elements inside a `ScrollView` within a `NavigationStack`. The `UIScrollView` touch-delay mechanism (`delaysContentTouches`) suppresses taps on non-`UIButton` views. Use `.safeAreaInset(edge:)` to pin interactive controls outside the scroll content, or use `List` which handles this internally. Similarly, avoid nesting a horizontal `ScrollView` (e.g. chip row) inside a vertical `ScrollView` -- the compound gesture conflicts cause missed taps. Pin the inner scrollable with `.safeAreaInset`, or ensure all tappable elements use `Button` with `.buttonStyle(.plain)`. See `references/swiftui-view-patterns.md` for examples.
- **Specify integration**: When `change-dir` is provided, the skill reads the `## iOS Shell Requirements` section from the feature spec and the `## iOS Shell Details` section from design.md. The primary input remains `app.rs` from the core; the feature spec's platform section supplements with requirements that may not be expressed in the Rust types alone (e.g., navigation style, specific UX behaviors, accessibility requirements, layout constraints).
