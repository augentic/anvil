# iOS Writer Runbook

Operational detail for `vectis-ios-writer`. The SKILL.md keeps only the orientation surface (Critical Path + Reference table + Guardrails); everything procedural lives here.

## Prerequisites

`{app-dir}` must contain `shared/src/app.rs`. `{project-dir}` defaults to `{app-dir}/iOS`. When `{slice-dir}` is supplied, the writer reads the `## iOS Shell Requirements` section from `{slice-dir}/specs/{feature-name}/spec.md` for platform-specific requirements.

Required tools (see README.md for installation): Xcode command line tools, xcode-build-server, xcbeautify, swiftformat, XcodeGen, cargo-swift (must be compatible with the pinned UniFFI contract; `make package` and `make xcode` surface mismatches) — builds the Rust static library as a Swift Package with XCFramework.

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

Also read: `{app-dir}/shared/src/lib.rs` (custom capability modules); `{app-dir}/shared/Cargo.toml` (capability dependencies); `tokens.yaml` (resolution: change-local `{change-dir}/tokens.yaml`, then project-level `design-system/tokens.yaml`; absent → HIG fallback per [`design-system-integration.md`](design-system-integration.md)); `assets.yaml` plus referenced files (resolution: change-local then project-level; copied into `iOS/<App>/Resources/Assets.xcassets/` per the copy-on-generate rule in [`design-system-integration.md`](design-system-integration.md)).

When `slice-dir` is provided also read `{slice-dir}/specs/{feature-name}/spec.md` `## iOS Shell Requirements`, `{slice-dir}/design.md` `## iOS Shell Details`, and `{slice-dir}/composition.yaml` or `.specify/specs/composition.yaml` (cross-artifact reference checks are owned by `specify tool run vectis -- validate composition`; the writer consumes the already-validated input set).

## Mode Detection

- **Create Mode** — `{project-dir}/` does **not** exist. The skill invokes `specify tool run vectis -- scaffold ios` to render the baseline, runs explicit iOS host checks, then proceeds directly into Update Mode.
- **Update Mode** — `{project-dir}/` exists with `.swift` files. Read existing code, diff against the core, apply targeted edits (steps U1–U8 below).

Detection rule: check for `{project-dir}/*/Core.swift`. If present, switch to update mode. If not, run:

```bash
cd {app-dir}
specify tool run vectis -- scaffold ios {AppName} [--caps {caps}]
cd {project-dir}
make typegen
make package
make xcode
```

`{app-dir}` is the parent of `shared/`; `{project-dir}` is normally `{app-dir}/iOS`. `{AppName}` and `{caps}` are derived from the core. On scaffold failure, surface the tool's structured output and stop — do **not** hand-author `project.yml`, `Makefile`, or any baseline `.swift` files. For each host command, record `name`, `passed`, and a failure snippet; stop on the first failed step.

If the scaffold and host checks succeed, switch to Update Mode. The scaffolded shell is a render-only baseline with CAP markers in `Core.swift` (one per optional capability: `http`, `kv`, `time`, `platform`, `sse`) and starter screens for `Loading` + `Home`. Update Mode replaces CAP markers and starter screens with real effect handlers and per-ViewModel-variant screen views derived from the current `app.rs`.

## Verification ownership

When the orchestrator passes `skip_verification: true`, the writer stops after code generation and does **not** run step U8. The orchestrator's dedicated iOS verify sub-agent handles formatting, `make typegen`, `make package`, and `make xcode` with its own repair loop and iteration limits. When invoked **standalone** (no flag, or `false`), the writer runs its full process including U8.

## Process: Create Mode

Use this process when no iOS shell exists at `{project-dir}`. The scaffold tool owns all render-only iOS boilerplate (`project.yml`, `Makefile`, entry point, render-only baseline `Core.swift`, `ContentView.swift`, `Views/LoadingScreen.swift`, `Views/HomeScreen.swift`, and Inject/XcodeGen wiring). This skill's Create-Mode responsibilities are: (1) read the Crux core to derive app name and capability set, (2) invoke the scaffold tool, (3) run host checks, (4) switch to Update Mode.

