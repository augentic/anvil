---
name: omnia-test-writer
description: "Generate or update test suites for Omnia Rust WASM crates from Specify artifacts -- MockProvider setup, integration tests, spec-to-test mapping, and drift detection. Use when an Omnia slice has pending crate-test tasks, or when an existing test suite needs to be regenerated after a crate update; not for the crate itself (`crate-writer`) or guest wiring (`guest-writer`)."
argument-hint: "[crate-name]"
---

# Test Writer

## Critical Path

1. **Load artifacts and references** — read `spec.md`, `design.md`, `mock-provider.md`, `spec-to-test-mapping.md`, and the closest example before generating tests.
2. **Inventory crate and tests** — inspect handlers, provider trait bounds, input/output types, existing `tests/`, fixtures, and assertion style.
3. **Map specs to tests** — create one deterministic test per scenario, trace each to the stable `REQ-XXX` ID, and derive validation/error/happy-path coverage from specs.
4. **Assert side effects from design** — enumerate every provider interaction in design.md and generate assertions for publishes, writes, cache changes, transactions, and rollback behavior.
5. **Generate MockProvider and fixtures** — implement only required provider traits, load JSON fixtures from `tests/data/`, and preserve existing test style.
6. **Handle drift without deletion** — report missing, extra, and assertion-drift cases; update tests to match changed specs while preserving manual tests unless clearly obsolete.
7. **Leave execution to orchestration** — verify structural checklist here; compilation and test execution happen in the build verify-repair loop.

Generate or update test suites for Omnia Rust WASM crates from Specify artifacts (specs + design.md) and existing crate code. Tests use `MockProvider` implementations and the `Client` typestate builder to invoke handlers.

**Relationship to other skills**:

- **crate-writer** generates code only (no tests). test-writer owns all test generation -- MockProvider, integration tests, spec-to-test traceability, and test updates.
- **replay-writer** adds regression tests from captured real-world fixtures. test-writer generates synthetic tests from spec scenarios.
- The **build orchestration layer** runs a unified verify-repair loop after both crate-writer and test-writer complete. test-writer generates tests but does not run them; compilation and test verification happen at the orchestration level.

## Arguments

```text
$CRATE_NAME     = $ARGUMENTS[0]

# Path derivation
$CRATE_PATH     = crates/$CRATE_NAME
$SLICE_DIR     = .specify/slices/$CRATE_NAME
$SPECS_DIR      = $SLICE_DIR/specs
$DESIGN_PATH    = $SLICE_DIR/design.md
```

## Required References

Before generating tests, read these documents:

1. [mock-provider.md](references/mock-provider.md) -- Static and Replay MockProvider patterns
2. [spec-to-test-mapping.md](references/spec-to-test-mapping.md) -- How spec scenarios map to test functions

### Examples

Read at least one matching your scenario:

- [testing.md](examples/testing.md) -- Core test patterns: layout, MockProvider, test structures, fixtures
- [testing-http.md](examples/testing-http.md) -- Simple HTTP handler testing with Config-only MockProvider
- [testing-statestore.md](examples/testing-statestore.md) -- Multi-trait MockProvider with StateStore and cache-aside
- [testing-publisher.md](examples/testing-publisher.md) -- Publish, event capture, request-reply, topic checks

## Authority Hierarchy

When conflicts arise, follow this strict precedence:

