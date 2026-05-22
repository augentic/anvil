# JSON Schema — Author

> **When to read this.** Read this when authoring or extending standalone JSON Schema documents under `contracts/schemas/` for a Specify change — i.e. when the contracts adapter build brief during `/spec:build` selects the author intent for shared payload vocabulary, or when an operator extends the baseline for new payload types referenced (or about to be referenced) by the `openapi` or `asyncapi` sub-flows. Skip this file when importing external schema files (use [`importer.md`](./importer.md)) or when verifying existing artefacts (use [`verifier.md`](./verifier.md)).

## Inputs

```text
$SLICE_DIR     = .specify/slices/<slice-name>
$SPECS_DIR      = $SLICE_DIR/specs
$CONTRACTS_DIR  = $SLICE_DIR/contracts
$SCHEMAS_DIR    = $CONTRACTS_DIR/schemas
$BASELINE_DIR   = contracts
```

## Authority hierarchy

When sources conflict, follow this strict precedence:

1. **This file** — author rules and hard constraints for JSON Schema documents.
2. **Specify artefacts** (specs) — behavioural requirements drive the field shape.
3. **Format conventions** — [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md).
4. **Baseline schemas** (`contracts/schemas/`) — existing platform vocabulary; never overwrite silently.
5. **LLM inference** — prohibited for unknowns; mark with `[unknown]` and surface in the alignment report.

If specs and baseline disagree on a shape (e.g. the spec asserts a `verified_at` field that the baseline `User` schema omits), surface the mismatch in the alignment report's Warnings section. Never silently overwrite baseline schemas — a human reviewer decides whether to extend the existing schema or fork a new `$id`.

## Ownership note

Per the cross-format ordering rule in `contracts.build`, this sub-flow **owns** all decisions about JSON Schema files. The `openapi` and `asyncapi` sub-flows only `$ref` into `contracts/schemas/`; they never inline or restructure schema files. When an OpenAPI or AsyncAPI path realises a payload type is missing or under-specified, it must route the schema work back here before continuing — never inline a workaround.

This skill therefore runs **before** the protocol skills in any mixed-format change. The author algorithm assumes the protocol skills will pick up `$ref` pointers later; it does not need to coordinate with them mid-run.

## The 4-step author algorithm

The author runs four sequential steps end-to-end whenever the contracts adapter build brief asks for shared payload coverage. Each step is a focused, independently-checkable phase; downstream steps assume the upstream output is well-formed.

### Step 1 — Read the baseline

Build an inventory of `$BASELINE_DIR/schemas/`:

| Source | Extract |
|---|---|
| `schemas/*.yaml` | `$id`, `title`, `description`, `type`, `properties` (names + types + formats), `required` array, `$ref` targets, `$defs`, `additionalProperties` policy |

For each baseline schema, record:

- **Identity** — `$id` (URN) plus filename.
- **Shape** — property names, types, formats, required-ness, nested `$ref` targets, file-local `$defs`.
- **Consumers** — which baseline files in `$BASELINE_DIR/http/` and `$BASELINE_DIR/messages/` reference the schema via `$ref: "../schemas/<name>.yaml"`. Knowing this is a precondition for the alignment report's compatibility section.

When `$BASELINE_DIR/schemas/` is empty or absent, the baseline is empty — every spec-derived type becomes delta. Record an empty inventory and proceed.

### Step 2 — Identify payload types from the specs

Read every `*.md` file under `$SPECS_DIR/` and harvest payload-shaped requirements. A spec scenario maps to a JSON Schema file when it describes:

- A named entity referenced as a request body, response body, or message payload (`UserRegistration`, `Order`, `OAuthToken`, `ErrorResponse`).
- A reusable sub-shape used across multiple operations or channels (`Address`, `Money`, `Pagination`).
- A platform-wide vocabulary type that appears across briefs (`ErrorResponse`, `ResourceLink`).

Build a structured list of **spec-derived types**, one entry per named type:

```text
- type_name: OAuthToken
  fields:
    - access_token: string (required)
    - refresh_token: string (optional)
    - expires_in: integer (required, seconds)
    - token_type: string, enum=[bearer], (required)
  source: specs/auth.md REQ-014
- type_name: VerificationCode
  fields:
    - email: string format=email (required)
    - code: string pattern=^[0-9]{6}$ (required)
    - expires_at: string format=date-time (required)
  source: specs/auth.md REQ-015
```

When a spec scenario references a type by name without describing its fields ("respond with a `User`"), check whether the type is already defined in `$BASELINE_DIR/schemas/`. If so, record the reference and skip — there is no new schema to author. If not, surface the gap as a `[unknown]` finding in the alignment report and (typically) halt; a downstream protocol skill cannot wire `$ref` to a non-existent file.

