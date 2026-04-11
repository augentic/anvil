---
name: core-reviewer
description: Review generated Crux core (Rust shared crate) code for structural issues, logic bugs, and quality problems. Use when reviewing a Crux app's core after generation, or when the user mentions core-reviewer.
---

# Crux Core Reviewer

Systematically review the generated Crux core (Rust `shared` crate) for
structural issues, logic bugs, and general code quality problems. Produces
a severity-graded report with actionable findings and suggested fixes.

This skill catches semantic issues that compilers, linters, and clippy miss:
missing `render()` calls, conflict-resolution gaps, pending-op coalescing bugs,
state machine incompleteness, and interaction-sequence race conditions.

## Arguments

| Argument | Required | Description |
|---|---|---|
| `target-dir` | **Yes** | Path to the Crux app directory to review (contains `shared/src/`) |
| `reference-dir` | No | Path to a known-good Crux app for comparative review |
| `scope` | No | `full` (default) runs all five passes (structural, logic, quality, universal, comparative); `quick` runs structural + quality only, skipping logic simulation, universal checks, and comparative review |

## Process

This skill uses an agent team with 3 specialist reviewers and 1 antagonist.
The lead coordinates the team, synthesizes findings, and produces the final
report. See [Agent Team Patterns](references/agent-teams.md) for shared
protocols (team roles, antagonist protocol, synthesis rules, file ownership,
and confidence scoring).

### 1. Gather context

Read the following files from `{target-dir}`:

- `spec.md` -- the app specification (required for logic specialist)
- `shared/Cargo.toml` -- dependencies and features
- All `.rs` files under `shared/src/` -- focus on `app.rs` (the `update()` function)

If `reference-dir` is provided, also read the corresponding files from the
reference app. Differences between the two highlight potential regressions.

### 2. Review-fix cycle (max 3 iterations)

Follow the [Reviewer Workflow](../../../references/reviewer-workflow.md) for
the review-fix cycle, synthesis, auto-fix, and loop control orchestration.
The platform-specific details below define the specialists, universal check
heuristics, and auto-fix list for this cycle.

#### 2a. Initialize team

The domain-specific specialist for Crux core is the **Logic Specialist**
(skipped on `quick` scope and subsequent iterations).

**Spawn Structural Specialist**:

```text
You are a Structural Reviewer for a Crux shared crate at $TARGET_DIR.

Read `references/crux-review-checks.md`.

Apply checks CRX-001 through CRX-011 against the source code. These are
pattern-based checks that scan for known Crux-specific issues:

- Missing `render()` after state mutations
- Missing serde derives on bridge-crossing types
- Input validation gaps on user-supplied text
- Timestamp completeness on `PendingOp` variants
- ViewModel field typing (typed values vs pre-formatted strings)
- Unused dependencies in `Cargo.toml`

For each finding, report: check ID (CRX-NNN), file:line, code snippet,
severity (Critical or Warning), risk description, suggested fix, and
whether it is auto-fixable (mechanical).

Output your findings as a numbered list in markdown. Prefix each finding
ID with "CRX-" (e.g., CRX-001-1, CRX-005-1).
```

If `iteration > 1`, append: "Scope your analysis to these files modified
in the previous iteration: [list of changed files]."

**Spawn Logic Specialist** (first iteration only; skip if scope = quick):

