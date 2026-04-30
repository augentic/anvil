---
name: vectis-ios-reviewer
description: Review generated iOS shell (SwiftUI) code for structural issues, integration correctness, and quality problems. Use when reviewing a Crux app's iOS shell after generation, or when the user mentions ios-reviewer.
argument-hint: "<target-dir>"
---

# Crux iOS Shell Reviewer

Systematically review the generated iOS shell (SwiftUI) for structural issues, integration correctness, and general code quality problems. Produces a severity-graded report with actionable findings and suggested fixes.

This skill catches issues that the Swift compiler and swiftformat miss: missing ViewModel/screen view correspondence, incomplete effect handlers, hardcoded design tokens, missing accessibility labels, and concurrency violations.

## Arguments

| Argument | Required | Description |
|---|---|---|
| `target-dir` | **Yes** | Path to the Crux app directory containing an `iOS/` shell |
| `reference-dir` | No | Path to a known-good app for comparative review |
| `scope` | No | `full` (default) runs all four passes (structural, quality, universal, integration); `quick` runs structural + quality only, skipping universal and integration |
| `orchestrated` | No | When `true`, the reviewer skips step 3 (`/spec:define`) and instead returns classified design-level findings in its output for the orchestrator to consolidate across platforms. Set by the build brief when running iOS and Android reviews in parallel. Defaults to `false`. |

## Process

This skill uses an agent team with 3 specialist reviewers and 1 antagonist. The lead coordinates the team, synthesizes findings, and produces the final report. See [Agent Team Patterns](references/agent-teams.md) for shared protocols (team roles, antagonist protocol, synthesis rules, file ownership, and confidence scoring).

### 1. Gather context

Read the following files from `{target-dir}`:

- `shared/src/app.rs` -- the Crux core (source of truth for types)
- `shared/Cargo.toml` -- capability dependencies
- All `.swift` files under `iOS/` -- the iOS shell code
- `iOS/project.yml` -- build configuration
- `iOS/Makefile` -- build automation

If `reference-dir` is provided, also read the corresponding files from the reference app.

Also read:
- `design-system/tokens.yaml` -- expected design tokens
- `design-system/spec.md` -- design system usage rules

### 2. Review-fix cycle (max 3 iterations)

Before starting, initialize:

- `iteration = 1`, `max_iterations = 3`
- An empty list of **accumulated design-level findings**

The cycle repeats: spawn the team, run specialist analysis, challenge via antagonist, synthesize findings, auto-fix mechanical issues, then re-review. Exit when no mechanical fixes are applied or `max_iterations` is reached.

#### 2a. Initialize team

**CREATE** agent team with specialists appropriate for this iteration and scope. Each receives the target-dir path and their assigned review scope.

**First iteration (`scope = full`)**: Spawn all three specialists. This is the comprehensive initial review.

**First iteration (`scope = quick`)**: Spawn only the **Structural Specialist** and **Quality Specialist**. Skip the Integration Specialist.

**Subsequent iterations (either scope)**: Spawn only the **Structural Specialist** and **Quality Specialist**, scoped to files modified by the previous iteration's fixes. Skip the Integration Specialist -- mechanical fixes do not alter FFI type mappings or build configuration.

**Spawn Structural Specialist**:

```text
You are a Structural Reviewer for a Crux iOS shell at $TARGET_DIR.

Read `references/ios-review-checks.md`.

Apply checks IOS-001 through IOS-018 against the Swift source. These are
pattern-based checks that verify the shell correctly maps to the Crux core:

- ViewModel/screen view correspondence
- Effect handler completeness
- Event dispatch coverage
- Route/navigation completeness
- Design system token usage
- ContentView switch exhaustiveness
- ScrollView interaction hazards (touch delay, nested gesture conflicts)

For each finding, report: check ID (IOS-NNN), file:line, code snippet,
severity (Critical or Warning), risk description, suggested fix, and
whether it is auto-fixable (mechanical).

Output your findings as a numbered list in markdown. Prefix each finding
ID with "IOS-" (e.g., IOS-001-1, IOS-005-1).
```

If `iteration > 1`, append: "Scope your analysis to these files modified in the previous iteration: [list of changed files]."

**Spawn Quality Specialist**:

