# Universal Review Checks

Language-agnostic code quality checks applicable to both Rust core and Swift
iOS shell code. Each reviewer skill applies these checks using
platform-specific detection heuristics documented in its own SKILL.md.

Checks tagged with **Spec-change indicator** commonly surface as requirements
gaps rather than pure code defects. When the spec is silent on the concern
a check raises, the reviewer should express the finding as a proposed spec
change (via `/spec:define`) rather than only fixing the code.

---

## UNI-001: Uninitialised or Incorrectly Defaulted Values

**Severity**: Warning

Values initialised to a language default (`Default::default()`, `nil`, zero,
empty string) that have no valid domain meaning in that state. Includes
sentinel values used ambiguously for multiple purposes (e.g., `None` meaning
both "not yet loaded" and "intentionally empty").

**What to look for**:

- Struct fields whose default value is used at runtime but has no domain
  meaning (e.g., `count: 0` where zero is indistinguishable from "unknown").
- Optional / nullable fields accessed before the value is populated by an
  async load, with no guard or loading-state check.
- Default enum variants that silently swallow missing data rather than
  representing a genuine initial state.

---

## UNI-002: Unvalidated Input

**Severity**: Critical

Any user-supplied or external data entering a handler without validation.
This includes text input (missing trim / empty check), numeric input (missing
range check), and ID references (target may not exist in the model).

**What to look for**:

- Handler entry points that accept strings from the user without trimming
  whitespace or rejecting empty values.
- Numeric parameters without range or sign validation.
- ID lookups that assume the referenced item exists (no guard for missing).
- Data received from external APIs consumed without schema or type validation.

**Spec-change indicator**: When the spec is silent on validation rules for a
user action, the finding should propose adding explicit acceptance criteria
(e.g., "title must be non-empty after trimming", "quantity must be 1..999").

---

## UNI-003: Serialization / Deserialization Failures Not Handled

**Severity**: Critical

Fallible encode / decode operations where the error path is missing,
silently swallowed, or causes a crash. Includes FFI boundary type mismatches
where the serialised format on one side does not match the expected schema on
the other.

**What to look for**:

- Serialize / deserialize calls with no error handling or a catch-all that
  discards the error.
- Types crossing a serialization boundary (FFI, persistence, network) that
  are missing required derive macros or protocol conformances.
- Deserialization failures that fall back to a default value which could
  overwrite valid persisted data.

---

## UNI-004: Logic Bugs

**Severity**: Critical

Defects in control flow, conditionals, or state transitions that produce
incorrect behavior. This is a reasoning-intensive check that cannot be
detected by pattern matching alone.

**What to look for**:

- State machine transitions with missing edges (state A can reach state B,
  but the handler for that transition is absent).
- Inverted boolean conditions (`if !condition` where `if condition` was
  intended).
- Off-by-one errors in index arithmetic or boundary checks.
- Conditions that are always true or always false, making one branch
  unreachable.

**Spec-change indicator**: When a state transition is missing because the
spec never defined that edge case, the spec needs a new scenario rather than
an ad-hoc code fix.

---

## UNI-005: Unbounded Growth and Resource Leaks

**Severity**: Warning

Collections, queues, or caches that grow without bound; subscriptions or
observers registered without cleanup; retained references that prevent
deallocation.

**What to look for**:

- Collections (lists, maps, queues) that receive `.push()` / `.append()` /
  `.insert()` without a corresponding removal, cap, or eviction policy.
- Event listeners, observers, or subscriptions registered without a
  matching unsubscribe or cancellation path.
- Strong reference cycles between objects (especially closures capturing
  their owning object).
- Long-lived async tasks or futures that are never cancelled when they
  become irrelevant.

---

## UNI-006: Race Conditions and Concurrency Bugs

**Severity**: Critical

Shared mutable state accessed from multiple async contexts without
synchronization; interleaving of operations that can corrupt state; missing
guards on in-flight operations.

**What to look for**:

- State mutations performed outside the expected isolation context (e.g.,
  updating UI state from a background thread).
- Two async operations that can complete in either order, where only one
  ordering is handled correctly.
- Missing "operation in-flight" guards that allow a second operation to
  start before the first completes, corrupting shared state.
