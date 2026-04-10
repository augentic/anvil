# Kotlin / Compose Material 3 Token Templates

Concrete Kotlin code templates for each token value shape. The design-system-writer
skill uses these templates to generate the **VectisDesign** Android library from
`tokens.yaml`, alongside the Swift package.

**Stack**: Jetpack Compose Material 3 (`androidx.compose.material3`), Compose BOM
versions aligned with the android-writer app template (see
`plugins/vectis/skills/android-writer/references/android-project-config.md`).

**Package**: `com.vectis.design` (default). All public types live in this package
so the app module can `implementation(project(":vectis-design"))` and
`import com.vectis.design.VectisTheme`.

---

## Shared rules (mirror Swift)

- Preserve token **order** from YAML within each file.
- **Color grouping**: blank lines between semantic groups using the same prefix
  table as `swift-token-templates.md` (primary, secondary, surface, error,
  ungrouped).
- **Weight mapping** (typography): identical to Swift.

| YAML value | Kotlin `FontWeight` |
|---|---|
| `ultraLight` | `FontWeight.ExtraLight` |
| `thin` | `FontWeight.Thin` |
| `light` | `FontWeight.Light` |
| `regular` | `FontWeight.Normal` |
| `medium` | `FontWeight.Medium` |
| `semibold` | `FontWeight.SemiBold` |
| `bold` | `FontWeight.Bold` |
| `heavy` | `FontWeight.ExtraBold` |
| `black` | `FontWeight.Black` |

---

## Hex to Compose `Color`

Color strings in `tokens.yaml` are **`#RRGGBB`** — a `#` prefix plus **6** hex
digits (opaque RGB). This matches the Swift `UIColor(hex:)` template in
`swift-token-templates.md`.

Compose `Color(color: Int)` expects a packed **ARGB** int. Generated code treats
the token as **24-bit RGB** and supplies full opacity by combining **`0xFF000000`**
with the parsed value → **`0xFFRRGGBB`**.

**8-digit `#AARRGGBB`** (alpha in tokens) is **not** supported here; adding it
would require the same parsing rules in Swift and Kotlin so both platforms stay
aligned.

Generated helper (internal or private in the same module):

```kotlin
import androidx.compose.ui.graphics.Color

internal fun vectisColor(hex: String): Color {
    val h = hex.trim().removePrefix("#")
    require(h.length == 6) {
        "Expected #RRGGBB (6 hex digits), got #${h} — see kotlin-token-templates.md"
    }
    val rgb = h.toLong(16).toInt() and 0x00FFFFFF
    return Color(0xFF000000.toInt() or rgb)
}
```

---

## ColorScheme template (`VectisColorScheme.kt`)

**File**: `VectisColorScheme.kt`  
**Header** on generated token sections:

```kotlin
// Generated from design-system/tokens.yaml — do not edit manually.
```

Map each YAML color token to Material 3 `ColorScheme` parameters via
`lightColorScheme(...)` and `darkColorScheme(...)` using **light** and **dark**
hex values respectively.

### Semantic name → `ColorScheme` parameter mapping

YAML keys that match M3 names map directly (`primary`, `onPrimary`,
`primaryContainer`, `onPrimaryContainer`, `secondary`, …, `error`, `onError`,
`surface`, `onSurface`, `outline`).

| YAML key | `ColorScheme` parameter |
|---|---|
| `surfaceSecondary` | `surfaceVariant` |
| `onSurfaceSecondary` | `onSurfaceVariant` |
| `shadow` | `scrim` |
| `outline` | `outline` |

If the YAML adds colors with no M3 slot (rare), add a `val` on a small
`object VectisColors` **or** document the omission; prefer extending
`ColorScheme` usage only when a standard slot exists.

### Skeleton

```kotlin
package com.vectis.design

import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color

// Generated from design-system/tokens.yaml — do not edit manually.

fun vectisLightColorScheme(): androidx.compose.material3.ColorScheme = lightColorScheme(
    primary = vectisColor("#007AFF"),
    onPrimary = vectisColor("#FFFFFF"),
    // ... all mapped tokens from YAML `light` values
)

fun vectisDarkColorScheme(): androidx.compose.material3.ColorScheme = darkColorScheme(
    primary = vectisColor("#0A84FF"),
    onPrimary = vectisColor("#FFFFFF"),
    // ... all mapped tokens from YAML `dark` values
)
```

Fill `background` / `onBackground` from `surface` / `onSurface` when the YAML has
no explicit background tokens (common parity with iOS surface-centric setups).

---

## Typography template (`Typography.kt`)

**File**: `Typography.kt`

1. **`object VectisTypography`** — one `val` per YAML typography token, type
   `TextStyle`, using `sp` and `FontWeight`. Use `FontFamily.Default` (system /
   Material sans) for native Android feel.

