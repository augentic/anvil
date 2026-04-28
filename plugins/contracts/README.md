# API Contracts

Generate and validate machine-readable API contracts (JSON Schema, OpenAPI 3.1, AsyncAPI 3.0) from Specify artifacts.

## Skills

| Skill | Description |
|-------|-------------|
| [writer](skills/writer/SKILL.md) | Validate spec alignment with baseline contracts and produce the minimal contract delta |
| [validator](skills/validator/SKILL.md) | Verify consistency of contract artifacts. Two modes: `single` (default — `$ref` resolution, schema metadata, binding completeness for one change) and `cross-project` (RFC-9 §3B — compare a producer's updated contract against a consumer's tier-2 workspace clone for breaking changes) |
| [importer](skills/importer/SKILL.md) | Import external contracts with format detection, version upgrade, and metadata injection (Layer 2) |

### Cross-project compatibility check (RFC-9 §3B)

`/contracts:validator --mode cross-project` is the wire-level safety net for multi-repo platforms. The `/spec:execute` driver invokes it after every successful merge of a contract listed in the producer project's `registry.yaml:contracts.produces`. The validator compares the merged contract against each consumer's tier-2 workspace clone (`.specify/workspace/<consumer>/.specify/contracts/...`) and emits a structured YAML report of breaking changes (removed fields, newly-required fields, narrowed types, removed endpoints/channels). Findings are non-fatal — the execute driver records them as `cross-project-warning:` entries in the merged change's `journal.yaml` and renders a warning block in the merge transcript, but never halts the loop.

See [validator/SKILL.md → §Cross-Project Mode](skills/validator/SKILL.md#cross-project-mode-rfc-9-3b) for the algorithm and output schema, and [`/spec:execute` → §Cross-project contract check](../spec/skills/execute/SKILL.md#cross-project-contract-check-rfc-9-3b) for the post-merge invocation contract.

## References

- [JSON Schema Conventions](references/json-schema-conventions.md) -- `$id` format, metadata rules, type mapping, `$ref` conventions
- [OpenAPI Conventions](references/openapi-conventions.md) -- OpenAPI 3.1 structure, path/method conventions, `$ref → ../schemas/`
- [AsyncAPI Conventions](references/asyncapi-conventions.md) -- AsyncAPI 3.0 structure, channel/operation conventions
- [Artifact Structure](references/artifact-structure.md) -- `.specify/contracts/` layout, naming, change-level delta rules
