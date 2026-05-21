# JSON Schema Conventions

Rules for JSON Schema files under `contracts/schemas/`. These schemas define the shared payload vocabulary — domain types referenced by both OpenAPI and AsyncAPI bindings.

## File Naming

- **Kebab-case** `.yaml` files named after the domain type: `user-registration.yaml`, `error-response.yaml`, `order-placed.yaml`.
- **One type per file.** Each schema file defines exactly one top-level type. Shared sub-types that appear in multiple schemas get their own file and are referenced via `$ref`.
- Names should reflect the domain concept, not the transport or binding. Use `order-placed.yaml`, not `order-placed-event.yaml` or `post-order-body.yaml`.

## `$id` Format

Every JSON Schema file **must** have a `$id` field. Use a URN-shaped identifier derived from the file path:

```yaml
$id: "urn:specify:schemas/user-registration"
```

Rules:

- **Prefix**: always `urn:specify:`.
- **Path segment**: matches the file's path under `contracts/` without the `.yaml` extension. A schema at `contracts/schemas/user-registration.yaml` gets `$id: "urn:specify:schemas/user-registration"`.
- **Stability**: `$id` values must not change once a schema is merged into the baseline. Renaming a schema requires a new `$id` and deprecation of the old one.
- **Uniqueness**: each `$id` must be unique within the contract tree. The file-path derivation guarantees this when the one-type-per-file rule is followed.
- **Tooling compatibility**: the URN format is a valid URI per RFC 8141 and works with standard JSON Schema tooling (`ajv`, `typify`, `json-schema-ref-parser`).

## Required Metadata

Every JSON Schema file **must** include these top-level fields:

| Field | Rule | Example |
|-------|------|---------|
| `$id` | URN derived from file path (see above) | `"urn:specify:schemas/user-registration"` |
| `$schema` | Always `"https://json-schema.org/draft/2020-12/schema"` | — |
| `title` | PascalCase type name matching the domain concept | `"UserRegistration"` |
| `description` | From the spec's behavioral description of this type. Must describe what the type represents, not how it is used. | `"Payload for creating a new user account."` |
| `type` | The JSON Schema type (`object`, `string`, etc.) | `"object"` |

Example:

```yaml
$id: "urn:specify:schemas/user-registration"
$schema: "https://json-schema.org/draft/2020-12/schema"
title: UserRegistration
description: Payload for creating a new user account.
type: object
properties:
  email:
    type: string
    format: email
    description: The user's email address. Must be unique across the platform.
  display_name:
    type: string
    minLength: 1
    maxLength: 100
    description: The user's chosen display name.
  password:
    type: string
    format: password
    minLength: 8
    description: Account password. Must meet the platform's complexity requirements.
required:
  - email
  - display_name
  - password
additionalProperties: false
```

## `$ref` Conventions

- **Between schema files**: use same-directory relative paths. A schema at `schemas/order-placed.yaml` referencing `schemas/user.yaml` uses `$ref: "user.yaml"`.
- **From OpenAPI/AsyncAPI bindings**: use relative paths to the `../schemas/` directory. An OpenAPI file at `http/user-api.yaml` references `$ref: "../schemas/user-registration.yaml"`.
- **No absolute URIs in `$ref`**: all references are local, relative paths. External schema references are not supported — import external schemas as local files first.
- **No inline definitions**: do not use `$defs` / `definitions` blocks for types that other schemas also need. Extract shared types into their own files and use `$ref`.
- **`$defs` for file-local sub-types**: `$defs` is acceptable for sub-types used only within the same file (e.g. an `Address` sub-object used only inside `UserRegistration`). If a second schema later needs the same sub-type, extract it to its own file.

## Type Mapping

Map spec scenario data to JSON Schema types using the following guidance:

| Spec Concept | JSON Schema | Notes |
|---|---|---|
| Text / names / identifiers | `type: string` | Add `minLength`, `maxLength` where the spec constrains length |
| Email addresses | `type: string`, `format: email` | — |
| URLs / URIs | `type: string`, `format: uri` | — |
| Dates (no time) | `type: string`, `format: date` | ISO 8601 date: `2024-03-15` |
| Timestamps | `type: string`, `format: date-time` | ISO 8601 datetime: `2024-03-15T10:30:00Z` |
| Integers | `type: integer` | Add `minimum`, `maximum` where the spec constrains range |
| Decimal numbers / money | `type: number` | Add `minimum`, `maximum` where appropriate |
| Booleans | `type: boolean` | — |
| Known value sets | `type: string`, `enum: [...]` | For status codes, categories, roles, etc. |
| Ordered collections | `type: array`, `items: { ... }` | Specify `minItems`, `maxItems` where the spec constrains cardinality |
| Nested structures | `type: object`, `properties: { ... }` | Extract to a separate schema file via `$ref` when reused |
| Optional fields | Omit from `required` array | Do not use `type: ["string", "null"]` unless the spec explicitly distinguishes "absent" from "null" |
| Binary / file content | `type: string`, `format: binary` | For base64-encoded payloads in JSON; prefer Blobstore for large files |

### Constraints

- Use `required` arrays on object schemas. List every field that the spec mandates.
- Use `additionalProperties: false` on object schemas unless the spec explicitly allows arbitrary keys.
- Use `pattern` for fields with known regex constraints (e.g. phone numbers, postal codes).
- Use `enum` for fields with a known, closed set of values. Do not use `enum` for open-ended fields.

## Draft Version

All schemas use **JSON Schema Draft 2020-12**:

```yaml
$schema: "https://json-schema.org/draft/2020-12/schema"
```

This draft is natively supported by OpenAPI 3.1 and is compatible with `ajv`, `typify`, and `schemars`.

## Property Naming

- **snake_case** for property names in JSON Schema definitions: `display_name`, `created_at`, `order_id`.
- This aligns with Rust struct field naming conventions and avoids serde `rename_all` in generated types.
- When a spec or external system uses camelCase, the schema still uses snake_case. Serde attributes handle wire-format conversion in generated code.

## Error Types

Error response schemas follow a standard structure:

```yaml
$id: "urn:specify:schemas/error-response"
$schema: "https://json-schema.org/draft/2020-12/schema"
title: ErrorResponse
description: Standard error response returned by all API endpoints on failure.
type: object
properties:
  code:
    type: string
    description: Machine-readable error code.
  message:
    type: string
    description: Human-readable error description.
  details:
    type: object
    description: Additional error context. Structure varies by error type.
    additionalProperties: true
required:
  - code
  - message
additionalProperties: false
```

Platform-wide error types like `ErrorResponse` are defined once and referenced everywhere via `$ref`.

## See Also

- [openapi-conventions.md](openapi-conventions.md) -- OpenAPI 3.1 binding conventions
- [asyncapi-conventions.md](asyncapi-conventions.md) -- AsyncAPI 3.0 binding conventions
- [artifact-structure.md](artifact-structure.md) -- Directory layout and naming rules
