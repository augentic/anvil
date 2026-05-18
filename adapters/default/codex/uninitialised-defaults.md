---
id: UNI-001
title: Uninitialised or Incorrectly Defaulted Values
severity: important
trigger: A default, sentinel, nil, zero, or empty value is used where that state has no valid domain meaning.
---

## Rule

Do not let language defaults or ambiguous sentinel values represent meaningful domain state unless that state is explicitly valid. Model unknown, unloaded, empty, and intentionally absent values distinctly when the distinction affects behavior.

## Look For

- Struct fields whose default value is used at runtime but has no domain meaning, such as `count: 0` where zero is indistinguishable from "unknown".
- Optional or nullable fields accessed before the value is populated by an async load, with no guard or loading-state check.
- Default enum variants that silently swallow missing data rather than representing a genuine initial state.
