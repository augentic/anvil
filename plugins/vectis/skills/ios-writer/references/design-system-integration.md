# Design System Integration

How to use the VectisDesign Swift Package in generated iOS shell views.

## Package Dependency

The VectisDesign package lives at `design-system/ios/` in the repo root.
The XcodeGen `project.yml` references it as a local package:

```yaml
packages:
  VectisDesign:
    path: ../../../design-system/ios
```

The path is relative from the `iOS/` directory. Adjust if the app is at a
different depth in the repo.

## Importing

Add the import at the top of every view file:

```swift
import VectisDesign
```

## Using Color Tokens

Access colors via the `VectisColors` enum:

```swift
Text("Hello")
    .foregroundStyle(VectisColors.onSurface)

Rectangle()
    .fill(VectisColors.primary)

Button("Delete") { ... }
    .tint(VectisColors.error)
```

Colors automatically adapt to light/dark mode. Never use hardcoded
`Color(red:green:blue:)` or `Color("name")` in generated views.

## Using Typography Tokens

Access fonts via the `VectisTypography` enum:

```swift
Text("Title")
    .font(VectisTypography.title)

Text("Body text")
    .font(VectisTypography.body)

Text("Caption")
    .font(VectisTypography.caption)
```

## Using Spacing Tokens

Access spacing values via the `VectisSpacing` enum:

```swift
VStack(spacing: VectisSpacing.md) {
    // children spaced 16pt apart
}

.padding(.horizontal, VectisSpacing.md)
.padding(.vertical, VectisSpacing.sm)
```

## Using Corner Radius Tokens

Access corner radius values via the `VectisCornerRadius` enum:

```swift
RoundedRectangle(cornerRadius: VectisCornerRadius.md)

.clipShape(RoundedRectangle(cornerRadius: VectisCornerRadius.lg))
```

## Theme Environment (Optional)

For views that need access to the full theme bundle:

```swift
// Apply at app root
ContentView(core: core)
    .vectisTheme()

// Access in any descendant view
@Environment(\.vectisTheme) private var theme
```

This is optional -- most views should use the static `VectisColors`,
`VectisTypography`, and `VectisSpacing` directly.

## Disabled State Convention

For disabled interactive elements, apply 38% opacity to the normal color:

```swift
.foregroundStyle(VectisColors.primary.opacity(isDisabled ? 0.38 : 1.0))
```

## Icons

Use SF Symbols with design system colors:

```swift
Image(systemName: "plus.circle.fill")
    .foregroundStyle(VectisColors.primary)

Image(systemName: "exclamationmark.triangle")
    .foregroundStyle(VectisColors.error)
```

Icon size should match the adjacent text style. Use `.font()` to control:

```swift
Image(systemName: "checkmark")
    .font(VectisTypography.body)
```

## Review Compliance

The ios-reviewer skill checks that generated views:

1. Use `VectisColors` for all color references (no hardcoded hex/RGB).
2. Use `VectisTypography` for all font references (no inline `.system(size:)`).
3. Use `VectisSpacing` for padding and spacing values (no magic numbers).
4. Use `VectisCornerRadius` for corner radius values.

Exceptions are allowed for system-provided styles (e.g., `.buttonStyle(.borderedProminent)`)
where the platform applies its own colors.
