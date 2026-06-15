# Design System Integration

How the Android writer integrates `tokens.yaml` and `assets.yaml` into a
generated Compose shell. Tokens become **shell-local** Theme code under
`Android/app/src/main/java/com/vectis/<appname>/ui/theme/`; referenced
asset files are **copied** into `Android/app/src/main/res/drawable*/`,
`mipmap*/`, and `raw/` during generation. There is no separate
`:vectis-design` Gradle module, no
`implementation(project(":vectis-design"))` dependency, and no path back
into `design-system/android/` from the rendered shell project.

This file complements [`kotlin-token-templates.md`](token-templates.md),
which carries the concrete code templates per token shape.

## Authority hierarchy

When this document conflicts with another source, follow this precedence:

1. `tokens.yaml` and `assets.yaml` — the operator-owned input artifacts.
2. [Layout Inferer Contract](../layout-inferer-contract.md), [Component Catalog](../spec-runtime/components.md), and the Vectis schemas/tool validators.
3. [`kotlin-token-templates.md`](token-templates.md) — concrete code
   templates per token category.
4. This document — integration policy and fallback rules.

## Generated layout

When `tokens.yaml` is present, the Android writer emits theme code under
the app module's package tree:

```
Android/
├── build.gradle.kts
├── settings.gradle.kts
├── app/
│   ├── build.gradle.kts
│   └── src/main/
│       ├── AndroidManifest.xml
│       ├── java/com/vectis/<appname>/
│       │   ├── <AppName>Application.kt
│       │   ├── MainActivity.kt
│       │   ├── core/
│       │   │   └── Core.kt
│       │   ├── ui/
│       │   │   ├── screens/...
│       │   │   ├── components/                # one file per component: <slug>
│       │   │   │   └── TaskRow.kt
│       │   │   └── theme/                     # generated from tokens.yaml
│       │   │       ├── Colors.kt
│       │   │       ├── Typography.kt
│       │   │       ├── Spacing.kt
│       │   │       ├── (Elevation.kt, Border.kt, Opacity.kt, … as needed)
│       │   │       └── Theme.kt
│       │   └── di/...
│       └── res/                               # copied from assets/exports/android/
│           ├── drawable-mdpi/<asset-id>.png
│           ├── drawable-hdpi/<asset-id>.png
│           ├── drawable-xhdpi/<asset-id>.png
│           ├── drawable-xxhdpi/<asset-id>.png
│           ├── drawable-xxxhdpi/<asset-id>.png
│           ├── drawable/<asset-id>.xml         # vector drawable (icon / decorative)
│           ├── mipmap-*/ic_launcher*.png       # app-icon only
│           ├── mipmap-anydpi-v26/ic_launcher*.xml
│           └── values/themes.xml
└── shared/
    └── build.gradle.kts
```

The `specify tool run vectis -- scaffold android <AppName>` render step produces the package
tree (`<AppName>Application.kt`, `MainActivity.kt`, `core/Core.kt`, the
starter `ui/screens/HomeScreen.kt`), the `res/values/themes.xml` baseline,
and the Gradle / manifest / version-catalog wiring. The Android writer
adds `ui/components/`, `ui/theme/`, and the per-density drawable
directories on first generation when the corresponding input artifacts
exist. The Android Gradle plugin already lists `app/src/main/java/` as a
source root; nested directories are picked up automatically — no
`build.gradle.kts` edits are required when adding new theme or component
files.

The generated app **MUST NOT** depend on
`implementation(project(":vectis-design"))` and **MUST NOT** declare an
`include(":vectis-design")` line in `settings.gradle.kts`, an
`implementation` dependency on a `com.vectis.design` AAR, or a path under
`design-system/android/` (per the generated-layout policy below). The
`Android/` shell must build from its own platform directory after
generation.

## Token integration

### Reading `tokens.yaml`

The Android writer's primary token input is `tokens.yaml`. Resolution
order follows the Vectis input policy:

1. Slice-local `.specify/slices/<name>/tokens.yaml`, when present.
2. Project-level `design-system/tokens.yaml`.
3. Neither — fall through to the Material 3 fallback policy below.

