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
| `scope` | No | `full` (default) runs all four passes (structural, quality, universal, integration); `quick` runs structural + quality only, skipping universal and integration |

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

**First iteration (`scope = full`)**: Run all four passes -- structural,
quality, universal, and integration. This is the comprehensive initial
review.

**First iteration (`scope = quick`)**: Run only the **structural pass** and
**quality pass**. Skip the universal pass and integration pass entirely.

**Subsequent iterations (either scope)**: Run only the passes that were
active in the first iteration, minus the integration pass, scoped to files
modified by the previous iteration's fixes. For `full` this means
structural + quality + universal; for `quick` this means structural +
quality only. Skip the integration pass on all subsequent iterations --
mechanical fixes do not alter FFI type mappings or build configuration.

#### 2b. Structural pass

Read `references/ios-review-checks.md` in this skill's directory.

Apply checks IOS-001 through IOS-016 against the Swift source. These are
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

#### 2d. Universal checks pass (skip if scope = quick)

Read `../../references/universal-review-checks.md` (shared across all Vectis
reviewer skills).

Apply checks UNI-001 through UNI-017 with Swift-specific detection. Several
universal checks overlap with platform-specific checks already applied in
earlier passes. Skip those and focus on the gaps:

| Universal check | Already covered by | Action |
|---|---|---|
| UNI-003 Serialization failures | IOS-013, IOS-015, IOS-016 | Skip |
| UNI-006 Race conditions | SWF-003, IOS-014 | Skip |
| UNI-010 Panics/crashes | SWF-001 | Skip |

Apply the remaining checks with these Swift-specific heuristics:

- **UNI-001** (uninitialised values): Look for `var` properties initialised
  to `nil` or placeholder values that are accessed before an async load
  completes. Check for `@Published` properties with default values that
  represent an invalid domain state.
- **UNI-002** (unvalidated input): Look for shell-side text inputs (e.g.,
  `TextField` bindings) dispatched to the core via `core.update()` without
  local trim or empty check. While the core should also validate, the shell
  should prevent obviously invalid dispatches.
- **UNI-004** (logic bugs): Reason about the `processEffect` switch for
  missing cases, incorrect effect resolution sequences, and navigation
  handlers that produce unreachable states.
- **UNI-005** (unbounded growth): Look for strong reference cycles (`self`
  captured in closures without `[weak self]` where the closure outlives the
  expected scope), never-cancelled `Task` instances, and growing arrays of
  SSE observations or event subscriptions without cleanup.
- **UNI-007** (chatty calls): Look for `URLSession` calls that re-fetch
  data the core already has from SSE or other real-time channels. Check for
  effect handlers that fire identical resolve calls on repeated renders.
- **UNI-008** (instrumentation balance): Look for error paths with no
  `assertionFailure` or `os.Logger` call. Flag per-event logging inside
  hot loops (e.g., logging every SSE message body).
- **UNI-009** (handle-then-throw): Look for `do/catch` blocks that partially
  update `self.view` or other `@Published` properties before re-throwing,
  leaving the UI in an inconsistent state.
- **UNI-011** (timeout/retry): Look for `URLSession` requests with no
  `timeoutInterval` configured. Check whether effect handlers have a
  retry or fallback path for transient network failures.
- **UNI-012** (persisted state compat): Look for `Codable` model changes
  (new properties without default values) that would break decoding of
  existing `UserDefaults` or file-persisted data.
- **UNI-013** (dead code): Look for switch cases that can never match,
  unreachable code after `return` / `break`, and unused private functions
  or properties.
- **UNI-014** (hardcoded config): Look for hardcoded timeout intervals,
  literal URL strings, and magic number page sizes or retry counts.
- **UNI-015** (stale captures): Look for `Task` blocks capturing `self` or
  local state that may mutate before the async work completes. Check for
  closures that capture loop variables.
- **UNI-016** (error message quality): Look for `assertionFailure` messages
  with no context about which item or operation failed, and catch blocks
  that log the error type but not the message.
- **UNI-017** (type safety): Look for `String` properties on view models or
  event types that hold values from a known closed set (should be Swift
  enums).

Record findings with the severity defined in the universal checklist. Tag
findings that have a **Spec-change indicator** (UNI-002, UNI-004, UNI-007,
UNI-008, UNI-011, UNI-012, UNI-014) for inclusion in the spec-change
output in step 3.

