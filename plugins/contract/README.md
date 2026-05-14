# Contracts

Format-specialist skills to author, import, and verify API/interface contracts — OpenAPI 3.1 HTTP APIs, AsyncAPI 3.0 evented messaging, and standalone JSON Schema (Draft 2020-12) — from Specify artifacts.

Each format skill owns three intents (author, import, verify) inside a single skill directory. The shared cross-format references under `references/` carry the format-neutral rules every skill obeys.

## Skills

| Skill | Description |
|-------|-------------|
| [openapi](skills/openapi/SKILL.md) | OpenAPI 3.1 HTTP API contracts — path operations, request and response schemas, parameters, security schemes, and baseline deltas. Author from specs, import external OpenAPI / Swagger 2.0 documents, and verify internal consistency or cross-project consumer compatibility. |
| [asyncapi](skills/asyncapi/SKILL.md) | AsyncAPI 3.0 evented contracts — pub/sub, streaming, queue, and WebSocket-style channels, operations, and message bindings. Author from specs, import external AsyncAPI 2.x or 3.0 documents, and verify internal consistency or cross-project consumer compatibility. |
| [json-schema](skills/json-schema/SKILL.md) | Standalone JSON Schema (Draft 2020-12) payload vocabulary shared by the protocol skills. Author reusable schemas from specs, import external schema files (Draft 4 / 6 / 7 / 2019-09 / 2020-12), and verify `$ref` resolution, `$id` discipline, and cross-format consumer compatibility against existing OpenAPI and AsyncAPI bindings. |

Each skill's `SKILL.md` dispatches to format-specific `author.md`, `importer.md`, and `verifier.md` siblings; the contracts capability build brief enforces the json-schema → openapi → asyncapi ordering when more than one format is in scope.

## References

Format-specific conventions:

- [JSON Schema Conventions](references/json-schema-conventions.md) — `$id` URN format, metadata rules, type mapping, `$ref` conventions
- [OpenAPI Conventions](references/openapi-conventions.md) — OpenAPI 3.1 structure, path / method conventions, `$ref → ../schemas/`
- [AsyncAPI Conventions](references/asyncapi-conventions.md) — AsyncAPI 3.0 structure, channel / operation conventions

Cross-format references shared by every format skill:

- [Artifact Structure](references/artifact-structure.md) — `contracts/` layout, naming, change-level delta rules
- [Baseline vs Delta](references/baseline-vs-delta.md) — three authorship patterns (contract-first / spec-first / contract-given), already-covered / new-or-modified / normalisation classification, opaque-file-replacement merge contract
- [Import / Upgrade Policy](references/import-upgrade-policy.md) — format detection, per-format upgrade targets (Swagger 2.0 → OpenAPI 3.1; AsyncAPI 2.x → 3.0; JSON Schema Draft 4 / 6 / 7 / 2019-09 → 2020-12), lossless-vs-lossy decisions, when to refuse and ask the operator
- [Report Shape](references/report-shape.md) — single-mode markdown, baseline validator JSON, compatibility report JSON, locator format, exit semantics
- [Cross-Project Compatibility](references/cross-project-compatibility.md) — RM-04 classifications, `change-kind` enumeration, consumer-view resolution, breaking-change classification policy
- [Contracts Codex](../../capabilities/contracts/codex/) — stable `IFACE-*` reviewer rules