1. **This SKILL.md** -- test generation rules
2. **Specify artifacts (specs + design.md)** -- behavioral requirements that tests must verify
3. **references/** -- MockProvider and mapping patterns
4. **examples/** -- canonical test code patterns
5. **Existing crate code** -- handler signatures, provider bounds, type definitions
6. **Existing tests** -- style and conventions to follow

## Test Generation Process

### Step 1: Read Crate and Artifacts

1. Read the spec file from `$SPECS_DIR/$CRATE_NAME/spec.md` (consolidated file with flat `### Requirement:` / `ID: REQ-XXX` / `#### Scenario:` blocks)
2. Read design.md from `$DESIGN_PATH`
3. Read existing crate code from `$CRATE_PATH/src/` to identify:
   - Handler implementations and their provider trait bounds
   - Input/output types and serde attributes
   - Domain error variants
   - Validation logic (structural in `from_input()`, temporal in `handle()`)

### Step 2: Inventory Existing Tests

If `$CRATE_PATH/tests/` exists, parse it to understand the current test state:

| Source | What to Extract |
| --- | --- |
| `tests/provider.rs` | MockProvider: which traits implemented, config keys, HTTP fixtures |
| `tests/*.rs` | Test names, handlers covered, assertion patterns, fixture usage |
| `tests/data/` | Existing fixture files |

### Step 3: Map Spec Scenarios to Tests

For the spec file at `$SPECS_DIR/$CRATE_NAME/spec.md`, and for each requirement block (`### Requirement:` plus its `ID: REQ-XXX` line) and each `#### Scenario:` within it:

1. **One test function per scenario** -- deterministic naming: `test_<crate>_<scenario_snake_case>`
2. **Happy path tests** from success scenarios (WHEN/THEN with expected output)
3. **Error case tests** from error scenarios (WHEN/THEN with expected error code)
4. **Validation tests** from requirement constraints (field presence, format, range)
5. **Traceability comments** should cite the stable requirement ID so renaming a requirement title does not orphan the test
6. **Side-effect assertions from design.md** -- for each scenario, read the corresponding handler's Business Logic section in design.md and enumerate every provider interaction tagged `[infrastructure]` or `[domain]` that produces an observable side effect:
   - **Messaging publishes** (`Publish::send`) -- assert `MockProvider` captured a publish to the expected topic with the expected payload shape. If the design.md specifies payload transformations (field stripping, additions, renames), assert the transformed shape, not the raw entity.
   - **Database writes** (`TableStore::exec`) -- assert the expected write operations were executed with expected parameter values
   - **Cache writes** (`StateStore::set` / `StateStore::delete`) -- assert cache keys were set or invalidated as specified
   - **Cross-entity mutations** -- if the handler's Business Logic specifies mutating a related entity (e.g., incrementing a counter on a parent record after inserting a child), assert that the secondary write occurred
   - **Transaction boundaries** -- if the handler wraps operations in a transaction, assert that a failed step triggers rollback and that messaging publishes do NOT occur on rollback

   **The spec and design.md are ground truth, not the generated code.** Generate side-effect assertions for every specified interaction regardless of whether the current handler code appears to implement it. If a handler is missing an implementation, the test SHOULD fail -- the verify-repair loop will route the failure back to crate-writer.

See [spec-to-test-mapping.md](references/spec-to-test-mapping.md) for the detailed mapping rules.

### Step 4: Generate MockProvider

Generate `tests/provider.rs` implementing all provider traits the handlers require:

- **Config**: Return test values for each `Config::get` key in the crate; error for unknown keys
- **HttpRequest**: Dispatch on `request.uri().path()` to return fixture data; record requests for assertion
- **Publish**: Capture events via `Arc<Mutex<Vec<T>>>` for assertion
- **Identity**: Return mock tokens
- **StateStore**: In-memory `HashMap` behind `Mutex` with get/set/delete
- **TableStore**: Return fixture rows from `query`, affected count from `exec`
- **Broadcast**: Capture sends with channel and target info
- **Blobstore**: In-memory nested `HashMap` (container -> name -> bytes) with get/write/delete/list
- **DocumentStore**: In-memory nested `HashMap` (store -> id -> Document) with get/insert/put/delete/query

See [mock-provider.md](references/mock-provider.md) for complete patterns (Static and Replay variants).

### Step 5: Generate Test Files

Generate the primary spec-driven test file at `tests/<crate_name>.rs`:

```rust
mod provider;

use <crate_name>::<HandlerRequest>;
use omnia_sdk::api::Client;
use provider::MockProvider;

#[tokio::test]
async fn test_<crate>_happy_path() {
    let provider = MockProvider::new();
    let client = Client::new("owner").provider(provider.clone());

    let request = <HandlerRequest> { /* fields from scenario */ };
    let response = client.request(request).await.expect("should succeed");

    assert_eq!(response.status, 200);
    // assert on response.body fields per scenario THEN clause
}

