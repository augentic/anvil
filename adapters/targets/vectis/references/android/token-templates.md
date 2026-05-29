# Kotlin / Compose Material 3 Token Templates

Concrete Kotlin code templates the Android writer emits when `tokens.yaml` is
present. The generated files are **shell-local** — they live inside the
Android app module (`Android/app/src/main/java/com/vectis/<appname>/ui/theme/`)
and do **not** form a separate Gradle module. There is no
`implementation(project(":vectis-design"))` dependency, no
`include(":vectis-design")` line in `settings.gradle.kts`, and no path back
into `design-system/android/` from the rendered shell project.

When `tokens.yaml` is **absent**, the Android writer skips this entire emit
step and falls back to platform-native Material 3 defaults — see
[`design-system-integration.md`](design-system-integration.md) for the
fallback policy. This file describes only the present-tokens path.

**Stack**: Jetpack Compose Material 3 (`androidx.compose.material3`), Compose
BOM versions aligned with the app module the CLI scaffolded (see
[`shell-pattern.md`](shell-pattern.md)).

**Package**: `com.vectis.<appname>.ui.theme` (matching the rest of the app
module's package convention). The previous standalone token approach emitted
`package com.vectis.design` because it produced a separate Gradle module;
the shell-local equivalent uses the app's own package tree so no external
Gradle dependency is needed. However, because `ui.theme` is a sibling
package to `ui.screens` and `ui.components`, consumer files must include
an explicit `import com.vectis.<appname>.ui.theme.*` (Kotlin only
auto-imports declarations within the exact same package).

## File layout

Generated under `Android/app/src/main/java/com/vectis/<appname>/ui/theme/`:

| Token category(ies) | Generated file | Notes |
|---|---|---|
| `colors` | `Colors.kt` | One file per category; defines the M3 light/dark `ColorScheme` plus `vectisColor()` parser. |
| `typography` | `Typography.kt` | Defines `object VectisTypography` plus `vectisTypography()` mapping into M3 slots. |
| `spacing` + `cornerRadius` | `Spacing.kt` | Colocated as two `object`s (mirrors Swift `Spacing.swift`). |
| `elevation` | `Elevation.kt` | v1 token category; separate file. |
| `border` | `Border.kt` | New v1 category; uses composite shape. |
| `opacity` | `Opacity.kt` | New v1 category. |
| _any new scalar / color / font category_ | `<Name>.kt` | One file per new category unless the writer explicitly colocates. |
| Theme composable | `Theme.kt` | Structural scaffold; wraps `MaterialTheme` and exposes `VectisTheme` composable. No "Generated from" header. |

The app module's source root is `app/src/main/java/`, set by the CLI scaffold;
nested directories are picked up automatically by the Android Gradle plugin,
so no `build.gradle.kts` edits are required for new theme files.

Every generated **token** file (i.e. everything under `ui/theme/` except
`Theme.kt`) carries this header at the top:

```kotlin
package com.vectis.<appname>.ui.theme

// Generated from design-system/tokens.yaml — do not edit manually.
```

`Theme.kt` is structural scaffolding (the `VectisTheme` composable that wraps
`MaterialTheme`) — it is rewritten on every regeneration to reference the
current enum / object set, but it never carries the "Generated from" header
because operators sometimes extend it with shell-local additions (see the
worked Theme.kt example below).

### Access modifier

Generated declarations use the **default `internal` / public-by-omission**
visibility. They are part of the app module, not a library, so there is no
`public` API surface to expose to other Gradle modules. The previous
standalone token approach emitted top-level declarations with implicit `public`
visibility because it generated a separate library module; the shell-local
equivalent leaves the modifier off entirely. Existing screen composable code
that referenced `VectisColors` / `VectisSpacing` / `VectisCornerRadius` /
`VectisTypography` keeps working unchanged because Kotlin's default
`public` visibility (when no modifier is specified) covers same-module use.

## Shared rules (mirror Swift)

- Preserve token **order** from YAML within each file.
- **Color grouping**: blank lines between semantic groups using the same
  prefix table as
  [`ios/token-templates.md`](../ios/token-templates.md)
  (primary, secondary, surface, error, ungrouped).
- **Weight mapping** (typography): identical to Swift.

| YAML value | Kotlin `FontWeight` |
|---|---|
| `ultra-light` | `FontWeight.ExtraLight` |
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

Color strings in `tokens.yaml` are **`#RRGGBB`** — a `#` prefix plus **6**
hex digits (opaque RGB). This matches the Swift `UIColor(hex:)` template in
[`ios/token-templates.md`](../ios/token-templates.md).

Compose `Color(color: Int)` expects a packed **ARGB** int. Generated code
treats the token as **24-bit RGB** and supplies full opacity by combining
**`0xFF000000`** with the parsed value → **`0xFFRRGGBB`**.

**8-digit `#AARRGGBB`** (alpha in tokens) is **not** supported here; adding
it would require the same parsing rules in Swift and Kotlin so both
platforms stay aligned.

Generated helper (declared at file scope alongside the color scheme builders):

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

## Color Template (`Colors.kt`)

Map each YAML color token to Material 3 `ColorScheme` parameters via
`lightColorScheme(...)` and `darkColorScheme(...)` using **light** and
**dark** hex values respectively.

### Semantic name → `ColorScheme` parameter mapping

YAML token IDs are kebab-case (per [`tokens.schema.json`](https://schemas.specify.dev/vectis/tokens.schema.json)). Kebab-case YAML
keys whose camelCased form matches an M3 `ColorScheme` parameter map
directly: `primary` → `primary`, `on-primary` → `onPrimary`,
`primary-container` → `primaryContainer`,
`on-primary-container` → `onPrimaryContainer`, `secondary` → `secondary`,
…, `error` → `error`, `on-error` → `onError`, `surface` → `surface`,
`on-surface` → `onSurface`, `outline` → `outline`.

| YAML key | `ColorScheme` parameter |
|---|---|
| `surface-secondary` | `surfaceVariant` |
| `on-surface-secondary` | `onSurfaceVariant` |
| `shadow` | `scrim` |
| `outline` | `outline` |

If the YAML adds colors with no M3 slot (rare), add a `val` on a small
`object VectisColors` **or** document the omission; prefer extending
`ColorScheme` usage only when a standard slot exists.

### Skeleton

```kotlin
package com.vectis.<appname>.ui.theme

import androidx.compose.material3.ColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color

// Generated from design-system/tokens.yaml — do not edit manually.

internal fun vectisLightColorScheme(): ColorScheme = lightColorScheme(
    primary = vectisColor("#007AFF"),
    onPrimary = vectisColor("#FFFFFF"),
    // ... all mapped tokens from YAML `light` values
)

internal fun vectisDarkColorScheme(): ColorScheme = darkColorScheme(
    primary = vectisColor("#0A84FF"),
    onPrimary = vectisColor("#FFFFFF"),
    // ... all mapped tokens from YAML `dark` values
)

internal fun vectisColor(hex: String): Color {
    val h = hex.trim().removePrefix("#")
    require(h.length == 6) {
        "Expected #RRGGBB (6 hex digits), got #${h} — see kotlin-token-templates.md"
    }
    val rgb = h.toLong(16).toInt() and 0x00FFFFFF
    return Color(0xFF000000.toInt() or rgb)
}
```

Fill `background` / `onBackground` from `surface` / `onSurface` when the
YAML has no explicit background tokens (common parity with iOS
surface-centric setups).

---

## Typography Template (`Typography.kt`)

1. **`object VectisTypography`** — one `val` per YAML typography token, type
   `TextStyle`, using `sp` and `FontWeight`. Use `FontFamily.Default` (system
   / Material sans) for native Android feel.

```kotlin
package com.vectis.<appname>.ui.theme

import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

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

2. **`fun vectisTypography(): Typography`** — maps token names onto Material
   3 `Typography` constructor slots so `MaterialTheme.typography` matches
   tokens.

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
internal fun vectisTypography(): Typography = Typography(
    displaySmall = VectisTypography.largeTitle,
    titleLarge = VectisTypography.title,
    // ...
)
```

---

## Scalar Template (`Spacing.kt`)

Colocate **spacing** and **cornerRadius** in one file (mirrors Swift
`Spacing.swift`).

```kotlin
package com.vectis.<appname>.ui.theme

import androidx.compose.ui.unit.dp

// Generated from design-system/tokens.yaml — do not edit manually.

// Spacing Scale

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

New scalar categories (`elevation`, `opacity`) get their own file unless
explicitly colocated.

---

## Elevation Template (`Elevation.kt`)

`elevation` is a scalar category — each entry is a `Dp` value matching
Material 3's elevation tokens.

```kotlin
package com.vectis.<appname>.ui.theme

import androidx.compose.ui.unit.dp

// Generated from design-system/tokens.yaml — do not edit manually.

// Elevation Scale

object VectisElevation {
    val none = 0.dp
    val low = 2.dp
    val medium = 4.dp
    val high = 8.dp
}
```

Apply at call sites via `Card(elevation = CardDefaults.cardElevation(defaultElevation = VectisElevation.medium))`
or `Modifier.shadow(elevation = VectisElevation.low)`.

---

## Border Template (`Border.kt`)

`border` is a composite category — each entry carries `width`, `color` (a
reference to a `colors` token), and optional `radius` (a reference to a
`cornerRadius` token).

```kotlin
package com.vectis.<appname>.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

// Generated from design-system/tokens.yaml — do not edit manually.

// Border Scale

data class VectisBorderStyle(
    val width: Dp,
    val color: Color,
    val radius: Dp?,
)

object VectisBorders {
    /**
     * Subtle 1dp outline border. The color resolves at call time so it
     * tracks `MaterialTheme.colorScheme` light/dark switches.
     */
    val subtle: VectisBorderStyle
        @Composable
        @ReadOnlyComposable
        get() = VectisBorderStyle(
            width = 1.dp,
            color = MaterialTheme.colorScheme.outline,
            radius = null,
        )
}
```

Apply at call sites via `Modifier.border(width, color, shape)` — wrap the
shape in `RoundedCornerShape(radius)` when the entry's `radius` is set.

---

## Opacity Template (`Opacity.kt`)

`opacity` is a scalar category in the **`Float`** range `[0.0, 1.0]` — Compose's
`Modifier.alpha()` and `Color.copy(alpha = ...)` both take `Float`. (Swift's
sister template uses `Double` because SwiftUI's `.opacity()` modifier takes
a `Double`; the platforms diverge here on purpose.)

```kotlin
package com.vectis.<appname>.ui.theme

// Generated from design-system/tokens.yaml — do not edit manually.

// Opacity Scale

object VectisOpacity {
    val disabled: Float = 0.38f
    val deemphasised: Float = 0.6f
    val full: Float = 1.0f
}
```

---

## Theme Composable Template (`Theme.kt`)

`Theme.kt` is structural scaffolding that wires the generated `ColorScheme`
and `Typography` into a `MaterialTheme` wrapper. It is regenerated on every
`tokens.yaml` change so the bundle includes only the categories that
currently exist.

Wraps `MaterialTheme` with token-derived `ColorScheme` and `Typography`.
**Do not** use `dynamicLightColorScheme` / `dynamicDarkColorScheme` (Material
You wallpaper colors) when applying Vectis tokens — static light/dark from
YAML preserves parity with iOS `Color(light:dark:)`.

```kotlin
package com.vectis.<appname>.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable

/**
 * Root theme for Vectis apps using `tokens.yaml`. Applies Material 3 with
 * static light/dark schemes from design tokens (not dynamic wallpaper
 * colors).
 *
 * Apply at the activity root:
 *
 * ```kotlin
 * setContent {
 *     VectisTheme {
 *         AppView()
 *     }
 * }
 * ```
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

When `tokens.yaml` adds a new category that the `MaterialTheme` constructor
recognises (e.g. `Shapes` from a future `shapes:` token block), extend the
`MaterialTheme(...)` call here in the same change that lands the new
category template. Categories that do not have an M3 slot (`elevation`,
`border`, `opacity`) are consumed at the call site directly via the
generated `VectisElevation` / `VectisBorders` / `VectisOpacity` objects.

---

## YAML-to-File Mapping Summary

| YAML key | Value shape | Kotlin output | File |
|---|---|---|---|
| `colors` | Color | `vectisLightColorScheme`, `vectisDarkColorScheme`, `vectisColor` | `Colors.kt` |
| `typography` | Font | `VectisTypography`, `vectisTypography()` | `Typography.kt` |
| `spacing` | Scalar | `object VectisSpacing` | `Spacing.kt` |
| `cornerRadius` | Scalar | `object VectisCornerRadius` | `Spacing.kt` |
| `elevation` | Scalar | `object VectisElevation` | `Elevation.kt` |
| `border` | Composite | `object VectisBorders` (+ `VectisBorderStyle` data class) | `Border.kt` |
| `opacity` | Scalar (`Float`) | `object VectisOpacity` | `Opacity.kt` |
| _(new scalar)_ | Scalar | `object Vectis<Name>` | `<Name>.kt` |
| _(new color)_ | Color | extend color scheme mapping or new file | TBD in same change as Swift |

When iOS gains a new value shape or file, extend **both**
[`ios/token-templates.md`](../ios/token-templates.md)
and this file in the same change.

## Removing stale files

When a token category is removed from `tokens.yaml`, the Android writer
deletes the corresponding generated file under
`Android/app/src/main/java/com/vectis/<appname>/ui/theme/`. Files without
the "Generated from" header are operator-owned and never deleted
automatically — that includes `Theme.kt` (which is structural and rewritten
in place, not deleted). When every token category is removed, the writer
rewrites `Theme.kt` to wrap `MaterialTheme` with empty / default arguments
and leaves it on disk so any composable code that wraps content in
`VectisTheme { ... }` keeps compiling.

## Build verification

The standard Android shell build (`make build` → `./gradlew :shared:cargoBuild` →
`./gradlew :app:assembleDebug`, the U8 build-and-verify step in the
Android shell skill) compiles every generated
file as part of the app module. There is no separate
`./gradlew :vectis-design:compileDebugKotlin` step — the previous
standalone token approach emitted a `vectis-design` Gradle library that needed
its own compile pass before the app module could pick it up; the
shell-local equivalent compiles in lockstep with the app sources.

When `tokens.yaml` is **absent**, the Android writer skips this entire
emit step and falls back to platform-native Material 3 defaults — see
[`design-system-integration.md`](design-system-integration.md) for the
fallback policy.
