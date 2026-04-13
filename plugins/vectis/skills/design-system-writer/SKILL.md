---
name: design-system-writer
description: Generate or update the platform-specific design system implementation from tokens.yaml for iOS (Swift Package) and Android (Jetpack Compose Material 3 library). Use when implementing design-system tasks from a Specify change, or when the user mentions design-system-writer.
---

# Design System Writer

Generate (or regenerate) the platform-specific design system code from
`tokens.yaml` for **both**:

1. **iOS** — Swift files under `design-system/ios/` (`VectisDesign` Swift
   Package), verified with `swift build`.
2. **Android** — Kotlin sources under `design-system/android/` (`vectis-design`
   Gradle library module using Compose Material 3), verified with
   `./gradlew :vectis-design:compileDebugKotlin` from the Android project.

Generated token files carry a "do not edit manually" comment (except iOS
`Theme.swift` and Android `Theme.kt`, which are structural scaffolds without
that header — same rule as Swift).

Unlike core-writer or ios-writer, there is no create vs update distinction.
The mapping from YAML to Swift and Kotlin is mechanical and deterministic — the
skill always regenerates token outputs from scratch for both platforms.

## Arguments

| Argument | Required | Description |
|---|---|---|
| `tokens-file` | No | Path to the tokens YAML file. Defaults to `design-system/tokens.yaml` |
| `output-dir` | No | Path to the iOS Swift Package directory. Defaults to `design-system/ios` |
| `android-output-dir` | No | Path to the Android library module root. Defaults to `design-system/android` |
| `android-package` | No | Kotlin package for generated sources. Defaults to `com.vectis.design` |
| `change-dir` | No | Path to a Specify change directory. When provided, the skill reads the `## Design System Requirements` section from `{change-dir}/specs/{feature-name}/spec.md` for context on what token changes are expected. |

## Process

### 1. Read and inventory tokens

Read the tokens YAML file. Build an inventory of every top-level key (excluding
`version`). Each key is a **token category** that maps to one Swift enum **and**
the corresponding Kotlin artifact (see table below).

| YAML key | Swift enum | Kotlin | File(s) |
|---|---|---|---|
| `colors` | `VectisColors` | `vectisLightColorScheme()`, `vectisDarkColorScheme()`, `vectisColor()` | iOS: `Colors.swift` / Android: `VectisColorScheme.kt` |
| `typography` | `VectisTypography` | `VectisTypography` object + `vectisTypography()` | `Typography.swift` / `Typography.kt` |
| `spacing` | `VectisSpacing` | `VectisSpacing` object | `Spacing.swift` (first section) / `Spacing.kt` |
| `cornerRadius` | `VectisCornerRadius` | `VectisCornerRadius` object | `Spacing.swift` (second section) / `Spacing.kt` |

If a new top-level key appears in the YAML (e.g., `elevation`, `opacity`,
`animation`), generate new Swift **and** Kotlin artifacts using the appropriate
value-shape mapping (see step 2). Extend
`references/swift-token-templates.md` and `references/kotlin-token-templates.md`
together when introducing a new shape.

### 2. Classify value shapes

Each token category has a **value shape** that determines code patterns on both
platforms. Detect the shape from the first entry in the category:

| Shape | Detection | Swift pattern | Kotlin pattern |
|---|---|---|---|
| **Color** | Values have `light` and `dark` keys | `Color(light:dark:)` static | `vectisColor` + `lightColorScheme` / `darkColorScheme` |
| **Font** | Values have `size` and `weight` keys | `Font.system(size:weight:)` static | `TextStyle` + `vectisTypography(): Typography` |
| **Scalar** | Values are plain numbers | `CGFloat` static | `Dp` in `object` |

Read `references/swift-token-templates.md` and
`references/kotlin-token-templates.md` for exact templates, weight mapping, and
Material 3 `ColorScheme` field mapping (`surfaceSecondary` → `surfaceVariant`,
`shadow` → `scrim`, etc.).

---

## iOS generation

### 3. Generate Swift token files

For each token category, generate or overwrite the corresponding Swift file
under `{output-dir}/Sources/VectisDesign/`. This step covers token enum files
only — `Theme.swift` is handled separately in step 5.

**File naming rules:**

- Convert the YAML key to PascalCase for the filename (e.g., `colors` →
  `Colors.swift`, `typography` → `Typography.swift`).
- Exception: `spacing` and `cornerRadius` are colocated in `Spacing.swift` as
  two separate enums (`VectisSpacing` and `VectisCornerRadius`).

**Enum naming:** `Vectis{PascalCaseCategory}`.