```text
You are a Logic Reviewer for a Crux shared crate at $TARGET_DIR.

Read `references/logic-review-checks.md`.

Also read `spec.md` for the app specification -- you need it for spec gap
detection and spec-to-test coverage analysis.

Apply checks LOG-001 through LOG-009. These require reasoning about event
sequences, not just pattern matching. For each check:

1. LOG-001 State machine completeness -- Enumerate every state enum
   (Page, SyncStatus, SseConnectionState, etc.). For each transition in
   `update()`, verify that all required side-effects fire (render, save,
   sync). Draw the state machine mentally; flag incomplete edges.

2. LOG-002 Operation coalescing -- Trace the sequence: Create -> Delete
   before sync. Does the code skip the server call for items that were
   never synced? Check both delete and clear-completed handlers.

3. LOG-003 Concurrent operation conflicts -- Trace: sync in-flight + SSE
   event for the same item. Does `pending_ops.retain()` in the SSE handler
   clobber the in-flight sync state?

4. LOG-004 Temporal ordering -- For every conflict-resolution comparison,
   verify both sides have timestamps. Check `PendingOp` variants for
   missing temporal fields.

5. LOG-005 Fallback-on-None -- For every `unwrap_or_default()`, `Option`
   with `_ => true`, or `None` fallback, check if the default is
   semantically correct in the domain.

6. LOG-006 Rapid-action sequences -- Trace what happens when the user
   fires the same action twice before the first async operation completes.
   Check for duplicate pending ops or unbounded queue growth.

7. LOG-007 Spec gap detection -- Compare each user-facing Event variant
   against the Features section of `spec.md`. Flag events that accept
   untrusted input without validation that common sense requires (empty
   strings, negative numbers, duplicate IDs) even if the spec is silent.

8. LOG-008 Spec-to-test coverage gap -- For each `#### Scenario:` in
   `spec.md`, verify a test with a matching `/// Spec:` traceability
   comment (referencing the stable `REQ-XXX` ID **and** the scenario
   title) exists in the `#[cfg(test)]` module. A single requirement can
   contain multiple scenarios; matching by ID alone is insufficient.
   Additionally, cross-reference interaction sequences from LOG-001--007
   to verify edge-case coverage. List all missing scenarios.

9. LOG-009 Stale tests -- Identify tests with `/// Spec:` traceability
   comments that reference scenarios no longer present in the spec. Flag
   them for human review. Do not auto-delete.

For each finding, report: check ID (LOG-NNN), file:line, code snippet,
severity (Critical for data loss/incorrect server calls/conflict-resolution
failure; Warning for stale UI/missing tests), risk description, suggested
fix, and whether it is auto-fixable.

Output your findings as a numbered list in markdown. Prefix each finding
ID with "LOG-" (e.g., LOG-001-1, LOG-003-1).
```

**Spawn Quality Specialist**:

```text
You are a Quality Reviewer for a Crux shared crate at $TARGET_DIR.

Read `references/general-review-checks.md`.

Apply checks GEN-001 through GEN-012 against all `.rs` files. These are
language-level quality checks:

- No `unwrap()`/`expect()` in production code (tests exempt)
- No debug output (`println!`, `dbg!`, `eprintln!`)
- No hardcoded secrets or credentials
- Error propagation (not silent swallowing)
- Match arm exhaustiveness
- Serialization round-trip completeness
- Function length (under 50 lines)

For each finding, report: check ID (GEN-NNN), file:line, code snippet,
severity (Warning or Info), risk description, suggested fix, and whether
it is auto-fixable.

Output your findings as a numbered list in markdown. Prefix each finding
ID with "GEN-" (e.g., GEN-001-1, GEN-005-1).
```

If `iteration > 1`, append: "Scope your analysis to these files modified
in the previous iteration: [list of changed files]."

#### 2b-c. Specialist analysis and universal checks

Specialists run concurrently per the shared workflow. Universal checks
use Rust-specific heuristics. Skip table:

| Universal check | Already covered by | Action |
|---|---|---|
| UNI-002 Unvalidated input | CRX-002, LOG-007 | Skip |
| UNI-003 Serialization failures | CRX-005, GEN-009 | Skip |
| UNI-004 Logic bugs | LOG-001..008 | Skip |
| UNI-006 Race conditions | LOG-003, LOG-006 | Skip |
| UNI-010 Panics/crashes | GEN-001, CRX-011 | Skip |
| UNI-017 Type safety (partial) | CRX-008 | Apply beyond ViewModel |
| UNI-018 Hardcoded secrets | GEN-003 | Skip |

Apply the remaining checks with these Rust-specific heuristics:

- **UNI-001** (uninitialised values): Look for `#[derive(Default)]` on
  structs where the default value has no valid domain meaning. Check
  `Option::None` fields accessed without distinguishing "not loaded" from
  "intentionally empty".
- **UNI-005** (unbounded growth): Look for `Vec` or `VecDeque` fields that
  receive `.push()` without corresponding `.remove()`, `.retain()` bounds,
  or capacity limits. Check for `Command` futures that are never cancelled.
- **UNI-007** (chatty calls): Look for duplicate `HttpRequest` calls
  fetching the same data, SSE reconnect handlers that re-fetch data already
  delivered by the SSE event, and missing debounce on rapid-fire user
  actions.
- **UNI-008** (instrumentation balance): Look for `Err` branches with no
  `log::error!` or `log::warn!`. Flag `log::debug!` or `log::info!` inside
  loops over collection items. Check for PII in log interpolations.