```text
You are a Quality Reviewer for a Crux iOS shell at $TARGET_DIR.

Read `references/swift-quality-checks.md`.

Apply checks SWF-001 through SWF-010 against all `.swift` files. These are
Swift/SwiftUI best practice checks:

- Concurrency correctness (`@MainActor`, `Sendable`)
- No force unwraps in production code
- Accessibility labels on interactive elements
- SwiftUI state management (`@Published`, `@ObservedObject`, `@State`)
- Preview coverage
- swiftformat compliance

For each finding, report: check ID (SWF-NNN), file:line, code snippet,
severity (Warning or Info), risk description, suggested fix, and whether
it is auto-fixable.

Output your findings as a numbered list in markdown. Prefix each finding
ID with "SWF-" (e.g., SWF-001-1, SWF-006-1).
```

If `iteration > 1`, append: "Scope your analysis to these files modified in the previous iteration: [list of changed files]."

**Spawn Integration Specialist** (first iteration only; skip if scope = quick):

```text
You are an Integration Reviewer for a Crux iOS shell at $TARGET_DIR.

Cross-reference the Rust core types (`shared/src/app.rs`) against the
Swift implementation:

1. Type completeness -- every FFI-crossing type in `app.rs` must have a
   corresponding Swift type in the generated bindings.
2. Serialization correctness -- verify Bincode serialize/deserialize calls
   use the correct types.
3. Build configuration -- verify `project.yml` references the correct
   shared library path, correct deployment target, correct Swift version.
4. Capability alignment -- every Effect variant in `app.rs` must have a
   handler in `Core.swift`.

For each finding, report: a finding ID (INT-NNN), file:line, code snippet,
severity (Critical or Warning), risk description, suggested fix, and
whether it is auto-fixable.

Output your findings as a numbered list in markdown. Prefix each finding
ID with "INT-" (e.g., INT-001, INT-002).
```

#### 2b. Specialist analysis (concurrent)

The specialists analyze the shell concurrently. Each reads all `.swift` files under `iOS/` (and `shared/src/app.rs` for the Integration Specialist) but reports only on their assigned checks.

**Lead waits** for all specialists to complete before proceeding.

#### 2c. Universal checks (lead; skip if scope = quick)

After all specialists report, the lead reads `../../references/review-checks.md` and applies checks UNI-001 through UNI-021 with Swift-specific detection. Several universal checks overlap with categories already assigned to the specialists. Skip those and focus on the gaps:

| Universal check | Already covered by | Action |
|---|---|---|
| UNI-003 Serialization failures | IOS-013, IOS-015, IOS-016 | Skip |
| UNI-006 Race conditions | SWF-003, IOS-014 | Skip |
| UNI-010 Panics/crashes | SWF-001 | Skip |

Apply the remaining checks with these Swift-specific heuristics:

- **UNI-001** (uninitialised values): Look for `var` properties initialised to `nil` or placeholder values that are accessed before an async load completes. Check for `@Published` properties with default values that represent an invalid domain state.
- **UNI-002** (unvalidated input): Look for shell-side text inputs (e.g., `TextField` bindings) dispatched to the core via `core.update()` without local trim or empty check. While the core should also validate, the shell should prevent obviously invalid dispatches.
- **UNI-004** (logic bugs): Reason about the `processEffect` switch for missing cases, incorrect effect resolution sequences, and navigation handlers that produce unreachable states.
- **UNI-005** (unbounded growth): Look for strong reference cycles (`self` captured in closures without `[weak self]` where the closure outlives the expected scope), never-cancelled `Task` instances, and growing arrays of SSE observations or event subscriptions without cleanup.
- **UNI-007** (chatty calls): Look for `URLSession` calls that re-fetch data the core already has from SSE or other real-time channels. Check for effect handlers that fire identical resolve calls on repeated renders.
- **UNI-008** (instrumentation balance): Look for error paths with no `assertionFailure` or `os.Logger` call. Flag per-event logging inside hot loops (e.g., logging every SSE message body).
- **UNI-009** (handle-then-throw): Look for `do/catch` blocks that partially update `self.view` or other `@Published` properties before re-throwing, leaving the UI in an inconsistent state.
- **UNI-011** (timeout/retry): Look for `URLSession` requests with no `timeoutInterval` configured. Check whether effect handlers have a retry or fallback path for transient network failures.
- **UNI-012** (persisted state compat): Look for `Codable` model changes (new properties without default values) that would break decoding of existing `UserDefaults` or file-persisted data.
- **UNI-013** (dead code): Look for switch cases that can never match, unreachable code after `return` / `break`, and unused private functions or properties.
- **UNI-014** (hardcoded config): Look for hardcoded timeout intervals, literal URL strings, and magic number page sizes or retry counts.
- **UNI-015** (stale captures): Look for `Task` blocks capturing `self` or local state that may mutate before the async work completes. Check for closures that capture loop variables.
- **UNI-016** (error message quality): Look for `assertionFailure` messages with no context about which item or operation failed, and catch blocks that log the error type but not the message.
- **UNI-017** (type safety): Look for `String` properties on view models or event types that hold values from a known closed set (should be Swift enums).
- **UNI-018** (hardcoded secrets): Look for API keys, tokens, passwords, or connection strings embedded as string literals in Swift source files. Check for secrets in `Info.plist` values, hardcoded `Authorization` headers, and credentials stored in plain-text constants rather than Keychain.
- **UNI-019** (injection vulnerabilities): Look for user input interpolated into `WKWebView` HTML content without escaping, URL path segments built via string concatenation, and `Process`/`NSTask` invocations with user-controlled arguments.
- **UNI-020** (unsafe deserialization): Look for `JSONDecoder` decoding of untrusted external payloads directly into model types that carry privilege state. Check for missing `Content-Length` header checks, `URLResponse.expectedContentLength` checks, or explicit payload size limits on data fetched from external sources.
- **UNI-021** (missing auth checks): Check that effect handlers attaching authentication credentials (Bearer tokens, API keys) to outbound requests source them from secure storage (Keychain), not from hardcoded values or unprotected UserDefaults. Flag API calls to protected endpoints dispatched without any auth header.

