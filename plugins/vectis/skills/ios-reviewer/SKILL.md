---
name: ios-reviewer
description: Review generated iOS shell (SwiftUI) code for structural issues, integration correctness, and quality problems. Use when reviewing a Crux app's iOS shell after generation, or when the user mentions ios-reviewer.
---

# Crux iOS Shell Reviewer

Systematically review the generated iOS shell (SwiftUI) for structural issues,
integration correctness, and general code quality problems. Produces a
severity-graded report with actionable findings and suggested fixes.

This skill catches issues that the Swift compiler and swiftformat miss:
missing ViewModel/screen view correspondence, incomplete effect handlers,
hardcoded design tokens, missing accessibility labels, and concurrency
violations.

## Arguments

| Argument | Required | Description |
|---|---|---|
| `target-dir` | **Yes** | Path to the Crux app directory containing an `iOS/` shell |
| `reference-dir` | No | Path to a known-good app for comparative review |
| `scope` | No | `full` (default) runs all three passes; `quick` runs structural + quality only |

## Process

### 1. Gather context

Read the following files from `{target-dir}`:

- `shared/src/app.rs` -- the Crux core (source of truth for types)
- `shared/Cargo.toml` -- capability dependencies
- All `.swift` files under `iOS/` -- the iOS shell code
- `iOS/project.yml` -- build configuration
- `iOS/Makefile` -- build automation

If `reference-dir` is provided, also read the corresponding files from the
reference app.

Also read:
- `design-system/tokens.yaml` -- expected design tokens
- `design-system/spec.md` -- design system usage rules

### 2. Review-fix cycle (max 3 iterations)

Before starting, initialize:

- `iteration = 1`, `max_iterations = 3`
- An empty list of **accumulated design-level findings**

The cycle repeats: run review passes, report findings, auto-fix mechanical
issues, then re-review. Exit when no mechanical fixes are applied or
`max_iterations` is reached.

#### 2a. Select passes for this iteration

**First iteration**: Run all three passes -- structural, quality, and
integration.

**Subsequent iterations**: Run only structural and quality passes, scoped
to files modified by the previous iteration's fixes.

#### 2b. Structural pass

Read `references/ios-review-checks.md` in this skill's directory.

Apply checks IOS-001 through IOS-010 against the Swift source. These are
pattern-based checks that verify the shell correctly maps to the Crux core:

- ViewModel/screen view correspondence
- Effect handler completeness
- Event dispatch coverage
- Route/navigation completeness
- Design system token usage
- ContentView switch exhaustiveness

For each violation found, record: check ID, file, line range, description,
severity (Critical or Warning), and suggested fix.

#### 2c. Quality pass

Read `references/swift-quality-checks.md` in this skill's directory.

Apply checks SWF-001 through SWF-010 against all `.swift` files. These are
Swift/SwiftUI best practice checks:

- Concurrency correctness (`@MainActor`, `Sendable`)
- No force unwraps in production code
- Accessibility labels on interactive elements
- SwiftUI state management (`@Published`, `@ObservedObject`, `@State`)
- Preview coverage
- swiftformat compliance

Record findings with severity Warning or Info.

#### 2d. Integration pass (first iteration only; skip if scope = quick)

Cross-reference the Rust core types against the Swift implementation:

1. **Type completeness** -- every FFI-crossing type in `app.rs` must have a
   corresponding Swift type in the generated bindings.
2. **Serialization correctness** -- verify Bincode serialize/deserialize calls
   use the correct types.
3. **Build configuration** -- verify `project.yml` references the correct
   shared library path, correct deployment target, correct Swift version.
4. **Capability alignment** -- every Effect variant in `app.rs` must have a
   handler in `Core.swift`.

Record findings with severity Critical or Warning.

#### 2e. Produce iteration report

Output findings for this iteration:

```
## iOS Shell Review Report: {app-name} (iteration {N})

### Summary
- Critical: N findings
- Warning: N findings
- Info: N findings

### Critical Findings

#### [IOS-001] Missing screen view for ViewModel variant
- **File**: iOS/{AppName}/ContentView.swift
- **Issue**: ViewModel variant `Settings(SettingsView)` has no corresponding
  screen view file.
- **Fix**: Create `Views/SettingsScreen.swift` and add the case to ContentView.

### Warning Findings
...

### Info Findings
...
```

Classify each finding as **mechanical** (auto-fixable) or **design-level**.

#### 2f. Auto-fix mechanical issues

Apply fixes for findings that are mechanical:

- Adding missing accessibility labels
- Adding missing `import VectisDesign`
- Replacing hardcoded colors with `VectisColors` tokens
- Replacing hardcoded spacing with `VectisSpacing` tokens
- Adding missing `#Preview` blocks

Do NOT auto-fix structural issues (missing screen views, missing effect
handlers) without confirmation -- these may require design decisions about
layout and interaction.

After fixes, run `swiftformat` on modified files.

#### 2g. Loop control

1. If **no mechanical fixes** were applied, exit the cycle.
2. If `iteration >= max_iterations`, exit the cycle.
3. Otherwise, increment `iteration` and return to step 2a.

### 3. Express accumulated design-level findings as a Specify change

Same pattern as the core-reviewer skill. If design-level findings exist,
create a single Specify change:

1. Derive a change name: `review-{app-name}-ios-{YYYY-MM-DDTHH-MM}`
2. Delegate to `/spec:define` to create the change and generate artifacts
   (proposal, design, specs, tasks) populated with the accumulated review
   findings.

## Severity Definitions

| Severity | Meaning | Action |
|---|---|---|
| **Critical** | Missing screen views, missing effect handlers, broken build, data not rendered | Must fix before merge |
| **Warning** | Hardcoded tokens, missing previews, accessibility gaps, style inconsistencies | Should fix; acceptable to defer |
| **Info** | Minor improvements, alternative patterns | Fix if convenient |

## Integration with Specify Workflow

This skill is invoked as part of the Vectis build phase, after ios-writer
generation and build verification:

```
define -> build (ios-writer) -> verify build -> review-fix cycle (this skill) -> generate change for design issues -> merge
```

The skill can also be invoked standalone:

> Use the ios-reviewer skill to review `<target-dir>`

> Review the iOS shell at `<target-dir>` against `<reference-dir>` as a reference
