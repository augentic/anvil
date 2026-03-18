# Swift Token Templates

Concrete Swift code templates for each token value shape. The design-system-writer
skill uses these templates to generate the VectisDesign Swift Package from
`tokens.yaml`.

All generated token files share this structure (Theme.swift uses a separate
template -- see the Theme Template section below):

```
import SwiftUI

// MARK: - {Human-Readable Category Name}
// Generated from design-system/tokens.yaml — do not edit manually.

public enum Vectis{Category} {
    {token entries}
}

{extensions, if any}
```

## Color Template

### Enum

MARK label: `Semantic Colors`

Each entry:

```swift
public static let {name} = Color(light: "{light}", dark: "{dark}")
```

Group entries with a blank line between semantic groups. Groups are determined
by name prefix root:

| Prefix root | Tokens |
|---|---|
| `primary` | `primary`, `primaryContainer`, `onPrimary`, `onPrimaryContainer` |
| `secondary` | `secondary`, `secondaryContainer`, `onSecondary`, `onSecondaryContainer` |
| `surface` | `surface`, `surfaceSecondary`, `onSurface`, `onSurfaceSecondary` |
| `error` | `error`, `onError` |
| _(ungrouped)_ | `outline`, `shadow`, and any others |

### Required Extensions

The color file must include these two extensions after the enum. They provide
the `Color(light:dark:)` initializer used by every color token.

```swift
// MARK: - Color Initializer from Hex

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

public enum VectisColors {
    public static let primary = Color(light: "#007AFF", dark: "#0A84FF")
    public static let primaryContainer = Color(light: "#D6E4FF", dark: "#003A70")
    public static let onPrimary = Color(light: "#FFFFFF", dark: "#FFFFFF")
    public static let onPrimaryContainer = Color(light: "#001D36", dark: "#D6E4FF")

    public static let secondary = Color(light: "#5856D6", dark: "#5E5CE6")
    public static let secondaryContainer = Color(light: "#E8E0FF", dark: "#2F2D6E")
    public static let onSecondary = Color(light: "#FFFFFF", dark: "#FFFFFF")
    public static let onSecondaryContainer = Color(light: "#1C1B33", dark: "#E8E0FF")

    public static let surface = Color(light: "#FFFFFF", dark: "#1C1C1E")
    public static let surfaceSecondary = Color(light: "#F2F2F7", dark: "#2C2C2E")
    public static let onSurface = Color(light: "#000000", dark: "#FFFFFF")
    public static let onSurfaceSecondary = Color(light: "#3C3C43", dark: "#EBEBF5")

    public static let error = Color(light: "#FF3B30", dark: "#FF453A")
    public static let onError = Color(light: "#FFFFFF", dark: "#FFFFFF")

    public static let outline = Color(light: "#C6C6C8", dark: "#38383A")
    public static let shadow = Color(light: "#000000", dark: "#000000")
}

// MARK: - Color Initializer from Hex

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
public static let {name} = Font.system(size: {size}, weight: .{weight})
```

### Weight Mapping

| YAML value | Swift value |
|---|---|
| `ultraLight` | `.ultraLight` |
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

public enum VectisTypography {
    public static let largeTitle = Font.system(size: 34, weight: .bold)
    public static let title = Font.system(size: 28, weight: .bold)
    public static let title2 = Font.system(size: 22, weight: .bold)
    public static let title3 = Font.system(size: 20, weight: .semibold)
    public static let headline = Font.system(size: 17, weight: .semibold)
    public static let body = Font.system(size: 17, weight: .regular)
    public static let callout = Font.system(size: 16, weight: .regular)
    public static let subheadline = Font.system(size: 15, weight: .regular)
    public static let footnote = Font.system(size: 13, weight: .regular)
    public static let caption = Font.system(size: 12, weight: .regular)
    public static let caption2 = Font.system(size: 11, weight: .regular)
}
```

## Scalar Template

### Enum

MARK label: `{Category Name} Scale` (e.g., `Spacing Scale`, `Corner Radius Scale`)

Each entry:

```swift
public static let {name}: CGFloat = {value}
```

Values are written as integers when the YAML value is a whole number (e.g., `16`
not `16.0`). If the YAML value has a decimal component, preserve it (e.g., `1.5`).

### Colocated Scalars

`spacing` and `cornerRadius` share `Spacing.swift`. They are written as two
separate enums separated by a blank line and a MARK comment:

```swift
import SwiftUI

// MARK: - Spacing Scale
// Generated from design-system/tokens.yaml — do not edit manually.

public enum VectisSpacing {
    public static let xxs: CGFloat = 2
    public static let xs: CGFloat = 4
    public static let sm: CGFloat = 8
    public static let md: CGFloat = 16
    public static let lg: CGFloat = 24
    public static let xl: CGFloat = 32
    public static let xxl: CGFloat = 48
}

// MARK: - Corner Radius Scale

public enum VectisCornerRadius {
    public static let none: CGFloat = 0
    public static let sm: CGFloat = 4
    public static let md: CGFloat = 8
    public static let lg: CGFloat = 12
    public static let xl: CGFloat = 16
    public static let full: CGFloat = 9999
}
```

New scalar categories (e.g., `elevation`, `opacity`) get their own file
unless explicitly colocated.

## Theme Template

`Theme.swift` is structural scaffolding that references all generated enums.

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
public struct VectisTheme: Sendable {
    {one property per category}

    public init() {}
}

// MARK: - Environment Key

private struct VectisThemeKey: EnvironmentKey {
    static let defaultValue = VectisTheme()
}

extension EnvironmentValues {
    public var vectisTheme: VectisTheme {
        get { self[VectisThemeKey.self] }
        set { self[VectisThemeKey.self] = newValue }
    }
}

// MARK: - View Modifier

extension View {
    /// Injects the Vectis design system theme into the view hierarchy.
    public func vectisTheme() -> some View {
        environment(\.vectisTheme, VectisTheme())
    }
}
```

### Theme Property Pattern

Each category gets one property line:

```swift
public let {camelCaseCategory}: Vectis{PascalCaseCategory}.Type = Vectis{PascalCaseCategory}.self
```

For the current four categories:

```swift
public let colors: VectisColors.Type = VectisColors.self
public let typography: VectisTypography.Type = VectisTypography.self
public let spacing: VectisSpacing.Type = VectisSpacing.self
public let cornerRadius: VectisCornerRadius.Type = VectisCornerRadius.self
```

## Package.swift Template

Only generated when the file does not exist. Never overwritten.

```swift
// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "VectisDesign",
    platforms: [
        .iOS(.v17),
        .macOS(.v14),
    ],
    products: [
        .library(name: "VectisDesign", targets: ["VectisDesign"]),
    ],
    targets: [
        .target(name: "VectisDesign"),
    ]
)
```

## YAML-to-File Mapping Summary

| YAML key | Value shape | Swift enum | File | MARK label |
|---|---|---|---|---|
| `colors` | Color | `VectisColors` | `Colors.swift` | `Semantic Colors` |
| `typography` | Font | `VectisTypography` | `Typography.swift` | `Typography Scale` |
| `spacing` | Scalar | `VectisSpacing` | `Spacing.swift` | `Spacing Scale` |
| `cornerRadius` | Scalar | `VectisCornerRadius` | `Spacing.swift` | `Corner Radius Scale` |
| _(new scalar)_ | Scalar | `Vectis{Name}` | `{Name}.swift` | `{Name} Scale` |
| _(new color)_ | Color | `Vectis{Name}` | `{Name}.swift` | `{Name}` |
| _(new font)_ | Font | `Vectis{Name}` | `{Name}.swift` | `{Name} Scale` |
