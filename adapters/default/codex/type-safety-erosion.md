---
id: UNI-017
title: Type Safety Erosion
severity: important
trigger: Weakly typed values are used where enums, newtypes, or stronger domain types would prevent invalid states.
---

## Rule

Use domain types to make invalid states unrepresentable where practical. Prefer enums, newtypes, and constrained types over plain strings, booleans, or loosely typed values when the domain has known valid values or distinct identifiers.

## Look For

- Fields typed as `String` that hold values from a known, closed set, such as status codes, filter names, categories, or roles.
- ID fields typed as plain `String` that are interchangeable with unrelated IDs, such as a user ID accidentally passed where an item ID is expected.
- Boolean parameters where more than two states exist or may exist in the future.
- Struct fields whose valid values are constrained, but the constraint is only enforced at one call site rather than by the type system.
