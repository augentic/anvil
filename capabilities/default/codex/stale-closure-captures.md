---
id: UNI-015
title: Stale Closure Captures
severity: important
trigger: A closure or async block can execute after captured values have changed or become invalid.
---

## Rule

Ensure closures and async blocks observe the state they are meant to use at execution time. Avoid stale snapshots when the value may change between capture and execution, and avoid references whose owning scope can invalidate them.

## Look For

- Async blocks or callbacks that capture local variables which are mutated between the capture point and the execution point.
- Closures that capture a model state snapshot before an async operation, then use the snapshot when the operation resolves even though the model may have changed in the interim.
- Event handlers that capture loop variables or iterator state.
- Closures capturing mutable references where the owning scope may invalidate the reference before the closure runs.