When `tokens.yaml` is present, generate one Theme file per category under
`Android/app/src/main/java/com/vectis/<appname>/ui/theme/` per
[`kotlin-token-templates.md`](token-templates.md). The token file
generation is mechanical: each YAML category maps to either an M3
constructor slot (colors, typography) or a top-level `object` keyed by
the camelCased token id (spacing, cornerRadius, elevation, opacity).
Adding a new category extends both `kotlin-token-templates.md` and this
document.

### Using token references in views

Reference Theme types from screen composables and components —
they are part of the same Gradle module as the views that consume them,
so no external Gradle dependency is needed. However, because theme files
live in `com.vectis.<appname>.ui.theme` while screens and components live
in sibling packages (`ui.screens`, `ui.components`), an explicit
`import com.vectis.<appname>.ui.theme.*` is required in each consumer file
(Kotlin only auto-imports within the exact same package):

```kotlin
Text(
    text = "Hello",
    color = MaterialTheme.colorScheme.onSurface,
)

Surface(color = MaterialTheme.colorScheme.primary) { /* ... */ }

Column(
    verticalArrangement = Arrangement.spacedBy(VectisSpacing.md),
) {
    // children spaced 16dp apart
}

Modifier
    .padding(horizontal = VectisSpacing.md)
    .padding(vertical = VectisSpacing.sm)

Surface(
    shape = RoundedCornerShape(VectisCornerRadius.md),
) { /* ... */ }
```

Prefer **`MaterialTheme.colorScheme`** and **`MaterialTheme.typography`**
in composables — `VectisTheme` installs the token-derived scheme and
Typography, so consumer code stays idiomatic Compose:

```kotlin
Text(
    text = "Title",
    style = MaterialTheme.typography.titleLarge,
)

Button(
    onClick = { /* ... */ },
    colors = ButtonDefaults.buttonColors(
        containerColor = MaterialTheme.colorScheme.error,
    ),
) { Text("Delete") }
```

Use `VectisTypography.<token>` directly only when a TextStyle does not
have an M3 slot equivalent (rare). Never emit hardcoded
`Color(0xFF…)`, inline `TextStyle(fontSize = 17.sp, …)`, or magic
numbers in generated views.

`VectisTheme` applies **static** light/dark `ColorScheme` values from
`tokens.yaml` (not Material You dynamic wallpaper colors), matching iOS
`Color(light:dark:)` behavior.

### Disabled state convention

For disabled interactive elements, apply 38% alpha to the normal color:

```kotlin
Text(
    text = "Disabled",
    color = MaterialTheme.colorScheme.primary.copy(
        alpha = if (isDisabled) VectisOpacity.disabled else 1f,
    ),
)
```

When `tokens.yaml` does not define an `opacity.disabled` token, fall back
to the literal `0.38f`.

### Token reference resolution and CLI gate

The deterministic check that every token reference in `composition.yaml`
resolves to a `tokens.yaml` entry lives in
`specify tool run vectis -- validate composition` (via the Vectis validator): when sibling
`tokens.yaml` exists, the validator auto-invokes `tokens` mode and reports
unresolved references as errors before the Android writer is called. The
writer does not need to re-validate references at generation time; it
consumes the already-validated input set.

## Material 3 fallback policy