- **UNI-009** (handle-then-throw): Look for `Err(e) => { model.field = ...;
  return Err(e) }` patterns where the model mutation is visible to the view
  but the error also propagates, leaving the UI in an inconsistent state.
- **UNI-011** (timeout/retry): Check whether effect handlers account for
  external calls that may hang or fail transiently. In the Crux core, this
  surfaces as missing timeout events or retry commands.
- **UNI-012** (persisted state compat): Check whether `PersistedState` struct
  changes include `#[serde(default)]` on new fields and whether removed
  fields use `#[serde(skip)]` or migration logic.
- **UNI-013** (dead code): Look for match arms shadowed by earlier guards,
  functions with no call sites, and Event variants never dispatched by any
  view.
- **UNI-014** (hardcoded config): Look for magic-number timeouts, hardcoded
  URL strings, and literal page sizes or retry counts.
- **UNI-015** (stale captures): Look for `Command` chains that capture
  model field values before an async operation and use the snapshot after
  resolution, when the model may have been mutated by intervening events.
- **UNI-016** (error message quality): Look for error messages with no item
  IDs, field names, or operation context.
- **UNI-017** (type safety): Beyond CRX-008 (ViewModel), look for `String`
  fields on model types, Event payloads, or PendingOp variants that hold
  values from a known closed set (should be enums or newtypes).
- **UNI-019** (injection vulnerabilities): Crux cores do not access
  databases or spawn processes directly (these go through effects), but
  check for user input interpolated into URL path segments, query strings,
  or HTML/XML output built as strings. Also check for `format!` used to
  construct structured data (JSON, SQL, URLs) rather than proper builders.
- **UNI-020** (unsafe deserialization): Look for deserialization of
  untrusted external payloads (SSE events, HTTP responses) directly into
  internal model types that carry authorization or privilege state. Check
  for missing size limits on payloads deserialized from effects.
- **UNI-021** (missing auth checks): In a Crux core, authentication is
  typically managed by the shell and passed as model state. Check that
  handlers for sensitive operations (delete, admin actions) verify
  `model.auth_state` or equivalent before proceeding. Flag handlers that
  assume authentication without checking.

Prefix findings from this step with `UNI-` (e.g., UNI-1, UNI-2). Use the
severity defined in the universal checklist for each check.

Tag findings that have a **Spec-change indicator** (UNI-002, UNI-004,
UNI-007, UNI-008, UNI-011, UNI-012, UNI-014, UNI-021) for inclusion in
the adversarial review and spec-change output in step 3.

#### 2d. Comparative review (first iteration only; if reference-dir provided; skip if scope = quick)

Compare structural decisions between the target and reference apps:

- Event variant signatures (do they carry timestamps/IDs from the shell?)
- PendingOp variant structure (do they carry enough data for conflict resolution?)
- ViewModel field types (typed vs pre-formatted)
- Test coverage breadth (count and categorize tests in both)

Flag significant divergences as Warning with a note explaining what the
reference app does differently and why. Prefix findings with `CMP-`.

#### 2e. Adversarial challenge

After the specialist reports, universal checks, and comparative review are
complete, the lead sends all combined findings (CRX-, LOG-, GEN-, UNI-, and
CMP- prefixed) to the antagonist.

**Spawn Antagonist**:

```text
You are the Antagonist Reviewer for a Crux shared crate at $TARGET_DIR.

You receive findings from specialist reviewers (Structural, Logic, Quality)
and from the lead's universal and comparative checks. Your job is to
challenge every finding and find what they missed.

For EACH finding (CRX-, LOG-, GEN-, UNI-, and CMP- prefixed):
1. Validate evidence: Is there a real file:line reference and code snippet?
2. Challenge severity: Is Critical really critical? Is Info actually higher?
3. Check for false positives: Could this be a non-issue or acceptable
   Crux pattern?
4. Assess auto-fix safety: Could the suggested fix introduce regressions?

Then perform a COUNTER-SCAN of all `.rs` files in `shared/src/` looking
for issues ALL specialists missed. Common Crux blind spots:
- Missing `render()` in deeply nested match arms (not just top-level)
- Effect ordering bugs (render before vs after async command chains)
- Model mutation without corresponding Command return
- State machine edges that silently drop events (no-op match arms)
- PendingOp cleanup paths that leak entries on error
- Stale model field reads after `.and()` chains that may have mutated state

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

#### 2f-g. Synthesis and iteration report

Follow the shared workflow synthesis steps. Report template:

````
## Code Review Report: {app-name} (iteration {N})

**Review Team**: 3 specialists + 1 antagonist
**Confidence Level**: [HIGH | MEDIUM | LOW]

### Summary
- Critical: N findings
- Warning: N findings
- Info: N findings

### Critical Findings

#### [CRX-001-1] Missing render() after page transition
- **File**: shared/src/app.rs, lines 384-388
- **Reviewer**: Structural Specialist
- **Antagonist**: Confirmed
- **Issue**: Navigating from Error to Loading mutates `model.page` without
  emitting `render()`. The shell may not see the Loading state.
- **Fix**: Wrap the return in `render().and(Command::event(Event::Initialize))`

... (one block per finding, ordered by severity then file)

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

### Test Gap Summary
- Missing test for: [scenario description]
- Missing test for: ...
````

Classify each finding as **mechanical** (auto-fixable) or **design-level**
(requires architectural decisions). Add design-level findings to the
accumulated list.

#### 2h. Auto-fix mechanical issues

Apply mechanical auto-fixes per the shared workflow. Platform-specific
fixes for Crux core:

- Adding missing `Serialize`/`Deserialize` derives
- Wrapping returns in `render().and(...)`
- Adding `.trim()` and empty checks on text inputs
- Removing unused dependencies from `Cargo.toml`

Do NOT auto-fix logic bugs (LOG-001 through LOG-008) without explicit
confirmation -- these require design decisions.

After any fixes, re-run `cargo check`, `cargo test`, and `cargo clippy`.

#### 2i. Loop control

Follow the shared workflow loop control. Exit when no fixes are applied or
`max_iterations` is reached.

### 3. Express accumulated design-level findings as a Specify change

Follow the [Reviewer Workflow](../../../references/reviewer-workflow.md)
design-level findings procedure. Use platform suffix `core` in the change
name template (`review-{app-name}-core-{YYYY-MM-DDTHH-MM}`).

Core-specific guidance for artifacts:

- **specs/**: Group by logical area (e.g., `sync-logic`,
  `input-validation`, `resilience`). Derive scenarios from the Logic
  Specialist simulation traces (LOG-001 through LOG-008).
- **tasks.md**: Order by dependency -- spec updates, data-type changes,
  event signatures, handler logic, test updates, verification. Include a
  final task that re-runs core-reviewer to confirm resolution.

## Severity Definitions

| Severity | Meaning | Action |
|---|---|---|
| **Critical** | Data loss, incorrect server calls, conflict-resolution failure, panic in production | Must fix before merge |
| **Warning** | Stale UI, missing tests, suboptimal types, unnecessary clones | Should fix; acceptable to defer with justification |
| **Info** | Style, documentation, minor improvements | Fix if convenient |

## Verification Checklist

Team Execution and Report Quality checks are in the shared
[Reviewer Workflow](../../../references/reviewer-workflow.md#common-verification-checklist-items).

### Scan Coverage

- [ ] Structural Specialist: CRX-001 through CRX-011 checked
- [ ] Logic Specialist: LOG-001 through LOG-009 checked (first iteration)
- [ ] Quality Specialist: GEN-001 through GEN-012 checked
- [ ] Universal Checks: UNI-001 through UNI-021 applied with Rust-specific
  heuristics (skipped where covered by CRX/LOG/GEN)
- [ ] Antagonist: counter-scan completed for Crux-specific blind spots
- [ ] Comparative review completed (if reference-dir provided)

## Integration with Specify Workflow

This skill is invoked as part of the Vectis build phase, after core-writer
generation and compiler verification, before merge:

```
define -> build (core-writer) -> test (test-writer) -> verify -> review-fix cycle (this skill, up to 3 iterations) -> generate change for design issues -> merge
```

The review-fix cycle auto-fixes mechanical issues and re-reviews its own
fixes, iterating until the code is clean or the iteration limit is reached.
Design-level findings from all iterations are accumulated into a single
Specify change with all artifacts (proposal, design, specs, tasks) ready
for implementation. This makes the output of a review directly actionable --
the user can immediately run `/spec:build` to start fixing the issues.

The skill can also be invoked standalone at any time:

> Use the core-reviewer skill to review `<project-dir>`

> Review `<project-dir>` against `<reference-dir>` as a reference
