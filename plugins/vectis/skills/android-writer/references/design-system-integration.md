# Design System Integration

How to use the **VectisDesign** Android library in generated shell composables.

## Token source

Design tokens live in `design-system/tokens.yaml` at the repo root. The **design-system-writer** skill regenerates:

- `design-system/ios/` — Swift Package `VectisDesign` (SwiftUI)
- `design-system/android/` — Gradle module `vectis-design` (Jetpack Compose Material 3)

Both are mechanical outputs from the same YAML. The Android app does **not** duplicate hex or `sp` literals in `ui/theme/Color.kt` or `Type.kt` when this library is present.

## Gradle wiring

From the Android project directory (typically `{workspace}/Android/`):

1. **`settings.gradle.kts`** — include the library and point `projectDir` at the generated module (adjust the relative path to match the repo layout):

   ```kotlin
   include(":vectis-design")
   project(":vectis-design").projectDir = file("../design-system/android")
   ```

2. **`app/build.gradle.kts`** — depend on the module (same version catalog / Compose BOM as `app`; the library’s `build.gradle.kts` uses the same BOM pattern):

   ```kotlin
   implementation(project(":vectis-design"))
   ```

See `android-project-config.md` for full Gradle templates.

## App theme

When tokens exist, the app exposes only a thin `AppTheme` in `ui/theme/Theme.kt` that delegates to `VectisTheme`:

```kotlin
import androidx.compose.runtime.Composable
import com.vectis.design.VectisTheme

@Composable
fun AppTheme(content: @Composable () -> Unit) {
    VectisTheme(content = content)
}
```

`VectisTheme` applies **static** light/dark `ColorScheme` values from `tokens.yaml` (not Material You dynamic wallpaper colors), matching iOS `Color(light:dark:)` behavior.

## Using colors

Prefer **`MaterialTheme.colorScheme`** in composables — `VectisTheme` installs the token-derived scheme:

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

Do not use hardcoded `Color(0xFF...)` in `app/` screen code; hex appears only inside the generated `design-system/android/` library.

## Using typography

Use **`MaterialTheme.typography`** — slots are filled from YAML via `vectisTypography()` inside `VectisTheme`:

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

For a **direct** token match to Swift’s `VectisTypography.title`, you may use `com.vectis.design.VectisTypography.title` as a `TextStyle` when needed.

## Using spacing and corner radius

Import the generated objects (default package `com.vectis.design`):

```kotlin
import com.vectis.design.VectisCornerRadius
import com.vectis.design.VectisSpacing
```

```kotlin
Column(
    verticalArrangement = Arrangement.spacedBy(VectisSpacing.md)
) {
    // ...
}

Modifier
    .padding(horizontal = VectisSpacing.md)
    .padding(vertical = VectisSpacing.sm)

Surface(
    shape = RoundedCornerShape(VectisCornerRadius.md)
) { ... }
```

## Fallback when no design system

When `design-system/tokens.yaml` does not exist, generate composables using Material 3 defaults in `ui/theme/` (`Color.kt`, `Type.kt`, `Theme.kt` with `dynamicLightColorScheme` / `dynamicDarkColorScheme` on Android 12+), as described in `compose-view-patterns.md`.

## Disabled state convention

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
```

## Review compliance

The android-reviewer skill expects:

1. `MaterialTheme.colorScheme` for semantic colors in **app** sources (no hardcoded hex in `app/...`).
2. `MaterialTheme.typography` (or `VectisTypography` tokens) for text styles.
3. `VectisSpacing` / `VectisCornerRadius` from `com.vectis.design` for layout metrics.

The generated library under `design-system/android/` may contain `Color(0xFF...)` produced from YAML; that is expected.
