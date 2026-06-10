# Spec-to-Test Mapping (Vectis / Crux)

Vectis-specific deltas on top of the shared [spec-to-test mapping discipline](spec-runtime/spec-to-test-mapping.md). The shared base owns the target-neutral rules: scenario → one test function, the `test_<unit_snake>_<scenario_snake>` naming convention, REQ-ID traceability comments, requirement coverage (N scenarios → N tests), and drift-detection mechanics. This file carries only what is specific to a Crux shared crate.

## Test location and attribute

Each domain spec maps to tests inside the `#[cfg(test)] mod tests` block in `shared/src/app.rs` (Crux convention — tests live alongside the app, not in a separate `tests/` directory):

```text
specs/<domain>/spec.md  →  #[cfg(test)] mod tests { ... } in app.rs
```

All spec-mapped tests are synchronous `#[test]` — Crux's testing model does not require an async runtime:

```text
#### Scenario: Successful item fetch
  →  #[test] fn test_<unit_snake>_successful_item_fetch()
```

## WHEN clause to test setup

The WHEN clause determines how the test constructs initial model state and which Event to send:

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

## THEN clause to assertions

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

## Effect chain mapping

Scenarios describing async operations map to multi-step tests that resolve effects and feed events back:

```text
#### Scenario: Fetch items on load
- WHEN app starts
- THEN app shows loading and fetches items from /api/items
- AND WHEN items are returned
- THEN app shows the item list
```

Maps to:

```rust
/// Spec: specs/<domain>/spec.md > REQ-001 > Scenario: Fetch items on load
#[test]
fn test_<unit_snake>_fetch_items_on_load() {
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

## Worked example — validation requirement

Validation requirements (per the shared base) construct invalid input and assert the resulting model/view state:

```markdown
### Requirement: Input validation
ID: REQ-002
#### Scenario: Empty title rejected
- WHEN user submits item with empty title
- THEN app shows validation error "Title is required"
```

```rust
/// Spec: specs/<domain>/spec.md > REQ-002 > Scenario: Empty title rejected
#[test]
fn test_<unit_snake>_empty_title_rejected() {
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

## Navigation requirements

Navigation scenarios test page transitions:

```markdown
### Requirement: Error recovery
ID: REQ-003
#### Scenario: Retry from error page
- WHEN user is on error page and taps retry
- THEN app returns to loading and re-fetches data
```

```rust
/// Spec: specs/<domain>/spec.md > REQ-003 > Scenario: Retry from error page
#[test]
fn test_<unit_snake>_retry_from_error_page() {
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

## Drift detection (Vectis specifics)

The drift-detection mechanics live in the [shared base](spec-runtime/spec-to-test-mapping.md#drift-detection-mechanics). For Vectis, the test side of the comparison parses `#[test]` functions with `/// Spec:` comments from `app.rs`; assertion-drift comparison focuses on page states, view fields, and effect types.
