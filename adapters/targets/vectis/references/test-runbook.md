# Test Writer Runbook

Operational detail for `vectis-test-writer`. The SKILL.md keeps only the orientation surface (Critical Path + Reference table + Guardrails); everything procedural lives here.

## Arguments

```text
$DOMAIN_NAME    = $ARGUMENTS[0]

# Path derivation
$SLICE_DIR      = .specify/slices/<active-change>
$SPECS_DIR      = $SLICE_DIR/specs
$SPEC_PATH      = $SPECS_DIR/$DOMAIN_NAME/spec.md
$DESIGN_PATH    = $SLICE_DIR/design.md
$PROJECT_DIR    = <project directory>
$APP_RS         = $PROJECT_DIR/shared/src/app.rs
```

## Required References

Before generating tests, read these documents:

1. [`spec-to-test-mapping.md`](test-spec-mapping.md) — how spec scenarios map to test functions, traceability format, and drift detection
2. [`crux-testing-patterns.md`](crux/testing-patterns.md) — Crux test API: `update()`, `Command`, effect assertions, resolving effects

## Authority Hierarchy

When conflicts arise, follow this strict precedence:

1. **The SKILL.md** — test generation rules and structure
2. **Specify artifacts (specs + design.md)** — behavioral requirements that tests must verify
3. **`references/`** — mapping rules and testing patterns
4. **Existing crate code** — Event/Model/ViewModel types, `update()` logic, Command/effect patterns
5. **Existing tests** — style and conventions to preserve

## Mode Detection

Check whether `$APP_RS` contains a `#[cfg(test)]` module with spec traceability comments (`/// Spec:` lines referencing `REQ-` IDs):

- If **no** `#[cfg(test)]` module exists, or it exists but has **no** traceability comments, use **create mode**.
- If the test module **has** traceability comments, use **update mode**.

### Repair mode

This skill may be invoked as a **repair sub-agent** from the verify-repair loop. In repair mode the skill receives:

- `mode: repair` (not `create` or `update`)
- The full compiler or test error output
- The repair discipline constraints (minimum change, scoped diff)
- Paths to Crux API references when the errors involve API-surface mismatches (provided via `extra_context`)

When invoked in repair mode:

1. Run `cargo test` in `$PROJECT_DIR` to get the actual compiler and test errors. The passed-in error output is a starting point, but a fresh run captures the current state after any prior partial fixes in the same verify-repair iteration.
2. Read the `#[cfg(test)]` module in `app.rs` and any files referenced in the fresh `cargo test` output from step 1.
3. Read [`crux-testing-patterns.md`](crux/testing-patterns.md) and [`crux-command-api.md`](crux/command-api.md) to identify the correct Crux 0.17 API surface for the failing code. These references are the canonical source for method signatures, effect assertion patterns (`expect_one_effect()`, `expect_http()`, `resolve()`), HTTP/KV/SSE builder APIs, and `Command` combinators.
4. Diagnose the root cause from the fresh `cargo test` output. Common API-surface mismatches include: wrong method name on `Command`, incorrect effect assertion chain, stale `HttpRequest`/`HttpResponse` builder pattern, wrong `resolve()` argument shape, missing or incorrect imports.
5. Apply the minimum change to fix the reported errors — update test assertions, fix factory functions, correct imports, adjust API calls to match the patterns in the references. **Preserve test logic and spec traceability**: do not change test names, `/// Spec:` traceability comments, or the intent of assertions (what is being checked). Only the syntax used to express them should change.
6. Run `cargo test` again to verify the fix. If errors remain, report the remaining failures in the sub-agent output rather than returning a broken state. Do not loop internally — the outer verify-repair loop owns iteration control.
7. Return the list of files modified, the fix applied, and the verification result (pass or remaining errors).

## Test Generation Process: Create Mode

### Step 1: Read Crate and Artifacts

