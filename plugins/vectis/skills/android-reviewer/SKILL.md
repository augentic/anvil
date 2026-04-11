---
name: android-reviewer
description: Review generated Android shell (Kotlin/Jetpack Compose) code for structural issues, integration correctness, and quality problems. Use when reviewing a Crux app's Android shell after generation, or when the user mentions android-reviewer.
---

# Crux Android Shell Reviewer

Systematically review the generated Android shell (Kotlin/Jetpack Compose)
for structural issues, integration correctness, and general code quality
problems. Produces a severity-graded report with actionable findings and
suggested fixes.

This skill catches issues that the Kotlin compiler and linter miss: missing
screen composable / root branch correspondence, incomplete effect handlers,
hardcoded design tokens, missing accessibility descriptions, coroutine safety
violations, missing UniFFI library override, and incorrect generated-type
import patterns.

## Arguments

| Argument | Required | Description |
|---|---|---|
| `target-dir` | **Yes** | Path to the Crux app directory containing an `Android/` shell |
| `reference-dir` | No | Path to a known-good app for comparative review |
| `scope` | No | `full` (default) runs all four passes (structural, quality, universal, integration); `quick` runs structural + quality only, skipping universal and integration |

## Process

This skill uses an agent team with 3 specialist reviewers and 1 antagonist.
The lead coordinates the team, synthesizes findings, and produces the final
report. See [Agent Team Patterns](references/agent-teams.md) for shared
protocols (team roles, antagonist protocol, synthesis rules, file ownership,
and confidence scoring).

### 1. Gather context

Read the following files from `{target-dir}`:

- `shared/src/app.rs` -- the Crux core (source of truth for types)
- `shared/Cargo.toml` -- capability dependencies
- All `.kt` files under `Android/app/src/main/java/` -- the Android shell code
- `Android/app/build.gradle.kts` -- app module build configuration
- `Android/shared/build.gradle.kts` -- shared module build configuration
- `Android/gradle/libs.versions.toml` -- version catalog
- `Android/settings.gradle.kts` -- module includes
- `Android/gradle.properties` -- build properties
- `Android/app/src/main/AndroidManifest.xml` -- manifest configuration
- `Android/Makefile` -- build automation

If `reference-dir` is provided, also read the corresponding files from the
reference app.

Also read:
- `design-system/tokens.yaml` -- expected design tokens
- `design-system/spec.md` -- design system usage rules

### 2. Review-fix cycle (max 3 iterations)

Follow the [Reviewer Workflow](../../../references/reviewer-workflow.md) for
the review-fix cycle, synthesis, auto-fix, and loop control orchestration.
The platform-specific details below define the specialists, universal check
heuristics, and auto-fix list for this cycle.

#### 2a. Initialize team

The domain-specific specialist for Android is the **Integration Specialist**
(skipped on `quick` scope and subsequent iterations).

**Spawn Structural Specialist**:

```text
You are a Structural Reviewer for a Crux Android shell at $TARGET_DIR.

Read `references/android-review-checks.md`.

Apply checks AND-001 through AND-024 against the Kotlin source. These are
pattern-based checks that verify the shell correctly maps to the Crux core:

- Screen composable / ViewModel variant correspondence
- Effect handler completeness
- Event dispatch coverage
- Route/navigation completeness
- Design system token usage
- Root composable `when` exhaustiveness
- UniFFI library override presence
- Generated type import correctness
- CoreFFI error handling (try/catch with diagnostic logging)
- Render effect view preservation on failure
- Coroutine error handling patterns
- Timer job tracking and cancellation
- Build configuration correctness

For each finding, report: check ID (AND-NNN), file:line, code snippet,
severity (Critical or Warning), risk description, suggested fix, and
whether it is auto-fixable (mechanical).

Output your findings as a numbered list in markdown. Prefix each finding
ID with "AND-" (e.g., AND-001-1, AND-023-1).
```

If `iteration > 1`, append: "Scope your analysis to these files modified
in the previous iteration: [list of changed files]."

**Spawn Quality Specialist**:

```text
You are a Quality Reviewer for a Crux Android shell at $TARGET_DIR.

Read `references/kotlin-quality-checks.md`.

Apply checks KTL-001 through KTL-010 against all `.kt` files. These are
Kotlin/Jetpack Compose best practice checks:

- No `!!` non-null assertions in production code
- No debug output (`println`, `System.out`, `e.printStackTrace`)
- Coroutine safety (`CancellationException` rethrown, correct dispatchers)
- Compose state management (`mutableStateOf`, `StateFlow`, `collectAsState`)
- Preview coverage
- Design system imports
- Deprecated API usage
- Accessibility descriptions on interactive icons
- Event callback naming consistency

For each finding, report: check ID (KTL-NNN), file:line, code snippet,
severity (Warning or Info), risk description, suggested fix, and whether
it is auto-fixable.

Output your findings as a numbered list in markdown. Prefix each finding
ID with "KTL-" (e.g., KTL-001-1, KTL-006-1).
```

