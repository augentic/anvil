---
name: design-system-writer
description: Generate or update the platform-specific design system implementation from tokens.yaml. Use when implementing design-system module tasks from a Specify change, or when the user mentions design-system-writer.
---

# Design System Writer

Generate (or regenerate) the platform-specific design system code from
`tokens.yaml`. The Swift files under `design-system/ios/` are fully derived
from the YAML source and carry a "do not edit manually" comment. Every
invocation reads the YAML and overwrites the Swift files.

Unlike core-writer or ios-writer, there is no create vs update distinction.
The mapping from YAML to Swift is mechanical and deterministic -- the skill
always regenerates from scratch.

## Arguments

| Argument | Required | Description |
|---|---|---|
| `tokens-file` | No | Path to the tokens YAML file. Defaults to `design-system/tokens.yaml` |
| `output-dir` | No | Path to the iOS Swift Package directory. Defaults to `design-system/ios` |
| `change-dir` | No | Path to a Specify change directory. When provided, the skill reads design-system specs from `{change-dir}/specs/{module-name}/spec.md` for context on what token changes are expected. |

## Process

### 1. Read and inventory tokens

Read the tokens YAML file. Build an inventory of every top-level key (excluding
`version`). Each key is a **token category** that maps to one Swift enum.

Currently defined categories:

| YAML key | Swift enum | File |
|---|---|---|
| `colors` | `VectisColors` | `Colors.swift` |
| `typography` | `VectisTypography` | `Typography.swift` |
| `spacing` | `VectisSpacing` | `Spacing.swift` (first section) |
| `cornerRadius` | `VectisCornerRadius` | `Spacing.swift` (second section) |

If a new top-level key appears in the YAML (e.g., `elevation`, `opacity`,
`animation`), the skill generates a new Swift file for it using the
appropriate value-shape mapping (see step 2).

### 2. Classify value shapes

Each token category has a **value shape** that determines the Swift code
pattern. Detect the shape from the first entry in the category:

| Shape | Detection | Swift pattern |
|---|---|---|
| **Color** | Values have `light` and `dark` keys | `Color(light:dark:)` static |
| **Font** | Values have `size` and `weight` keys | `Font.system(size:weight:)` static |
| **Scalar** | Values are plain numbers | `CGFloat` static |

Read `references/swift-token-templates.md` for the exact Swift code templates
for each shape, including helper extensions and the weight mapping table.

### 3. Generate token files

For each token category, generate or overwrite the corresponding Swift file
under `{output-dir}/Sources/VectisDesign/`. This step covers token enum files
only -- `Theme.swift` is handled separately in step 4.

**File naming rules:**

- Convert the YAML key to PascalCase for the filename (e.g., `colors` ->
  `Colors.swift`, `typography` -> `Typography.swift`).
- Exception: `spacing` and `cornerRadius` are colocated in `Spacing.swift`
  as two separate enums (`VectisSpacing` and `VectisCornerRadius`). This
  matches the existing layout and avoids a file with only five entries.

**Enum naming:** `Vectis{PascalCaseCategory}` (e.g., `VectisColors`,
`VectisTypography`, `VectisSpacing`, `VectisCornerRadius`).

**File structure:**

1. `import SwiftUI`
2. MARK comment with the category name
3. Generated-file comment: `// Generated from design-system/tokens.yaml — do not edit manually.`
4. `public enum Vectis{Category} { ... }` with one `public static let` per token
5. For color files only: append the `Color(light:dark:)` initializer extension
   and the `UIColor(hex:)` convenience initializer

Token entries within an enum preserve the order from the YAML file.

For colocated files (e.g., `Spacing.swift` containing both `VectisSpacing` and
`VectisCornerRadius`), the `// Generated from` comment appears only after the
first MARK. The second enum section has only its own `// MARK:` comment.

For color tokens, group entries with a blank line between semantic groups
(primary group, secondary group, surface group, error group, utility group).
Use the explicit grouping table in `references/swift-token-templates.md` for
current tokens. For novel tokens, group by shared prefix root
(e.g., `primary`, `primaryContainer`, `onPrimary`, `onPrimaryContainer`).

### 4. Generate Theme.swift

`Theme.swift` is structural scaffolding, not a token file. It does NOT use the
step 3 file structure (no MARK header, no "Generated from" comment). Instead
it uses the dedicated Theme template from the reference documentation.

Regenerate `Theme.swift` to reference all generated enums. The file contains:

- The `VectisTheme` struct with one `public let` property per token category,
  typed as `Vectis{Category}.Type` and defaulting to `Vectis{Category}.self`
- The `VectisThemeKey` environment key
- The `EnvironmentValues` extension
- The `.vectisTheme()` view modifier

Read `references/swift-token-templates.md` for the complete `Theme.swift`
template.

### 5. Generate Package.swift (if missing)

If `{output-dir}/Package.swift` does not exist, generate it:

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

If the file already exists, leave it unchanged. SPM auto-discovers source
files so no update is needed when files are added or removed.

### 6. Verify build

Run `swift build` in the `{output-dir}` directory. If the build fails:

1. Read the error output
2. Fix the generated file that caused the failure
3. Re-run `swift build`
4. Repeat until clean

### 7. Check downstream impact (optional)

Search the workspace for downstream consumers that `import VectisDesign`:

```
grep -r "import VectisDesign" . --include="*.swift" -l
```

If any are found, run `swift build` (or the project's build command) in each
consumer's directory to verify the token changes did not break downstream
consumers. Report any failures.

### 8. Removing stale files

If a token category was removed from `tokens.yaml`, the corresponding Swift
file should be deleted from `Sources/VectisDesign/`. Compare the set of
expected files (derived from YAML keys) against the actual files in the
directory. Delete any generated file that no longer has a corresponding
YAML category.

Do NOT delete `Theme.swift` or `Package.swift` -- these are always present.

Do NOT delete files that do not carry the "Generated from" header comment --
these may be hand-written additions.

## Adding a New Token Category

When a user adds a new top-level key to `tokens.yaml`:

1. Detect the value shape (color, font, or scalar)
2. Generate a new Swift file with the appropriate enum
3. Add a property to `VectisTheme` for the new category
4. Rebuild and verify

No changes to `Package.swift` are needed (SPM auto-discovers sources).

## Value Shape Reference

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

Weight mapping: `bold` -> `.bold`, `semibold` -> `.semibold`, `regular` ->
`.regular`, `medium` -> `.medium`, `light` -> `.light`, `thin` -> `.thin`,
`ultraLight` -> `.ultraLight`, `heavy` -> `.heavy`, `black` -> `.black`.

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

## Error Handling

| Error | Resolution |
|---|---|
| `tokens.yaml` not found | Verify `tokens-file` path; default is `design-system/tokens.yaml` |
| Unknown value shape | Token values must be color (light/dark), font (size/weight), or scalar (number). Report the unexpected structure and skip the category. |
| `swift build` fails | Read compiler errors, fix the generated file, rebuild |
| Downstream shell breaks | A renamed or removed token was referenced by an iOS shell. Report the affected file and token name. |

## Verification Checklist

- [ ] Every YAML category has a corresponding Swift enum
- [ ] Every token in the YAML has a corresponding `public static let` in its enum
- [ ] Token order in Swift matches order in YAML
- [ ] `Theme.swift` references every generated enum
- [ ] `Package.swift` exists
- [ ] `swift build` passes
- [ ] All token files have the "Generated from" header comment (Theme.swift is exempt)
- [ ] No stale Swift files remain for removed categories
- [ ] Downstream iOS shells (if any) still build