1. Read the spec file from `$SPEC_PATH`. Extract all requirement blocks: each `### Requirement:` with its `ID: REQ-XXX` line and each `#### Scenario:` within it. Read only the **core requirements** (main body). Platform-specific sections (`## iOS Shell Requirements`, etc.) are not relevant to core tests.
2. Read `$DESIGN_PATH` for domain model context, API contracts, and capability details.
3. Read `$APP_RS` to identify:
   - Event variants (shell-facing and internal)
   - Model structure and Page enum
   - ViewModel variants and per-page view structs
   - Effect variants and capability type aliases
   - `update()` match arms and their logic
   - `view()` function mapping
   - Helper functions and validation logic
   - Supporting domain types

### Step 2: Map Spec Scenarios to Tests

For each requirement block and each scenario within it, generate one test function following the deterministic mapping rules in [`spec-to-test-mapping.md`](test-spec-mapping.md):

1. **One test function per scenario** — naming: `test_<unit_snake>_<scenario_snake_case>` where `<unit_snake>` is the spec folder name converted to snake_case (e.g., `weather-forecast` becomes `weather_forecast`).
2. **Happy path tests** from success scenarios (WHEN/THEN with expected state and view model).
3. **Error case tests** from error scenarios (WHEN/THEN with expected error state or error view).
4. **Validation tests** from requirement constraints (field presence, format, range).
5. **Effect chain tests** from scenarios that involve async operations (HTTP, KV, SSE) — test the full resolve chain.
6. **Page transition tests** from scenarios describing navigation or loading sequences.
7. **Traceability comments** on each test citing the stable requirement ID:

```rust
/// Spec: specs/<domain>/spec.md > REQ-XXX > Scenario: <scenario title>
#[test]
fn test_<unit_snake>_<scenario_snake_case>() {
    // ...
}
```

### Step 3: Generate Test Module

Generate or replace the `#[cfg(test)] mod tests` block at the bottom of `$APP_RS`. The test module follows Crux conventions:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crux_core::App as _;

    // --- Test helpers ---

    fn app() -> MyApp {
        MyApp
    }

    // Factory functions for domain types used across tests
    fn make_item() -> Item {
        Item {
            id: "test-1".to_string(),
            title: "Test Item".to_string(),
            // ...fields with sensible defaults
        }
    }

    // --- Spec-mapped tests ---

    /// Spec: specs/<domain>/spec.md > REQ-001 > Scenario: <title>
    #[test]
    fn test_<unit_snake>_<scenario_snake_case>() {
        let app = app();
        let mut model = Model::default();

        let mut cmd = app.update(Event::SomeAction, &mut model);

        // Assert on model state per THEN clause
        // Assert on effects per expected behavior
        // Assert on view model if scenario describes UI output
    }

    // ...one test per scenario
}
```

### Step 4: Write Effect Chain Tests

For scenarios involving async operations (HTTP requests, KV operations, SSE streams), test the full effect-resolve chain:

1. Send the triggering Event via `app.update()`
2. Extract the expected effect from the Command
3. Resolve the effect with a simulated response
4. Feed the resulting Event back into `app.update()`
5. Assert on final model state, effects, and view model

Follow [`crux-testing-patterns.md`](crux/testing-patterns.md) for the effect resolution API (`expect_one_effect()`, `expect_http()`, `resolve()`, `expect_one_event()`).

### Step 5: Ensure Minimum Coverage

After mapping all spec scenarios, verify these coverage minimums:

- At least one test per **shell-facing Event variant** (these are the user actions the spec describes)
- At least one test per **page transition** described in the spec (Loading to content, content to error, error recovery)
- At least one test for each **validation rule** in the spec (empty input, invalid format, out of range)
- At least one **happy path** and one **error path** for each capability interaction (HTTP success/failure, KV hit/miss)

If a spec scenario does not map to a shell-facing Event (e.g., it describes internal behavior), the test should construct the appropriate internal Event directly.

### Step 6: Verify Structure

Before completing, verify all structural items. Do NOT run `cargo test` in create or update mode — that happens at the orchestration level. (In repair mode, `cargo test` is run by the repair process itself; see Repair mode above.)

- [ ] All tests are inside `#[cfg(test)] mod tests` in `app.rs`
- [ ] Every spec scenario has a corresponding test function
- [ ] Every test has a `/// Spec:` traceability comment with `REQ-XXX` ID
- [ ] Test naming follows `test_<unit_snake>_<scenario_snake_case>` convention
- [ ] Tests use synchronous `#[test]` (not `#[tokio::test]`)
- [ ] Tests import `crux_core::App as _` for `update()` and `view()` access
- [ ] Effect chain tests resolve effects and feed events back into `update()`
- [ ] Factory functions extract repeated type construction
- [ ] No `unwrap()` or `expect()` in production code (allowed in tests)