Prefix findings from this step with `UNI-` (e.g., UNI-1, UNI-2). Use the severity defined in the universal checklist for each check.

Tag findings that have a **Spec-change indicator** (UNI-002, UNI-004, UNI-007, UNI-008, UNI-011, UNI-012, UNI-014, UNI-021) for inclusion in the adversarial review and spec-change output in step 3.

#### 2d. Adversarial challenge

After the specialist reports and universal checks are complete, the lead sends all combined findings (IOS-, SWF-, INT-, and UNI- prefixed) to the antagonist.

**Spawn Antagonist**:

```text
You are the Antagonist Reviewer for a Crux iOS shell at $TARGET_DIR.

You receive findings from specialist reviewers (Structural, Quality,
Integration) and from the lead's universal checks. Your job is to
challenge every finding and find what they missed.

For EACH finding (IOS-, SWF-, INT-, and UNI- prefixed):
1. Validate evidence: Is there a real file:line reference and code snippet?
2. Challenge severity: Is Critical really critical? Is Info actually higher?
3. Check for false positives: Could this be a non-issue or acceptable
   SwiftUI pattern?
4. Assess auto-fix safety: Could the suggested fix introduce regressions?

Then perform a COUNTER-SCAN of all `.swift` files under `iOS/` looking
for issues ALL specialists missed. Common SwiftUI blind spots:
- Missing `@MainActor` on classes that update `@Published` properties
- `Sendable` conformance violations in async contexts
- Preview data that is stale relative to the current ViewModel structure
- Retain cycles from `self` capture in Task or URLSession closures
- Navigation state inconsistencies (deep link paths not handled)
- Missing `onDisappear` cleanup for SSE or timer subscriptions
- Hardcoded design tokens that don't match `tokens.yaml`

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

The antagonist:

1. Reviews every finding for evidence quality and severity accuracy
2. Performs a counter-scan for missed SwiftUI-specific issues
3. Sends challenged report to lead with: confirmed, downgraded, upgraded, disputed, and new findings

#### 2e. Synthesis

The lead merges all findings (specialist reports, universal checks, and antagonist challenges) into a single iteration report:

1. **Confirmed findings**: Include verbatim from specialist reports
2. **Downgraded findings**: Include with the antagonist's revised severity and rationale
3. **Upgraded findings**: Include with the antagonist's revised severity and rationale
4. **Disputed findings**: Lead makes final call; if included, add dispute note
5. **New findings**: Include with the antagonist's severity and evidence
6. Assign overall **confidence level** per [Agent Team Patterns - Confidence Scoring](references/agent-teams.md#confidence-scoring)

#### 2f. Produce iteration report

Output the synthesized findings for this iteration. On the first iteration, use the full report format. On subsequent iterations, report only new findings discovered in re-review and note the iteration number.

````
## iOS Shell Review Report: {app-name} (iteration {N})

**Review Team**: 3 specialists + 1 antagonist
**Confidence Level**: [HIGH | MEDIUM | LOW]

### Summary
- Critical: N findings
- Warning: N findings
- Info: N findings

### Critical Findings

#### [IOS-001-1] Missing screen view for ViewModel variant
- **File**: iOS/{AppName}/ContentView.swift
- **Reviewer**: Structural Specialist
- **Antagonist**: Confirmed
- **Issue**: ViewModel variant `Settings(SettingsView)` has no corresponding
  screen view file.
- **Fix**: Create `Views/SettingsScreen.swift` and add the case to ContentView.

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

The **lead** applies all auto-fixes directly (specialists and antagonist have completed their analysis). The finding prefix (IOS-, SWF-, INT-, UNI-, NEW-) tracks which reviewer or pass identified the issue for accountability in the report.

Apply fixes for findings that are mechanical and confirmed or upgraded (not disputed):

- Adding missing accessibility labels
- Adding missing `import VectisDesign`
- Replacing hardcoded colors with `VectisColors` tokens
- Replacing hardcoded spacing with `VectisSpacing` tokens
- Adding missing `#Preview` blocks
- Adding missing Inject boilerplate (`import Inject`, `@ObserveInjection var inject`, `.enableInjection()`) to view files