**File structure:** See `references/swift-token-templates.md` (import,
MARK, generated comment, enum, color extensions).

### 4. Generate iOS `Theme.swift`

Regenerate `Theme.swift` to reference all generated enums (environment key +
`.vectisTheme()` modifier). No "Generated from" header.

### 5. Generate iOS `Package.swift` (if missing)

If `{output-dir}/Package.swift` does not exist, generate it per
`references/swift-token-templates.md`. If the file already exists, leave it
unchanged.

---

## Android generation

### 6. Generate Kotlin token files

Emit sources under:

`{android-output-dir}/src/main/kotlin/{android-package path}/`

Example package `com.vectis.design` → directory `com/vectis/design/`.

**Files (regenerate from YAML each run):**

| File | Contents |
|---|---|
| `VectisColorScheme.kt` | `vectisColor(hex)`, `vectisLightColorScheme()`, `vectisDarkColorScheme()` |
| `Typography.kt` | `object VectisTypography` + `fun vectisTypography(): Typography` |
| `Spacing.kt` | `object VectisSpacing` and `object VectisCornerRadius` (colocated, like Swift) |
| `Theme.kt` | `@Composable fun VectisTheme(...)` wrapping `MaterialTheme` with static token schemes |

Use **static** light/dark schemes from tokens (not `dynamicLightColorScheme`) so
Android matches iOS adaptive colors. Reserve dynamic color for app shells that
have **no** `tokens.yaml` (see android-writer).

Apply the generated-file comment to every file **except** `Theme.kt` (scaffold,
same as iOS `Theme.swift`).

**Imports:** `androidx.compose.material3`, `androidx.compose.ui.graphics.Color`,
`androidx.compose.ui.text.*`, `androidx.compose.ui.unit.*`, as required by
templates in `references/kotlin-token-templates.md`.

### 7. Generate Android `build.gradle.kts` (if missing)

If `{android-output-dir}/build.gradle.kts` does not exist, generate the minimal
Android library file from `references/kotlin-token-templates.md` (Compose + M3,
same `compileSdk` / `minSdk` / JVM target as the android-writer app template).

If the file already exists, **do not overwrite** — teams may pin dependencies or
ABI options (mirror: iOS `Package.swift` left unchanged when present).

### 8. Consumer wiring (documentation only)

The **android-writer** skill is responsible for `settings.gradle.kts` and
`app/build.gradle.kts` (`include(":vectis-design")` and
`implementation(project(":vectis-design"))`). The design-system-writer does not
edit consumer apps unless the task explicitly includes them; after generating
the library, remind that Android projects need the include path, typically:

```kotlin
include(":vectis-design")
project(":vectis-design").projectDir = file("../design-system/android")
```

Adjust the relative path when the Android directory is not one level below the
repo root (same idea as the iOS path to `design-system/ios`).

---

## Verification

### 9. Verify iOS build

Run `swift build` in `{output-dir}`. Note that `swift build` compiles for the
**host** platform (macOS) by default, so generated code must compile for macOS
even when the primary deployment target is iOS.

On failure: read errors, fix generated Swift, repeat until clean.

If `swift build` fails due to network errors (package resolution) or sandbox
restrictions, log the full error and mark iOS verification as **pending** rather
than retrying indefinitely.

### 10. Verify Android build

If the repo has **no Android project yet** (no `settings.gradle.kts` that
includes `:vectis-design`), skip this step and record that verification is
pending shell generation.

#### Gradle wrapper bootstrap

Before running `./gradlew`, check whether the wrapper is usable:

1. Verify `gradlew` exists and is executable **and** `gradle/wrapper/gradle-wrapper.jar` exists in the Android project directory.
2. If the wrapper is missing or incomplete, bootstrap it from a **minimal init
   project** to avoid triggering full AGP classpath resolution:
   ```bash
   tmp_dir=$(mktemp -d)
   cd "$tmp_dir" && gradle wrapper && cd -
   cp "$tmp_dir/gradlew" "$tmp_dir/gradlew.bat" "$ANDROID_SHELL_DIR/"
   cp -r "$tmp_dir/gradle" "$ANDROID_SHELL_DIR/"
   chmod +x "$ANDROID_SHELL_DIR/gradlew"
   rm -rf "$tmp_dir"
   ```
3. If `gradle` is not installed, report the prerequisite error:
   `"Gradle is required to bootstrap the wrapper. Install with: brew install gradle"`
   and mark verification as **pending**.

#### Build verification

From the **Android** project directory that includes `:vectis-design`:

