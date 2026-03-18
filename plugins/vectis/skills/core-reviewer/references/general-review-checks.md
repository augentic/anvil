# General Code Quality Checks

Language-level and framework-agnostic quality checks for Rust code. These apply
to all `.rs` files under `shared/src/`. Tests (`#[cfg(test)]` modules) are
exempt from GEN-001 and GEN-002.

---

## GEN-001: No unwrap() or expect() in production code

**Severity**: Warning

`unwrap()` and `expect()` panic on failure. In a Crux core, a panic crashes
the shell and is unrecoverable. Use `?` with error propagation, `if let`,
`match`, `unwrap_or_default()` (with care -- see LOG-005), or
`unwrap_or_else(|| ...)`.

**Detection**: Search for `.unwrap()` and `.expect(` outside `#[cfg(test)]`
modules.

**Exemption**: Tests may use `unwrap()` freely.

**Example**:
```rust
// BAD
let bytes = serde_json::to_vec(&state).expect("serializing PersistedState");

// BETTER (graceful degradation)
let bytes = match serde_json::to_vec(&state) {
    Ok(b) => b,
    Err(e) => {
        log::error!("Failed to serialize state: {e}");
        return Command::done();
    }
};
```

---

## GEN-002: No debug output in production code

**Severity**: Info

`println!`, `dbg!`, and `eprintln!` should not appear in production code.
Use `log::debug!`, `log::info!`, or `tracing` macros instead. These integrate
with the shell's logging infrastructure and can be filtered by level.

**Detection**: Search for `println!`, `dbg!`, `eprintln!` outside
`#[cfg(test)]` modules.

---

## GEN-003: No hardcoded secrets or credentials

**Severity**: Critical

API keys, passwords, tokens, and other secrets must not appear in source code.
Use configuration, environment variables, or the shell's secure storage.

**Detection**: Search for patterns that commonly indicate secrets:
- Strings matching `sk-`, `pk-`, `ghp_`, `Bearer `, `token`, `password`,
  `secret`, `api_key`, `apikey`
- Base64-encoded strings longer than 20 characters in `const` declarations
- URLs containing credentials (`https://user:pass@...`)

**Exemption**: Example/placeholder URLs like `https://api.example.com` are fine.

---

## GEN-004: Public types and functions have doc comments

**Severity**: Info

All `pub` items (structs, enums, functions, type aliases) should have a doc
comment (`///`) explaining their purpose. This is especially important for
types that cross the FFI bridge, as shell developers rely on these docs.

**Detection**: Find `pub struct`, `pub enum`, `pub fn`, `pub type` declarations
that lack a preceding `///` doc comment.

**Exemption**: Short, self-descriptive items where the name fully conveys the
purpose (e.g. `pub struct Filter`) may omit doc comments at the reviewer's
discretion.

---

## GEN-005: Error paths propagate errors, not silently swallow them

**Severity**: Warning

When an operation can fail, the error must be handled explicitly: propagated,
logged, or converted to a user-visible error state. Silently ignoring errors
(empty `Err(_) => {}`, `let _ = fallible_call()`, or `ok()`) hides bugs and
makes debugging impossible.

**Detection**: Search for:
- `Err(_) => {}` or `Err(_) => Command::done()` without logging
- `let _ = ...` on a `Result`
- `.ok()` discarding the error variant
- `unwrap_or_default()` on deserialization of user data (see LOG-005)

**Acceptable patterns**: `Err(_) => { model.error = ...; render() }` is fine
because the error is surfaced to the user.

---

## GEN-006: Unnecessary .clone()

**Severity**: Info

`.clone()` should only be used when ownership transfer is genuinely needed.
Cloning large structures (vectors, strings) in hot paths wastes memory and CPU.
Check if a borrow (`&`) would suffice.

**Detection**: Look for `.clone()` on:
- Items immediately passed to a function that accepts `&T`
- Struct fields that are only read, not moved
- Loop iterations that clone the iterator element but only read fields

**Note**: In Crux `update()`, cloning is sometimes unavoidable because the
`Command` chain takes ownership. Use judgment -- false positives are common.

---

## GEN-007: Integer arithmetic overflow

**Severity**: Warning

Rust integer arithmetic panics on overflow in debug builds and wraps in release.
For counters, indices, or IDs derived from arithmetic, verify that overflow
is either impossible (e.g., bounded by a small collection size) or explicitly
handled with `checked_add`, `saturating_add`, or `wrapping_add`.

**Detection**: Search for `+= 1`, `+ 1`, `- 1` on integer variables. Check
whether the variable is bounded or could theoretically overflow.

**Example**: A `next_local_id: u32` counter that increments on every add
would overflow after ~4 billion items. Practically unreachable, but worth
a note if the field type is smaller (e.g., `u16`).

---

## GEN-008: Non-exhaustive match arms hiding new variants

**Severity**: Warning

Catch-all `_ =>` or `.. =>` patterns in `match` statements can silently
swallow new enum variants added during updates. Prefer listing all variants
explicitly so the compiler flags missing arms when variants are added.

**Detection**: Search for `_ =>` in match statements on project-defined enums
(not on external types where exhaustive matching is impractical). Check whether
the catch-all could hide a meaningful variant.

**Acceptable**: `_ => Command::done()` on SSE event type strings (open set)
is fine. `_ =>` on `Filter` or `Page` (closed set) is not.

---

## GEN-009: Serialization round-trip completeness

**Severity**: Warning

Types stored in `KeyValue` (persistence) must derive both `Serialize` and
`Deserialize` to support save and load. Types sent through effects (Event
payloads, ViewModel) must derive both to cross the FFI bridge.

**Detection**: For types used in `serde_json::to_vec` or `serde_json::to_string`,
verify they also appear in `serde_json::from_slice` or `serde_json::from_str`
contexts (or could reasonably need to). Check derives match.

See also CRX-005 for the Crux-specific bridge variant of this check.

---

## GEN-010: No unsafe blocks

**Severity**: Critical

`unsafe` blocks should never appear in a Crux shared crate. The core runs in
a sandboxed environment and has no need for unsafe operations. If unsafe code
is present, it indicates a design problem.

**Detection**: Search for `unsafe {` or `unsafe fn`.

---

## GEN-011: Functions under 50 lines

**Severity**: Info

Functions longer than 50 lines are harder to review, test, and maintain.
Extract helper functions for logical sub-tasks within `update()`, `view()`,
or standalone functions.

**Detection**: Count the lines (excluding blank lines and comments) of each
function. Flag functions exceeding 50 lines.

**Exemption**: The `update()` match block often exceeds this limit due to
the number of Event variants. In that case, verify that each arm is short
and delegates to helper functions for non-trivial logic.

---

## GEN-012: Option/Result handling covers all cases

**Severity**: Warning

Every `if let Some(x) = ...` should have an `else` branch (or the absence
should be justified). Every `match` on `Option` or `Result` should explicitly
handle `None`/`Err` rather than silently doing nothing.

**Detection**: Search for `if let Some` and `if let Ok` without a
corresponding `else`. Check whether the missing branch could indicate a logic
gap (e.g., an item not found in the list -- should this be an error?).

**Acceptable**: `if let Some(item) = model.items.find(...)` where the item
not being found is a harmless no-op (e.g., stale event for a deleted item).