When the slice has **no specs** (rare for the author intent — usually a contract-only import slice), skip steps 2–4 and route to [`importer.md`](./importer.md) instead.

### Step 3 — Compute the minimal delta

Compare each spec-derived type from Step 2 against the baseline inventory from Step 1. Classify into one of three buckets:

#### Already covered

The baseline already defines a schema with a matching `$id` (or matching filename). Verify alignment:

- Property names, types, and formats from the spec are present in the baseline schema.
- Spec-required fields are in the baseline's `required` array.
- Sub-type `$ref` targets exist in the baseline (transitively).

If alignment fails, record a warning with `{ baseline_file, spec_requirement_id, discrepancy }` for the alignment report. **Do not regenerate covered schemas** and **do not overwrite the baseline** — flag the mismatch and let a human resolve it (typically by either updating the spec or by introducing a new schema with a fresh `$id` and a deprecation note on the old one).

#### New or modified

The spec describes a type absent from the baseline, or asserts new fields on an existing baseline type. Add to the schema delta:

- New top-level types: write a new file under `$SCHEMAS_DIR/<kebab-name>.yaml`.
- New optional fields on an existing baseline type: produce a delta file that copies the baseline schema verbatim plus the new property; surface the slice as a compatibility note (additive optional fields are backwards-compatible).
- New required fields on an existing baseline type: produce the delta file but flag the slice as a backwards-incompatible warning — every consumer of the baseline schema will need to update.
- Type narrowing on an existing field (tighter `pattern`, narrower `enum`, lower `maximum`, etc.): also flag as a warning.

#### Normalisation

The baseline file lacks Specify-required metadata (`$id`, `$schema`, `title`, `description`). Propose a normalisation delta that adds the metadata without changing the schema's shape (no property additions, no required-set changes). Surface as a separate section in the alignment report.

### Step 4 — Generate or update schema files

For every type in the delta, write a file under `$SCHEMAS_DIR/<kebab-name>.yaml`. Apply the conventions in [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md) verbatim; the rules below are the operationally-critical ones for the author path.

Required structure:

```yaml
$schema: "https://json-schema.org/draft/2020-12/schema"
$id: "urn:specify:schemas/<filename-without-extension>"
title: <PascalCaseTypeName>
description: <one-sentence behavioural description from the spec>
type: object
properties:
  <snake_case_field>:
    type: <json-schema-type>
    description: <one-sentence purpose from the spec>
required:
  - <field>
additionalProperties: false
```

Then verify each generated file against the rules below.

## `$id` assignment policy

Every schema file declares a `$id` field. The format is non-negotiable:

```yaml
$id: "urn:specify:schemas/<filename-without-extension>"
```

Rules:

- **Prefix.** Always `urn:specify:`. Never `https://` or `file://`. The URN format is RFC 8141-compliant and works with `ajv`, `typify`, and `json-schema-ref-parser` without requiring a base URL.
- **Path segment.** Always `schemas/<kebab-name>` — the file's path under `contracts/` minus the `.yaml` extension. A schema at `contracts/schemas/user-registration.yaml` gets `$id: "urn:specify:schemas/user-registration"`.
- **Stability.** Once a schema is in the baseline, its `$id` is frozen. Renaming the conceptual type requires a new file with a new `$id`; never rewrite the existing `$id` in place. Surface a deprecation note on the old schema if needed.
- **Uniqueness.** Each `$id` is unique across the contract tree. The file-path derivation guarantees this when the one-type-per-file rule is honoured. The verifier flags duplicates.
- **No URLs.** Even though JSON Schema permits HTTP `$id` values, do not use them. URN form is the platform standard, ensures resolver-portability, and prevents accidental coupling to a domain.
- **No fragment suffixes.** `$id` values never carry `#` fragments. File-local sub-types use `$defs` with `$ref: "#/$defs/<name>"` inside the same file.

## One-type-per-file decomposition

Each schema file holds exactly one named, top-level type. This rule is enforced by both the author and the verifier.

When a spec scenario describes a payload that contains nested object shapes:

| Sub-shape usage | Decomposition |
|---|---|
| Used only inside one parent payload, no other consumer | Inline as `type: object` under `properties.<field>`, or use `$defs` if the structure recurs inside the same parent file |
| Used in two or more parent schemas | Extract to its own file `<sub-type>.yaml` and `$ref` from each parent |
| Already exists in `$BASELINE_DIR/schemas/` with a matching shape | Drop the inline definition; replace with `$ref: "<baseline-file>.yaml"` (relative path resolves either to change-local or baseline scope per [`../../references/artifact-structure.md`](../../references/artifact-structure.md)) |

