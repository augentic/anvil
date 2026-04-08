# Design System Integration

How to use design system tokens in generated Android shell composables.

## Token Source

Design tokens live in `design-system/tokens.yaml` at the repo root. The
tokens define color, typography, spacing, and corner radius values that
are shared across all platform shells.

When a design system is available, the android-writer generates a Compose
theme that maps these tokens to Material 3 theme values.

## Using Color Tokens

When a design system is present, map token colors to the Material 3
`ColorScheme` in `ui/theme/Color.kt` and `ui/theme/Theme.kt`:

```kotlin
// Color.kt
val VectisPrimary = Color(0xFF007AFF)       // from tokens.yaml colors.primary.light
val VectisPrimaryDark = Color(0xFF0A84FF)   // from tokens.yaml colors.primary.dark
val VectisOnPrimary = Color(0xFFFFFFFF)     // from tokens.yaml colors.onPrimary.light

// Theme.kt
private val LightColorScheme = lightColorScheme(
    primary = VectisPrimary,
    onPrimary = VectisOnPrimary,
    // ... map all token colors
)
```

Access colors in composables via `MaterialTheme.colorScheme`:

```kotlin
Text(
    text = "Hello",
    color = MaterialTheme.colorScheme.onSurface
)

Surface(color = MaterialTheme.colorScheme.primary) { ... }

Button(
    onClick = { ... },
    colors = ButtonDefaults.buttonColors(
        containerColor = MaterialTheme.colorScheme.error
    )
) { Text("Delete") }
```

Colors automatically adapt to light/dark mode when the theme switches.
Never use hardcoded `Color(0xFF...)` in screen composables.

## Using Typography Tokens

Map token typography to Material 3 `Typography` in `ui/theme/Type.kt`:

```kotlin
val AppTypography = Typography(
    titleLarge = TextStyle(
        fontSize = 28.sp,       // from tokens.yaml typography.title.size
        fontWeight = FontWeight.Bold
    ),
    bodyLarge = TextStyle(
        fontSize = 16.sp,       // from tokens.yaml typography.body.size
        fontWeight = FontWeight.Normal
    ),
    labelSmall = TextStyle(
        fontSize = 12.sp,       // from tokens.yaml typography.caption.size
        fontWeight = FontWeight.Normal
    )
)
```

Access in composables via `MaterialTheme.typography`:

```kotlin
Text(
    text = "Title",
    style = MaterialTheme.typography.titleLarge
)

Text(
    text = "Body text",
    style = MaterialTheme.typography.bodyLarge
)
```

## Using Spacing Tokens

Define spacing constants derived from `tokens.yaml`:

```kotlin
object VectisSpacing {
    val xs = 4.dp    // from tokens.yaml spacing.xs
    val sm = 8.dp    // from tokens.yaml spacing.sm
    val md = 16.dp   // from tokens.yaml spacing.md
    val lg = 24.dp   // from tokens.yaml spacing.lg
    val xl = 32.dp   // from tokens.yaml spacing.xl
}
```

Use in composables:

```kotlin
Column(
    verticalArrangement = Arrangement.spacedBy(VectisSpacing.md)
) {
    // children spaced 16dp apart
}

Modifier
    .padding(horizontal = VectisSpacing.md)
    .padding(vertical = VectisSpacing.sm)
```

## Using Corner Radius Tokens

Define corner radius constants derived from `tokens.yaml`:

```kotlin
object VectisCornerRadius {
    val sm = 4.dp    // from tokens.yaml cornerRadius.sm
    val md = 8.dp    // from tokens.yaml cornerRadius.md
    val lg = 16.dp   // from tokens.yaml cornerRadius.lg
    val xl = 24.dp   // from tokens.yaml cornerRadius.xl
}
```

Use in composables:

```kotlin
Surface(
    shape = RoundedCornerShape(VectisCornerRadius.md)
) { ... }

Modifier.clip(RoundedCornerShape(VectisCornerRadius.lg))
```

## Fallback When No Design System

When `design-system/tokens.yaml` does not exist, the android-writer
generates composables using Material 3 defaults without custom token
values. The generated theme uses `dynamicLightColorScheme` /
`dynamicDarkColorScheme` on Android 12+ and falls back to a default
Material 3 color scheme on older versions.

## Disabled State Convention

For disabled interactive elements, apply 38% alpha to the normal color:

```kotlin
Text(
    text = "Disabled",
    color = MaterialTheme.colorScheme.primary.copy(alpha = if (isDisabled) 0.38f else 1f)
)
```

## Icons

Use Material Icons with theme colors:

```kotlin
Icon(
    imageVector = Icons.Default.Add,
    contentDescription = "Add item",
    tint = MaterialTheme.colorScheme.primary
)

Icon(
    imageVector = Icons.Default.Warning,
    contentDescription = "Error",
    tint = MaterialTheme.colorScheme.error
)
```

## Review Compliance

The android-reviewer skill checks that generated composables:

1. Use `MaterialTheme.colorScheme` for all color references (no hardcoded hex).
2. Use `MaterialTheme.typography` for all font references (no inline `TextStyle`).
3. Use `VectisSpacing` for padding and spacing values (no magic numbers).
4. Use `VectisCornerRadius` for corner radius values.

Exceptions are allowed for Material 3 component defaults where the
platform applies its own colors.
