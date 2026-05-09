---
id: UNI-006
title: Race Conditions and Concurrency Bugs
severity: critical
trigger: Shared mutable state or in-flight operations can interleave in an order the implementation does not safely handle.
---

## Rule

Protect shared mutable state and async workflows against unsafe interleavings. The implementation should define the isolation, ordering, cancellation, or in-flight guards needed to keep state consistent.

## Look For

- State mutations performed outside the expected isolation context, such as updating UI state from a background thread.
- Two async operations that can complete in either order, where only one ordering is handled correctly.
- Missing operation in-flight guards that allow a second operation to start before the first completes, corrupting shared state.
- Broad-scope cleanup, such as removing all pending operations for an item, that can interfere with an unrelated in-flight operation.