## Test Generation Process: Update Mode

Use this process when the test module already has traceability comments and the spec has changed.

### U1. Read Artifacts and Code

Same as create mode Step 1. Read the spec, design, and crate code.

### U2. Inventory Existing Tests

Parse the `#[cfg(test)] mod tests` module in `$APP_RS`:

| Source | What to Extract |
|---|---|
| `/// Spec:` comments | Requirement ID (`REQ-XXX`) and scenario title for each test |
| Test function names | Current naming, which scenarios are covered |
| Test bodies | Assertion patterns, factory functions, effect chains |
| Helper functions | `make_*` factories, setup helpers, utility functions |

Build a map of `REQ-XXX + scenario title` → test function name.

### U3. Diff Spec Against Tests

Compare the current spec scenarios against the test inventory:

| Category | Condition | Action |
|---|---|---|
| **Added** | Scenario in spec, no matching test | Generate new test |
| **Removed** | Test references scenario not in spec | Mark as stale (see below) |
| **Modified** | Scenario WHEN/THEN changed | Update test assertions |
| **Renamed** | Scenario title changed but REQ-ID matches | Update test name and traceability comment |
| **Unchanged** | Scenario and test match | Leave alone |

### U4. Apply Changes

1. **Add tests** for new scenarios, following the same patterns and helpers used in existing tests.
2. **Update tests** for modified scenarios: change assertions to match new THEN clauses, update setup to match new WHEN clauses, update effect chains if capabilities changed.
3. **Handle stale tests**: tests referencing removed scenarios should be flagged with a `// STALE: scenario removed from spec` comment rather than silently deleted. The human decides whether the test covers behavior that moved to a different requirement or is truly obsolete.
4. **Update factory functions** if domain types gained or lost fields.
5. **Preserve test utilities** (helpers, constants) unless the types they construct changed.

### U5. Verify Structure

Same checklist as create mode Step 6, plus:

- [ ] New tests follow existing naming and style conventions
- [ ] Stale tests are flagged, not silently removed
- [ ] Factory functions updated for type changes
- [ ] No orphaned imports or dead helper functions

## Spec-to-Test Mapping

The mapping from spec scenarios to tests is deterministic — the same spec always produces the same test structure. Each BDD scenario maps to exactly one test function. The requirement identity is the stable `REQ-XXX` ID (not the requirement display name, which may change). Within a requirement, each scenario is matched by its title — a requirement with 3 scenarios produces 3 tests, each traced to `REQ-XXX` + its scenario title.

See [`spec-to-test-mapping.md`](test-spec-mapping.md) for the full mapping rules, including WHEN-to-setup and THEN-to-assertion translation.

## Drift Detection

When invoked against a crate with existing tests and baseline specs at `.specify/specs/<domain>/spec.md`:

1. **Regenerate** the expected test structure from the baseline spec
2. **Compare** against existing tests in the `#[cfg(test)]` module
3. **Report** divergences:
   - **Missing tests**: spec scenarios with no corresponding test (by REQ-ID + scenario title)
   - **Stale tests**: tests with traceability comments referencing scenarios that no longer exist in the spec
   - **Assertion drift**: test assertions that don't match spec THEN clauses (approximate — catches obvious divergences like wrong status codes or missing fields)
4. **Surface** as either spec drift (spec changed, tests not updated) or code drift (code changed, spec not updated) for human review

This enables the spec-as-contract model: specs have teeth because tests enforce them, and drift is visible.

**CI integration (future)**: A CI step can regenerate the expected test structure from baseline specs, diff against committed tests, and fail the build if they diverge. This closes the loop between specs, tests, and code.

## Test Conventions

