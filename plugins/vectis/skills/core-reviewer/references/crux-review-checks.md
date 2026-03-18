# Crux Structural Review Checks

Pattern-based checks for common Crux framework issues. Each check includes a
detection heuristic, a bad/good code example (drawn from real `opsx_todo` vs
`todo` comparisons), and a severity rating.

---

## CRX-001: Missing render() after state mutation

**Severity**: Critical

Every assignment to `model.page`, `model.sync_status`, `model.sse_state`, or
any other field that the `view()` function reads must be accompanied by a
`render()` call in the same `Command` chain. Without it, the shell continues
displaying stale state until the next unrelated event triggers a render.

**Detection**: In each `Event` arm of `update()`, find assignments to model
fields that `view()` reads. Verify the returned `Command` includes `render()`
(directly or via `.and(render())`).

**Bad** (opsx_todo):
```rust
Event::ConnectSse => {
    model.sse_state = SseConnectionState::Connecting;
    // No render() -- UI stays on previous connection state
    ServerSentEvents::get_events(format!("{API_URL}/api/todos/events"))
        .then_send(Event::SseReceived)
}
```

**Good** (todo):
```rust
Event::ConnectSse => {
    model.sse_state = SseConnectionState::Connecting;
    render().and(
        ServerSentEvents::get_events(format!("{API_URL}/api/todos/events"))
            .then_send(Event::SseReceived),
    )
}
```

Also check: `Event::SseDisconnected`, `Event::Navigate(Route::...)` from
Error page, and any handler that transitions `model.page`.

---

## CRX-002: User-supplied text input not validated

**Severity**: Critical

Any Event variant that carries user-typed text (titles, names, descriptions)
must trim whitespace and reject empty strings before mutating the model.
Accepting empty or whitespace-only values leads to invisible list items,
empty database records, or confusing UI state.

**Detection**: Find Event variants with `String` payloads that represent user
input (not IDs or system values). Verify the handler calls `.trim()` and checks
`.is_empty()` before proceeding.

**Bad** (opsx_todo):
```rust
Event::EditTitle(id, new_title) => {
    if let Some(item) = model.items.iter_mut().find(|i| i.id == id) {
        item.title = new_title; // accepts "   " as a valid title
        // ...
    }
}
```

**Good** (todo):
```rust
Event::EditTitle(id, new_title, timestamp) => {
    let new_title = new_title.trim().to_string();
    if new_title.is_empty() {
        return Command::done();
    }
    if let Some(item) = model.items.iter_mut().find(|i| i.id == id) {
        item.title = new_title;
        // ...
    }
}
```

---

## CRX-003: PendingOp variants missing conflict-resolution data

**Severity**: Critical

Every `PendingOp` variant must carry enough information to resolve conflicts
with the server. For destructive operations like `Delete`, this means storing
a `deleted_at` timestamp so the conflict-resolution logic can compare it
against the server's `updated_at`.

**Detection**: Inspect the `PendingOp` enum. Each variant should carry temporal
metadata (timestamps) alongside the item data. A bare `Delete(String)` (ID only)
is insufficient.

**Bad** (opsx_todo):
```rust
pub enum PendingOp {
    Create(TodoItem),
    Update(TodoItem),
    Delete(String), // no timestamp -- can't resolve conflicts
}
```

**Good** (todo):
```rust
pub enum PendingOp {
    Create(TodoItem),
    Update(TodoItem),
    Delete { id: String, deleted_at: String },
}
```

---

## CRX-004: PendingOp coalescing missing on destructive operations

**Severity**: Critical

When deleting an item (via `DeleteTodo` or `ClearCompleted`), the handler must
check whether the item only exists as a pending `Create`. If so, the `Create`
should be removed from the queue and no `Delete` sent to the server -- the
server has never seen the item.

Failing to coalesce causes unnecessary 404 errors from the server and may
trigger error-handling paths that confuse sync state.

**Detection**: In `DeleteTodo` and `ClearCompleted` handlers, check whether the
code inspects existing pending ops before pushing a `Delete`. Look for logic
that detects `PendingOp::Create` for the same item ID and skips the server
delete in that case.

**Bad** (opsx_todo):
```rust
Event::ClearCompleted => {
    // ...
    for id in &completed_ids {
        model.pending_ops.retain(|op| op.item_id() != id);
        model.pending_ops.push(PendingOp::Delete(id.clone()));
        // Always pushes Delete, even for items that were only pending Creates
    }
}
```

**Good** (todo):
```rust
for id in completed {
    let mut saw_create = false;
    let mut saw_non_create = false;
    model.pending_ops.retain(|op| {
        match op {
            PendingOp::Create(item) if item.id == id => {
                saw_create = true;
                false
            }
            PendingOp::Update(item) if item.id == id => {
                saw_non_create = true;
                false
            }
            PendingOp::Delete { id: delete_id, .. } if delete_id == &id => {
                saw_non_create = true;
                false
            }
            _ => true,
        }
    });
    if saw_create && !saw_non_create {
        continue; // item was never synced; no server delete needed
    }
    model.pending_ops.push(PendingOp::Delete { id, deleted_at: timestamp.clone() });
}
```

