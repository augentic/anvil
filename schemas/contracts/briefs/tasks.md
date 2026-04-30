---
id: tasks
description: Create the task list for contract validation
generates: tasks.md
needs: [specs, contracts]
---

Follow the task format conventions defined in the define skill for checkbox format, grouping, ordering, and skill directive tags.

## Available Skills

| Directive | Skill | When to Use |
|-----------|-------|-------------|
| `interfaces:openapi` | Author or verify OpenAPI artifacts | HTTP/resource interactions |
| `interfaces:asyncapi` | Author or verify AsyncAPI artifacts | Evented, pub/sub, streaming, or WebSocket interactions |
| `interfaces:json-schema` | Author or verify reusable JSON Schema artifacts | Shared payload vocabulary referenced by HTTP and/or evented interactions |

Pick the directive whose format matches each interface. When a change contains both HTTP and evented interactions, order the per-interface tasks `interfaces:json-schema` first (shared payloads), then `interfaces:openapi` (HTTP), then `interfaces:asyncapi` (events) so later format passes can reuse the schemas authored earlier. Each skill exposes both an author intent (generate or extend the artifact) and a verifier intent (consistency checks); the build phase runs the verifier intent of every format skill that owns artifacts in the change.

## Standard Task Groups

Contract changes produce a fixed set of authoring and verification tasks. Generate one group per interface in the specs, plus a cross-cutting verification group:

### Per-interface tasks

For each interface in `specs/`, emit one task per format skill the interface needs (skip formats with no interactions):

- [ ] `interfaces:json-schema` — Author shared payload schemas for `<interface>`
- [ ] `interfaces:openapi` — Author OpenAPI delta for `<interface>` (HTTP interactions only)
- [ ] `interfaces:asyncapi` — Author AsyncAPI delta for `<interface>` (evented interactions only)
- [ ] `interfaces:json-schema` — Verify `<interface>` JSON Schema artifacts
- [ ] `interfaces:openapi` — Verify `<interface>` OpenAPI artifacts (HTTP interactions only)
- [ ] `interfaces:asyncapi` — Verify `<interface>` AsyncAPI artifacts (evented interactions only)

### Cross-cutting verification

Emit one task per format skill present in the change. For mixed-format changes, also emit a final cross-format consistency task.

- [ ] `interfaces:json-schema` — Verify `$ref` resolution and schema metadata (`$id`, `title`, `description`) across all change-local schemas
- [ ] `interfaces:openapi` — Verify `$ref` resolution and binding completeness across the OpenAPI delta
- [ ] `interfaces:asyncapi` — Verify `$ref` resolution and binding completeness across the AsyncAPI delta
- [ ] `interfaces:json-schema` — Verify cross-format `$ref` consistency and report duplicate schema identities (mixed-format changes only)
- [ ] `interfaces:openapi` — Review alignment report for warnings (HTTP interactions only)
- [ ] `interfaces:asyncapi` — Review alignment report for warnings (evented interactions only)
