# Crux App Pattern (0.17+ API)

This reference describes the types and trait implementation for a Crux core application
targeting the 0.17+ API on the master branch.

## App Trait

The `App` trait is the central interface. It has four associated types and two methods:

```rust
use crux_core::{
    App, Command,
    macros::effect,
    render::{RenderOperation, render},
};

#[derive(Default)]
pub struct MyApp;

impl App for MyApp {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
        // handle events, mutate model, return side-effect commands
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        // map model to view model (pure function, no side effects)
    }
}
```

Key rules:
- The app struct must derive `Default` so Crux can construct it.
- `update()` returns `Command` (no generic parameters in 0.17+).
- `view()` is a pure function -- it reads model and returns a view model.
- There is no `Capabilities` associated type or `caps` parameter.

## Model

The `Model` holds all internal application state. It is never sent to the shell.

```rust
#[derive(Default)]
pub struct Model {
    page: Page,
    count: isize,
    items: Vec<Item>,
    error_message: Option<String>,
}
```

Rules:
- Must implement `Default` to define initial state.
- Must include a `page: Page` field to track the current view (see **Page Management**).
- Fields are `pub(crate)` or private -- never `pub` (they don't leave the core).
- Use newtypes for domain identifiers: `struct ItemId(String)`.
- Use enums for known value sets: `enum Filter { All, Active, Completed }`.
- Complex nested state is fine -- the model is a tree.

## Event

Events are the input to the core. They come from two sources:

1. **Shell-facing** -- triggered by user interaction in the UI, sent across FFI.
   These must be serializable.
2. **Internal** -- used as callbacks from side-effects. Marked with `#[serde(skip)]`
   and `#[facet(skip)]` so they cannot be sent from the shell.

```rust
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Event {
    // Shell-facing events (user actions)
    Navigate(Route),
    Increment,
    Decrement,
    Reset,
    FetchData,

    // Internal events (effect callbacks)
    #[serde(skip)]
    #[facet(skip)]
    DataFetched(#[facet(opaque)] crux_http::Result<crux_http::Response<MyData>>),
}
```

Rules:
- Derive `Facet, Serialize, Deserialize` for FFI compatibility.
- Add `#[repr(C)]` for `facet` enum layout.
- Include `Navigate(Route)` for shell-initiated view changes (see **Shell-initiated
  navigation** under Page Management).
- Internal variants with non-serializable payloads (like `crux_http::Result`) must have
  `#[serde(skip)]` and `#[facet(skip)]`.
- Mark opaque fields inside skipped variants with `#[facet(opaque)]`.
- Enum tuple variants double as constructor functions for `then_send`:
  `Http::get(url).build().then_send(Event::DataFetched)`.

## ViewModel

The ViewModel is an **enum** where each variant represents a distinct view (page or
screen) of the application. It crosses the FFI boundary so it must be fully
serializable and have type generation support. The shell pattern-matches on the
ViewModel to decide which screen to render.

```rust
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
#[repr(C)]
pub enum ViewModel {
    #[default]
    Loading,
    Main(MainView),
    Error(ErrorView),
}
```

Each variant that carries data wraps a **per-page view struct**:

```rust
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct MainView {
    pub count: String,
    pub items: Vec<ItemView>,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default)]
pub struct ErrorView {
    pub message: String,
    pub can_retry: bool,
}
```

Rules:
- The ViewModel enum derives `Facet, Serialize, Deserialize, Clone, Debug, Default`
  and has `#[repr(C)]`.
- Per-page view structs derive `Facet, Serialize, Deserialize, Clone, Debug, Default`.
- All fields on per-page view structs are `pub` (the shell reads them).
- Use `String` for formatted display values -- formatting logic belongs in the core's
  `view()` function, not in the shell.
- Use simple types the shell can easily consume. Avoid complex enums in per-page view
  structs when a bool or string suffices.
- The ViewModel is computed fresh on each `view()` call; it is not incrementally updated.
- `#[default]` on the initial variant (typically `Loading`) provides the `Default` impl.
- Variants without associated data (e.g. `Loading`) represent screens the shell renders
  without needing data from the core.

### Error views vs in-page errors

Two categories of error require different treatment:

- **Blocking errors** (failed initialization, auth failure) warrant a dedicated
  `ViewModel::Error(ErrorView)` variant. `ErrorView` carries a user-facing `message`
  and a `can_retry` flag so the shell knows whether to show a retry button.
- **Recoverable errors** (a single HTTP request failing, going offline) are handled
  as fields within a page's view struct (e.g. `sync_status: String`). The user stays
  on the current page -- no separate ViewModel variant is needed.

## Page Management

The core controls all navigation. The `Model` tracks which page is current using a
private `Page` enum. The `view()` function matches on `model.page` to produce the
right ViewModel variant. Page transitions happen in `update()` by setting `model.page`.

### Page enum (internal)

```rust
#[derive(Default)]
enum Page {
    #[default]
    Loading,
    Main,
    Error,
}
```

The `Page` enum is internal to the core -- it does NOT derive `Facet` or `Serialize`
and never crosses the FFI boundary. Add it as a field on `Model`:

```rust
#[derive(Default)]
pub struct Model {
    page: Page,
    error_message: Option<String>,
    // ... other fields
}
```

### Page transitions in `update()`

Set `model.page` to transition between views. The shell sees the change on the
next `view()` call (triggered by a `render()`).

```rust
Event::Initialize => {
    // page is already Page::Loading (the default)
    KeyValue::get("state").then_send(Event::DataLoaded)
}
Event::DataLoaded(Ok(Some(bytes))) => {
    // restore state ...
    model.page = Page::Main;
    render()
}
Event::DataLoaded(Err(e)) => {
    model.page = Page::Error;
    model.error_message = Some(format!("Failed to load data: {e}"));
    render()
}
Event::Navigate(Route::Main) => match model.page {
    Page::Error => {
        model.page = Page::Loading;
        model.error_message = None;
        Command::event(Event::Initialize)
    }
    _ => Command::done(),
}
```

### `view()` matches on page

```rust
fn view(&self, model: &Self::Model) -> Self::ViewModel {
    match model.page {
        Page::Loading => ViewModel::Loading,
        Page::Main => ViewModel::Main(MainView {
            count: format!("Count is: {}", model.count),
            items: model.items.iter().map(/* ... */).collect(),
        }),
        Page::Error => ViewModel::Error(ErrorView {
            message: model.error_message.clone().unwrap_or_default(),
            can_retry: true,
        }),
    }
}
```

Rules:
- Every `Page` variant must have a corresponding `ViewModel` variant and a match arm
  in `view()`.
- Every `Page` variant must be reachable by at least one transition in `update()`.
- The `Page` enum and `ViewModel` enum variants should have a 1:1 correspondence.
- For single-page apps, the ViewModel enum has a single variant wrapping the page's
  view struct. A `Loading` variant is recommended when the app loads data on startup.

### Shell-initiated navigation

The shell can request view changes via an `Event::Navigate(Route)` event. The
`Route` enum is a shell-facing type that enumerates navigable destinations --
typically a subset of `Page`, excluding internal states like `Loading` and `Error`.

```rust
#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub enum Route {
    #[default]
    Main,
    Settings,
    ItemDetail(String),
}
```

Add a shell-facing event variant:

```rust
pub enum Event {
    Navigate(Route),
    // ... other events
}
```

The `update()` handler maps `Route` to `Page`, taking current state into account.
The core decides what action is required -- it may need to load data, clear error
state, or simply switch the page:

```rust
Event::Navigate(route) => {
    match route {
        Route::Main => match model.page {
            Page::Error => {
                model.page = Page::Loading;
                model.error_message = None;
                Command::event(Event::Initialize)
            }
            Page::Loading => Command::done(),
            Page::Main => Command::done(),
        },
        Route::Settings => {
            model.page = Page::Settings;
            render()
        }
        Route::ItemDetail(id) => {
            model.selected_item_id = Some(id);
            model.page = Page::ItemDetail;
            render()
        }
    }
}
```

Rules:
- `Route` derives `Facet, Serialize, Deserialize` and has `#[repr(C)]` -- it crosses FFI.
- `Route` variants represent **user-navigable destinations** only. Internal states
  (Loading, Error) are not `Route` variants -- the core transitions to those
  automatically.
- The `Navigate` handler must be state-aware: navigating to a view that requires
  data may trigger a load sequence rather than an immediate page switch.
- `Route` can carry payload for parameterised views (e.g. `ItemDetail(String)` for
  a detail screen that needs an item ID).
- Use cases: deep links, back button, tab bar, push notification opens.

## Effect

The `Effect` enum declares which side-effects the app can request from the shell.
Each variant wraps the `Operation` type from a capability.

```rust
use crux_core::{
    macros::effect,
    render::RenderOperation,
};
use crux_http::HttpRequest;
use crux_kv::KeyValueOperation;

#[effect(facet_typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    KeyValue(KeyValueOperation),
}
```

Rules:
- Annotate with `#[effect(facet_typegen)]` for type generation support.
- Derive `Debug`.
- Always include `Render(RenderOperation)` -- every app needs UI updates.
- Add one variant per capability used. The variant name is arbitrary but conventionally
  matches the capability name.
- The macro generates helper methods like `expect_render()`, `expect_http()`, etc.
  on effect types for use in tests.

## Type Aliases for Capabilities

When using published capabilities, define a type alias that binds the capability
to your app's `Effect` and `Event` types:

```rust
type Http = crux_http::Http<Effect, Event>;
type KeyValue = crux_kv::KeyValue<Effect, Event>;
```

Then use them in `update()`:

```rust
Http::get(API_URL).expect_json().build().then_send(Event::DataFetched)
```

## Supporting Types

Domain types used across Model, Event, and ViewModel:

```rust
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub completed: bool,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemView {
    pub id: String,
    pub title: String,
    pub completed: bool,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub enum Filter {
    #[default]
    All,
    Active,
    Completed,
}
```

Rules:
- Types that cross the FFI boundary (in Event or ViewModel) must derive `Facet`.
- Types only used internally (Model-only) do not need `Facet` or `Serialize`.
- Enums used in FFI types need `#[repr(C)]`.
- Use `#[default]` on the default variant of enums that derive `Default`.

## The `view()` Function

The `view()` function maps Model to ViewModel. It is a pure function with no
side effects. All formatting and presentation logic belongs here.

The function matches on `model.page` and returns the corresponding ViewModel variant:

```rust
fn view(&self, model: &Self::Model) -> Self::ViewModel {
    match model.page {
        Page::Loading => ViewModel::Loading,
        Page::Main => ViewModel::Main(MainView {
            count: format!("Count is: {}", model.count),
            items: model
                .items
                .iter()
                .filter(|item| match model.filter {
                    Filter::All => true,
                    Filter::Active => !item.completed,
                    Filter::Completed => item.completed,
                })
                .map(|item| ItemView {
                    id: item.id.clone(),
                    title: item.title.clone(),
                    completed: item.completed,
                })
                .collect(),
        }),
        Page::Error => ViewModel::Error(ErrorView {
            message: model.error_message.clone().unwrap_or_default(),
            can_retry: true,
        }),
    }
}
```

## The `update()` Function

The `update()` function handles events, mutates the model, and returns commands.
Every match arm must return a `Command`.

```rust
fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
    match event {
        Event::Increment => {
            model.count += 1;
            render::render()
        }
        Event::FetchData => {
            model.loading = true;
            render::render().and(
                Http::get(API_URL)
                    .expect_json()
                    .build()
                    .then_send(Event::DataFetched),
            )
        }
        Event::DataFetched(Ok(mut response)) => {
            model.data = response.take_body().unwrap();
            model.loading = false;
            render::render()
        }
        Event::DataFetched(Err(_)) => {
            model.loading = false;
            model.error = Some("Failed to fetch data".to_string());
            render::render()
        }
    }
}
```

Use `Command::done()` when no side-effects are needed and no render is required
(rare -- usually you want at least `render::render()`).

## Pending Operation Sync Queue

When the app queues local mutations (create, update, delete) as pending operations and
syncs them to a server one at a time via HTTP, use the following pattern to avoid a
race condition where concurrent events (SSE, fetch-all) shift the queue while an HTTP
request is in-flight.

**The bug this prevents:** calling `pending_ops.remove(0)` in the HTTP response handler
assumes the completed op is still at index 0. If an SSE event removes that same op via
`retain(...)` first, `remove(0)` silently drops an unrelated pending operation, losing
a user mutation.

### Model fields

```rust
#[derive(Default)]
pub struct Model {
    page: Page,
    items: Vec<Item>,
    pending_ops: Vec<PendingOp>,
    /// Item ID of the currently in-flight sync operation.
    syncing_id: Option<String>,
    sync_status: SyncStatus,
    // ...
}
```

### Starting sync

Always sync from `pending_ops[0]`. Record its ID before dispatching the HTTP command:

```rust
fn start_sync(model: &mut Model) -> Command<Effect, Event> {
    if model.pending_ops.is_empty() {
        model.sync_status = SyncStatus::Idle;
        return render();
    }
    if model.sync_status == SyncStatus::Syncing {
        return Command::done();
    }
    model.sync_status = SyncStatus::Syncing;

    let op = model.pending_ops[0].clone();
    model.syncing_id = Some(op.item_id().to_string());

    // ... build and return the HTTP command for `op`
}
```

### Handling the response

Remove by ID match, not by index. Use `retain` so it is a no-op when SSE already
removed the op:

```rust
Event::OpResponse(Ok(mut response)) => {
    if let Some(server_item) = response.take_body() {
        update_or_insert_item(model, &server_item);
    }
    if let Some(synced_id) = model.syncing_id.take() {
        model.pending_ops.retain(|op| op.item_id() != synced_id);
    }
    model.sync_status = SyncStatus::Idle;
    Self::save_state(model).and(Command::event(Event::RetrySync))
}
```

### Error handling

Clear `syncing_id` on error so the next `RetrySync` can re-attempt the same op:

```rust
Event::OpResponse(Err(_)) | Event::DeleteOpResponse(Err(_)) => {
    model.syncing_id = None;
    model.sync_status = SyncStatus::Offline;
    render()
}
```

### Why `retain` instead of `remove`

- `retain(|op| op.item_id() != id)` is idempotent -- safe if SSE already removed the op.
- `remove(0)` is positional -- if the queue shifted, it drops the wrong operation.
- The `syncing_id` field makes the "which op is in-flight" question explicit rather than
  relying on the assumption that it is always at index 0.
