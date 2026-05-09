---
id: UNI-012
title: Backward-Incompatible Changes to Persisted State
severity: critical
trigger: A persisted model changes shape without migration or backward-compatible deserialization behavior.
---

## Rule

Preserve compatibility for persisted state across schema changes. Added, removed, renamed, or retyped fields need migration logic, safe defaults, or other compatibility handling so existing data is not lost or made unreadable.

## Look For

- Persisted-state struct changes, such as new fields, renamed fields, or changed types, without corresponding default annotations or migration code.
- Removed fields that cause existing stored data to fail schema validation.
- Enum variants added to a persisted type without a fallback for unrecognised values in old data.
- Missing integration tests that verify deserialization of data stored by the previous version.

## Spec Guidance

When the spec does not address data migration for model changes, propose forward-compatibility requirements or a migration strategy.
