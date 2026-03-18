# Logic Review Checks

These checks cannot be detected by pattern-matching alone. They require
simulating event sequences, enumerating state transitions, and reasoning about
what happens when async operations interleave. Each check includes a simulation
technique and a concrete example drawn from real issues found in generated code.

The reviewer should read `spec.md` alongside `app.rs` when applying these
checks -- many logic bugs originate from gaps between what the spec describes
and what common sense requires.

---

## LOG-001: State machine completeness

**Severity**: Critical

**What to check**: For every enum used as a page, phase, or connection state
(`Page`, `SyncStatus`, `SseConnectionState`, etc.), enumerate all transitions
in `update()`. For each transition, verify that every required side-effect fires.

**Simulation technique**:

1. List all values of each state enum.
2. For each Event arm in `update()`, identify assignments to state fields.
3. For each assignment, list the side-effects in the returned `Command`:
   - `render()` -- does the view read this field?
   - `save_state()` -- is this field persisted?
   - Sync/reconnect -- does the transition require follow-up actions?
4. Flag any transition that mutates a view-visible field without `render()`.

**Example**: `ConnectSse` sets `model.sse_state = Connecting` but returns only
the SSE command without `render()`. The UI never shows a "connecting" indicator.
Similarly, navigating from `Page::Error` back to `Page::Loading` changes the
page but omits `render()`, so the Error view stays visible until `DataLoaded`
fires.

**State machine to verify** (for a typical sync app):

```
Page:  Loading -> TodoList (on DataLoaded Ok)
       Loading -> Error    (on DataLoaded Err)
       Error   -> Loading  (on Navigate/Retry)

SseConnectionState:  Disconnected -> Connecting  (on ConnectSse)
                     Connecting   -> Connected   (on SseReceived)
                     Connected    -> Disconnected (on SseDisconnected)

SyncStatus:  Idle    -> Syncing  (on start_sync)
             Syncing -> Idle     (on OpResponse Ok)
             Syncing -> Offline  (on OpResponse Err)
             Offline -> Syncing  (on RetrySync)
```

Each edge must emit `render()` if the `view()` function reads the field.

---

## LOG-002: Operation coalescing

**Severity**: Critical

**What to check**: When a destructive operation (Delete, ClearCompleted)
targets an item that only exists as a pending Create (never synced to the
server), the code must skip the server call entirely.

**Simulation technique**:

1. Trace this sequence:
   - User creates item A (pending Create pushed)
   - Sync has NOT yet run
   - User deletes item A (or clears completed including A)
2. After the delete handler runs, inspect `model.pending_ops`:
   - Does it contain a `PendingOp::Delete` for item A?
   - If yes: the sync loop will send a DELETE to the server for an item
     the server has never seen -> 404 or error
3. Correct behavior: the handler detects that A's only pending op is `Create`,
   removes the `Create`, and does NOT push a `Delete`.

**What to look for in code**:
```rust
// BAD: blindly replaces all ops with a Delete
model.pending_ops.retain(|op| op.item_id() != id);
model.pending_ops.push(PendingOp::Delete(id));

// GOOD: inspects what ops existed before deciding
let mut saw_create = false;
let mut saw_non_create = false;
model.pending_ops.retain(|op| { /* categorize and remove */ });
if saw_create && !saw_non_create {
    continue; // nothing to delete on server
}
```

Also apply this check to `ClearCompleted`, which must apply the same logic
per-item in a loop.

---

## LOG-003: Concurrent operation conflicts

**Severity**: Critical

**What to check**: When a sync operation is in-flight (`model.syncing_id` is
`Some(id)`) and a real-time event (SSE) arrives for the same item, the
pending-op cleanup must not corrupt the sync state.

**Simulation technique**:

1. Trace this sequence:
   - Sync starts for item A: `syncing_id = Some("A")`, the first pending op
     for A is being sent to the server
   - SSE `item_deleted` arrives for item A
   - SSE handler runs `pending_ops.retain(|op| op.item_id() != "A")` --
     this removes the op that is currently being synced
   - Server responds with `OpResponse(Ok(...))` or `DeleteOpResponse(Ok(...))`
   - Handler does `syncing_id.take()` and `pending_ops.retain(|op| op.item_id() != synced_id)`
2. What goes wrong: the SSE handler already removed the op, so the response
   handler retains everything and moves on. But if a second pending op for A
   was queued after the first, it may also have been removed by the SSE handler's
   overly broad `retain`.

**What to look for**:
- SSE `item_deleted` handler: does it check `syncing_id` before removing ops?
- Does it only remove ops that are NOT currently being synced?
- Is there a test that covers this exact interleaving?

---

## LOG-004: Temporal ordering / conflict resolution

**Severity**: Critical

**What to check**: Every conflict-resolution comparison must have timestamps
available on both sides. If either side can be `None` or missing, the
comparison logic must be explicitly designed for that case rather than falling
through to a default.

**Simulation technique**:

1. Find the `apply_server_item` or equivalent function.
2. For each comparison between local and server state:
   - What data is available on the local side? (fields of `PendingOp`, fields
     of the local `TodoItem`)
   - What data is available on the server side? (fields of the server response)
   - Is there a comparison like `server_ts >= local_ts`?
