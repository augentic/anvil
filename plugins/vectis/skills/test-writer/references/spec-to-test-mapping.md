# Spec-to-Test Mapping for Crux

How Specify spec scenarios map to test functions in a Crux shared crate.
This mapping is deterministic -- the same spec always produces the same
test structure.

## Mapping Rules

### Spec File to Test Module

Each feature spec maps to tests inside the `#[cfg(test)] mod tests` block
in `shared/src/app.rs` (Crux convention -- tests live alongside the app,
not in a separate `tests/` directory):

```text
specs/<feature>/spec.md  →  #[cfg(test)] mod tests { ... } in app.rs
```

### Scenario to Test Function

Each scenario under a requirement maps to one test function. The
requirement's stable `ID: REQ-XXX` line is the traceability key:

```text
#### Scenario: Successful item fetch
  →  #[test] fn test_<feature_snake>_successful_item_fetch()

#### Scenario: Item not found
  →  #[test] fn test_<feature_snake>_item_not_found()
```

Naming convention: `test_<feature_snake>_<scenario_snake>` where
`<feature_snake>` is the spec folder name converted to snake_case
(replace `-` with `_`).

All tests are synchronous `#[test]` -- Crux's testing model does not
require an async runtime.

### Traceability Comments

Every spec-mapped test must have a doc comment linking it to the source
requirement and scenario using the stable `REQ-XXX` ID:

```rust
/// Spec: specs/<feature>/spec.md > REQ-001 > Scenario: Successful item fetch
#[test]
fn test_<feature_snake>_successful_item_fetch() { ... }
```

The REQ-ID is the traceability key. If a requirement title is renamed but
keeps the same ID, the test is still linked. If a scenario title changes,
update the comment but keep the REQ-ID reference.

### WHEN Clause to Test Setup

The WHEN clause determines how the test constructs initial model state and
which Event to send:

| WHEN Pattern | Test Setup |
|---|---|
| WHEN user triggers action X | `let mut cmd = app.update(Event::X, &mut model);` |
| WHEN user provides input with field Y = Z | Construct Event variant with payload: `Event::Submit(Input { y: "Z".into(), .. })` |
| WHEN input is missing required field | Construct Event with empty/invalid field value |
| WHEN app is on page P | Seed model: `model.page = Page::P;` before calling `update()` |
| WHEN HTTP response returns data | Resolve HTTP effect with simulated response, feed event back |
| WHEN HTTP request fails | Resolve HTTP effect with error response |
| WHEN KV contains key K with value V | Set up model or resolve KV get with `Some(value)` |
| WHEN KV key is missing | Resolve KV get with `None` |
| WHEN SSE stream delivers event | Resolve SSE effect with `SseResponse::Chunk(data)` |

### THEN Clause to Assertions

The THEN clause determines what the test asserts:

| THEN Pattern | Assertion |
|---|---|
| THEN app shows loading state | `assert!(matches!(model.page, Page::Loading));` or `assert!(matches!(app.view(&model), ViewModel::Loading));` |
| THEN app displays items | `let view = app.view(&model); assert_eq!(view.items.len(), N);` |
| THEN app shows error message M | `let view = app.view(&model);` then assert on error view fields |
| THEN app navigates to page P | `assert!(matches!(model.page, Page::P { .. }));` |
| THEN app sends HTTP request to URL | `let request = cmd.expect_one_effect().expect_http(); assert_eq!(&request.operation, &HttpRequest::get(URL).build());` |
| THEN app stores value under key K | `let kv = cmd.expect_one_effect().expect_key_value();` then assert operation |
| THEN app renders | `cmd.expect_one_effect().expect_render();` |
| THEN app renders and fetches data | `cmd.expect_effect().expect_render();` then `cmd.expect_one_effect().expect_http();` |
| THEN field F has value V | `assert_eq!(model.field, expected_value);` or `assert_eq!(view.field, expected_value);` |

### Effect Chain Mapping

Scenarios describing async operations map to multi-step tests that resolve
effects and feed events back:

```text
#### Scenario: Fetch items on load
- WHEN app starts
- THEN app shows loading and fetches items from /api/items
- AND WHEN items are returned
- THEN app shows the item list
```

Maps to:

```rust
/// Spec: specs/<feature>/spec.md > REQ-001 > Scenario: Fetch items on load
#[test]
fn test_<feature_snake>_fetch_items_on_load() {
    let app = MyApp;
    let mut model = Model::default();

    // Step 1: User triggers fetch
    let mut cmd = app.update(Event::FetchItems, &mut model);
    assert!(matches!(model.page, Page::Loading));

    // Step 2: Extract and resolve HTTP effect
    cmd.expect_effect().expect_render();
    let mut request = cmd.expect_one_effect().expect_http();
    assert_eq!(
        &request.operation,
        &HttpRequest::get("https://api.example.com/items").build()
    );

    request
        .resolve(HttpResult::Ok(
            HttpResponse::ok()
                .body(r#"[{"id":"1","title":"Item 1"}]"#)
                .build(),
        ))
        .unwrap();

    // Step 3: Feed response event back
    let event = cmd.expect_one_event();
    let mut cmd = app.update(event, &mut model);

    // Step 4: Assert final state per THEN clause
    cmd.expect_one_effect().expect_render();
    let view = app.view(&model);
    // assert on view fields per scenario
}
```

## Requirement Coverage

### Requirements with Multiple Scenarios

Each scenario becomes its own test. A requirement with 3 scenarios produces
3 test functions:

```markdown
### Requirement: Item management
ID: REQ-001
#### Scenario: Add new item
#### Scenario: Delete existing item
#### Scenario: Delete non-existent item
```

Produces:

```rust
/// Spec: specs/<feature>/spec.md > REQ-001 > Scenario: Add new item
#[test]
fn test_<feature_snake>_add_new_item() { ... }

/// Spec: specs/<feature>/spec.md > REQ-001 > Scenario: Delete existing item
#[test]
fn test_<feature_snake>_delete_existing_item() { ... }

/// Spec: specs/<feature>/spec.md > REQ-001 > Scenario: Delete non-existent item
#[test]
fn test_<feature_snake>_delete_non_existent_item() { ... }
```

### Validation Requirements

Validation requirements produce tests that send invalid input and assert on
the resulting model/view state:

```markdown
### Requirement: Input validation
ID: REQ-002
#### Scenario: Empty title rejected
- WHEN user submits item with empty title
- THEN app shows validation error "Title is required"
```

Produces:

```rust
/// Spec: specs/<feature>/spec.md > REQ-002 > Scenario: Empty title rejected
#[test]
fn test_<feature_snake>_empty_title_rejected() {
    let app = MyApp;
    let mut model = Model::default();
    model.page = Page::AddItem;

    let mut cmd = app.update(
        Event::Submit(Input { title: String::new() }),
        &mut model,
    );

    cmd.expect_one_effect().expect_render();
    let view = app.view(&model);
    // assert validation error is visible in the view
}
```

### Navigation Requirements

Navigation scenarios test page transitions:

```markdown
### Requirement: Error recovery
ID: REQ-003
#### Scenario: Retry from error page
- WHEN user is on error page and taps retry
- THEN app returns to loading and re-fetches data
```

Produces:

```rust
/// Spec: specs/<feature>/spec.md > REQ-003 > Scenario: Retry from error page
#[test]
fn test_<feature_snake>_retry_from_error_page() {
    let app = MyApp;
    let mut model = Model::default();
    model.page = Page::Error {
        message: "Network error".to_string(),
    };

    let mut cmd = app.update(
        Event::Navigate(Route::Home),
        &mut model,
    );

    assert!(matches!(model.page, Page::Loading));
    // assert HTTP effect was emitted for data fetch
}
```

## Traceability

Each generated test includes a traceability comment linking back to the spec
with the stable requirement ID:

```rust
/// Spec: specs/todo/spec.md > REQ-001 > Scenario: Add new item
#[test]
fn test_todo_add_new_item() { ... }
```

This enables automated drift detection: parse test comments to find the
source scenario and requirement ID, then verify the requirement and scenario
still exist in the spec with matching WHEN/THEN clauses.

## Drift Detection Mechanics

### Detecting Missing Tests

1. Parse all requirement blocks from the spec, including each
   `### Requirement:`, `ID: REQ-XXX`, and `#### Scenario:` entry
2. Parse all `#[test]` functions with `/// Spec:` comments from `app.rs`
3. For each scenario, check if a corresponding test exists (match on
   REQ-ID + scenario title)
4. Report scenarios without tests as **missing coverage**

### Detecting Stale Tests

1. Parse all test functions with `/// Spec:` traceability comments
2. Check if the referenced requirement ID and scenario still exist in
   the spec
3. Report tests referencing removed scenarios as **stale tests**

Tests without `/// Spec:` comments are treated as manually added and are
not flagged by drift detection.

### Detecting Assertion Drift

1. Parse THEN clauses from the spec scenario
2. Parse assertions from the test function
3. Compare expected values (page states, view fields, effect types)
4. Report mismatches as **assertion drift**

This comparison is approximate -- it catches obvious divergences (wrong
page state, missing effect assertion, wrong field value) but may not detect
subtle logic changes.