Do NOT auto-fix structural issues (missing screen views, missing effect handlers) without confirmation -- these may require design decisions about layout and interaction. Respect antagonist regression flags.

After fixes, run `swiftformat` on modified files. If fixes cause build errors, revert all auto-fixes and warn in the report.

#### 2h. Loop control

After applying fixes, verifying, and shutting down the team:

1. If **no mechanical fixes** were applied, exit the cycle.
2. If `iteration >= max_iterations`, exit the cycle.
3. Otherwise, increment `iteration` and return to step 2a.

When the cycle exits, shut down all remaining teammates and output a summary across all iterations:

```
### Review Cycle Summary
- Iteration 1: Fixed N mechanical issues (IOS-005 x2, SWF-006, UNI-016).
  M design-level findings deferred. Confidence: HIGH.
- Iteration 2: Fixed K regressions from iteration 1 fixes.
  No new design-level findings. Confidence: HIGH.
- Total: N+K mechanical fixes applied. M design-level findings accumulated.
```

### 3. Express accumulated design-level findings

After the review-fix cycle completes, check whether any **design-level findings** were accumulated -- findings that require architectural decisions, missing screen views, missing effect handlers, or issues that indicate the spec is incomplete (typically IOS-001, IOS-003, IOS-010, and universal checks tagged with a **Spec-change indicator**). If none were accumulated across any iteration, skip this step.

#### Classify findings: code-fix vs spec-change

Classify each design-level finding:

- **Code-fix**: The spec is clear and the code simply does not implement it correctly. The fix is a code change; no spec update is needed. These become tasks in `tasks.md`.
- **Spec-change**: The spec is silent, ambiguous, or mandates behavior that the review identified as problematic. The fix requires updating the spec first, then implementing. These become requirements in `specs/` and decisions in `design.md`.

Universal checks with a Spec-change indicator (UNI-002, UNI-004, UNI-007, UNI-008, UNI-011, UNI-012, UNI-014, UNI-021) commonly surface as spec-change findings. Consult `../../references/review-checks.md` for the indicator description on each check.

#### When `orchestrated: true` (build-phase invocation)

Return the classified findings in the `design_findings` output field
and stop. Each finding entry includes: finding ID (e.g., IOS-001-1),
check ID, severity, classification (`code-fix` or `spec-change`),
file:line, description, and suggested fix. The orchestrator
consolidates findings from all platform reviewers and creates a
single Specify change. Do **not** call `/spec:define`.

#### When `orchestrated: false` (standalone invocation, default)

If design-level findings exist, delegate to `/spec:define` to create a
single Specify change that tracks all of them:

1. **Derive a change name** from the app name and append the current date-time for traceability:

   ```
   review-{app-name}-ios-{YYYY-MM-DDTHH-MM}
   ```

   Example: `review-my-crux-app-ios-2026-03-25T10-30`

   Use the shell to get the current timestamp:
   ```bash
   date -u +"%Y-%m-%dT%H-%M"
   ```

