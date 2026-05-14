# Swift Token Templates

Concrete Swift code templates the iOS writer emits when `tokens.yaml` is present.
The generated files are **shell-local** — they live inside the iOS shell target
(`iOS/<App>/Theme/`) and do **not** form a separate Swift Package. There is no
`import VectisDesign`. Token names are referenced directly because the Theme
enums are part of the same target as the screen views that consume them.

When `tokens.yaml` is **absent**, the iOS writer skips this entire emit step and
falls back to platform-native HIG defaults — see
[`design-system-integration.md`](design-system-integration.md) for the fallback
policy. This file describes only the present-tokens path.

## File layout

Generated under `iOS/<App>/Theme/`:

| Token category(ies) | Generated file | Notes |
|---|---|---|
| `colors` | `Theme/Colors.swift` | One file per category. |
| `typography` | `Theme/Typography.swift` | |
| `spacing` + `cornerRadius` | `Theme/Spacing.swift` | Colocated as two enums. |
| `elevation` | `Theme/Elevation.swift` | New v1 category; separate file. |
| `border` | `Theme/Border.swift` | New v1 category; uses composite shape. |
| `opacity` | `Theme/Opacity.swift` | New v1 category. |
| _any new scalar / color / font category_ | `Theme/<Name>.swift` | One file per new category unless the writer explicitly colocates. |
| Theme bundle | `Theme/Theme.swift` | Structural scaffold; references every enum. No "Generated from" header. |

The shell target's `project.yml` already lists `<App>/` as a source root via
the CLI scaffold; nested directories are picked up automatically by XcodeGen,
so no `project.yml` edits are required for new Theme files.

Every generated **token** file (i.e. everything under `Theme/` except
`Theme.swift`) carries this header at the top:

```swift
import SwiftUI

// MARK: - {Human-Readable Category Name}
// Generated from design-system/tokens.yaml — do not edit manually.

enum Vectis{Category} {
    {token entries}
}

{extensions, if any}
```

`Theme.swift` is structural scaffolding (the SwiftUI environment key + view
modifier) — it is rewritten on every regeneration to reference the current
enum set, but it never carries the "Generated from" header because operators
sometimes extend it with shell-local additions (see the comment block at the
top of the worked Theme.swift example below).

### Access modifier

Token enums use the **default internal** access level. They are part of the
shell target, not a library, so there is no `public` API surface to expose.
The previous standalone token approach emitted `public enum Vectis…` because it
generated a separate Swift Package; the shell-local equivalent drops the
modifier entirely. Existing screen-view code that referenced `VectisColors.primary`
keeps working unchanged because `internal` is the default Swift visibility for
same-target access.

## Color Template

### Enum

MARK label: `Semantic Colors`

Each entry:

```swift
static let {name} = Color(light: "{light}", dark: "{dark}")
```

Group entries with a blank line between semantic groups. Groups are determined
by name prefix root. YAML token IDs are kebab-case (`on-primary`,
`surface-secondary`); the Swift `static let` names below are the camelCased
form the writer emits.

| Prefix root | Tokens (Swift identifier form) |
|---|---|
| `primary` | `primary`, `primaryContainer`, `onPrimary`, `onPrimaryContainer` |
| `secondary` | `secondary`, `secondaryContainer`, `onSecondary`, `onSecondaryContainer` |
| `surface` | `surface`, `surfaceSecondary`, `onSurface`, `onSurfaceSecondary` |
| `error` | `error`, `onError` |
| _(ungrouped)_ | `outline`, `shadow`, and any others |

### Required Extensions

The color file must include the `Color(light:dark:)` initializer extension
after the enum. The shell target deploys to iOS only, so the extension uses
`UIKit` directly without the `#if canImport` guard the previous design-system
package required (the package had to compile for both `iOS(.v17)` and
`macOS(.v14)`; the shell target deploys to iOS only).

```swift
// MARK: - Color Initializer from Hex

import UIKit

extension Color {
    init(light: String, dark: String) {
        self.init(uiColor: UIColor { traits in
            traits.userInterfaceStyle == .dark
                ? UIColor(hex: dark)
                : UIColor(hex: light)
        })
    }
}

extension UIColor {
    convenience init(hex: String) {
        let hex = hex.trimmingCharacters(in: .init(charactersIn: "#"))
        var rgb: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&rgb)
        self.init(
            red: CGFloat((rgb >> 16) & 0xFF) / 255,
            green: CGFloat((rgb >> 8) & 0xFF) / 255,
            blue: CGFloat(rgb & 0xFF) / 255,
            alpha: 1
        )
    }
}
```

### Complete Example