1. **Read the Crux core.** Read `{app-dir}/shared/src/app.rs` and extract all types listed in the Input Analysis table above. Derive the App struct name (used by the scaffold to name the Xcode target, directory, and entry point file) and note which capabilities the core actually uses — this drives which CAP markers Update Mode must replace in the scaffolded `Core.swift`. If `app.rs` cannot be read or parsed, report and stop.

2. **Invoke the scaffold tool and host checks.** Run the command in Mode Detection above. The writer derives the app name from `shared/Cargo.toml` / `app.rs`; the scaffold produces `iOS/project.yml`, `iOS/Makefile`, `iOS/{AppName}/{AppName}App.swift`, `iOS/{AppName}/Core.swift` (with CAP markers for every optional capability), `iOS/{AppName}/ContentView.swift`, and the starter `iOS/{AppName}/Views/{Loading,Home}Screen.swift`. On non-zero exit, surface tool output and stop. For host steps, return `name`, `passed`, and any failure snippet.

3. **Switch to Update Mode.** After the scaffold and host checks return green, treat the scaffolded iOS shell as an existing implementation and execute Update Mode below to: strip CAP markers for capabilities the core does not use; expand CAP blocks (with real effect handlers + helpers) for capabilities the core does use; replace the `HomeScreen` starter with real per-ViewModel-variant screen files driven by the core's `ViewModel` enum + per-page view structs; rewrite `ContentView.swift`'s `switch` to cover every ViewModel variant; apply any `## iOS Shell Requirements` from the active Specify change.

## Process: Update Mode

Use this process when `{project-dir}/` already exists with Swift files.

**U1. Read and analyze the Crux core.** Same as create-mode step 1 (read `{app-dir}/shared/src/app.rs` and extract the full type inventory using the Input Analysis table above). When `slice-dir` is provided, also read the `## iOS Shell Requirements` section from `{slice-dir}/specs/{feature-name}/spec.md` and the `## iOS Shell Details` section from `{slice-dir}/design.md` for platform-specific requirements.

**U2. Read existing Swift code.** Read all `.swift` files in `{project-dir}/{AppName}/`:

- `Core.swift` — current effect handler switch cases
- `ContentView.swift` — current ViewModel switch cases
- `Views/*.swift` — current screen views
- `{AppName}App.swift` — app entry point

Also check for existing Inject integration: look for `import Inject` and `@ObserveInjection` in view files. Record whether Inject is already present so step U6 knows whether to add it.

**U3. Build implementation inventory.** Extract from existing Swift:

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

**U4. Diff analysis.** Compare the Rust core types (U1) and input artifacts (`tokens.yaml`, `assets.yaml`, `composition.yaml`) against the Swift inventory (U3). Classify Added / Removed / Modified / Unchanged. Walk in order: Effect variants → ViewModel variants → per-page view struct fields → Event variants → Route variants → Token categories → Asset entries → Component directives. Output the diff summary before making edits.

**U5. Apply changes to Core.swift.** Add new effect handler cases for added capabilities; remove cases for removed capabilities; add or remove HTTP/KV/SSE helper functions as needed.

**U6. Apply changes to views.** Add new screen view files for added ViewModel variants; remove screen view files for removed variants; update ContentView.swift switch; update existing screen views for changed per-page view struct fields; add/remove event dispatch calls; if Inject is missing from any view file (including `ContentView.swift`, `{AppName}App.swift`, and all screen views), add the boilerplate: `import Inject`, `@ObserveInjection var inject` property, and `.enableInjection()` as the outermost body modifier.

**U6a. Refresh `Theme/` from `tokens.yaml`.** When `tokens.yaml` is present, regenerate `iOS/<App>/Theme/` per [`swift-token-templates.md`](swift-token-templates.md): one Swift file per token category (with `spacing` / `cornerRadius` colocated in `Theme/Spacing.swift`), plus the structural `Theme/Theme.swift` scaffold. Files carry the `// Generated from design-system/tokens.yaml — do not edit manually.` header (except `Theme.swift`) so a subsequent regeneration overwrites safely. The header always uses the canonical project-level path regardless of which file generation actually read — this keeps the header stable and avoids breaking header-based detection. When a category is removed from `tokens.yaml`, delete the corresponding file under `Theme/`. Files lacking the header are operator-owned and must be preserved. When `tokens.yaml` is **absent**, skip this step entirely and follow the HIG fallback policy in [`design-system-integration.md`](design-system-integration.md).