```bash
./gradlew :vectis-design:compileDebugKotlin
```

On failure: read errors, fix generated Kotlin or the **missing** consumer
Gradle wiring, repeat until clean.

If `./gradlew` fails due to network errors, SSL/connection resets, or sandbox
restrictions, log the full error and mark Android verification as **pending**
rather than retrying indefinitely.

---

## Downstream impact (optional)

**iOS** — search for `import VectisDesign` and rebuild consumers.

**Android** — search for `import com.vectis.design` (or the chosen
`android-package`) and rebuild the app module.

---

## Removing stale files

**Swift:** If a token category was removed from `tokens.yaml`, delete the
corresponding generated Swift file under `Sources/VectisDesign/`. Do not delete
`Theme.swift` or `Package.swift`. Do not delete files without the "Generated
from" header.

**Kotlin:** Delete generated token files under `{android-output-dir}/src/.../`
that no longer correspond to YAML categories. Do not delete `build.gradle.kts`
if present. Do not delete `Theme.kt` when categories are removed — regenerate it
so `VectisTheme` still compiles (it may only depend on remaining APIs).

---

## Adding a new token category

1. Detect the value shape (color, font, or scalar).
2. Generate new Swift file + Kotlin file(s) with the appropriate pattern.
3. Add a property to iOS `VectisTheme` for the new category.
4. If the new category affects Material theming on Android, extend
   `vectisTypography()` / color scheme mapping or add a new Kotlin object.
5. Rebuild iOS and Android.

---

## Value shape reference

### Color shape

YAML:

```yaml
colors:
  primary:
    light: "#007AFF"
    dark: "#0A84FF"
```

Swift:

```swift
public static let primary = Color(light: "#007AFF", dark: "#0A84FF")
```

Kotlin: map `light` into `vectisLightColorScheme()`, `dark` into
`vectisDarkColorScheme()` using `vectisColor("#...")` per
`references/kotlin-token-templates.md`.

### Font shape

YAML:

```yaml
typography:
  largeTitle:
    size: 34
    weight: bold
```

Swift:

```swift
public static let largeTitle = Font.system(size: 34, weight: .bold)
```

Kotlin: `VectisTypography.largeTitle = TextStyle(fontSize = 34.sp, fontWeight = FontWeight.Bold, ...)`
and wire into `vectisTypography()`.

Weight mapping matches Swift (see swift-token-templates / kotlin-token-templates).

### Scalar shape

YAML:

```yaml
spacing:
  md: 16
```

Swift:

```swift
public static let md: CGFloat = 16
```

Kotlin:

```kotlin
val md = 16.dp
```

---

## Error handling

| Error | Resolution |
|---|---|
| `tokens.yaml` not found | Verify `tokens-file` path; default is `design-system/tokens.yaml` |
| Unknown value shape | Token values must be color (light/dark), font (size/weight), or scalar (number). Report the unexpected structure and skip the category on both platforms. |
| `swift build` fails | Read compiler errors, fix the generated Swift, rebuild. If failure is due to network/sandbox, mark verification as pending. |
| `compileDebugKotlin` fails | Read compiler errors; fix Kotlin, or add/fix `:vectis-design` in settings and BOM alignment. If failure is due to network/sandbox, mark verification as pending. |
| `./gradlew` not found or wrapper jar missing | Bootstrap the wrapper from a minimal init project (see step 10). Do not re-run the broken command. |
| Network / SSL / timeout errors | Log the full error. Do not retry indefinitely. Mark verification as pending and report to user. |
| Downstream shell breaks | A renamed or removed token was referenced by a shell. Report the affected file and token name. |

---

## Verification checklist

**Shared**

- [ ] Every YAML category has corresponding Swift and Kotlin outputs
- [ ] Every token has a corresponding definition on both platforms
- [ ] Token order matches YAML on both platforms

**iOS**

- [ ] `Theme.swift` references every generated enum
- [ ] `Package.swift` exists
- [ ] `swift build` passes
- [ ] Generated Swift files have the "Generated from" header where required
- [ ] No stale Swift token files for removed categories

**Android**

- [ ] `VectisColorScheme.kt`, `Typography.kt`, `Spacing.kt`, `Theme.kt` present and compile
- [ ] Library uses Compose Material 3 and BOM alignment per kotlin-token-templates
- [ ] `./gradlew :vectis-design:compileDebugKotlin` passes when the module is wired
- [ ] No stale Kotlin token artifacts for removed categories

**Consumers**

- [ ] Downstream iOS shells (if any) still build
- [ ] Downstream Android apps (if any) still build after `vectis-design` wiring
