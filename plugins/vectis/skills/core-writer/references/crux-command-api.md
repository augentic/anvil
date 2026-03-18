# Crux Command API

`Command` represents one or more side-effects returned from the `update()` function.
Commands describe what should happen; the shell executes them.

In 0.17.0-rc3 the return type is `Command<Effect, Event>`.

## Creating Commands

### From capabilities (most common)

Capabilities return command builders. Convert to a `Command` with `.then_send()`:

```rust
use crux_core::render::render;

// Render -- notify the shell to re-render the UI
render()

// HTTP GET -- fetch JSON, send result back as an Event
Http::get("https://api.example.com/items")
    .expect_json()
    .build()
    .then_send(Event::ItemsFetched)

// HTTP POST with JSON body
Http::post("https://api.example.com/items")
    .body_json(&new_item)?
    .expect_json()
    .build()
    .then_send(Event::ItemCreated)

// Key-Value get
KeyValue::get("my-key")
    .then_send(Event::ValueLoaded)

// Key-Value set
KeyValue::set("my-key", serialized_bytes)
    .then_send(Event::ValueSaved)
```

### No-op command

When no side-effects are needed:

```rust
Command::done()
```

### Emit an event without a side-effect

When you need to dispatch an event to yourself without requesting anything from the shell:

```rust
Command::event(Event::ProcessNext)
```

## Combining Commands

### Concurrent execution with `.and()`

Run two commands concurrently:

```rust
render().and(
    Http::get(API_URL)
        .expect_json()
        .build()
        .then_send(Event::DataFetched)
)
```

### Concurrent execution with `Command::all()`

Run multiple commands concurrently:

```rust
Command::all([
    render(),
    Http::get(url_a).expect_json().build().then_send(Event::GotA),
    Http::get(url_b).expect_json().build().then_send(Event::GotB),
])
```

## Chaining Commands

### Sequential effects with `.then_request()`

Use the output of one effect as input to the next:

```rust
Http::post(API_URL)
    .body_json(&new_post)?
    .expect_json::<Post>()
    .build()
    .then_request(|result| {
        let post = result.unwrap();
        let url = &post.body().unwrap().url;
        Http::get(url).expect_json().build()
    })
    .then_send(Event::GotPost)
```

### Mapping outputs with `.map()`

Transform the output of a request before sending:

```rust
Http::get(API_URL)
    .expect_json::<ApiResponse>()
    .build()
    .map(|result| result.map(|r| r.take_body().unwrap().items))
    .then_send(Event::GotItems)
```

## Async Commands

For complex orchestrations that are more natural in async Rust:

```rust
Command::new(|ctx| async move {
    // Sequential requests
    let first = Http::post(API_URL)
        .body_json(&data)
        .expect_json::<Post>()
        .build()
        .into_future(ctx.clone())
        .await;

    let post = first.unwrap();
    let url = &post.body().unwrap().url;

    let second = Http::get(url)
        .expect_json()
        .build()
        .into_future(ctx.clone())
        .await;

    ctx.send_event(Event::GotPost(second));
})
```

### Spawning concurrent tasks

```rust
Command::new(|ctx| async move {
    let (tx, rx) = async_channel::unbounded();

    ctx.spawn(|ctx| async move {
        for i in 0..10u8 {
            let output = ctx.request_from_shell(AnOperation(i)).await;
            tx.send(output).await.unwrap();
        }
    });

    ctx.spawn(|ctx| async move {
        while let Ok(value) = rx.recv().await {
            ctx.send_event(Event::Completed(value));
        }
    });
})
```

### Converting builders to futures

Inside `Command::new`, convert capability builders to futures with `.into_future(ctx)`:

```rust
Command::new(|ctx| async move {
    let response = Http::get(url)
        .expect_json::<Data>()
        .build()
        .into_future(ctx.clone())
        .await;

    ctx.send_event(Event::DataReady(response));
})
```

## Abort Handles

Commands can be cancelled:

```rust
let mut cmd = Command::all([
    Command::request_from_shell(OpA).then_send(Event::Done),
    Command::request_from_shell(OpB).then_send(Event::Done),
]);

let handle = cmd.abort_handle();
// later...
handle.abort();
assert!(cmd.was_aborted());
```

## Common Patterns

### Optimistic update + API call

```rust
Event::Increment => {
    model.count += 1;
    model.confirmed = false;

    render().and(
        Http::post(format!("{API_URL}/inc"))
            .expect_json()
            .build()
            .then_send(Event::ServerUpdated)
    )
}
```

### Load then render

```rust
Event::Initialize => {
    Http::get(API_URL)
        .expect_json()
        .build()
        .then_send(Event::DataLoaded)
}
Event::DataLoaded(Ok(mut response)) => {
    model.data = response.take_body().unwrap();
    model.initialized = true;
    render()
}
```

### Save to KV then confirm

```rust
Event::Save => {
    let bytes = serde_json::to_vec(&model.items).unwrap();
    KeyValue::set("items", bytes)
        .then_send(Event::Saved)
}
Event::Saved(Ok(_)) => {
    model.save_status = SaveStatus::Saved;
    render()
}
```

## Important Rules

- Commands execute asynchronously. They do **not** have access to the model after creation.
- To update state based on an effect's result, send the result back as an Event.
- `render()` must be called whenever the view model should change. It is not automatic.
- `Command::done()` means "nothing to do" -- use it only when no render is needed either.
- Every match arm in `update()` must return a `Command`.
