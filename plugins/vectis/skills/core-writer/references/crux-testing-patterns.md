# Crux Testing Patterns

Crux apps are tested by calling `update()` directly, inspecting the returned `Command`
for effects and events, resolving effects with simulated responses, and asserting on
model and view model state.

No mocking or async runtime is needed. Tests are synchronous and deterministic.

## Basic Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crux_core::App as _;

    #[test]
    fn test_event() {
        let app = MyApp;
        let mut model = Model::default();

        // 1. Send an event
        let mut cmd = app.update(Event::SomeAction, &mut model);

        // 2. Assert on model state
        assert_eq!(model.count, 1);

        // 3. Assert on effects
        cmd.expect_one_effect().expect_render();

        // 4. Assert on view model
        let view = app.view(&model);
        assert_eq!(view.count, "1");
    }
}
```

Import `crux_core::App as _` to bring the `update` and `view` methods into scope
when your app type shadows the `App` name.

## Asserting Effects

### Single render effect

```rust
let mut cmd = app.update(Event::Increment, &mut model);
cmd.expect_one_effect().expect_render();
```

`expect_one_effect()` asserts there is exactly one effect and returns it.
`expect_render()` asserts that effect is a `Render`.

### Single HTTP effect

```rust
let mut cmd = app.update(Event::FetchData, &mut model);
let mut request = cmd.expect_one_effect().expect_http();
```

`expect_http()` returns an HTTP request that can be inspected and resolved.

### Multiple effects

When a command produces multiple effects (e.g., `render().and(http_call)`),
use `expect_effect()` to consume them one at a time:

```rust
let mut cmd = app.update(Event::Increment, &mut model);

// First effect: render
cmd.expect_effect().expect_render();

// Second effect: HTTP request
let mut request = cmd.expect_one_effect().expect_http();
```

`expect_effect()` returns the next available effect without asserting it's the only one.
`expect_one_effect()` asserts it's the last remaining effect.

### Using `assert_effect!` macro

For quick assertions without consuming the effect:

```rust
use crux_core::assert_effect;

let mut cmd = app.update(Event::Reset, &mut model);
assert_effect!(cmd, Effect::Render(_));
```

### Splitting operation and request handle

For inspecting the operation separately from resolving:

```rust
let mut cmd = app.update(Event::FetchData, &mut model);
let (operation, mut request) = cmd.expect_one_effect().expect_http().split();

assert_eq!(
    operation,
    HttpRequest::get("https://api.example.com/data").build()
);
```

## Inspecting HTTP Requests

```rust
let mut cmd = app.update(Event::FetchData, &mut model);
let mut request = cmd.expect_one_effect().expect_http();

// Check the operation details
assert_eq!(
    &request.operation,
    &HttpRequest::get("https://api.example.com/items").build()
);

// For POST requests with body
assert_eq!(
    &request.operation,
    &HttpRequest::post("https://api.example.com/items")
        .header("content-type", "application/json")
        .body(r#"{"title":"New Item"}"#)
        .build()
);
```

## Resolving Effects

After inspecting an effect, resolve it with a simulated response to continue
the command's execution.

### Resolving HTTP requests

```rust
use crux_http::protocol::{HttpRequest, HttpResponse, HttpResult};
use crux_http::testing::ResponseBuilder;

// Resolve with a successful JSON response
request
    .resolve(HttpResult::Ok(
        HttpResponse::ok()
            .body(r#"{"value": 42}"#)
            .build(),
    ))
    .expect("Resolve should succeed");

// After resolving, check the resulting event
let event = cmd.expect_one_event();
let expected = Event::DataFetched(Ok(
    ResponseBuilder::ok()
        .body(MyData { value: 42 })
        .build()
));
assert_eq!(event, expected);
```

### Resolving KV requests

```rust
use crux_kv::KeyValueOperation;

let mut request = cmd.expect_one_effect().expect_key_value();

// Resolve get with existing value
request
    .resolve(crux_kv::KeyValueResult::Ok {
        response: crux_kv::KeyValueResponse::Get {
            value: Some(b"stored data".to_vec()),
        },
    })
    .unwrap();
```

### Resolving stream effects (SSE)

Stream effects can be resolved multiple times:

```rust
let mut request = cmd.expect_one_effect().expect_server_sent_events();

// First chunk
request
    .resolve(SseResponse::Chunk(br#"data: {"value":1}

"#.to_vec()))
    .unwrap();

let event = cmd.expect_one_event();
assert_eq!(event, Event::Update(Data { value: 1 }));

// Second chunk
request
    .resolve(SseResponse::Chunk(br#"data: {"value":2}

"#.to_vec()))
    .unwrap();

let event = cmd.expect_one_event();
assert_eq!(event, Event::Update(Data { value: 2 }));

// End the stream
request.resolve(SseResponse::Done).unwrap();
```

## Chaining Updates

Feed events back into the app to test multi-step flows:

```rust
#[test]
fn full_fetch_flow() {
    let app = MyApp;
    let mut model = Model::default();

    // Step 1: User triggers fetch
    let mut cmd = app.update(Event::FetchData, &mut model);
    assert!(model.loading);

    // Step 2: Resolve the HTTP request
    let mut request = cmd.expect_one_effect().expect_http();
    request
        .resolve(HttpResult::Ok(
            HttpResponse::ok()
                .body(r#"{"items": [{"id": "1", "title": "Test"}]}"#)
                .build(),
        ))
        .unwrap();

    // Step 3: Get the resulting event
    let event = cmd.expect_one_event();

    // Step 4: Feed the event back to update
    let mut cmd = app.update(event, &mut model);
    assert!(!model.loading);
    assert_eq!(model.items.len(), 1);

    // Step 5: Check render was requested
    cmd.expect_one_effect().expect_render();

    // Step 6: Verify the view
    let view = app.view(&model);
    assert_eq!(view.items.len(), 1);
}
```

## Command State Assertions

```rust
// Check if all effects have been consumed
assert!(cmd.is_done());

// Check if command was aborted
assert!(cmd.was_aborted());
```

## Testing with Pre-set Model State

Set up model state before calling update to test specific scenarios:

```rust
#[test]
fn decrement_from_five() {
    let app = Counter;
    let mut model = Model { count: 5 };

    let mut cmd = app.update(Event::Decrement, &mut model);
    assert_eq!(model.count, 4);

    let view = app.view(&model);
    assert_eq!(view.count, "Count is: 4");

    cmd.expect_one_effect().expect_render();
}
```

## Snapshot Testing with insta

For complex models or view models, use the `insta` crate for snapshot testing:

```rust
use insta::assert_yaml_snapshot;

#[test]
fn model_after_update() {
    let app = MyApp;
    let mut model = Model::default();

    app.update(Event::Initialize, &mut model);

    assert_yaml_snapshot!(model, @r#"
    items: []
    loading: true
    error: ~
    "#);
}
```

Add `insta` to dev-dependencies:

```toml
[dev-dependencies]
insta = { version = "1", features = ["yaml"] }
```

The model must derive `Serialize` for insta snapshots.

## Test Conventions

1. **One test per event variant** minimum.
2. **Test both success and error paths** for effects that can fail.
3. **Test the view model** after state changes, not just the model.
4. **Name tests descriptively**: `test_increment_updates_count`, `test_fetch_error_shows_message`.
5. **Use `#[cfg(test)] mod tests`** inside `app.rs`, not a separate file.
6. **Import `crux_core::App as _`** at the top of the test module.