```kotlin
package com.vectis.design

import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import androidx.compose.material3.Typography

// Generated from design-system/tokens.yaml — do not edit manually.

object VectisTypography {
    val title: TextStyle = TextStyle(
        fontFamily = FontFamily.Default,
        fontWeight = FontWeight.Bold,
        fontSize = 28.sp,
        lineHeight = 34.sp,
        letterSpacing = 0.sp,
    )
    // ... one property per YAML key, preserve order
}
```

2. **`fun vectisTypography(): Typography`** — maps token names onto Material 3
   `Typography` constructor slots so `MaterialTheme.typography` matches tokens.

Default mapping when YAML uses the usual iOS-aligned names:

| YAML key | Material 3 slot |
|---|---|
| `largeTitle` | `displaySmall` |
| `title` | `titleLarge` |
| `title2` | `titleMedium` |
| `title3` | `titleSmall` |
| `headline` | `headlineLarge` |
| `body` | `bodyLarge` |
| `callout` | `bodyMedium` |
| `subheadline` | `bodySmall` |
| `footnote` | `labelMedium` |
| `caption` | `labelSmall` |
| `caption2` | `labelSmall` (or `lineHeight` tweak) |

For YAML keys not in the table, assign to the nearest slot or duplicate
`bodyLarge`; document in a short KDoc on `vectisTypography()`.

```kotlin
fun vectisTypography(): Typography = Typography(
    displaySmall = VectisTypography.largeTitle,
    titleLarge = VectisTypography.title,
    // ...
)
```

---

## Scalar template (`Spacing.kt`)

**File**: `Spacing.kt` — colocate **spacing** and **cornerRadius** in one file
(mirrors Swift `Spacing.swift`).

```kotlin
package com.vectis.design

import androidx.compose.ui.unit.dp

// MARK equivalent: section comments

// Generated from design-system/tokens.yaml — do not edit manually.

object VectisSpacing {
    val md = 16.dp
    // ... preserve YAML order; use whole numbers as `N.dp`, decimals as `N.N.dp`
}

// Corner Radius Scale

object VectisCornerRadius {
    val md = 8.dp
    // ...
}
```

---

## Theme composable template (`Theme.kt`)

**File**: `Theme.kt` — structural scaffold (no "Generated from" comment, same
convention as Swift `Theme.swift`).

Wraps `MaterialTheme` with token-derived `ColorScheme` and `Typography`.
**Do not** use dynamic / Material You color when applying Vectis tokens — static
light/dark from YAML preserves parity with iOS `Color(light:dark:)`.

```kotlin
package com.vectis.design

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable

/**
 * Root theme for Vectis apps using [tokens.yaml]. Applies Material 3 with static
 * light/dark schemes from design tokens (not dynamic wallpaper colors).
 */
@Composable
fun VectisTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    val colorScheme = if (darkTheme) vectisDarkColorScheme() else vectisLightColorScheme()
    MaterialTheme(
        colorScheme = colorScheme,
        typography = vectisTypography(),
        content = content,
    )
}
```

---

## Android library `build.gradle.kts` template

**Only generate when** `{android-output-dir}/build.gradle.kts` does not exist.
If it exists, **do not overwrite** (mirror `Package.swift` rule).

Use the same `compileSdk`, `minSdk`, JVM target, and Compose BOM pattern as the
app module in `android-project-config.md`:

```kotlin
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "com.vectis.design"
    compileSdk { version = release(36) }

    defaultConfig {
        minSdk = 34
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    kotlin {
        compilerOptions {
            jvmTarget = JvmTarget.JVM_11
        }
    }

    buildFeatures {
        compose = true
    }
}

dependencies {
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.ui)
    implementation(libs.androidx.material3)
}
```

The library does **not** depend on `:app` or `:shared`.

---

## YAML-to-file mapping summary (Android)

| YAML key | Value shape | Kotlin output | File |
|---|---|---|---|
| `colors` | Color | `vectisLightColorScheme`, `vectisDarkColorScheme`, `vectisColor` | `VectisColorScheme.kt` |
| `typography` | Font | `VectisTypography`, `vectisTypography()` | `Typography.kt` |
| `spacing` | Scalar | `object VectisSpacing` | `Spacing.kt` |
| `cornerRadius` | Scalar | `object VectisCornerRadius` | `Spacing.kt` |
| _(new scalar)_ | Scalar | `object Vectis{Name}` | `{Name}.kt` |
| _(new color)_ | Color | extend color scheme mapping or new file | TBD in same change as Swift |

When iOS gains a new value shape or file, extend **both**
`swift-token-templates.md` and this file in the same change.

---

## Gradle verification

From the Android project directory (sibling of `design-system/`):

```bash
./gradlew :vectis-design:compileDebugKotlin
```

`settings.gradle.kts` must include:

```kotlin
include(":vectis-design")
project(":vectis-design").projectDir = file("../design-system/android")
```

Adjust the relative path if the Android project is not one level below the repo
root (same idea as the iOS `project.yml` path to `design-system/ios`).
