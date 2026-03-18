# Crux Capabilities: HTTP and Key-Value

Capabilities provide ergonomic APIs for requesting side-effects from the shell.
Each capability defines an `Operation` type (the request/response protocol) and
convenience methods that return command builders.

## Setup Pattern

Each capability requires:

1. A variant in your `Effect` enum wrapping the capability's operation type.
2. A type alias binding the capability to your `Effect` and `Event` types.
3. The capability crate in `Cargo.toml` dependencies.

## Render (built-in)

Render is built into `crux_core`. It notifies the shell that a new view model is available.

```rust
use crux_core::render::{render, RenderOperation};
```

Effect variant:

```rust
#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
}
```

Usage in `update()`:

```rust
render()  // returns a Command
```

Render is a notification (fire-and-forget) -- it never sends an event back.
Call `render()` at the end of any `update()` branch that changes the view model.

## HTTP (`crux_http`)

### Dependencies

```toml
# workspace Cargo.toml
[workspace.dependencies]
crux_http = { git = "https://github.com/redbadger/crux", tag = "crux_core-v0.17.0-rc3" }
```

### Effect variant

```rust
use crux_http::HttpRequest;

#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
}
```

### Type alias

```rust
type Http = crux_http::Http<Effect, Event>;
```

### Event variants for HTTP responses

```rust
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Event {
    FetchItems,

    #[serde(skip)]
    #[facet(skip)]
    ItemsFetched(#[facet(opaque)] crux_http::Result<crux_http::Response<Vec<Item>>>),

    #[serde(skip)]
    #[facet(skip)]
    ItemCreated(#[facet(opaque)] crux_http::Result<crux_http::Response<Item>>),
}
```

### API Methods

**GET with JSON response:**

```rust
Http::get("https://api.example.com/items")
    .expect_json()          // deserialize response body as JSON
    .build()                // returns a crux_core command RequestBuilder
    .then_send(Event::ItemsFetched)
```

The type parameter for `expect_json()` is inferred from the Event variant's payload.

**POST with JSON body:**

```rust
Http::post("https://api.example.com/items")
    .body_json(&new_item)   // serialize body as JSON (returns Result)
    .expect("serialize body")
    .expect_json()
    .build()
    .then_send(Event::ItemCreated)
```

**POST with string body:**

```rust
Http::post(url)
    .body_string("hello".to_string())
    .build()
    .then_send(Event::Response)
```

**Setting headers:**

```rust
Http::get(url)
    .header("Authorization", format!("Bearer {token}"))
    .header("Accept", "application/json")
    .expect_json()
    .build()
    .then_send(Event::Response)
```

**Expect string response (instead of JSON):**

```rust
Http::get(url)
    .expect_string()
    .build()
    .then_send(Event::GotHtml)
```

**URL construction with `url` crate:**

```rust
use url::Url;

let base = Url::parse(API_URL).unwrap();
let url = base.join("/items").unwrap();
Http::get(url).expect_json().build().then_send(Event::Items)
```

### Handling HTTP responses in `update()`

```rust
Event::ItemsFetched(Ok(mut response)) => {
    let items = response.take_body().unwrap();
    model.items = items;
    model.loading = false;
    render()
}
Event::ItemsFetched(Err(e)) => {
    model.error = Some(format!("Failed to fetch: {e}"));
    model.loading = false;
    render()
}
```

Key points:
- `response.take_body()` extracts the deserialized body (consumes it).
- The response also carries status code and headers if needed.
- Always handle both `Ok` and `Err` variants.

### HTTP Methods Available

| Method | Usage |
|--------|-------|
| `Http::get(url)` | GET request |
| `Http::post(url)` | POST request |
| `Http::put(url)` | PUT request |
| `Http::patch(url)` | PATCH request |
| `Http::delete(url)` | DELETE request |
| `Http::head(url)` | HEAD request |
| `Http::options(url)` | OPTIONS request |

## Key-Value (`crux_kv`)

### Dependencies

```toml
# workspace Cargo.toml
[workspace.dependencies]
crux_kv = { git = "https://github.com/redbadger/crux", tag = "crux_core-v0.17.0-rc3" }
```

### Effect variant

```rust
use crux_kv::KeyValueOperation;

#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    KeyValue(KeyValueOperation),
}
```

### Type alias

```rust
type KeyValue = crux_kv::KeyValue<Effect, Event>;
```

### Event variants for KV responses

```rust
use crux_kv::error::KeyValueError;

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Event {
    Save,
    Load,

    #[serde(skip)]
    #[facet(skip)]
    Loaded(#[facet(opaque)] Result<Option<Vec<u8>>, KeyValueError>),

    #[serde(skip)]
    #[facet(skip)]
    Saved(#[facet(opaque)] Result<Option<Vec<u8>>, KeyValueError>),
}
```

### API Methods

**Get a value:**

```rust
KeyValue::get("my-key")
    .then_send(Event::Loaded)
```

Returns `Result<Option<Vec<u8>>, KeyValueError>` -- `None` if key doesn't exist.

**Set a value:**

```rust
let bytes = serde_json::to_vec(&model.data).unwrap();
KeyValue::set("my-key", bytes)
    .then_send(Event::Saved)
```

Returns `Result<Option<Vec<u8>>, KeyValueError>` — the previous value if one existed.

**Delete a value:**

```rust
KeyValue::delete("my-key")
    .then_send(Event::Deleted)
```

Returns `Result<Option<Vec<u8>>, KeyValueError>` -- the previous value if it existed.

**Check if a key exists:**

```rust
KeyValue::exists("my-key")
    .then_send(Event::Exists)
```

Returns `Result<bool, KeyValueError>`.

**List keys by prefix:**

```rust
KeyValue::list_keys("items:", 0)  // prefix, cursor
    .then_send(Event::KeysListed)
```

Returns `Result<(Vec<String>, u64), KeyValueError>` -- keys and next cursor (0 if done).

### Handling KV responses in `update()`

```rust
Event::Loaded(Ok(Some(bytes))) => {
    let items: Vec<Item> = serde_json::from_slice(&bytes).unwrap_or_default();
    model.items = items;
    render()
}
Event::Loaded(Ok(None)) => {
    model.items = Vec::new();
    render()
}
Event::Loaded(Err(e)) => {
    model.error = Some(format!("Load failed: {e}"));
    render()
}
Event::Saved(Ok(_)) => {
    model.save_status = SaveStatus::Saved;
    render()
}
Event::Saved(Err(e)) => {
    model.save_status = SaveStatus::Failed;
    model.error = Some(format!("Save failed: {e}"));
    render()
}
```

### Serialization Pattern for KV

The KV store operates on raw bytes. Serialize/deserialize with `serde_json`:

```rust
// Saving
let bytes = serde_json::to_vec(&model.items).unwrap();
KeyValue::set("items", bytes).then_send(Event::Saved)

// Loading
Event::Loaded(Ok(Some(bytes))) => {
    let items: Vec<Item> = serde_json::from_slice(&bytes).unwrap_or_default();
    model.items = items;
    render()
}
```

## Combining Capabilities

When an event needs multiple effects:

```rust
Event::Initialize => {
    Command::all([
        render(),
        Http::get(API_URL).expect_json().build().then_send(Event::DataFetched),
        KeyValue::get("cache").then_send(Event::CacheLoaded),
    ])
}
```

Or with `.and()`:

```rust
render().and(
    Http::get(API_URL).expect_json().build().then_send(Event::DataFetched)
)
```