- Broad-scope cleanup (e.g., removing all pending ops for an item) that
  can interfere with an unrelated in-flight operation.

---

## UNI-007: Unnecessarily Chatty External Calls

**Severity**: Warning

Redundant or excessive calls to external systems that could be batched,
debounced, deduplicated, or eliminated entirely.

**What to look for**:

- Re-fetching data the app already has (e.g., full reload after receiving a
  real-time update that already contains the new state).
- N+1 call patterns: looping over items and making one external call per
  item when a batch API exists.
- Missing debounce on rapid-fire user actions that each trigger a network
  call.
- Fetch-on-navigate patterns that re-request unchanged data without caching
  or staleness checks.

**Spec-change indicator**: When the spec mandates behavior that inherently
creates chattiness (e.g., "refresh the full list on every keystroke"), the
finding should propose a spec amendment with a more efficient interaction
pattern.

---

## UNI-008: Instrumentation and Logging Balance

**Severity**: Warning

Error paths with no logging (under-instrumented) or hot paths with
per-item logging (over-instrumented). Also covers sensitive data leaked
into logs and debug-only output left in production code.

**What to look for**:

- Error / failure branches that silently discard the error with no log
  statement, metric, or diagnostic output.
- Log statements inside tight loops or per-item processing that would
  produce excessive output at scale.
- Personally identifiable information (PII), tokens, or credentials
  interpolated into log messages.
- Debug-only output (`println!`, `dbg!`, `print()`, `debugPrint()`)
  remaining in production code.

**Spec-change indicator**: When the spec has no observability requirements
but the app clearly needs them (error tracking, performance metrics), the
finding should propose adding an observability section to the spec.

---

## UNI-009: Handle-Then-Throw Anti-Pattern

**Severity**: Warning

Catching an error, performing partial side effects (state mutation, UI
update, resource allocation), then re-raising or returning a different error.
This leaves the system in an inconsistent state where the side effects are
visible but the operation is reported as failed.

Also covers the inverse: catching errors at too low a level when they should
bubble up to a caller that has the context to handle them properly.

**What to look for**:

- Catch blocks that mutate shared state (model, view, database) before
  re-throwing or returning an error -- the mutation persists even though the
  operation failed.
- Error handlers that convert a specific, informative error into a generic
  one, losing diagnostic context.
- Try/catch at a low level that swallows errors which the caller needs to
  know about (e.g., catching a network error inside a helper and returning
  a default, when the caller should show an error state).
- Nested error handling where an inner handler partially handles and the
  outer handler also partially handles, with neither completing the job.

---

## UNI-010: Unhandled Exceptions / Panics Causing Crashes

**Severity**: Critical

Fallible operations without error handling that terminate the process on
failure. In a Crux app, a panic or force-unwrap crash in the core kills the
host process (iOS app, browser tab) with no recovery path.

**What to look for**:

- Calls to operations that can fail (I/O, parsing, arithmetic, collection
  access) without error handling, try/catch, or Result propagation.
- Force-unwrap patterns that assume a value is always present when it may
  not be.
- Index-based collection access without bounds checking.
- Division or modulo operations without zero-divisor guards.
- FFI boundary methods that panic instead of returning an error type.

---

## UNI-011: Missing Timeout or Retry on External Calls

**Severity**: Warning

HTTP requests, SSE connections, and effect resolutions with no timeout or
retry strategy. A hanging call with no timeout blocks the effect chain
indefinitely, leaving the app in a non-responsive state with no user-visible
indication of failure.

**What to look for**:

- HTTP requests dispatched without a configured timeout.
- SSE or WebSocket connections with no reconnection strategy after
  disconnect.
- Effect handlers that await a response indefinitely with no timeout or
  cancellation path.
- Retry logic that retries without backoff, risking a tight retry loop on
  persistent failures.

**Spec-change indicator**: When the spec does not define timeout or retry
behavior for external calls, the finding should propose adding resilience
requirements (timeout duration, retry count, backoff strategy, user-facing
indication of failure).

---

## UNI-012: Backward-Incompatible Changes to Persisted State

**Severity**: Critical

