---
id: UNI-003
title: Serialization / Deserialization Failures Not Handled
severity: critical
trigger: Fallible encode or decode operations can fail without a handled error path.
---

## Rule

Handle serialization and deserialization failures explicitly. A failed encode, decode, or boundary conversion must not be silently swallowed, crash the process, or fall back to a value that can corrupt state.

## Look For

- Serialize or deserialize calls with no error handling or a catch-all that discards the error.
- Types crossing a serialization boundary, such as FFI, persistence, or network, that are missing required derive macros or protocol conformances.
- Deserialization failures that fall back to a default value which could overwrite valid persisted data.