```swift
import SwiftUI

// MARK: - Semantic Colors
// Generated from design-system/tokens.yaml — do not edit manually.

enum VectisColors {
    static let primary = Color(light: "#007AFF", dark: "#0A84FF")
    static let primaryContainer = Color(light: "#D6E4FF", dark: "#003A70")
    static let onPrimary = Color(light: "#FFFFFF", dark: "#FFFFFF")
    static let onPrimaryContainer = Color(light: "#001D36", dark: "#D6E4FF")

    static let secondary = Color(light: "#5856D6", dark: "#5E5CE6")
    static let secondaryContainer = Color(light: "#E8E0FF", dark: "#2F2D6E")
    static let onSecondary = Color(light: "#FFFFFF", dark: "#FFFFFF")
    static let onSecondaryContainer = Color(light: "#1C1B33", dark: "#E8E0FF")

    static let surface = Color(light: "#FFFFFF", dark: "#1C1C1E")
    static let surfaceSecondary = Color(light: "#F2F2F7", dark: "#2C2C2E")
    static let onSurface = Color(light: "#000000", dark: "#FFFFFF")
    static let onSurfaceSecondary = Color(light: "#3C3C43", dark: "#EBEBF5")

    static let error = Color(light: "#FF3B30", dark: "#FF453A")
    static let onError = Color(light: "#FFFFFF", dark: "#FFFFFF")

    static let outline = Color(light: "#C6C6C8", dark: "#38383A")
    static let shadow = Color(light: "#000000", dark: "#000000")
}

// MARK: - Color Initializer from Hex

import UIKit

extension Color {
    init(light: String, dark: String) {
        self.init(uiColor: UIColor { traits in
            traits.userInterfaceStyle == .dark
                ? UIColor(hex: dark)
                : UIColor(hex: light)
        })
    }
}

extension UIColor {
    convenience init(hex: String) {
        let hex = hex.trimmingCharacters(in: .init(charactersIn: "#"))
        var rgb: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&rgb)
        self.init(
            red: CGFloat((rgb >> 16) & 0xFF) / 255,
            green: CGFloat((rgb >> 8) & 0xFF) / 255,
            blue: CGFloat(rgb & 0xFF) / 255,
            alpha: 1
        )
    }
}
```

## Typography Template

### Enum

MARK label: `Typography Scale`

Each entry:

```swift
static let {name} = Font.system(size: {size}, weight: .{weight})
```

When the YAML entry carries `lineHeight` or `letterSpacing`, omit the values
from the `Font.system(...)` call (SwiftUI's `Font` does not expose those
attributes directly) and leave a `// lineHeight: <n>, letterSpacing: <n>`
comment so the operator can apply them at the call site via
`.lineSpacing()` / `.kerning()` modifiers.

### Weight Mapping

| YAML value | Swift value |
|---|---|
| `ultra-light` | `.ultraLight` |
| `thin` | `.thin` |
| `light` | `.light` |
| `regular` | `.regular` |
| `medium` | `.medium` |
| `semibold` | `.semibold` |
| `bold` | `.bold` |
| `heavy` | `.heavy` |
| `black` | `.black` |

### Complete Example

```swift
import SwiftUI

// MARK: - Typography Scale
// Generated from design-system/tokens.yaml — do not edit manually.

enum VectisTypography {
    static let largeTitle = Font.system(size: 34, weight: .bold)
    static let title = Font.system(size: 28, weight: .bold)
    static let title2 = Font.system(size: 22, weight: .bold)
    static let title3 = Font.system(size: 20, weight: .semibold)
    static let headline = Font.system(size: 17, weight: .semibold)
    static let body = Font.system(size: 17, weight: .regular)
    static let callout = Font.system(size: 16, weight: .regular)
    static let subheadline = Font.system(size: 15, weight: .regular)
    static let footnote = Font.system(size: 13, weight: .regular)
    static let caption = Font.system(size: 12, weight: .regular)
    static let caption2 = Font.system(size: 11, weight: .regular)
}
```

## Scalar Template

### Enum

MARK label: `{Category Name} Scale` (e.g., `Spacing Scale`, `Corner Radius Scale`,
`Elevation Scale`, `Opacity Scale`)

Each entry:

```swift
static let {name}: CGFloat = {value}
```

Values are written as integers when the YAML value is a whole number (e.g.,
`16` not `16.0`). If the YAML value has a decimal component, preserve it
(e.g., `1.5`).

For the `opacity` category, use `Double` instead of `CGFloat` because SwiftUI's
`.opacity()` modifier takes a `Double`:

```swift
static let disabled: Double = 0.38
```

### Colocated Scalars

`spacing` and `cornerRadius` share `Theme/Spacing.swift`. They are written as
two separate enums separated by a blank line and a MARK comment:

```swift
import SwiftUI

// MARK: - Spacing Scale
// Generated from design-system/tokens.yaml — do not edit manually.

enum VectisSpacing {
    static let xxs: CGFloat = 2
    static let xs: CGFloat = 4
    static let sm: CGFloat = 8
    static let md: CGFloat = 16
    static let lg: CGFloat = 24
    static let xl: CGFloat = 32
    static let xxl: CGFloat = 48
}

// MARK: - Corner Radius Scale

enum VectisCornerRadius {
    static let none: CGFloat = 0
    static let sm: CGFloat = 4
    static let md: CGFloat = 8
    static let lg: CGFloat = 12
    static let xl: CGFloat = 16
    static let full: CGFloat = 9999
}
```