**U6b. Refresh `Components/` from `composition.yaml`.** For every `group` carrying `component: <slug>`, emit one named SwiftUI `View` under `iOS/<App>/Components/<PascalCaseSlug>.swift`. Props are inferred from variation observed across instances of the slug: `bind`, `event`, `error`, `asset`, token references, `*-when` keys, and free text content that differ across instances become parameters; values constant across all instances are baked into the view body. Structural identity is pre-enforced by `specify tool run vectis -- validate composition`. When a slug disappears, delete the file and rewrite each former call site to inline the group body. See [`design-system-integration.md`](design-system-integration.md) for a worked `task-row` example.

**U6c. Refresh `Resources/Assets.xcassets/` from `assets.yaml`.** For every `assets.yaml` entry referenced from `composition.yaml`, copy the per-platform iOS source(s) into `iOS/<App>/Resources/Assets.xcassets/<asset-id>.imageset/` (raster or vector). `kind: symbol` entries skip the catalog — emit `Image(systemName: "<sf-symbol>")` at the call site. The canonical `source:` SVG is provenance only and is **never** copied into the shell tree. When an entry is removed from `assets.yaml` or its references are removed from `composition.yaml`, delete the corresponding generated catalog entry. Operator-authored entries (e.g. `AppIcon.appiconset/`) are preserved. Missing iOS exports for `vector` entries referenced from `composition.yaml` are validation errors, not deferred TODOs — halt and surface the validator output verbatim.

**U7. Update build configuration.** Update `project.yml` if new dependencies are needed; update `Makefile` if build targets changed; if `project.yml` lacks the `Inject` SPM package, add it along with the `- package: Inject` target dependency, Debug-only `OTHER_LDFLAGS` (`["-w", "-Xlinker", "-interposable"]`), and `EMIT_FRONTEND_COMMAND_LINES: "YES"` in the Debug config.

**U8. Format and verify.** Run `swiftformat` on modified files → `make typegen` (regenerate Swift domain types) → `make package` (build Rust static library and Swift package) → `make xcode` (regenerate the Xcode project). Fix any build errors.

## Composition Mapping Priority

When `composition.yaml` is present, region structure and group container tree take precedence over convention-based inference for view body composition:

- **Groups** → SwiftUI stacks: `direction: row` → `HStack(spacing:)`, `direction: column` → `VStack(spacing:)`, `direction: stack` → `ZStack`.
- **Sizing** → `.frame()` modifiers: `fill` → `.frame(maxWidth: .infinity)`, fixed values → `.frame(width:)` / `.frame(height:)`.
- **Surface decoration** → styled container views: `background` → `.background()`, `corner_radius` → `.cornerRadius()` or `.clipShape(RoundedRectangle())`, `elevation` → `.shadow()`.
- **Platform-specific overrides**: when `composition.yaml` contains `platforms.ios` region overrides for a screen, use those over the shared regions.

When `composition.yaml` is absent, fall back to convention-based inference.

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
2. **Preserve custom styling** added beyond design system defaults.
3. **Preserve custom view logic** (e.g., animations, gestures) not driven by the ViewModel.
4. **Preserve `#Preview` blocks** on unchanged views.
5. **Preserve `project.yml` customizations** (signing, entitlements, custom build phases).
6. **Preserve `Makefile` customizations** (additional targets, environment variables).

## Scaffold ownership

