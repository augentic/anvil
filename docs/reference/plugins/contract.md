# Contract

The Contract plugin provides format-first specialist skills for API contract generation, validation, and import. It works with three standard formats: JSON Schema for payload definitions, OpenAPI 3.1 for HTTP / resource bindings, and AsyncAPI 3.0 for messaging / event bindings.

The plugin is the format-first reorganisation of the contract surface. The persisted artifact surface stays unchanged: the `contracts@v1` schema, the `contracts/` baseline directory, and every contract artifact path keep their original names. The Cursor plugin and slash-command surface live under `/contract:*`.

## Skills

| Skill | Format | Purpose |
|-------|--------|---------|
| `/contract:openapi` | OpenAPI 3.1 | HTTP / resource APIs (paths, methods, request/response bodies) |
| `/contract:asyncapi` | AsyncAPI 3.0 | Evented / pub-sub / streaming interfaces (channels, operations, messages) |
| `/contract:json-schema` | JSON Schema (draft 2020-12) | Reusable payload schemas without a protocol wrapper |

Each skill carries three intents internally and dispatches via its own intent table. Operators or briefs select the format first; the skill then matches the prompt to one of three sibling files:

| Intent | Trigger | Sibling file |
|--------|---------|--------------|
| Author or extend | contracts schema build brief during `/spec:build`; operator extending the baseline for new interactions | `author.md` |
| Import or normalise | operator drops an external document into a slice's `contracts/` directory | `importer.md` |
| Verify or run cross-project consumer check | contracts schema build brief in `/spec:build` (verify-repair loop); post-merge cross-project compatibility check (RFC-9 §3B) | `verifier.md` |

### Author intent

Reads baseline contracts at `contracts/` and the slice's specs, validates alignment, and produces the minimal delta for interactions the specs require that the baseline does not already cover. The algorithm is the same regardless of baseline state -- the three authorship patterns (contract-first, spec-first, contract-given) differ in outcome, not in code path.

The author intent produces an **alignment report** summarising:

- **Covered by baseline** -- interactions already defined in the baseline, with alignment pass/warning per interaction.
- **New (delta produced)** -- interactions the specs require that the baseline does not cover.
- **Normalisation** -- baseline files that received missing metadata (`$id`, `description`).

A clean report with zero delta is the expected outcome for implementation slices in a contract-first workflow.

### Verifier intent

Read-only validation of contract artifacts after the author intent completes. Checks:

1. **`$ref` resolution** -- all `$ref` pointers in OpenAPI and AsyncAPI files resolve to existing schema files.
2. **Schema metadata** -- every JSON Schema file has `$id`, `title`, and `description`.
3. **Binding completeness** -- every schema that appears as a top-level payload in a spec scenario has at least one protocol binding. Shared vocabulary types (e.g. `ErrorResponse`) used only as `$ref` targets are exempt.

The verifier intent does not modify files -- it reports issues for the brief's verify-repair loop to act on.

It also exposes a `--mode cross-project` flag that compares a producer's merged contract against each consumer's tier-2 workspace clone. The mode is a verifier sub-mode, not a separate skill: the algorithms and output shapes are unchanged from the previous standalone surface.

### Importer intent (Layer 2)

Automates the manual import workflow for external contracts:

1. **Format detection** -- identifies Swagger 2.0, OpenAPI 3.0/3.1, AsyncAPI 2.x/3.0, standalone JSON Schema.
2. **Version upgrade** -- converts older formats to target versions (Swagger 2.0 → OpenAPI 3.1, AsyncAPI 2.x → 3.0).
3. **Schema decomposition** -- extracts inline schema definitions into separate files under `contracts/schemas/`.
4. **Metadata injection** -- adds `$id`, `$schema`, `title`, `description` where missing.

In Layer 1, operators perform these steps manually by placing conformant files into the slice's `contracts/` directory.

## References

Format-neutral material is shared across the three skills under `plugins/contract/references/`:

| Reference | Content |
|-----------|---------|
| Baseline vs delta | What lives in `contracts/` versus a slice's `contracts/`, and how merges promote |
| Cross-project compatibility | Producer / consumer roles, compatibility rules, finding categories |
| Import upgrade policy | Swagger 2.0 → OpenAPI 3.1, AsyncAPI 2.x → 3.0, schema metadata defaults |
| Report shape | Alignment report and verifier output schemas |

Format-specific patterns and examples (OpenAPI conventions, AsyncAPI conventions, JSON Schema conventions, artifact structure) live alongside each format's `SKILL.md` under `plugins/contract/references/`.

## How the plugin is invoked

The Contract plugin is schema-independent. It is invoked from the `contracts` brief in the define pipeline, which is present in:

- The **Contracts schema** -- for dedicated contract changes (authoring and import).
- The **Omnia schema** -- for alignment validation during implementation slices.
- The **Vectis schema** -- for alignment validation during implementation slices.

The brief picks the format-appropriate skill (OpenAPI for HTTP / resource APIs, AsyncAPI for evented / pub-sub / streaming, JSON Schema for shared payload schemas) and dispatches to its author intent (alignment validation and delta production), then to its verifier intent (post-generation consistency checks), with a verify-repair loop of up to 2 iterations.

## CLI counterpart

The matching read-only CLI surface lives under [`specify contract`](../cli/contract.md): `specify contract list` projects every top-level contract under `contracts/`, and `specify contract validate` runs the RFC-12 §Validation checks (SemVer `info.version`, kebab-case `info.x-specify-id` when present, cross-repo id uniqueness).