New scalar categories (`elevation`, `opacity`) get their own file unless
explicitly colocated.

## Border Template

`border` is a composite category — each entry carries `width`, `color` (a
reference to a `colors` token), and optional `radius` (a reference to a
`cornerRadius` token).

```swift
import SwiftUI

// MARK: - Border Scale
// Generated from design-system/tokens.yaml — do not edit manually.

struct VectisBorderStyle {
    let width: CGFloat
    let color: Color
    let radius: CGFloat?
}

enum VectisBorders {
    static let subtle = VectisBorderStyle(
        width: 1,
        color: VectisColors.outline,
        radius: nil
    )
}
```

Apply at call sites via `.overlay { RoundedRectangle(...).stroke(...) }`.

## Theme Template

`Theme.swift` is structural scaffolding that references every generated enum,
exposes the SwiftUI environment key, and provides the `.vectisTheme()` view
modifier. It is regenerated on every `tokens.yaml` change so the bundle
includes only the categories that currently exist.

```swift
import SwiftUI

/// Bundles the full Vectis design system for SwiftUI environment injection.
///
/// Apply at the app root:
/// ```swift
/// @main
/// struct MyApp: App {
///     var body: some Scene {
///         WindowGroup {
///             ContentView()
///                 .vectisTheme()
///         }
///     }
/// }
/// ```
///
/// Access in any view:
/// ```swift
/// @Environment(\.vectisTheme) private var theme
/// Text("Hello").font(theme.typography.title)
/// ```
struct VectisTheme: Sendable {
    {one property per category}
}

// MARK: - Environment Key

private struct VectisThemeKey: EnvironmentKey {
    static let defaultValue = VectisTheme()
}

extension EnvironmentValues {
    var vectisTheme: VectisTheme {
        get { self[VectisThemeKey.self] }
        set { self[VectisThemeKey.self] = newValue }
    }
}

// MARK: - View Modifier

extension View {
    /// Injects the Vectis design system theme into the view hierarchy.
    func vectisTheme() -> some View {
        environment(\.vectisTheme, VectisTheme())
    }
}
```

### Theme Property Pattern

Each category gets one property line:

```swift
let {camelCaseCategory}: Vectis{PascalCaseCategory}.Type = Vectis{PascalCaseCategory}.self
```

For the four categories every shell currently consumes:

```swift
let colors: VectisColors.Type = VectisColors.self
let typography: VectisTypography.Type = VectisTypography.self
let spacing: VectisSpacing.Type = VectisSpacing.self
let cornerRadius: VectisCornerRadius.Type = VectisCornerRadius.self
```

For new v1 categories add the matching line:

```swift
let elevation: VectisElevation.Type = VectisElevation.self
let borders: VectisBorders.Type = VectisBorders.self
let opacity: VectisOpacity.Type = VectisOpacity.self
```

## YAML-to-File Mapping Summary

| YAML key | Value shape | Swift enum | File | MARK label |
|---|---|---|---|---|
| `colors` | Color | `VectisColors` | `Theme/Colors.swift` | `Semantic Colors` |
| `typography` | Font | `VectisTypography` | `Theme/Typography.swift` | `Typography Scale` |
| `spacing` | Scalar | `VectisSpacing` | `Theme/Spacing.swift` | `Spacing Scale` |
| `cornerRadius` | Scalar | `VectisCornerRadius` | `Theme/Spacing.swift` | `Corner Radius Scale` |
| `elevation` | Scalar | `VectisElevation` | `Theme/Elevation.swift` | `Elevation Scale` |
| `border` | Composite | `VectisBorders` (+ `VectisBorderStyle` struct) | `Theme/Border.swift` | `Border Scale` |
| `opacity` | Scalar (`Double`) | `VectisOpacity` | `Theme/Opacity.swift` | `Opacity Scale` |
| _(new scalar)_ | Scalar | `Vectis{Name}` | `Theme/{Name}.swift` | `{Name} Scale` |
| _(new color)_ | Color | `Vectis{Name}` | `Theme/{Name}.swift` | `{Name}` |
| _(new font)_ | Font | `Vectis{Name}` | `Theme/{Name}.swift` | `{Name} Scale` |

## Removing stale files

When a token category is removed from `tokens.yaml`, the iOS writer deletes
the corresponding generated file under `iOS/<App>/Theme/`. Files without the
"Generated from" header are operator-owned and never deleted automatically —
that includes `Theme.swift` (which is structural and rewritten in place, not
deleted). When every token category is removed, the writer rewrites
`Theme.swift` to an empty bundle and leaves it on disk so any view code that
calls `.vectisTheme()` keeps compiling.
