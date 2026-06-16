# Design System Integration

How the iOS writer integrates `tokens.yaml` and `assets.yaml` into a generated
SwiftUI shell. Tokens become **shell-local** Theme code under
`iOS/<App>/Theme/`; referenced asset files are **copied** into
`iOS/<App>/Resources/Assets.xcassets/` during generation. There is no shared
Swift Package, no `import VectisDesign`, and no path back into
`design-system/` from the rendered shell project.

This file complements [`swift-token-templates.md`](token-templates.md),
which carries the concrete code templates per token shape.

## Authority hierarchy

When this document conflicts with another source, follow this precedence:

1. `tokens.yaml` and `assets.yaml` — the operator-owned input artifacts.
2. [`swift-token-templates.md`](token-templates.md) — concrete code
   templates per token category.
3. This document — integration policy and fallback rules.

## Generated layout

When `tokens.yaml` is present, the iOS writer emits theme code under the
shell target's source root:

```
iOS/
├── project.yml
├── Makefile
└── <App>/
    ├── <App>App.swift
    ├── Core.swift
    ├── ContentView.swift
    ├── Views/...
    ├── Components/                        # one file per component: <slug>
    │   └── TaskRow.swift
    ├── Theme/                             # generated from tokens.yaml
    │   ├── Colors.swift
    │   ├── Typography.swift
    │   ├── Spacing.swift
    │   ├── (Elevation.swift, Border.swift, Opacity.swift, … as needed)
    │   └── Theme.swift
    └── Resources/
        └── Assets.xcassets/               # copied from assets/exports/ios/
            ├── AppIcon.appiconset/
            └── <asset-id>.imageset/
```

The `specify tool run vectis -- scaffold ios <AppName>` render step produces `<App>/`, the
entry point, `Core.swift`, `ContentView.swift`, and the starter `Views/`
directory. The iOS writer adds `Components/`, `Theme/`, and
`Resources/Assets.xcassets/` on first generation when the corresponding input
artifacts exist. XcodeGen's `project.yml` already lists `<App>/` as a source
root; nested directories are picked up automatically — no `project.yml` edits
are required when adding new theme or component files.

The generated app **MUST NOT** depend on `import VectisDesign` and **MUST
NOT** reference an external Swift Package, Xcode framework, or path under
`design-system/ios/`. The `iOS/` shell must build from its own platform
directory after generation.

## Token integration

### Reading `tokens.yaml`

The iOS writer's primary token input is `tokens.yaml`. Resolution order:

1. Slice-local `.specify/slices/<name>/tokens.yaml`, when present.
2. Project-level `design-system/tokens.yaml`.
3. Neither — fall through to the HIG fallback policy below.

When `tokens.yaml` is present, generate one Theme file per category under
`iOS/<App>/Theme/` per [`swift-token-templates.md`](token-templates.md).
The token file generation is mechanical: each YAML category maps to a single
Swift enum keyed by the camelCased token id. Adding a new category extends
both `swift-token-templates.md` and this document.

### Using token references in views

Reference Theme types directly from screen views and components — they are
part of the same target as the views that consume them, so no `import` is
needed:

```swift
Text("Hello")
    .foregroundStyle(VectisColors.onSurface)

Rectangle()
    .fill(VectisColors.primary)

VStack(spacing: VectisSpacing.md) {
    // children spaced 16pt apart
}

.padding(.horizontal, VectisSpacing.md)
.padding(.vertical, VectisSpacing.sm)

RoundedRectangle(cornerRadius: VectisCornerRadius.md)

.clipShape(RoundedRectangle(cornerRadius: VectisCornerRadius.lg))
```

