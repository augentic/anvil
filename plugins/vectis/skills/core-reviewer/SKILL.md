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

### 1. Gather context

Read the following files from `{target-dir}`:

- `spec.md` -- the app specification (required for logic pass)
- `shared/Cargo.toml` -- dependencies and features
- All `.rs` files under `shared/src/` -- focus on `app.rs` (the `update()` function)

If `reference-dir` is provided, also read the corresponding files from the
reference app. Differences between the two highlight potential regressions.

### 2. Review-fix cycle (max 3 iterations)

Before starting, initialize:

- `iteration = 1`, `max_iterations = 3`
- An empty list of **accumulated design-level findings** (carried across
  iterations)

The cycle repeats: run review passes, report findings, auto-fix mechanical
issues, then re-review the fixes. The cycle exits when no mechanical fixes
are applied or `max_iterations` is reached.

#### 2a. Select passes for this iteration

**First iteration** (`iteration = 1`): Run all five passes -- structural,
logic, quality, universal, and comparative. This is the comprehensive
initial review.

**Subsequent iterations** (`iteration > 1`): Run only the **structural pass**,
**quality pass**, and **universal pass**, scoped to files modified by the
previous iteration's fixes. Skip the logic pass and comparative review --
mechanical fixes (serde derives, `render().and()`, `.trim()` checks) do not
alter event sequences or conflict-resolution logic.

#### 2b. Structural pass

Read `references/crux-review-checks.md` in this skill's directory.

Apply checks CRX-001 through CRX-011 against the source code. These are
pattern-based checks that scan for known Crux-specific issues:

- Missing `render()` after state mutations
- Missing serde derives on bridge-crossing types
- Input validation gaps on user-supplied text
- Timestamp completeness on `PendingOp` variants
- ViewModel field typing (typed values vs pre-formatted strings)
- Unused dependencies in `Cargo.toml`

For each violation found, record: check ID, file, line range, description,
severity (Critical or Warning), and suggested fix.

#### 2c. Logic pass (first iteration only; skip if scope = quick)

Read `references/logic-review-checks.md` in this skill's directory.

Apply checks LOG-001 through LOG-008. These require reasoning about event
sequences, not just pattern matching. For each check:

1. **LOG-001 State machine completeness** -- Enumerate every state enum
   (Page, SyncStatus, SseConnectionState, etc.). For each transition in
   `update()`, verify that all required side-effects fire (render, save, sync).
   Draw the state machine mentally; flag incomplete edges.

2. **LOG-002 Operation coalescing** -- Trace the sequence: Create -> Delete
   before sync. Does the code skip the server call for items that were never
   synced? Check both delete and clear-completed handlers.

3. **LOG-003 Concurrent operation conflicts** -- Trace: sync in-flight +
   SSE event for the same item. Does `pending_ops.retain()` in the SSE handler
   clobber the in-flight sync state?

4. **LOG-004 Temporal ordering** -- For every conflict-resolution comparison,
   verify both sides have timestamps. Check `PendingOp` variants for missing
   temporal fields.

5. **LOG-005 Fallback-on-None** -- For every `unwrap_or_default()`, `Option`
   with `_ => true`, or `None` fallback, check if the default is semantically
   correct in the domain.

6. **LOG-006 Rapid-action sequences** -- Trace what happens when the user
   fires the same action twice before the first async operation completes.
   Check for duplicate pending ops or unbounded queue growth.

7. **LOG-007 Spec gap detection** -- Compare each user-facing Event variant
   against the Features section of `spec.md`. Flag events that accept untrusted
   input without validation that common sense requires (empty strings, negative
   numbers, duplicate IDs) even if the spec is silent.

8. **LOG-008 Missing edge-case tests** -- Cross-reference the `#[cfg(test)]`
   module against the interaction sequences from LOG-001--007. Each identified
   risk should have at least one test.

Record findings with severity Critical (data loss, incorrect server calls) or
Warning (stale UI, missing tests).

#### 2d. Quality pass

Read `references/general-review-checks.md` in this skill's directory.

Apply checks GEN-001 through GEN-012 against all `.rs` files. These are
language-level quality checks:

- No `unwrap()`/`expect()` in production code (tests exempt)
- No debug output (`println!`, `dbg!`, `eprintln!`)
- No hardcoded secrets or credentials
- Error propagation (not silent swallowing)
- Match arm exhaustiveness
- Serialization round-trip completeness
- Function length (under 50 lines)

Record findings with severity Warning or Info.

#### 2e. Universal checks pass

Read `../../references/universal-review-checks.md` (shared across all Vectis
reviewer skills).

Apply checks UNI-001 through UNI-017 with Rust-specific detection. Several
universal checks overlap with platform-specific checks already applied in
earlier passes. Skip those and focus on the gaps:

| Universal check | Already covered by | Action |
|---|---|---|
| UNI-002 Unvalidated input | CRX-002, LOG-007 | Skip |
| UNI-003 Serialization failures | CRX-005, GEN-009 | Skip |
| UNI-004 Logic bugs | LOG-001..008 | Skip |
| UNI-006 Race conditions | LOG-003, LOG-006 | Skip |
| UNI-010 Panics/crashes | GEN-001, CRX-011 | Skip |
| UNI-017 Type safety (partial) | CRX-008 | Apply beyond ViewModel |

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