XcodeGen `project.yml`, the `Makefile` pipeline, and all baseline shell scaffolding (`project.yml` packages, Inject SPM wiring, CAP markers, starter screens) are owned by the Vectis scaffold templates in [`augentic/specify-cli`](https://github.com/augentic/specify-cli) (`<specify-cli>/crates/vectis/` and the Vectis template sources). Do not hand-edit those files in Create Mode; let `specify tool run vectis -- scaffold ios` write them and then modify in Update Mode.

## Examples

| Example | Capabilities | Demonstrates |
|---|---|---|
| [`examples/01-simple-counter-ios.md`](examples/01-simple-counter-ios.md) | Render | Minimal shell, Core.swift, two screens, project setup |
| [`examples/02-http-counter-ios.md`](examples/02-http-counter-ios.md) | Render + HTTP | Async HTTP handling, error view, three screens |

## Error Handling

| Error | Resolution |
|---|---|
| `app.rs` not found | Verify `app-dir` points to a Crux app with `shared/src/app.rs` |
| Unknown Effect variant | Add a placeholder `case` with a `fatalError("unhandled")` and report |
| `xcodegen` fails | Check `project.yml` syntax; verify path references |
| Build fails with missing types | Verify `uniffi` matches the active Vectis version pins, then rerun `make typegen`, `make package`, and `make xcode` to isolate the mismatch |
| `specify tool run vectis -- validate composition` reports unresolved token / asset reference | Composition references a token/asset id not declared in `tokens.yaml` / `assets.yaml`. Writer halts; operator must add the missing entry or remove the reference |
| Missing iOS export for a `kind: vector` asset | Validation error, not a deferred TODO. Halt shell generation for the affected screen and report the missing `sources.ios` field |

## Verification Checklist

### Build

- [ ] `make typegen` completes without errors
- [ ] `make package` builds the Rust static library and Swift package
- [ ] `make xcode` regenerates the Xcode project
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
- [ ] `kind: symbol` entries render via `Image(systemName: "<sf-symbol>")` at the call site (no catalog entry)

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
- [ ] No `TextField` or small `Button` inside a `ScrollView` within a `NavigationStack` — use `.safeAreaInset(edge:)` to pin interactive controls, or use `List`
- [ ] No horizontal `ScrollView` nested inside a vertical `ScrollView` with tappable content — use `.safeAreaInset(edge:)` for the inner scrollable, or ensure tappable elements use `Button` with `.buttonStyle(.plain)`

## Platform Rules (Full)

The condensed Guardrails block in the SKILL.md surfaces only the highest-priority NEVER/ALWAYS rules. The full set:

- **NEVER add business logic to Swift code.** All business logic lives in the Rust core; the shell only renders views and performs platform I/O. Run `core-writer` first — this skill assumes an existing Crux core with `crate-type = ["staticlib"]` and the `uniffi` feature gate.
- **NEVER hand-pin UniFFI versions inside the Swift project.** The `uniffi` crate pin must be compatible with the contract expected by `cargo-swift` and the `crux_core` bundled bindgen; surface mismatches via `make package` and `make xcode` rather than editing pins.
- **NEVER place `TextField` or small `Button` inside a `ScrollView` within a `NavigationStack`.** The `UIScrollView` touch-delay mechanism suppresses taps; use `.safeAreaInset(edge:)` to pin interactive controls or use `List`. The same hazard applies to a horizontal `ScrollView` nested inside a vertical one — pin the inner scrollable, or ensure tappable elements use `Button` with `.buttonStyle(.plain)`. See [`swiftui-view-patterns.md`](swiftui-view-patterns.md).
- **ALWAYS produce both Swift packages**: `SharedTypes` (domain types via `facet_typegen`) and `Shared` (UniFFI bindings + XCFramework via `cargo-swift`).
- **ALWAYS leave the Inject hot-reload boilerplate in place.** Inject is a no-op in Release builds (stripped by LLVM); the scaffold wires Inject into `project.yml` (SPM + Debug-only `OTHER_LDFLAGS: -Xlinker -interposable` + `EMIT_FRONTEND_COMMAND_LINES: YES`). Update Mode only has to add `@ObserveInjection` / `.enableInjection()` to new screen views.
- **ALWAYS treat `app.rs` as the primary input.** When `{slice-dir}` is supplied, supplement with `## iOS Shell Requirements` from the feature spec and `## iOS Shell Details` from `design.md` for platform-specific requirements not expressible in the Rust types alone.