Colors automatically adapt to light/dark mode through the
`Color(light:dark:)` initializer (see
[`swift-token-templates.md`](token-templates.md#color-template)). Never
emit hardcoded `Color(red:green:blue:)`, `Color("name")`, inline
`.system(size:)`, or magic numbers in generated views.

### Theme environment (optional)

For views that need the full theme bundle:

```swift
ContentView(core: core)
    .vectisTheme()

@Environment(\.vectisTheme) private var theme
Text("Hello").font(theme.typography.title)
```

The `.vectisTheme()` modifier is defined in `Theme/Theme.swift` (the
structural scaffold) and is part of the shell target. Most views should use
the static enums directly; the environment bundle is provided for ergonomics
in deeply nested views or for downstream operator extensions.

### Disabled state convention

For disabled interactive elements, apply 38% opacity to the normal color:

```swift
.foregroundStyle(VectisColors.primary.opacity(isDisabled ? 0.38 : 1.0))
```

When `tokens.yaml` defines an `opacity.disabled` token, prefer
`VectisOpacity.disabled` over the literal `0.38`.

### Token reference resolution and CLI gate

The deterministic check that every token reference in `composition.yaml`
resolves to a `tokens.yaml` entry lives in
`specify tool run vectis -- validate composition`: when sibling
`tokens.yaml` exists, the validator auto-invokes `tokens` mode and reports
unresolved references as errors before the iOS writer is called. The writer
does not need to re-validate references at generation time; it consumes the
already-validated input set.

## HIG fallback policy

When `tokens.yaml` is **absent** the iOS writer falls back to platform-native
HIG defaults instead of emitting a Theme directory — fallback policy belongs
to shell writers. The skill emits no `Theme/` directory and no Theme enums;
screen views reference the platform defaults directly.

Per-category fallback:

| Category | HIG fallback |
|---|---|
| Colors | SwiftUI semantic colors (`.primary`, `.secondary`, `Color.accentColor`, `.foregroundStyle(.primary)`, `.foregroundStyle(.red)`); `.background` follows the system surface (`Color(.systemBackground)`). |
| Typography | `Font.system(.body)`, `.system(.title)`, `.system(.headline)`, `.system(.caption)` etc. — the standard SwiftUI dynamic-type ramp. |
| Spacing | SwiftUI default stack spacing (omit the `spacing:` argument on `VStack` / `HStack`); for explicit padding use `8` (`sm`), `16` (`md`), and `24` (`lg`) inline literals. |
| Corner radius | Inline `8` for medium, `12` for large; SwiftUI's standard component radii. |
| Elevation | `.shadow(radius: 2)` for cards, omitted otherwise. |
| Opacity | Inline `0.38` for disabled states, `0.6` for de-emphasised text. |

When `tokens.yaml` is **present but incomplete** (some categories defined,
others absent), shell writers MAY use the same platform default for the
**absent** categories. Shell writers MUST NOT silently substitute defaults
for a token name that is referenced from `composition.yaml` but missing from
`tokens.yaml` — that condition is an error reported by
`specify tool run vectis -- validate composition` and halts shell generation for the
affected screen. The writer surfaces the validator output verbatim and
declines to emit code that papers over the missing token.

When the HIG fallback is in use, the iOS writer prefers SwiftUI's built-in
semantic colors (which already adapt to light / dark mode) over hex-coded
defaults. This keeps the no-tokens path operator-friendly: a freshly
scaffolded app looks correct on both appearances without any token authoring.

## Asset integration

### Render-by-`kind`

Shell writers resolve each composition `icon` / `image` / `icon-button` / `fab`
reference through `assets.yaml` and emit view code strictly by entry `kind`:

| `assets.<id>.kind` | iOS emission |
|---|---|
| `vector` | `Image("<id>")` from a shell-local imageset copied from the materialized export |
| `raster` | `Image("<id>")` from a shell-local imageset with per-density PNGs copied from the materialized export |
| `symbol` | `Image(systemName: symbols.ios)` — no catalog copy |

**Forbidden at build time:** emitting `Image(systemName:)` (or any SF Symbol
substitute) for an id whose entry is `vector` or `raster`. Missing platform
exports are validation errors (`assets-materialization-missing`) — never a
writer shortcut. Platform glyph use requires an explicit `kind: symbol` entry
(optionally `inferred: true` when promoted from screenshot inference; see
[Layout Inferer Contract](../layout-inferer-contract.md) and
`adapters/sources/screenshots/briefs/extract.md`).

### Reading `assets.yaml`

The iOS writer's primary asset input is `assets.yaml`. Resolution order:

1. Slice-local `.specify/slices/<name>/assets.yaml`, when present, plus
   files under `.specify/slices/<name>/assets/`.
2. Project-level `design-system/assets.yaml` plus files under
   `design-system/assets/`.
3. Neither — generate views without referenced asset entries (any
   composition that references an asset id will already have failed
   validation at the CLI gate).

The deterministic check that every asset reference in `composition.yaml`
resolves to an `assets.yaml` entry lives in
`specify tool run vectis -- validate composition` (auto-invokes `assets` mode when
present). Missing files are errors; missing optional densities are warnings.
The writer consumes the already-validated input set.

### Materialize-before-copy

Canonical masters live under `design-system/assets/` (`source:` on each entry).
Per-platform binaries live under `design-system/assets/exports/ios/` and are
recorded in `sources.ios` (operator-pinned or auto-written by
`vectis materialize assets`). Materialization runs automatically at
`specify slice build --phase prepare` for in-scope assets with missing exports;
operators may also run `specify tool run vectis -- materialize assets` manually
after editing canonical masters. Committed `exports/` trees are version-controlled
— CI and shell builds consume them without re-running materialize on every job.

Build hand-off is **materialize-then-copy**: the iOS writer **copies** files
from each entry's resolved `sources.ios` export path(s) into the shell target's
asset catalog at `iOS/<App>/Resources/Assets.xcassets/`. The canonical
`source:` file is provenance only — never copied into the shell. The generated
shell project must build from its own platform directory after generation; it
MUST NOT symlink, alias, or path-reference `design-system/assets/` from
`project.yml`, nor consume files from `<change>/assets/` at runtime.
Per-platform copy targets (paths relative to `design-system/`; materialize writes under `assets/exports/ios/`):

| `role` + `kind` | Export tree read (`sources.ios` pin) | Shell catalog target |
|---|---|---|
| `icon` or `decorative` + `vector` | `<id>.imageset/<id>.pdf` and `Contents.json` (SVG master materialized to PDF) | Copy the whole `<asset-id>.imageset/` directory into `Assets.xcassets/`. |
| `illustration` + `vector` | `<id>.imageset/<id>@2x.png`, `<id>@3x.png`, and `Contents.json` (no `@1x` — illustration materialize emits `@2x` / `@3x` only) | Copy the whole `<asset-id>.imageset/` directory. |
| `photo` or UI `icon` + `raster` | `{1x,2x,3x}` per-density files under the pinned imageset (operator-pinned; materialize does not invent density ladders) | `<asset-id>.imageset/` with per-density PNG / JPEG files plus `Contents.json`. |
| `symbol` (any role) | `symbols.ios` | No catalog copy — emit `Image(systemName: "<sf-symbol>")` at the call site. |
| `app-icon` | `assets/exports/ios/app-icon/AppIcon.appiconset/` directory pin (`AppIcon.png` + `Contents.json`; path A auto-convert or path B operator pin) | Copy into `AppIcon.appiconset/`; scaffold ships an empty skeleton materialize fills. |

Reference the copied asset by its kebab-case asset id at the call site:

```swift
Image("onboarding-hero")           // raster / vector

Image(systemName: "xmark")          // symbol entry's symbols.ios value
    .foregroundStyle(VectisColors.onSurface)
```

For symbols the `tint` token (when present in `assets.yaml`) becomes a
`.foregroundStyle(VectisColors.<tint>)` modifier at the call site. Single
colour vector assets MAY also be tinted via `.foregroundStyle(...)` when the
PDF is rendered with template intent.

### Missing platform exports

When a `vector` or `raster` asset is referenced from `composition.yaml` but
`sources.ios` is missing or the pinned export path is absent on disk, the
validator reports `assets-materialization-missing` and shell generation halts
for the affected screen. The iOS writer does **not** silently fall back to an
SF Symbol, generate from the canonical `source:` at build time, or skip the
screen. The legitimate operator responses are to run materialize (or commit
operator-pinned exports under `exports/ios/`), re-declare the asset as
`kind: symbol` with an explicit platform glyph mapping, or remove the reference
from `composition.yaml`.

### Stale catalog cleanup

When an asset entry is removed from `assets.yaml`, the iOS writer deletes
the corresponding `<asset-id>.imageset/` (and any other generated catalog
entries) from `Resources/Assets.xcassets/`. Operator-authored catalog
entries (e.g. `AppIcon.appiconset/`) are preserved; the writer only deletes
entries it generated.

## Component directive contract

When a `composition.yaml` `group` carries `component: <slug>`, the iOS
writer emits **one named SwiftUI `View`** per slug under
`iOS/<App>/Components/`, PascalCased from the slug:

| `composition.yaml` slug | Generated file | Type |
|---|---|---|
| `task-row` | `Components/TaskRow.swift` | `struct TaskRow: View` |
| `news-card` | `Components/NewsCard.swift` | `struct NewsCard: View` |

Every call site in `composition.yaml` becomes a use of the named view.
Props are inferred from variation observed across instances of the slug:

- `bind`, `event`, `error`, `asset`, token references, `*-when` keys, and
  free text content that **differ** across instances become parameters on
  the generated view.
- Values that are **constant** across all instances are baked into the view
  body.

The structural-identity rule is enforced by
`specify tool run vectis -- validate composition` before the iOS writer runs, so the
writer can trust that every instance of the slug shares the same skeleton
and only the wiring varies.

The directive is platform-agnostic; the inferred prop shape is per-platform.
Android may emit a slightly different prop signature for the same slug —
v1 does not require cross-shell prop agreement.

### Component examples

For a `task-row` slug whose instances all carry the same skeleton (an
`HStack` of a checkbox, a title `Text`, and a swipe action), but whose
`bind`, `event`, and `strikethrough-when` keys vary across screens:

```swift
import SwiftUI

struct TaskRow: View {
    let title: String
    let isCompleted: Bool
    let onToggle: () -> Void

    var body: some View {
        HStack(spacing: VectisSpacing.sm) {
            Button {
                onToggle()
            } label: {
                Image(systemName: isCompleted ? "checkmark.circle.fill" : "circle")
                    .foregroundStyle(
                        isCompleted ? VectisColors.primary : VectisColors.onSurfaceSecondary
                    )
            }
            .buttonStyle(.plain)

            Text(title)
                .font(VectisTypography.body)
                .strikethrough(isCompleted)
                .foregroundStyle(VectisColors.onSurface)

            Spacer()
        }
        .padding(.horizontal, VectisSpacing.md)
        .padding(.vertical, VectisSpacing.sm)
    }
}
```

The call site becomes:

```swift
ForEach(viewModel.tasks, id: \.id) { task in
    TaskRow(
        title: task.title,
        isCompleted: task.isCompleted,
        onToggle: { onEvent(.toggle(task.id)) }
    )
}
```

instead of the flattened `HStack { ... }` body it would have produced
without the directive.

## Review compliance

The `vectis-ios-reviewer` skill checks generated views for:

1. Token-backed visual literals when `tokens.yaml` is present — `VectisColors`
   for color references, `VectisTypography` for fonts, `VectisSpacing` for
   spacing values, `VectisCornerRadius` for corner radii.
2. **No** stale external design-system dependencies — `import VectisDesign`,
   `:vectis-design`, `design-system/ios`, `design-system/android`.
3. Asset references that resolve to entries in the shell-local
   `Assets.xcassets/` (no string-literal paths into `design-system/assets/`).
4. Groups that visibly recur in `composition.yaml` without a `component:`
   slug — flagged so the operator can promote them to a named component
   before drift compounds.

When `tokens.yaml` is absent (HIG fallback path), the reviewer accepts
SwiftUI semantic colors (`.primary`, `.secondary`, `.accentColor`),
`Font.system(.body)` etc., and the inline-literal padding values listed in
the HIG fallback table. Hardcoded hex (`Color(red:green:blue:)`,
`Color("named-asset")` for color tokens) remains a defect even on the
fallback path because the operator can always introduce `tokens.yaml` later.

Exceptions are allowed for system-provided styles (e.g.,
`.buttonStyle(.borderedProminent)`, `.tint(...)`) where the platform applies
its own colors.

## References

- [Component Catalog](../spec-runtime/components.md) — shared component factoring workflow.
- [Layout Inferer Contract](../layout-inferer-contract.md) — component directive and validation rules.
- [Vectis runtime schemas](../../schemas/README.md) — tool-owned schema retrieval and validation commands.