When `tokens.yaml` is **absent** the Android writer falls back to
platform-native Material 3 defaults instead of emitting a `ui/theme/`
directory (fallback policy belongs to shell writers). The
skill emits a minimal `<AppName>Theme` composable (or rewrites the
scaffold's `Theme.kt`) that wraps `MaterialTheme` with the standard
dynamic / static Material 3 schemes; screen views reference the M3
defaults directly.

Per-category fallback:

| Category | M3 fallback |
|---|---|
| Colors | `MaterialTheme.colorScheme.*` (M3 defaults). On Android 12+ devices use `dynamicLightColorScheme(LocalContext.current)` / `dynamicDarkColorScheme(LocalContext.current)`; on earlier API levels use `lightColorScheme()` / `darkColorScheme()` with no arguments (M3's static defaults). |
| Typography | `MaterialTheme.typography.*` (M3 defaults — `bodyLarge`, `titleLarge`, etc.); the M3 type scale already covers the Material 3 specification. |
| Spacing | Inline `8.dp` (`sm`), `16.dp` (`md`), and `24.dp` (`lg`) literals; or `Arrangement.spacedBy(8.dp)` for stack spacing. |
| Corner radius | Inline `8.dp` for medium, `12.dp` for large, `RoundedCornerShape(...)` calls. |
| Elevation | `CardDefaults.cardElevation()` (M3 defaults) for cards; `Modifier.shadow(2.dp)` for ad-hoc shadows. |
| Opacity | Inline `0.38f` for disabled states, `0.6f` for de-emphasised text. |

The Material 3 fallback `Theme.kt` looks like:

```kotlin
package com.vectis.<appname>.ui.theme

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext

@Composable
fun <AppName>Theme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = true,
    content: @Composable () -> Unit,
) {
    val context = LocalContext.current
    val colorScheme = when {
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S ->
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        darkTheme -> darkColorScheme()
        else -> lightColorScheme()
    }
    MaterialTheme(
        colorScheme = colorScheme,
        content = content,
    )
}
```

When `tokens.yaml` is **present but incomplete** (some categories defined,
others absent), shell writers MAY use the same Material 3 default for the
**absent** categories. Shell writers MUST NOT silently substitute defaults
for a token name that is referenced from `composition.yaml` but missing
from `tokens.yaml` — that condition is an error reported by
`specify tool run vectis -- validate composition` and halts shell generation for the
affected screen. The writer surfaces the validator output verbatim and
declines to emit code that papers over the missing token.

When the M3 fallback is in use, the Android writer prefers Compose's
built-in dynamic color (Android 12+) over hex-coded defaults. This keeps
the no-tokens path operator-friendly: a freshly scaffolded app looks
correct on both light and dark appearances and adapts to wallpaper colors
on Android 12+ without any token authoring.

## Asset integration

### Render-by-`kind`

Shell writers resolve each composition `icon` / `image` / `icon-button` / `fab`
reference through `assets.yaml` and emit view code strictly by entry `kind`:

| `assets.<id>.kind` | Android emission |
|---|---|
| `vector` | `painterResource(R.drawable.<id_snake>)` from a shell-local drawable copied from the materialized export |
| `raster` | `painterResource` from per-density drawables copied from the materialized export |
| `symbol` | `Icon(imageVector = Icons.Default.<glyph>, …)` — no resource copy |

**Forbidden at build time:** emitting `Icons.Default.*` (or any Material Icons
substitute) for an id whose entry is `vector` or `raster`. Missing platform
exports are validation errors (`assets-materialization-missing`) — never a
writer shortcut. Platform glyph use requires an explicit `kind: symbol` entry
(optionally `inferred: true` when promoted from screenshot inference; see
[Layout Inferer Contract](../layout-inferer-contract.md) and
`adapters/sources/screenshots/briefs/extract.md`).

### Reading `assets.yaml`

The Android writer's primary asset input is `assets.yaml`. Resolution
order follows the Vectis input policy:

1. Slice-local `.specify/slices/<name>/assets.yaml`, when present, plus
   files under `.specify/slices/<name>/assets/`.
2. Project-level `design-system/assets.yaml` plus files under
   `design-system/assets/`.
3. Neither — generate composables without referenced asset entries (any
   composition that references an asset id will already have failed
   validation at the CLI gate).

The deterministic check that every asset reference in `composition.yaml`
resolves to an `assets.yaml` entry lives in
`specify tool run vectis -- validate composition` (auto-invokes `assets` mode when
present). Missing files are errors; missing optional densities are
warnings (per Phase 1.7). The writer consumes the already-validated input
set.

### Materialize-before-copy

Canonical masters live under `design-system/assets/` (`source:` on each entry).
Per-platform binaries live under `design-system/assets/exports/android/` and are
recorded in `sources.android` (operator-pinned or auto-written by
`vectis materialize assets`). Materialization runs automatically at
`specify slice build --phase prepare` for in-scope assets with missing exports;
operators may also run `specify tool run vectis -- materialize assets` manually
after editing canonical masters. Committed `exports/` trees are version-controlled
— CI and shell builds consume them without re-running materialize on every job.

Build hand-off is **materialize-then-copy**: the Android writer **copies**
files from each entry's resolved `sources.android` export path(s) into the app
module's `res/` tree at generation time. The canonical `source:` file is
provenance only — never copied into the shell. The generated shell project must
build from its own platform directory after generation; it MUST NOT symlink,
alias, or path-reference `design-system/assets/` from `build.gradle.kts`, nor
consume files from `<change>/assets/` at runtime. Per-platform copy targets (paths relative to `design-system/`; materialize writes under `assets/exports/android/`):

| `role` + `kind` | Export tree read (`sources.android` pin) | Shell `res/` target |
|---|---|---|
| `icon` or `decorative` + `vector` | `drawable/<id_snake>.xml` (SVG master materialized to Vector Drawable; `.svg` is never an Android resource) | `res/drawable/<asset-id_snake>.xml`. |
| `illustration` + `vector` | `drawable-{mdpi,hdpi,xhdpi,xxhdpi,xxxhdpi}/<id_snake>.png` (one PNG per density bucket) | Matching `res/drawable-<density>/<asset-id_snake>.png` files. |
| `photo` or UI `icon` + `raster` | `{mdpi,hdpi,xhdpi,xxhdpi,xxxhdpi}` under operator-pinned bucket paths (materialize does not invent density ladders) | `res/drawable-<density>/<asset-id_snake>.png` (or `.jpg`) — one file per declared bucket. |
| `symbol` (any role) | `symbols.android` | No resource copy — emit `Icons.Default.<glyph>` (or an explicit `material-icons-extended` reference) at the call site. |
| `app-icon` | `assets/exports/android/app-icon/` directory pin (`mipmap-*/`, `mipmap-anydpi-v26/`, `drawable-*/ic_launcher_foreground.png`, `values/ic_launcher_background.xml`; path A auto-convert or path B operator pin) | Copy the export tree into matching `res/` locations; scaffold ships empty adaptive stubs materialize fills. |

Reference the copied asset by its kebab-case asset id at the call site:

```kotlin
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.ui.res.painterResource

// raster / vector
Image(
    painter = painterResource(id = R.drawable.onboarding_hero),
    contentDescription = "Onboarding illustration",
)

// symbol entry's symbols.android value
Icon(
    imageVector = Icons.Default.Close,
    contentDescription = "Close",
    tint = MaterialTheme.colorScheme.onSurface,
)
```

Android resource ids are lowercase-with-underscores. Asset ids in
`assets.yaml` are kebab-case (e.g. `onboarding-hero`); the Android writer
translates the id to `R.drawable.onboarding_hero` at the call site, and
copies the file into `res/drawable-*/<asset-id-with-underscores>.png`
accordingly.

For symbols the `tint` token (when present in `assets.yaml`) becomes a
`tint = MaterialTheme.colorScheme.<tint>` argument on the `Icon`
composable. Single colour vector drawables MAY also be tinted via
`Modifier.colorFilter(...)` when the drawable's path has
`android:fillColor="?attr/colorControlNormal"` or similar attribute
indirection.

### Missing platform exports

When a `vector` or `raster` asset is referenced from `composition.yaml` but
`sources.android` is missing or the pinned export path is absent on disk, the
validator reports `assets-materialization-missing` and shell generation halts
for the affected screen. The Android writer does **not** silently fall back to a
Material Icon, generate from the canonical `source:` at build time, or skip the
screen. The legitimate operator responses are to run materialize (or commit
operator-pinned exports under `exports/android/`), re-declare the asset as
`kind: symbol` with an explicit glyph mapping, or remove the reference from
`composition.yaml`.

### Stale resource cleanup

When an asset entry is removed from `assets.yaml`, the Android writer
deletes the corresponding `res/drawable-*/<asset-id>.png` (or `.xml`)
files. Operator-authored resources (e.g. `mipmap-*/ic_launcher*.png`,
custom XML drawables outside the asset id namespace) are preserved; the
writer only deletes entries it generated.

## Component directive contract

When a `composition.yaml` `group` carries `component: <slug>` (per the component directive contract), the Android writer emits **one named `@Composable`** per slug under
`Android/app/src/main/java/com/vectis/<appname>/ui/components/`,
PascalCased from the slug:

| `composition.yaml` slug | Generated file | Composable signature |
|---|---|---|
| `task-row` | `ui/components/TaskRow.kt` | `@Composable fun TaskRow(...)` |
| `news-card` | `ui/components/NewsCard.kt` | `@Composable fun NewsCard(...)` |

Every call site in `composition.yaml` becomes a use of the named
composable. Props are inferred from variation observed across instances
of the slug per the component directive contract:

- `bind`, `event`, `error`, `asset`, token references, `*-when` keys, and
  free text content that **differ** across instances become parameters on
  the generated composable.
- Values that are **constant** across all instances are baked into the
  composable body.

The structural-identity rule (§G) is enforced by
`specify tool run vectis -- validate composition` before the Android writer runs, so
the writer can trust that every instance of the slug shares the same
skeleton and only the wiring varies.

The directive is platform-agnostic; the inferred prop shape is
per-platform. iOS may emit a slightly different prop signature for the
same slug — v1 does not require cross-shell prop agreement (per the platform-local prop-shape policy).

### Component examples

For a `task-row` slug whose instances all carry the same skeleton (a
`Row` of a checkbox, a title `Text`, and a `Spacer`), but whose `bind`,
`event`, and `strikethrough-when` keys vary across screens:

```kotlin
package com.vectis.<appname>.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.outlined.Circle
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextDecoration
import com.vectis.<appname>.ui.theme.VectisSpacing

@Composable
fun TaskRow(
    title: String,
    isCompleted: Boolean,
    onToggle: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(VectisSpacing.sm),
        modifier = modifier
            .padding(horizontal = VectisSpacing.md, vertical = VectisSpacing.sm),
    ) {
        IconButton(onClick = onToggle) {
            Icon(
                imageVector = if (isCompleted) {
                    Icons.Default.CheckCircle
                } else {
                    Icons.Outlined.Circle
                },
                contentDescription = if (isCompleted) "Mark incomplete" else "Mark complete",
                tint = if (isCompleted) {
                    MaterialTheme.colorScheme.primary
                } else {
                    MaterialTheme.colorScheme.onSurfaceVariant
                },
            )
        }

        Text(
            text = title,
            style = MaterialTheme.typography.bodyLarge,
            textDecoration = if (isCompleted) TextDecoration.LineThrough else null,
            color = MaterialTheme.colorScheme.onSurface,
        )
    }
}
```

The call site becomes:

```kotlin
LazyColumn {
    items(viewModel.tasks, key = { it.id }) { task ->
        TaskRow(
            title = task.title,
            isCompleted = task.isCompleted,
            onToggle = { onEvent(Event.Toggle(task.id)) },
        )
    }
}
```

instead of the flattened `Row { ... }` body it would have produced
without the directive.

## Review compliance

The `vectis-android-reviewer` skill checks generated composables for:

1. Token-backed visual literals when `tokens.yaml` is present —
   `MaterialTheme.colorScheme` (or `VectisColors`-equivalent if the
   writer emits a fallback object) for color references,
   `MaterialTheme.typography` (or `VectisTypography`) for text styles,
   `VectisSpacing` for spacing values, `VectisCornerRadius` for corner
   radii, `VectisElevation` for elevation values.
2. **No** stale external design-system dependencies —
   `implementation(project(":vectis-design"))`,
   `include(":vectis-design")`, `import com.vectis.design.*`,
   `design-system/android/`, `design-system/ios/` (per the reviewer surface and generated-layout compatibility policy).
3. Asset references that resolve to entries in the shell-local
   `app/src/main/res/drawable*/` tree (no string-literal paths into
   `design-system/assets/`).
4. Groups that visibly recur in `composition.yaml` without a
   `component:` slug — flagged so the operator can promote them to a
   named component before drift compounds (per the reviewer surface).

When `tokens.yaml` is absent (M3 fallback path), the reviewer accepts
`MaterialTheme.colorScheme.*` slots, `MaterialTheme.typography.*` slots,
`Icons.Default.*` references, and the inline-literal `dp` / `sp` /
`alpha` values listed in the M3 fallback table. Hardcoded
`Color(0xFF…)` outside generated theme files remains a defect even on
the fallback path because the operator can always introduce
`tokens.yaml` later.

Exceptions are allowed for generated theme files (`Colors.kt` legitimately
contains `Color(0xFF…)` produced from YAML, and `Typography.kt`
legitimately contains `TextStyle(fontSize = 17.sp, …)`); the reviewer
detects the `// Generated from design-system/tokens.yaml` header and
skips those files.
