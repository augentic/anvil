---
id: UNI-010
title: Unhandled Exceptions / Panics Causing Crashes
severity: critical
trigger: A fallible operation can terminate the process because the failure is not handled or propagated safely.
---

## Rule

Handle or safely propagate failures from operations that can throw, panic, trap, or otherwise terminate execution. A recoverable failure should not crash the host process or leave the user without a recovery path.

## Look For

- Calls to operations that can fail, such as I/O, parsing, arithmetic, or collection access, without error handling, try/catch, or result propagation.
- Force-unwrap patterns that assume a value is always present when it may not be.
- Index-based collection access without bounds checking.
- Division or modulo operations without zero-divisor guards.
- FFI boundary methods that panic instead of returning an error type.