If `iteration > 1`, append: "Scope your analysis to these files modified
in the previous iteration: [list of changed files]."

**Spawn Integration Specialist** (first iteration only; skip if scope = quick):

```text
You are an Integration Reviewer for a Crux Android shell at $TARGET_DIR.

Cross-reference the Rust core types (`shared/src/app.rs`) against the
Kotlin implementation:

1. Type completeness -- every FFI-crossing type in `app.rs` must have a
   corresponding Kotlin type in the generated bindings.
2. Serialization correctness -- verify Bincode serialize/deserialize calls
   use the correct types. Check that `ULong`, `UInt`, `UShort`, and
   `List<UByte>` conversions are applied where needed.
3. Build configuration -- verify `build.gradle.kts` references the correct
   NDK version, Gradle version matches AGP requirements,
   `gradle.properties` pins Java 21, and `shared` module includes
   `rust-android-gradle` plugin.
4. Capability alignment -- every Effect variant in `app.rs` must have a
   handler in `Core.kt` and the corresponding capability client class
   (if needed).
5. Module structure -- verify `app` and `shared` module namespaces differ,
   `settings.gradle.kts` includes both modules, and `shared` module's
   `sourceSets` includes the generated types directory.
6. Manifest correctness -- verify `AndroidManifest.xml` references the
   Application class (if Koin), the correct theme, and network security
   config (if HTTP/SSE effects).

For each finding, report: a finding ID (INT-NNN), file:line, code snippet,
severity (Critical or Warning), risk description, suggested fix, and
whether it is auto-fixable.

Output your findings as a numbered list in markdown. Prefix each finding
ID with "INT-" (e.g., INT-001, INT-002).
```

#### 2b-c. Specialist analysis and universal checks

Specialists run concurrently per the shared workflow. Universal checks
use Kotlin/Android-specific heuristics. Skip table:

| Universal check | Already covered by | Action |
|---|---|---|
| UNI-003 Serialization failures | AND-013, AND-014, AND-020 | Skip |
| UNI-006 Race conditions | KTL-003, AND-015, AND-016 | Skip |
| UNI-010 Panics/crashes | KTL-001 | Skip |

Apply the remaining checks with these Kotlin/Android-specific heuristics:

- **UNI-001** (uninitialised values): Look for `var` properties initialised
  to `null` or placeholder values that are accessed before a coroutine load
  completes. Check for `MutableStateFlow` initialised with default values
  that represent an invalid domain state.
- **UNI-002** (unvalidated input): Look for shell-side `TextField` values
  dispatched to the core via `onEvent(Event.Something(text))` without local
  trim or empty check. While the core should also validate, the shell should
  prevent obviously invalid dispatches.
- **UNI-004** (logic bugs): Reason about the `processRequest` `when` for
  missing branches, incorrect effect resolution sequences, and navigation
  handlers that produce unreachable states.
- **UNI-005** (unbounded growth): Look for `scope.launch` blocks that create
  coroutines without cancellation tracking, growing lists of SSE observations
  without cleanup, and `MutableStateFlow` subscribers that are never
  collected. Check for `Job` references stored without cancellation.
- **UNI-007** (chatty calls): Look for Ktor HTTP calls that re-fetch data
  the core already has from SSE or other real-time channels. Check for
  effect handlers that fire identical resolve calls on repeated recompositions.
- **UNI-008** (instrumentation balance): Look for error paths with no
  `Log.e` or `Log.w` call. Flag per-event logging inside hot loops (e.g.,
  logging every SSE chunk body).
- **UNI-009** (handle-then-throw): Look for `try/catch` blocks that
  partially update `_viewModel.value` or other `MutableStateFlow` values
  before rethrowing, leaving the UI in an inconsistent state.
- **UNI-011** (timeout/retry): Look for Ktor `HttpClient` instances without
  `HttpTimeout` installed. Check whether SSE reconnection logic exists for
  transient network failures.
- **UNI-012** (persisted state compat): Look for `SharedPreferences` model
  changes (new keys, changed serialization format) that would break
  deserialization of existing stored data.
- **UNI-013** (dead code): Look for `when` branches that can never match,
  unreachable code after `return` / `break`, unused private functions or
  properties, and composables with no call site.
