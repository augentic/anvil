# Spec-to-Test Mapping

How Specify spec scenarios map to test functions. This mapping is deterministic -- the same spec always produces the same test structure.

## Mapping Rules

### Spec File to Test File

Each spec file maps to a primary test file:

```text
specs/worksite/spec.md  →  tests/worksite.rs
specs/order/spec.md     →  tests/order.rs
```

Naming convention: snake_case of the spec directory name.

### Scenario to Test Function

Each scenario under a requirement maps to one test function. The requirement's stable `ID: REQ-XXX` line is the traceability key:

```text
#### Scenario: Successful worksite retrieval
  →  #[tokio::test] async fn test_worksite_successful_retrieval()

#### Scenario: Worksite not found
  →  #[tokio::test] async fn test_worksite_not_found()
```

Naming convention: `test_<spec_name_snake>_<scenario_snake>`.

### WHEN Clause to Test Setup

The WHEN clause determines test input construction:

| WHEN Pattern | Test Setup |
| --- | --- |
| WHEN user sends valid request with field X = Y | `let request = Handler { x: "Y".to_string(), .. };` |
| WHEN request is missing required field | `let request = Handler { field: "".to_string(), .. };` |
| WHEN external API returns error | Configure MockProvider to return error for that path |
| WHEN message arrives on topic T | `let message = build_message(/* topic T payload */);` |

### THEN Clause to Assertions

The THEN clause determines test assertions:

| THEN Pattern | Assertion |
| --- | --- |
| THEN system returns HTTP 200 with data | `assert_eq!(response.status, 200);` + body field assertions |
| THEN system returns error CODE | `let err = client.request(req).await.expect_err("...");` + `assert_eq!(err.code(), "CODE");` |
| THEN system publishes event to topic T | `let events = provider.events();` + topic and payload assertions |
| THEN system caches result for N seconds | Assert StateStore was called with expected TTL |
| THEN system calls external API at path P | `let calls = provider.requests_for("P");` + `assert_eq!(calls.len(), 1);` |

## Requirement Coverage

### Requirements with Multiple Scenarios

Each scenario becomes its own test. A requirement with 3 scenarios produces 3 test functions:

```markdown
### Requirement: Worksite data retrieval
ID: REQ-001
#### Scenario: Successful retrieval
#### Scenario: Worksite not found
#### Scenario: External API timeout
```

Produces:

```rust
#[tokio::test] async fn test_worksite_successful_retrieval() { ... }
#[tokio::test] async fn test_worksite_not_found() { ... }
#[tokio::test] async fn test_worksite_external_api_timeout() { ... }
```

### Validation Requirements

Validation requirements in specs often produce tests for `from_input()`:

```markdown
### Requirement: Input validation
ID: REQ-002
#### Scenario: Missing worksite code
- WHEN request has empty worksite_code
- THEN system returns BadRequest with code "missing_worksite_code"
```

Produces a test that constructs invalid input and asserts the error:

```rust
#[tokio::test]
async fn test_worksite_missing_worksite_code() {
    let provider = MockProvider::new();
    let client = Client::new("owner").provider(provider.clone());

    let request = GetWorksiteRequest { worksite_code: "".to_string(), .. };
    let error = client.request(request).await.expect_err("should reject empty code");
    assert_eq!(error.code(), "missing_worksite_code");
}
```

## Traceability

Each generated test should include a traceability comment linking back to the spec with the stable requirement ID:

```rust
/// Spec: specs/fleet-api/spec.md > Requirement ID: REQ-001 > Requirement: Worksite data retrieval > Scenario: Successful retrieval
#[tokio::test]
async fn test_fleet_api_successful_retrieval() { ... }
```

This enables automated drift detection: parse test comments to find the source scenario and requirement ID, then verify the requirement and scenario still exist in the spec with matching WHEN/THEN clauses.

## Drift Detection Mechanics

### Detecting Missing Tests

1. Parse all requirement blocks from the spec, including each `### Requirement:`, `ID: REQ-XXX`, and `#### Scenario:` entry
2. Parse all `#[tokio::test]` function names from `tests/*.rs`
3. For each scenario, check if a corresponding test function exists
4. Report scenarios without tests as **missing coverage**

### Detecting Extra Tests

1. Parse all test functions with traceability comments
2. Check if the referenced requirement ID and scenario still exist in the spec
3. Report tests referencing removed scenarios as **stale tests**

Tests without traceability comments are treated as manually added and are not flagged.

### Detecting Assertion Drift

1. Parse THEN clauses from the spec scenario
2. Parse assertions from the test function
3. Compare expected values (status codes, error codes, field values)
4. Report mismatches as **assertion drift**

This comparison is approximate -- it catches obvious divergences (wrong status code, wrong error code) but may not detect subtle logic changes.