3. For `PendingOp::Delete`: does it carry a `deleted_at` timestamp?
   Without it, the code cannot determine whether a server update happened
   before or after the local delete.
4. Check fallback cases: if either timestamp is `None`, does the code
   explicitly decide who wins, or does it fall through to a default?

**What to look for**:
```rust
// BAD: Option with implicit server-wins fallback
match (&local.updated_at, &server.updated_at) {
    (Some(l), Some(s)) => s >= l,
    _ => true, // server wins when either is None -- data loss risk
}

// GOOD: non-optional field eliminates the ambiguity
server_item.updated_at >= local_item.updated_at
```

---

## LOG-005: Fallback-on-None / default semantics

**Severity**: Warning

**What to check**: For every `unwrap_or_default()`, `Option` with a `_ => true`
catch-all, or `None` fallback path, verify that the default value is
semantically correct in the domain.

**Simulation technique**:

For each instance, ask these questions:
- What does the zero/empty/default value mean in this domain?
- Is `""` (empty string) a valid title? (Usually no.)
- Is `0` a valid count or does it mean "unknown"? (Context-dependent.)
- Is "no timestamp" older than all timestamps or newer? (Neither -- it's
  ambiguous, which is why it should not be `Option` in the first place.)
- Does `unwrap_or_default()` on a serialization failure silently produce
  an empty state that will overwrite valid persisted data?

**Example**:
```rust
// Risky: if serialization fails, replaces valid state with empty defaults
let state: PersistedState = serde_json::from_slice(&bytes).unwrap_or_default();
```

If `bytes` is corrupted, this silently initializes with an empty state,
discarding all the user's data. Consider logging the error or returning it
to the shell.

---

## LOG-006: Rapid-action sequences

**Severity**: Warning

**What to check**: Verify correct behavior when the user performs the same
action multiple times faster than async operations can complete.

**Simulation technique**:

1. Trace: user toggles item A -> first sync starts -> user toggles item A
   again before sync completes
   - Does a second `PendingOp::Update` get pushed?
   - When the first sync completes and the handler runs `start_sync`, does
     it pick up the second op correctly?
   - Are there now duplicate `Update` ops for the same item?

2. Trace: user clicks "Add" rapidly 5 times with the same text
   - Are 5 items created with different IDs? (Correct if IDs come from shell.)
   - Or are 5 items created with sequential IDs that collide with existing
     items? (Bug if IDs are generated from a model counter.)

3. Trace: user deletes item A, then item B, then item C in rapid succession
   - Does `start_sync` only process one at a time (correct)?
   - Or does it start multiple syncs concurrently, potentially corrupting
     `syncing_id`?

**What to look for**:
- `start_sync` should check `syncing_id.is_some()` and return `Command::done()`
  if a sync is already in-flight.
- Pending ops should not accumulate duplicates for the same item unless the
  item's state genuinely changed between ops.

---

## LOG-007: Spec gap detection

**Severity**: Warning

**What to check**: Compare each user-facing Event variant against the Features
section of `spec.md`. For each Event, identify untrusted inputs and verify
that common-sense validation exists even when the spec is silent.

**Simulation technique**:

For each user-facing Event (not internal/callback events):

1. What inputs does it accept? (Strings, IDs, booleans)
2. What are the preconditions the spec states? (Usually none for simple actions.)
3. What preconditions does common sense require?
   - Text inputs: non-empty after trimming?
   - IDs: does the referenced item exist in the model?
   - Toggles: is the item in a valid state for toggling?
4. What happens with adversarial input?
   - Empty string for title
   - ID that doesn't match any item
   - Duplicate add with same ID

**Example**: The spec says "Edit title -- user edits the title of a todo item."
It does not mention empty titles. But accepting an empty title creates an
invisible item in the list. The generated code should reject empty titles
regardless of spec silence.

**Cross-reference**: Each Event should map to at least one Feature in the spec.
Events with no spec Feature may indicate dead code or missing spec coverage.

---

## LOG-008: Missing edge-case tests

**Severity**: Warning

**What to check**: Cross-reference the `#[cfg(test)]` module against the
interaction sequences identified by LOG-001 through LOG-007. Each identified
risk should have at least one test.

**Required test scenarios** (minimum set for a sync app):

| Scenario | Checks | Why |
|---|---|---|
| SSE event during in-flight sync for same item | LOG-003 | Race condition between SSE and sync completion |
| SSE delete does not clobber next pending op | LOG-003 | Ensures retain() is scoped to the right op |
| EditTitle with empty string is a no-op | LOG-007 | Input validation on untrusted text |
| ClearCompleted with no completed items | LOG-007 | Edge case: nothing to do |
| ClearCompleted coalesces pending Creates | LOG-002 | No phantom server deletes |
| Server-wins conflict resolution | LOG-004 | Server has newer timestamp |
| Local-wins conflict resolution | LOG-004 | Local has newer timestamp |
| Rapid toggle of same item | LOG-006 | No duplicate pending ops |
| Delete of item that was never synced | LOG-002 | Create->Delete before sync |

**Detection**: Search the test module for function names or assertion patterns
that cover each scenario. Missing coverage is a Warning finding. List all
missing scenarios in the review report.