Record findings with the severity defined in the universal checklist. Tag
findings that have a **Spec-change indicator** (UNI-002, UNI-004, UNI-007,
UNI-008, UNI-011, UNI-012, UNI-014) for inclusion in the spec-change
output in step 3.

#### 2f. Comparative review (first iteration only; if reference-dir provided)

Compare structural decisions between the target and reference apps:

- Event variant signatures (do they carry timestamps/IDs from the shell?)
- PendingOp variant structure (do they carry enough data for conflict resolution?)
- ViewModel field types (typed vs pre-formatted)
- Test coverage breadth (count and categorize tests in both)

Flag significant divergences as Warning with a note explaining what the
reference app does differently and why.

#### 2g. Produce iteration report

Output the findings for this iteration. On the first iteration, use the
full report format. On subsequent iterations, report only new findings
discovered in re-review and note the iteration number.

```
## Code Review Report: {app-name} (iteration {N})

### Summary
- Critical: N findings
- Warning: N findings
- Info: N findings

### Critical Findings

#### [CRX-001] Missing render() after page transition
- **File**: shared/src/app.rs, lines 384-388
- **Issue**: Navigating from Error to Loading mutates `model.page` without
  emitting `render()`. The shell may not see the Loading state.
- **Fix**: Wrap the return in `render().and(Command::event(Event::Initialize))`

... (one block per finding, ordered by severity then file)

### Warning Findings
...

### Info Findings
...

### Test Gap Summary
- Missing test for: [scenario description]
- Missing test for: ...
```

Classify each finding as **mechanical** (auto-fixable) or **design-level**
(requires architectural decisions). Add design-level findings to the
accumulated list.

#### 2h. Auto-fix mechanical issues

Apply fixes for findings that are mechanical:

- Adding missing `Serialize`/`Deserialize` derives
- Wrapping returns in `render().and(...)`
- Adding `.trim()` and empty checks on text inputs
- Removing unused dependencies from `Cargo.toml`

Do NOT auto-fix logic bugs (LOG-001 through LOG-008) without explicit
confirmation -- these require design decisions.

After any fixes, re-run `cargo check`, `cargo test`, and `cargo clippy` to
verify the fixes compile and pass.

#### 2i. Loop control

After applying fixes and verifying:

1. If **no mechanical fixes** were applied in this iteration, exit the cycle.
2. If `iteration >= max_iterations`, exit the cycle.
3. Otherwise, increment `iteration` and return to step 2a.

When the cycle exits, output a summary across all iterations:

```
### Review Cycle Summary
- Iteration 1: Fixed N mechanical issues (CRX-001 x3, CRX-002, CRX-005).
  M design-level findings deferred.
- Iteration 2: Fixed K regressions (GEN-005 from iteration 1 fix).
  No new design-level findings.
- Total: N+K mechanical fixes applied. M design-level findings accumulated.
```

### 3. Express accumulated design-level findings as a Specify change

After the review-fix cycle completes, check whether any **design-level
findings** were accumulated -- findings that require architectural decisions,
data-type changes, event-signature modifications, or logic rewrites
(typically CRX-003, CRX-004, CRX-006, CRX-007, LOG-001 through LOG-008,
and universal checks tagged with a **Spec-change indicator**).
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
   review-{app-name}-{YYYY-MM-DDTHH-MM}
   ```

   Example: `review-my-crux-app-2026-03-11T14-30`

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
     section identifies affected files, shell contract changes, and
     migration concerns.

   - **design.md**: Each design-level finding becomes a Decision section
     with rationale and alternatives considered. Group related findings
     (e.g., all timestamp-related changes under one decision). Reference
     the specific check IDs (CRX-xxx, LOG-xxx, UNI-xxx) that motivated
     each decision. For spec-change findings, explain why the current spec
     is insufficient and what the proposed requirement should be.

   - **specs/**: Create one spec file per logical area (e.g., `sync-logic`,
     `input-validation`, `resilience`). Each requirement maps to a review
     finding. Spec-change findings become new requirements with explicit
     acceptance criteria. Code-fix findings become scenarios under existing
     requirements. Scenarios should be derived from the simulation traces
     performed during the logic pass (LOG-001 through LOG-008) and from
     the spec-change indicators in the universal checks. Use WHEN/THEN
     format.

   - **tasks.md**: Order tasks by dependency -- spec updates first (so
     requirements are clear before implementation), then data-type changes,
     then event signatures, then handler logic, then test updates, then
     new tests, then verification. Each task references the finding ID it
     addresses. Include a final verification section that re-runs the
     core-reviewer skill to confirm all Critical findings are resolved.

4. **Show final status** using `/spec:status` and summarize: change name,
   location, artifacts created, and prompt the user with "Run `/spec:build`
   or ask me to implement to start working on the tasks."

## Severity Definitions

| Severity | Meaning | Action |
|---|---|---|
| **Critical** | Data loss, incorrect server calls, conflict-resolution failure, panic in production | Must fix before merge |
| **Warning** | Stale UI, missing tests, suboptimal types, unnecessary clones | Should fix; acceptable to defer with justification |
| **Info** | Style, documentation, minor improvements | Fix if convenient |

## Integration with Specify Workflow

This skill is invoked as part of the Vectis build phase, after core-writer
generation and compiler verification, before merge:

```
define -> build (core-writer) -> verify -> review-fix cycle (this skill, up to 3 iterations) -> generate change for design issues -> merge
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