1. **All tests** live inside `#[cfg(test)] mod tests` in `app.rs` (Crux convention — not a separate `tests/` directory)
2. **Synchronous tests** using `#[test]` (Crux's testing model is fully synchronous; no async runtime needed)
3. **Import** `crux_core::App as _` for `update()` and `view()` method access
4. **Create app** with `let app = MyApp;` (or a helper `fn app()`)
5. **Create model** with `let mut model = Model::default();` or a seeded state for specific scenarios
6. **Send events** with `let mut cmd = app.update(event, &mut model);`
7. **Assert effects** with `cmd.expect_one_effect().expect_render()`, `cmd.expect_one_effect().expect_http()`, etc.
8. **Resolve effects** with `.resolve(response)` and feed resulting events back into `app.update()`
9. **Assert view model** with `let view = app.view(&model);` then field assertions
10. **Traceability** via `/// Spec:` doc comments on every spec-mapped test
11. **Factory functions** for domain types to reduce repetition
12. **No `unwrap()`/`expect()` in production code** (allowed in tests for clarity)

## Verification Checklist

Before completing, verify ALL structural items. In create and update modes, compilation and test execution are verified at the orchestration level after test-writer completes. In repair mode, test-writer runs `cargo test` itself as part of the fix-and-verify cycle.

### Coverage

- [ ] Every `#### Scenario:` in the spec has a corresponding test function
- [ ] Every test has a `/// Spec:` traceability comment with `REQ-XXX` ID
- [ ] At least one test per shell-facing Event variant
- [ ] Page transition scenarios have tests (Loading → content, error recovery)
- [ ] Validation rules from spec have corresponding tests
- [ ] Capability interactions have happy-path and error-path tests

### Structure

- [ ] Tests are inside `#[cfg(test)] mod tests` in `app.rs`
- [ ] Test naming follows `test_<unit_snake>_<scenario_snake_case>`
- [ ] Tests use `#[test]` (synchronous, no async runtime)
- [ ] `crux_core::App as _` imported in test module
- [ ] Factory functions extract repeated type construction
- [ ] Effect chains resolve effects and feed events back

### Quality

- [ ] No `unwrap()` or `expect()` in production code
- [ ] Test assertions match spec THEN clauses
- [ ] Stale tests flagged (update mode), not silently removed

## Related Skills

- **core-writer** — generates crate production code only; test-writer owns all test generation and spec-to-test traceability
- **core-reviewer** — reviews generated code and validates spec-to-test coverage using traceability comments (LOG-008, LOG-009)

## Troubleshooting

### Tests won't compile after core-writer changes

The verify-repair loop at the orchestration level classifies failures and routes them to the correct skill. If the failure is in the test module (type mismatch, missing field on a constructed type), test-writer is re-entered in repair mode with the error output and Crux API references. The repair sub-agent runs `cargo test` to get fresh errors, reads the testing pattern references for correct API surface, applies the minimum fix, and verifies the fix before returning.

### Tests use wrong Crux API surface

When test code uses method names or patterns that don't exist in Crux 0.17 (e.g., wrong `Command` methods, incorrect effect assertion chains, stale builder patterns), the repair sub-agent reads [`crux-testing-patterns.md`](crux/testing-patterns.md) and [`crux-command-api.md`](crux/command-api.md) to identify the correct API and adjusts the test syntax. Test logic (what is being tested) and spec traceability (`/// Spec:` comments, test names) are preserved — only the API calls change.

### Scenario has no obvious Event mapping

Some spec scenarios describe behavior that is triggered by internal events (e.g., "When the HTTP response arrives..."). Map these to the internal Event variant that carries the response. If no Event variant exists, flag it — core-writer may need to add one.

### Spec uses vague THEN clauses

When the spec's THEN clause is not specific enough to derive assertions (e.g., "the user sees the updated list"), derive assertions from the ViewModel structure: assert on the fields of the per-page view struct that `view()` would populate from the model state described in the WHEN clause.

### Multiple scenarios share setup

Extract the shared setup into a helper function (e.g., `fn seeded_model()`) and call it from each test. Keep the assertions specific to each scenario.