2. **Delegate to `/spec:define`** with the derived change name and a description synthesized from the accumulated design-level findings. Provide the following guidance for artifact generation:

3. **Content guidelines for each artifact**:

   - **proposal.md**: The "Why" section summarizes the accumulated review findings by severity and risk, distinguishing spec-change findings (requirements gaps) from code-fix findings (implementation bugs). The "What Changes" section lists each design-level finding as a bullet, prefixed with `[spec]` or `[code]` to indicate its classification. Note which mechanical fixes were already applied across all iterations and how many review cycles ran. The "Impact" section identifies affected files, core contract changes, and migration concerns.

   - **design.md**: Each design-level finding becomes a Decision section with rationale and alternatives considered. Group related findings (e.g., all effect-handler-related changes under one decision). Reference the specific check IDs (IOS-xxx, SWF-xxx, UNI-xxx) that motivated each decision. For spec-change findings, explain why the current spec is insufficient and what the proposed requirement should be.

   - **specs/**: Create one spec file per logical area (e.g., `ios-shell-effects`, `ios-shell-navigation`). Each requirement maps to a review finding. Spec-change findings become new requirements with explicit acceptance criteria. Code-fix findings become scenarios under existing requirements. Use WHEN/THEN format.

   - **tasks.md**: Order tasks by dependency -- spec updates first (so requirements are clear before implementation), then missing screen views, then missing effect handlers, then navigation fixes, then design system corrections, then verification. Each task references the finding ID it addresses. Include a final verification section that re-runs the ios-reviewer skill to confirm all Critical findings are resolved.

4. **Show final status** by running `specify change status <name>` and summarize: change name, location, artifacts created, and prompt the user with "Run `/spec:build` or ask me to implement to start working on the tasks."

## Severity Definitions

| Severity | Meaning | Action |
|---|---|---|
| **Critical** | Missing screen views, missing effect handlers, broken build, data not rendered | Must fix before merge |
| **Warning** | Hardcoded tokens, missing previews, accessibility gaps, style inconsistencies | Should fix; acceptable to defer |
| **Info** | Minor improvements, alternative patterns | Fix if convenient |

## Verification Checklist

Before completing review:

### Team Execution

- [ ] All specialists spawned with correct category assignments
- [ ] All specialists completed before antagonist spawned
- [ ] Antagonist received all specialist + universal findings
- [ ] Antagonist provided evidence for every challenge
- [ ] Lead synthesized all findings with confidence scoring
- [ ] Team shut down and cleaned up

### Scan Coverage

- [ ] Structural Specialist: IOS-001 through IOS-018 checked
- [ ] Quality Specialist: SWF-001 through SWF-010 checked
- [ ] Integration Specialist: type completeness, serialization, build config, capability alignment checked (first iteration, full scope)
- [ ] Universal Checks: UNI-001 through UNI-021 applied with Swift-specific heuristics (skipped where covered by IOS/SWF)
- [ ] Antagonist: counter-scan completed for SwiftUI-specific blind spots

### Report Quality

- [ ] Each issue has file:line reference and code snippet
- [ ] Severity reflects antagonist adjustments (upgrades/downgrades applied)
- [ ] Adversarial Review section included with challenge statistics
- [ ] Confidence level assigned based on antagonist results
- [ ] Finding IDs use correct prefixes (IOS-, SWF-, INT-, UNI-, NEW-)
- [ ] Design-level findings classified as code-fix or spec-change

## Integration with Specify Workflow

This skill is invoked as part of the Vectis build phase, after ios-writer
generation and build verification. When invoked from the build phase, the
orchestrator passes `orchestrated: true` so that this skill returns
design-level findings for cross-platform consolidation instead of calling
`/spec:define` directly. iOS and Android reviews run in parallel since
they operate on disjoint file trees with no build-tool contention:

```
define -> build (ios-writer + android-writer in parallel)
       -> verify iOS -> verify Android (serial -- cargo workspace lock)
       -> review iOS + review Android (parallel -- code analysis only)
       -> orchestrator consolidates design-level findings -> merge
```

The skill can also be invoked standalone (with `orchestrated: false`,
the default), in which case it creates its own Specify change for any
design-level findings:

> Use the ios-reviewer skill to review `<target-dir>`

> Review the iOS shell at `<target-dir>` against `<reference-dir>` as a reference
