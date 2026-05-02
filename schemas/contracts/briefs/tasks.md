---
id: tasks
description: Create the task list for contract build and validation
generates: tasks.md
needs: [specs]
---

Follow the task format conventions defined in the define skill for checkbox format, grouping, ordering, and skill directive tags.

## Agent-Completable Constraint

Generate only tasks that an agent can complete and verify with contract artifacts and local validators. Do not generate manual review, external service, production credentials, or user-confirmation tasks.

When alignment needs review, express it as a format-skill verification task that produces machine-readable output. Use the relevant `contract:*` verifier intent for `$ref` resolution, schema metadata, binding completeness, and warning-free alignment checks.

## Self-Review

After drafting `tasks.md`, re-read every checkbox line and ask, for each task:

1. Could a coding agent perform this action using the `contract:*` author/verifier intents, local validators, or contract artifacts available below?
2. If the task mentions humans, manual review, external services, production credentials, or user confirmation, is the action genuinely avoiding them or genuinely requiring them? Requiring them is a rewrite. Avoiding them is fine, but prefer to omit the reference entirely so future readers don't have to parse the negation.
3. Does the list as a whole include at least one verification task — a `contract:*` verifier intent for `$ref` resolution, schema metadata, binding completeness, or warning-free alignment?

Rewrite any task that fails (1) or (2) before handing the file off. If (3) fails, add a verifier-intent task using a skill from the table below.

For `tasks.md`, `specify change validate` checks checkbox/grouping shape only — it does not inspect task intent. Agent-completability is judged here at write-time and re-checked by `/spec:build` as a preflight.

## Available Skills

| Directive | Skill | When to Use |
|-----------|-------|-------------|
| `contract:openapi` | Author, import, or verify OpenAPI artifacts | HTTP/resource interactions |
| `contract:asyncapi` | Author, import, or verify AsyncAPI artifacts | Evented, pub/sub, streaming, or WebSocket interactions |
| `contract:json-schema` | Author, import, or verify reusable JSON Schema artifacts | Shared payload vocabulary referenced by HTTP and/or evented interactions |

Pick the directive whose format matches each interface. When a change contains both HTTP and evented interactions, order the per-interface tasks `contract:json-schema` first (shared payloads), then `contract:openapi` (HTTP), then `contract:asyncapi` (events) so later format passes can reuse the schemas authored or imported earlier. Each skill exposes author, importer, and verifier intents; the build phase selects the intent from the proposal's Authorship Mode and the presence of supplied files.

## Standard Task Groups

Contract changes produce a fixed set of build and verification tasks. Generate one group per interface in the specs, plus a cross-cutting verification group:

### Per-interface tasks

For each interface in `specs/`, emit one build task per format skill the interface needs (skip formats with no interactions). Use **Author** wording for prose-driven or modification changes and **Import and normalize** wording for import-driven changes:

- [ ] `contract:json-schema` — Author shared payload schemas for `<interface>`
- [ ] `contract:openapi` — Author OpenAPI delta for `<interface>` (HTTP interactions only)
- [ ] `contract:asyncapi` — Author AsyncAPI delta for `<interface>` (evented interactions only)
- [ ] `contract:json-schema` — Import and normalize JSON Schema artifacts for `<interface>` (import mode only)
- [ ] `contract:openapi` — Import and normalize OpenAPI artifacts for `<interface>` (import mode only)
- [ ] `contract:asyncapi` — Import and normalize AsyncAPI artifacts for `<interface>` (import mode only)
- [ ] `contract:json-schema` — Verify `<interface>` JSON Schema artifacts
- [ ] `contract:openapi` — Verify `<interface>` OpenAPI artifacts (HTTP interactions only)
- [ ] `contract:asyncapi` — Verify `<interface>` AsyncAPI artifacts (evented interactions only)

### Cross-cutting verification

Emit one task per format skill present in the change. For mixed-format changes, also emit a final cross-format consistency task.

- [ ] `contract:json-schema` — Verify `$ref` resolution and schema metadata (`$id`, `title`, `description`) across all change-local schemas
- [ ] `contract:openapi` — Verify `$ref` resolution and binding completeness across the OpenAPI delta
- [ ] `contract:asyncapi` — Verify `$ref` resolution and binding completeness across the AsyncAPI delta
- [ ] `contract:json-schema` — Verify cross-format `$ref` consistency and report duplicate schema identities (mixed-format changes only)
- [ ] `contract:openapi` — Review alignment report for warnings (HTTP interactions only)
- [ ] `contract:asyncapi` — Review alignment report for warnings (evented interactions only)
