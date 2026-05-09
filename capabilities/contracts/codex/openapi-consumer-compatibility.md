---
id: IFACE-001
title: OpenAPI Consumer Compatibility
severity: critical
trigger: An OpenAPI path, operation, parameter, request body, response, status code, or referenced payload schema changes in a contract that existing consumers may use.
---

## Rule

Preserve compatibility for existing OpenAPI consumers unless the slice intentionally introduces a breaking contract change and classifies its impact. Removing or renaming paths, methods, operation ids, status codes, media types, parameters, request fields, or response fields is breaking. Adding new endpoints, status codes, examples, documentation, or optional fields is compatible when existing requests and responses remain valid.

When editing an existing `contracts/http/*.yaml` file, keep the unrelated baseline content intact. Contract merge uses opaque file replacement, so omitted operations or reordered unrelated sections can hide accidental deletions from reviewers.

## Look For

- Removed or renamed paths, methods, `operationId` values, parameters, request bodies, response bodies, content types, or status codes.
- Previously optional parameters or request-body properties that become required.
- Response schemas that remove fields, narrow field types, remove enum values, or tighten validation constraints.
- `$ref` changes that point to a narrower schema or break the shared `../schemas/` reference discipline.
- Delta files that omit existing operations from the same baseline file or reformat unrelated sections during an otherwise small compatibility change.

## Spec Guidance

When the spec requires a breaking HTTP contract change, require an explicit consumer-impact note: affected consumers, migration path, version bump, and whether the previous endpoint or response shape remains available during transition.