The author's job is to predict reuse: when in doubt, extract. A file-local `$defs` entry can later be promoted to its own file without breaking consumers (the `$ref` rewrite is mechanical), but a wrongly-inlined shape that other schemas need is harder to refactor across an in-flight slice.

## Schema-file naming policy

Filenames are kebab-case and mirror the PascalCase type name in `title`:

| Type name (PascalCase) | Filename (kebab-case) | `$id` URN segment |
|---|---|---|
| `UserRegistration` | `user-registration.yaml` | `urn:specify:schemas/user-registration` |
| `OAuthToken` | `oauth-token.yaml` | `urn:specify:schemas/oauth-token` |
| `OrderPlaced` | `order-placed.yaml` | `urn:specify:schemas/order-placed` |
| `ErrorResponse` | `error-response.yaml` | `urn:specify:schemas/error-response` |
| `IPAddress` | `ip-address.yaml` | `urn:specify:schemas/ip-address` |

Acronym handling: collapse runs of capital letters into a single kebab segment (`OAuthToken` → `oauth-token`, `IPAddress` → `ip-address`). The `title` keeps the canonical PascalCase form (`OAuthToken`, `IPAddress`).

Naming guidance:

- **Domain-first**, transport-agnostic. `order-placed.yaml`, not `post-order-body.yaml` or `order-placed-event.yaml`. The same schema may appear as both an HTTP response body and an AsyncAPI message payload.
- **Singular by default** for entity types (`user.yaml`), plural only when the type itself is a collection wrapper (`order-list.yaml` for a paginated list type).
- **Suffix conventions** for narrow categories: `-request` / `-response` only when the schema is shape-specific to one direction and would otherwise collide with the entity name (e.g. `user-create-request.yaml` if the create payload differs from `user.yaml`).

## Vocabulary for shared payloads

Every schema file declares the metadata fields below. The vocabulary is fixed; do not invent new top-level keys.

| Field | Required? | Rule |
|---|---|---|
| `$schema` | required | Always `"https://json-schema.org/draft/2020-12/schema"`. |
| `$id` | required | URN derived from the file path (see §`$id` assignment policy). |
| `title` | required | PascalCase type name matching the domain concept. |
| `description` | required | One-sentence behavioural description sourced from the spec. Describes *what the type represents*, not *how it is used*. Never `[imported — description pending review]` from the author path (that placeholder is reserved for the importer). |
| `type` | required | The JSON Schema type — almost always `object`. Primitive top-level schemas are rare; prefer wrapping primitives in objects unless the spec explicitly defines a scalar vocabulary. |
| `properties` | required for `type: object` | Map of `snake_case` field names to type definitions. |
| `required` | required for `type: object` | Array listing every field the spec mandates. Empty arrays are allowed but discouraged — most payloads have at least one mandatory field. |
| `additionalProperties` | required for `type: object` | Default to `false` (closed schema). Use `true` only when the spec explicitly allows arbitrary extension keys (rare; usually a smell). |
| `$defs` | optional | File-local sub-type map. Use only for shapes that are not reused across files. |

Optional vocabulary (use when the spec justifies it):

- `enum` — for fields with a known closed set of values.
- `pattern` — for regex-constrained strings.
- `format` — for known string formats (`date-time`, `email`, `uri`, `uuid`, `binary`).
- `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum` — for numeric ranges. In Draft 2020-12, exclusive bounds are numeric values, never booleans.
- `minLength`, `maxLength` — for string length constraints.
- `minItems`, `maxItems`, `uniqueItems` — for array constraints.

## Spec → schema mapping rules

Map spec-scenario data to JSON Schema types using the table from [`../../references/json-schema-conventions.md`](../../references/json-schema-conventions.md). The most-used cases:

| Spec concept | JSON Schema | Notes |
|---|---|---|
| Text / names / identifiers | `type: string` | Add `minLength` / `maxLength` if the spec constrains length |
| Email | `type: string`, `format: email` | — |
| URL / URI | `type: string`, `format: uri` | — |
| Date (no time) | `type: string`, `format: date` | ISO 8601 date |
| Timestamp | `type: string`, `format: date-time` | ISO 8601 datetime, UTC preferred |
| UUID | `type: string`, `format: uuid` | — |
| Integer | `type: integer` | Add `minimum` / `maximum` for ranges |
| Decimal / money | `type: number` or string-encoded depending on precision needs | Spec dictates |
| Boolean | `type: boolean` | — |
| Closed value set | `type: string`, `enum: [...]` | Status codes, categories, roles |
| Ordered collection | `type: array`, `items: { ... }` | Specify `minItems` / `maxItems` if the spec constrains cardinality |
| Nested object (reused) | `$ref: "<other-file>.yaml"` | Extract per the decomposition rules above |
| Nested object (file-local) | `type: object` inline, or `$defs` entry | — |
| Optional field | omit from `required` array | Do not use `type: ["string", "null"]` unless the spec distinguishes "absent" from "null" |
| Binary content | `type: string`, `format: binary` | For base64 in JSON; large blobs go through Blobstore (see `design.md`) |