#[tokio::test]
async fn test_<crate>_<error_scenario>() {
    let provider = MockProvider::new();
    let client = Client::new("owner").provider(provider.clone());

    let request = <HandlerRequest> { /* fields triggering error */ };
    let error = client.request(request).await.expect_err("should fail");

    assert_eq!(error.code(), "<expected_code>");
}
```

### Step 6: Generate Fixture Data

For tests that require mock HTTP responses or complex input data:

- Store JSON fixtures in `tests/data/` (e.g., `tests/data/worksite-search.json`)
- Reference in MockProvider with `include_bytes!("data/<fixture>.json")`
- Derive fixture content from design.md API response shapes and example data

## Test Conventions

1. **Each test file** starts with `mod provider;`
2. **Create provider** with `MockProvider::new()`
3. **Create client** with `Client::new("owner").provider(provider.clone())`
4. **Invoke handler** with `client.request(request).await`
5. **Assert on response**: `response.status`, `response.body`
6. **Assert on side effects**: `provider.events()`, `provider.requests_for(path)`
7. **Error testing**: `.expect_err("message")` then assert `error.code()` and `error.description()`
8. **Async runtime**: `#[tokio::test]`
9. **tokio in dev-dependencies only**: `tokio = { version = "1", features = ["macros", "rt"] }`

## Test Directory Structure

```text
$CRATE_PATH/
├── tests/
│   ├── provider.rs         # MockProvider (shared across tests)
│   ├── <handler_a>.rs      # Tests for handler A
│   └── <handler_b>.rs      # Tests for handler B
└── tests/data/             # JSON/XML fixture files (optional)
    ├── response-a.json
    └── response-b.json
```

## Spec-to-Test Mapping (Forward-Looking)

The long-term goal is deterministic, repeatable spec-to-test compilation:

- **Each BDD scenario in spec.md maps to exactly one test function**. The mapping is deterministic -- the same spec always produces the same test structure.
- **Spec drift detection**: Regenerate tests from baseline specs at `.specify/specs/$CRATE_NAME/spec.md` and compare against existing tests. Differences indicate either spec drift (spec changed without updating tests) or code drift (code changed without updating spec).
- **CI integration**: A future CI step can regenerate tests from specs, diff against committed tests, and fail the build if they diverge. This closes the loop: specs produce tests, tests validate code, and drift is caught automatically.

See [spec-to-test-mapping.md](references/spec-to-test-mapping.md) for the mapping rules that enable this.

## Drift Detection (Forward-Looking)

When invoked against a crate with existing tests and baseline specs:

1. **Regenerate** the expected test structure from `.specify/specs/$CRATE_NAME/spec.md`
2. **Compare** against existing tests in `$CRATE_PATH/tests/`
3. **Report** divergences:
   - **Missing tests**: Spec scenarios with no corresponding test function
   - **Extra tests**: Test functions with no corresponding spec scenario (may be manual additions -- flag, don't remove)
   - **Assertion drift**: Test assertions that don't match spec THEN clauses
4. **Surface** as either spec drift or code drift for human review

This enables the spec-as-contract model: specs have teeth because tests enforce them, and drift is visible.

## Verification Checklist

Before completing, verify ALL structural items. Compilation and test execution are verified at the orchestration level after test-writer completes.

- [ ] `tests/provider.rs` with MockProvider implementing all required traits
- [ ] At least one happy-path test for the spec file
- [ ] Error case tests for validation failures documented in specs
- [ ] Tests use `Client::new("owner").provider(mock)` pattern
- [ ] Test fixtures in `tests/data/` or inline
- [ ] No `unwrap()` or `expect()` in production code (allowed in tests)
- [ ] Each spec scenario has a corresponding test function (when specs are available)
- [ ] Side-effect assertions for every `[infrastructure]` provider interaction in design.md Business Logic (messaging publishes, DB writes, cache mutations, cross-entity mutations)
- [ ] Transaction rollback tests for handlers with atomic write sequences

## Related Skills

- **crate-writer** -- generates crate code only; test-writer owns all test generation
- **replay-writer** -- adds regression tests from captured real-world fixtures (complementary; test-writer generates from specs, replay-writer generates from production data)
- **code-reviewer** -- reviews generated code including test quality