- **UNI-014** (hardcoded config): Look for hardcoded timeout intervals,
  literal URL strings, and magic number page sizes or retry counts.
- **UNI-015** (stale captures): Look for `scope.launch` blocks capturing
  `this` or local state that may mutate before the coroutine completes.
  Check for lambda captures in `LazyColumn` `items` blocks that reference
  loop-scoped variables.
- **UNI-016** (error message quality): Look for `Log.e` messages with no
  context about which item or operation failed, and catch blocks that log
  the exception type but not the message.
- **UNI-017** (type safety): Look for `String` properties on view model
  types or event types that hold values from a known closed set (should be
  Kotlin enums or sealed interfaces).
- **UNI-018** (hardcoded secrets): Look for API keys, tokens, passwords,
  or connection strings embedded as string literals in Kotlin source files.
  Check for secrets in `local.properties` committed to git, hardcoded
  `Authorization` headers, and credentials stored in plain-text
  `SharedPreferences` rather than `EncryptedSharedPreferences` or the
  Android Keystore.
- **UNI-019** (injection vulnerabilities): Look for user input interpolated
  into `WebView` HTML content without escaping, URL path segments built via
  string concatenation, and `Runtime.exec` invocations with user-controlled
  arguments.
- **UNI-020** (unsafe deserialization): Look for bincode or JSON
  deserialization of untrusted external payloads directly into model types
  that carry privilege state. Check for missing payload size limits on data
  fetched from external sources.
- **UNI-021** (missing auth checks): Check that effect handlers attaching
  authentication credentials (Bearer tokens, API keys) to outbound requests
  source them from secure storage (Android Keystore /
  `EncryptedSharedPreferences`), not from hardcoded values or unprotected
  `SharedPreferences`. Flag API calls to protected endpoints dispatched
  without any auth header.

Prefix findings from this step with `UNI-` (e.g., UNI-1, UNI-2). Use the
severity defined in the universal checklist for each check.

Tag findings that have a **Spec-change indicator** (UNI-002, UNI-004,
UNI-007, UNI-008, UNI-011, UNI-012, UNI-014, UNI-021) for inclusion in
the adversarial review and spec-change output in step 3.

#### 2d. Adversarial challenge

After the specialist reports and universal checks are complete, the lead
sends all combined findings (AND-, KTL-, INT-, and UNI- prefixed) to the
antagonist.

**Spawn Antagonist**:

```text
You are the Antagonist Reviewer for a Crux Android shell at $TARGET_DIR.

You receive findings from specialist reviewers (Structural, Quality,
Integration) and from the lead's universal checks. Your job is to
challenge every finding and find what they missed.

For EACH finding (AND-, KTL-, INT-, and UNI- prefixed):
1. Validate evidence: Is there a real file:line reference and code snippet?
2. Challenge severity: Is Critical really critical? Is Info actually higher?
3. Check for false positives: Could this be a non-issue or acceptable
   Android/Compose pattern?
4. Assess auto-fix safety: Could the suggested fix introduce regressions?

Then perform a COUNTER-SCAN of all `.kt` files under
`Android/app/src/main/java/` looking for issues ALL specialists missed.
Common Android/Compose blind spots:
- Coroutine leaks from `scope.launch` without Job tracking or cancellation
- Missing `CancellationException` rethrow in catch blocks (breaks
  structured concurrency)
- Theme/resource mismatches between `themes.xml` and Compose theme wrapper
- Missing `network_security_config.xml` for apps with HTTP effects
- `@Preview` composables with stale sample data after ViewModel changes
- Timer `Job` references not cleaned up in `onCleared()`
- Missing `SupervisorJob` in CoroutineScope (child failure crashes parent)
- Hardcoded design tokens not matching `tokens.yaml` values

Output format:
## Confirmed: [ID] -- evidence solid, severity accurate
## Downgraded: [ID] ORIG_SEVERITY -> NEW_SEVERITY -- rationale
## Upgraded: [ID] ORIG_SEVERITY -> NEW_SEVERITY -- rationale
## Disputed: [ID] -- rationale (must cite evidence for dispute)
## New Findings: NEW-1, NEW-2, etc. with full finding details

You MUST provide evidence for every challenge. Opinion alone is insufficient.
You CANNOT remove findings entirely -- the minimum action is to downgrade.
Severity downgrades move at most one level (Critical to Warning, not to Info).
```

Follow the shared [Reviewer Workflow](../../../references/reviewer-workflow.md)
for adversarial challenge, synthesis, and confidence scoring.

#### 2e-f. Synthesis and iteration report

Follow the shared workflow synthesis steps. Report template:

````
## Android Shell Review Report: {app-name} (iteration {N})

**Review Team**: 3 specialists + 1 antagonist
**Confidence Level**: [HIGH | MEDIUM | LOW]

### Summary
- Critical: N findings
- Warning: N findings
- Info: N findings

### Critical Findings

#### [AND-001-1] Missing screen composable for ViewModel variant
- **File**: Android/app/src/main/java/com/vectis/{appname}/ui/screens/
- **Reviewer**: Structural Specialist
- **Antagonist**: Confirmed
- **Issue**: ViewModel variant `Settings(SettingsView)` has no corresponding
  screen composable file.
- **Fix**: Create `ui/screens/SettingsScreen.kt` and add the branch to the
  root composable.

### Warning Findings
...

### Info Findings
...

### Adversarial Review

**Antagonist Activity Summary**:

| Action       | Count   |
| ------------ | ------- |
| Confirmed    | [count] |
| Downgraded   | [count] |
| Upgraded     | [count] |
| Disputed     | [count] |
| New Findings | [count] |

**Acceptance Rate**: [confirmed / total specialist findings]%

#### Downgraded Findings
- [ID] ORIG -> NEW: rationale

#### Upgraded Findings
- [ID] ORIG -> NEW: rationale

#### Disputed Findings
- [ID] Reported as SEVERITY: "description"
  Dispute: rationale
  Lead Decision: [Included | Excluded]

#### New Findings (Missed by Specialists)
- [NEW-1] SEVERITY: description (file:line)
  Evidence: details
````

Classify each finding as **mechanical** (auto-fixable) or **design-level**.

#### 2g. Auto-fix mechanical issues

Apply mechanical auto-fixes per the shared workflow. Platform-specific
fixes for Android shell:

- Adding missing accessibility `contentDescription` values
- Adding missing design system imports
- Replacing hardcoded colors with design system tokens
- Replacing hardcoded spacing with design system tokens
- Adding missing `@Preview` composables
- Adding missing `@OptIn(ExperimentalUnsignedTypes::class)` annotations
- Adding missing `import com.example.app.*` statements
- Adding `CancellationException` rethrow to catch blocks

Do NOT auto-fix structural issues (missing screen composables, missing effect
handlers) without confirmation.

After fixes, run Kotlin formatting on modified files if configured.

#### 2h. Loop control

Follow the shared workflow loop control. Exit when no fixes are applied or
`max_iterations` is reached.

### 3. Express accumulated design-level findings as a Specify change

Follow the [Reviewer Workflow](../../../references/reviewer-workflow.md)
design-level findings procedure. Use platform suffix `android` in the change
name template (`review-{app-name}-android-{YYYY-MM-DDTHH-MM}`).

Android-specific guidance for artifacts:

- **specs/**: Group by logical area (e.g., `android-shell-effects`,
  `android-shell-navigation`).
- **tasks.md**: Order by dependency -- spec updates, missing screen
  composables, missing effect handlers, navigation fixes, design system
  corrections, verification. Include a final task that re-runs
  android-reviewer to confirm resolution.

## Severity Definitions

| Severity | Meaning | Action |
|---|---|---|
| **Critical** | Missing screen composables, missing effect handlers, broken build, data not rendered, crash on launch | Must fix before merge |
| **Warning** | Hardcoded tokens, missing previews, accessibility gaps, style inconsistencies, coroutine best practices | Should fix; acceptable to defer |
| **Info** | Minor improvements, alternative patterns | Fix if convenient |

## Verification Checklist

Team Execution and Report Quality checks are in the shared
[Reviewer Workflow](../../../references/reviewer-workflow.md#common-verification-checklist-items).

### Scan Coverage

- [ ] Structural Specialist: AND-001 through AND-024 checked
- [ ] Quality Specialist: KTL-001 through KTL-010 checked
- [ ] Integration Specialist: type completeness, serialization, build config,
  capability alignment, module structure, manifest checked (first iteration,
  full scope)
- [ ] Universal Checks: UNI-001 through UNI-021 applied with Kotlin/Android-specific
  heuristics (skipped where covered by AND/KTL)
- [ ] Antagonist: counter-scan completed for Android-specific blind spots

## Integration with Specify Workflow

This skill is invoked as part of the Vectis build phase, after android-writer
generation and build verification:

```
define -> build (android-writer) -> verify build -> review-fix cycle (this skill) -> generate change for design issues -> merge
```

The skill can also be invoked standalone:

> Use the android-reviewer skill to review `<target-dir>`

> Review the Android shell at `<target-dir>` against `<reference-dir>` as a reference
