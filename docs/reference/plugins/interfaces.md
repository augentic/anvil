# Contracts

The Contracts plugin provides specialist skills for API contract generation, validation, and import. It works with three standard formats: JSON Schema for payload definitions, OpenAPI 3.1 for HTTP bindings, and AsyncAPI 3.0 for messaging bindings.

## Skills

| Skill | Purpose |
|-------|---------|
| `/contracts:writer` | Validate spec alignment with baseline contracts and produce the minimal contract delta |
| `/contracts:validator` | Verify internal consistency of contract artifacts (`$ref` resolution, metadata, binding completeness) |
| `/contracts:importer` | Import and normalise external contracts with format detection, version upgrade, and metadata injection (Layer 2) |

### /contracts:writer

Reads baseline contracts at `.specify/contracts/` and the change's specs, validates alignment, and produces the minimal delta for interactions the specs require that the baseline does not already cover. The algorithm is the same regardless of baseline state -- the three authorship patterns (contract-first, spec-first, contract-given) differ in outcome, not in code path.

The writer produces an **alignment report** summarising:

- **Covered by baseline** -- interactions already defined in the baseline, with alignment pass/warning per interaction.
- **New (delta produced)** -- interactions the specs require that the baseline does not cover.
- **Normalisation** -- baseline files that received missing metadata (`$id`, `description`).

A clean report with zero delta is the expected outcome for implementation changes in a contract-first workflow.

### /contracts:validator

Read-only validation of contract artifacts after the writer completes. Checks:

1. **`$ref` resolution** -- all `$ref` pointers in OpenAPI and AsyncAPI files resolve to existing schema files.
2. **Schema metadata** -- every JSON Schema file has `$id`, `title`, and `description`.
3. **Binding completeness** -- every schema that appears as a top-level payload in a spec scenario has at least one protocol binding. Shared vocabulary types (e.g. `ErrorResponse`) used only as `$ref` targets are exempt.

The validator does not modify files -- it reports issues for the brief's verify-repair loop to act on.

### /contracts:importer (Layer 2)

Automates the manual import workflow for external contracts:

1. **Format detection** -- identifies Swagger 2.0, OpenAPI 3.0/3.1, AsyncAPI 2.x/3.0, standalone JSON Schema.
2. **Version upgrade** -- converts older formats to target versions (Swagger 2.0 → OpenAPI 3.1, AsyncAPI 2.x → 3.0).
3. **Schema decomposition** -- extracts inline schema definitions into separate files under `contracts/schemas/`.
4. **Metadata injection** -- adds `$id`, `$schema`, `title`, `description` where missing.

In Layer 1, operators perform these steps manually by placing conformant files into the change's `contracts/` directory.

## References

The plugin bundles reference documents consulted by skills during generation:

| Reference | Content |
|-----------|---------|
| JSON Schema Conventions | `$id` format, metadata rules, type mapping, `$ref` conventions |
| OpenAPI Conventions | OpenAPI 3.1 structure, path/method conventions, `$ref → ../schemas/` |
| AsyncAPI Conventions | AsyncAPI 3.0 structure, channel/operation conventions |
| Artifact Structure | `.specify/contracts/` layout, naming, change-level delta rules |

## How the plugin is invoked

The Contracts plugin is schema-independent. It is invoked from the `contracts` brief in the define pipeline, which is present in:

- The **Contracts schema** -- for dedicated contract changes (authoring and import).
- The **Omnia schema** -- for alignment validation during implementation changes.
- The **Vectis schema** -- for alignment validation during implementation changes.

The brief delegates to `/contracts:writer` (alignment validation and delta production) and `/contracts:validator` (post-generation consistency checks), with a verify-repair loop of up to 2 iterations.