---

## CRX-005: Domain types crossing FFI bridge missing serde derives

**Severity**: Warning

All types that are part of `ViewModel`, `Event`, or `Effect` (or nested within
them) must derive both `Serialize` and `Deserialize` to cross the FFI bridge
between core and shell. Types used in `StateStore` (KeyValue) persistence also
need both.

Internal-only types used exclusively within `update()` may omit `Deserialize`
if they are never read back, but err on the side of including it.

**Detection**: For each struct/enum, check if it appears in:
- A `ViewModel` variant or view struct -> needs `Serialize + Deserialize`
- An `Event` variant payload -> needs `Serialize + Deserialize`
- An `SseMessage` or similar capability type -> needs `Serialize + Deserialize`
- A `PersistedState` field -> needs `Serialize + Deserialize`

**Bad** (opsx_todo):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SseMessage {  // missing Serialize, Deserialize
    pub event: String,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SyncStatus { ... }  // missing Serialize, Deserialize
```

**Good** (todo):
```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct SseMessage {
    pub event: String,
    pub data: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub enum SyncStatus { ... }
```

---

## CRX-006: Ordering fields as Option weakening conflict resolution

**Severity**: Warning

Fields used for ordering or conflict resolution (`updated_at`, `created_at`,
`version`) should not be `Option` unless the domain genuinely allows missing
values. An `Option` field forces fallback logic (typically "server wins") that
can silently discard valid local edits.

**Detection**: Find timestamp or version fields on domain types. If they are
`Option<String>` or `Option<T>`, check the conflict-resolution code. Look for
`_ => true` or similar catch-all patterns that default to one side winning
when the timestamp is absent.

**Bad** (opsx_todo):
```rust
pub struct TodoItem {
    pub updated_at: Option<String>,  // Option forces fallback
}

// In conflict resolution:
match (&local_item.updated_at, &server_item.updated_at) {
    (Some(local_ts), Some(server_ts)) => server_ts >= local_ts,
    _ => true, // server always wins when either is None
}
```

**Good** (todo):
```rust
pub struct TodoItem {
    pub updated_at: String,  // always present
}
```

---

## CRX-007: IDs/timestamps generated inside update() instead of received from shell

**Severity**: Warning

User-facing Event variants should receive IDs and timestamps as parameters
from the shell, not generate them inside `update()`. This keeps the core
deterministic and testable -- tests can supply known values without mocking
random number generators or clocks.

**Detection**: Search `update()` for calls to UUID generation, `Utc::now()`,
`SystemTime::now()`, or incrementing counters (`model.next_local_id += 1`).
These should instead be parameters on the Event variant.

**Bad**:
```rust
Event::AddTodo => {
    model.next_local_id += 1;
    let id = format!("local-{}", model.next_local_id);
    // ID generated inside update() -- not deterministic in tests
}
```

**Good**:
```rust
Event::AddTodo(id, timestamp) => {
    // ID and timestamp provided by shell -- tests can supply known values
}
```

---

## CRX-008: ViewModel fields using pre-formatted strings

**Severity**: Info

ViewModel fields should use typed values (`usize`, `bool`, enums) rather than
pre-formatted strings. The shell is responsible for presentation; the core
should provide raw data.

Pre-formatted strings couple the core to a specific display format and prevent
shells from localizing, pluralizing, or styling values independently.

**Detection**: In ViewModel / view structs, look for `String` fields that hold
formatted counts, status labels, or display text derived from model state.

**Bad** (opsx_todo):
```rust
pub struct TodoListView {
    pub pending_count: String,  // "3 pending" -- pre-formatted
}
```

**Good** (todo):
```rust
pub struct TodoListView {
    pub pending_count: usize,  // raw count -- shell formats
}
```

---

## CRX-009: Missing test coverage for critical scenarios

**Severity**: Warning

The test module must include tests for the following scenarios at minimum.
Absence of any is a finding.

**Required test scenarios**:
1. SSE event arrives while sync is in-flight for the same item
2. SSE delete removes syncing op without clobbering the next pending op
3. `EditTitle` with empty or whitespace-only string is a no-op
4. `ClearCompleted` with no completed items is a no-op
5. Conflict resolution when server has a newer timestamp than local
6. Conflict resolution when local has a newer timestamp than server
7. `ClearCompleted` correctly coalesces pending Creates (no server delete)
8. Toggling an item during an in-flight sync for the same item

**Detection**: Search the `#[cfg(test)]` module for test function names or
assertions that cover each scenario. Missing coverage is a Warning finding.

---

## CRX-010: Unused dependencies in Cargo.toml

**Severity**: Info

Every dependency listed in `[dependencies]` in `shared/Cargo.toml` must be
referenced by at least one `use` statement in the source code. Unused
dependencies increase compile time and binary size.

**Detection**: For each dependency in `Cargo.toml`, search all `.rs` files
under `shared/src/` for a corresponding `use <crate_name>` or
`<crate_name>::` reference. Flag deps with no matching usage.

**Example**: `opsx_todo` lists `url = "2"` in dependencies but never imports
or uses the `url` crate.