#### 2e. Integration pass (first iteration only; skip if scope = quick)

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

#### 2f. Produce iteration report

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

#### 2g. Auto-fix mechanical issues

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

#### 2h. Loop control

1. If **no mechanical fixes** were applied, exit the cycle.
2. If `iteration >= max_iterations`, exit the cycle.
3. Otherwise, increment `iteration` and return to step 2a.

When the cycle exits, output a summary across all iterations:

```
### Review Cycle Summary
- Iteration 1: Fixed N mechanical issues (IOS-005 x2, SWF-006, UNI-016).
  M design-level findings deferred.
- Iteration 2: Fixed K regressions from iteration 1 fixes.
  No new design-level findings.
- Total: N+K mechanical fixes applied. M design-level findings accumulated.
```

### 3. Express accumulated design-level findings as a Specify change

After the review-fix cycle completes, check whether any **design-level
findings** were accumulated -- findings that require architectural decisions,
missing screen views, missing effect handlers, or issues that indicate the
spec is incomplete (typically IOS-001, IOS-003, IOS-010, and universal
checks tagged with a **Spec-change indicator**).
If none were accumulated across any iteration, skip this step.

#### Classify findings: code-fix vs spec-change

Before creating the Specify change, classify each design-level finding:

- **Code-fix**: The spec is clear and the code simply does not implement it
  correctly. The fix is a code change; no spec update is needed. These
  become tasks in `tasks.md`.
- **Spec-change**: The spec is silent, ambiguous, or mandates behavior that
  the review identified as problematic. The fix requires updating the spec
  first, then implementing. These become requirements in `specs/` and
  decisions in `design.md`.

Universal checks with a Spec-change indicator (UNI-002, UNI-004, UNI-007,
UNI-008, UNI-011, UNI-012, UNI-014) commonly surface as spec-change
findings. Consult `../../references/universal-review-checks.md` for the
indicator description on each check.

If design-level findings exist, delegate to `/spec:define` to create a
single Specify change that tracks all of them:

1. **Derive a change name** from the app name and append the current
   date-time for traceability:

   ```
   review-{app-name}-ios-{YYYY-MM-DDTHH-MM}
   ```

   Example: `review-my-crux-app-ios-2026-03-25T10-30`

   Use the shell to get the current timestamp:
   ```bash
   date -u +"%Y-%m-%dT%H-%M"
   ```

2. **Delegate to `/spec:define`** with the derived change name and a
   description synthesized from the accumulated design-level findings.
   Provide the following guidance for artifact generation:

3. **Content guidelines for each artifact**:

   - **proposal.md**: The "Why" section summarizes the accumulated review
     findings by severity and risk, distinguishing spec-change findings
     (requirements gaps) from code-fix findings (implementation bugs).
     The "What Changes" section lists each design-level finding as a
     bullet, prefixed with `[spec]` or `[code]` to indicate its
     classification. Note which mechanical fixes were already applied
     across all iterations and how many review cycles ran. The "Impact"
     section identifies affected files, core contract changes, and
     migration concerns.

   - **design.md**: Each design-level finding becomes a Decision section
     with rationale and alternatives considered. Group related findings
     (e.g., all effect-handler-related changes under one decision).
     Reference the specific check IDs (IOS-xxx, SWF-xxx, UNI-xxx) that
     motivated each decision. For spec-change findings, explain why the
     current spec is insufficient and what the proposed requirement
     should be.

   - **specs/**: Create one spec file per logical area (e.g.,
     `ios-shell-effects`, `ios-shell-navigation`). Each requirement maps
     to a review finding. Spec-change findings become new requirements
     with explicit acceptance criteria. Code-fix findings become scenarios
     under existing requirements. Use WHEN/THEN format.

   - **tasks.md**: Order tasks by dependency -- spec updates first (so
     requirements are clear before implementation), then missing screen
     views, then missing effect handlers, then navigation fixes, then
     design system corrections, then verification. Each task references
     the finding ID it addresses. Include a final verification section
     that re-runs the ios-reviewer skill to confirm all Critical findings
     are resolved.

4. **Show final status** using `/spec:status` and summarize: change name,
   location, artifacts created, and prompt the user with "Run `/spec:build`
   or ask me to implement to start working on the tasks."

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
