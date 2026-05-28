---
id: IFACE-003
title: JSON Schema Evolution Safety
severity: critical
trigger: A JSON Schema under contracts/schemas changes fields, required properties, types, formats, enum values, validation constraints, additionalProperties, or its stable $id.
---

## Rule

Treat JSON Schema files as shared payload vocabulary consumed by OpenAPI, AsyncAPI, generated code, and downstream projects. Once a schema is merged, its `$id` is stable. Schema changes must be backward-compatible for existing payload producers and consumers, or they must be classified as breaking with a migration path.

Compatible evolution is additive or widening: optional fields, wider enum sets, looser ranges, broader accepted types, documentation, examples, or descriptions. Breaking evolution removes or renames fields, adds required fields, narrows types or formats, removes enum values, tightens patterns or numeric ranges, changes `additionalProperties` from open to closed, or changes a schema's `$id`.

## Look For

- `$id`, filename, or `title` drift on an existing schema instead of introducing a new schema and deprecating the old one.
- Removed or renamed properties, newly required properties, narrower `type` / `format`, stricter `pattern`, tighter numeric or length constraints, or enum values removed.
- `additionalProperties` tightened from absent or `true` to `false`.
- Schema changes made without checking HTTP and messaging contracts that already reference the schema.
- Inline schema definitions used to avoid evolving the shared schema file.
- Opaque replacement deltas that omit existing properties from the baseline schema file.

## Spec Guidance

When the spec requires a breaking payload change, ask whether it should be a new schema identity with a deprecation path for the old `$id`, or an accepted breaking change with named consumers and a planned rollout.
