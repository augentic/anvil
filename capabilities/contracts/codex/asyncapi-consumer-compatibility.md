---
id: IFACE-002
title: AsyncAPI Consumer Compatibility
severity: critical
trigger: An AsyncAPI channel, operation, message, header, payload schema, or operation role changes in a contract that existing producers or consumers may use.
---

## Rule

Preserve compatibility for existing AsyncAPI producers and consumers unless the slice intentionally introduces a breaking event contract change and classifies its impact. Removing or renaming channel addresses, channel keys, operations, messages, message names, payload schemas, required headers, or operation roles is breaking. Adding new optional channels, operations, messages, headers, or payload fields is compatible when existing publishers and subscribers keep working unchanged.

AsyncAPI contracts describe the wire shape and operation semantics, not broker runtime policy. Keep retry, dead-letter, partitioning, retention, and consumer-group behavior in implementation design unless those details are required for wire compatibility.

## Look For

- Removed or renamed channel addresses, channel keys, operations, message definitions, or `send` / `receive` role declarations.
- Message payload `$ref` changes that remove fields, add required fields, narrow types, remove enum values, or tighten validation constraints.
- Required message headers added without a migration path for existing publishers.
- Channel address changes that look like a rename but are represented as delete-plus-add for consumers.
- Broker-specific bindings used to smuggle implementation policy into the contract when they do not affect wire compatibility.
- Delta files that omit existing channels or operations from the same baseline file during opaque replacement.

## Spec Guidance

When the spec requires a breaking event contract change, require an explicit producer / consumer impact note: affected channels, migration path, version bump, and whether both old and new channels or message shapes coexist during transition.
