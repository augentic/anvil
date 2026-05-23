---
id: UNI-002
title: Unvalidated Input
severity: critical
trigger: User-supplied or external data enters a handler or domain boundary without validation.
---

## Rule

Validate all user-supplied or external data at the boundary before domain logic consumes it. Validation should cover text normalization, required values, numeric ranges, identifier existence, and external payload shape.

## Look For

- Handler entry points that accept strings from the user without trimming whitespace or rejecting empty values.
- Numeric parameters without range or sign validation.
- ID lookups that assume the referenced item exists, with no guard for missing targets.
- Data received from external APIs consumed without schema or type validation.

## Spec Guidance

When the spec is silent on validation rules for a user action, propose explicit acceptance criteria such as "title must be non-empty after trimming" or "quantity must be 1..999".
