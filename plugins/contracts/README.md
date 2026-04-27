# API Contracts

Generate and validate machine-readable API contracts (JSON Schema, OpenAPI 3.1, AsyncAPI 3.0) from Specify artifacts.

## Skills

| Skill | Description |
|-------|-------------|
| [writer](skills/writer/SKILL.md) | Validate spec alignment with baseline contracts and produce the minimal contract delta |
| [validator](skills/validator/SKILL.md) | Verify internal consistency of contract artifacts (`$ref` resolution, metadata, binding completeness) |
| [importer](skills/importer/SKILL.md) | Import external contracts with format detection, version upgrade, and metadata injection (Layer 2) |

## References

- [JSON Schema Conventions](references/json-schema-conventions.md) -- `$id` format, metadata rules, type mapping, `$ref` conventions
- [OpenAPI Conventions](references/openapi-conventions.md) -- OpenAPI 3.1 structure, path/method conventions, `$ref → ../schemas/`
- [AsyncAPI Conventions](references/asyncapi-conventions.md) -- AsyncAPI 3.0 structure, channel/operation conventions
- [Artifact Structure](references/artifact-structure.md) -- `.specify/contracts/` layout, naming, change-level delta rules