When a model struct used in persistence (KeyValue, UserDefaults, file
storage) changes -- fields added, removed, renamed, or retyped -- existing
persisted data may fail to deserialize. Without migration logic or
backward-compatible defaults, the app silently loses user data or crashes on
launch.

**What to look for**:

- Persisted-state struct changes (new fields, renamed fields, changed types)
  without corresponding default annotations or migration code.
- Removed fields that cause existing stored data to fail schema validation.
- Enum variants added to a persisted type without a fallback for unrecognised
  values in old data.
- Missing integration tests that verify deserialization of data stored by
  the previous version.

**Spec-change indicator**: When the spec does not address data migration for
model changes, the finding should propose adding forward-compatibility
requirements or a migration strategy.

---

## UNI-013: Dead Code and Unreachable Paths

**Severity**: Info

Code that can never execute: statements after early returns, match arms
shadowed by earlier guards, event handlers with no dispatch site, functions
defined but never called. Compilers catch some of this, but logically dead
paths (always-true conditions, unreachable state combinations) require
human reasoning.

**What to look for**:

- Functions or methods with no call site in the codebase.
- Match/switch arms that are unreachable because an earlier arm catches
  all matching values.
- Code after unconditional `return`, `break`, `continue`, or `throw`.
- Conditional branches guarded by conditions that are always true or
  always false given the surrounding context.
- Event variants defined in the data model but never dispatched by any
  view or handler.

---

## UNI-014: Hardcoded Configuration Values

**Severity**: Warning

Operational parameters (timeouts, URLs, retry counts, buffer sizes, page
sizes) embedded as literal values in code rather than named constants or
configuration. Hardcoded values are hard to find, hard to change, and easy
to leave inconsistent across call sites.

**What to look for**:

- Numeric literals in function calls that represent tunable parameters
  (timeout durations, retry counts, page sizes, polling intervals).
- URL strings embedded directly in handler code rather than sourced from
  configuration.
- Magic numbers that require domain knowledge to understand (e.g., `42`,
  `1000`, `86400`).
- The same literal value repeated in multiple locations (should be a shared
  constant).

**Spec-change indicator**: When the spec does not define operational
parameters (timeouts, page sizes, retry limits, polling intervals), the
finding should propose adding them as explicit design decisions so they are
reviewed and documented.

---

## UNI-015: Stale Closure Captures

**Severity**: Warning

Closures or async blocks that capture values by value or reference, where
the captured value changes before the closure executes. The closure operates
on a stale snapshot, producing incorrect results or silently dropping updates.

**What to look for**:

- Async blocks or callbacks that capture local variables which are mutated
  between the capture point and the execution point.
- Closures that capture a model state snapshot before an async operation,
  then use the snapshot when the operation resolves (the model may have
  changed in the interim).
- Event handlers that capture loop variables or iterator state.
- Closures capturing mutable references where the owning scope may
  invalidate the reference before the closure runs.

---

## UNI-016: Error Message Quality

**Severity**: Info

Error messages that lack enough diagnostic context to identify the problem
without reproducing it. Every error path should answer: what operation
failed, on what data, and why.

**What to look for**:

- Generic error messages with no specifics: "operation failed", "invalid
  input", "something went wrong".
- Error messages that omit the item ID, field name, or value that caused
  the failure.
- Catch blocks that log the error type but not the error message or
  underlying cause.
- Multiple error sites using identical messages, making it impossible to
  determine which site produced the error in logs.

---

## UNI-017: Type Safety Erosion

**Severity**: Warning

Using `String` (or equivalent weakly-typed representation) where an enum,
newtype, or stronger type would prevent bugs at compile time. Stringly-typed
fields allow invalid values that the compiler cannot catch, pushing
validation to runtime where it is easily forgotten.

**What to look for**:

- Fields typed as `String` that hold values from a known, closed set
  (status codes, filter names, categories, roles). These should be enums.
- ID fields typed as plain `String` that are interchangeable with unrelated
  IDs (e.g., a user ID accidentally passed where an item ID is expected).
  These should be newtypes.
- Boolean parameters where more than two states exist or may exist in the
  future (should be an enum for extensibility).
- Struct fields whose valid values are constrained but the constraint is
  only enforced at one call site rather than by the type system.