Property naming: **always `snake_case`** in JSON Schema (`display_name`, `created_at`, `order_id`). This aligns with Rust struct field conventions and avoids serde `rename_all` in generated code. When a spec uses camelCase, the schema still uses snake_case; serde attributes handle wire-format conversion in generated types.

## Alignment report

Every author run produces an alignment report alongside the delta files. The report is the primary output for the contracts adapter build brief — the YAML files are the artefact, but the report is how the brief decides whether the slice can proceed.

```markdown
## Alignment Report (Schemas)

### Coverage
- **Covered by baseline:** N types (M with alignment warnings)
- **New (delta produced):** N types
- **Normalisation:** N files updated with metadata

### Alignment Warnings
- `User`: spec scenario REQ-003 asserts `verified_at` field; baseline schema does not define it
- `OAuthToken`: spec defines `expires_in` as `integer`; baseline defines as `string`

### Generated Delta
- `contracts/schemas/oauth-token.yaml` (new)
- `contracts/schemas/verification-code.yaml` (new)
- `contracts/schemas/user.yaml` (updated — added optional `verified_at`)

### Backwards-compatibility flags
- `user.yaml` — added optional field `verified_at` (additive; no consumer impact)
- `error-response.yaml` — narrowed `code` enum (potential consumer impact; verify each baseline binding that references this schema)

### Normalisation
- `pagination.yaml` — added missing `description`
```

After producing the report, run [`verifier.md`](./verifier.md) in `single` mode against `$SLICE_DIR` to confirm `$ref` resolution, metadata completeness, duplicate-`$id` checks, and cross-format consumer compatibility before declaring the artefacts ready.

## Edge cases

| Scenario | Handling |
|---|---|
| Spec describes a payload without listing all fields | Author the known fields; mark unknowns with a `description: "[unknown] — not specified in current scenarios"` line on a placeholder property and surface in the alignment report. Never invent fields. |
| Spec defines a wrapper type (e.g. paginated list) | Extract the wrapper to its own schema (`paginated-list.yaml` or domain-specific `order-list.yaml`); use `items: { $ref: ... }` to reference the element type. |
| Two specs reference the same type with conflicting shapes | Surface the conflict as a warning; do not write a delta until the specs are reconciled. |
| Existing baseline schema lacks `$id` (legacy import) | Author a normalisation delta that adds `$id` derived from the filename, without changing the property shape. Surface as a `Normalisation` finding. |
| Spec asserts a field shape that contradicts the baseline | Two valid resolutions: (a) extend the baseline schema (additive), or (b) introduce a new schema with a fresh `$id`. The author surfaces both options in the alignment report and lets the operator pick. |
| Circular `$ref` between schemas | JSON Schema permits circular references but they complicate codegen. Author the circle correctly and flag it in the alignment report as a code-generation consideration. |

## Verification checklist

Before declaring the author run complete:

- [ ] Every spec-derived type maps to a schema in either the baseline or the delta.
- [ ] Every delta file has `$schema`, `$id`, `title`, `description`, `type`, `properties` (when `type: object`), `required`, and `additionalProperties`.
- [ ] Each filename, `$id`, and `title` form a coherent triple per the naming policy.
- [ ] One named type per file; shared sub-types are extracted; file-local sub-types use `$defs`.
- [ ] Property names are `snake_case`.
- [ ] No invented fields; every property traces back to a spec scenario or a baseline schema.
- [ ] Alignment report enumerates coverage, warnings, generated delta files, backwards-compatibility flags, and normalisation entries.
- [ ] [`verifier.md`](./verifier.md) (single mode) ran clean against `$SLICE_DIR`.

## See also

- [`json-schema-conventions`](../../references/json-schema-conventions.md) — full convention reference for `$id`, metadata, `$ref`, type mapping, naming.
- [`artifact-structure`](../../references/artifact-structure.md) — directory layout for the slice-local delta and the baseline.
- [`baseline-vs-delta`](../../references/baseline-vs-delta.md) — cross-format rules for the three authorship patterns, the already-covered / new-or-modified / normalisation classification, and the opaque-file-replacement merge contract that the §Step 3 delta computation operationalises for schemas.
- [`report-shape`](../../references/report-shape.md) — markdown shape for the alignment report produced by this author path.
- [`importer.md`](./importer.md) — sibling for normalising external schema files.
- [`verifier.md`](./verifier.md) — sibling for validating the authored output.
